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
├── lib.rs          # Library entry point
├── main.rs         # CLI entry point
├── lexer.rs        # Tokenization
├── parser.rs       # Pratt parser
├── ast.rs          # Abstract syntax tree
├── compiler.rs     # Bytecode generation
├── bytecode.rs     # Instruction set (60+ opcodes)
├── vm.rs           # Virtual machine
├── vm_internals.rs # VM internals (inline caching, heap)
├── gc.rs           # Generational garbage collector
├── value.rs        # NaN-boxing value representation
├── intern.rs       # String interning
├── object.rs       # Object system with hidden classes
├── runtime.rs      # Runtime support
├── stdlib.rs       # Standard library functions
├── pattern.rs      # Pattern matching compiler
├── class.rs        # Class system with inheritance
├── async_runtime.rs# Async/await with promises
├── module.rs       # Module system (import/export)
├── diagnostic.rs   # Enhanced error messages
├── formatter.rs    # Code formatter
├── linter.rs       # Static analysis (17 rules)
├── lsp.rs          # Language Server Protocol
├── debugger.rs     # Debug Adapter Protocol
├── optimizer.rs    # Bytecode optimization passes
├── package.rs      # Package manager with semver
├── repl.rs         # Interactive REPL
└── error.rs        # Error handling
```

## Standard Library

### Math Functions
```hate
// Basic math
abs(-5)           // 5
sqrt(16)          // 4.0
pow(2, 10)        // 1024
floor(3.7)        // 3
ceil(3.2)         // 4
round(3.5)        // 4

// Trigonometry
sin(0)            // 0.0
cos(0)            // 1.0
tan(0)            // 0.0

// Constants
PI                // 3.14159...
E                 // 2.71828...

// Random
random()          // 0.0 to 1.0
random_int(1, 6)  // 1 to 6
```

### String Functions
```hate
let s = "Hello, World!";
s.length()        // 13
s.upper()         // "HELLO, WORLD!"
s.lower()         // "hello, world!"
s.trim()          // removes whitespace
s.split(",")      // ["Hello", " World!"]
s.contains("World") // true
s.replace("World", "Hate") // "Hello, Hate!"
s.starts_with("Hello") // true
s.ends_with("!")  // true
s.slice(0, 5)     // "Hello"
```

### Array Functions
```hate
let arr = [1, 2, 3, 4, 5];

// Transformation
arr.map((x) => x * 2)     // [2, 4, 6, 8, 10]
arr.filter((x) => x > 2)  // [3, 4, 5]
arr.reduce((a, b) => a + b, 0) // 15

// Access
arr.length()      // 5
arr.first()       // 1
arr.last()        // 5
arr.get(2)        // 3
arr.slice(1, 3)   // [2, 3]

// Mutation
arr.push(6)       // [1, 2, 3, 4, 5, 6]
arr.pop()         // 6, arr = [1, 2, 3, 4, 5]
arr.reverse()     // [5, 4, 3, 2, 1]
arr.sort()        // [1, 2, 3, 4, 5]

// Search
arr.find((x) => x > 3)    // 4
arr.index_of(3)   // 2
arr.contains(3)   // true
arr.every((x) => x > 0)   // true
arr.some((x) => x > 4)    // true
```

### Object Functions
```hate
let obj = { name: "Alice", age: 30 };

Object.keys(obj)    // ["name", "age"]
Object.values(obj)  // ["Alice", 30]
Object.entries(obj) // [["name", "Alice"], ["age", 30]]
Object.assign({}, obj, { city: "NYC" })
```

## Module System

```hate
// math_utils.hate
export fn square(x) {
    return x * x;
}

export let PI = 3.14159;

// main.hate
import { square, PI } from "./math_utils";
import * as utils from "./math_utils";

println(square(5));      // 25
println(utils.PI);       // 3.14159
```

## Async/Await

```hate
async fn fetch_data(url) {
    let response = await http_get(url);
    return response.json();
}

async fn main() {
    let data = await fetch_data("https://api.example.com/data");
    println(data);
}
```

## Pattern Matching

```hate
// Simple patterns
match value {
    0 => println("zero"),
    1 | 2 | 3 => println("small"),
    n if n > 100 => println("large: " + n),
    _ => println("other"),
}

// Destructuring patterns
match point {
    Point { x: 0, y: 0 } => println("origin"),
    Point { x, y } if x == y => println("diagonal"),
    Point { x, y } => println("at " + x + ", " + y),
}

// Array patterns
match list {
    [] => println("empty"),
    [x] => println("single: " + x),
    [first, ...rest] => println("head: " + first),
}
```

## Tooling

### Formatter
```bash
hate fmt script.hate          # Format file
hate fmt --check script.hate  # Check formatting
hate fmt .                    # Format all files
```

Configuration in `hate.toml`:
```toml
[format]
indent_width = 2
use_tabs = false
line_width = 100
trailing_commas = true
semicolons = true
```

### Linter
```bash
hate lint script.hate         # Lint file
hate lint --fix script.hate   # Auto-fix issues
```

Lint rules include:
- `unused-variable` - Warn on unused variables
- `unused-parameter` - Warn on unused function parameters
- `shadowed-variable` - Warn when variable shadows outer scope
- `undefined-variable` - Error on undefined variables
- `unreachable-code` - Warn on unreachable code
- `empty-block` - Warn on empty blocks
- `constant-condition` - Warn on always true/false conditions
- `max-complexity` - Warn on high cyclomatic complexity
- `prefer-const` - Suggest removing `mut` if never reassigned
- `no-double-equals` - Prefer `===` over `==`

### Package Manager
```bash
hate init                     # Initialize new package
hate add lodash               # Add dependency
hate remove lodash            # Remove dependency
hate install                  # Install all dependencies
hate update                   # Update dependencies
hate publish                  # Publish to registry
```

Package manifest `hate.toml`:
```toml
[package]
name = "my-app"
version = "1.0.0"
description = "My awesome app"
author = "Alice <alice@example.com>"

[dependencies]
lodash = "^4.0.0"
express = "~2.1.0"

[dev-dependencies]
testing = "1.0.0"
```

## Performance Optimizations

### NaN-Boxing
All values fit in 8 bytes using IEEE 754 NaN-boxing, eliminating heap allocations for primitives.

### Hidden Classes
Objects use V8-style hidden classes for fast property access with predictable memory layout.

### Inline Caching
Property accesses and method calls use inline caches that remember the hidden class, enabling O(1) lookups.

### Generational GC
Young generation uses copying collection, old generation uses mark-sweep with incremental marking.

### Bytecode Optimization
- Constant folding - `2 + 3` becomes `5`
- Dead code elimination - Unreachable code removed
- Peephole optimization - Redundant instruction elimination

## Benchmarks

Coming soon! The goal is to outperform Python, Ruby, and cold-start Node.js.

## Roadmap

- [x] Lexer with string interning
- [x] Pratt parser
- [x] Bytecode compiler
- [x] Register-based VM (60+ opcodes)
- [x] Generational GC
- [x] REPL with syntax highlighting
- [x] Complete standard library
- [x] Pattern matching compiler
- [x] Class system with inheritance
- [x] Async/await with promises
- [x] Module system (import/export)
- [x] Enhanced error messages
- [x] Code formatter
- [x] Linter (17 rules)
- [x] Language Server (LSP)
- [x] Debug Adapter (DAP)
- [x] Package manager with semver
- [x] Bytecode optimizer
- [ ] Inline caching (in progress)
- [ ] JIT compilation (planned)

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
