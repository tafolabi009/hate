//! Module System
//!
//! Implements import/export with:
//! - File-based module resolution
//! - Cyclic dependency handling
//! - Module caching
//! - Named and default exports

use crate::value::Value;
use crate::intern::{Symbol, intern};
use crate::bytecode::Chunk;
use crate::error::{HateError, HateResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use ahash::AHashMap;

/// Module state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    /// Not yet loaded
    Unloaded,
    /// Currently loading (for cycle detection)
    Loading,
    /// Loaded and exports are available
    Loaded,
    /// Failed to load
    Failed,
}

/// An export entry
#[derive(Debug, Clone)]
pub struct Export {
    /// Local name in the module
    pub local_name: Symbol,
    /// Exported as this name
    pub export_name: Symbol,
    /// Value (populated after execution)
    pub value: Value,
    /// Whether it's the default export
    pub is_default: bool,
}

/// An import request
#[derive(Debug, Clone)]
pub struct Import {
    /// The module specifier (path or name)
    pub specifier: String,
    /// Import bindings: (import_name, local_name)
    pub bindings: Vec<(Symbol, Symbol)>,
    /// Whether this is a namespace import (import * as name)
    pub namespace: Option<Symbol>,
    /// Whether this imports the default export
    pub default_binding: Option<Symbol>,
}

/// A module record
#[derive(Debug, Clone)]
pub struct Module {
    /// Module path (normalized)
    pub path: PathBuf,
    /// Module state
    pub state: ModuleState,
    /// Source code
    pub source: String,
    /// Compiled bytecode
    pub chunk: Option<Chunk>,
    /// Exports
    pub exports: Vec<Export>,
    /// Export name to index
    pub export_map: AHashMap<u32, usize>,
    /// Imports
    pub imports: Vec<Import>,
    /// Dependencies (module indices)
    pub dependencies: Vec<usize>,
    /// Module namespace object (for import *)
    pub namespace_object: Value,
}

impl Module {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            state: ModuleState::Unloaded,
            source: String::new(),
            chunk: None,
            exports: Vec::new(),
            export_map: AHashMap::new(),
            imports: Vec::new(),
            dependencies: Vec::new(),
            namespace_object: Value::null(),
        }
    }
    
    /// Add an export
    pub fn add_export(&mut self, local: Symbol, export_as: Symbol, is_default: bool) {
        let idx = self.exports.len();
        let hash = hash_symbol(export_as);
        self.export_map.insert(hash, idx);
        self.exports.push(Export {
            local_name: local,
            export_name: export_as,
            value: Value::null(),
            is_default,
        });
    }
    
    /// Look up an export by name
    pub fn get_export(&self, name: Symbol) -> Option<&Export> {
        let hash = hash_symbol(name);
        self.export_map.get(&hash).and_then(|&idx| self.exports.get(idx))
    }
    
    /// Get the default export
    pub fn get_default_export(&self) -> Option<&Export> {
        self.exports.iter().find(|e| e.is_default)
    }
    
    /// Set an export value
    pub fn set_export_value(&mut self, name: Symbol, value: Value) {
        let hash = hash_symbol(name);
        if let Some(&idx) = self.export_map.get(&hash) {
            if let Some(export) = self.exports.get_mut(idx) {
                export.value = value;
            }
        }
    }
}

/// Module resolver
pub struct ModuleResolver {
    /// Base path for resolution
    base_path: PathBuf,
    /// Search paths
    search_paths: Vec<PathBuf>,
    /// Extension to try
    extensions: Vec<String>,
}

impl ModuleResolver {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            search_paths: vec![],
            extensions: vec![
                String::from(".hate"),
                String::from(".ht"),
            ],
        }
    }
    
    /// Add a search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }
    
    /// Resolve a module specifier to a path
    pub fn resolve(&self, specifier: &str, from_module: Option<&Path>) -> HateResult<PathBuf> {
        // Relative import
        if specifier.starts_with("./") || specifier.starts_with("../") {
            let base = from_module
                .and_then(|p| p.parent())
                .unwrap_or(&self.base_path);
            return self.resolve_relative(specifier, base);
        }
        
        // Absolute import (with search paths)
        self.resolve_absolute(specifier)
    }
    
    fn resolve_relative(&self, specifier: &str, base: &Path) -> HateResult<PathBuf> {
        let path = base.join(specifier);
        
        // Try exact path
        if path.exists() {
            return Ok(path.canonicalize().unwrap_or(path));
        }
        
        // Try with extensions
        for ext in &self.extensions {
            let with_ext = path.with_extension(ext.trim_start_matches('.'));
            if with_ext.exists() {
                return Ok(with_ext.canonicalize().unwrap_or(with_ext));
            }
        }
        
        // Try as directory with index file
        let index = path.join("index.hate");
        if index.exists() {
            return Ok(index.canonicalize().unwrap_or(index));
        }
        
        Err(HateError::Internal(format!(
            "Cannot resolve module: {}",
            specifier
        )))
    }
    
    fn resolve_absolute(&self, specifier: &str) -> HateResult<PathBuf> {
        // Check search paths
        for search_path in &self.search_paths {
            let path = search_path.join(specifier);
            
            if path.exists() {
                return Ok(path.canonicalize().unwrap_or(path));
            }
            
            // Try with extensions
            for ext in &self.extensions {
                let with_ext = path.with_extension(ext.trim_start_matches('.'));
                if with_ext.exists() {
                    return Ok(with_ext.canonicalize().unwrap_or(with_ext));
                }
            }
            
            // Try as package with index
            let index = path.join("index.hate");
            if index.exists() {
                return Ok(index.canonicalize().unwrap_or(index));
            }
        }
        
        // Try base path
        let path = self.base_path.join(specifier);
        for ext in &self.extensions {
            let with_ext = path.with_extension(ext.trim_start_matches('.'));
            if with_ext.exists() {
                return Ok(with_ext.canonicalize().unwrap_or(with_ext));
            }
        }
        
        Err(HateError::Internal(format!(
            "Cannot resolve module: {}",
            specifier
        )))
    }
}

/// Module loader
pub struct ModuleLoader {
    /// All loaded modules
    modules: Vec<Module>,
    /// Path to module index
    path_to_index: HashMap<PathBuf, usize>,
    /// Module resolver
    resolver: ModuleResolver,
}

impl ModuleLoader {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            modules: Vec::new(),
            path_to_index: HashMap::new(),
            resolver: ModuleResolver::new(base_path),
        }
    }
    
    /// Add a search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.resolver.add_search_path(path);
    }
    
    /// Load a module
    pub fn load(&mut self, specifier: &str, from_module: Option<&Path>) -> HateResult<usize> {
        let path = self.resolver.resolve(specifier, from_module)?;
        
        // Check cache
        if let Some(&idx) = self.path_to_index.get(&path) {
            let module = &self.modules[idx];
            
            // Cycle detection
            if module.state == ModuleState::Loading {
                return Err(HateError::Internal(format!(
                    "Circular dependency detected: {}",
                    path.display()
                )));
            }
            
            return Ok(idx);
        }
        
        // Create new module
        let idx = self.modules.len();
        let mut module = Module::new(path.clone());
        
        // Read source
        module.source = std::fs::read_to_string(&path)
            .map_err(|e| HateError::Internal(format!("Failed to read module: {}", e)))?;
        
        module.state = ModuleState::Loading;
        
        self.path_to_index.insert(path, idx);
        self.modules.push(module);
        
        Ok(idx)
    }
    
    /// Mark a module as loaded
    pub fn mark_loaded(&mut self, idx: usize) {
        if let Some(module) = self.modules.get_mut(idx) {
            module.state = ModuleState::Loaded;
        }
    }
    
    /// Mark a module as failed
    pub fn mark_failed(&mut self, idx: usize) {
        if let Some(module) = self.modules.get_mut(idx) {
            module.state = ModuleState::Failed;
        }
    }
    
    /// Get a module
    pub fn get(&self, idx: usize) -> Option<&Module> {
        self.modules.get(idx)
    }
    
    /// Get a mutable module
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut Module> {
        self.modules.get_mut(idx)
    }
    
    /// Get module by path
    pub fn get_by_path(&self, path: &Path) -> Option<&Module> {
        self.path_to_index.get(path).and_then(|&idx| self.modules.get(idx))
    }
    
    /// Get module count
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }
}

/// Import bindings for the compiler
#[derive(Debug, Clone)]
pub struct ImportBinding {
    /// Module index
    pub module_idx: usize,
    /// Export name in the module
    pub export_name: Symbol,
    /// Local binding name
    pub local_name: Symbol,
    /// Is default import
    pub is_default: bool,
}

/// Import compiler
pub struct ImportCompiler {
    /// Pending imports for current module
    imports: Vec<ImportBinding>,
}

impl ImportCompiler {
    pub fn new() -> Self {
        Self {
            imports: Vec::new(),
        }
    }
    
    /// Add an import binding
    pub fn add_import(
        &mut self,
        module_idx: usize,
        export_name: Symbol,
        local_name: Symbol,
        is_default: bool,
    ) {
        self.imports.push(ImportBinding {
            module_idx,
            export_name,
            local_name,
            is_default,
        });
    }
    
    /// Get all imports
    pub fn imports(&self) -> &[ImportBinding] {
        &self.imports
    }
    
    /// Clear imports (for next module)
    pub fn clear(&mut self) {
        self.imports.clear();
    }
}

impl Default for ImportCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Export compiler
pub struct ExportCompiler {
    /// Pending exports for current module
    exports: Vec<Export>,
}

impl ExportCompiler {
    pub fn new() -> Self {
        Self {
            exports: Vec::new(),
        }
    }
    
    /// Add a named export
    pub fn add_named(&mut self, local: Symbol, export_as: Option<Symbol>) {
        self.exports.push(Export {
            local_name: local,
            export_name: export_as.unwrap_or(local),
            value: Value::null(),
            is_default: false,
        });
    }
    
    /// Add the default export
    pub fn add_default(&mut self, local: Symbol) {
        self.exports.push(Export {
            local_name: local,
            export_name: intern("default"),
            value: Value::null(),
            is_default: true,
        });
    }
    
    /// Get all exports
    pub fn exports(&self) -> &[Export] {
        &self.exports
    }
    
    /// Clear exports (for next module)
    pub fn clear(&mut self) {
        self.exports.clear();
    }
}

impl Default for ExportCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash a symbol for export lookup
fn hash_symbol(sym: Symbol) -> u32 {
    let mut hash: u32 = 2166136261;
    for b in sym.as_str().bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_module_exports() {
        let mut module = Module::new(PathBuf::from("test.hate"));
        
        module.add_export(intern("foo"), intern("foo"), false);
        module.add_export(intern("_default"), intern("default"), true);
        
        assert!(module.get_export(intern("foo")).is_some());
        assert!(module.get_default_export().is_some());
        assert!(module.get_default_export().unwrap().is_default);
    }
    
    #[test]
    fn test_import_compiler() {
        let mut compiler = ImportCompiler::new();
        
        compiler.add_import(0, intern("foo"), intern("foo"), false);
        compiler.add_import(0, intern("default"), intern("myDefault"), true);
        
        assert_eq!(compiler.imports().len(), 2);
        assert!(compiler.imports()[1].is_default);
    }
    
    #[test]
    fn test_export_compiler() {
        let mut compiler = ExportCompiler::new();
        
        compiler.add_named(intern("foo"), None);
        compiler.add_named(intern("bar"), Some(intern("baz")));
        compiler.add_default(intern("MyClass"));
        
        assert_eq!(compiler.exports().len(), 3);
        assert!(!compiler.exports()[0].is_default);
        assert_eq!(compiler.exports()[1].export_name.as_str(), "baz");
        assert!(compiler.exports()[2].is_default);
    }
}
