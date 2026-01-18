//! Parser Benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hate::lexer::Lexer;
use hate::parser::Parser;

fn bench_parse_expression(c: &mut Criterion) {
    let source = "1 + 2 * 3 - 4 / 5 + (6 * 7) - 8;";
    
    c.bench_function("parse_expression", |b| {
        b.iter(|| {
            let mut lexer = Lexer::new(source);
            let tokens = lexer.tokenize().unwrap();
            let mut parser = Parser::new(tokens);
            black_box(parser.parse().unwrap())
        })
    });
}

fn bench_parse_function(c: &mut Criterion) {
    let source = r#"
        fn add(a, b) {
            return a + b;
        }
        
        fn multiply(x, y) {
            let result = x * y;
            return result;
        }
    "#;
    
    c.bench_function("parse_function", |b| {
        b.iter(|| {
            let mut lexer = Lexer::new(source);
            let tokens = lexer.tokenize().unwrap();
            let mut parser = Parser::new(tokens);
            black_box(parser.parse().unwrap())
        })
    });
}

fn bench_parse_class(c: &mut Criterion) {
    let source = r#"
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
    "#;
    
    c.bench_function("parse_class", |b| {
        b.iter(|| {
            let mut lexer = Lexer::new(source);
            let tokens = lexer.tokenize().unwrap();
            let mut parser = Parser::new(tokens);
            black_box(parser.parse().unwrap())
        })
    });
}

fn bench_lex_only(c: &mut Criterion) {
    let source = r#"
        let x = 123.456;
        let str = "hello world";
        fn test(a, b, c) {
            if a > b and b > c {
                return true;
            }
            return false;
        }
    "#;
    
    c.bench_function("lex_only", |b| {
        b.iter(|| {
            let mut lexer = Lexer::new(source);
            black_box(lexer.tokenize().unwrap())
        })
    });
}

criterion_group!(benches, bench_parse_expression, bench_parse_function, bench_parse_class, bench_lex_only);
criterion_main!(benches);
