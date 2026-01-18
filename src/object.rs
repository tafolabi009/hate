//! Object system for Hate
//!
//! Provides GC-managed arrays, objects, classes, and closures with:
//! - Hidden classes for efficient property access
//! - Inline caching support
//! - Prototype chain for inheritance

use crate::value::Value;
use crate::intern::Symbol;
use ahash::AHashMap;

// Type tags for GC objects
pub const TYPE_STRING: u16 = 1;
pub const TYPE_ARRAY: u16 = 2;
pub const TYPE_OBJECT: u16 = 3;
pub const TYPE_CLOSURE: u16 = 4;
pub const TYPE_UPVALUE: u16 = 5;
pub const TYPE_CLASS: u16 = 6;
pub const TYPE_INSTANCE: u16 = 7;
pub const TYPE_BOUND_METHOD: u16 = 8;
pub const TYPE_RANGE: u16 = 9;
pub const TYPE_ITERATOR: u16 = 10;
pub const TYPE_PROMISE: u16 = 11;

/// Hidden class (shape/map) for objects
/// Enables V8-style inline caching
#[derive(Debug, Clone)]
pub struct HiddenClass {
    /// Property name -> slot index
    properties: AHashMap<Symbol, u16>,
    /// Transition table for adding properties
    transitions: AHashMap<Symbol, u32>,
    /// Parent hidden class (for prototype chain)
    parent: Option<u32>,
    /// Number of properties
    property_count: u16,
    /// Is this class frozen?
    frozen: bool,
    /// Is this class sealed?
    sealed: bool,
}

impl HiddenClass {
    pub fn new() -> Self {
        Self {
            properties: AHashMap::new(),
            transitions: AHashMap::new(),
            parent: None,
            property_count: 0,
            frozen: false,
            sealed: false,
        }
    }
    
    pub fn with_parent(parent: u32) -> Self {
        Self {
            properties: AHashMap::new(),
            transitions: AHashMap::new(),
            parent: Some(parent),
            property_count: 0,
            frozen: false,
            sealed: false,
        }
    }
    
    /// Add a property, returning its slot index
    pub fn add_property(&mut self, name: Symbol) -> Option<u16> {
        if self.frozen || self.sealed {
            return None;
        }
        
        if self.properties.contains_key(&name) {
            return Some(self.properties[&name]);
        }
        
        let slot = self.property_count;
        self.properties.insert(name, slot);
        self.property_count += 1;
        Some(slot)
    }
    
    /// Lookup a property slot
    pub fn lookup(&self, name: Symbol) -> Option<u16> {
        self.properties.get(&name).copied()
    }
    
    /// Get all property names
    pub fn property_names(&self) -> Vec<Symbol> {
        let mut names: Vec<_> = self.properties.iter().collect();
        names.sort_by_key(|(_, &slot)| slot);
        names.into_iter().map(|(name, _)| *name).collect()
    }
}

impl Default for HiddenClass {
    fn default() -> Self {
        Self::new()
    }
}

/// Object storage (manages all objects and hidden classes)
pub struct ObjectStore {
    /// All hidden classes
    hidden_classes: Vec<HiddenClass>,
    /// The empty hidden class index
    empty_class: u32,
}

impl ObjectStore {
    pub fn new() -> Self {
        let mut store = Self {
            hidden_classes: Vec::new(),
            empty_class: 0,
        };
        // Create the empty hidden class
        store.hidden_classes.push(HiddenClass::new());
        store
    }
    
    /// Get or create a hidden class with the given property added
    pub fn transition(&mut self, from_class: u32, property: Symbol) -> u32 {
        // Check if transition already exists
        if let Some(&to_class) = self.hidden_classes[from_class as usize].transitions.get(&property) {
            return to_class;
        }
        
        // Create new hidden class
        let mut new_class = self.hidden_classes[from_class as usize].clone();
        new_class.add_property(property);
        new_class.transitions.clear(); // Clear transitions in the new class
        
        let new_index = self.hidden_classes.len() as u32;
        self.hidden_classes.push(new_class);
        
        // Add transition to original class
        self.hidden_classes[from_class as usize].transitions.insert(property, new_index);
        
        new_index
    }
    
    /// Get a hidden class by index
    pub fn get_class(&self, index: u32) -> &HiddenClass {
        &self.hidden_classes[index as usize]
    }
    
    /// Get a mutable hidden class
    pub fn get_class_mut(&mut self, index: u32) -> &mut HiddenClass {
        &mut self.hidden_classes[index as usize]
    }
    
    pub fn empty_class(&self) -> u32 {
        self.empty_class
    }
}

impl Default for ObjectStore {
    fn default() -> Self {
        Self::new()
    }
}

/// A Hate array
#[derive(Debug, Clone)]
pub struct HateArray {
    /// Array elements
    pub elements: Vec<Value>,
}

impl HateArray {
    pub fn new() -> Self {
        Self { elements: Vec::new() }
    }
    
    pub fn with_capacity(capacity: usize) -> Self {
        Self { elements: Vec::with_capacity(capacity) }
    }
    
    pub fn from_values(values: Vec<Value>) -> Self {
        Self { elements: values }
    }
    
    pub fn len(&self) -> usize {
        self.elements.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
    
    pub fn push(&mut self, value: Value) {
        self.elements.push(value);
    }
    
    pub fn pop(&mut self) -> Option<Value> {
        self.elements.pop()
    }
    
    pub fn get(&self, index: usize) -> Option<Value> {
        self.elements.get(index).copied()
    }
    
    pub fn set(&mut self, index: usize, value: Value) -> bool {
        if index < self.elements.len() {
            self.elements[index] = value;
            true
        } else {
            false
        }
    }
    
    /// Array.map - allocate exact size
    pub fn map<F>(&self, f: F) -> HateArray
    where
        F: Fn(Value) -> Value,
    {
        HateArray {
            elements: self.elements.iter().map(|&v| f(v)).collect(),
        }
    }
    
    /// Array.filter - two-pass for exact allocation
    pub fn filter<F>(&self, f: F) -> HateArray
    where
        F: Fn(Value) -> bool,
    {
        HateArray {
            elements: self.elements.iter().filter(|&&v| f(v)).copied().collect(),
        }
    }
    
    /// Array.reduce
    pub fn reduce<F>(&self, init: Value, f: F) -> Value
    where
        F: Fn(Value, Value) -> Value,
    {
        self.elements.iter().fold(init, |acc, &v| f(acc, v))
    }
    
    /// Array.find
    pub fn find<F>(&self, f: F) -> Value
    where
        F: Fn(Value) -> bool,
    {
        self.elements.iter().find(|&&v| f(v)).copied().unwrap_or(Value::null())
    }
    
    /// Array.findIndex
    pub fn find_index<F>(&self, f: F) -> i32
    where
        F: Fn(Value) -> bool,
    {
        self.elements.iter().position(|&v| f(v)).map(|i| i as i32).unwrap_or(-1)
    }
    
    /// Array.some - short-circuit on true
    pub fn some<F>(&self, f: F) -> bool
    where
        F: Fn(Value) -> bool,
    {
        self.elements.iter().any(|&v| f(v))
    }
    
    /// Array.every - short-circuit on false
    pub fn every<F>(&self, f: F) -> bool
    where
        F: Fn(Value) -> bool,
    {
        self.elements.iter().all(|&v| f(v))
    }
    
    /// In-place reverse
    pub fn reverse(&mut self) {
        self.elements.reverse();
    }
    
    /// Slice (zero-copy when possible - here we copy for simplicity)
    pub fn slice(&self, start: i32, end: i32) -> HateArray {
        let len = self.elements.len() as i32;
        let start = if start < 0 { (len + start).max(0) as usize } else { start.min(len) as usize };
        let end = if end < 0 { (len + end).max(0) as usize } else { end.min(len) as usize };
        
        if start >= end {
            return HateArray::new();
        }
        
        HateArray {
            elements: self.elements[start..end].to_vec(),
        }
    }
    
    /// Concat - preallocate total size
    pub fn concat(&self, other: &HateArray) -> HateArray {
        let mut result = Vec::with_capacity(self.elements.len() + other.elements.len());
        result.extend_from_slice(&self.elements);
        result.extend_from_slice(&other.elements);
        HateArray { elements: result }
    }
    
    /// Join with separator - single allocation
    pub fn join(&self, separator: &str) -> String {
        // Calculate total length first
        let mut total_len = 0;
        for (i, elem) in self.elements.iter().enumerate() {
            if i > 0 {
                total_len += separator.len();
            }
            total_len += format!("{}", elem).len();
        }
        
        let mut result = String::with_capacity(total_len);
        for (i, elem) in self.elements.iter().enumerate() {
            if i > 0 {
                result.push_str(separator);
            }
            result.push_str(&format!("{}", elem));
        }
        result
    }
    
    /// Includes
    pub fn includes(&self, value: Value) -> bool {
        self.elements.iter().any(|&v| v.eq(value))
    }
    
    /// indexOf
    pub fn index_of(&self, value: Value) -> i32 {
        self.elements.iter().position(|&v| v.eq(value)).map(|i| i as i32).unwrap_or(-1)
    }
    
    /// lastIndexOf
    pub fn last_index_of(&self, value: Value) -> i32 {
        self.elements.iter().rposition(|&v| v.eq(value)).map(|i| i as i32).unwrap_or(-1)
    }
    
    /// Stable sort (Timsort via std)
    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: Fn(Value, Value) -> std::cmp::Ordering,
    {
        self.elements.sort_by(|&a, &b| compare(a, b));
    }
    
    /// Default numeric sort
    pub fn sort(&mut self) {
        self.elements.sort_by(|&a, &b| {
            let a_num = a.to_number().unwrap_or(f64::NAN);
            let b_num = b.to_number().unwrap_or(f64::NAN);
            a_num.partial_cmp(&b_num).unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    
    /// Flat (one level)
    pub fn flat(&self) -> HateArray {
        // For now, just return a copy since we don't have nested arrays yet
        self.clone()
    }
    
    /// Fill
    pub fn fill(&mut self, value: Value, start: usize, end: usize) {
        let end = end.min(self.elements.len());
        let start = start.min(end);
        for i in start..end {
            self.elements[i] = value;
        }
    }
    
    /// Splice
    pub fn splice(&mut self, start: usize, delete_count: usize, items: &[Value]) -> HateArray {
        let removed: Vec<Value> = self.elements.drain(start..start + delete_count.min(self.elements.len() - start)).collect();
        for (i, &item) in items.iter().enumerate() {
            self.elements.insert(start + i, item);
        }
        HateArray { elements: removed }
    }
}

impl Default for HateArray {
    fn default() -> Self {
        Self::new()
    }
}

/// A Hate object (hash map with hidden class)
#[derive(Debug, Clone)]
pub struct HateObject {
    /// Hidden class index
    pub class: u32,
    /// Property values (indexed by slot from hidden class)
    pub slots: Vec<Value>,
    /// Prototype (for inheritance)
    pub prototype: Option<Box<HateObject>>,
    /// Is frozen
    pub frozen: bool,
    /// Is sealed
    pub sealed: bool,
}

impl HateObject {
    pub fn new(empty_class: u32) -> Self {
        Self {
            class: empty_class,
            slots: Vec::new(),
            prototype: None,
            frozen: false,
            sealed: false,
        }
    }
    
    /// Get property value by slot
    pub fn get_slot(&self, slot: u16) -> Value {
        self.slots.get(slot as usize).copied().unwrap_or(Value::null())
    }
    
    /// Set property value by slot
    pub fn set_slot(&mut self, slot: u16, value: Value) {
        if !self.frozen {
            let slot = slot as usize;
            if slot >= self.slots.len() {
                self.slots.resize(slot + 1, Value::null());
            }
            self.slots[slot] = value;
        }
    }
    
    /// Get all property values
    pub fn values(&self) -> &[Value] {
        &self.slots
    }
    
    /// Freeze the object
    pub fn freeze(&mut self) {
        self.frozen = true;
        self.sealed = true;
    }
    
    /// Seal the object
    pub fn seal(&mut self) {
        self.sealed = true;
    }
}

impl Default for HateObject {
    fn default() -> Self {
        Self::new(0)
    }
}

/// A Hate class
#[derive(Debug, Clone)]
pub struct HateClass {
    /// Class name
    pub name: Symbol,
    /// Superclass (if any)
    pub superclass: Option<Box<HateClass>>,
    /// Methods (name -> closure index)
    pub methods: AHashMap<Symbol, u32>,
    /// Static methods
    pub statics: AHashMap<Symbol, u32>,
    /// Constructor function index
    pub constructor: Option<u32>,
}

impl HateClass {
    pub fn new(name: Symbol) -> Self {
        Self {
            name,
            superclass: None,
            methods: AHashMap::new(),
            statics: AHashMap::new(),
            constructor: None,
        }
    }
    
    /// Add a method
    pub fn add_method(&mut self, name: Symbol, func_idx: u32) {
        self.methods.insert(name, func_idx);
    }
    
    /// Add a static method
    pub fn add_static(&mut self, name: Symbol, func_idx: u32) {
        self.statics.insert(name, func_idx);
    }
    
    /// Look up a method (with prototype chain)
    pub fn lookup_method(&self, name: Symbol) -> Option<u32> {
        if let Some(&idx) = self.methods.get(&name) {
            return Some(idx);
        }
        if let Some(ref superclass) = self.superclass {
            return superclass.lookup_method(name);
        }
        None
    }
}

/// A Hate class instance
#[derive(Debug, Clone)]
pub struct HateInstance {
    /// The class this is an instance of
    pub class: HateClass,
    /// Instance fields
    pub fields: HateObject,
}

impl HateInstance {
    pub fn new(class: HateClass, empty_class: u32) -> Self {
        Self {
            class,
            fields: HateObject::new(empty_class),
        }
    }
}

/// A range (start..end or start..=end)
#[derive(Debug, Clone, Copy)]
pub struct HateRange {
    pub start: i64,
    pub end: i64,
    pub inclusive: bool,
    pub step: i64,
}

impl HateRange {
    pub fn new(start: i64, end: i64, inclusive: bool) -> Self {
        Self {
            start,
            end,
            inclusive,
            step: 1,
        }
    }
    
    pub fn with_step(start: i64, end: i64, inclusive: bool, step: i64) -> Self {
        Self { start, end, inclusive, step }
    }
    
    pub fn len(&self) -> usize {
        if self.step == 0 {
            return 0;
        }
        
        let diff = if self.inclusive {
            self.end - self.start + 1
        } else {
            self.end - self.start
        };
        
        if diff <= 0 {
            0
        } else {
            (diff / self.step.abs()) as usize
        }
    }
    
    pub fn contains(&self, value: i64) -> bool {
        if self.inclusive {
            value >= self.start && value <= self.end
        } else {
            value >= self.start && value < self.end
        }
    }
}

/// An iterator
#[derive(Debug, Clone)]
pub enum HateIterator {
    Range { range: HateRange, current: i64 },
    Array { array: HateArray, index: usize },
    Object { keys: Vec<Symbol>, index: usize },
    String { string: String, index: usize },
}

impl HateIterator {
    pub fn from_range(range: HateRange) -> Self {
        HateIterator::Range { range, current: range.start }
    }
    
    pub fn from_array(array: HateArray) -> Self {
        HateIterator::Array { array, index: 0 }
    }
    
    pub fn from_string(string: String) -> Self {
        HateIterator::String { string, index: 0 }
    }
    
    /// Get next value, returning (value, done)
    pub fn next(&mut self) -> (Value, bool) {
        match self {
            HateIterator::Range { range, current } => {
                let done = if range.inclusive {
                    *current > range.end
                } else {
                    *current >= range.end
                };
                
                if done {
                    (Value::null(), true)
                } else {
                    let value = Value::int64(*current);
                    *current += range.step;
                    (value, false)
                }
            }
            HateIterator::Array { array, index } => {
                if *index >= array.len() {
                    (Value::null(), true)
                } else {
                    let value = array.get(*index).unwrap_or(Value::null());
                    *index += 1;
                    (value, false)
                }
            }
            HateIterator::Object { keys, index } => {
                if *index >= keys.len() {
                    (Value::null(), true)
                } else {
                    let key = keys[*index];
                    *index += 1;
                    // Return key as string value
                    (Value::string(key.index()), false)
                }
            }
            HateIterator::String { string, index } => {
                let chars: Vec<char> = string.chars().collect();
                if *index >= chars.len() {
                    (Value::null(), true)
                } else {
                    let ch = chars[*index];
                    *index += 1;
                    // Return character as string
                    use crate::intern::intern;
                    let s = intern(&ch.to_string());
                    (Value::string(s.index()), false)
                }
            }
        }
    }
}

/// Promise state
#[derive(Debug, Clone)]
pub enum PromiseState {
    Pending,
    Fulfilled(Value),
    Rejected(Value),
}

/// A Promise for async/await
#[derive(Debug, Clone)]
pub struct HatePromise {
    pub state: PromiseState,
    pub then_callbacks: Vec<u32>,  // Closure indices
    pub catch_callbacks: Vec<u32>, // Closure indices
}

impl HatePromise {
    pub fn new() -> Self {
        Self {
            state: PromiseState::Pending,
            then_callbacks: Vec::new(),
            catch_callbacks: Vec::new(),
        }
    }
    
    pub fn resolve(&mut self, value: Value) {
        if matches!(self.state, PromiseState::Pending) {
            self.state = PromiseState::Fulfilled(value);
        }
    }
    
    pub fn reject(&mut self, error: Value) {
        if matches!(self.state, PromiseState::Pending) {
            self.state = PromiseState::Rejected(error);
        }
    }
    
    pub fn is_pending(&self) -> bool {
        matches!(self.state, PromiseState::Pending)
    }
    
    pub fn is_fulfilled(&self) -> bool {
        matches!(self.state, PromiseState::Fulfilled(_))
    }
    
    pub fn is_rejected(&self) -> bool {
        matches!(self.state, PromiseState::Rejected(_))
    }
}

impl Default for HatePromise {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_array_operations() {
        let mut arr = HateArray::new();
        arr.push(Value::int(1));
        arr.push(Value::int(2));
        arr.push(Value::int(3));
        
        assert_eq!(arr.len(), 3);
        assert_eq!(arr.get(1), Some(Value::int(2)));
        
        let popped = arr.pop();
        assert_eq!(popped, Some(Value::int(3)));
        assert_eq!(arr.len(), 2);
    }
    
    #[test]
    fn test_array_slice() {
        let arr = HateArray::from_values(vec![
            Value::int(1),
            Value::int(2),
            Value::int(3),
            Value::int(4),
            Value::int(5),
        ]);
        
        let slice = arr.slice(1, 4);
        assert_eq!(slice.len(), 3);
        assert_eq!(slice.get(0), Some(Value::int(2)));
        assert_eq!(slice.get(2), Some(Value::int(4)));
    }
    
    #[test]
    fn test_range_iterator() {
        let range = HateRange::new(1, 5, false);
        let mut iter = HateIterator::from_range(range);
        
        let (v1, d1) = iter.next();
        assert_eq!(v1.as_int(), Some(1));
        assert!(!d1);
        
        let (v2, _) = iter.next();
        assert_eq!(v2.as_int(), Some(2));
        
        // Skip to end
        iter.next();
        iter.next();
        let (_, done) = iter.next();
        assert!(done);
    }
    
    #[test]
    fn test_hidden_class() {
        let mut store = ObjectStore::new();
        
        let class1 = store.transition(0, crate::intern::intern("x"));
        let class2 = store.transition(class1, crate::intern::intern("y"));
        
        let class = store.get_class(class2);
        assert_eq!(class.property_count, 2);
        assert_eq!(class.lookup(crate::intern::intern("x")), Some(0));
        assert_eq!(class.lookup(crate::intern::intern("y")), Some(1));
    }
}
