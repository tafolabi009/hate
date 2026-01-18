//! Code Formatter
//!
//! Pretty-prints Hate source code with configurable style:
//! - Consistent indentation
//! - Line width limits
//! - Trailing commas
//! - Bracket spacing

use crate::ast::{Expr, Stmt, Literal, Param, Pattern};

/// Formatter configuration
#[derive(Debug, Clone)]
pub struct FormatConfig {
    /// Indentation width (spaces)
    pub indent_width: usize,
    /// Use tabs instead of spaces
    pub use_tabs: bool,
    /// Maximum line width
    pub line_width: usize,
    /// Add trailing commas in multi-line constructs
    pub trailing_commas: bool,
    /// Space inside brackets: [ 1, 2 ] vs [1, 2]
    pub bracket_spacing: bool,
    /// Space inside braces: { a: 1 } vs {a: 1}
    pub brace_spacing: bool,
    /// Semicolons at end of statements
    pub semicolons: bool,
    /// Single quotes for strings
    pub single_quote: bool,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            indent_width: 2,
            use_tabs: false,
            line_width: 80,
            trailing_commas: true,
            bracket_spacing: false,
            brace_spacing: true,
            semicolons: false,
            single_quote: false,
        }
    }
}

/// Code formatter
pub struct Formatter {
    config: FormatConfig,
    output: String,
    indent_level: usize,
    current_line_width: usize,
}

impl Formatter {
    pub fn new(config: FormatConfig) -> Self {
        Self {
            config,
            output: String::new(),
            indent_level: 0,
            current_line_width: 0,
        }
    }
    
    /// Format a list of statements
    pub fn format(&mut self, stmts: &[Stmt]) -> String {
        self.output.clear();
        self.indent_level = 0;
        self.current_line_width = 0;
        
        for (i, stmt) in stmts.iter().enumerate() {
            self.format_stmt(stmt);
            if i < stmts.len() - 1 {
                self.newline();
            }
        }
        
        self.output.clone()
    }
    
    fn format_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, value, mutable, .. } => {
                self.write("let ");
                if *mutable {
                    self.write("mut ");
                }
                self.write(name.as_str());
                if let Some(val) = value {
                    self.write(" = ");
                    self.format_expr(val);
                }
                self.maybe_semi();
            }
            
            Stmt::Function { name, params, body, is_async, .. } => {
                if *is_async {
                    self.write("async ");
                }
                self.write("fn ");
                self.write(name.as_str());
                self.write("(");
                self.format_params(params);
                self.write(") ");
                self.format_expr(body);
            }
            
            Stmt::Return { value, .. } => {
                self.write("return");
                if let Some(val) = value {
                    self.write(" ");
                    self.format_expr(val);
                }
                self.maybe_semi();
            }
            
            Stmt::If { condition, then_branch, else_branch, .. } => {
                self.write("if ");
                self.format_expr(&condition);
                self.write(" ");
                self.format_stmt(then_branch);
                if let Some(else_br) = else_branch {
                    self.write(" else ");
                    self.format_stmt(else_br);
                }
            }
            
            Stmt::While { condition, body, .. } => {
                self.write("while ");
                self.format_expr(&condition);
                self.write(" ");
                self.format_stmt(body);
            }
            
            Stmt::For { variable, iterable, body, .. } => {
                self.write("for ");
                self.write(variable.as_str());
                self.write(" in ");
                self.format_expr(iterable);
                self.write(" ");
                self.format_stmt(body);
            }
            
            Stmt::Break { .. } => {
                self.write("break");
                self.maybe_semi();
            }
            
            Stmt::Continue { .. } => {
                self.write("continue");
                self.maybe_semi();
            }
            
            Stmt::Class { name, methods, .. } => {
                self.write("class ");
                self.write(name.as_str());
                self.write(" {");
                self.indent_level += 1;
                
                for method in methods {
                    self.newline();
                    self.write_indent();
                    self.format_class_method(method);
                }
                
                self.indent_level -= 1;
                self.newline();
                self.write_indent();
                self.write("}");
            }
            
            Stmt::Expression { expr, .. } => {
                self.format_expr(expr);
                self.maybe_semi();
            }
            
            Stmt::Block { statements, .. } => {
                self.format_block(statements);
            }
            
            Stmt::Import { items, from, .. } => {
                self.write("import ");
                if !items.is_empty() {
                    self.write("{ ");
                    for (i, item) in items.iter().enumerate() {
                        self.write(item.name.as_str());
                        if let Some(alias) = &item.alias {
                            self.write(" as ");
                            self.write(alias.as_str());
                        }
                        if i < items.len() - 1 {
                            self.write(", ");
                        }
                    }
                    self.write(" } from ");
                }
                self.write("\"");
                self.write(from.as_str());
                self.write("\"");
                self.maybe_semi();
            }
            
            Stmt::Export { statement, .. } => {
                self.write("export ");
                self.format_stmt(statement);
            }
            
            Stmt::Impl { type_name, trait_name, methods, .. } => {
                self.write("impl ");
                if let Some(trait_n) = trait_name {
                    self.write(trait_n.as_str());
                    self.write(" for ");
                }
                self.write(type_name.as_str());
                self.write(" {");
                self.indent_level += 1;
                
                for method in methods {
                    self.newline();
                    self.write_indent();
                    self.format_stmt(method);
                }
                
                self.indent_level -= 1;
                self.newline();
                self.write_indent();
                self.write("}");
            }
        }
    }
    
    fn format_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal { value, .. } => self.format_literal(value),
            
            Expr::Identifier { name, .. } => {
                self.write(name.as_str());
            }
            
            Expr::Binary { left, operator, right, .. } => {
                self.format_expr(left);
                self.write(" ");
                self.write(operator.as_str());
                self.write(" ");
                self.format_expr(right);
            }
            
            Expr::Unary { operator, operand, .. } => {
                self.write(operator.as_str());
                self.format_expr(operand);
            }
            
            Expr::Logical { left, operator, right, .. } => {
                self.format_expr(left);
                self.write(" ");
                self.write(operator.as_str());
                self.write(" ");
                self.format_expr(right);
            }
            
            Expr::Call { callee, arguments, .. } => {
                self.format_expr(callee);
                self.write("(");
                self.format_args(arguments);
                self.write(")");
            }
            
            Expr::MethodCall { object, method, arguments, .. } => {
                self.format_expr(object);
                self.write(".");
                self.write(method.as_str());
                self.write("(");
                self.format_args(arguments);
                self.write(")");
            }
            
            Expr::Index { object, index, .. } => {
                self.format_expr(object);
                self.write("[");
                self.format_expr(index);
                self.write("]");
            }
            
            Expr::Property { object, property, .. } => {
                self.format_expr(object);
                self.write(".");
                self.write(property.as_str());
            }
            
            Expr::OptionalChain { object, property, .. } => {
                self.format_expr(object);
                self.write("?.");
                self.write(property.as_str());
            }
            
            Expr::Assign { target, value, .. } => {
                self.format_expr(target);
                self.write(" = ");
                self.format_expr(value);
            }
            
            Expr::CompoundAssign { target, operator, value, .. } => {
                self.format_expr(target);
                self.write(" ");
                self.write(operator.as_str());
                self.write("= ");
                self.format_expr(value);
            }
            
            Expr::Array { elements, .. } => {
                if elements.is_empty() {
                    self.write("[]");
                } else {
                    self.write("[");
                    if self.config.bracket_spacing {
                        self.write(" ");
                    }
                    self.format_args(elements);
                    if self.config.bracket_spacing {
                        self.write(" ");
                    }
                    self.write("]");
                }
            }
            
            Expr::Object { properties, .. } => {
                if properties.is_empty() {
                    self.write("{}");
                } else {
                    self.write("{");
                    if self.config.brace_spacing {
                        self.write(" ");
                    }
                    for (i, prop) in properties.iter().enumerate() {
                        self.write(prop.key.as_str());
                        self.write(": ");
                        self.format_expr(&prop.value);
                        if i < properties.len() - 1 {
                            self.write(", ");
                        }
                    }
                    if self.config.brace_spacing {
                        self.write(" ");
                    }
                    self.write("}");
                }
            }
            
            Expr::Lambda { params, body, .. } => {
                if params.len() == 1 {
                    self.write(params[0].name.as_str());
                } else {
                    self.write("(");
                    self.format_params(params);
                    self.write(")");
                }
                self.write(" => ");
                self.format_expr(body);
            }
            
            Expr::IfExpr { condition, then_branch, else_branch, .. } => {
                self.write("if ");
                self.format_expr(condition);
                self.write(" { ");
                self.format_expr(then_branch);
                self.write(" } else { ");
                self.format_expr(else_branch);
                self.write(" }");
            }
            
            Expr::Match { value, arms, .. } => {
                self.write("match ");
                self.format_expr(value);
                self.write(" {");
                self.indent_level += 1;
                
                for arm in arms {
                    self.newline();
                    self.write_indent();
                    self.format_pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.write(" if ");
                        self.format_expr(guard);
                    }
                    self.write(" => ");
                    self.format_expr(&arm.body);
                    self.write(",");
                }
                
                self.indent_level -= 1;
                self.newline();
                self.write_indent();
                self.write("}");
            }
            
            Expr::Block { statements, expression, .. } => {
                self.write("{");
                self.indent_level += 1;
                for stmt in statements {
                    self.newline();
                    self.write_indent();
                    self.format_stmt(stmt);
                }
                if let Some(expr) = expression {
                    self.newline();
                    self.write_indent();
                    self.format_expr(expr);
                }
                self.indent_level -= 1;
                self.newline();
                self.write_indent();
                self.write("}");
            }
            
            Expr::Await { expr, .. } => {
                self.write("await ");
                self.format_expr(expr);
            }
            
            Expr::Range { start, end, inclusive, .. } => {
                self.format_expr(start);
                if *inclusive {
                    self.write("..=");
                } else {
                    self.write("..");
                }
                self.format_expr(end);
            }
            
            Expr::Spread { expr, .. } => {
                self.write("...");
                self.format_expr(expr);
            }
            
            Expr::NullCoalesce { left, right, .. } => {
                self.format_expr(left);
                self.write(" ?? ");
                self.format_expr(right);
            }
            
            Expr::Grouping { expr, .. } => {
                self.write("(");
                self.format_expr(expr);
                self.write(")");
            }
        }
    }
    
    fn format_literal(&mut self, lit: &Literal) {
        match lit {
            Literal::Integer(n) => self.write(&n.to_string()),
            Literal::Float(f) => {
                let s = f.to_string();
                self.write(&s);
                if !s.contains('.') {
                    self.write(".0");
                }
            }
            Literal::String(s) => {
                let quote = if self.config.single_quote { "'" } else { "\"" };
                self.write(quote);
                self.write(&escape_string(s.as_str()));
                self.write(quote);
            }
            Literal::Bool(b) => self.write(if *b { "true" } else { "false" }),
            Literal::Null => self.write("null"),
        }
    }
    
    fn format_class_method(&mut self, _method: &crate::ast::ClassMethod) {
        // Class method formatting
        self.write("// method");
    }
    
    fn format_block(&mut self, stmts: &[Stmt]) {
        self.write("{");
        if stmts.is_empty() {
            self.write("}");
            return;
        }
        
        self.indent_level += 1;
        for stmt in stmts {
            self.newline();
            self.write_indent();
            self.format_stmt(stmt);
        }
        self.indent_level -= 1;
        self.newline();
        self.write_indent();
        self.write("}");
    }
    
    fn format_params(&mut self, params: &[Param]) {
        for (i, param) in params.iter().enumerate() {
            self.write(param.name.as_str());
            if i < params.len() - 1 {
                self.write(", ");
            }
        }
    }
    
    fn format_args(&mut self, args: &[Expr]) {
        for (i, arg) in args.iter().enumerate() {
            self.format_expr(arg);
            if i < args.len() - 1 {
                self.write(", ");
            }
        }
    }
    
    fn write(&mut self, s: &str) {
        self.output.push_str(s);
        self.current_line_width += s.len();
    }
    
    fn write_indent(&mut self) {
        let indent = if self.config.use_tabs {
            "\t".repeat(self.indent_level)
        } else {
            " ".repeat(self.indent_level * self.config.indent_width)
        };
        self.output.push_str(&indent);
        self.current_line_width = indent.len();
    }
    
    fn newline(&mut self) {
        self.output.push('\n');
        self.current_line_width = 0;
    }
    
    fn maybe_semi(&mut self) {
        if self.config.semicolons {
            self.write(";");
        }
    }
    
    fn format_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard { .. } => self.write("_"),
            Pattern::Literal { value, .. } => self.format_literal(value),
            Pattern::Binding { name, .. } => self.write(name.as_str()),
            Pattern::Range { start, end, inclusive, .. } => {
                self.format_literal(start);
                if *inclusive {
                    self.write("..=");
                } else {
                    self.write("..");
                }
                self.format_literal(end);
            }
            Pattern::Constructor { name, args, .. } => {
                self.write(name.as_str());
                if !args.is_empty() {
                    self.write("(");
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.format_pattern(arg);
                    }
                    self.write(")");
                }
            }
            Pattern::Array { elements, rest, .. } => {
                self.write("[");
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.format_pattern(elem);
                }
                if let Some(rest_name) = rest {
                    if !elements.is_empty() {
                        self.write(", ");
                    }
                    self.write("...");
                    self.write(rest_name.as_str());
                }
                self.write("]");
            }
            Pattern::Object { properties, rest, .. } => {
                self.write("{ ");
                for (i, (key, value)) in properties.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(key.as_str());
                    self.write(": ");
                    self.format_pattern(value);
                }
                if let Some(rest_name) = rest {
                    if !properties.is_empty() {
                        self.write(", ");
                    }
                    self.write("...");
                    self.write(rest_name.as_str());
                }
                self.write(" }");
            }
            Pattern::Or { patterns, .. } => {
                for (i, p) in patterns.iter().enumerate() {
                    if i > 0 {
                        self.write(" | ");
                    }
                    self.format_pattern(p);
                }
            }
        }
    }
}

/// Escape special characters in a string
fn escape_string(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\'' => result.push_str("\\'"),
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_config_default() {
        let config = FormatConfig::default();
        assert_eq!(config.indent_width, 2);
        assert!(!config.use_tabs);
        assert!(config.trailing_commas);
    }
    
    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_string("tab\there"), "tab\\there");
        assert_eq!(escape_string("quote\"here"), "quote\\\"here");
    }
}
