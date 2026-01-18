//! Bytecode compiler
//!
//! Transforms AST into bytecode with:
//! - Constant folding
//! - Register allocation
//! - Upvalue capture for closures

use crate::ast::*;
use crate::bytecode::*;
use crate::value::Value;
use crate::intern::Symbol;
use crate::error::{HateError, HateResult};

/// Local variable information
#[derive(Debug, Clone)]
struct Local {
    name: Symbol,
    depth: usize,
    mutable: bool,
    is_captured: bool,
    register: u8,  // Register where this local is stored
}

/// Upvalue during compilation
#[derive(Debug, Clone, Copy)]
struct CompilerUpvalue {
    index: u8,
    is_local: bool,
}

/// Loop context for break/continue
#[derive(Debug, Clone)]
struct LoopContext {
    start: usize,
    break_jumps: Vec<usize>,
}

/// Compiler state for a single function/closure
struct CompilerScope {
    function: Function,
    locals: Vec<Local>,
    upvalues: Vec<CompilerUpvalue>,
    scope_depth: usize,
    register_count: u8,
    max_registers: u8,
}

impl CompilerScope {
    fn new(name: Option<Symbol>) -> Self {
        let mut scope = Self {
            function: Function::new(),
            locals: Vec::new(),
            upvalues: Vec::new(),
            scope_depth: 0,
            register_count: 0,
            max_registers: 0,
        };
        scope.function.name = name;
        scope
    }
    
    fn alloc_register(&mut self) -> HateResult<u8> {
        if self.register_count >= 250 {
            return Err(HateError::TooManyLocals);
        }
        let reg = self.register_count;
        self.register_count += 1;
        self.max_registers = self.max_registers.max(self.register_count);
        Ok(reg)
    }
    
    fn free_register(&mut self) {
        if self.register_count > 0 {
            self.register_count -= 1;
        }
    }
    
    fn free_registers(&mut self, count: u8) {
        self.register_count = self.register_count.saturating_sub(count);
    }
}

/// Bytecode compiler
pub struct Compiler {
    /// Stack of compiler scopes (for nested functions)
    scopes: Vec<CompilerScope>,
    /// Loop stack for break/continue
    loops: Vec<LoopContext>,
}

impl Compiler {
    /// Create a new compiler
    pub fn new() -> Self {
        Self {
            scopes: vec![CompilerScope::new(None)],
            loops: Vec::new(),
        }
    }
    
    /// Compile a program to bytecode
    pub fn compile(&mut self, program: &Program) -> HateResult<Chunk> {
        for stmt in &program.statements {
            self.compile_stmt(stmt)?;
        }
        
        // Emit implicit return
        self.emit(Instruction::with_a(OpCode::LoadNull, 0), 0);
        self.emit(Instruction::with_a(OpCode::Return, 0), 0);
        
        let scope = self.scopes.pop().unwrap();
        let mut chunk = scope.function.chunk;
        chunk.max_registers = scope.max_registers;
        Ok(chunk)
    }
    
    /// Compile a statement
    fn compile_stmt(&mut self, stmt: &Stmt) -> HateResult<()> {
        match stmt {
            Stmt::Expression { expr, .. } => {
                let reg = self.compile_expr(expr)?;
                self.current_scope_mut().free_register();
                Ok(())
            }
            
            Stmt::Let { name, mutable, value, span, .. } => {
                let reg = self.current_scope_mut().alloc_register()?;
                
                if let Some(value) = value {
                    let value_reg = self.compile_expr(value)?;
                    if value_reg != reg {
                        // Move value to the local's register
                        self.emit(Instruction::with_ab(OpCode::LoadLocal, reg, value_reg), span.line as u32);
                    }
                } else {
                    self.emit(Instruction::with_a(OpCode::LoadNull, reg), span.line as u32);
                }
                
                self.add_local(*name, *mutable, reg);
                Ok(())
            }
            
            Stmt::Function { name, params, body, is_async, span, .. } => {
                self.compile_function(*name, params, body, span.line as u32)?;
                Ok(())
            }
            
            Stmt::Return { value, span } => {
                let reg = if let Some(value) = value {
                    self.compile_expr(value)?
                } else {
                    let reg = self.current_scope_mut().alloc_register()?;
                    self.emit(Instruction::with_a(OpCode::LoadNull, reg), span.line as u32);
                    reg
                };
                self.emit(Instruction::with_a(OpCode::Return, reg), span.line as u32);
                self.current_scope_mut().free_register();
                Ok(())
            }
            
            Stmt::If { condition, then_branch, else_branch, span } => {
                let cond_reg = self.compile_expr(condition)?;
                
                // Jump to else if false
                let else_jump = self.emit_jump(OpCode::JumpIfFalse, cond_reg, span.line as u32);
                self.current_scope_mut().free_register();
                
                // Compile then branch
                self.compile_stmt(then_branch)?;
                
                if let Some(else_branch) = else_branch {
                    // Jump over else
                    let end_jump = self.emit_jump(OpCode::Jump, 0, span.line as u32);
                    
                    // Patch else jump
                    self.patch_jump(else_jump);
                    
                    // Compile else branch
                    self.compile_stmt(else_branch)?;
                    
                    // Patch end jump
                    self.patch_jump(end_jump);
                } else {
                    self.patch_jump(else_jump);
                }
                
                Ok(())
            }
            
            Stmt::While { condition, body, span } => {
                let loop_start = self.current_chunk().len();
                
                // Push loop context
                self.loops.push(LoopContext {
                    start: loop_start,
                    break_jumps: Vec::new(),
                });
                
                // Compile condition
                let cond_reg = self.compile_expr(condition)?;
                let exit_jump = self.emit_jump(OpCode::JumpIfFalse, cond_reg, span.line as u32);
                self.current_scope_mut().free_register();
                
                // Compile body
                self.compile_stmt(body)?;
                
                // Loop back
                self.emit_loop(loop_start, span.line as u32)?;
                
                // Patch exit
                self.patch_jump(exit_jump);
                
                // Patch breaks
                let loop_ctx = self.loops.pop().unwrap();
                for jump in loop_ctx.break_jumps {
                    self.patch_jump(jump);
                }
                
                Ok(())
            }
            
            Stmt::For { variable, iterable, body, span } => {
                // Create iterator
                let iter_reg = self.compile_expr(iterable)?;
                self.emit(Instruction::with_ab(OpCode::NewIter, iter_reg, iter_reg), span.line as u32);
                
                let loop_start = self.current_chunk().len();
                
                self.loops.push(LoopContext {
                    start: loop_start,
                    break_jumps: Vec::new(),
                });
                
                // Get next value
                let value_reg = self.current_scope_mut().alloc_register()?;
                let done_reg = self.current_scope_mut().alloc_register()?;
                self.emit(Instruction::with_abc(OpCode::IterNext, value_reg, iter_reg, done_reg), span.line as u32);
                
                // Exit if done
                let exit_jump = self.emit_jump(OpCode::JumpIfTrue, done_reg, span.line as u32);
                self.current_scope_mut().free_register(); // done_reg
                
                // Add loop variable
                self.add_local(*variable, false, value_reg);
                
                // Compile body
                self.begin_scope();
                self.compile_stmt(body)?;
                self.end_scope(span.line as u32);
                
                // Loop back
                self.emit_loop(loop_start, span.line as u32)?;
                
                // Patch exit
                self.patch_jump(exit_jump);
                
                // Clean up
                self.current_scope_mut().free_registers(2); // iter_reg, value_reg
                
                let loop_ctx = self.loops.pop().unwrap();
                for jump in loop_ctx.break_jumps {
                    self.patch_jump(jump);
                }
                
                Ok(())
            }
            
            Stmt::Block { statements, span } => {
                self.begin_scope();
                for stmt in statements {
                    self.compile_stmt(stmt)?;
                }
                self.end_scope(span.line as u32);
                Ok(())
            }
            
            Stmt::Break { span } => {
                if self.loops.is_empty() {
                    return Err(HateError::BreakOutsideLoop { span: *span });
                }
                let jump = self.emit_jump(OpCode::Jump, 0, span.line as u32);
                self.loops.last_mut().unwrap().break_jumps.push(jump);
                Ok(())
            }
            
            Stmt::Continue { span } => {
                if self.loops.is_empty() {
                    return Err(HateError::ContinueOutsideLoop { span: *span });
                }
                let loop_start = self.loops.last().unwrap().start;
                self.emit_loop(loop_start, span.line as u32)?;
                Ok(())
            }
            
            Stmt::Import { items, from, span } => {
                // Emit import instruction
                let from_const = self.add_constant(Value::null()); // Placeholder
                self.emit(Instruction::with_a_imm16(OpCode::Import, 0, from_const), span.line as u32);
                Ok(())
            }
            
            Stmt::Export { statement, span } => {
                self.compile_stmt(statement)?;
                Ok(())
            }
            
            Stmt::Class { name, fields, methods, span } => {
                // Classes are compiled to constructor functions
                // This is a simplified implementation
                let reg = self.current_scope_mut().alloc_register()?;
                self.emit(Instruction::with_a(OpCode::NewObject, reg), span.line as u32);
                self.add_local(*name, false, reg);
                Ok(())
            }
            
            Stmt::Impl { type_name, methods, span, .. } => {
                // Compile methods and attach to type
                for method in methods {
                    self.compile_stmt(method)?;
                }
                Ok(())
            }
        }
    }
    
    /// Compile an expression, returning the register containing the result
    fn compile_expr(&mut self, expr: &Expr) -> HateResult<u8> {
        match expr {
            Expr::Literal { value, span } => {
                let reg = self.current_scope_mut().alloc_register()?;
                match value {
                    Literal::Null => {
                        self.emit(Instruction::with_a(OpCode::LoadNull, reg), span.line as u32);
                    }
                    Literal::Bool(true) => {
                        self.emit(Instruction::with_a(OpCode::LoadTrue, reg), span.line as u32);
                    }
                    Literal::Bool(false) => {
                        self.emit(Instruction::with_a(OpCode::LoadFalse, reg), span.line as u32);
                    }
                    Literal::Integer(i) => {
                        if *i >= -128 && *i <= 127 {
                            self.emit(Instruction::with_ab(OpCode::LoadInt, reg, *i as u8), span.line as u32);
                        } else {
                            let const_idx = self.add_constant(Value::int64(*i));
                            self.emit(Instruction::with_a_imm16(OpCode::LoadConst, reg, const_idx), span.line as u32);
                        }
                    }
                    Literal::Float(f) => {
                        let const_idx = self.add_constant(Value::float(*f));
                        self.emit(Instruction::with_a_imm16(OpCode::LoadConst, reg, const_idx), span.line as u32);
                    }
                    Literal::String(s) => {
                        // Store string as interned symbol index
                        let const_idx = self.add_constant(Value::string(s.index()));
                        self.emit(Instruction::with_a_imm16(OpCode::LoadConst, reg, const_idx), span.line as u32);
                    }
                }
                Ok(reg)
            }
            
            Expr::Identifier { name, span } => {
                // Look up variable
                if let Some((slot, _)) = self.resolve_local(*name) {
                    let reg = self.current_scope_mut().alloc_register()?;
                    self.emit(Instruction::with_ab(OpCode::LoadLocal, reg, slot), span.line as u32);
                    Ok(reg)
                } else if let Some(upvalue_idx) = self.resolve_upvalue(*name) {
                    let reg = self.current_scope_mut().alloc_register()?;
                    self.emit(Instruction::with_ab(OpCode::LoadUpvalue, reg, upvalue_idx), span.line as u32);
                    Ok(reg)
                } else {
                    // Store the symbol index as a value for global lookup
                    let reg = self.current_scope_mut().alloc_register()?;
                    let const_idx = name.index() as u16; // Use symbol index directly
                    self.emit(Instruction::with_a_imm16(OpCode::LoadGlobal, reg, const_idx), span.line as u32);
                    Ok(reg)
                }
            }
            
            Expr::Binary { left, operator, right, span } => {
                let left_reg = self.compile_expr(left)?;
                let right_reg = self.compile_expr(right)?;
                
                let result_reg = self.current_scope_mut().alloc_register()?;
                
                let opcode = match operator {
                    BinaryOp::Add => OpCode::Add,
                    BinaryOp::Sub => OpCode::Sub,
                    BinaryOp::Mul => OpCode::Mul,
                    BinaryOp::Div => OpCode::Div,
                    BinaryOp::Mod => OpCode::Mod,
                    BinaryOp::Eq => OpCode::Eq,
                    BinaryOp::Ne => OpCode::Ne,
                    BinaryOp::Lt => OpCode::Lt,
                    BinaryOp::Le => OpCode::Le,
                    BinaryOp::Gt => OpCode::Gt,
                    BinaryOp::Ge => OpCode::Ge,
                    BinaryOp::BitAnd => OpCode::BitAnd,
                    BinaryOp::BitOr => OpCode::BitOr,
                    BinaryOp::BitXor => OpCode::BitXor,
                    BinaryOp::Shl => OpCode::Shl,
                    BinaryOp::Shr => OpCode::Shr,
                };
                
                self.emit(Instruction::with_abc(opcode, result_reg, left_reg, right_reg), span.line as u32);
                
                // Free operand registers
                self.current_scope_mut().free_registers(2);
                
                Ok(result_reg)
            }
            
            Expr::Unary { operator, operand, span } => {
                let operand_reg = self.compile_expr(operand)?;
                let result_reg = self.current_scope_mut().alloc_register()?;
                
                let opcode = match operator {
                    UnaryOp::Neg => OpCode::Neg,
                    UnaryOp::Not => OpCode::Not,
                    UnaryOp::BitNot => OpCode::BitNot,
                };
                
                self.emit(Instruction::with_ab(opcode, result_reg, operand_reg), span.line as u32);
                self.current_scope_mut().free_register();
                
                Ok(result_reg)
            }
            
            Expr::Logical { left, operator, right, span } => {
                let left_reg = self.compile_expr(left)?;
                
                // Short-circuit evaluation
                let jump_opcode = match operator {
                    LogicalOp::And => OpCode::JumpIfFalse,
                    LogicalOp::Or => OpCode::JumpIfTrue,
                };
                
                let end_jump = self.emit_jump(jump_opcode, left_reg, span.line as u32);
                self.current_scope_mut().free_register();
                
                // Evaluate right side
                let right_reg = self.compile_expr(right)?;
                
                self.patch_jump(end_jump);
                
                Ok(right_reg)
            }
            
            Expr::Assign { target, value, span } => {
                let value_reg = self.compile_expr(value)?;
                
                match target.as_ref() {
                    Expr::Identifier { name, .. } => {
                        if let Some((slot, mutable)) = self.resolve_local(*name) {
                            if !mutable {
                                return Err(HateError::ImmutableReassign {
                                    name: name.as_str().to_string(),
                                    span: *span,
                                });
                            }
                            self.emit(Instruction::with_ab(OpCode::StoreLocal, slot, value_reg), span.line as u32);
                        } else if let Some(upvalue_idx) = self.resolve_upvalue(*name) {
                            self.emit(Instruction::with_ab(OpCode::StoreUpvalue, upvalue_idx, value_reg), span.line as u32);
                        } else {
                            let const_idx = self.add_constant(Value::null());
                            self.emit(Instruction::with_a_imm16(OpCode::StoreGlobal, value_reg, const_idx), span.line as u32);
                        }
                    }
                    Expr::Property { object, property, .. } => {
                        let obj_reg = self.compile_expr(object)?;
                        let prop_const = self.add_constant(Value::null());
                        self.emit(Instruction::with_abc(OpCode::SetProp, obj_reg, prop_const as u8, value_reg), span.line as u32);
                        self.current_scope_mut().free_register();
                    }
                    Expr::Index { object, index, .. } => {
                        let obj_reg = self.compile_expr(object)?;
                        let idx_reg = self.compile_expr(index)?;
                        self.emit(Instruction::with_abc(OpCode::SetIndex, obj_reg, idx_reg, value_reg), span.line as u32);
                        self.current_scope_mut().free_registers(2);
                    }
                    _ => {
                        return Err(HateError::InvalidAssignment { span: *span });
                    }
                }
                
                Ok(value_reg)
            }
            
            Expr::Call { callee, arguments, span } => {
                let callee_reg = self.compile_expr(callee)?;
                
                // Compile arguments into contiguous registers after callee
                let mut arg_regs = Vec::with_capacity(arguments.len());
                for arg in arguments {
                    let arg_reg = self.compile_expr(arg)?;
                    arg_regs.push(arg_reg);
                }
                
                // Move arguments to be contiguous after callee register
                let first_arg_reg = callee_reg + 1;
                for (i, &arg_reg) in arg_regs.iter().enumerate() {
                    let target_reg = first_arg_reg + i as u8;
                    if arg_reg != target_reg {
                        self.emit(Instruction::with_ab(OpCode::LoadLocal, target_reg, arg_reg), span.line as u32);
                    }
                }
                
                let result_reg = self.current_scope_mut().alloc_register()?;
                self.emit(Instruction::with_abc(OpCode::Call, result_reg, callee_reg, arguments.len() as u8), span.line as u32);
                
                Ok(result_reg)
            }
            
            Expr::MethodCall { object, method, arguments, span } => {
                let obj_reg = self.compile_expr(object)?;
                
                // Compile arguments
                for arg in arguments {
                    self.compile_expr(arg)?;
                }
                
                let result_reg = self.current_scope_mut().alloc_register()?;
                let method_const = self.add_constant(Value::null());
                self.emit(Instruction::with_abc(OpCode::CallMethod, result_reg, obj_reg, arguments.len() as u8), span.line as u32);
                
                self.current_scope_mut().free_registers(arguments.len() as u8 + 1);
                
                Ok(result_reg)
            }
            
            Expr::Property { object, property, span } => {
                let obj_reg = self.compile_expr(object)?;
                let result_reg = self.current_scope_mut().alloc_register()?;
                
                let prop_const = self.add_constant(Value::null());
                self.emit(Instruction::with_abc(OpCode::GetProp, result_reg, obj_reg, prop_const as u8), span.line as u32);
                
                self.current_scope_mut().free_register();
                
                Ok(result_reg)
            }
            
            Expr::Index { object, index, span } => {
                let obj_reg = self.compile_expr(object)?;
                let idx_reg = self.compile_expr(index)?;
                let result_reg = self.current_scope_mut().alloc_register()?;
                
                self.emit(Instruction::with_abc(OpCode::GetIndex, result_reg, obj_reg, idx_reg), span.line as u32);
                
                self.current_scope_mut().free_registers(2);
                
                Ok(result_reg)
            }
            
            Expr::Array { elements, span } => {
                let arr_reg = self.current_scope_mut().alloc_register()?;
                self.emit(Instruction::with_ab(OpCode::NewArray, arr_reg, elements.len() as u8), span.line as u32);
                
                for (i, elem) in elements.iter().enumerate() {
                    let elem_reg = self.compile_expr(elem)?;
                    let idx_reg = self.current_scope_mut().alloc_register()?;
                    self.emit(Instruction::with_ab(OpCode::LoadInt, idx_reg, i as u8), span.line as u32);
                    self.emit(Instruction::with_abc(OpCode::SetIndex, arr_reg, idx_reg, elem_reg), span.line as u32);
                    self.current_scope_mut().free_registers(2);
                }
                
                Ok(arr_reg)
            }
            
            Expr::Object { properties, span } => {
                let obj_reg = self.current_scope_mut().alloc_register()?;
                self.emit(Instruction::with_a(OpCode::NewObject, obj_reg), span.line as u32);
                
                for prop in properties {
                    let value_reg = self.compile_expr(&prop.value)?;
                    let key_const = self.add_constant(Value::null());
                    self.emit(Instruction::with_abc(OpCode::SetProp, obj_reg, key_const as u8, value_reg), span.line as u32);
                    self.current_scope_mut().free_register();
                }
                
                Ok(obj_reg)
            }
            
            Expr::Lambda { params, body, span, .. } => {
                // Create a new function scope
                self.begin_function(None, params.len() as u8)?;
                
                // Add parameters as locals
                for (i, param) in params.iter().enumerate() {
                    self.add_local(param.name, false, i as u8);
                }
                
                // Compile body
                let body_reg = self.compile_expr(body)?;
                self.emit(Instruction::with_a(OpCode::Return, body_reg), span.line as u32);
                
                // End function
                let function = self.end_function(span.line as u32);
                
                // Create closure
                let result_reg = self.current_scope_mut().alloc_register()?;
                let func_const = self.add_constant(Value::null()); // Placeholder for function
                self.emit(Instruction::with_a_imm16(OpCode::Closure, result_reg, func_const), span.line as u32);
                
                Ok(result_reg)
            }
            
            Expr::IfExpr { condition, then_branch, else_branch, span } => {
                let cond_reg = self.compile_expr(condition)?;
                let else_jump = self.emit_jump(OpCode::JumpIfFalse, cond_reg, span.line as u32);
                self.current_scope_mut().free_register();
                
                let then_reg = self.compile_expr(then_branch)?;
                let end_jump = self.emit_jump(OpCode::Jump, 0, span.line as u32);
                
                self.patch_jump(else_jump);
                self.current_scope_mut().free_register();
                
                let else_reg = self.compile_expr(else_branch)?;
                self.patch_jump(end_jump);
                
                Ok(else_reg)
            }
            
            Expr::Match { value, arms, span } => {
                let value_reg = self.compile_expr(value)?;
                let result_reg = self.current_scope_mut().alloc_register()?;
                
                let mut end_jumps = Vec::new();
                
                for arm in arms {
                    // Compile pattern matching
                    let pattern_matches = self.compile_pattern(&arm.pattern, value_reg, span.line as u32)?;
                    let skip_jump = self.emit_jump(OpCode::JumpIfFalse, pattern_matches, span.line as u32);
                    self.current_scope_mut().free_register();
                    
                    // Compile guard if present
                    if let Some(guard) = &arm.guard {
                        let guard_reg = self.compile_expr(guard)?;
                        let guard_skip = self.emit_jump(OpCode::JumpIfFalse, guard_reg, span.line as u32);
                        self.current_scope_mut().free_register();
                        
                        // Compile body
                        let body_reg = self.compile_expr(&arm.body)?;
                        self.emit(Instruction::with_ab(OpCode::StoreLocal, result_reg, body_reg), span.line as u32);
                        self.current_scope_mut().free_register();
                        
                        end_jumps.push(self.emit_jump(OpCode::Jump, 0, span.line as u32));
                        self.patch_jump(guard_skip);
                    } else {
                        // Compile body
                        let body_reg = self.compile_expr(&arm.body)?;
                        self.emit(Instruction::with_ab(OpCode::StoreLocal, result_reg, body_reg), span.line as u32);
                        self.current_scope_mut().free_register();
                        
                        end_jumps.push(self.emit_jump(OpCode::Jump, 0, span.line as u32));
                    }
                    
                    self.patch_jump(skip_jump);
                }
                
                // Patch all end jumps
                for jump in end_jumps {
                    self.patch_jump(jump);
                }
                
                self.current_scope_mut().free_register(); // value_reg
                
                Ok(result_reg)
            }
            
            Expr::Block { statements, expression, span } => {
                self.begin_scope();
                
                for stmt in statements {
                    self.compile_stmt(stmt)?;
                }
                
                let result_reg = if let Some(expr) = expression {
                    self.compile_expr(expr)?
                } else {
                    let reg = self.current_scope_mut().alloc_register()?;
                    self.emit(Instruction::with_a(OpCode::LoadNull, reg), span.line as u32);
                    reg
                };
                
                self.end_scope(span.line as u32);
                
                Ok(result_reg)
            }
            
            Expr::Range { start, end, inclusive, span } => {
                // Ranges are compiled as iterator constructors
                let start_reg = self.compile_expr(start)?;
                let end_reg = self.compile_expr(end)?;
                let result_reg = self.current_scope_mut().alloc_register()?;
                
                // Use a special range instruction or call
                self.emit(Instruction::with_abc(OpCode::NewIter, result_reg, start_reg, end_reg), span.line as u32);
                
                self.current_scope_mut().free_registers(2);
                
                Ok(result_reg)
            }
            
            Expr::NullCoalesce { left, right, span } => {
                let left_reg = self.compile_expr(left)?;
                
                // If not null, jump to end
                let end_jump = self.emit_jump(OpCode::JumpIfNotNull, left_reg, span.line as u32);
                self.current_scope_mut().free_register();
                
                // Evaluate right
                let right_reg = self.compile_expr(right)?;
                
                self.patch_jump(end_jump);
                
                Ok(right_reg)
            }
            
            Expr::Await { expr, span } => {
                let expr_reg = self.compile_expr(expr)?;
                let result_reg = self.current_scope_mut().alloc_register()?;
                self.emit(Instruction::with_ab(OpCode::Await, result_reg, expr_reg), span.line as u32);
                self.current_scope_mut().free_register();
                Ok(result_reg)
            }
            
            Expr::Grouping { expr, .. } => {
                self.compile_expr(expr)
            }
            
            Expr::OptionalChain { object, property, span } => {
                let obj_reg = self.compile_expr(object)?;
                
                // If null, short-circuit
                let null_jump = self.emit_jump(OpCode::JumpIfNull, obj_reg, span.line as u32);
                
                // Get property
                let result_reg = self.current_scope_mut().alloc_register()?;
                let prop_const = self.add_constant(Value::null());
                self.emit(Instruction::with_abc(OpCode::GetProp, result_reg, obj_reg, prop_const as u8), span.line as u32);
                
                let end_jump = self.emit_jump(OpCode::Jump, 0, span.line as u32);
                
                self.patch_jump(null_jump);
                self.emit(Instruction::with_a(OpCode::LoadNull, result_reg), span.line as u32);
                
                self.patch_jump(end_jump);
                self.current_scope_mut().free_register();
                
                Ok(result_reg)
            }
            
            Expr::Spread { expr, span } => {
                let expr_reg = self.compile_expr(expr)?;
                self.emit(Instruction::with_ab(OpCode::Spread, expr_reg, expr_reg), span.line as u32);
                Ok(expr_reg)
            }
            
            Expr::CompoundAssign { target, operator, value, span } => {
                // a += b becomes a = a + b
                let target_reg = self.compile_expr(target)?;
                let value_reg = self.compile_expr(value)?;
                let result_reg = self.current_scope_mut().alloc_register()?;
                
                let opcode = match operator {
                    BinaryOp::Add => OpCode::Add,
                    BinaryOp::Sub => OpCode::Sub,
                    BinaryOp::Mul => OpCode::Mul,
                    BinaryOp::Div => OpCode::Div,
                    _ => OpCode::Add,
                };
                
                self.emit(Instruction::with_abc(opcode, result_reg, target_reg, value_reg), span.line as u32);
                
                // Store back
                match target.as_ref() {
                    Expr::Identifier { name, .. } => {
                        if let Some((slot, _)) = self.resolve_local(*name) {
                            self.emit(Instruction::with_ab(OpCode::StoreLocal, slot, result_reg), span.line as u32);
                        }
                    }
                    _ => {}
                }
                
                self.current_scope_mut().free_registers(2);
                
                Ok(result_reg)
            }
        }
    }
    
    fn compile_pattern(&mut self, pattern: &Pattern, value_reg: u8, line: u32) -> HateResult<u8> {
        let result_reg = self.current_scope_mut().alloc_register()?;
        
        match pattern {
            Pattern::Wildcard { .. } => {
                // Always matches
                self.emit(Instruction::with_a(OpCode::LoadTrue, result_reg), line);
            }
            Pattern::Literal { value, .. } => {
                let lit_reg = self.current_scope_mut().alloc_register()?;
                match value {
                    Literal::Integer(i) => {
                        self.emit(Instruction::with_ab(OpCode::LoadInt, lit_reg, *i as u8), line);
                    }
                    Literal::Bool(true) => {
                        self.emit(Instruction::with_a(OpCode::LoadTrue, lit_reg), line);
                    }
                    Literal::Bool(false) => {
                        self.emit(Instruction::with_a(OpCode::LoadFalse, lit_reg), line);
                    }
                    Literal::Null => {
                        self.emit(Instruction::with_a(OpCode::LoadNull, lit_reg), line);
                    }
                    _ => {
                        self.emit(Instruction::with_a(OpCode::LoadNull, lit_reg), line);
                    }
                }
                self.emit(Instruction::with_abc(OpCode::Eq, result_reg, value_reg, lit_reg), line);
                self.current_scope_mut().free_register();
            }
            Pattern::Binding { name, .. } => {
                // Binding always matches and creates a local
                let local_reg = self.current_scope_mut().alloc_register()?;
                self.emit(Instruction::with_ab(OpCode::LoadLocal, local_reg, value_reg), line);
                self.add_local(*name, false, local_reg);
                self.emit(Instruction::with_a(OpCode::LoadTrue, result_reg), line);
            }
            Pattern::Range { start, end, .. } => {
                // value >= start && value <= end
                let start_reg = self.current_scope_mut().alloc_register()?;
                let end_reg = self.current_scope_mut().alloc_register()?;
                let temp1 = self.current_scope_mut().alloc_register()?;
                let temp2 = self.current_scope_mut().alloc_register()?;
                
                // Load start and end
                match start.as_ref() {
                    Literal::Integer(i) => {
                        self.emit(Instruction::with_ab(OpCode::LoadInt, start_reg, *i as u8), line);
                    }
                    _ => {}
                }
                match end.as_ref() {
                    Literal::Integer(i) => {
                        self.emit(Instruction::with_ab(OpCode::LoadInt, end_reg, *i as u8), line);
                    }
                    _ => {}
                }
                
                // value >= start
                self.emit(Instruction::with_abc(OpCode::Ge, temp1, value_reg, start_reg), line);
                // value <= end
                self.emit(Instruction::with_abc(OpCode::Le, temp2, value_reg, end_reg), line);
                // AND
                self.emit(Instruction::with_abc(OpCode::BitAnd, result_reg, temp1, temp2), line);
                
                self.current_scope_mut().free_registers(4);
            }
            _ => {
                // Default: always match
                self.emit(Instruction::with_a(OpCode::LoadTrue, result_reg), line);
            }
        }
        
        Ok(result_reg)
    }
    
    // ==================== Helper Methods ====================
    
    fn current_scope(&self) -> &CompilerScope {
        self.scopes.last().unwrap()
    }
    
    fn current_scope_mut(&mut self) -> &mut CompilerScope {
        self.scopes.last_mut().unwrap()
    }
    
    fn current_chunk(&self) -> &Chunk {
        &self.current_scope().function.chunk
    }
    
    fn current_chunk_mut(&mut self) -> &mut Chunk {
        &mut self.current_scope_mut().function.chunk
    }
    
    fn emit(&mut self, instruction: Instruction, line: u32) {
        self.current_chunk_mut().write(instruction, line);
    }
    
    fn emit_jump(&mut self, opcode: OpCode, reg: u8, line: u32) -> usize {
        self.emit(Instruction::with_a_imm16(opcode, reg, 0xFFFF), line);
        self.current_chunk().len() - 1
    }
    
    fn patch_jump(&mut self, offset: usize) {
        self.current_chunk_mut().patch_jump(offset);
    }
    
    fn emit_loop(&mut self, loop_start: usize, line: u32) -> HateResult<()> {
        let offset = self.current_chunk().len() - loop_start + 1;
        if offset > u16::MAX as usize {
            return Err(HateError::JumpTooLarge);
        }
        
        // Emit negative jump (loop back)
        let jump_back = -(offset as i16);
        self.emit(Instruction::with_a_imm16(OpCode::Loop, 0, jump_back as u16), line);
        Ok(())
    }
    
    fn add_constant(&mut self, value: Value) -> u16 {
        self.current_chunk_mut().add_constant(value)
    }
    
    fn begin_scope(&mut self) {
        self.current_scope_mut().scope_depth += 1;
    }
    
    fn end_scope(&mut self, line: u32) {
        self.current_scope_mut().scope_depth -= 1;
        
        // Pop locals from this scope
        loop {
            let should_pop = {
                let scope = self.current_scope();
                if !scope.locals.is_empty() && 
                   scope.locals.last().unwrap().depth > scope.scope_depth {
                    Some(scope.locals.last().unwrap().is_captured)
                } else {
                    None
                }
            };
            
            match should_pop {
                Some(is_captured) => {
                    self.current_scope_mut().locals.pop();
                    if is_captured {
                        self.emit(Instruction::with_a(OpCode::CloseUpvalue, 0), line);
                    }
                    self.current_scope_mut().register_count -= 1;
                }
                None => break,
            }
        }
    }
    
    fn add_local(&mut self, name: Symbol, mutable: bool, register: u8) {
        let scope = self.current_scope_mut();
        scope.locals.push(Local {
            name,
            depth: scope.scope_depth,
            mutable,
            is_captured: false,
            register,
        });
    }
    
    fn resolve_local(&self, name: Symbol) -> Option<(u8, bool)> {
        let scope = self.current_scope();
        for local in scope.locals.iter().rev() {
            if local.name == name {
                return Some((local.register, local.mutable));
            }
        }
        None
    }
    
    fn resolve_upvalue(&mut self, name: Symbol) -> Option<u8> {
        if self.scopes.len() <= 1 {
            return None;
        }
        
        // Look in enclosing scopes
        for i in (0..self.scopes.len() - 1).rev() {
            // Check locals in enclosing scope
            for (j, local) in self.scopes[i].locals.iter().enumerate() {
                if local.name == name {
                    // Mark as captured
                    self.scopes[i].locals[j].is_captured = true;
                    
                    // Add upvalue to current scope
                    return Some(self.add_upvalue(i + 1, j as u8, true));
                }
            }
            
            // Check upvalues in enclosing scope
            for (j, upvalue) in self.scopes[i].upvalues.iter().enumerate() {
                // This would require more complex tracking
            }
        }
        
        None
    }
    
    fn add_upvalue(&mut self, scope_idx: usize, index: u8, is_local: bool) -> u8 {
        let upvalues = &mut self.scopes[scope_idx].upvalues;
        
        // Check if already exists
        for (i, uv) in upvalues.iter().enumerate() {
            if uv.index == index && uv.is_local == is_local {
                return i as u8;
            }
        }
        
        let idx = upvalues.len() as u8;
        upvalues.push(CompilerUpvalue { index, is_local });
        self.scopes[scope_idx].function.upvalue_count += 1;
        idx
    }
    
    fn begin_function(&mut self, name: Option<Symbol>, arity: u8) -> HateResult<()> {
        let mut scope = CompilerScope::new(name);
        scope.function.arity = arity;
        self.scopes.push(scope);
        self.begin_scope();
        Ok(())
    }
    
    fn end_function(&mut self, line: u32) -> Function {
        self.end_scope(line);
        
        let scope = self.scopes.pop().unwrap();
        let mut function = scope.function;
        function.chunk.max_registers = scope.max_registers;
        
        // Copy upvalue info
        for uv in scope.upvalues {
            function.chunk.upvalues.push(UpvalueInfo {
                index: uv.index,
                is_local: uv.is_local,
            });
        }
        
        function
    }
    
    fn compile_function(&mut self, name: Symbol, params: &[Param], body: &Expr, line: u32) -> HateResult<()> {
        self.begin_function(Some(name), params.len() as u8)?;
        
        for (i, param) in params.iter().enumerate() {
            self.add_local(param.name, false, i as u8);
        }
        
        let body_reg = self.compile_expr(body)?;
        self.emit(Instruction::with_a(OpCode::Return, body_reg), line);
        
        let function = self.end_function(line);
        
        // Store function as global or local
        let func_const = self.add_constant(Value::null()); // Placeholder
        let result_reg = self.current_scope_mut().alloc_register()?;
        self.emit(Instruction::with_a_imm16(OpCode::Closure, result_reg, func_const), line);
        
        self.add_local(name, false, result_reg);
        
        Ok(())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    
    fn compile(source: &str) -> Chunk {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        let mut compiler = Compiler::new();
        compiler.compile(&ast).unwrap()
    }
    
    #[test]
    fn test_compile_literal() {
        let chunk = compile("42;");
        assert!(!chunk.is_empty());
    }
    
    #[test]
    fn test_compile_let() {
        let chunk = compile("let x = 10;");
        assert!(!chunk.is_empty());
    }
    
    #[test]
    fn test_compile_binary() {
        let chunk = compile("1 + 2;");
        // Should have: LoadInt, LoadInt, Add
        assert!(chunk.code.iter().any(|i| i.opcode == OpCode::Add));
    }
    
    #[test]
    fn test_compile_if() {
        let chunk = compile("if true { 1; } else { 2; }");
        assert!(chunk.code.iter().any(|i| i.opcode == OpCode::JumpIfFalse));
    }
    
    #[test]
    fn test_compile_while() {
        let chunk = compile("while true { 1; }");
        assert!(chunk.code.iter().any(|i| i.opcode == OpCode::Loop));
    }
    
    #[test]
    fn test_compile_function() {
        let chunk = compile("fn add(a: i64, b: i64): i64 => a + b;");
        assert!(!chunk.is_empty());
    }
}
