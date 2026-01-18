//! # Hate Programming Language
//!
//! A blazingly fast programming language with:
//! - NaN-boxed value representation (8 bytes per value)
//! - Register-based bytecode VM with direct-threaded dispatch
//! - Generational copying garbage collector
//! - Static types with full inference
//! - Modern syntax fixing JavaScript's mistakes

#![allow(dead_code)]
#![allow(unused_variables)]

pub mod value;
pub mod intern;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod bytecode;
pub mod compiler;
pub mod vm;
pub mod gc;
pub mod runtime;
pub mod error;
pub mod repl;
pub mod object;
pub mod vm_internals;
pub mod stdlib;
pub mod pattern;
pub mod class;
pub mod async_runtime;
pub mod module;
pub mod diagnostic;
pub mod formatter;
pub mod linter;
pub mod lsp;
pub mod debugger;
pub mod package;
pub mod optimizer;

// Re-exports for convenience
pub use value::Value;
pub use lexer::Lexer;
pub use parser::Parser;
pub use compiler::Compiler;
pub use vm::VM;
pub use gc::GC;
pub use error::{HateError, HateResult};

/// Version of the Hate language
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run a Hate program from source code
pub fn run(source: &str) -> HateResult<Value> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    
    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;
    
    let mut compiler = Compiler::new();
    let chunk = compiler.compile(&ast)?;
    
    let mut vm = VM::new();
    vm.run(&chunk)
}

/// Run a Hate program from a file
pub fn run_file(path: &str) -> HateResult<Value> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| HateError::IO(e.to_string()))?;
    run(&source)
}
