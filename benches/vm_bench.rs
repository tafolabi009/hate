//! VM Benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hate::lexer::Lexer;
use hate::parser::Parser;
use hate::compiler::Compiler;
use hate::vm::VM;

fn bench_arithmetic(c: &mut Criterion) {
    let source = r#"
        let x = 1 + 2 * 3 - 4 / 2;
        x;
    "#;
    
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();
    let mut compiler = Compiler::new();
    let chunk = compiler.compile(&ast).unwrap();
    
    c.bench_function("arithmetic", |b| {
        b.iter(|| {
            let mut vm = VM::new();
            black_box(vm.run(&chunk).unwrap())
        })
    });
}

fn bench_loop(c: &mut Criterion) {
    let source = r#"
        let mut sum = 0;
        let mut i = 0;
        while i < 1000 {
            sum = sum + i;
            i = i + 1;
        }
        sum;
    "#;
    
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();
    let mut compiler = Compiler::new();
    let chunk = compiler.compile(&ast).unwrap();
    
    c.bench_function("loop_1000", |b| {
        b.iter(|| {
            let mut vm = VM::new();
            black_box(vm.run(&chunk).unwrap())
        })
    });
}

fn bench_conditionals(c: &mut Criterion) {
    let source = r#"
        let mut result = 0;
        let mut i = 0;
        while i < 100 {
            if i % 2 == 0 {
                result = result + 1;
            } else {
                result = result - 1;
            }
            i = i + 1;
        }
        result;
    "#;
    
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();
    let mut compiler = Compiler::new();
    let chunk = compiler.compile(&ast).unwrap();
    
    c.bench_function("conditionals", |b| {
        b.iter(|| {
            let mut vm = VM::new();
            black_box(vm.run(&chunk).unwrap())
        })
    });
}

fn bench_fibonacci(c: &mut Criterion) {
    let source = r#"
        fn fib(n) {
            if n <= 1 {
                return n;
            }
            return fib(n - 1) + fib(n - 2);
        }
        fib(20);
    "#;
    
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();
    let mut compiler = Compiler::new();
    let chunk = compiler.compile(&ast).unwrap();
    
    c.bench_function("fibonacci_20", |b| {
        b.iter(|| {
            let mut vm = VM::new();
            black_box(vm.run(&chunk).unwrap())
        })
    });
}

fn bench_nested_function_calls(c: &mut Criterion) {
    let source = r#"
        fn inc(x) { return x + 1; }
        fn double(x) { return x * 2; }
        fn square(x) { return x * x; }
        fn compute(x) { return square(double(inc(x))); }
        
        let mut sum = 0;
        let mut i = 0;
        while i < 100 {
            sum = sum + compute(i);
            i = i + 1;
        }
        sum;
    "#;
    
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();
    let mut compiler = Compiler::new();
    let chunk = compiler.compile(&ast).unwrap();
    
    c.bench_function("nested_function_calls", |b| {
        b.iter(|| {
            let mut vm = VM::new();
            black_box(vm.run(&chunk).unwrap())
        })
    });
}

criterion_group!(benches, bench_arithmetic, bench_loop, bench_conditionals, bench_fibonacci, bench_nested_function_calls);
criterion_main!(benches);

