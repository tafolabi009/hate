# Hate

<div align="center">

**A blazingly fast programming language**

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

</div>

## Overview

Hate is a modern, high-performance programming language designed to be:

- **Blazingly Fast** - Register-based VM with NaN-boxing for 8-byte values
- **Memory Efficient** - Generational garbage collector with minimal pauses
- **Developer Friendly** - Clean syntax inspired by the best of Rust, TypeScript, and Python
- **Production Ready** - Full toolchain with REPL, compiler, and bytecode serialization

## Features

### Language Features
- First-class functions and closures
- Pattern matching with guards
- Async/await syntax
- Optional type annotations
- Null-safety with `??` and `?.` operators
- String interpolation
- Range expressions (`0..10`, `0..=10`)
- Destructuring

### Performance
- NaN-boxed values (all values in 8 bytes)
- Register-based bytecode VM
- String interning for O(1) equality
- Generational copying garbage collector
- Inline caching for property access (planned)
- JIT compilation (planned)

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/tafolabi009/hate.git
cd hate

# Build release version
cargo build --release

# Install (optional)
cargo install --path .
```

## Quick Start

### Hello World

```hate
// hello.hate
let name = "World";
println("Hello, " + name + "!");
```

Run it:
```bash
hate hello.hate
```

### Variables and Types

```hate
// Immutable by default
let x = 42;
let name = "Alice";
let pi = 3.14159;
let active = true;

// Mutable variables
let mut counter = 0;
counter = counter + 1;

// Type annotations (optional)
let age: i32 = 25;
let balance: f64 = 100.50;
```

### Functions

```hate
// Basic function
fn greet(name) {
    println("Hello, " + name + "!");
}

// With type annotations
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// Lambda expressions
let double = (x) => x * 2;
let numbers = [1, 2, 3].map((x) => x * 2);
```

### Control Flow

```hate
// If expressions
let result = if x > 0 { "positive" } else { "negative" };

// While loops
let mut i = 0;
while i < 10 {
    println(i);
    i = i + 1;
}

// For loops
for i in 0..10 {
    println(i);
}

// Pattern matching
match value {
    0 => println("zero"),
    1 | 2 => println("one or two"),
    n if n > 10 => println("big number"),
    _ => println("something else"),
}
```

### Classes

```hate
class Point {
    fn new(x, y) {
        this.x = x;
        this.y = y;
    }
    
    fn distance(other) {
        let dx = this.x - other.x;
        let dy = this.y - other.y;
        return sqrt(dx * dx + dy * dy);
    }
}

let p1 = new Point(0, 0);
let p2 = new Point(3, 4);
println(p1.distance(p2));  // 5
```

## CLI Usage

```bash
# Start REPL
hate repl

# Run a file
hate run script.hate

# Check for errors
hate check script.hate

# Show AST
hate ast script.hate

# Show bytecode
hate dis script.hate

# Compile to bytecode
hate compile script.hate -o script.hatec

# Run benchmarks
hate bench
```

## Architecture

```
Source Code (.hate)
       │
       ▼
   ┌───────┐
   │ Lexer │  String Interning
   └───────┘
       │ Tokens
       ▼
   ┌────────┐
   │ Parser │  Pratt Parsing
   └────────┘
       │ AST
       ▼
   ┌──────────┐
   │ Compiler │  Bytecode Generation
   └──────────┘
       │ Bytecode
       ▼
   ┌────┐
   │ VM │  Register-based Execution
   └────┘
       │
       ▼ 
   ┌────┐
   │ GC │  Generational Collection
   └────┘
```

### Bytecode Instructions

The VM uses a register-based architecture with 4-byte instructions:

| Field  | Size | Description |
|--------|------|-------------|
| opcode | 1B   | Operation code |
| a      | 1B   | Destination/first operand |
| b      | 1B   | Second operand |
| c      | 1B   | Third operand |

### Value Representation (NaN-Boxing)

All values are stored in 8 bytes using NaN-boxing:

```
Float:    Normal IEEE 754 double
Integer:  0x7FF80001_XXXXXXXX (32-bit signed)
Pointer:  0x7FFC_XXXXXXXXXXXX (48-bit pointer)
True:     0x7FF80002_00000001
False:    0x7FF80002_00000000
Null:     0x7FF80003_00000000
```

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Project Structure

```
src/
├── lib.rs        # Library entry point
├── main.rs       # CLI entry point
├── lexer.rs      # Tokenization
├── parser.rs     # Pratt parser
├── ast.rs        # Abstract syntax tree
├── compiler.rs   # Bytecode generation
├── bytecode.rs   # Instruction set
├── vm.rs         # Virtual machine
├── gc.rs         # Garbage collector
├── value.rs      # NaN-boxing
├── intern.rs     # String interning
├── runtime.rs    # Standard library
├── repl.rs       # Interactive REPL
└── error.rs      # Error handling
```

## Benchmarks

Coming soon! The goal is to outperform Python, Ruby, and cold-start Node.js.

## Roadmap

- [x] Lexer with string interning
- [x] Pratt parser
- [x] Bytecode compiler
- [x] Register-based VM
- [x] Generational GC
- [x] REPL with syntax highlighting
- [ ] Complete standard library
- [ ] Module system
- [ ] Inline caching
- [ ] JIT compilation
- [ ] Language Server (LSP)
- [ ] Package manager

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
