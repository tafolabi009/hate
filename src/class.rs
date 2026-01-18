//! Class System and Inheritance
//!
//! Implements JavaScript-style prototypal inheritance with:
//! - Hidden classes for efficient property access
//! - Method lookup with inline caching
//! - Constructor functions
//! - Super binding

use crate::value::Value;
use crate::object::ObjectStore;
use crate::intern::Symbol;
use crate::bytecode::Chunk;
use ahash::AHashMap;

/// Maximum prototype chain depth (for optimization)
pub const MAX_PROTOTYPE_DEPTH: usize = 5;

/// Method resolution cache entry
#[derive(Clone, Copy)]
pub struct MethodCache {
    /// Class ID this cache is for
    class_id: u32,
    /// Method name hash
    method_hash: u32,
    /// Method function index
    method_idx: Option<u32>,
}

impl MethodCache {
    pub fn new() -> Self {
        Self {
            class_id: 0,
            method_hash: 0,
            method_idx: None,
        }
    }
    
    pub fn lookup(&self, class_id: u32, method_hash: u32) -> Option<u32> {
        if self.class_id == class_id && self.method_hash == method_hash {
            self.method_idx
        } else {
            None
        }
    }
    
    pub fn update(&mut self, class_id: u32, method_hash: u32, method_idx: Option<u32>) {
        self.class_id = class_id;
        self.method_hash = method_hash;
        self.method_idx = method_idx;
    }
}

impl Default for MethodCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Class metadata for the runtime
#[derive(Debug, Clone)]
pub struct ClassMeta {
    /// Class name
    pub name: Symbol,
    /// Superclass index (if any)
    pub superclass: Option<u32>,
    /// Instance method table (name hash -> function index)
    pub methods: AHashMap<u32, u32>,
    /// Static method table
    pub statics: AHashMap<u32, u32>,
    /// Constructor function index
    pub constructor: Option<u32>,
    /// Field names (for initialization)
    pub fields: Vec<Symbol>,
    /// Hidden class ID for instances
    pub instance_class: u32,
}

impl ClassMeta {
    pub fn new(name: Symbol, object_store: &mut ObjectStore) -> Self {
        let instance_class = object_store.empty_class();
        Self {
            name,
            superclass: None,
            methods: AHashMap::new(),
            statics: AHashMap::new(),
            constructor: None,
            fields: Vec::new(),
            instance_class,
        }
    }
    
    /// Add a method
    pub fn add_method(&mut self, name: Symbol, func_idx: u32) {
        let hash = hash_symbol(name);
        self.methods.insert(hash, func_idx);
    }
    
    /// Add a static method
    pub fn add_static(&mut self, name: Symbol, func_idx: u32) {
        let hash = hash_symbol(name);
        self.statics.insert(hash, func_idx);
    }
    
    /// Look up a method in this class only
    pub fn lookup_own_method(&self, name_hash: u32) -> Option<u32> {
        self.methods.get(&name_hash).copied()
    }
    
    /// Look up a static method
    pub fn lookup_static(&self, name_hash: u32) -> Option<u32> {
        self.statics.get(&name_hash).copied()
    }
}

/// Class registry for the VM
pub struct ClassRegistry {
    /// All classes
    classes: Vec<ClassMeta>,
    /// Name to class index
    name_to_class: AHashMap<u32, u32>,
    /// Method caches (indexed by call site)
    method_caches: Vec<MethodCache>,
}

impl ClassRegistry {
    pub fn new() -> Self {
        Self {
            classes: Vec::new(),
            name_to_class: AHashMap::new(),
            method_caches: Vec::new(),
        }
    }
    
    /// Register a new class
    pub fn register(&mut self, meta: ClassMeta) -> u32 {
        let idx = self.classes.len() as u32;
        let name_hash = hash_symbol(meta.name);
        self.name_to_class.insert(name_hash, idx);
        self.classes.push(meta);
        idx
    }
    
    /// Get a class by index
    pub fn get(&self, idx: u32) -> Option<&ClassMeta> {
        self.classes.get(idx as usize)
    }
    
    /// Get a mutable class
    pub fn get_mut(&mut self, idx: u32) -> Option<&mut ClassMeta> {
        self.classes.get_mut(idx as usize)
    }
    
    /// Look up a class by name
    pub fn lookup_by_name(&self, name: Symbol) -> Option<u32> {
        let hash = hash_symbol(name);
        self.name_to_class.get(&hash).copied()
    }
    
    /// Look up a method with caching
    pub fn lookup_method(
        &mut self,
        class_idx: u32,
        method_name: Symbol,
        cache_idx: usize,
    ) -> Option<u32> {
        let method_hash = hash_symbol(method_name);
        
        // Check cache first
        if cache_idx < self.method_caches.len() {
            if let Some(idx) = self.method_caches[cache_idx].lookup(class_idx, method_hash) {
                return Some(idx);
            }
        }
        
        // Walk prototype chain
        let mut current = Some(class_idx);
        let mut depth = 0;
        
        while let Some(class_id) = current {
            if depth > MAX_PROTOTYPE_DEPTH {
                break;
            }
            
            if let Some(class) = self.classes.get(class_id as usize) {
                if let Some(method_idx) = class.lookup_own_method(method_hash) {
                    // Update cache
                    self.ensure_cache(cache_idx);
                    self.method_caches[cache_idx].update(class_idx, method_hash, Some(method_idx));
                    return Some(method_idx);
                }
                current = class.superclass;
            } else {
                break;
            }
            
            depth += 1;
        }
        
        // Update cache with miss
        self.ensure_cache(cache_idx);
        self.method_caches[cache_idx].update(class_idx, method_hash, None);
        None
    }
    
    fn ensure_cache(&mut self, idx: usize) {
        while self.method_caches.len() <= idx {
            self.method_caches.push(MethodCache::new());
        }
    }
}

impl Default for ClassRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash a symbol for method lookup
pub fn hash_symbol(sym: Symbol) -> u32 {
    let mut hash: u32 = 2166136261;
    for b in sym.as_str().bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

/// Compiled class representation
#[derive(Debug, Clone)]
pub struct CompiledClass {
    /// Class metadata index
    pub meta_idx: u32,
    /// Bytecode for constructor
    pub constructor_chunk: Option<Chunk>,
    /// Bytecode for each method
    pub method_chunks: Vec<(Symbol, Chunk)>,
    /// Static method chunks
    pub static_chunks: Vec<(Symbol, Chunk)>,
}

/// Class compiler
pub struct ClassCompiler {
    /// Current class being compiled
    current_class: Option<Symbol>,
    /// Enclosing class (for nested classes)
    enclosing_class: Option<Symbol>,
    /// Super class name
    super_class: Option<Symbol>,
}

impl ClassCompiler {
    pub fn new() -> Self {
        Self {
            current_class: None,
            enclosing_class: None,
            super_class: None,
        }
    }
    
    /// Start compiling a class
    pub fn begin_class(&mut self, name: Symbol, superclass: Option<Symbol>) {
        self.enclosing_class = self.current_class;
        self.current_class = Some(name);
        self.super_class = superclass;
    }
    
    /// Finish compiling a class
    pub fn end_class(&mut self) {
        self.current_class = self.enclosing_class;
        self.enclosing_class = None;
        self.super_class = None;
    }
    
    /// Check if we're inside a class
    pub fn in_class(&self) -> bool {
        self.current_class.is_some()
    }
    
    /// Check if this class has a superclass
    pub fn has_superclass(&self) -> bool {
        self.super_class.is_some()
    }
    
    /// Get the current class name
    pub fn current_class_name(&self) -> Option<Symbol> {
        self.current_class
    }
}

impl Default for ClassCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Instance runtime representation
#[derive(Debug, Clone)]
pub struct InstanceRuntime {
    /// Class index
    pub class_idx: u32,
    /// Field values (indexed by slot)
    pub fields: Vec<Value>,
}

impl InstanceRuntime {
    pub fn new(class_idx: u32, field_count: usize) -> Self {
        Self {
            class_idx,
            fields: vec![Value::null(); field_count],
        }
    }
    
    pub fn get_field(&self, slot: usize) -> Value {
        self.fields.get(slot).copied().unwrap_or(Value::null())
    }
    
    pub fn set_field(&mut self, slot: usize, value: Value) {
        if slot >= self.fields.len() {
            self.fields.resize(slot + 1, Value::null());
        }
        self.fields[slot] = value;
    }
}

/// Bound method (method + receiver)
#[derive(Debug, Clone)]
pub struct BoundMethod {
    /// The receiver object (instance)
    pub receiver: Value,
    /// The method function index
    pub method_idx: u32,
}

/// Super call info
#[derive(Debug, Clone)]
pub struct SuperCall {
    /// The subclass's class index
    pub subclass_idx: u32,
    /// The method being called
    pub method_name: Symbol,
    /// The receiver instance
    pub receiver: Value,
}

impl SuperCall {
    pub fn resolve(&self, registry: &ClassRegistry) -> Option<u32> {
        let subclass = registry.get(self.subclass_idx)?;
        let superclass_idx = subclass.superclass?;
        
        // Look up method in superclass
        let method_hash = hash_symbol(self.method_name);
        let superclass = registry.get(superclass_idx)?;
        superclass.lookup_own_method(method_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intern::intern;
    
    #[test]
    fn test_class_registry() {
        let mut object_store = ObjectStore::new();
        let mut registry = ClassRegistry::new();
        
        let animal = ClassMeta::new(intern("Animal"), &mut object_store);
        let animal_idx = registry.register(animal);
        
        let mut dog = ClassMeta::new(intern("Dog"), &mut object_store);
        dog.superclass = Some(animal_idx);
        let dog_idx = registry.register(dog);
        
        assert!(registry.get(animal_idx).is_some());
        assert_eq!(registry.get(dog_idx).unwrap().superclass, Some(animal_idx));
    }
    
    #[test]
    fn test_method_cache() {
        let mut cache = MethodCache::new();
        
        cache.update(1, 100, Some(42));
        assert_eq!(cache.lookup(1, 100), Some(42));
        assert_eq!(cache.lookup(1, 200), None);
        assert_eq!(cache.lookup(2, 100), None);
    }
    
    #[test]
    fn test_method_lookup_with_inheritance() {
        let mut object_store = ObjectStore::new();
        let mut registry = ClassRegistry::new();
        
        // Create Animal with speak method
        let mut animal = ClassMeta::new(intern("Animal"), &mut object_store);
        animal.add_method(intern("speak"), 10);
        animal.add_method(intern("eat"), 11);
        let animal_idx = registry.register(animal);
        
        // Create Dog extending Animal
        let mut dog = ClassMeta::new(intern("Dog"), &mut object_store);
        dog.superclass = Some(animal_idx);
        dog.add_method(intern("bark"), 20);
        let dog_idx = registry.register(dog);
        
        // Dog should find its own methods
        let bark_idx = registry.lookup_method(dog_idx, intern("bark"), 0);
        assert_eq!(bark_idx, Some(20));
        
        // Dog should inherit Animal's methods
        let speak_idx = registry.lookup_method(dog_idx, intern("speak"), 1);
        assert_eq!(speak_idx, Some(10));
        
        // Non-existent method
        let fly_idx = registry.lookup_method(dog_idx, intern("fly"), 2);
        assert_eq!(fly_idx, None);
    }
    
    #[test]
    fn test_instance_runtime() {
        let mut instance = InstanceRuntime::new(0, 3);
        
        instance.set_field(0, Value::int(42));
        instance.set_field(2, Value::bool(true));
        
        assert_eq!(instance.get_field(0).as_int(), Some(42));
        assert_eq!(instance.get_field(1).is_null(), true);
        assert_eq!(instance.get_field(2).as_bool(), Some(true));
    }
}
