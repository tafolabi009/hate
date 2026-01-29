//! JIT Compilation using Cranelift
//!
//! This module provides JIT compilation for hot code paths in Hate.
//! Uses Cranelift as the backend for generating native machine code.

#[cfg(feature = "jit")]
use cranelift::prelude::*;
#[cfg(feature = "jit")]
use cranelift_jit::{JITBuilder, JITModule};
#[cfg(feature = "jit")]
use cranelift_module::{Module, Linkage};
#[cfg(feature = "jit")]
use cranelift::codegen;
#[cfg(feature = "jit")]
use crate::bytecode::{Chunk, OpCode};

use std::collections::HashMap;

/// Threshold for JIT compilation (number of executions before compiling)
pub const JIT_THRESHOLD: u32 = 1000;

/// JIT compilation state
pub struct JitState {
    /// Execution counts for each function
    execution_counts: HashMap<usize, u32>,
    /// Compiled function pointers (function_idx -> native code)
    #[cfg(feature = "jit")]
    compiled_functions: HashMap<usize, *const u8>,
    #[cfg(not(feature = "jit"))]
    compiled_functions: HashMap<usize, ()>,
    /// Whether JIT is enabled
    enabled: bool,
}

impl JitState {
    pub fn new() -> Self {
        Self {
            execution_counts: HashMap::new(),
            compiled_functions: HashMap::new(),
            enabled: cfg!(feature = "jit"),
        }
    }

    /// Record a function execution and check if it should be JIT compiled
    pub fn record_execution(&mut self, func_idx: usize) -> bool {
        let count = self.execution_counts.entry(func_idx).or_insert(0);
        *count += 1;
        
        if *count >= JIT_THRESHOLD && !self.compiled_functions.contains_key(&func_idx) {
            return true; // Signal that this function should be compiled
        }
        false
    }

    /// Check if a function has been JIT compiled
    pub fn is_compiled(&self, func_idx: usize) -> bool {
        self.compiled_functions.contains_key(&func_idx)
    }

    /// Get execution count for a function
    pub fn get_execution_count(&self, func_idx: usize) -> u32 {
        *self.execution_counts.get(&func_idx).unwrap_or(&0)
    }
}

impl Default for JitState {
    fn default() -> Self {
        Self::new()
    }
}

/// JIT Compiler using Cranelift
#[cfg(feature = "jit")]
pub struct JitCompiler {
    /// Cranelift JIT module
    module: JITModule,
    /// Cranelift codegen context
    ctx: codegen::Context,
    /// Function builder context
    func_ctx: FunctionBuilderContext,
}

#[cfg(feature = "jit")]
impl JitCompiler {
    /// Create a new JIT compiler
    pub fn new() -> Result<Self, String> {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();
        flag_builder.set("opt_level", "speed").unwrap();
        
        let isa_builder = cranelift_native::builder()
            .map_err(|e| format!("Failed to create ISA builder: {}", e))?;
        let isa = isa_builder.finish(settings::Flags::new(flag_builder))
            .map_err(|e| format!("Failed to create ISA: {}", e))?;
        
        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let module = JITModule::new(builder);
        
        Ok(Self {
            module,
            ctx: codegen::Context::new(),
            func_ctx: FunctionBuilderContext::new(),
        })
    }

    /// Compile a Hate function to native code
    pub fn compile_function(&mut self, chunk: &Chunk, func_idx: usize) -> Result<*const u8, String> {
        // Get pointer type for the target architecture
        let pointer_type = self.module.target_config().pointer_type();
        
        // Create a signature for the function
        // Hate functions take (registers: *mut Value, frame_base: usize) -> Value
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type)); // registers pointer
        sig.params.push(AbiParam::new(types::I64));   // frame_base
        sig.returns.push(AbiParam::new(types::I64));  // return value (NaN-boxed)

        // Declare the function
        let func_name = format!("hate_func_{}", func_idx);
        let func_id = self.module
            .declare_function(&func_name, Linkage::Local, &sig)
            .map_err(|e| format!("Failed to declare function: {}", e))?;

        // Clear context for reuse
        self.ctx.func.signature = sig;
        self.ctx.func.name = codegen::ir::UserFuncName::user(0, func_idx as u32);
        
        // Build the function
        {
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.func_ctx);
            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);
            builder.seal_block(entry_block);

            // Get function parameters
            let registers_ptr = builder.block_params(entry_block)[0];
            let _frame_base = builder.block_params(entry_block)[1];

            // Compile bytecode to Cranelift IR
            Self::compile_bytecode(&mut builder, chunk, registers_ptr, pointer_type)?;
            
            builder.finalize();
        }

        // Compile to machine code
        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| format!("Failed to define function: {}", e))?;

        self.module.clear_context(&mut self.ctx);
        self.module.finalize_definitions()
            .map_err(|e| format!("Failed to finalize: {}", e))?;

        let code_ptr = self.module.get_finalized_function(func_id);
        Ok(code_ptr)
    }

    /// Compile bytecode instructions to Cranelift IR
    fn compile_bytecode(
        builder: &mut FunctionBuilder,
        chunk: &Chunk,
        registers_ptr: cranelift::prelude::Value,
        pointer_type: types::Type,
    ) -> Result<(), String> {
        // Track if we've emitted a terminator for the current block
        let mut terminated = false;
        
        // For simplicity, we compile linear code without complex CFG
        // A full implementation would handle control flow
        for instr in chunk.code.iter() {
            if terminated {
                // Skip unreachable code
                continue;
            }
            
            match instr.opcode {
                OpCode::LoadInt => {
                    let imm_val = instr.b as i8 as i32;
                    let value = crate::value::Value::int(imm_val);
                    let val = builder.ins().iconst(types::I64, value.bits() as i64);
                    Self::store_register(builder, registers_ptr, instr.a, val, pointer_type);
                }
                
                OpCode::LoadConst => {
                    let const_idx = instr.imm16() as usize;
                    if let Some(constant) = chunk.constants.get(const_idx) {
                        let constant: crate::value::Value = *constant;
                        let val = builder.ins().iconst(types::I64, constant.bits() as i64);
                        Self::store_register(builder, registers_ptr, instr.a, val, pointer_type);
                    }
                }

                OpCode::LoadNull => {
                    let val = builder.ins().iconst(types::I64, crate::value::Value::null().bits() as i64);
                    Self::store_register(builder, registers_ptr, instr.a, val, pointer_type);
                }

                OpCode::LoadTrue => {
                    let val = builder.ins().iconst(types::I64, crate::value::Value::bool(true).bits() as i64);
                    Self::store_register(builder, registers_ptr, instr.a, val, pointer_type);
                }

                OpCode::LoadFalse => {
                    let val = builder.ins().iconst(types::I64, crate::value::Value::bool(false).bits() as i64);
                    Self::store_register(builder, registers_ptr, instr.a, val, pointer_type);
                }

                OpCode::LoadLocal => {
                    let val = Self::load_register(builder, registers_ptr, instr.b, pointer_type);
                    Self::store_register(builder, registers_ptr, instr.a, val, pointer_type);
                }

                OpCode::StoreLocal => {
                    let val = Self::load_register(builder, registers_ptr, instr.b, pointer_type);
                    Self::store_register(builder, registers_ptr, instr.a, val, pointer_type);
                }

                OpCode::Add => {
                    // Optimistic integer addition
                    let a = Self::load_register(builder, registers_ptr, instr.b, pointer_type);
                    let b = Self::load_register(builder, registers_ptr, instr.c, pointer_type);
                    
                    let mask = builder.ins().iconst(types::I64, 0xFFFFFFFF);
                    let a_int = builder.ins().band(a, mask);
                    let b_int = builder.ins().band(b, mask);
                    let result = builder.ins().iadd(a_int, b_int);
                    
                    let tag = builder.ins().iconst(types::I64, crate::value::QNAN as i64);
                    let int_tag = builder.ins().iconst(types::I64, crate::value::TAG_INT as i64);
                    let tagged = builder.ins().bor(tag, int_tag);
                    let final_val = builder.ins().bor(tagged, result);
                    
                    Self::store_register(builder, registers_ptr, instr.a, final_val, pointer_type);
                }

                OpCode::Sub => {
                    let a = Self::load_register(builder, registers_ptr, instr.b, pointer_type);
                    let b = Self::load_register(builder, registers_ptr, instr.c, pointer_type);
                    let mask = builder.ins().iconst(types::I64, 0xFFFFFFFF);
                    let a_int = builder.ins().band(a, mask);
                    let b_int = builder.ins().band(b, mask);
                    let result = builder.ins().isub(a_int, b_int);
                    let tag = builder.ins().iconst(types::I64, crate::value::QNAN as i64);
                    let int_tag = builder.ins().iconst(types::I64, crate::value::TAG_INT as i64);
                    let tagged = builder.ins().bor(tag, int_tag);
                    let final_val = builder.ins().bor(tagged, result);
                    Self::store_register(builder, registers_ptr, instr.a, final_val, pointer_type);
                }

                OpCode::Mul => {
                    let a = Self::load_register(builder, registers_ptr, instr.b, pointer_type);
                    let b = Self::load_register(builder, registers_ptr, instr.c, pointer_type);
                    let mask = builder.ins().iconst(types::I64, 0xFFFFFFFF);
                    let a_int = builder.ins().band(a, mask);
                    let b_int = builder.ins().band(b, mask);
                    let result = builder.ins().imul(a_int, b_int);
                    let tag = builder.ins().iconst(types::I64, crate::value::QNAN as i64);
                    let int_tag = builder.ins().iconst(types::I64, crate::value::TAG_INT as i64);
                    let tagged = builder.ins().bor(tag, int_tag);
                    let final_val = builder.ins().bor(tagged, result);
                    Self::store_register(builder, registers_ptr, instr.a, final_val, pointer_type);
                }

                OpCode::Return => {
                    let val = Self::load_register(builder, registers_ptr, instr.a, pointer_type);
                    builder.ins().return_(&[val]);
                    terminated = true;
                }

                // For other opcodes, fall back to returning null (deoptimize)
                _ => {
                    // Unhandled opcode - return null for now
                    let null_val = builder.ins().iconst(types::I64, crate::value::Value::null().bits() as i64);
                    builder.ins().return_(&[null_val]);
                    terminated = true;
                }
            }
        }

        // Add final return if not terminated
        if !terminated {
            let null_val = builder.ins().iconst(types::I64, crate::value::Value::null().bits() as i64);
            builder.ins().return_(&[null_val]);
        }

        Ok(())
    }

    /// Load a value from a register
    fn load_register(
        builder: &mut FunctionBuilder,
        registers_ptr: cranelift::prelude::Value,
        reg: u8,
        pointer_type: types::Type,
    ) -> cranelift::prelude::Value {
        let offset = (reg as i32) * 8; // 8 bytes per Value
        let offset_val = builder.ins().iconst(pointer_type, offset as i64);
        let addr = builder.ins().iadd(registers_ptr, offset_val);
        builder.ins().load(types::I64, MemFlags::new(), addr, 0)
    }

    /// Store a value to a register
    fn store_register(
        builder: &mut FunctionBuilder,
        registers_ptr: cranelift::prelude::Value,
        reg: u8,
        value: cranelift::prelude::Value,
        pointer_type: types::Type,
    ) {
        let offset = (reg as i32) * 8;
        let offset_val = builder.ins().iconst(pointer_type, offset as i64);
        let addr = builder.ins().iadd(registers_ptr, offset_val);
        builder.ins().store(MemFlags::new(), value, addr, 0);
    }
}

#[cfg(feature = "jit")]
impl Default for JitCompiler {
    fn default() -> Self {
        Self::new().expect("Failed to create JIT compiler")
    }
}

/// Stub JIT compiler when feature is disabled
#[cfg(not(feature = "jit"))]
pub struct JitCompiler;

#[cfg(not(feature = "jit"))]
impl JitCompiler {
    pub fn new() -> Result<Self, String> {
        Err("JIT compilation not enabled. Enable the 'jit' feature.".to_string())
    }
}

#[cfg(not(feature = "jit"))]
impl Default for JitCompiler {
    fn default() -> Self {
        JitCompiler
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_state() {
        let mut state = JitState::new();
        
        // Test execution counting - JIT_THRESHOLD is 1000
        // First 999 executions should not trigger compilation
        for _ in 0..(JIT_THRESHOLD - 1) {
            assert!(!state.record_execution(0), "Should not trigger before threshold");
        }
        // 1000th execution triggers compilation (count == JIT_THRESHOLD)
        assert!(state.record_execution(0), "Should trigger at threshold");
        
        // After compilation is signaled, mark it as compiled to prevent re-triggering
        #[cfg(feature = "jit")]
        state.compiled_functions.insert(0, std::ptr::null());
        #[cfg(not(feature = "jit"))]
        state.compiled_functions.insert(0, ());
        
        // Subsequent executions don't trigger again
        assert!(!state.record_execution(0), "Should not trigger after compilation");
    }
}
