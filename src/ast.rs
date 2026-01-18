//! Abstract Syntax Tree for the Hate programming language
//!
//! Defines the complete AST representation for all Hate language constructs.

use crate::intern::Symbol;
use crate::error::Span;

/// A complete Hate program
#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

/// Statement nodes
#[derive(Debug, Clone)]
pub enum Stmt {
    /// Variable declaration: `let x = expr;` or `let mut x = expr;`
    Let {
        name: Symbol,
        mutable: bool,
        type_ann: Option<TypeExpr>,
        value: Option<Expr>,
        span: Span,
    },
    
    /// Function declaration: `fn name(params): return_type { body }`
    Function {
        name: Symbol,
        params: Vec<Param>,
        return_type: Option<TypeExpr>,
        body: Box<Expr>,
        is_async: bool,
        span: Span,
    },
    
    /// Class declaration
    Class {
        name: Symbol,
        fields: Vec<ClassField>,
        methods: Vec<ClassMethod>,
        span: Span,
    },
    
    /// Expression statement
    Expression {
        expr: Expr,
        span: Span,
    },
    
    /// Return statement: `return expr;`
    Return {
        value: Option<Expr>,
        span: Span,
    },
    
    /// If statement: `if cond { then } else { else }`
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
        span: Span,
    },
    
    /// While loop: `while cond { body }`
    While {
        condition: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    
    /// For loop: `for item in iterable { body }`
    For {
        variable: Symbol,
        iterable: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    
    /// Block: `{ statements }`
    Block {
        statements: Vec<Stmt>,
        span: Span,
    },
    
    /// Break statement
    Break { span: Span },
    
    /// Continue statement
    Continue { span: Span },
    
    /// Import statement: `import { x, y } from "module";`
    Import {
        items: Vec<ImportItem>,
        from: Symbol,
        span: Span,
    },
    
    /// Export statement: `export fn foo() { }`
    Export {
        statement: Box<Stmt>,
        span: Span,
    },
    
    /// Impl block: `impl Type { methods }`
    Impl {
        type_name: Symbol,
        trait_name: Option<Symbol>,
        methods: Vec<Stmt>,
        span: Span,
    },
}

impl Stmt {
    /// Get the span of this statement
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } => *span,
            Stmt::Function { span, .. } => *span,
            Stmt::Class { span, .. } => *span,
            Stmt::Expression { span, .. } => *span,
            Stmt::Return { span, .. } => *span,
            Stmt::If { span, .. } => *span,
            Stmt::While { span, .. } => *span,
            Stmt::For { span, .. } => *span,
            Stmt::Block { span, .. } => *span,
            Stmt::Break { span } => *span,
            Stmt::Continue { span } => *span,
            Stmt::Import { span, .. } => *span,
            Stmt::Export { span, .. } => *span,
            Stmt::Impl { span, .. } => *span,
        }
    }
}

/// Expression nodes
#[derive(Debug, Clone)]
pub enum Expr {
    /// Literal values
    Literal {
        value: Literal,
        span: Span,
    },
    
    /// Variable reference
    Identifier {
        name: Symbol,
        span: Span,
    },
    
    /// Binary operation: `a + b`
    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },
    
    /// Unary operation: `-a`, `!a`
    Unary {
        operator: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    
    /// Logical operation: `a && b`, `a || b`
    Logical {
        left: Box<Expr>,
        operator: LogicalOp,
        right: Box<Expr>,
        span: Span,
    },
    
    /// Assignment: `a = b`
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
    
    /// Compound assignment: `a += b`
    CompoundAssign {
        target: Box<Expr>,
        operator: BinaryOp,
        value: Box<Expr>,
        span: Span,
    },
    
    /// Function call: `foo(a, b)`
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
        span: Span,
    },
    
    /// Method call: `obj.method(a, b)`
    MethodCall {
        object: Box<Expr>,
        method: Symbol,
        arguments: Vec<Expr>,
        span: Span,
    },
    
    /// Property access: `obj.prop`
    Property {
        object: Box<Expr>,
        property: Symbol,
        span: Span,
    },
    
    /// Optional chaining: `obj?.prop`
    OptionalChain {
        object: Box<Expr>,
        property: Symbol,
        span: Span,
    },
    
    /// Index access: `arr[i]`
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    
    /// Array literal: `[1, 2, 3]`
    Array {
        elements: Vec<Expr>,
        span: Span,
    },
    
    /// Object literal: `{ a: 1, b: 2 }`
    Object {
        properties: Vec<ObjectProperty>,
        span: Span,
    },
    
    /// Lambda/arrow function: `(x) => x * 2`
    Lambda {
        params: Vec<Param>,
        return_type: Option<TypeExpr>,
        body: Box<Expr>,
        span: Span,
    },
    
    /// If expression: `if cond { a } else { b }`
    IfExpr {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
        span: Span,
    },
    
    /// Match expression
    Match {
        value: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    
    /// Block expression: `{ stmts; expr }`
    Block {
        statements: Vec<Stmt>,
        expression: Option<Box<Expr>>,
        span: Span,
    },
    
    /// Range: `1..10`
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
        span: Span,
    },
    
    /// Spread: `...arr`
    Spread {
        expr: Box<Expr>,
        span: Span,
    },
    
    /// Await expression: `await future`
    Await {
        expr: Box<Expr>,
        span: Span,
    },
    
    /// Null coalescing: `a ?? b`
    NullCoalesce {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    
    /// Grouping (parentheses): `(expr)`
    Grouping {
        expr: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    /// Get the span of this expression
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal { span, .. } => *span,
            Expr::Identifier { span, .. } => *span,
            Expr::Binary { span, .. } => *span,
            Expr::Unary { span, .. } => *span,
            Expr::Logical { span, .. } => *span,
            Expr::Assign { span, .. } => *span,
            Expr::CompoundAssign { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::MethodCall { span, .. } => *span,
            Expr::Property { span, .. } => *span,
            Expr::OptionalChain { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::Array { span, .. } => *span,
            Expr::Object { span, .. } => *span,
            Expr::Lambda { span, .. } => *span,
            Expr::IfExpr { span, .. } => *span,
            Expr::Match { span, .. } => *span,
            Expr::Block { span, .. } => *span,
            Expr::Range { span, .. } => *span,
            Expr::Spread { span, .. } => *span,
            Expr::Await { span, .. } => *span,
            Expr::NullCoalesce { span, .. } => *span,
            Expr::Grouping { span, .. } => *span,
        }
    }
    
    /// Check if this expression is a valid assignment target
    pub fn is_assignable(&self) -> bool {
        matches!(self,
            Expr::Identifier { .. } |
            Expr::Property { .. } |
            Expr::Index { .. }
        )
    }
}

/// Literal values
#[derive(Debug, Clone)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(Symbol),
    Bool(bool),
    Null,
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    
    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    
    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl BinaryOp {
    /// Get the string representation of this operator
    pub fn as_str(&self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::Shl => "<<",
            BinaryOp::Shr => ">>",
        }
    }
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,     // -
    Not,     // !
    BitNot,  // ~
}

impl UnaryOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
            UnaryOp::BitNot => "~",
        }
    }
}

/// Logical operators (short-circuiting)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    And,  // &&
    Or,   // ||
}

impl LogicalOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogicalOp::And => "&&",
            LogicalOp::Or => "||",
        }
    }
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Param {
    pub name: Symbol,
    pub type_ann: Option<TypeExpr>,
    pub default: Option<Expr>,
    pub span: Span,
}

/// Type expression
#[derive(Debug, Clone)]
pub enum TypeExpr {
    /// Simple type: `i64`, `str`, `MyType`
    Simple {
        name: Symbol,
        span: Span,
    },
    
    /// Generic type: `Array<T>`, `Result<T, E>`
    Generic {
        name: Symbol,
        args: Vec<TypeExpr>,
        span: Span,
    },
    
    /// Function type: `fn(i64, i64): i64`
    Function {
        params: Vec<TypeExpr>,
        return_type: Box<TypeExpr>,
        span: Span,
    },
    
    /// Optional type: `T?`
    Optional {
        inner: Box<TypeExpr>,
        span: Span,
    },
    
    /// Array type: `[T]`
    Array {
        element: Box<TypeExpr>,
        span: Span,
    },
}

impl TypeExpr {
    pub fn span(&self) -> Span {
        match self {
            TypeExpr::Simple { span, .. } => *span,
            TypeExpr::Generic { span, .. } => *span,
            TypeExpr::Function { span, .. } => *span,
            TypeExpr::Optional { span, .. } => *span,
            TypeExpr::Array { span, .. } => *span,
        }
    }
}

/// Object property in object literal
#[derive(Debug, Clone)]
pub struct ObjectProperty {
    pub key: Symbol,
    pub value: Expr,
    pub shorthand: bool,  // `{ x }` is shorthand for `{ x: x }`
    pub span: Span,
}

/// Match arm
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

/// Pattern for pattern matching
#[derive(Debug, Clone)]
pub enum Pattern {
    /// Wildcard: `_`
    Wildcard { span: Span },
    
    /// Literal pattern: `42`, `"hello"`
    Literal { value: Literal, span: Span },
    
    /// Variable binding: `x`
    Binding { name: Symbol, span: Span },
    
    /// Range pattern: `1..10`
    Range { start: Box<Literal>, end: Box<Literal>, inclusive: bool, span: Span },
    
    /// Constructor pattern: `Some(x)`
    Constructor { name: Symbol, args: Vec<Pattern>, span: Span },
    
    /// Array pattern: `[a, b, ...rest]`
    Array { elements: Vec<Pattern>, rest: Option<Symbol>, span: Span },
    
    /// Object pattern: `{ x, y: z }`
    Object { properties: Vec<(Symbol, Pattern)>, rest: Option<Symbol>, span: Span },
    
    /// Or pattern: `1 | 2 | 3`
    Or { patterns: Vec<Pattern>, span: Span },
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wildcard { span } => *span,
            Pattern::Literal { span, .. } => *span,
            Pattern::Binding { span, .. } => *span,
            Pattern::Range { span, .. } => *span,
            Pattern::Constructor { span, .. } => *span,
            Pattern::Array { span, .. } => *span,
            Pattern::Object { span, .. } => *span,
            Pattern::Or { span, .. } => *span,
        }
    }
}

/// Class field
#[derive(Debug, Clone)]
pub struct ClassField {
    pub name: Symbol,
    pub type_ann: TypeExpr,
    pub default: Option<Expr>,
    pub span: Span,
}

/// Class method
#[derive(Debug, Clone)]
pub struct ClassMethod {
    pub name: Symbol,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Expr,
    pub is_static: bool,
    pub span: Span,
}

/// Import item
#[derive(Debug, Clone)]
pub struct ImportItem {
    pub name: Symbol,
    pub alias: Option<Symbol>,
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intern::intern;
    
    #[test]
    fn test_expr_span() {
        let span = Span::new(0, 5, 1, 1);
        let expr = Expr::Literal {
            value: Literal::Integer(42),
            span,
        };
        assert_eq!(expr.span(), span);
    }
    
    #[test]
    fn test_assignable() {
        let span = Span::empty();
        
        let ident = Expr::Identifier { name: intern("x"), span };
        assert!(ident.is_assignable());
        
        let lit = Expr::Literal { value: Literal::Integer(1), span };
        assert!(!lit.is_assignable());
    }
}
