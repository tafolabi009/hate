//! String interning for zero-copy string handling
//!
//! All strings in the Hate language are interned, meaning identical strings
//! share the same memory. This enables O(1) string equality checks via
//! pointer comparison.

use std::collections::HashMap;
use std::sync::RwLock;
use once_cell::sync::Lazy;
use ahash::RandomState;

/// A globally unique string identifier
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Symbol(u32);

impl Symbol {
    /// Get the string value of this symbol
    #[inline]
    pub fn as_str(&self) -> &'static str {
        INTERNER.read().unwrap().resolve(*self)
    }
    
    /// Get the raw index of this symbol
    #[inline]
    pub fn index(&self) -> u32 {
        self.0
    }
}

impl std::fmt::Debug for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Symbol({:?})", self.as_str())
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Thread-safe string interner
pub struct Interner {
    /// Map from string content to symbol index
    map: HashMap<&'static str, Symbol, RandomState>,
    /// Vector of all interned strings
    strings: Vec<&'static str>,
}

impl Interner {
    /// Create a new interner
    pub fn new() -> Self {
        let mut interner = Self {
            map: HashMap::with_hasher(RandomState::new()),
            strings: Vec::with_capacity(1024),
        };
        
        // Pre-intern common keywords and symbols
        interner.intern_static("let");
        interner.intern_static("mut");
        interner.intern_static("fn");
        interner.intern_static("return");
        interner.intern_static("if");
        interner.intern_static("else");
        interner.intern_static("while");
        interner.intern_static("for");
        interner.intern_static("in");
        interner.intern_static("match");
        interner.intern_static("true");
        interner.intern_static("false");
        interner.intern_static("null");
        interner.intern_static("class");
        interner.intern_static("import");
        interner.intern_static("export");
        interner.intern_static("async");
        interner.intern_static("await");
        interner.intern_static("i32");
        interner.intern_static("i64");
        interner.intern_static("u32");
        interner.intern_static("u64");
        interner.intern_static("f64");
        interner.intern_static("bool");
        interner.intern_static("str");
        interner.intern_static("Array");
        interner.intern_static("Result");
        interner.intern_static("Ok");
        interner.intern_static("Err");
        
        interner
    }
    
    /// Intern a static string (no allocation needed)
    fn intern_static(&mut self, s: &'static str) -> Symbol {
        if let Some(&symbol) = self.map.get(s) {
            return symbol;
        }
        
        let symbol = Symbol(self.strings.len() as u32);
        self.strings.push(s);
        self.map.insert(s, symbol);
        symbol
    }
    
    /// Intern a string, returning its symbol
    pub fn intern(&mut self, s: &str) -> Symbol {
        // Check if already interned
        if let Some(&symbol) = self.map.get(s) {
            return symbol;
        }
        
        // Allocate a new string and leak it (it lives forever)
        let leaked: &'static str = Box::leak(s.to_string().into_boxed_str());
        let symbol = Symbol(self.strings.len() as u32);
        self.strings.push(leaked);
        self.map.insert(leaked, symbol);
        symbol
    }
    
    /// Resolve a symbol to its string
    pub fn resolve(&self, symbol: Symbol) -> &'static str {
        self.strings[symbol.0 as usize]
    }
    
    /// Get the number of interned strings
    pub fn len(&self) -> usize {
        self.strings.len()
    }
    
    /// Check if the interner is empty
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

impl Default for Interner {
    fn default() -> Self {
        Self::new()
    }
}

/// Global string interner
static INTERNER: Lazy<RwLock<Interner>> = Lazy::new(|| RwLock::new(Interner::new()));

/// Intern a string, returning its symbol
pub fn intern(s: &str) -> Symbol {
    INTERNER.write().unwrap().intern(s)
}

/// Resolve a symbol to its string
pub fn resolve(symbol: Symbol) -> &'static str {
    INTERNER.read().unwrap().resolve(symbol)
}

/// Get a pre-interned keyword symbol
pub mod keywords {
    use super::*;
    use once_cell::sync::Lazy;
    
    pub static LET: Lazy<Symbol> = Lazy::new(|| intern("let"));
    pub static MUT: Lazy<Symbol> = Lazy::new(|| intern("mut"));
    pub static FN: Lazy<Symbol> = Lazy::new(|| intern("fn"));
    pub static RETURN: Lazy<Symbol> = Lazy::new(|| intern("return"));
    pub static IF: Lazy<Symbol> = Lazy::new(|| intern("if"));
    pub static ELSE: Lazy<Symbol> = Lazy::new(|| intern("else"));
    pub static WHILE: Lazy<Symbol> = Lazy::new(|| intern("while"));
    pub static FOR: Lazy<Symbol> = Lazy::new(|| intern("for"));
    pub static IN: Lazy<Symbol> = Lazy::new(|| intern("in"));
    pub static MATCH: Lazy<Symbol> = Lazy::new(|| intern("match"));
    pub static TRUE: Lazy<Symbol> = Lazy::new(|| intern("true"));
    pub static FALSE: Lazy<Symbol> = Lazy::new(|| intern("false"));
    pub static NULL: Lazy<Symbol> = Lazy::new(|| intern("null"));
    pub static CLASS: Lazy<Symbol> = Lazy::new(|| intern("class"));
    pub static IMPORT: Lazy<Symbol> = Lazy::new(|| intern("import"));
    pub static EXPORT: Lazy<Symbol> = Lazy::new(|| intern("export"));
    pub static ASYNC: Lazy<Symbol> = Lazy::new(|| intern("async"));
    pub static AWAIT: Lazy<Symbol> = Lazy::new(|| intern("await"));
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_intern_same_string() {
        let s1 = intern("hello");
        let s2 = intern("hello");
        assert_eq!(s1, s2);
    }
    
    #[test]
    fn test_intern_different_strings() {
        let s1 = intern("hello");
        let s2 = intern("world");
        assert_ne!(s1, s2);
    }
    
    #[test]
    fn test_resolve() {
        let s = intern("test_string");
        assert_eq!(s.as_str(), "test_string");
    }
    
    #[test]
    fn test_symbol_size() {
        // Symbol should be 4 bytes
        assert_eq!(std::mem::size_of::<Symbol>(), 4);
    }
}
