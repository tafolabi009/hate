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
use crate::object::{HateArray, HateObject};
use crate::vm_internals::{make_heap_ptr, decode_heap_ptr, TAG_ARRAY, TAG_OBJECT};
use crate::intern::Symbol;

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
    /// Heap for objects and arrays
    heap: crate::vm_internals::Heap,
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
            heap: crate::vm_internals::Heap::new(),
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
                    // Create new object via heap
                    let empty_class = self.heap.object_store.empty_class();
                    let idx = self.heap.alloc_object(HateObject::new(empty_class));
                    self.set_reg(instruction.a, make_heap_ptr(idx, TAG_OBJECT));
                }
                
                OpCode::GetProp => {
                    // Property access: a = b.property_name
                    // b = object register, c = property name constant index
                    // Save IP for inline cache before borrowing
                    let cache_idx = frame.ip.saturating_sub(1);
                    let current_chunk_idx = frame.chunk_idx;
                    
                    let obj_val = self.reg(instruction.b);
                    if let Some((idx, tag)) = decode_heap_ptr(obj_val) {
                        if tag == TAG_OBJECT {
                            if let Some(obj) = self.heap.get_object(idx) {
                                let obj_class = obj.class;
                                
                                // Try inline cache first (keyed by instruction index)
                                if cache_idx < self.property_caches.len() {
                                    let cache = &self.property_caches[cache_idx];
                                    if cache.map == obj_class as u64 {
                                        // Cache hit! Fast path
                                        let value = obj.get_slot(cache.slot);
                                        self.set_reg(instruction.a, value);
                                        continue;
                                    }
                                }
                                
                                // Cache miss - slow path with hidden class lookup
                                let prop_name_idx = instruction.c as usize;
                                if let Some(symbol) = self.chunks[current_chunk_idx].constants.get(prop_name_idx) {
                                    if let Some(sym_idx) = symbol.as_string_index() {
                                        let sym = Symbol::from_index(sym_idx);
                                        // Lookup property in hidden class
                                        if let Some(slot) = self.heap.object_store.get_class(obj_class).lookup(sym) {
                                            let value = obj.get_slot(slot);
                                            self.set_reg(instruction.a, value);
                                            
                                            // Update inline cache for next time
                                            if cache_idx < self.property_caches.len() {
                                                self.property_caches[cache_idx] = InlineCache {
                                                    map: obj_class as u64,
                                                    slot,
                                                };
                                            }
                                        } else {
                                            self.set_reg(instruction.a, Value::null());
                                        }
                                    } else {
                                        self.set_reg(instruction.a, Value::null());
                                    }
                                } else {
                                    self.set_reg(instruction.a, Value::null());
                                }
                            } else {
                                self.set_reg(instruction.a, Value::null());
                            }
                        } else {
                            self.set_reg(instruction.a, Value::null());
                        }
                    } else {
                        self.set_reg(instruction.a, Value::null());
                    }
                }
                
                OpCode::SetProp => {
                    // Property setting: a.property_name = b
                    // a = object register, b = value register, c = property name constant index
                    // Save frame info for inline cache
                    let cache_idx = frame.ip.saturating_sub(1);
                    let current_chunk_idx = frame.chunk_idx;
                    
                    let obj_val = self.reg(instruction.a);
                    let value = self.reg(instruction.b);
                    if let Some((idx, tag)) = decode_heap_ptr(obj_val) {
                        if tag == TAG_OBJECT {
                            if let Some(obj) = self.heap.get_object(idx) {
                                let obj_class = obj.class;
                                
                                // Try inline cache first for fast path (existing property)
                                if cache_idx < self.property_caches.len() {
                                    let cache = &self.property_caches[cache_idx];
                                    if cache.map == obj_class as u64 {
                                        // Cache hit! Fast path - directly set slot
                                        if let Some(obj) = self.heap.get_object_mut(idx) {
                                            obj.set_slot(cache.slot, value);
                                        }
                                        continue;
                                    }
                                }
                            }
                            
                            // Cache miss - slow path
                            let prop_name_idx = instruction.c as usize;
                            if let Some(symbol) = self.chunks[current_chunk_idx].constants.get(prop_name_idx) {
                                if let Some(sym_idx) = symbol.as_string_index() {
                                    let sym = Symbol::from_index(sym_idx);
                                    
                                    // Get the object's current class and check if property exists
                                    let obj_class = self.heap.get_object(idx).unwrap().class;
                                    let (slot, final_class) = if let Some(s) = self.heap.object_store.get_class(obj_class).lookup(sym) {
                                        (s, obj_class)
                                    } else {
                                        // Need to transition to a new hidden class with this property
                                        let new_class = self.heap.object_store.transition(obj_class, sym);
                                        // Update the object's class
                                        self.heap.get_object_mut(idx).unwrap().class = new_class;
                                        (self.heap.object_store.get_class(new_class).lookup(sym).unwrap(), new_class)
                                    };
                                    
                                    // Set the slot value
                                    if let Some(obj) = self.heap.get_object_mut(idx) {
                                        obj.set_slot(slot, value);
                                    }
                                    
                                    // Update inline cache for next time
                                    if cache_idx < self.property_caches.len() {
                                        self.property_caches[cache_idx] = InlineCache {
                                            map: final_class as u64,
                                            slot,
                                        };
                                    }
                                }
                            }
                        }
                    }
                }
                
                OpCode::DelProp => {
                    // TODO: Property deletion (complex - needs hidden class transition)
                }
                
                OpCode::HasProp => {
                    // Check if property exists
                    let obj_val = self.reg(instruction.b);
                    if let Some((idx, tag)) = decode_heap_ptr(obj_val) {
                        if tag == TAG_OBJECT {
                            if let Some(obj) = self.heap.get_object(idx) {
                                let prop_name_idx = instruction.c as usize;
                                let chunk_idx = self.frames[self.frames.len() - 1].chunk_idx;
                                if let Some(symbol) = self.chunks[chunk_idx].constants.get(prop_name_idx) {
                                    if let Some(sym_idx) = symbol.as_string_index() {
                                        let sym = Symbol::from_index(sym_idx);
                                        let has = self.heap.object_store.get_class(obj.class).lookup(sym).is_some();
                                        self.set_reg(instruction.a, Value::bool(has));
                                    } else {
                                        self.set_reg(instruction.a, Value::bool(false));
                                    }
                                } else {
                                    self.set_reg(instruction.a, Value::bool(false));
                                }
                            } else {
                                self.set_reg(instruction.a, Value::bool(false));
                            }
                        } else {
                            self.set_reg(instruction.a, Value::bool(false));
                        }
                    } else {
                        self.set_reg(instruction.a, Value::bool(false));
                    }
                }
                
                // ==================== Arrays ====================
                OpCode::NewArray => {
                    // Create new array via heap
                    // instruction.b = number of initial elements (from following registers)
                    let count = instruction.b as usize;
                    let mut elements = Vec::with_capacity(count);
                    let frame_base = self.frame_base();
                    let start_reg = instruction.c as usize;
                    for i in 0..count {
                        elements.push(self.registers[frame_base + start_reg + i]);
                    }
                    let idx = self.heap.alloc_array(HateArray::from_values(elements));
                    self.set_reg(instruction.a, make_heap_ptr(idx, TAG_ARRAY));
                }
                
                OpCode::GetIndex => {
                    // Array indexing: a = b[c]
                    let arr_val = self.reg(instruction.b);
                    let index_val = self.reg(instruction.c);
                    if let Some((idx, tag)) = decode_heap_ptr(arr_val) {
                        if tag == TAG_ARRAY {
                            if let Some(arr) = self.heap.get_array(idx) {
                                if let Some(i) = index_val.as_int() {
                                    let i = i as i64;
                                    let i = if i < 0 { 
                                        (arr.len() as i64 + i) as usize 
                                    } else { 
                                        i as usize 
                                    };
                                    let val = arr.get(i).unwrap_or(Value::null());
                                    self.set_reg(instruction.a, val);
                                } else {
                                    self.set_reg(instruction.a, Value::null());
                                }
                            } else {
                                self.set_reg(instruction.a, Value::null());
                            }
                        } else {
                            self.set_reg(instruction.a, Value::null());
                        }
                    } else {
                        self.set_reg(instruction.a, Value::null());
                    }
                }
                
                OpCode::SetIndex => {
                    // Array element setting: a[b] = c
                    let arr_val = self.reg(instruction.a);
                    let index_val = self.reg(instruction.b);
                    let value = self.reg(instruction.c);
                    if let Some((idx, tag)) = decode_heap_ptr(arr_val) {
                        if tag == TAG_ARRAY {
                            if let Some(arr) = self.heap.get_array_mut(idx) {
                                if let Some(i) = index_val.as_int() {
                                    let i = i as i64;
                                    let i = if i < 0 {
                                        (arr.len() as i64 + i) as usize
                                    } else {
                                        i as usize
                                    };
                                    // Extend array if necessary
                                    while arr.len() <= i {
                                        arr.push(Value::null());
                                    }
                                    arr.set(i, value);
                                }
                            }
                        }
                    }
                }
                
                OpCode::ArrayLen => {
                    // Get array length: a = len(b)
                    let arr_val = self.reg(instruction.b);
                    if let Some((idx, tag)) = decode_heap_ptr(arr_val) {
                        if tag == TAG_ARRAY {
                            if let Some(arr) = self.heap.get_array(idx) {
                                self.set_reg(instruction.a, Value::int(arr.len() as i32));
                            } else {
                                self.set_reg(instruction.a, Value::int(0));
                            }
                        } else {
                            self.set_reg(instruction.a, Value::int(0));
                        }
                    } else {
                        self.set_reg(instruction.a, Value::int(0));
                    }
                }
                
                OpCode::ArrayPush => {
                    // Push to array: a.push(b)
                    let arr_val = self.reg(instruction.a);
                    let value = self.reg(instruction.b);
                    if let Some((idx, tag)) = decode_heap_ptr(arr_val) {
                        if tag == TAG_ARRAY {
                            if let Some(arr) = self.heap.get_array_mut(idx) {
                                arr.push(value);
                            }
                        }
                    }
                }
                
                OpCode::ArrayPop => {
                    // Pop from array: a = b.pop()
                    let arr_val = self.reg(instruction.b);
                    if let Some((idx, tag)) = decode_heap_ptr(arr_val) {
                        if tag == TAG_ARRAY {
                            if let Some(arr) = self.heap.get_array_mut(idx) {
                                let val = arr.pop().unwrap_or(Value::null());
                                self.set_reg(instruction.a, val);
                            } else {
                                self.set_reg(instruction.a, Value::null());
                            }
                        } else {
                            self.set_reg(instruction.a, Value::null());
                        }
                    } else {
                        self.set_reg(instruction.a, Value::null());
                    }
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
                    } else if let Some((chunk_idx, func_idx)) = callee.as_closure_indices() {
                        // User-defined function call
                        let chunk_idx = chunk_idx as usize;
                        let func_idx = func_idx as usize;
                        
                        // Get the function from the chunk
                        let function = self.chunks[chunk_idx].functions[func_idx].clone();
                        
                        // Check arity
                        if argc != function.arity as usize {
                            return Err(HateError::WrongArity {
                                expected: function.arity as usize,
                                got: argc,
                            });
                        }
                        
                        // Save the current frame's return register
                        let return_reg = instruction.a;
                        
                        // Store the function's chunk
                        let new_chunk_idx = self.chunks.len();
                        self.chunks.push(function.chunk.clone());
                        
                        // Create a new call frame
                        // The base is after current frame's registers
                        let new_base = self.frame_base() + self.current_register_count();
                        
                        // Copy arguments to the new frame's registers
                        let frame_base = self.frame_base();
                        let arg_start = instruction.b as usize + 1;
                        for i in 0..argc {
                            let arg_val = self.registers[frame_base + arg_start + i];
                            self.ensure_register(new_base + i);
                            self.registers[new_base + i] = arg_val;
                        }
                        
                        // Push the new frame
                        self.frames.push(CallFrame::new(new_chunk_idx, new_base, return_reg));
                    } else {
                        // Not callable
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
                            let return_reg = frame.return_reg;
                            // Need to set in the NEW current frame (after pop)
                            let new_frame_base = self.frame_base();
                            self.registers[new_frame_base + return_reg as usize] = result;
                        }
                    } else {
                        return Ok(result);
                    }
                }
                
                OpCode::Closure => {
                    // Create a closure value
                    // imm16 contains the function index within the current chunk
                    let func_idx = instruction.imm16();
                    let current_chunk_idx = self.frames.last().unwrap().chunk_idx as u16;
                    
                    // Store as a closure value that references this chunk and function
                    let closure_val = Value::closure(current_chunk_idx, func_idx);
                    self.set_reg(instruction.a, closure_val);
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
                    // Loop back - offset is stored as positive value
                    let offset = instruction.imm16() as usize;
                    let frame = self.frames.last_mut().unwrap();
                    frame.ip = frame.ip - offset;
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
    
    /// Get the current frame's register count (for allocating new frame's base)
    fn current_register_count(&self) -> usize {
        if let Some(frame) = self.frames.last() {
            self.chunks.get(frame.chunk_idx)
                .map(|c| c.max_registers as usize)
                .unwrap_or(MAX_REGISTERS)
        } else {
            MAX_REGISTERS
        }
    }
    
    /// Ensure register array is large enough
    fn ensure_register(&mut self, idx: usize) {
        if idx >= self.registers.len() {
            self.registers.resize(idx + MAX_REGISTERS, Value::null());
        }
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
