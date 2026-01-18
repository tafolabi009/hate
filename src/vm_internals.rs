//! Enhanced VM internals with object system integration
//!
//! Provides:
//! - Object and array allocation
//! - Property access with inline caching
//! - Class instantiation
//! - Closure handling

use crate::value::Value;
use crate::object::{
    HateArray, HateObject, HateClass, HateInstance, HateIterator, HatePromise, ObjectStore,
};
use std::collections::HashMap;

/// Inline cache entry for property access
#[derive(Clone, Copy, Default)]
pub struct InlineCache {
    /// Hidden class ID at time of caching
    pub class_id: u32,
    /// Property slot index
    pub slot: u16,
    /// Cache hit count
    pub hits: u16,
}

impl InlineCache {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Check if this cache entry matches
    pub fn check(&self, class_id: u32) -> Option<u16> {
        if self.class_id == class_id {
            Some(self.slot)
        } else {
            None
        }
    }
    
    /// Update the cache
    pub fn update(&mut self, class_id: u32, slot: u16) {
        self.class_id = class_id;
        self.slot = slot;
        self.hits = 0;
    }
    
    /// Record a cache hit
    pub fn hit(&mut self) {
        self.hits = self.hits.saturating_add(1);
    }
}

/// Call site info for devirtualization
#[derive(Clone)]
pub struct CallSite {
    /// Target function observed at this call site
    pub target: Option<u32>,
    /// Number of times this target was called
    pub call_count: u32,
    /// Is this site monomorphic?
    pub monomorphic: bool,
}

impl CallSite {
    pub fn new() -> Self {
        Self {
            target: None,
            call_count: 0,
            monomorphic: true,
        }
    }
    
    /// Record a call to a target
    pub fn record(&mut self, target: u32) {
        if let Some(prev) = self.target {
            if prev != target {
                self.monomorphic = false;
            }
        }
        self.target = Some(target);
        self.call_count += 1;
    }
}

impl Default for CallSite {
    fn default() -> Self {
        Self::new()
    }
}

/// Exception handler
#[derive(Clone, Copy)]
pub struct ExceptionHandler {
    /// Start of try block (instruction index)
    pub try_start: usize,
    /// End of try block
    pub try_end: usize,
    /// Catch handler location
    pub catch_ip: usize,
    /// Finally handler location (if any)
    pub finally_ip: Option<usize>,
    /// Exception register
    pub exception_reg: u8,
}

/// Heap storage for VM objects
pub struct Heap {
    /// All arrays
    pub arrays: Vec<HateArray>,
    /// All objects  
    pub objects: Vec<HateObject>,
    /// All classes
    pub classes: Vec<HateClass>,
    /// All instances
    pub instances: Vec<HateInstance>,
    /// All closures (function index + upvalues)
    pub closures: Vec<Closure>,
    /// All iterators
    pub iterators: Vec<HateIterator>,
    /// All promises
    pub promises: Vec<HatePromise>,
    /// Object store for hidden classes
    pub object_store: ObjectStore,
    /// Free lists for reuse
    array_free: Vec<usize>,
    object_free: Vec<usize>,
}

/// A closure (function + captured environment)
#[derive(Clone)]
pub struct Closure {
    /// Index of the function in chunks
    pub function_idx: usize,
    /// Captured upvalues
    pub upvalues: Vec<Upvalue>,
}

/// An upvalue (captured variable)
#[derive(Clone)]
pub enum Upvalue {
    /// Open upvalue - still on the stack
    Open { stack_idx: usize },
    /// Closed upvalue - moved to heap
    Closed { value: Value },
}

impl Heap {
    pub fn new() -> Self {
        Self {
            arrays: Vec::new(),
            objects: Vec::new(),
            classes: Vec::new(),
            instances: Vec::new(),
            closures: Vec::new(),
            iterators: Vec::new(),
            promises: Vec::new(),
            object_store: ObjectStore::new(),
            array_free: Vec::new(),
            object_free: Vec::new(),
        }
    }
    
    /// Allocate a new array, returning its index
    pub fn alloc_array(&mut self, arr: HateArray) -> usize {
        if let Some(idx) = self.array_free.pop() {
            self.arrays[idx] = arr;
            idx
        } else {
            let idx = self.arrays.len();
            self.arrays.push(arr);
            idx
        }
    }
    
    /// Get an array
    pub fn get_array(&self, idx: usize) -> Option<&HateArray> {
        self.arrays.get(idx)
    }
    
    /// Get a mutable array
    pub fn get_array_mut(&mut self, idx: usize) -> Option<&mut HateArray> {
        self.arrays.get_mut(idx)
    }
    
    /// Allocate a new object
    pub fn alloc_object(&mut self, obj: HateObject) -> usize {
        if let Some(idx) = self.object_free.pop() {
            self.objects[idx] = obj;
            idx
        } else {
            let idx = self.objects.len();
            self.objects.push(obj);
            idx
        }
    }
    
    /// Get an object
    pub fn get_object(&self, idx: usize) -> Option<&HateObject> {
        self.objects.get(idx)
    }
    
    /// Get a mutable object
    pub fn get_object_mut(&mut self, idx: usize) -> Option<&mut HateObject> {
        self.objects.get_mut(idx)
    }
    
    /// Allocate a closure
    pub fn alloc_closure(&mut self, closure: Closure) -> usize {
        let idx = self.closures.len();
        self.closures.push(closure);
        idx
    }
    
    /// Get a closure
    pub fn get_closure(&self, idx: usize) -> Option<&Closure> {
        self.closures.get(idx)
    }
    
    /// Allocate an iterator
    pub fn alloc_iterator(&mut self, iter: HateIterator) -> usize {
        let idx = self.iterators.len();
        self.iterators.push(iter);
        idx
    }
    
    /// Get a mutable iterator
    pub fn get_iterator_mut(&mut self, idx: usize) -> Option<&mut HateIterator> {
        self.iterators.get_mut(idx)
    }
    
    /// Allocate a class
    pub fn alloc_class(&mut self, class: HateClass) -> usize {
        let idx = self.classes.len();
        self.classes.push(class);
        idx
    }
    
    /// Get a class
    pub fn get_class(&self, idx: usize) -> Option<&HateClass> {
        self.classes.get(idx)
    }
    
    /// Allocate an instance
    pub fn alloc_instance(&mut self, instance: HateInstance) -> usize {
        let idx = self.instances.len();
        self.instances.push(instance);
        idx
    }
    
    /// Get an instance
    pub fn get_instance(&self, idx: usize) -> Option<&HateInstance> {
        self.instances.get(idx)
    }
    
    /// Get a mutable instance
    pub fn get_instance_mut(&mut self, idx: usize) -> Option<&mut HateInstance> {
        self.instances.get_mut(idx)
    }
    
    /// Allocate a promise
    pub fn alloc_promise(&mut self, promise: HatePromise) -> usize {
        let idx = self.promises.len();
        self.promises.push(promise);
        idx
    }
    
    /// Get a mutable promise
    pub fn get_promise_mut(&mut self, idx: usize) -> Option<&mut HatePromise> {
        self.promises.get_mut(idx)
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

/// Value tags for heap objects (stored in lower bits of pointer)
pub const TAG_ARRAY: u64 = 0x01;
pub const TAG_OBJECT: u64 = 0x02;
pub const TAG_CLOSURE: u64 = 0x03;
pub const TAG_CLASS: u64 = 0x04;
pub const TAG_INSTANCE: u64 = 0x05;
pub const TAG_ITERATOR: u64 = 0x06;
pub const TAG_PROMISE: u64 = 0x07;

/// Create a heap pointer value
pub fn make_heap_ptr(index: usize, tag: u64) -> Value {
    // Encode index and tag into the pointer value
    let encoded = ((index as u64) << 8) | tag;
    unsafe { Value::from_raw_ptr(encoded as *mut u8) }
}

/// Decode a heap pointer value
pub fn decode_heap_ptr(value: Value) -> Option<(usize, u64)> {
    if !value.is_ptr() {
        return None;
    }
    let encoded = value.as_ptr_unchecked::<u8>() as u64;
    let tag = encoded & 0xFF;
    let index = (encoded >> 8) as usize;
    Some((index, tag))
}

/// Check if value is an array
pub fn is_heap_array(value: Value) -> bool {
    decode_heap_ptr(value).map(|(_, tag)| tag == TAG_ARRAY).unwrap_or(false)
}

/// Check if value is an object
pub fn is_heap_object(value: Value) -> bool {
    decode_heap_ptr(value).map(|(_, tag)| tag == TAG_OBJECT).unwrap_or(false)
}

/// Check if value is a closure
pub fn is_heap_closure(value: Value) -> bool {
    decode_heap_ptr(value).map(|(_, tag)| tag == TAG_CLOSURE).unwrap_or(false)
}

/// Check if value is an iterator
pub fn is_heap_iterator(value: Value) -> bool {
    decode_heap_ptr(value).map(|(_, tag)| tag == TAG_ITERATOR).unwrap_or(false)
}

/// Call frame with exception handling
#[derive(Clone)]
pub struct EnhancedCallFrame {
    /// Chunk index
    pub chunk_idx: usize,
    /// Instruction pointer
    pub ip: usize,
    /// Base register
    pub base: usize,
    /// Return register
    pub return_reg: u8,
    /// Closure index (if this is a closure call)
    pub closure_idx: Option<usize>,
    /// Exception handlers for this frame
    pub exception_handlers: Vec<ExceptionHandler>,
    /// Is this an async frame?
    pub is_async: bool,
}

impl EnhancedCallFrame {
    pub fn new(chunk_idx: usize, base: usize, return_reg: u8) -> Self {
        Self {
            chunk_idx,
            ip: 0,
            base,
            return_reg,
            closure_idx: None,
            exception_handlers: Vec::new(),
            is_async: false,
        }
    }
    
    /// Find exception handler for current IP
    pub fn find_handler(&self, ip: usize) -> Option<&ExceptionHandler> {
        self.exception_handlers
            .iter()
            .find(|h| ip >= h.try_start && ip < h.try_end)
    }
}

/// Async state machine state
#[derive(Clone)]
pub struct AsyncState {
    /// Current state index
    pub state: usize,
    /// Saved registers
    pub registers: Vec<Value>,
    /// Promise index
    pub promise_idx: usize,
    /// Original frame
    pub frame: EnhancedCallFrame,
}

/// Microtask queue for promises
pub struct MicrotaskQueue {
    tasks: Vec<Microtask>,
}

#[derive(Clone)]
pub struct Microtask {
    pub promise_idx: usize,
    pub callback_idx: usize,
    pub value: Value,
}

impl MicrotaskQueue {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }
    
    pub fn enqueue(&mut self, task: Microtask) {
        self.tasks.push(task);
    }
    
    pub fn dequeue(&mut self) -> Option<Microtask> {
        if self.tasks.is_empty() {
            None
        } else {
            Some(self.tasks.remove(0))
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

impl Default for MicrotaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// VM profiling data
#[derive(Default)]
pub struct ProfileData {
    /// Instruction execution counts
    pub instruction_counts: HashMap<u8, u64>,
    /// Time spent per opcode (nanoseconds)
    pub opcode_time_ns: HashMap<u8, u64>,
    /// Inline cache hit rate (hits, misses)
    pub cache_stats: (u64, u64),
    /// GC pause times
    pub gc_pauses: Vec<u64>,
    /// Total bytecodes executed
    pub total_instructions: u64,
}

impl ProfileData {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn record_instruction(&mut self, opcode: u8) {
        *self.instruction_counts.entry(opcode).or_insert(0) += 1;
        self.total_instructions += 1;
    }
    
    pub fn record_cache_hit(&mut self) {
        self.cache_stats.0 += 1;
    }
    
    pub fn record_cache_miss(&mut self) {
        self.cache_stats.1 += 1;
    }
    
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_stats.0 + self.cache_stats.1;
        if total == 0 {
            0.0
        } else {
            self.cache_stats.0 as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_heap_allocation() {
        let mut heap = Heap::new();
        
        let arr = HateArray::from_values(vec![Value::int(1), Value::int(2)]);
        let idx = heap.alloc_array(arr);
        
        let arr = heap.get_array(idx).unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr.get(0), Some(Value::int(1)));
    }
    
    #[test]
    fn test_heap_ptr_encoding() {
        let ptr = make_heap_ptr(42, TAG_ARRAY);
        let (index, tag) = decode_heap_ptr(ptr).unwrap();
        assert_eq!(index, 42);
        assert_eq!(tag, TAG_ARRAY);
    }
    
    #[test]
    fn test_inline_cache() {
        let mut cache = InlineCache::new();
        cache.update(5, 3);
        
        assert_eq!(cache.check(5), Some(3));
        assert_eq!(cache.check(6), None);
        
        cache.hit();
        assert_eq!(cache.hits, 1);
    }
    
    #[test]
    fn test_call_site() {
        let mut site = CallSite::new();
        site.record(10);
        site.record(10);
        assert!(site.monomorphic);
        
        site.record(20);
        assert!(!site.monomorphic);
    }
}
