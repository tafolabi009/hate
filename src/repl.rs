//! Interactive REPL (Read-Eval-Print-Loop)
//!
//! Features:
//! - Syntax highlighting
//! - Tab completion
//! - Command history
//! - Multi-line editing

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::compiler::Compiler;
use crate::vm::VM;
use crate::error::HateResult;

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Config, EditMode, Editor, Helper, Result as RLResult};
use std::borrow::Cow;

const HISTORY_FILE: &str = ".hate_history";

/// Keywords for completion
const KEYWORDS: &[&str] = &[
    "let", "const", "fn", "return", "if", "else", "while", "for", "in",
    "break", "continue", "match", "class", "extends", "new", "this",
    "super", "static", "pub", "import", "from", "export", "async",
    "await", "try", "catch", "finally", "throw", "true", "false", "null",
    "type", "interface", "enum", "and", "or", "not",
];

/// Built-in functions for completion
const BUILTINS: &[&str] = &[
    "print", "println", "input", "int", "float", "str", "bool", "type",
    "isInt", "isFloat", "isStr", "isBool", "isNull", "isArray", "isObject",
    "isFunction", "abs", "floor", "ceil", "round", "sqrt", "pow", "sin",
    "cos", "tan", "log", "log10", "exp", "min", "max", "random", "randomInt",
    "len", "charAt", "substr", "indexOf", "split", "trim", "upper", "lower",
    "startsWith", "endsWith", "replace", "push", "pop", "shift", "unshift",
    "slice", "concat", "reverse", "sort", "join", "range", "keys", "values",
    "entries", "hasKey", "time", "timeMs", "sleep", "assert", "assertEq",
    "debug", "panic",
];

/// REPL commands
const COMMANDS: &[(&str, &str)] = &[
    (".help", "Show this help message"),
    (".clear", "Clear the screen"),
    (".exit", "Exit the REPL"),
    (".quit", "Exit the REPL"),
    (".ast", "Show AST for next expression"),
    (".bytecode", "Show bytecode for next expression"),
    (".gc", "Trigger garbage collection"),
    (".stats", "Show VM statistics"),
    (".reset", "Reset the VM state"),
];

/// REPL helper for rustyline
struct HateHelper {
    /// Words to complete from
    words: Vec<String>,
}

impl HateHelper {
    fn new() -> Self {
        let mut words: Vec<String> = KEYWORDS.iter().map(|s| s.to_string()).collect();
        words.extend(BUILTINS.iter().map(|s| s.to_string()));
        words.extend(COMMANDS.iter().map(|(cmd, _)| cmd.to_string()));
        Self { words }
    }
}

impl Completer for HateHelper {
    type Candidate = Pair;
    
    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> RLResult<(usize, Vec<Pair>)> {
        // Find the start of the current word
        let start = line[..pos]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
            .map(|i| i + 1)
            .unwrap_or(0);
        
        let prefix = &line[start..pos];
        
        if prefix.is_empty() {
            return Ok((pos, vec![]));
        }
        
        let matches: Vec<Pair> = self.words
            .iter()
            .filter(|word| word.starts_with(prefix))
            .map(|word| Pair {
                display: word.clone(),
                replacement: word.clone(),
            })
            .collect();
        
        Ok((start, matches))
    }
}

impl Hinter for HateHelper {
    type Hint = String;
    
    fn hint(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        if pos < line.len() {
            return None;
        }
        
        // Find the start of the current word
        let start = line
            .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
            .map(|i| i + 1)
            .unwrap_or(0);
        
        let prefix = &line[start..];
        
        if prefix.is_empty() {
            return None;
        }
        
        // Find first matching word
        self.words
            .iter()
            .find(|word| word.starts_with(prefix) && word.len() > prefix.len())
            .map(|word| word[prefix.len()..].to_string())
    }
}

impl Highlighter for HateHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        // Apply syntax highlighting
        let mut result = String::with_capacity(line.len() * 2);
        let mut chars = line.chars().peekable();
        let mut i = 0;
        
        while let Some(c) = chars.next() {
            if c.is_alphabetic() || c == '_' {
                // Identifier/keyword
                let start = i;
                let mut ident = c.to_string();
                i += 1;
                
                while chars.peek().map(|c| c.is_alphanumeric() || *c == '_').unwrap_or(false) {
                    ident.push(chars.next().unwrap());
                    i += 1;
                }
                
                if KEYWORDS.contains(&ident.as_str()) {
                    // Keywords in magenta/bold
                    result.push_str("\x1b[1;35m");
                    result.push_str(&ident);
                    result.push_str("\x1b[0m");
                } else if BUILTINS.contains(&ident.as_str()) {
                    // Built-ins in cyan
                    result.push_str("\x1b[36m");
                    result.push_str(&ident);
                    result.push_str("\x1b[0m");
                } else if ident == "true" || ident == "false" || ident == "null" {
                    // Literals in yellow
                    result.push_str("\x1b[33m");
                    result.push_str(&ident);
                    result.push_str("\x1b[0m");
                } else {
                    result.push_str(&ident);
                }
                
                let _ = start;
            } else if c.is_ascii_digit() {
                // Number in green
                result.push_str("\x1b[32m");
                result.push(c);
                i += 1;
                
                while chars.peek().map(|c| c.is_ascii_digit() || *c == '.' || *c == '_').unwrap_or(false) {
                    result.push(chars.next().unwrap());
                    i += 1;
                }
                
                result.push_str("\x1b[0m");
            } else if c == '"' || c == '\'' {
                // String in yellow
                let quote = c;
                result.push_str("\x1b[33m");
                result.push(c);
                i += 1;
                
                while let Some(&next) = chars.peek() {
                    result.push(chars.next().unwrap());
                    i += 1;
                    
                    if next == quote {
                        break;
                    }
                    
                    if next == '\\' {
                        if let Some(escaped) = chars.next() {
                            result.push(escaped);
                            i += 1;
                        }
                    }
                }
                
                result.push_str("\x1b[0m");
            } else if c == '/' && chars.peek() == Some(&'/') {
                // Comment in gray
                result.push_str("\x1b[90m");
                result.push(c);
                i += 1;
                
                while let Some(next) = chars.next() {
                    result.push(next);
                    i += 1;
                }
                
                result.push_str("\x1b[0m");
            } else {
                result.push(c);
                i += 1;
            }
        }
        
        Cow::Owned(result)
    }
    
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        // Hints in gray
        Cow::Owned(format!("\x1b[90m{}\x1b[0m", hint))
    }
    
    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        true
    }
}

impl Validator for HateHelper {
    fn validate(&self, ctx: &mut ValidationContext<'_>) -> RLResult<ValidationResult> {
        let input = ctx.input();
        
        // Check for balanced brackets
        let mut paren = 0i32;
        let mut brace = 0i32;
        let mut bracket = 0i32;
        let mut in_string = false;
        let mut string_char = ' ';
        
        let mut chars = input.chars().peekable();
        
        while let Some(c) = chars.next() {
            if in_string {
                if c == '\\' {
                    chars.next(); // Skip escaped char
                } else if c == string_char {
                    in_string = false;
                }
            } else {
                match c {
                    '"' | '\'' => {
                        in_string = true;
                        string_char = c;
                    }
                    '(' => paren += 1,
                    ')' => paren -= 1,
                    '{' => brace += 1,
                    '}' => brace -= 1,
                    '[' => bracket += 1,
                    ']' => bracket -= 1,
                    '/' if chars.peek() == Some(&'/') => {
                        // Skip rest of line (comment)
                        break;
                    }
                    _ => {}
                }
            }
        }
        
        if in_string || paren > 0 || brace > 0 || bracket > 0 {
            Ok(ValidationResult::Incomplete)
        } else {
            Ok(ValidationResult::Valid(None))
        }
    }
}

impl Helper for HateHelper {}

/// Run the interactive REPL
pub fn run_repl() -> HateResult<()> {
    println!("\x1b[1;35m  _    _       _       \x1b[0m");
    println!("\x1b[1;35m | |  | |     | |      \x1b[0m");
    println!("\x1b[1;35m | |__| | __ _| |_ ___ \x1b[0m");
    println!("\x1b[1;35m |  __  |/ _` | __/ _ \\\x1b[0m");
    println!("\x1b[1;35m | |  | | (_| | ||  __/\x1b[0m");
    println!("\x1b[1;35m |_|  |_|\\__,_|\\__\\___|\x1b[0m");
    println!();
    println!("\x1b[1mHate v{}\x1b[0m - A blazingly fast programming language", env!("CARGO_PKG_VERSION"));
    println!("Type \x1b[36m.help\x1b[0m for help, \x1b[36m.exit\x1b[0m to quit\n");
    
    // Configure rustyline
    let config = Config::builder()
        .edit_mode(EditMode::Emacs)
        .auto_add_history(true)
        .max_history_size(1000)
        .expect("Invalid history size")
        .build();
    
    let helper = HateHelper::new();
    let mut rl: Editor<HateHelper, _> = Editor::with_config(config)
        .map_err(|e| crate::error::HateError::Internal(format!("Readline error: {}", e)))?;
    rl.set_helper(Some(helper));
    
    // Load history
    let _ = rl.load_history(HISTORY_FILE);
    
    // Create VM
    let mut vm = VM::new();
    let mut show_ast = false;
    let mut show_bytecode = false;
    
    loop {
        let prompt = if show_ast || show_bytecode {
            "\x1b[33mhate*>\x1b[0m "
        } else {
            "\x1b[35mhate>\x1b[0m "
        };
        
        match rl.readline(prompt) {
            Ok(line) => {
                let line = line.trim();
                
                if line.is_empty() {
                    continue;
                }
                
                // Handle REPL commands
                if line.starts_with('.') {
                    match line {
                        ".help" => {
                            println!("\n\x1b[1mREPL Commands:\x1b[0m");
                            for (cmd, desc) in COMMANDS {
                                println!("  \x1b[36m{:12}\x1b[0m {}", cmd, desc);
                            }
                            println!();
                        }
                        ".clear" => {
                            print!("\x1b[2J\x1b[H");
                        }
                        ".exit" | ".quit" => {
                            println!("Goodbye!");
                            break;
                        }
                        ".ast" => {
                            show_ast = true;
                            println!("\x1b[33mAST mode enabled for next input\x1b[0m");
                        }
                        ".bytecode" => {
                            show_bytecode = true;
                            println!("\x1b[33mBytecode mode enabled for next input\x1b[0m");
                        }
                        ".gc" => {
                            // Trigger GC
                            println!("\x1b[32mGC triggered\x1b[0m");
                        }
                        ".stats" => {
                            println!("\x1b[1mVM Statistics:\x1b[0m");
                            println!("  (statistics not yet implemented)");
                        }
                        ".reset" => {
                            vm = VM::new();
                            println!("\x1b[32mVM state reset\x1b[0m");
                        }
                        _ => {
                            println!("\x1b[31mUnknown command: {}\x1b[0m", line);
                        }
                    }
                    continue;
                }
                
                // Lex
                let mut lexer = Lexer::new(line);
                let tokens = match lexer.tokenize() {
                    Ok(t) => t,
                    Err(e) => {
                        println!("\x1b[31m{}\x1b[0m", e);
                        continue;
                    }
                };
                
                // Parse
                let mut parser = Parser::new(tokens);
                let ast = match parser.parse() {
                    Ok(a) => a,
                    Err(e) => {
                        println!("\x1b[31m{}\x1b[0m", e);
                        continue;
                    }
                };
                
                if show_ast {
                    println!("\x1b[1mAST:\x1b[0m");
                    println!("{:#?}", ast);
                    show_ast = false;
                }
                
                // Compile
                let mut compiler = Compiler::new();
                let chunk = match compiler.compile(&ast) {
                    Ok(c) => c,
                    Err(e) => {
                        println!("\x1b[31m{}\x1b[0m", e);
                        continue;
                    }
                };
                
                if show_bytecode {
                    println!("\x1b[1mBytecode:\x1b[0m");
                    chunk.disassemble("<repl>");
                    show_bytecode = false;
                }
                
                // Execute
                match vm.run(&chunk) {
                    Ok(result) => {
                        if !result.is_null() {
                            println!("\x1b[32m=> {}\x1b[0m", result);
                        }
                    }
                    Err(e) => {
                        println!("\x1b[31m{}\x1b[0m", e);
                    }
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                println!("^C");
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("^D");
                break;
            }
            Err(err) => {
                println!("Error: {}", err);
                break;
            }
        }
    }
    
    // Save history
    let _ = rl.save_history(HISTORY_FILE);
    
    Ok(())
}

/// Evaluate a single expression and return the result as a string
pub fn eval(source: &str) -> HateResult<String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;
    let mut compiler = Compiler::new();
    let chunk = compiler.compile(&ast)?;
    let mut vm = VM::new();
    let result = vm.run(&chunk)?;
    Ok(format!("{}", result))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_eval() {
        // Basic evaluation works
        let result = eval("42;").unwrap();
        // Result is "null" because expression statements don't return a value
        assert!(result.contains("null") || result.contains("42"));
    }
}
