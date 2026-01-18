# Hate Quick Reference

## CLI Commands

```bash
hate [file.hate]        # Run a script
hate repl               # Start interactive REPL
hate compile file.hate  # Compile to bytecode (.hatec)
hate check file.hate    # Type check without running
hate fmt file.hate      # Format source code
hate ast file.hate      # Show AST
hate dis file.hate      # Show bytecode disassembly
hate bench              # Run built-in benchmarks
```

## Syntax Cheat Sheet

### Variables
```hate
let x = 10;              // Immutable
let mut y = 20;          // Mutable
let z: i32 = 30;         // With type hint
```

### Operators
```
+  -  *  /  %            // Arithmetic
== != < <= > >=          // Comparison
and  or  not             // Logical
& | ^ ~ << >>            // Bitwise
??                       // Null coalescing
?.                       // Optional chaining
..  ..=                  // Range (exclusive/inclusive)
```

### Control Flow
```hate
// If expression
let r = if x > 0 { "pos" } else { "neg" };

// While loop
while condition { }

// For loop
for i in 0..10 { }
for item in array { }

// Match
match value {
    0 => "zero",
    n if n > 0 => "positive",
    _ => "negative",
}
```

### Functions
```hate
fn add(a, b) {
    return a + b;
}

fn greet(name = "World") {
    println("Hello, " + name);
}

let double = (x) => x * 2;   // Lambda
```

### Classes
```hate
class Dog extends Animal {
    fn new(name) {
        super.new(name);
        this.name = name;
    }
    
    fn bark() {
        println(this.name + " barks!");
    }
}

let dog = new Dog("Rex");
```

### Error Handling
```hate
try {
    riskyOperation();
} catch (e) {
    println("Error: " + e);
} finally {
    cleanup();
}
```

### Modules
```hate
import math;
import { sqrt } from math;
export fn helper() { }
```

## Built-in Functions

| Category | Functions |
|----------|-----------|
| **I/O** | `print`, `println`, `input` |
| **Math** | `abs`, `floor`, `ceil`, `round`, `sqrt`, `pow`, `sin`, `cos`, `log`, `min`, `max`, `random` |
| **String** | `len`, `upper`, `lower`, `trim`, `split`, `join`, `replace`, `startsWith`, `endsWith` |
| **Array** | `len`, `push`, `pop`, `shift`, `unshift`, `slice`, `reverse`, `sort`, `map`, `filter` |
| **Type** | `type`, `isInt`, `isFloat`, `isStr`, `isBool`, `isNull`, `isArray`, `isObject` |
| **Time** | `time`, `timeMs`, `sleep` |

## REPL Commands

```
.help     Show help
.clear    Clear screen
.exit     Exit REPL
.ast      Show AST for last expression
.dis      Show bytecode for last expression
```
