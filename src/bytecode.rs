//! Bytecode instruction set for the Hate VM
//!
//! Register-based bytecode with 60+ opcodes optimized for:
//! - Direct-threaded dispatch
//! - Inline caching
//! - Integer fast-paths

use std::fmt;
use serde::{Serialize, Deserialize};
use crate::value::Value;
use crate::intern::Symbol;

/// Bytecode opcodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum OpCode {
    // ==================== Constants & Variables ====================
    /// Load constant: r1 = constants[idx]
    LoadConst = 0,
    /// Load null: r1 = null
    LoadNull,
    /// Load true: r1 = true
    LoadTrue,
    /// Load false: r1 = false
    LoadFalse,
    /// Load small integer: r1 = imm8 (signed)
    LoadInt,
    
    /// Load local variable: r1 = locals[idx]
    LoadLocal,
    /// Store local variable: locals[idx] = r1
    StoreLocal,
    /// Load upvalue (closure variable): r1 = upvalues[idx]
    LoadUpvalue,
    /// Store upvalue: upvalues[idx] = r1
    StoreUpvalue,
    /// Load global: r1 = globals[name]
    LoadGlobal,
    /// Store global: globals[name] = r1
    StoreGlobal,
    
    // ==================== Arithmetic ====================
    /// Add: r1 = r2 + r3
    Add,
    /// Subtract: r1 = r2 - r3
    Sub,
    /// Multiply: r1 = r2 * r3
    Mul,
    /// Divide: r1 = r2 / r3
    Div,
    /// Modulo: r1 = r2 % r3
    Mod,
    /// Negate: r1 = -r2
    Neg,
    
    /// Integer add (fast path): r1 = r2 + r3
    IAdd,
    /// Integer subtract: r1 = r2 - r3
    ISub,
    /// Integer multiply: r1 = r2 * r3
    IMul,
    /// Integer divide: r1 = r2 / r3
    IDiv,
    
    // ==================== Bitwise ====================
    /// Bitwise AND: r1 = r2 & r3
    BitAnd,
    /// Bitwise OR: r1 = r2 | r3
    BitOr,
    /// Bitwise XOR: r1 = r2 ^ r3
    BitXor,
    /// Bitwise NOT: r1 = ~r2
    BitNot,
    /// Left shift: r1 = r2 << r3
    Shl,
    /// Right shift (arithmetic): r1 = r2 >> r3
    Shr,
    
    // ==================== Comparison ====================
    /// Equal: r1 = r2 == r3
    Eq,
    /// Not equal: r1 = r2 != r3
    Ne,
    /// Less than: r1 = r2 < r3
    Lt,
    /// Less or equal: r1 = r2 <= r3
    Le,
    /// Greater than: r1 = r2 > r3
    Gt,
    /// Greater or equal: r1 = r2 >= r3
    Ge,
    
    // ==================== Logical ====================
    /// Logical NOT: r1 = !r2
    Not,
    
    // ==================== Objects ====================
    /// Create new object: r1 = {}
    NewObject,
    /// Get property: r1 = r2.key (with inline cache)
    GetProp,
    /// Set property: r1.key = r2 (with inline cache)
    SetProp,
    /// Delete property: delete r1.key
    DelProp,
    /// Check if property exists: r1 = key in r2
    HasProp,
    
    // ==================== Arrays ====================
    /// Create new array: r1 = Array(size)
    NewArray,
    /// Get array element: r1 = r2[r3]
    GetIndex,
    /// Set array element: r1[r2] = r3
    SetIndex,
    /// Get array length: r1 = r2.length
    ArrayLen,
    /// Push to array: r1.push(r2)
    ArrayPush,
    /// Pop from array: r1 = r2.pop()
    ArrayPop,
    
    // ==================== Functions ====================
    /// Call function: r1 = func(args...)
    Call,
    /// Call method: r1 = obj.method(args...)
    CallMethod,
    /// Return from function: return r1
    Return,
    /// Create closure: r1 = closure(func, upvalues)
    Closure,
    /// Close upvalue (move to heap)
    CloseUpvalue,
    
    // ==================== Control Flow ====================
    /// Unconditional jump: pc += offset
    Jump,
    /// Jump if true: if r1 then pc += offset
    JumpIfTrue,
    /// Jump if false: if !r1 then pc += offset
    JumpIfFalse,
    /// Jump if null: if r1 == null then pc += offset
    JumpIfNull,
    /// Jump if not null: if r1 != null then pc += offset
    JumpIfNotNull,
    /// Loop back (for optimizer hints)
    Loop,
    
    // ==================== Type Operations ====================
    /// Get type: r1 = typeof r2
    TypeOf,
    /// Is null check: r1 = r2 == null
    IsNull,
    /// To integer: r1 = i64(r2)
    ToInt,
    /// To float: r1 = f64(r2)
    ToFloat,
    /// To string: r1 = str(r2)
    ToString,
    /// To bool: r1 = bool(r2)
    ToBool,
    
    // ==================== Iterators ====================
    /// Create iterator: r1 = iter(r2)
    NewIter,
    /// Iterator next: r1 = iter.next(), r2 = done flag
    IterNext,
    /// For-in loop preparation
    ForIn,
    
    // ==================== Pattern Matching ====================
    /// Match pattern: jump table dispatch
    Match,
    /// Destructure array: [r1, r2, ...] = r3
    DestructArray,
    /// Destructure object: {a: r1, b: r2} = r3
    DestructObject,
    
    // ==================== Special ====================
    /// Spread operator: ...r1
    Spread,
    /// Null coalescing: r1 = r2 ?? r3
    NullCoalesce,
    /// Optional chain: r1 = r2?.prop
    OptionalChain,
    
    // ==================== Concurrency ====================
    /// Await promise: r1 = await r2
    Await,
    /// Yield value (for generators)
    Yield,
    
    // ==================== Debugging ====================
    /// Print value (debug): print r1
    Print,
    /// Assertion: assert r1
    Assert,
    /// Breakpoint (for debugger)
    Breakpoint,
    
    // ==================== GC Hints ====================
    /// Write barrier for GC
    WriteBarrier,
    
    // ==================== Module ====================
    /// Import module
    Import,
    /// Export value
    Export,
    
    /// No operation
    Nop,
    /// Halt execution
    Halt,
    
    // ==================== Error Handling ====================
    /// Begin try block
    TryBegin,
    /// End try block
    TryEnd,
    /// Catch exception: r1 = exception
    Catch,
    /// Throw exception: throw r1
    Throw,
    /// Finally block
    Finally,
    
    // ==================== Extended Pattern Matching ====================
    /// Match begin: prepare match on r1
    MatchBegin,
    /// Match arm test: test pattern, jump if not matched
    MatchArm,
    /// Match guard: test guard condition
    MatchGuard,
    /// Match end: default case or error
    MatchEnd,
    
    // ==================== Range Operations ====================
    /// Exclusive range: r1 = start..end
    RangeExclusive,
    /// Inclusive range: r1 = start..=end
    RangeInclusive,
    /// Range iterator step
    RangeStep,
    
    // ==================== Advanced Object Operations ====================
    /// Spread object into another: r1 = { ...r2 }
    SpreadObject,
    /// Spread array: r1 = [ ...r2 ]
    SpreadArray,
    /// Destructure object into registers
    DestructObject2,
    /// Destructure array into registers
    DestructArray2,
    
    // ==================== Class Operations ====================
    /// Create new class: r1 = class
    NewClass,
    /// Get superclass: r1 = super(r2)
    GetSuper,
    /// Invoke superclass method
    InvokeSuper,
    /// Inherit from superclass
    Inherit,
    /// Get method: r1 = r2.method
    GetMethod,
    /// Bind method to instance
    BindMethod,
    
    // ==================== Async Operations ====================
    /// Create async function state machine
    AsyncBegin,
    /// Resume async execution
    AsyncResume,
    /// Suspend async execution
    AsyncSuspend,
    /// Create promise
    NewPromise,
    /// Resolve promise
    ResolvePromise,
    /// Reject promise
    RejectPromise,
}

impl OpCode {
    /// Get the number of operands for this opcode
    pub fn operand_count(&self) -> usize {
        match self {
            OpCode::LoadNull | OpCode::LoadTrue | OpCode::LoadFalse |
            OpCode::Return | OpCode::Halt | OpCode::Nop | 
            OpCode::Breakpoint | OpCode::CloseUpvalue => 1,
            
            OpCode::LoadConst | OpCode::LoadInt | OpCode::LoadLocal |
            OpCode::StoreLocal | OpCode::LoadUpvalue | OpCode::StoreUpvalue |
            OpCode::LoadGlobal | OpCode::StoreGlobal | OpCode::Neg |
            OpCode::BitNot | OpCode::Not | OpCode::NewObject |
            OpCode::NewArray | OpCode::ArrayLen | OpCode::Jump |
            OpCode::JumpIfTrue | OpCode::JumpIfFalse | OpCode::JumpIfNull |
            OpCode::JumpIfNotNull | OpCode::Loop | OpCode::TypeOf |
            OpCode::IsNull | OpCode::ToInt | OpCode::ToFloat |
            OpCode::ToString | OpCode::ToBool | OpCode::NewIter |
            OpCode::Print | OpCode::Assert | OpCode::Closure => 2,
            
            OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div |
            OpCode::Mod | OpCode::IAdd | OpCode::ISub | OpCode::IMul |
            OpCode::IDiv | OpCode::BitAnd | OpCode::BitOr | OpCode::BitXor |
            OpCode::Shl | OpCode::Shr | OpCode::Eq | OpCode::Ne |
            OpCode::Lt | OpCode::Le | OpCode::Gt | OpCode::Ge |
            OpCode::GetProp | OpCode::SetProp | OpCode::GetIndex |
            OpCode::SetIndex | OpCode::Call | OpCode::CallMethod |
            OpCode::IterNext | OpCode::NullCoalesce | OpCode::ArrayPush |
            OpCode::ArrayPop | OpCode::Await | OpCode::Yield => 3,
            
            _ => 2,
        }
    }
    
    /// Check if this opcode is a jump
    pub fn is_jump(&self) -> bool {
        matches!(self,
            OpCode::Jump | OpCode::JumpIfTrue | OpCode::JumpIfFalse |
            OpCode::JumpIfNull | OpCode::JumpIfNotNull | OpCode::Loop
        )
    }
    
    /// Check if this opcode terminates a basic block
    pub fn is_terminator(&self) -> bool {
        matches!(self,
            OpCode::Jump | OpCode::JumpIfTrue | OpCode::JumpIfFalse |
            OpCode::Return | OpCode::Halt
        )
    }
}

impl fmt::Display for OpCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// A single bytecode instruction
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Instruction {
    pub opcode: OpCode,
    pub a: u8,      // Destination register or operand
    pub b: u8,      // Source register or operand
    pub c: u8,      // Source register or operand
}

impl Instruction {
    /// Create a new instruction with no operands
    pub fn new(opcode: OpCode) -> Self {
        Self { opcode, a: 0, b: 0, c: 0 }
    }
    
    /// Create a new instruction with one operand
    pub fn with_a(opcode: OpCode, a: u8) -> Self {
        Self { opcode, a, b: 0, c: 0 }
    }
    
    /// Create a new instruction with two operands
    pub fn with_ab(opcode: OpCode, a: u8, b: u8) -> Self {
        Self { opcode, a, b, c: 0 }
    }
    
    /// Create a new instruction with three operands
    pub fn with_abc(opcode: OpCode, a: u8, b: u8, c: u8) -> Self {
        Self { opcode, a, b, c }
    }
    
    /// Create a new instruction with a 16-bit immediate (stored in b and c)
    pub fn with_a_imm16(opcode: OpCode, a: u8, imm: u16) -> Self {
        Self {
            opcode,
            a,
            b: (imm >> 8) as u8,
            c: (imm & 0xFF) as u8,
        }
    }
    
    /// Get the 16-bit immediate value from b and c
    pub fn imm16(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }
    
    /// Get the signed 16-bit immediate (for jumps)
    pub fn simm16(&self) -> i16 {
        self.imm16() as i16
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.opcode.operand_count() {
            1 => write!(f, "{} r{}", self.opcode, self.a),
            2 => write!(f, "{} r{}, {}", self.opcode, self.a, self.b),
            3 => write!(f, "{} r{}, r{}, r{}", self.opcode, self.a, self.b, self.c),
            _ => write!(f, "{}", self.opcode),
        }
    }
}

/// A compiled chunk of bytecode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// The bytecode instructions
    pub code: Vec<Instruction>,
    /// Constant pool
    pub constants: Vec<Value>,
    /// Nested functions
    pub functions: Vec<Function>,
    /// Source line numbers for each instruction (for debugging)
    pub lines: Vec<u32>,
    /// Local variable names (for debugging)
    pub locals: Vec<Symbol>,
    /// Upvalue information
    pub upvalues: Vec<UpvalueInfo>,
    /// Function name (for debugging)
    pub name: Option<Symbol>,
    /// Number of parameters
    pub arity: u8,
    /// Number of registers needed
    pub max_registers: u8,
}

/// Information about an upvalue (captured variable)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UpvalueInfo {
    /// Index in the enclosing function's locals or upvalues
    pub index: u8,
    /// True if this captures a local, false if it captures an upvalue
    pub is_local: bool,
}

impl Chunk {
    /// Create a new empty chunk
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            functions: Vec::new(),
            lines: Vec::new(),
            locals: Vec::new(),
            upvalues: Vec::new(),
            name: None,
            arity: 0,
            max_registers: 0,
        }
    }
    
    /// Write an instruction to the chunk
    pub fn write(&mut self, instruction: Instruction, line: u32) {
        self.code.push(instruction);
        self.lines.push(line);
    }
    
    /// Add a constant to the pool, returning its index
    pub fn add_constant(&mut self, value: Value) -> u16 {
        // Check if constant already exists
        for (i, c) in self.constants.iter().enumerate() {
            if c.bits() == value.bits() {
                return i as u16;
            }
        }
        
        let index = self.constants.len();
        self.constants.push(value);
        index as u16
    }
    
    /// Get the current code offset
    pub fn len(&self) -> usize {
        self.code.len()
    }
    
    /// Check if the chunk is empty
    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }
    
    /// Add a function to the chunk, returning its index
    pub fn add_function(&mut self, function: Function) -> u16 {
        let index = self.functions.len();
        self.functions.push(function);
        index as u16
    }
    
    /// Patch a jump instruction with the correct offset
    pub fn patch_jump(&mut self, offset: usize) {
        let jump_distance = (self.code.len() - offset - 1) as u16;
        let instruction = &mut self.code[offset];
        instruction.b = (jump_distance >> 8) as u8;
        instruction.c = (jump_distance & 0xFF) as u8;
    }
    
    /// Disassemble the chunk
    pub fn disassemble(&self, name: &str) {
        println!("== {} ==", name);
        
        let mut offset = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset);
        }
    }
    
    /// Disassemble a single instruction
    pub fn disassemble_instruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);
        
        if offset > 0 && self.lines[offset] == self.lines[offset - 1] {
            print!("   | ");
        } else {
            print!("{:4} ", self.lines[offset]);
        }
        
        let instruction = &self.code[offset];
        
        match instruction.opcode {
            OpCode::LoadConst => {
                let constant_idx = instruction.imm16() as usize;
                let constant = &self.constants[constant_idx];
                println!("{:<16} r{} <- {:?}", "LOAD_CONST", instruction.a, constant);
            }
            OpCode::LoadLocal => {
                println!("{:<16} r{} <- local[{}]", "LOAD_LOCAL", instruction.a, instruction.b);
            }
            OpCode::StoreLocal => {
                println!("{:<16} local[{}] <- r{}", "STORE_LOCAL", instruction.a, instruction.b);
            }
            OpCode::Jump | OpCode::JumpIfTrue | OpCode::JumpIfFalse => {
                let target = offset as i32 + 1 + instruction.simm16() as i32;
                println!("{:<16} r{} -> {}", instruction.opcode, instruction.a, target);
            }
            _ => {
                println!("{}", instruction);
            }
        }
        
        offset + 1
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

/// A compiled function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub chunk: Chunk,
    pub name: Option<Symbol>,
    pub arity: u8,
    pub upvalue_count: u8,
}

impl Function {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            name: None,
            arity: 0,
            upvalue_count: 0,
        }
    }
}

impl Default for Function {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_instruction_size() {
        // Each instruction should be 4 bytes
        assert_eq!(std::mem::size_of::<Instruction>(), 4);
    }
    
    #[test]
    fn test_imm16() {
        let inst = Instruction::with_a_imm16(OpCode::LoadConst, 0, 0x1234);
        assert_eq!(inst.imm16(), 0x1234);
    }
    
    #[test]
    fn test_chunk_write() {
        let mut chunk = Chunk::new();
        chunk.write(Instruction::new(OpCode::LoadNull), 1);
        assert_eq!(chunk.len(), 1);
        assert_eq!(chunk.lines[0], 1);
    }
    
    #[test]
    fn test_add_constant() {
        let mut chunk = Chunk::new();
        let idx1 = chunk.add_constant(Value::int(42));
        let idx2 = chunk.add_constant(Value::int(42)); // Duplicate
        assert_eq!(idx1, idx2); // Should return same index
    }
}
