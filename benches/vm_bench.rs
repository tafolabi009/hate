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
        let sum = 0;
        let i = 0;
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
        let result = 0;
        let i = 0;
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

criterion_group!(benches, bench_arithmetic, bench_loop, bench_conditionals);
criterion_main!(benches);
