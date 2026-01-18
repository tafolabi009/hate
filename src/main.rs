//! Hate - A blazingly fast programming language
//!
//! CLI entry point

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use hate::repl::run_repl;
use hate::lexer::Lexer;
use hate::parser::Parser as HateParser;
use hate::compiler::Compiler;
use hate::bytecode::Chunk;

#[derive(Parser)]
#[command(name = "hate")]
#[command(author = "The Hate Authors")]
#[command(version)]
#[command(about = "A blazingly fast programming language", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Script file to run
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the interactive REPL
    Repl,
    
    /// Run a script file
    Run {
        /// The script file to run
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    
    /// Compile a script to bytecode
    Compile {
        /// The script file to compile
        #[arg(value_name = "FILE")]
        file: PathBuf,
        
        /// Output file (default: same name with .hatec extension)
        #[arg(short, long, value_name = "OUTPUT")]
        output: Option<PathBuf>,
    },
    
    /// Check a script for errors without running
    Check {
        /// The script file to check
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    
    /// Format a script file
    Fmt {
        /// The script file to format
        #[arg(value_name = "FILE")]
        file: PathBuf,
        
        /// Write formatted output back to file
        #[arg(short, long)]
        write: bool,
    },
    
    /// Show the AST of a script
    Ast {
        /// The script file to parse
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    
    /// Disassemble bytecode
    Dis {
        /// The script file or bytecode file
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    
    /// Run benchmarks
    Bench {
        /// Optional benchmark filter
        #[arg(value_name = "FILTER")]
        filter: Option<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    
    let result = match &cli.command {
        Some(Commands::Repl) => {
            run_repl().map(|_| ())
        }
        
        Some(Commands::Run { file }) => {
            run_script(file)
        }
        
        Some(Commands::Compile { file, output }) => {
            compile_script(file, output.as_ref())
        }
        
        Some(Commands::Check { file }) => {
            check_script(file)
        }
        
        Some(Commands::Fmt { file, write }) => {
            format_script(file, *write)
        }
        
        Some(Commands::Ast { file }) => {
            show_ast(file)
        }
        
        Some(Commands::Dis { file }) => {
            disassemble(file)
        }
        
        Some(Commands::Bench { filter }) => {
            run_benchmarks(filter.as_deref())
        }
        
        None => {
            // No command specified
            if let Some(file) = &cli.file {
                // Run the file
                run_script(file)
            } else {
                // Start REPL
                run_repl().map(|_| ())
            }
        }
    };
    
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("\x1b[31merror:\x1b[0m {}", e);
            ExitCode::FAILURE
        }
    }
}

fn run_script(path: &PathBuf) -> Result<(), hate::error::HateError> {
    let source = std::fs::read_to_string(path).map_err(|e| {
        hate::error::HateError::Internal(format!("Failed to read file: {}", e))
    })?;
    
    hate::run(&source)?;
    Ok(())
}

fn compile_script(input: &PathBuf, output: Option<&PathBuf>) -> Result<(), hate::error::HateError> {
    let source = std::fs::read_to_string(input).map_err(|e| {
        hate::error::HateError::Internal(format!("Failed to read file: {}", e))
    })?;
    
    // Lex
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize()?;
    
    // Parse
    let mut parser = HateParser::new(tokens);
    let ast = parser.parse()?;
    
    // Compile
    let mut compiler = Compiler::new();
    let chunk = compiler.compile(&ast)?;
    
    // Serialize
    let output_path = output.cloned().unwrap_or_else(|| {
        let mut p = input.clone();
        p.set_extension("hatec");
        p
    });
    
    let bytes = serialize_chunk(&chunk)?;
    std::fs::write(&output_path, bytes).map_err(|e| {
        hate::error::HateError::Internal(format!("Failed to write output: {}", e))
    })?;
    
    println!("\x1b[32mCompiled to {:?}\x1b[0m", output_path);
    Ok(())
}

fn serialize_chunk(chunk: &Chunk) -> Result<Vec<u8>, hate::error::HateError> {
    // Simple serialization format:
    // - Magic: "HATE" (4 bytes)
    // - Version: u32 (4 bytes)
    // - Code length: u32 (4 bytes)
    // - Code: [Instruction] (code_len * 4 bytes)
    // - Constants length: u32 (4 bytes)
    // - Constants: [Value] (const_len * 8 bytes)
    
    let mut bytes = Vec::new();
    
    // Magic
    bytes.extend_from_slice(b"HATE");
    
    // Version
    bytes.extend_from_slice(&1u32.to_le_bytes());
    
    // Code
    bytes.extend_from_slice(&(chunk.code.len() as u32).to_le_bytes());
    for instr in &chunk.code {
        bytes.push(instr.opcode as u8);
        bytes.push(instr.a);
        bytes.push(instr.b);
        bytes.push(instr.c);
    }
    
    // Constants
    bytes.extend_from_slice(&(chunk.constants.len() as u32).to_le_bytes());
    for constant in &chunk.constants {
        bytes.extend_from_slice(&constant.0.to_le_bytes());
    }
    
    Ok(bytes)
}

fn check_script(path: &PathBuf) -> Result<(), hate::error::HateError> {
    let source = std::fs::read_to_string(path).map_err(|e| {
        hate::error::HateError::Internal(format!("Failed to read file: {}", e))
    })?;
    
    // Lex
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize()?;
    
    // Parse
    let mut parser = HateParser::new(tokens);
    let _ast = parser.parse()?;
    
    // TODO: Type checking
    
    println!("\x1b[32m✓ No errors found in {:?}\x1b[0m", path);
    Ok(())
}

fn format_script(path: &PathBuf, write: bool) -> Result<(), hate::error::HateError> {
    let source = std::fs::read_to_string(path).map_err(|e| {
        hate::error::HateError::Internal(format!("Failed to read file: {}", e))
    })?;
    
    // Lex
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize()?;
    
    // Parse
    let mut parser = HateParser::new(tokens);
    let ast = parser.parse()?;
    
    // TODO: Pretty print the AST
    let formatted = format!("{:#?}", ast);
    
    if write {
        std::fs::write(path, &formatted).map_err(|e| {
            hate::error::HateError::Internal(format!("Failed to write file: {}", e))
        })?;
        println!("\x1b[32mFormatted {:?}\x1b[0m", path);
    } else {
        println!("{}", formatted);
    }
    
    Ok(())
}

fn show_ast(path: &PathBuf) -> Result<(), hate::error::HateError> {
    let source = std::fs::read_to_string(path).map_err(|e| {
        hate::error::HateError::Internal(format!("Failed to read file: {}", e))
    })?;
    
    // Lex
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize()?;
    
    // Parse
    let mut parser = HateParser::new(tokens);
    let ast = parser.parse()?;
    
    println!("{:#?}", ast);
    Ok(())
}

fn disassemble(path: &PathBuf) -> Result<(), hate::error::HateError> {
    let extension = path.extension().and_then(|e| e.to_str());
    
    if extension == Some("hatec") {
        // Load compiled bytecode
        let bytes = std::fs::read(path).map_err(|e| {
            hate::error::HateError::Internal(format!("Failed to read file: {}", e))
        })?;
        
        let chunk = deserialize_chunk(&bytes)?;
        chunk.disassemble(&path.to_string_lossy());
    } else {
        // Compile from source
        let source = std::fs::read_to_string(path).map_err(|e| {
            hate::error::HateError::Internal(format!("Failed to read file: {}", e))
        })?;
        
        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize()?;
        
        let mut parser = HateParser::new(tokens);
        let ast = parser.parse()?;
        
        let mut compiler = Compiler::new();
        let chunk = compiler.compile(&ast)?;
        
        chunk.disassemble(&path.to_string_lossy());
    }
    
    Ok(())
}

fn deserialize_chunk(bytes: &[u8]) -> Result<Chunk, hate::error::HateError> {
    if bytes.len() < 12 || &bytes[0..4] != b"HATE" {
        return Err(hate::error::HateError::Internal("Invalid bytecode file".to_string()));
    }
    
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if version != 1 {
        return Err(hate::error::HateError::Internal(format!(
            "Unsupported bytecode version: {}", version
        )));
    }
    
    let code_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    
    let mut chunk = Chunk::new();
    let mut offset = 12;
    
    // Read code
    for _ in 0..code_len {
        let opcode = unsafe { std::mem::transmute(bytes[offset]) };
        let instr = hate::bytecode::Instruction {
            opcode,
            a: bytes[offset + 1],
            b: bytes[offset + 2],
            c: bytes[offset + 3],
        };
        chunk.code.push(instr);
        chunk.lines.push(0);
        offset += 4;
    }
    
    // Read constants
    let const_len = u32::from_le_bytes([
        bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]
    ]) as usize;
    offset += 4;
    
    for _ in 0..const_len {
        let value_bits = u64::from_le_bytes([
            bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3],
            bytes[offset + 4], bytes[offset + 5], bytes[offset + 6], bytes[offset + 7],
        ]);
        chunk.constants.push(hate::value::Value(value_bits));
        offset += 8;
    }
    
    Ok(chunk)
}

fn run_benchmarks(filter: Option<&str>) -> Result<(), hate::error::HateError> {
    println!("\x1b[1mRunning benchmarks...\x1b[0m\n");
    
    let benchmarks = [
        ("fib(30)", r#"
fn fib(n) {
    if n < 2 { return n; }
    return fib(n - 1) + fib(n - 2);
}
fib(30);
"#),
        ("loop 1M", r#"
let sum = 0;
for i in 0..1000000 {
    sum = sum + i;
}
sum;
"#),
        ("array push 100K", r#"
let arr = [];
for i in 0..100000 {
    push(arr, i);
}
len(arr);
"#),
    ];
    
    for (name, code) in benchmarks {
        if let Some(f) = filter {
            if !name.contains(f) {
                continue;
            }
        }
        
        print!("{:20} ", name);
        std::io::Write::flush(&mut std::io::stdout()).ok();
        
        let start = std::time::Instant::now();
        
        match hate::run(code) {
            Ok(_) => {
                let elapsed = start.elapsed();
                println!("\x1b[32m{:>10.3?}\x1b[0m", elapsed);
            }
            Err(e) => {
                println!("\x1b[31mERROR: {}\x1b[0m", e);
            }
        }
    }
    
    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_serialize_deserialize() {
        let mut chunk = Chunk::new();
        chunk.write(hate::bytecode::Instruction::with_ab(
            hate::bytecode::OpCode::LoadInt, 0, 42
        ), 1);
        chunk.write(hate::bytecode::Instruction::with_a(
            hate::bytecode::OpCode::Return, 0
        ), 1);
        
        let bytes = serialize_chunk(&chunk).unwrap();
        let loaded = deserialize_chunk(&bytes).unwrap();
        
        assert_eq!(chunk.code.len(), loaded.code.len());
    }
}
