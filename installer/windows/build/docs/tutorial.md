# Hate Programming Language Tutorial

Welcome to Hate! This tutorial will guide you through the language from basics to advanced features.

## Table of Contents

1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Basic Syntax](#basic-syntax)
4. [Variables and Types](#variables-and-types)
5. [Operators](#operators)
6. [Control Flow](#control-flow)
7. [Functions](#functions)
8. [Classes and Objects](#classes-and-objects)
9. [Pattern Matching](#pattern-matching)
10. [Error Handling](#error-handling)
11. [Modules](#modules)
12. [Standard Library](#standard-library)
13. [Best Practices](#best-practices)

---

## Introduction

### What is Hate?

Hate is a modern, dynamically-typed programming language designed for:

- **Speed**: Register-based bytecode VM with NaN-boxing
- **Simplicity**: Clean, intuitive syntax
- **Safety**: Null-safe operators and pattern matching
- **Productivity**: Interactive REPL and helpful error messages

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Source Code (.hate)                       │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  LEXER: Converts source text into tokens                    │
│  - String interning for O(1) string equality                │
│  - Line/column tracking for error messages                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  PARSER: Builds Abstract Syntax Tree (AST)                  │
│  - Pratt parsing for expressions (precedence climbing)      │
│  - Recursive descent for statements                         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  COMPILER: Generates bytecode from AST                      │
│  - Register allocation                                      │
│  - Constant folding                                         │
│  - Closure capture                                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  VIRTUAL MACHINE: Executes bytecode                         │
│  - Register-based (256 registers per frame)                 │
│  - NaN-boxed values (8 bytes per value)                     │
│  - Generational garbage collector                           │
└─────────────────────────────────────────────────────────────┘
```

### Why "Interpreter"?

Hate is a **bytecode interpreter**, meaning:

1. Your source code is first compiled to **bytecode** (a compact, efficient intermediate representation)
2. The bytecode is then **interpreted** by a virtual machine (VM)
3. This is different from:
   - **Pure interpreters** (like early BASIC) that execute source directly
   - **Ahead-of-time compilers** (like C/Rust) that produce native machine code
   - **JIT compilers** (like V8/LuaJIT) that compile bytecode to native code at runtime

This approach gives us:
- Fast startup (no slow compilation step)
- Portability (bytecode runs on any platform with the VM)
- Good performance (much faster than source interpretation)

---

## Getting Started

### Installation

#### Linux/macOS (Quick Install)

```bash
curl -fsSL https://raw.githubusercontent.com/tafolabi009/hate/main/install.sh | bash
```

#### Windows

Download and run the installer from the [releases page](https://github.com/tafolabi009/hate/releases).

#### From Source

```bash
git clone https://github.com/tafolabi009/hate.git
cd hate
cargo build --release
sudo cp target/release/hate /usr/local/bin/
```

### Your First Program

Create a file called `hello.hate`:

```hate
// hello.hate
println("Hello, World!");
```

Run it:

```bash
hate hello.hate
```

### Using the REPL

Start the interactive REPL:

```bash
hate repl
```

You'll see:

```
  _    _       _       
 | |  | |     | |      
 | |__| | __ _| |_ ___ 
 |  __  |/ _` | __/ _ \
 | |  | | (_| | ||  __/
 |_|  |_|\__,_|\__\___|

Hate v0.1.0 - A blazingly fast programming language
Type .help for help, .exit to quit

hate> 
```

Try some expressions:

```hate
hate> 1 + 2 * 3
=> 7
hate> "Hello" + ", " + "World"
=> "Hello, World"
hate> let x = 42
hate> x * 2
=> 84
```

---

## Basic Syntax

### Comments

```hate
// Single-line comment

/*
   Multi-line
   comment
*/

/// Documentation comment (for functions and classes)
```

### Statements

Statements end with a semicolon or newline:

```hate
let x = 10;
let y = 20

// Multiple statements on one line
let a = 1; let b = 2; let c = 3;
```

### Expressions

Almost everything is an expression in Hate:

```hate
// If is an expression
let status = if active { "on" } else { "off" };

// Blocks are expressions (last expression is the value)
let result = {
    let a = 10;
    let b = 20;
    a + b  // This is returned
};
println(result);  // 30
```

---

## Variables and Types

### Variable Declaration

```hate
// Immutable (default)
let name = "Alice";
let age = 30;
let pi = 3.14159;

// Mutable
let mut counter = 0;
counter = counter + 1;  // OK

// Attempting to reassign immutable variable
let x = 10;
x = 20;  // Error: Cannot assign to immutable variable
```

### Type Annotations (Optional)

```hate
// Hate is dynamically typed, but you can add type hints
let name: str = "Bob";
let age: i32 = 25;
let balance: f64 = 100.50;
let active: bool = true;

// The type checker will warn about mismatches
let count: i32 = "hello";  // Warning: type mismatch
```

### Data Types

#### Integers

```hate
let decimal = 42;
let negative = -17;
let big = 1_000_000;  // Underscores for readability
```

#### Floats

```hate
let pi = 3.14159;
let scientific = 1.5e10;
let negative = -0.001;
```

#### Strings

```hate
let greeting = "Hello, World!";
let multiline = "Line 1
Line 2
Line 3";

// String operations
let name = "Alice";
let message = "Hello, " + name + "!";
```

#### Booleans

```hate
let yes = true;
let no = false;

// Boolean operations
let result = yes and no;   // false
let other = yes or no;     // true
let negated = not yes;     // false
```

#### Null

```hate
let nothing = null;

// Null checks
if value == null {
    println("No value");
}

// Null-safe operators
let name = user?.name ?? "Anonymous";
```

#### Arrays

```hate
let numbers = [1, 2, 3, 4, 5];
let mixed = [1, "two", 3.0, true];
let empty = [];

// Access elements
let first = numbers[0];      // 1
let last = numbers[4];       // 5

// Modify (if mutable)
let mut arr = [1, 2, 3];
arr[0] = 10;
push(arr, 4);
```

#### Objects

```hate
let person = {
    name: "Alice",
    age: 30,
    active: true
};

// Access properties
let name = person.name;
let age = person["age"];

// Modify
person.age = 31;
person["status"] = "online";
```

---

## Operators

### Arithmetic

| Operator | Description | Example |
|----------|-------------|---------|
| `+` | Addition | `5 + 3` → `8` |
| `-` | Subtraction | `5 - 3` → `2` |
| `*` | Multiplication | `5 * 3` → `15` |
| `/` | Division | `5 / 3` → `1.666...` |
| `%` | Modulo | `5 % 3` → `2` |

### Comparison

| Operator | Description | Example |
|----------|-------------|---------|
| `==` | Equal | `5 == 5` → `true` |
| `!=` | Not equal | `5 != 3` → `true` |
| `<` | Less than | `3 < 5` → `true` |
| `<=` | Less or equal | `5 <= 5` → `true` |
| `>` | Greater than | `5 > 3` → `true` |
| `>=` | Greater or equal | `5 >= 5` → `true` |

### Logical

| Operator | Description | Example |
|----------|-------------|---------|
| `and` | Logical AND | `true and false` → `false` |
| `or` | Logical OR | `true or false` → `true` |
| `not` | Logical NOT | `not true` → `false` |

### Bitwise

| Operator | Description | Example |
|----------|-------------|---------|
| `&` | Bitwise AND | `5 & 3` → `1` |
| `\|` | Bitwise OR | `5 \| 3` → `7` |
| `^` | Bitwise XOR | `5 ^ 3` → `6` |
| `~` | Bitwise NOT | `~5` → `-6` |
| `<<` | Left shift | `5 << 1` → `10` |
| `>>` | Right shift | `5 >> 1` → `2` |

### Null-Safe Operators

```hate
// Null coalescing: returns right if left is null
let name = user.name ?? "Anonymous";

// Optional chaining: returns null if object is null
let city = user?.address?.city;

// Combined
let city = user?.address?.city ?? "Unknown";
```

### Range Operators

```hate
// Exclusive range (0 to 9)
for i in 0..10 {
    println(i);
}

// Inclusive range (0 to 10)
for i in 0..=10 {
    println(i);
}
```

---

## Control Flow

### If/Else

```hate
// Statement form
if condition {
    // do something
} else if other_condition {
    // do something else
} else {
    // fallback
}

// Expression form (requires else)
let result = if x > 0 { "positive" } else { "non-positive" };

// Chained
let grade = if score >= 90 {
    "A"
} else if score >= 80 {
    "B"
} else if score >= 70 {
    "C"
} else {
    "F"
};
```

### While Loop

```hate
let mut i = 0;
while i < 10 {
    println(i);
    i = i + 1;
}

// Break and continue
let mut j = 0;
while true {
    j = j + 1;
    if j % 2 == 0 {
        continue;  // Skip even numbers
    }
    if j > 10 {
        break;     // Exit loop
    }
    println(j);
}
```

### For Loop

```hate
// Range-based
for i in 0..10 {
    println(i);
}

// Array iteration
let fruits = ["apple", "banana", "cherry"];
for fruit in fruits {
    println(fruit);
}

// With index (destructuring)
for (i, fruit) in enumerate(fruits) {
    println(i + ": " + fruit);
}
```

---

## Functions

### Basic Functions

```hate
fn greet() {
    println("Hello!");
}

fn add(a, b) {
    return a + b;
}

// Call functions
greet();
let sum = add(3, 4);
```

### Return Values

```hate
// Explicit return
fn square(x) {
    return x * x;
}

// Implicit return (last expression)
fn cube(x) {
    x * x * x
}

// Early return
fn absolute(x) {
    if x < 0 {
        return -x;
    }
    x
}
```

### Type Annotations

```hate
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

fn greet(name: str) -> str {
    return "Hello, " + name + "!";
}
```

### Default Parameters

```hate
fn greet(name, greeting = "Hello") {
    println(greeting + ", " + name + "!");
}

greet("Alice");              // Hello, Alice!
greet("Bob", "Hi");          // Hi, Bob!
```

### Variadic Functions

```hate
fn sum(...numbers) {
    let mut total = 0;
    for n in numbers {
        total = total + n;
    }
    return total;
}

println(sum(1, 2, 3, 4, 5));  // 15
```

### Lambda Expressions

```hate
// Arrow syntax
let double = (x) => x * 2;
let add = (a, b) => a + b;

// Block body
let factorial = (n) => {
    if n <= 1 { return 1; }
    return n * factorial(n - 1);
};

// Use with higher-order functions
let numbers = [1, 2, 3, 4, 5];
let doubled = numbers.map((x) => x * 2);
let evens = numbers.filter((x) => x % 2 == 0);
```

### Closures

```hate
fn make_counter() {
    let mut count = 0;
    return () => {
        count = count + 1;
        return count;
    };
}

let counter = make_counter();
println(counter());  // 1
println(counter());  // 2
println(counter());  // 3
```

---

## Classes and Objects

### Class Definition

```hate
class Person {
    // Constructor
    fn new(name, age) {
        this.name = name;
        this.age = age;
    }
    
    // Methods
    fn greet() {
        println("Hi, I'm " + this.name);
    }
    
    fn birthday() {
        this.age = this.age + 1;
        println("Happy birthday! Now " + this.age);
    }
}

// Create instance
let alice = new Person("Alice", 30);
alice.greet();      // Hi, I'm Alice
alice.birthday();   // Happy birthday! Now 31
```

### Inheritance

```hate
class Animal {
    fn new(name) {
        this.name = name;
    }
    
    fn speak() {
        println(this.name + " makes a sound");
    }
}

class Dog extends Animal {
    fn new(name, breed) {
        super.new(name);
        this.breed = breed;
    }
    
    fn speak() {
        println(this.name + " barks!");
    }
    
    fn fetch() {
        println(this.name + " fetches the ball");
    }
}

let dog = new Dog("Rex", "German Shepherd");
dog.speak();   // Rex barks!
dog.fetch();   // Rex fetches the ball
```

### Static Methods

```hate
class Math {
    static fn square(x) {
        return x * x;
    }
    
    static fn max(a, b) {
        if a > b { return a; }
        return b;
    }
}

println(Math.square(5));    // 25
println(Math.max(10, 20));  // 20
```

### Getters and Setters

```hate
class Circle {
    fn new(radius) {
        this._radius = radius;
    }
    
    fn radius() {
        return this._radius;
    }
    
    fn set_radius(value) {
        if value < 0 {
            panic("Radius cannot be negative");
        }
        this._radius = value;
    }
    
    fn area() {
        return 3.14159 * this._radius * this._radius;
    }
}
```

---

## Pattern Matching

### Basic Match

```hate
let value = 2;

match value {
    0 => println("zero"),
    1 => println("one"),
    2 => println("two"),
    _ => println("something else"),
}
```

### Multiple Patterns

```hate
match day {
    "Saturday" | "Sunday" => println("Weekend!"),
    _ => println("Weekday"),
}
```

### Guards

```hate
match number {
    n if n < 0 => println("negative"),
    n if n == 0 => println("zero"),
    n if n > 0 => println("positive"),
}
```

### Destructuring

```hate
// Arrays
let point = [10, 20];
match point {
    [0, 0] => println("origin"),
    [x, 0] => println("on x-axis at " + x),
    [0, y] => println("on y-axis at " + y),
    [x, y] => println("at " + x + ", " + y),
}

// Objects
let person = { name: "Alice", age: 30 };
match person {
    { name: "Alice", age } => println("Alice is " + age),
    { name, age: 0 } => println(name + " is a baby"),
    { name, age } => println(name + " is " + age),
}
```

### Match Expressions

```hate
let description = match status {
    200 => "OK",
    404 => "Not Found",
    500 => "Server Error",
    code if code >= 400 => "Error: " + code,
    _ => "Unknown",
};
```

---

## Error Handling

### Try/Catch

```hate
try {
    let result = risky_operation();
    println(result);
} catch (e) {
    println("Error: " + e);
} finally {
    cleanup();
}
```

### Throwing Errors

```hate
fn divide(a, b) {
    if b == 0 {
        throw "Division by zero";
    }
    return a / b;
}

try {
    let result = divide(10, 0);
} catch (e) {
    println("Caught: " + e);
}
```

### Assertions

```hate
fn process(value) {
    assert(value != null, "Value cannot be null");
    assert(value > 0, "Value must be positive");
    // ...
}
```

---

## Modules

### Importing

```hate
// Import entire module
import math;
println(math.sqrt(16));

// Import specific items
import { sqrt, pow } from math;
println(sqrt(16));

// Import with alias
import math as m;
println(m.sqrt(16));

// Import from file
import { helper } from "./utils.hate";
```

### Exporting

```hate
// utils.hate

// Export function
export fn helper(x) {
    return x * 2;
}

// Export class
export class Tool {
    fn new(name) {
        this.name = name;
    }
}

// Export variable
export let VERSION = "1.0.0";
```

---

## Standard Library

### I/O Functions

```hate
print("Hello");          // Print without newline
println("Hello");         // Print with newline
let name = input("Name: ");  // Read input
```

### Math Functions

```hate
abs(-5)          // 5
floor(3.7)       // 3
ceil(3.2)        // 4
round(3.5)       // 4
sqrt(16)         // 4
pow(2, 8)        // 256
sin(0)           // 0
cos(0)           // 1
log(10)          // 2.302...
min(3, 7, 2)     // 2
max(3, 7, 2)     // 7
random()         // 0.0 to 1.0
randomInt(1, 10) // 1 to 9
```

### String Functions

```hate
len("hello")              // 5
upper("hello")            // "HELLO"
lower("HELLO")            // "hello"
trim("  hi  ")            // "hi"
split("a,b,c", ",")       // ["a", "b", "c"]
join(["a", "b"], "-")     // "a-b"
startsWith("hello", "he") // true
endsWith("hello", "lo")   // true
replace("hello", "l", "L") // "heLLo"
```

### Array Functions

```hate
let arr = [1, 2, 3];

len(arr)           // 3
push(arr, 4)       // [1, 2, 3, 4]
pop(arr)           // 4 (arr is now [1, 2, 3])
shift(arr)         // 1 (arr is now [2, 3])
unshift(arr, 0)    // [0, 2, 3]
slice(arr, 1, 3)   // [2, 3]
reverse(arr)       // [3, 2, 0]
sort(arr)          // [0, 2, 3]
```

### Type Functions

```hate
type(42)          // "int"
type(3.14)        // "float"
type("hi")        // "string"
type(true)        // "bool"
type(null)        // "null"
type([1, 2])      // "array"
type({a: 1})      // "object"

isInt(42)         // true
isFloat(3.14)     // true
isStr("hi")       // true
isBool(true)      // true
isNull(null)      // true
isArray([])       // true
isObject({})      // true
```

### Time Functions

```hate
time()            // Unix timestamp (seconds)
timeMs()          // Unix timestamp (milliseconds)
sleep(1000)       // Sleep for 1000ms
```

---

## Best Practices

### 1. Use Immutable by Default

```hate
// Prefer immutable
let x = 10;

// Only use mut when necessary
let mut counter = 0;
```

### 2. Prefer Pattern Matching Over If Chains

```hate
// Instead of:
if x == 1 {
    // ...
} else if x == 2 {
    // ...
} else if x == 3 {
    // ...
}

// Use:
match x {
    1 => // ...,
    2 => // ...,
    3 => // ...,
    _ => // ...,
}
```

### 3. Use Null-Safe Operators

```hate
// Instead of:
let name;
if user != null and user.profile != null {
    name = user.profile.name;
} else {
    name = "Anonymous";
}

// Use:
let name = user?.profile?.name ?? "Anonymous";
```

### 4. Small Functions

```hate
// Break complex logic into small functions
fn validateEmail(email) {
    return email.contains("@");
}

fn validatePassword(password) {
    return len(password) >= 8;
}

fn validateUser(user) {
    return validateEmail(user.email) and 
           validatePassword(user.password);
}
```

### 5. Meaningful Names

```hate
// Bad
let x = 86400;
let l = users.filter((u) => u.a);

// Good
let SECONDS_PER_DAY = 86400;
let activeUsers = users.filter((user) => user.isActive);
```

---

## Next Steps

- Read the [API Reference](./api-reference.md)
- Explore [Example Projects](../examples/)
- Join the [Community](https://github.com/tafolabi009/hate/discussions)
- Contribute on [GitHub](https://github.com/tafolabi009/hate)

Happy coding with Hate! 🚀
