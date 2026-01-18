//! Hate WASM Module
//!
//! This crate provides WebAssembly bindings for the Hate programming language,
//! enabling Hate code to run in the browser with minimal JavaScript overhead.
//!
//! Architecture:
//! - 90%+ execution happens in WASM (Hate VM, GC, bytecode interpreter)
//! - Minimal JS bridge (~100 lines) for DOM/Web API access
//! - Native performance with full browser API access

use wasm_bindgen::prelude::*;
use js_sys::Promise;
use web_sys::{console, Window, Document};
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

mod bridge;
mod dom;
mod runtime;

pub use bridge::*;
pub use dom::*;
pub use runtime::*;

/// Initialize panic hook for better error messages in browser console
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    console::log_1(&"Hate WASM runtime initialized".into());
}

/// The main Hate VM exposed to JavaScript
#[wasm_bindgen]
pub struct HateVM {
    /// Source code cache
    sources: HashMap<String, String>,
    /// Global bindings accessible from Hate code
    globals: Rc<RefCell<GlobalBindings>>,
    /// Event handlers registered from Hate
    event_handlers: Rc<RefCell<HashMap<u32, JsValue>>>,
    /// Next handler ID
    next_handler_id: u32,
}

/// Global bindings for DOM and Web APIs
pub struct GlobalBindings {
    /// Window object
    pub window: Window,
    /// Document object
    pub document: Document,
    /// Custom globals
    pub custom: HashMap<String, JsValue>,
}

#[wasm_bindgen]
impl HateVM {
    /// Create a new Hate VM instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<HateVM, JsValue> {
        let window = web_sys::window()
            .ok_or_else(|| JsValue::from_str("No window object"))?;
        let document = window.document()
            .ok_or_else(|| JsValue::from_str("No document object"))?;
        
        let globals = Rc::new(RefCell::new(GlobalBindings {
            window,
            document,
            custom: HashMap::new(),
        }));
        
        Ok(HateVM {
            sources: HashMap::new(),
            globals,
            event_handlers: Rc::new(RefCell::new(HashMap::new())),
            next_handler_id: 0,
        })
    }
    
    /// Run Hate source code and return the result as a string
    #[wasm_bindgen]
    pub fn run(&mut self, code: &str) -> Result<JsValue, JsValue> {
        // Parse and compile the code
        let result = self.execute_internal(code)?;
        Ok(result)
    }
    
    /// Run Hate code asynchronously (for async/await support)
    #[wasm_bindgen]
    pub fn run_async(&mut self, code: &str) -> Promise {
        let code = code.to_string();
        let globals = self.globals.clone();
        
        wasm_bindgen_futures::future_to_promise(async move {
            // Execute with async runtime
            let result = execute_async(&code, globals).await?;
            Ok(result)
        })
    }
    
    /// Evaluate an expression and return the result
    #[wasm_bindgen]
    pub fn eval(&mut self, expr: &str) -> Result<JsValue, JsValue> {
        self.execute_internal(expr)
    }
    
    /// Load a Hate module/file
    #[wasm_bindgen]
    pub fn load_module(&mut self, name: &str, code: &str) {
        self.sources.insert(name.to_string(), code.to_string());
    }
    
    /// Set a global variable accessible from Hate code
    #[wasm_bindgen]
    pub fn set_global(&mut self, name: &str, value: JsValue) {
        self.globals.borrow_mut().custom.insert(name.to_string(), value);
    }
    
    /// Get a global variable
    #[wasm_bindgen]
    pub fn get_global(&self, name: &str) -> JsValue {
        self.globals.borrow()
            .custom
            .get(name)
            .cloned()
            .unwrap_or(JsValue::UNDEFINED)
    }
    
    /// Register an event handler (returns handler ID)
    #[wasm_bindgen]
    pub fn register_handler(&mut self, handler: JsValue) -> u32 {
        let id = self.next_handler_id;
        self.next_handler_id += 1;
        self.event_handlers.borrow_mut().insert(id, handler);
        id
    }
    
    /// Unregister an event handler
    #[wasm_bindgen]
    pub fn unregister_handler(&mut self, id: u32) {
        self.event_handlers.borrow_mut().remove(&id);
    }
    
    /// Get VM version
    #[wasm_bindgen]
    pub fn version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
    
    // Internal execution
    fn execute_internal(&mut self, code: &str) -> Result<JsValue, JsValue> {
        use hate::{lexer::Lexer, parser::Parser, compiler::Compiler, vm::VM};
        
        // Lexing
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize()
            .map_err(|e| JsValue::from_str(&format!("Lexer error: {}", e)))?;
        
        // Parsing
        let mut parser = Parser::new(tokens);
        let ast = parser.parse()
            .map_err(|e| JsValue::from_str(&format!("Parser error: {}", e)))?;
        
        // Compiling
        let mut compiler = Compiler::new();
        let chunk = compiler.compile(&ast)
            .map_err(|e| JsValue::from_str(&format!("Compiler error: {}", e)))?;
        
        // Executing
        let mut vm = VM::new();
        
        // Register DOM bridge functions
        self.register_builtins(&mut vm);
        
        let result = vm.run(&chunk)
            .map_err(|e| JsValue::from_str(&format!("Runtime error: {}", e)))?;
        
        // Convert result to JsValue
        Ok(hate_value_to_js(result))
    }
    
    fn register_builtins(&self, _vm: &mut hate::vm::VM) {
        // Register built-in functions that bridge to JS/DOM
        // This is where we connect Hate's runtime to browser APIs
    }
}

impl Default for HateVM {
    fn default() -> Self {
        Self::new().expect("Failed to create HateVM")
    }
}

/// Convert Hate Value to JsValue
fn hate_value_to_js(value: hate::value::Value) -> JsValue {
    if value.is_null() {
        JsValue::NULL
    } else if let Some(b) = value.as_bool() {
        JsValue::from_bool(b)
    } else if let Some(i) = value.as_int() {
        JsValue::from_f64(i as f64)
    } else if let Some(f) = value.as_float() {
        JsValue::from_f64(f)
    } else {
        // For complex types, convert to string representation
        JsValue::from_str(&format!("{:?}", value))
    }
}

/// Execute code asynchronously
async fn execute_async(
    code: &str, 
    _globals: Rc<RefCell<GlobalBindings>>
) -> Result<JsValue, JsValue> {
    use hate::{lexer::Lexer, parser::Parser, compiler::Compiler, vm::VM};
    
    // Lexing
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize()
        .map_err(|e| JsValue::from_str(&format!("Lexer error: {}", e)))?;
    
    // Parsing
    let mut parser = Parser::new(tokens);
    let ast = parser.parse()
        .map_err(|e| JsValue::from_str(&format!("Parser error: {}", e)))?;
    
    // Compiling
    let mut compiler = Compiler::new();
    let chunk = compiler.compile(&ast)
        .map_err(|e| JsValue::from_str(&format!("Compiler error: {}", e)))?;
    
    // Executing
    let mut vm = VM::new();
    let result = vm.run(&chunk)
        .map_err(|e| JsValue::from_str(&format!("Runtime error: {}", e)))?;
    
    Ok(hate_value_to_js(result))
}

// Re-export for tests
#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    wasm_bindgen_test_configure!(run_in_browser);
    
    #[wasm_bindgen_test]
    fn test_vm_creation() {
        let vm = HateVM::new();
        assert!(vm.is_ok());
    }
    
    #[wasm_bindgen_test]
    fn test_simple_eval() {
        let mut vm = HateVM::new().unwrap();
        let result = vm.eval("1 + 2");
        assert!(result.is_ok());
    }
}
