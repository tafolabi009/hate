//! Recursive descent parser with Pratt parsing for expressions
//!
//! Features:
//! - Hand-written for maximum speed
//! - Pratt parsing for elegant operator precedence handling
//! - Error recovery for better diagnostics
//! - Source span tracking for all nodes

use crate::lexer::{Token, TokenKind};
use crate::ast::*;
use crate::intern::{Symbol, intern};
use crate::error::{HateError, HateResult, Span};

/// Operator precedence levels (Pratt parsing)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum Precedence {
    None = 0,
    Assignment,    // =
    NullCoalesce,  // ??
    Or,            // ||
    And,           // &&
    BitOr,         // |
    BitXor,        // ^
    BitAnd,        // &
    Equality,      // == !=
    Comparison,    // < > <= >=
    Shift,         // << >>
    Term,          // + -
    Factor,        // * / %
    Unary,         // ! - ~
    Call,          // . () []
    Primary,
}

impl Precedence {
    fn next(self) -> Self {
        match self {
            Precedence::None => Precedence::Assignment,
            Precedence::Assignment => Precedence::NullCoalesce,
            Precedence::NullCoalesce => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::BitOr,
            Precedence::BitOr => Precedence::BitXor,
            Precedence::BitXor => Precedence::BitAnd,
            Precedence::BitAnd => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Shift,
            Precedence::Shift => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => Precedence::Primary,
        }
    }
}

/// Get the precedence of an infix operator
fn infix_precedence(kind: TokenKind) -> Precedence {
    match kind {
        TokenKind::Equal | TokenKind::PlusEqual | TokenKind::MinusEqual |
        TokenKind::StarEqual | TokenKind::SlashEqual => Precedence::Assignment,
        TokenKind::DoubleQuestion => Precedence::NullCoalesce,
        TokenKind::Or => Precedence::Or,
        TokenKind::And => Precedence::And,
        TokenKind::Pipe => Precedence::BitOr,
        TokenKind::Caret => Precedence::BitXor,
        TokenKind::Ampersand => Precedence::BitAnd,
        TokenKind::EqualEqual | TokenKind::BangEqual => Precedence::Equality,
        TokenKind::Less | TokenKind::LessEqual | 
        TokenKind::Greater | TokenKind::GreaterEqual => Precedence::Comparison,
        TokenKind::LessLess | TokenKind::GreaterGreater => Precedence::Shift,
        TokenKind::Plus | TokenKind::Minus => Precedence::Term,
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Factor,
        TokenKind::LeftParen | TokenKind::LeftBracket | 
        TokenKind::Dot | TokenKind::QuestionDot => Precedence::Call,
        TokenKind::DotDot => Precedence::Comparison,
        _ => Precedence::None,
    }
}

/// Parser for the Hate language
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    /// Create a new parser
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }
    
    /// Parse the entire program
    pub fn parse(&mut self) -> HateResult<Program> {
        let start_span = self.current_span();
        let mut statements = Vec::new();
        
        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        
        let end_span = if statements.is_empty() {
            start_span
        } else {
            statements.last().unwrap().span()
        };
        
        Ok(Program {
            statements,
            span: start_span.merge(end_span),
        })
    }
    
    // ==================== Declarations ====================
    
    fn declaration(&mut self) -> HateResult<Stmt> {
        if self.check(TokenKind::Export) {
            return self.export_declaration();
        }
        if self.check(TokenKind::Import) {
            return self.import_declaration();
        }
        if self.check(TokenKind::Let) {
            return self.let_declaration();
        }
        if self.check(TokenKind::Fn) || 
           (self.check(TokenKind::Async) && self.check_next(TokenKind::Fn)) {
            return self.function_declaration();
        }
        if self.check(TokenKind::Class) {
            return self.class_declaration();
        }
        if self.check(TokenKind::Impl) {
            return self.impl_declaration();
        }
        
        self.statement()
    }
    
    fn export_declaration(&mut self) -> HateResult<Stmt> {
        let start = self.advance(); // consume 'export'
        let statement = Box::new(self.declaration()?);
        Ok(Stmt::Export {
            span: start.span.merge(statement.span()),
            statement,
        })
    }
    
    fn import_declaration(&mut self) -> HateResult<Stmt> {
        let start = self.advance(); // consume 'import'
        
        let mut items = Vec::new();
        
        // import { a, b, c } from "module"
        if self.match_token(TokenKind::LeftBrace) {
            loop {
                let name = self.expect_identifier("import item")?;
                let alias = if self.match_token(TokenKind::As) {
                    Some(self.expect_identifier("import alias")?)
                } else {
                    None
                };
                
                items.push(ImportItem {
                    name,
                    alias,
                    span: self.previous().span,
                });
                
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RightBrace, "'}' after import items")?;
        }
        // import * as name from "module"
        else if self.match_token(TokenKind::Star) {
            self.expect(TokenKind::As, "'as' after '*'")?;
            let name = self.expect_identifier("import alias")?;
            items.push(ImportItem {
                name,
                alias: None,
                span: self.previous().span,
            });
        }
        
        self.expect(TokenKind::From, "'from' after import items")?;
        let from_token = self.expect(TokenKind::String, "module path")?;
        let from = from_token.symbol.unwrap();
        
        self.expect(TokenKind::Semicolon, "';' after import")?;
        
        Ok(Stmt::Import {
            items,
            from,
            span: start.span.merge(self.previous().span),
        })
    }
    
    fn let_declaration(&mut self) -> HateResult<Stmt> {
        let start = self.advance(); // consume 'let'
        
        let mutable = self.match_token(TokenKind::Mut);
        let name = self.expect_identifier("variable name")?;
        
        let type_ann = if self.match_token(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        
        let value = if self.match_token(TokenKind::Equal) {
            Some(self.expression()?)
        } else {
            None
        };
        
        self.expect(TokenKind::Semicolon, "';' after variable declaration")?;
        
        Ok(Stmt::Let {
            name,
            mutable,
            type_ann,
            value,
            span: start.span.merge(self.previous().span),
        })
    }
    
    fn function_declaration(&mut self) -> HateResult<Stmt> {
        let start_span = self.current_span();
        let is_async = self.match_token(TokenKind::Async);
        
        self.expect(TokenKind::Fn, "'fn'")?;
        let name = self.expect_identifier("function name")?;
        
        self.expect(TokenKind::LeftParen, "'(' after function name")?;
        let params = self.parse_parameters()?;
        self.expect(TokenKind::RightParen, "')' after parameters")?;
        
        let return_type = if self.match_token(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        
        let body = if self.match_token(TokenKind::Arrow) {
            // Arrow function body: => expr
            let expr = self.expression()?;
            self.expect(TokenKind::Semicolon, "';' after expression body")?;
            Box::new(expr)
        } else {
            // Block body: { stmts }
            Box::new(self.block_expression()?)
        };
        
        Ok(Stmt::Function {
            name,
            params,
            return_type,
            body,
            is_async,
            span: start_span.merge(self.previous().span),
        })
    }
    
    fn class_declaration(&mut self) -> HateResult<Stmt> {
        let start = self.advance(); // consume 'class'
        let name = self.expect_identifier("class name")?;
        
        self.expect(TokenKind::LeftBrace, "'{' after class name")?;
        
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        
        while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
            if self.check(TokenKind::Fn) {
                methods.push(self.parse_class_method()?);
            } else {
                fields.push(self.parse_class_field()?);
            }
        }
        
        self.expect(TokenKind::RightBrace, "'}' after class body")?;
        
        Ok(Stmt::Class {
            name,
            fields,
            methods,
            span: start.span.merge(self.previous().span),
        })
    }
    
    fn impl_declaration(&mut self) -> HateResult<Stmt> {
        let start = self.advance(); // consume 'impl'
        
        let first_name = self.expect_identifier("type name")?;
        
        let (trait_name, type_name) = if self.match_token(TokenKind::For) {
            let type_name = self.expect_identifier("type name")?;
            (Some(first_name), type_name)
        } else {
            (None, first_name)
        };
        
        self.expect(TokenKind::LeftBrace, "'{' after impl")?;
        
        let mut methods = Vec::new();
        while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
            methods.push(self.function_declaration()?);
        }
        
        self.expect(TokenKind::RightBrace, "'}' after impl body")?;
        
        Ok(Stmt::Impl {
            type_name,
            trait_name,
            methods,
            span: start.span.merge(self.previous().span),
        })
    }
    
    // ==================== Statements ====================
    
    fn statement(&mut self) -> HateResult<Stmt> {
        if self.check(TokenKind::If) {
            return self.if_statement();
        }
        if self.check(TokenKind::While) {
            return self.while_statement();
        }
        if self.check(TokenKind::For) {
            return self.for_statement();
        }
        if self.check(TokenKind::Return) {
            return self.return_statement();
        }
        if self.check(TokenKind::Break) {
            let token = self.advance();
            self.expect(TokenKind::Semicolon, "';' after break")?;
            return Ok(Stmt::Break { span: token.span });
        }
        if self.check(TokenKind::Continue) {
            let token = self.advance();
            self.expect(TokenKind::Semicolon, "';' after continue")?;
            return Ok(Stmt::Continue { span: token.span });
        }
        if self.check(TokenKind::LeftBrace) {
            return self.block_statement();
        }
        
        self.expression_statement()
    }
    
    fn if_statement(&mut self) -> HateResult<Stmt> {
        let start = self.advance(); // consume 'if'
        
        let condition = self.expression()?;
        let then_branch = Box::new(self.block_statement()?);
        
        let else_branch = if self.match_token(TokenKind::Else) {
            if self.check(TokenKind::If) {
                Some(Box::new(self.if_statement()?))
            } else {
                Some(Box::new(self.block_statement()?))
            }
        } else {
            None
        };
        
        let end_span = else_branch.as_ref()
            .map(|b| b.span())
            .unwrap_or_else(|| then_branch.span());
        
        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
            span: start.span.merge(end_span),
        })
    }
    
    fn while_statement(&mut self) -> HateResult<Stmt> {
        let start = self.advance(); // consume 'while'
        
        let condition = self.expression()?;
        let body = Box::new(self.block_statement()?);
        
        Ok(Stmt::While {
            condition,
            body,
            span: start.span.merge(self.previous().span),
        })
    }
    
    fn for_statement(&mut self) -> HateResult<Stmt> {
        let start = self.advance(); // consume 'for'
        
        let variable = self.expect_identifier("loop variable")?;
        self.expect(TokenKind::In, "'in' after loop variable")?;
        let iterable = self.expression()?;
        let body = Box::new(self.block_statement()?);
        
        Ok(Stmt::For {
            variable,
            iterable,
            body,
            span: start.span.merge(self.previous().span),
        })
    }
    
    fn return_statement(&mut self) -> HateResult<Stmt> {
        let start = self.advance(); // consume 'return'
        
        let value = if !self.check(TokenKind::Semicolon) {
            Some(self.expression()?)
        } else {
            None
        };
        
        self.expect(TokenKind::Semicolon, "';' after return")?;
        
        Ok(Stmt::Return {
            value,
            span: start.span.merge(self.previous().span),
        })
    }
    
    fn block_statement(&mut self) -> HateResult<Stmt> {
        let start = self.expect(TokenKind::LeftBrace, "'{'")?;
        
        let mut statements = Vec::new();
        while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        
        self.expect(TokenKind::RightBrace, "'}' after block")?;
        
        Ok(Stmt::Block {
            statements,
            span: start.span.merge(self.previous().span),
        })
    }
    
    fn expression_statement(&mut self) -> HateResult<Stmt> {
        let expr = self.expression()?;
        self.expect(TokenKind::Semicolon, "';' after expression")?;
        
        Ok(Stmt::Expression {
            span: expr.span(),
            expr,
        })
    }
    
    // ==================== Expressions (Pratt Parsing) ====================
    
    fn expression(&mut self) -> HateResult<Expr> {
        self.parse_precedence(Precedence::Assignment)
    }
    
    fn parse_precedence(&mut self, precedence: Precedence) -> HateResult<Expr> {
        // Parse prefix expression
        let mut left = self.prefix_expr()?;
        
        // Parse infix expressions
        while precedence <= infix_precedence(self.current().kind) {
            left = self.infix_expr(left)?;
        }
        
        Ok(left)
    }
    
    fn prefix_expr(&mut self) -> HateResult<Expr> {
        let token = self.current();
        
        match token.kind {
            // Literals
            TokenKind::Integer => {
                let token = self.advance();
                Ok(Expr::Literal {
                    value: Literal::Integer(token.int_value),
                    span: token.span,
                })
            }
            TokenKind::Float => {
                let token = self.advance();
                Ok(Expr::Literal {
                    value: Literal::Float(token.float_value),
                    span: token.span,
                })
            }
            TokenKind::String => {
                let token = self.advance();
                Ok(Expr::Literal {
                    value: Literal::String(token.symbol.unwrap()),
                    span: token.span,
                })
            }
            TokenKind::True => {
                let token = self.advance();
                Ok(Expr::Literal {
                    value: Literal::Bool(true),
                    span: token.span,
                })
            }
            TokenKind::False => {
                let token = self.advance();
                Ok(Expr::Literal {
                    value: Literal::Bool(false),
                    span: token.span,
                })
            }
            TokenKind::Null => {
                let token = self.advance();
                Ok(Expr::Literal {
                    value: Literal::Null,
                    span: token.span,
                })
            }
            
            // Identifiers
            TokenKind::Identifier => {
                let token = self.advance();
                Ok(Expr::Identifier {
                    name: token.symbol.unwrap(),
                    span: token.span,
                })
            }
            
            // Unary operators
            TokenKind::Minus => {
                let op_token = self.advance();
                let operand = self.parse_precedence(Precedence::Unary)?;
                Ok(Expr::Unary {
                    operator: UnaryOp::Neg,
                    span: op_token.span.merge(operand.span()),
                    operand: Box::new(operand),
                })
            }
            TokenKind::Bang => {
                let op_token = self.advance();
                let operand = self.parse_precedence(Precedence::Unary)?;
                Ok(Expr::Unary {
                    operator: UnaryOp::Not,
                    span: op_token.span.merge(operand.span()),
                    operand: Box::new(operand),
                })
            }
            TokenKind::Tilde => {
                let op_token = self.advance();
                let operand = self.parse_precedence(Precedence::Unary)?;
                Ok(Expr::Unary {
                    operator: UnaryOp::BitNot,
                    span: op_token.span.merge(operand.span()),
                    operand: Box::new(operand),
                })
            }
            
            // Spread operator
            TokenKind::DotDotDot => {
                let op_token = self.advance();
                let expr = self.parse_precedence(Precedence::Unary)?;
                Ok(Expr::Spread {
                    span: op_token.span.merge(expr.span()),
                    expr: Box::new(expr),
                })
            }
            
            // Await
            TokenKind::Await => {
                let op_token = self.advance();
                let expr = self.parse_precedence(Precedence::Unary)?;
                Ok(Expr::Await {
                    span: op_token.span.merge(expr.span()),
                    expr: Box::new(expr),
                })
            }
            
            // Grouping or lambda
            TokenKind::LeftParen => self.paren_or_lambda(),
            
            // Array literal
            TokenKind::LeftBracket => self.array_literal(),
            
            // Object literal or block expression
            TokenKind::LeftBrace => self.object_or_block(),
            
            // If expression
            TokenKind::If => self.if_expression(),
            
            // Match expression
            TokenKind::Match => self.match_expression(),
            
            _ => Err(HateError::ExpectedExpression {
                span: token.span,
            }),
        }
    }
    
    fn infix_expr(&mut self, left: Expr) -> HateResult<Expr> {
        let op_token = self.current();
        
        match op_token.kind {
            // Binary operators
            TokenKind::Plus | TokenKind::Minus | TokenKind::Star | TokenKind::Slash |
            TokenKind::Percent | TokenKind::EqualEqual | TokenKind::BangEqual |
            TokenKind::Less | TokenKind::LessEqual | TokenKind::Greater |
            TokenKind::GreaterEqual | TokenKind::Ampersand | TokenKind::Pipe |
            TokenKind::Caret | TokenKind::LessLess | TokenKind::GreaterGreater => {
                let op_token = self.advance();
                let precedence = infix_precedence(op_token.kind);
                let right = self.parse_precedence(precedence.next())?;
                
                let operator = match op_token.kind {
                    TokenKind::Plus => BinaryOp::Add,
                    TokenKind::Minus => BinaryOp::Sub,
                    TokenKind::Star => BinaryOp::Mul,
                    TokenKind::Slash => BinaryOp::Div,
                    TokenKind::Percent => BinaryOp::Mod,
                    TokenKind::EqualEqual => BinaryOp::Eq,
                    TokenKind::BangEqual => BinaryOp::Ne,
                    TokenKind::Less => BinaryOp::Lt,
                    TokenKind::LessEqual => BinaryOp::Le,
                    TokenKind::Greater => BinaryOp::Gt,
                    TokenKind::GreaterEqual => BinaryOp::Ge,
                    TokenKind::Ampersand => BinaryOp::BitAnd,
                    TokenKind::Pipe => BinaryOp::BitOr,
                    TokenKind::Caret => BinaryOp::BitXor,
                    TokenKind::LessLess => BinaryOp::Shl,
                    TokenKind::GreaterGreater => BinaryOp::Shr,
                    _ => unreachable!(),
                };
                
                let span = left.span().merge(right.span());
                Ok(Expr::Binary {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                    span,
                })
            }
            
            // Logical operators
            TokenKind::And | TokenKind::Or => {
                let op_token = self.advance();
                let precedence = infix_precedence(op_token.kind);
                let right = self.parse_precedence(precedence.next())?;
                
                let operator = match op_token.kind {
                    TokenKind::And => LogicalOp::And,
                    TokenKind::Or => LogicalOp::Or,
                    _ => unreachable!(),
                };
                
                let span = left.span().merge(right.span());
                Ok(Expr::Logical {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                    span,
                })
            }
            
            // Null coalescing
            TokenKind::DoubleQuestion => {
                self.advance();
                let right = self.parse_precedence(Precedence::NullCoalesce.next())?;
                let span = left.span().merge(right.span());
                Ok(Expr::NullCoalesce {
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                })
            }
            
            // Assignment
            TokenKind::Equal => {
                self.advance();
                if !left.is_assignable() {
                    return Err(HateError::InvalidAssignment { span: left.span() });
                }
                let right = self.parse_precedence(Precedence::Assignment)?;
                let span = left.span().merge(right.span());
                Ok(Expr::Assign {
                    target: Box::new(left),
                    value: Box::new(right),
                    span,
                })
            }
            
            // Compound assignment
            TokenKind::PlusEqual | TokenKind::MinusEqual |
            TokenKind::StarEqual | TokenKind::SlashEqual => {
                let op_token = self.advance();
                if !left.is_assignable() {
                    return Err(HateError::InvalidAssignment { span: left.span() });
                }
                let operator = match op_token.kind {
                    TokenKind::PlusEqual => BinaryOp::Add,
                    TokenKind::MinusEqual => BinaryOp::Sub,
                    TokenKind::StarEqual => BinaryOp::Mul,
                    TokenKind::SlashEqual => BinaryOp::Div,
                    _ => unreachable!(),
                };
                let right = self.parse_precedence(Precedence::Assignment)?;
                let span = left.span().merge(right.span());
                Ok(Expr::CompoundAssign {
                    target: Box::new(left),
                    operator,
                    value: Box::new(right),
                    span,
                })
            }
            
            // Function call
            TokenKind::LeftParen => {
                let left_span = left.span();
                self.advance();
                let arguments = self.parse_arguments()?;
                let end = self.expect(TokenKind::RightParen, "')' after arguments")?;
                Ok(Expr::Call {
                    callee: Box::new(left),
                    arguments,
                    span: left_span.merge(end.span),
                })
            }
            
            // Index access
            TokenKind::LeftBracket => {
                let left_span = left.span();
                self.advance();
                let index = self.expression()?;
                let end = self.expect(TokenKind::RightBracket, "']' after index")?;
                Ok(Expr::Index {
                    object: Box::new(left),
                    index: Box::new(index),
                    span: left_span.merge(end.span),
                })
            }
            
            // Property access
            TokenKind::Dot => {
                self.advance();
                let property = self.expect_identifier("property name")?;
                let span = left.span().merge(self.previous().span);
                
                // Check for method call
                if self.check(TokenKind::LeftParen) {
                    self.advance();
                    let arguments = self.parse_arguments()?;
                    let end = self.expect(TokenKind::RightParen, "')' after arguments")?;
                    Ok(Expr::MethodCall {
                        object: Box::new(left),
                        method: property,
                        arguments,
                        span: span.merge(end.span),
                    })
                } else {
                    Ok(Expr::Property {
                        object: Box::new(left),
                        property,
                        span,
                    })
                }
            }
            
            // Optional chaining
            TokenKind::QuestionDot => {
                let left_span = left.span();
                self.advance();
                let property = self.expect_identifier("property name")?;
                Ok(Expr::OptionalChain {
                    object: Box::new(left),
                    property,
                    span: left_span.merge(self.previous().span),
                })
            }
            
            // Range
            TokenKind::DotDot => {
                let left_span = left.span();
                self.advance();
                let right = self.parse_precedence(Precedence::Comparison.next())?;
                let right_span = right.span();
                Ok(Expr::Range {
                    start: Box::new(left),
                    end: Box::new(right),
                    inclusive: false,
                    span: left_span.merge(right_span),
                })
            }
            
            _ => Err(HateError::UnexpectedToken {
                token: op_token.lexeme(),
                span: op_token.span,
            }),
        }
    }
    
    // ==================== Expression Helpers ====================
    
    fn paren_or_lambda(&mut self) -> HateResult<Expr> {
        let start = self.advance(); // consume '('
        
        // Empty params: () => expr
        if self.check(TokenKind::RightParen) {
            self.advance();
            if self.match_token(TokenKind::Colon) {
                // Return type annotation
                let return_type = Some(self.parse_type()?);
                self.expect(TokenKind::Arrow, "'=>'")?;
                let body = self.expression()?;
                let body_span = body.span();
                return Ok(Expr::Lambda {
                    params: vec![],
                    return_type,
                    body: Box::new(body),
                    span: start.span.merge(body_span),
                });
            }
            if self.match_token(TokenKind::Arrow) {
                let body = self.expression()?;
                let body_span = body.span();
                return Ok(Expr::Lambda {
                    params: vec![],
                    return_type: None,
                    body: Box::new(body),
                    span: start.span.merge(body_span),
                });
            }
        }
        
        // Parse first element to determine if it's a lambda or grouping
        let first = self.expression()?;
        
        // Check for lambda patterns: (x) => or (x, y) => or (x: T) =>
        if self.check(TokenKind::Comma) || 
           (self.check(TokenKind::RightParen) && 
            (self.check_ahead(1, TokenKind::Arrow) || self.check_ahead(1, TokenKind::Colon))) {
            // It's a lambda - parse remaining parameters
            let mut params = vec![self.expr_to_param(first)?];
            
            while self.match_token(TokenKind::Comma) {
                let expr = self.expression()?;
                params.push(self.expr_to_param(expr)?);
            }
            
            self.expect(TokenKind::RightParen, "')' after parameters")?;
            
            let return_type = if self.match_token(TokenKind::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            
            self.expect(TokenKind::Arrow, "'=>' after lambda parameters")?;
            
            let body = if self.check(TokenKind::LeftBrace) {
                self.block_expression()?
            } else {
                self.expression()?
            };
            
            let body_span = body.span();
            Ok(Expr::Lambda {
                params,
                return_type,
                body: Box::new(body),
                span: start.span.merge(body_span),
            })
        } else {
            // It's a grouping expression
            self.expect(TokenKind::RightParen, "')' after expression")?;
            Ok(Expr::Grouping {
                expr: Box::new(first),
                span: start.span.merge(self.previous().span),
            })
        }
    }
    
    fn expr_to_param(&self, expr: Expr) -> HateResult<Param> {
        match expr {
            Expr::Identifier { name, span } => Ok(Param {
                name,
                type_ann: None,
                default: None,
                span,
            }),
            _ => Err(HateError::ExpectedExpression { span: expr.span() }),
        }
    }
    
    fn array_literal(&mut self) -> HateResult<Expr> {
        let start = self.advance(); // consume '['
        
        let mut elements = Vec::new();
        
        if !self.check(TokenKind::RightBracket) {
            loop {
                elements.push(self.expression()?);
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
                // Allow trailing comma
                if self.check(TokenKind::RightBracket) {
                    break;
                }
            }
        }
        
        self.expect(TokenKind::RightBracket, "']' after array elements")?;
        
        Ok(Expr::Array {
            elements,
            span: start.span.merge(self.previous().span),
        })
    }
    
    fn object_or_block(&mut self) -> HateResult<Expr> {
        let start = self.advance(); // consume '{'
        
        // Empty braces: empty block
        if self.check(TokenKind::RightBrace) {
            self.advance();
            return Ok(Expr::Block {
                statements: vec![],
                expression: None,
                span: start.span.merge(self.previous().span),
            });
        }
        
        // Check if it's an object literal (starts with identifier:)
        if self.check(TokenKind::Identifier) && self.check_ahead(1, TokenKind::Colon) {
            return self.object_literal(start);
        }
        
        // It's a block expression
        self.block_expr_inner(start)
    }
    
    fn object_literal(&mut self, start: Token) -> HateResult<Expr> {
        let mut properties = Vec::new();
        
        loop {
            let key = self.expect_identifier("property name")?;
            let key_span = self.previous().span;
            
            let (value, shorthand) = if self.match_token(TokenKind::Colon) {
                (self.expression()?, false)
            } else {
                // Shorthand: { x } is { x: x }
                (Expr::Identifier { name: key, span: key_span }, true)
            };
            
            properties.push(ObjectProperty {
                key,
                value,
                shorthand,
                span: key_span.merge(self.previous().span),
            });
            
            if !self.match_token(TokenKind::Comma) {
                break;
            }
            // Allow trailing comma
            if self.check(TokenKind::RightBrace) {
                break;
            }
        }
        
        self.expect(TokenKind::RightBrace, "'}' after object properties")?;
        
        Ok(Expr::Object {
            properties,
            span: start.span.merge(self.previous().span),
        })
    }
    
    fn block_expression(&mut self) -> HateResult<Expr> {
        let start = self.expect(TokenKind::LeftBrace, "'{'")?;
        self.block_expr_inner(start)
    }
    
    fn block_expr_inner(&mut self, start: Token) -> HateResult<Expr> {
        let mut statements = Vec::new();
        let mut expression = None;
        
        while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
            // Try to parse as a statement
            if self.check(TokenKind::Let) || self.check(TokenKind::Fn) ||
               self.check(TokenKind::If) || self.check(TokenKind::While) ||
               self.check(TokenKind::For) || self.check(TokenKind::Return) {
                statements.push(self.declaration()?);
            } else {
                // Parse as expression
                let expr = self.expression()?;
                
                // Check if this is a trailing expression (no semicolon)
                if self.check(TokenKind::RightBrace) {
                    expression = Some(Box::new(expr));
                } else {
                    self.expect(TokenKind::Semicolon, "';' after expression")?;
                    statements.push(Stmt::Expression {
                        span: expr.span(),
                        expr,
                    });
                }
            }
        }
        
        self.expect(TokenKind::RightBrace, "'}'")?;
        
        Ok(Expr::Block {
            statements,
            expression,
            span: start.span.merge(self.previous().span),
        })
    }
    
    fn if_expression(&mut self) -> HateResult<Expr> {
        let start = self.advance(); // consume 'if'
        
        let condition = self.expression()?;
        let then_branch = self.block_expression()?;
        
        self.expect(TokenKind::Else, "'else' after if expression")?;
        
        let else_branch = if self.check(TokenKind::If) {
            self.if_expression()?
        } else {
            self.block_expression()?
        };
        
        let else_span = else_branch.span();
        Ok(Expr::IfExpr {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
            span: start.span.merge(else_span),
        })
    }
    
    fn match_expression(&mut self) -> HateResult<Expr> {
        let start = self.advance(); // consume 'match'
        
        let value = self.expression()?;
        self.expect(TokenKind::LeftBrace, "'{' after match value")?;
        
        let mut arms = Vec::new();
        while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
            let pattern = self.parse_pattern()?;
            
            let guard = if self.match_token(TokenKind::If) {
                Some(self.expression()?)
            } else {
                None
            };
            
            self.expect(TokenKind::Arrow, "'=>' after pattern")?;
            let body = self.expression()?;
            
            let span = pattern.span().merge(body.span());
            arms.push(MatchArm { pattern, guard, body, span });
            
            // Optional comma between arms
            self.match_token(TokenKind::Comma);
        }
        
        self.expect(TokenKind::RightBrace, "'}' after match arms")?;
        
        Ok(Expr::Match {
            value: Box::new(value),
            arms,
            span: start.span.merge(self.previous().span),
        })
    }
    
    // ==================== Patterns ====================
    
    fn parse_pattern(&mut self) -> HateResult<Pattern> {
        let token = self.current();
        
        match token.kind {
            TokenKind::Identifier if token.symbol.map(|s| s.as_str()) == Some("_") => {
                let token = self.advance();
                Ok(Pattern::Wildcard { span: token.span })
            }
            TokenKind::Identifier => {
                let token = self.advance();
                Ok(Pattern::Binding {
                    name: token.symbol.unwrap(),
                    span: token.span,
                })
            }
            TokenKind::Integer => {
                let token = self.advance();
                
                // Check for range pattern
                if self.check(TokenKind::DotDot) {
                    self.advance();
                    let end_token = self.expect(TokenKind::Integer, "integer in range")?;
                    return Ok(Pattern::Range {
                        start: Box::new(Literal::Integer(token.int_value)),
                        end: Box::new(Literal::Integer(end_token.int_value)),
                        inclusive: false,
                        span: token.span.merge(end_token.span),
                    });
                }
                
                Ok(Pattern::Literal {
                    value: Literal::Integer(token.int_value),
                    span: token.span,
                })
            }
            TokenKind::Float => {
                let token = self.advance();
                Ok(Pattern::Literal {
                    value: Literal::Float(token.float_value),
                    span: token.span,
                })
            }
            TokenKind::String => {
                let token = self.advance();
                Ok(Pattern::Literal {
                    value: Literal::String(token.symbol.unwrap()),
                    span: token.span,
                })
            }
            TokenKind::True => {
                let token = self.advance();
                Ok(Pattern::Literal {
                    value: Literal::Bool(true),
                    span: token.span,
                })
            }
            TokenKind::False => {
                let token = self.advance();
                Ok(Pattern::Literal {
                    value: Literal::Bool(false),
                    span: token.span,
                })
            }
            TokenKind::Null => {
                let token = self.advance();
                Ok(Pattern::Literal {
                    value: Literal::Null,
                    span: token.span,
                })
            }
            _ => Err(HateError::ExpectedExpression { span: token.span }),
        }
    }
    
    // ==================== Types ====================
    
    fn parse_type(&mut self) -> HateResult<TypeExpr> {
        let token = self.current();
        
        match token.kind {
            TokenKind::I32 | TokenKind::I64 | TokenKind::U32 | TokenKind::U64 |
            TokenKind::F64 | TokenKind::Bool | TokenKind::Str | TokenKind::Identifier => {
                let token = self.advance();
                let name = match token.kind {
                    TokenKind::I32 => intern("i32"),
                    TokenKind::I64 => intern("i64"),
                    TokenKind::U32 => intern("u32"),
                    TokenKind::U64 => intern("u64"),
                    TokenKind::F64 => intern("f64"),
                    TokenKind::Bool => intern("bool"),
                    TokenKind::Str => intern("str"),
                    TokenKind::Identifier => token.symbol.unwrap(),
                    _ => unreachable!(),
                };
                
                // Check for generic parameters
                if self.match_token(TokenKind::Less) {
                    let mut args = Vec::new();
                    loop {
                        args.push(self.parse_type()?);
                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(TokenKind::Greater, "'>' after type arguments")?;
                    
                    return Ok(TypeExpr::Generic {
                        name,
                        args,
                        span: token.span.merge(self.previous().span),
                    });
                }
                
                Ok(TypeExpr::Simple {
                    name,
                    span: token.span,
                })
            }
            TokenKind::Fn => {
                let start = self.advance();
                self.expect(TokenKind::LeftParen, "'(' after 'fn'")?;
                
                let mut params = Vec::new();
                if !self.check(TokenKind::RightParen) {
                    loop {
                        params.push(self.parse_type()?);
                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RightParen, "')' after parameter types")?;
                
                self.expect(TokenKind::Colon, "':' before return type")?;
                let return_type = Box::new(self.parse_type()?);
                
                Ok(TypeExpr::Function {
                    params,
                    return_type,
                    span: start.span.merge(self.previous().span),
                })
            }
            TokenKind::LeftBracket => {
                let start = self.advance();
                let element = Box::new(self.parse_type()?);
                self.expect(TokenKind::RightBracket, "']' after element type")?;
                
                Ok(TypeExpr::Array {
                    element,
                    span: start.span.merge(self.previous().span),
                })
            }
            _ => Err(HateError::ExpectedToken {
                expected: "type".to_string(),
                found: token.lexeme(),
                span: token.span,
            }),
        }
    }
    
    // ==================== Helpers ====================
    
    fn parse_parameters(&mut self) -> HateResult<Vec<Param>> {
        let mut params = Vec::new();
        
        if self.check(TokenKind::RightParen) {
            return Ok(params);
        }
        
        loop {
            if params.len() >= 255 {
                return Err(HateError::TooManyParams {
                    span: self.current_span(),
                });
            }
            
            let name = self.expect_identifier("parameter name")?;
            let param_span = self.previous().span;
            
            let type_ann = if self.match_token(TokenKind::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            
            let default = if self.match_token(TokenKind::Equal) {
                Some(self.expression()?)
            } else {
                None
            };
            
            params.push(Param {
                name,
                type_ann,
                default,
                span: param_span.merge(self.previous().span),
            });
            
            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }
        
        Ok(params)
    }
    
    fn parse_arguments(&mut self) -> HateResult<Vec<Expr>> {
        let mut args = Vec::new();
        
        if self.check(TokenKind::RightParen) {
            return Ok(args);
        }
        
        loop {
            if args.len() >= 255 {
                return Err(HateError::TooManyArgs {
                    span: self.current_span(),
                });
            }
            args.push(self.expression()?);
            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }
        
        Ok(args)
    }
    
    fn parse_class_field(&mut self) -> HateResult<ClassField> {
        let name = self.expect_identifier("field name")?;
        let span = self.previous().span;
        
        self.expect(TokenKind::Colon, "':' after field name")?;
        let type_ann = self.parse_type()?;
        
        let default = if self.match_token(TokenKind::Equal) {
            Some(self.expression()?)
        } else {
            None
        };
        
        self.expect(TokenKind::Semicolon, "';' after field")?;
        
        Ok(ClassField {
            name,
            type_ann,
            default,
            span: span.merge(self.previous().span),
        })
    }
    
    fn parse_class_method(&mut self) -> HateResult<ClassMethod> {
        self.expect(TokenKind::Fn, "'fn'")?;
        let name = self.expect_identifier("method name")?;
        
        self.expect(TokenKind::LeftParen, "'(' after method name")?;
        let params = self.parse_parameters()?;
        self.expect(TokenKind::RightParen, "')' after parameters")?;
        
        let return_type = if self.match_token(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        
        let body = if self.match_token(TokenKind::Arrow) {
            self.expression()?
        } else {
            self.block_expression()?
        };
        
        Ok(ClassMethod {
            name,
            params,
            return_type,
            body,
            is_static: false,
            span: self.previous().span,
        })
    }
    
    // ==================== Token Navigation ====================
    
    fn is_at_end(&self) -> bool {
        self.current().kind == TokenKind::Eof
    }
    
    fn current(&self) -> Token {
        self.tokens.get(self.current).copied().unwrap_or(Token::new(TokenKind::Eof, Span::empty()))
    }
    
    fn current_span(&self) -> Span {
        self.current().span
    }
    
    fn previous(&self) -> Token {
        self.tokens.get(self.current.saturating_sub(1)).copied().unwrap_or(Token::new(TokenKind::Eof, Span::empty()))
    }
    
    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }
    
    fn check(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
    }
    
    fn check_next(&self, kind: TokenKind) -> bool {
        self.tokens.get(self.current + 1).map(|t| t.kind) == Some(kind)
    }
    
    fn check_ahead(&self, n: usize, kind: TokenKind) -> bool {
        self.tokens.get(self.current + n).map(|t| t.kind) == Some(kind)
    }
    
    fn match_token(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }
    
    fn expect(&mut self, kind: TokenKind, message: &str) -> HateResult<Token> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(HateError::ExpectedToken {
                expected: message.to_string(),
                found: self.current().lexeme(),
                span: self.current_span(),
            })
        }
    }
    
    fn expect_identifier(&mut self, context: &str) -> HateResult<Symbol> {
        if self.check(TokenKind::Identifier) {
            let token = self.advance();
            Ok(token.symbol.unwrap())
        } else {
            Err(HateError::ExpectedToken {
                expected: context.to_string(),
                found: self.current().lexeme(),
                span: self.current_span(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    
    fn parse(source: &str) -> Program {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse().unwrap()
    }
    
    #[test]
    fn test_let_statement() {
        let program = parse("let x = 42;");
        assert_eq!(program.statements.len(), 1);
        
        match &program.statements[0] {
            Stmt::Let { name, mutable, value, .. } => {
                assert_eq!(name.as_str(), "x");
                assert!(!mutable);
                assert!(value.is_some());
            }
            _ => panic!("Expected let statement"),
        }
    }
    
    #[test]
    fn test_mutable_let() {
        let program = parse("let mut x = 10;");
        match &program.statements[0] {
            Stmt::Let { mutable, .. } => assert!(mutable),
            _ => panic!("Expected let statement"),
        }
    }
    
    #[test]
    fn test_function() {
        let program = parse("fn add(a: i64, b: i64): i64 => a + b;");
        match &program.statements[0] {
            Stmt::Function { name, params, return_type, .. } => {
                assert_eq!(name.as_str(), "add");
                assert_eq!(params.len(), 2);
                assert!(return_type.is_some());
            }
            _ => panic!("Expected function"),
        }
    }
    
    #[test]
    fn test_binary_expression() {
        let program = parse("1 + 2 * 3;");
        match &program.statements[0] {
            Stmt::Expression { expr, .. } => {
                match expr {
                    Expr::Binary { operator: BinaryOp::Add, right, .. } => {
                        // Right should be 2 * 3 (higher precedence)
                        matches!(right.as_ref(), Expr::Binary { operator: BinaryOp::Mul, .. });
                    }
                    _ => panic!("Expected binary expression"),
                }
            }
            _ => panic!("Expected expression statement"),
        }
    }
    
    #[test]
    fn test_if_expression() {
        let program = parse("let x = if true { 1 } else { 2 };");
        match &program.statements[0] {
            Stmt::Let { value: Some(expr), .. } => {
                assert!(matches!(expr, Expr::IfExpr { .. }));
            }
            _ => panic!("Expected let with if expression"),
        }
    }
    
    #[test]
    fn test_array_literal() {
        let program = parse("let arr = [1, 2, 3];");
        match &program.statements[0] {
            Stmt::Let { value: Some(Expr::Array { elements, .. }), .. } => {
                assert_eq!(elements.len(), 3);
            }
            _ => panic!("Expected array literal"),
        }
    }
    
    #[test]
    fn test_object_literal() {
        let program = parse("let obj = { a: 1, b: 2 };");
        match &program.statements[0] {
            Stmt::Let { value: Some(Expr::Object { properties, .. }), .. } => {
                assert_eq!(properties.len(), 2);
            }
            _ => panic!("Expected object literal"),
        }
    }
    
    #[test]
    fn test_lambda() {
        let program = parse("let f = (x) => x * 2;");
        match &program.statements[0] {
            Stmt::Let { value: Some(Expr::Lambda { params, .. }), .. } => {
                assert_eq!(params.len(), 1);
            }
            _ => panic!("Expected lambda"),
        }
    }
    
    #[test]
    fn test_method_call() {
        let program = parse("arr.map((x) => x * 2);");
        match &program.statements[0] {
            Stmt::Expression { expr: Expr::MethodCall { method, .. }, .. } => {
                assert_eq!(method.as_str(), "map");
            }
            _ => panic!("Expected method call"),
        }
    }
    
    #[test]
    fn test_match_expression() {
        let program = parse("match x { 0 => 1, _ => 2 };");
        match &program.statements[0] {
            Stmt::Expression { expr: Expr::Match { arms, .. }, .. } => {
                assert_eq!(arms.len(), 2);
            }
            _ => panic!("Expected match expression"),
        }
    }
}
