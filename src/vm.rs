//! Register-based Virtual Machine
//!
//! Features:
//! - 256 registers per frame
//! - Direct-threaded dispatch (via match, computed goto in future)
//! - Inline caching for property access
//! - NaN-boxed values

use crate::bytecode::*;
use crate::value::Value;
use crate::gc::GC;
use crate::error::{HateError, HateResult};
use crate::runtime::{Runtime, NativeFn};

/// Maximum call stack depth
const MAX_FRAMES: usize = 256;

/// Maximum registers per frame
const MAX_REGISTERS: usize = 256;

/// A call frame on the stack
#[derive(Clone)]
struct CallFrame {
    /// The chunk being executed
    chunk_idx: usize,
    /// Instruction pointer (index into chunk.code)
    ip: usize,
    /// Base register for this frame
    base: usize,
    /// Return register
    return_reg: u8,
}

impl CallFrame {
    fn new(chunk_idx: usize, base: usize, return_reg: u8) -> Self {
        Self {
            chunk_idx,
            ip: 0,
            base,
            return_reg,
        }
    }
}

/// Inline cache entry for property access
#[derive(Clone, Copy, Default)]
struct InlineCache {
    /// Hidden class (shape) pointer
    map: u64,
    /// Property slot index
    slot: u16,
}

/// The Hate Virtual Machine
pub struct VM {
    /// Register stack (all frames share this)
    registers: Vec<Value>,
    /// Call stack
    frames: Vec<CallFrame>,
    /// Compiled chunks
    chunks: Vec<Chunk>,
    /// Global variables
    globals: ahash::AHashMap<u32, Value>,
    /// Garbage collector
    gc: GC,
    /// Inline caches for property access
    property_caches: Vec<InlineCache>,
    /// Open upvalues list
    open_upvalues: Vec<*mut Value>,
    /// Native functions map (name hash -> function)
    natives: ahash::AHashMap<u32, (&'static str, NativeFn)>,
    /// Runtime for native functions
    runtime: Runtime,
}

impl VM {
    /// Create a new VM
    pub fn new() -> Self {
        let mut natives = ahash::AHashMap::new();
        
        // Register all native functions
        for nf in Runtime::builtins() {
            let hash = Self::hash_name(nf.name);
            natives.insert(hash, (nf.name, nf.func));
        }
        
        Self {
            registers: vec![Value::null(); MAX_REGISTERS * 16], // Pre-allocate some frames
            frames: Vec::with_capacity(MAX_FRAMES),
            chunks: Vec::new(),
            globals: ahash::AHashMap::new(),
            gc: GC::new(),
            property_caches: Vec::new(),
            open_upvalues: Vec::new(),
            natives,
            runtime: Runtime::new(),
        }
    }
    
    /// Simple hash function for function names
    fn hash_name(name: &str) -> u32 {
        let mut hash: u32 = 2166136261;
        for b in name.bytes() {
            hash ^= b as u32;
            hash = hash.wrapping_mul(16777619);
        }
        hash
    }
    
    /// Hash a name from a symbol index
    fn hash_name_from_symbol(symbol_idx: u32) -> u32 {
        use crate::intern::Symbol;
        // Get the name from the interner
        let symbol = unsafe { std::mem::transmute::<u32, Symbol>(symbol_idx) };
        Self::hash_name(symbol.as_str())
    }
    
    /// Run a chunk of bytecode
    pub fn run(&mut self, chunk: &Chunk) -> HateResult<Value> {
        // Store the chunk
        let chunk_idx = self.chunks.len();
        self.chunks.push(chunk.clone());
        
        // Create initial frame
        self.frames.push(CallFrame::new(chunk_idx, 0, 0));
        
        // Ensure we have enough property caches
        self.property_caches.resize(chunk.code.len(), InlineCache::default());
        
        // Run the interpreter loop
        self.interpret()
    }
    
    /// Main interpreter loop
    fn interpret(&mut self) -> HateResult<Value> {
        loop {
            // Get current frame
            let frame = match self.frames.last_mut() {
                Some(f) => f,
                None => return Ok(Value::null()),
            };
            
            let chunk = &self.chunks[frame.chunk_idx];
            
            // Check if we've reached the end
            if frame.ip >= chunk.code.len() {
                return Ok(self.reg(0));
            }
            
            let instruction = chunk.code[frame.ip];
            frame.ip += 1;
            
            // Dispatch
            match instruction.opcode {
                // ==================== Constants & Variables ====================
                OpCode::LoadConst => {
                    let const_idx = instruction.imm16() as usize;
                    let value = chunk.constants[const_idx];
                    self.set_reg(instruction.a, value);
                }
                
                OpCode::LoadNull => {
                    self.set_reg(instruction.a, Value::null());
                }
                
                OpCode::LoadTrue => {
                    self.set_reg(instruction.a, Value::bool(true));
                }
                
                OpCode::LoadFalse => {
                    self.set_reg(instruction.a, Value::bool(false));
                }
                
                OpCode::LoadInt => {
                    let value = instruction.b as i8 as i32;
                    self.set_reg(instruction.a, Value::int(value));
                }
                
                OpCode::LoadLocal => {
                    let value = self.reg(instruction.b);
                    self.set_reg(instruction.a, value);
                }
                
                OpCode::StoreLocal => {
                    let value = self.reg(instruction.b);
                    self.set_reg(instruction.a, value);
                }
                
                OpCode::LoadUpvalue => {
                    // TODO: Implement upvalue loading
                    self.set_reg(instruction.a, Value::null());
                }
                
                OpCode::StoreUpvalue => {
                    // TODO: Implement upvalue storing
                }
                
                OpCode::LoadGlobal => {
                    let name_idx = instruction.imm16() as u32;
                    // First check if it's a native function
                    if let Some(&(name, _)) = self.natives.get(&Self::hash_name_from_symbol(name_idx)) {
                        // Store a special native function marker
                        // We use the name index as the value (encoded specially)
                        self.set_reg(instruction.a, Value::native_fn(name_idx));
                    } else if let Some(value) = self.globals.get(&name_idx).copied() {
                        self.set_reg(instruction.a, value);
                    } else {
                        self.set_reg(instruction.a, Value::null());
                    }
                }
                
                OpCode::StoreGlobal => {
                    let name_idx = instruction.imm16() as u32;
                    let value = self.reg(instruction.a);
                    self.globals.insert(name_idx, value);
                }
                
                // ==================== Arithmetic ====================
                OpCode::Add | OpCode::IAdd => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    let result = a.add(b).ok_or(HateError::InvalidOperator {
                        op: "+".to_string(),
                        left: a.type_name().to_string(),
                        right: b.type_name().to_string(),
                        span: crate::error::Span::empty(),
                    })?;
                    self.set_reg(instruction.a, result);
                }
                
                OpCode::Sub | OpCode::ISub => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    let result = a.sub(b).ok_or(HateError::InvalidOperator {
                        op: "-".to_string(),
                        left: a.type_name().to_string(),
                        right: b.type_name().to_string(),
                        span: crate::error::Span::empty(),
                    })?;
                    self.set_reg(instruction.a, result);
                }
                
                OpCode::Mul | OpCode::IMul => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    let result = a.mul(b).ok_or(HateError::InvalidOperator {
                        op: "*".to_string(),
                        left: a.type_name().to_string(),
                        right: b.type_name().to_string(),
                        span: crate::error::Span::empty(),
                    })?;
                    self.set_reg(instruction.a, result);
                }
                
                OpCode::Div | OpCode::IDiv => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    
                    // Check for division by zero
                    if b.is_int() && b.as_int_unchecked() == 0 {
                        return Err(HateError::DivisionByZero {
                            span: crate::error::Span::empty(),
                        });
                    }
                    
                    let result = a.div(b).ok_or(HateError::InvalidOperator {
                        op: "/".to_string(),
                        left: a.type_name().to_string(),
                        right: b.type_name().to_string(),
                        span: crate::error::Span::empty(),
                    })?;
                    self.set_reg(instruction.a, result);
                }
                
                OpCode::Mod => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    let result = a.modulo(b).ok_or(HateError::InvalidOperator {
                        op: "%".to_string(),
                        left: a.type_name().to_string(),
                        right: b.type_name().to_string(),
                        span: crate::error::Span::empty(),
                    })?;
                    self.set_reg(instruction.a, result);
                }
                
                OpCode::Neg => {
                    let a = self.reg(instruction.b);
                    let result = a.neg().ok_or(HateError::InvalidOperator {
                        op: "-".to_string(),
                        left: a.type_name().to_string(),
                        right: "".to_string(),
                        span: crate::error::Span::empty(),
                    })?;
                    self.set_reg(instruction.a, result);
                }
                
                // ==================== Bitwise ====================
                OpCode::BitAnd => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    let result = a.bit_and(b).unwrap_or(Value::int(0));
                    self.set_reg(instruction.a, result);
                }
                
                OpCode::BitOr => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    let result = a.bit_or(b).unwrap_or(Value::int(0));
                    self.set_reg(instruction.a, result);
                }
                
                OpCode::BitXor => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    let result = a.bit_xor(b).unwrap_or(Value::int(0));
                    self.set_reg(instruction.a, result);
                }
                
                OpCode::BitNot => {
                    let a = self.reg(instruction.b);
                    let result = a.bit_not().unwrap_or(Value::int(0));
                    self.set_reg(instruction.a, result);
                }
                
                OpCode::Shl => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    let result = a.shl(b).unwrap_or(Value::int(0));
                    self.set_reg(instruction.a, result);
                }
                
                OpCode::Shr => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    let result = a.shr(b).unwrap_or(Value::int(0));
                    self.set_reg(instruction.a, result);
                }
                
                // ==================== Comparison ====================
                OpCode::Eq => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    let result = a.eq(b);
                    self.set_reg(instruction.a, Value::bool(result));
                }
                
                OpCode::Ne => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    let result = !a.eq(b);
                    self.set_reg(instruction.a, Value::bool(result));
                }
                
                OpCode::Lt => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    let result = a.lt(b).unwrap_or(false);
                    self.set_reg(instruction.a, Value::bool(result));
                }
                
                OpCode::Le => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    let result = a.le(b).unwrap_or(false);
                    self.set_reg(instruction.a, Value::bool(result));
                }
                
                OpCode::Gt => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    let result = a.gt(b).unwrap_or(false);
                    self.set_reg(instruction.a, Value::bool(result));
                }
                
                OpCode::Ge => {
                    let a = self.reg(instruction.b);
                    let b = self.reg(instruction.c);
                    let result = a.ge(b).unwrap_or(false);
                    self.set_reg(instruction.a, Value::bool(result));
                }
                
                // ==================== Logical ====================
                OpCode::Not => {
                    let a = self.reg(instruction.b);
                    let result = !a.is_truthy();
                    self.set_reg(instruction.a, Value::bool(result));
                }
                
                // ==================== Objects ====================
                OpCode::NewObject => {
                    // TODO: Create new object via GC
                    self.set_reg(instruction.a, Value::null());
                }
                
                OpCode::GetProp => {
                    // TODO: Property access with inline caching
                    self.set_reg(instruction.a, Value::null());
                }
                
                OpCode::SetProp => {
                    // TODO: Property setting with inline caching
                }
                
                OpCode::DelProp => {
                    // TODO: Property deletion
                }
                
                OpCode::HasProp => {
                    // TODO: Property existence check
                    self.set_reg(instruction.a, Value::bool(false));
                }
                
                // ==================== Arrays ====================
                OpCode::NewArray => {
                    // TODO: Create new array via GC
                    self.set_reg(instruction.a, Value::null());
                }
                
                OpCode::GetIndex => {
                    // TODO: Array indexing
                    self.set_reg(instruction.a, Value::null());
                }
                
                OpCode::SetIndex => {
                    // TODO: Array element setting
                }
                
                OpCode::ArrayLen => {
                    // TODO: Get array length
                    self.set_reg(instruction.a, Value::int(0));
                }
                
                OpCode::ArrayPush => {
                    // TODO: Push to array
                }
                
                OpCode::ArrayPop => {
                    // TODO: Pop from array
                    self.set_reg(instruction.a, Value::null());
                }
                
                // ==================== Functions ====================
                OpCode::Call => {
                    let callee = self.reg(instruction.b);
                    let argc = instruction.c as usize;
                    
                    // Check if it's a native function
                    if let Some(symbol_idx) = callee.as_native_fn_index() {
                        let hash = Self::hash_name_from_symbol(symbol_idx);
                        if let Some(&(_, func)) = self.natives.get(&hash) {
                            // Collect arguments
                            let frame_base = self.frame_base();
                            let arg_start = instruction.b as usize + 1; // Args start after callee register
                            let mut args = Vec::with_capacity(argc);
                            for i in 0..argc {
                                args.push(self.registers[frame_base + arg_start + i]);
                            }
                            
                            // Call the native function
                            let result = func(&mut self.runtime, &args)?;
                            self.set_reg(instruction.a, result);
                        } else {
                            // Unknown native function
                            self.set_reg(instruction.a, Value::null());
                        }
                    } else {
                        // TODO: Implement user-defined function calls
                        self.set_reg(instruction.a, Value::null());
                    }
                }
                
                OpCode::CallMethod => {
                    // TODO: Method calls
                    self.set_reg(instruction.a, Value::null());
                }
                
                OpCode::Return => {
                    let result = self.reg(instruction.a);
                    
                    // Pop frame
                    if let Some(frame) = self.frames.pop() {
                        if self.frames.is_empty() {
                            // Top-level return
                            return Ok(result);
                        } else {
                            // Store result in caller's return register
                            self.set_reg(frame.return_reg, result);
                        }
                    } else {
                        return Ok(result);
                    }
                }
                
                OpCode::Closure => {
                    // TODO: Create closure
                    self.set_reg(instruction.a, Value::null());
                }
                
                OpCode::CloseUpvalue => {
                    // TODO: Close upvalue
                }
                
                // ==================== Control Flow ====================
                OpCode::Jump => {
                    let offset = instruction.simm16() as isize;
                    let frame = self.frames.last_mut().unwrap();
                    frame.ip = (frame.ip as isize + offset) as usize;
                }
                
                OpCode::JumpIfTrue => {
                    let cond = self.reg(instruction.a);
                    if cond.is_truthy() {
                        let offset = instruction.simm16() as isize;
                        let frame = self.frames.last_mut().unwrap();
                        frame.ip = (frame.ip as isize + offset) as usize;
                    }
                }
                
                OpCode::JumpIfFalse => {
                    let cond = self.reg(instruction.a);
                    if !cond.is_truthy() {
                        let offset = instruction.simm16() as isize;
                        let frame = self.frames.last_mut().unwrap();
                        frame.ip = (frame.ip as isize + offset) as usize;
                    }
                }
                
                OpCode::JumpIfNull => {
                    let value = self.reg(instruction.a);
                    if value.is_null() {
                        let offset = instruction.simm16() as isize;
                        let frame = self.frames.last_mut().unwrap();
                        frame.ip = (frame.ip as isize + offset) as usize;
                    }
                }
                
                OpCode::JumpIfNotNull => {
                    let value = self.reg(instruction.a);
                    if !value.is_null() {
                        let offset = instruction.simm16() as isize;
                        let frame = self.frames.last_mut().unwrap();
                        frame.ip = (frame.ip as isize + offset) as usize;
                    }
                }
                
                OpCode::Loop => {
                    let offset = instruction.simm16() as isize;
                    let frame = self.frames.last_mut().unwrap();
                    frame.ip = (frame.ip as isize - offset) as usize;
                }
                
                // ==================== Type Operations ====================
                OpCode::TypeOf => {
                    let value = self.reg(instruction.b);
                    // TODO: Return type string
                    self.set_reg(instruction.a, Value::null());
                }
                
                OpCode::IsNull => {
                    let value = self.reg(instruction.b);
                    self.set_reg(instruction.a, Value::bool(value.is_null()));
                }
                
                OpCode::ToInt => {
                    let value = self.reg(instruction.b);
                    if value.is_int() {
                        self.set_reg(instruction.a, value);
                    } else if value.is_float() {
                        let f = value.as_float_unchecked();
                        self.set_reg(instruction.a, Value::int(f as i32));
                    } else {
                        self.set_reg(instruction.a, Value::int(0));
                    }
                }
                
                OpCode::ToFloat => {
                    let value = self.reg(instruction.b);
                    if value.is_float() {
                        self.set_reg(instruction.a, value);
                    } else if value.is_int() {
                        let i = value.as_int_unchecked();
                        self.set_reg(instruction.a, Value::float(i as f64));
                    } else {
                        self.set_reg(instruction.a, Value::float(0.0));
                    }
                }
                
                OpCode::ToString => {
                    // TODO: Convert to string
                    self.set_reg(instruction.a, Value::null());
                }
                
                OpCode::ToBool => {
                    let value = self.reg(instruction.b);
                    self.set_reg(instruction.a, Value::bool(value.is_truthy()));
                }
                
                // ==================== Iterators ====================
                OpCode::NewIter => {
                    // TODO: Create iterator
                    self.set_reg(instruction.a, Value::null());
                }
                
                OpCode::IterNext => {
                    // TODO: Iterator next
                    self.set_reg(instruction.a, Value::null());
                    self.set_reg(instruction.c, Value::bool(true)); // Done
                }
                
                OpCode::ForIn => {
                    // TODO: For-in iteration
                }
                
                // ==================== Pattern Matching ====================
                OpCode::Match => {
                    // TODO: Pattern matching dispatch
                }
                
                OpCode::DestructArray => {
                    // TODO: Array destructuring
                }
                
                OpCode::DestructObject => {
                    // TODO: Object destructuring
                }
                
                // ==================== Special ====================
                OpCode::Spread => {
                    // TODO: Spread operator
                }
                
                OpCode::NullCoalesce => {
                    let left = self.reg(instruction.b);
                    let right = self.reg(instruction.c);
                    let result = if left.is_null() { right } else { left };
                    self.set_reg(instruction.a, result);
                }
                
                OpCode::OptionalChain => {
                    // TODO: Optional chaining
                    self.set_reg(instruction.a, Value::null());
                }
                
                // ==================== Concurrency ====================
                OpCode::Await => {
                    // TODO: Await implementation
                    self.set_reg(instruction.a, Value::null());
                }
                
                OpCode::Yield => {
                    // TODO: Generator yield
                }
                
                // ==================== Debugging ====================
                OpCode::Print => {
                    let value = self.reg(instruction.a);
                    println!("{}", value);
                }
                
                OpCode::Assert => {
                    let value = self.reg(instruction.a);
                    if !value.is_truthy() {
                        return Err(HateError::Internal("Assertion failed".to_string()));
                    }
                }
                
                OpCode::Breakpoint => {
                    // TODO: Debugger integration
                }
                
                // ==================== GC ====================
                OpCode::WriteBarrier => {
                    // TODO: GC write barrier
                }
                
                // ==================== Module ====================
                OpCode::Import => {
                    // TODO: Module import
                }
                
                OpCode::Export => {
                    // TODO: Module export
                }
                
                // ==================== Misc ====================
                OpCode::Nop => {
                    // Do nothing
                }
                
                OpCode::Halt => {
                    return Ok(self.reg(0));
                }
                
                // ==================== New Opcodes (to be implemented) ====================
                OpCode::TryBegin | OpCode::TryEnd | OpCode::Catch | 
                OpCode::Throw | OpCode::Finally => {
                    // TODO: Exception handling
                }
                
                OpCode::MatchBegin | OpCode::MatchArm | OpCode::MatchGuard | OpCode::MatchEnd => {
                    // TODO: Enhanced pattern matching
                }
                
                OpCode::RangeExclusive | OpCode::RangeInclusive | OpCode::RangeStep => {
                    // TODO: Range operations
                }
                
                OpCode::SpreadObject | OpCode::SpreadArray | 
                OpCode::DestructObject2 | OpCode::DestructArray2 => {
                    // TODO: Spread and destructuring
                }
                
                OpCode::NewClass | OpCode::GetSuper | OpCode::InvokeSuper |
                OpCode::Inherit | OpCode::GetMethod | OpCode::BindMethod => {
                    // TODO: Class operations
                }
                
                OpCode::AsyncBegin | OpCode::AsyncResume | OpCode::AsyncSuspend |
                OpCode::NewPromise | OpCode::ResolvePromise | OpCode::RejectPromise => {
                    // TODO: Async operations
                }
            }
            
            // Periodically check for GC
            self.gc.collect();
        }
    }
    
    // ==================== Register Access ====================
    
    /// Get the current frame's base register index
    fn frame_base(&self) -> usize {
        self.frames.last().map(|f| f.base).unwrap_or(0)
    }
    
    /// Get a register value
    #[inline(always)]
    fn reg(&self, idx: u8) -> Value {
        let base = self.frame_base();
        self.registers[base + idx as usize]
    }
    
    /// Set a register value
    #[inline(always)]
    fn set_reg(&mut self, idx: u8, value: Value) {
        let base = self.frame_base();
        self.registers[base + idx as usize] = value;
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::compiler::Compiler;
    
    fn run(source: &str) -> Value {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        let mut compiler = Compiler::new();
        let chunk = compiler.compile(&ast).unwrap();
        let mut vm = VM::new();
        vm.run(&chunk).unwrap()
    }
    
    #[test]
    fn test_literal() {
        // The VM should evaluate literals
        let result = run("42;");
        // Result is null because expression statements don't return
        assert!(result.is_null());
    }
    
    #[test]
    fn test_arithmetic() {
        // Simple arithmetic
        let mut chunk = Chunk::new();
        chunk.write(Instruction::with_ab(OpCode::LoadInt, 0, 10), 1);
        chunk.write(Instruction::with_ab(OpCode::LoadInt, 1, 20), 1);
        chunk.write(Instruction::with_abc(OpCode::Add, 2, 0, 1), 1);
        chunk.write(Instruction::with_a(OpCode::Return, 2), 1);
        
        let mut vm = VM::new();
        let result = vm.run(&chunk).unwrap();
        assert_eq!(result.as_int(), Some(30));
    }
    
    #[test]
    fn test_comparison() {
        let mut chunk = Chunk::new();
        chunk.write(Instruction::with_ab(OpCode::LoadInt, 0, 10), 1);
        chunk.write(Instruction::with_ab(OpCode::LoadInt, 1, 20), 1);
        chunk.write(Instruction::with_abc(OpCode::Lt, 2, 0, 1), 1);
        chunk.write(Instruction::with_a(OpCode::Return, 2), 1);
        
        let mut vm = VM::new();
        let result = vm.run(&chunk).unwrap();
        assert_eq!(result.as_bool(), Some(true));
    }
    
    #[test]
    fn test_jump() {
        let mut chunk = Chunk::new();
        chunk.write(Instruction::with_ab(OpCode::LoadInt, 0, 10), 1);
        chunk.write(Instruction::with_a_imm16(OpCode::Jump, 0, 1), 1); // Skip next
        chunk.write(Instruction::with_ab(OpCode::LoadInt, 0, 99), 1); // Skipped
        chunk.write(Instruction::with_a(OpCode::Return, 0), 1);
        
        let mut vm = VM::new();
        let result = vm.run(&chunk).unwrap();
        assert_eq!(result.as_int(), Some(10)); // Should be 10, not 99
    }
    
    #[test]
    fn test_conditional_jump() {
        let mut chunk = Chunk::new();
        chunk.write(Instruction::with_a(OpCode::LoadTrue, 0), 1);
        chunk.write(Instruction::with_a_imm16(OpCode::JumpIfFalse, 0, 1), 1);
        chunk.write(Instruction::with_ab(OpCode::LoadInt, 1, 42), 1);
        chunk.write(Instruction::with_a(OpCode::Return, 1), 1);
        
        let mut vm = VM::new();
        let result = vm.run(&chunk).unwrap();
        assert_eq!(result.as_int(), Some(42));
    }
}
