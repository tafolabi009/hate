//! Linter
//!
//! Static analysis for Hate code:
//! - Unused variables
//! - Unreachable code
//! - Shadowed variables
//! - Type mismatches
//! - Code style violations

use crate::ast::{Expr, Stmt, BinaryOp, Pattern};
use crate::intern::Symbol;
use crate::diagnostic::{Diagnostic, ErrorCode, Severity, SourceSpan};
use std::collections::HashMap;

/// Lint rule severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSeverity {
    Off,
    Warning,
    Error,
}

/// Lint rule identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintRule {
    // Variable rules
    UnusedVariable,
    UnusedParameter,
    ShadowedVariable,
    UndefinedVariable,
    
    // Code quality
    UnreachableCode,
    EmptyBlock,
    ConstantCondition,
    UnnecessaryElse,
    
    // Style
    NamingConvention,
    MaxLineLength,
    MaxFunctionLength,
    MaxComplexity,
    
    // Best practices
    NoVar,
    PreferConst,
    NoUnusedExpressions,
    NoImplicitAny,
    NoDoubleEquals,
}

impl LintRule {
    pub fn name(&self) -> &'static str {
        match self {
            LintRule::UnusedVariable => "unused-variable",
            LintRule::UnusedParameter => "unused-parameter",
            LintRule::ShadowedVariable => "shadowed-variable",
            LintRule::UndefinedVariable => "undefined-variable",
            LintRule::UnreachableCode => "unreachable-code",
            LintRule::EmptyBlock => "empty-block",
            LintRule::ConstantCondition => "constant-condition",
            LintRule::UnnecessaryElse => "unnecessary-else",
            LintRule::NamingConvention => "naming-convention",
            LintRule::MaxLineLength => "max-line-length",
            LintRule::MaxFunctionLength => "max-function-length",
            LintRule::MaxComplexity => "max-complexity",
            LintRule::NoVar => "no-var",
            LintRule::PreferConst => "prefer-const",
            LintRule::NoUnusedExpressions => "no-unused-expressions",
            LintRule::NoImplicitAny => "no-implicit-any",
            LintRule::NoDoubleEquals => "no-double-equals",
        }
    }
}

/// Linter configuration
#[derive(Debug, Clone)]
pub struct LintConfig {
    /// Rule severities
    pub rules: HashMap<LintRule, RuleSeverity>,
    /// Maximum line length
    pub max_line_length: usize,
    /// Maximum function length (lines)
    pub max_function_length: usize,
    /// Maximum cyclomatic complexity
    pub max_complexity: usize,
}

impl Default for LintConfig {
    fn default() -> Self {
        let mut rules = HashMap::new();
        
        // Default rule severities
        rules.insert(LintRule::UnusedVariable, RuleSeverity::Warning);
        rules.insert(LintRule::UnusedParameter, RuleSeverity::Warning);
        rules.insert(LintRule::ShadowedVariable, RuleSeverity::Warning);
        rules.insert(LintRule::UndefinedVariable, RuleSeverity::Error);
        rules.insert(LintRule::UnreachableCode, RuleSeverity::Warning);
        rules.insert(LintRule::EmptyBlock, RuleSeverity::Warning);
        rules.insert(LintRule::ConstantCondition, RuleSeverity::Warning);
        rules.insert(LintRule::UnnecessaryElse, RuleSeverity::Off);
        rules.insert(LintRule::NamingConvention, RuleSeverity::Warning);
        rules.insert(LintRule::MaxLineLength, RuleSeverity::Warning);
        rules.insert(LintRule::MaxFunctionLength, RuleSeverity::Warning);
        rules.insert(LintRule::MaxComplexity, RuleSeverity::Warning);
        rules.insert(LintRule::NoVar, RuleSeverity::Off);
        rules.insert(LintRule::PreferConst, RuleSeverity::Off);
        rules.insert(LintRule::NoUnusedExpressions, RuleSeverity::Warning);
        rules.insert(LintRule::NoImplicitAny, RuleSeverity::Off);
        rules.insert(LintRule::NoDoubleEquals, RuleSeverity::Warning);
        
        Self {
            rules,
            max_line_length: 100,
            max_function_length: 50,
            max_complexity: 10,
        }
    }
}

/// Variable usage tracking
#[derive(Debug, Clone)]
struct VariableInfo {
    /// Variable name
    name: Symbol,
    /// Declaration span
    span: SourceSpan,
    /// Is it used?
    used: bool,
    /// Is it a parameter?
    is_param: bool,
    /// Is it mutable (let mut)?
    is_mutable: bool,
    /// Was it reassigned?
    was_reassigned: bool,
}

/// Scope for variable tracking
#[derive(Debug)]
struct Scope {
    variables: HashMap<String, VariableInfo>,
    parent: Option<usize>,
}

impl Scope {
    fn new(parent: Option<usize>) -> Self {
        Self {
            variables: HashMap::new(),
            parent,
        }
    }
}

/// Linter state
pub struct Linter {
    config: LintConfig,
    diagnostics: Vec<Diagnostic>,
    scopes: Vec<Scope>,
    current_scope: usize,
    in_function: bool,
    current_complexity: usize,
}

impl Linter {
    pub fn new(config: LintConfig) -> Self {
        Self {
            config,
            diagnostics: Vec::new(),
            scopes: vec![Scope::new(None)],
            current_scope: 0,
            in_function: false,
            current_complexity: 0,
        }
    }
    
    /// Lint a list of statements
    pub fn lint(&mut self, stmts: &[Stmt]) -> Vec<Diagnostic> {
        self.diagnostics.clear();
        self.scopes = vec![Scope::new(None)];
        self.current_scope = 0;
        
        for stmt in stmts {
            self.lint_stmt(stmt);
        }
        
        // Check for unused variables in global scope
        self.check_unused_variables();
        
        self.diagnostics.clone()
    }
    
    fn lint_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, value, mutable, span, .. } => {
                let info = VariableInfo {
                    name: *name,
                    span: SourceSpan::point(0, span.line as u32, span.column as u32),
                    used: false,
                    is_param: false,
                    is_mutable: *mutable,
                    was_reassigned: false,
                };
                
                // Check for shadowing
                if self.lookup_variable(name.as_str()).is_some() {
                    if self.rule_enabled(LintRule::ShadowedVariable) {
                        self.add_diagnostic(
                            LintRule::ShadowedVariable,
                            format!("variable `{}` shadows a variable in outer scope", name.as_str()),
                            info.span,
                        );
                    }
                }
                
                self.declare_variable(name.as_str().to_string(), info);
                
                if let Some(val) = value {
                    self.lint_expr(val);
                }
            }
            
            Stmt::Function { name, params, body, span, .. } => {
                let was_in_function = self.in_function;
                let old_complexity = self.current_complexity;
                
                self.in_function = true;
                self.current_complexity = 1; // Base complexity
                
                self.push_scope();
                
                // Declare parameters
                for param in params {
                    let info = VariableInfo {
                        name: param.name,
                        span: SourceSpan::point(0, param.span.line as u32, param.span.column as u32),
                        used: false,
                        is_param: true,
                        is_mutable: true,
                        was_reassigned: false,
                    };
                    self.declare_variable(param.name.as_str().to_string(), info);
                }
                
                // Lint body
                self.lint_expr(body);
                
                // Check for unused parameters
                self.check_unused_variables();
                
                // Check function complexity
                if self.rule_enabled(LintRule::MaxComplexity) 
                    && self.current_complexity > self.config.max_complexity 
                {
                    self.add_diagnostic(
                        LintRule::MaxComplexity,
                        format!(
                            "function `{}` has cyclomatic complexity of {} (max {})",
                            name.as_str(),
                            self.current_complexity,
                            self.config.max_complexity
                        ),
                        SourceSpan::point(0, span.line as u32, span.column as u32),
                    );
                }
                
                self.pop_scope();
                self.in_function = was_in_function;
                self.current_complexity = old_complexity;
            }
            
            Stmt::If { condition, then_branch, else_branch, .. } => {
                self.lint_expr(condition);
                
                // Check for constant condition
                if self.is_constant_bool(condition) && self.rule_enabled(LintRule::ConstantCondition) {
                    self.add_diagnostic(
                        LintRule::ConstantCondition,
                        "condition is always true or false".to_string(),
                        SourceSpan::point(0, 0, 0),
                    );
                }
                
                self.current_complexity += 1;
                
                self.push_scope();
                self.lint_stmt(then_branch);
                self.check_unused_variables();
                self.pop_scope();
                
                if let Some(else_br) = else_branch {
                    self.push_scope();
                    self.lint_stmt(else_br);
                    self.check_unused_variables();
                    self.pop_scope();
                }
            }
            
            Stmt::While { condition, body, .. } => {
                self.lint_expr(condition);
                self.current_complexity += 1;
                
                self.push_scope();
                self.lint_stmt(body);
                self.check_unused_variables();
                self.pop_scope();
            }
            
            Stmt::For { variable, iterable, body, span, .. } => {
                self.lint_expr(iterable);
                self.current_complexity += 1;
                
                self.push_scope();
                
                let info = VariableInfo {
                    name: *variable,
                    span: SourceSpan::point(0, span.line as u32, span.column as u32),
                    used: false,
                    is_param: false,
                    is_mutable: true,
                    was_reassigned: false,
                };
                self.declare_variable(variable.as_str().to_string(), info);
                
                self.lint_stmt(body);
                
                self.check_unused_variables();
                self.pop_scope();
            }
            
            Stmt::Return { value, .. } => {
                if let Some(val) = value {
                    self.lint_expr(val);
                }
            }
            
            Stmt::Expression { expr, .. } => {
                self.lint_expr(expr);
                
                // Check for unused expression
                if self.rule_enabled(LintRule::NoUnusedExpressions) && !self.is_side_effect_expr(expr) {
                    self.add_diagnostic(
                        LintRule::NoUnusedExpressions,
                        "expression result is unused".to_string(),
                        SourceSpan::point(0, 0, 0),
                    );
                }
            }
            
            Stmt::Block { statements, .. } => {
                if statements.is_empty() && self.rule_enabled(LintRule::EmptyBlock) {
                    self.add_diagnostic(
                        LintRule::EmptyBlock,
                        "empty block".to_string(),
                        SourceSpan::point(0, 0, 0),
                    );
                }
                
                self.push_scope();
                for s in statements {
                    self.lint_stmt(s);
                }
                self.check_unused_variables();
                self.pop_scope();
            }
            
            _ => {}
        }
    }
    
    fn lint_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Identifier { name, .. } => {
                self.mark_variable_used(name.as_str());
                
                // Check if undefined
                if self.lookup_variable(name.as_str()).is_none() 
                    && self.rule_enabled(LintRule::UndefinedVariable) 
                {
                    self.add_diagnostic(
                        LintRule::UndefinedVariable,
                        format!("undefined variable `{}`", name.as_str()),
                        SourceSpan::point(0, 0, 0),
                    );
                }
            }
            
            Expr::Binary { left, operator, right, .. } => {
                self.lint_expr(left);
                self.lint_expr(right);
                
                // Check for == vs ===
                if *operator == BinaryOp::Eq && self.rule_enabled(LintRule::NoDoubleEquals) {
                    self.add_diagnostic(
                        LintRule::NoDoubleEquals,
                        "use `===` instead of `==`".to_string(),
                        SourceSpan::point(0, 0, 0),
                    );
                }
            }
            
            Expr::Unary { operand, .. } => {
                self.lint_expr(operand);
            }
            
            Expr::Logical { left, right, .. } => {
                self.lint_expr(left);
                self.lint_expr(right);
            }
            
            Expr::Call { callee, arguments, .. } => {
                self.lint_expr(callee);
                for arg in arguments {
                    self.lint_expr(arg);
                }
            }
            
            Expr::MethodCall { object, arguments, .. } => {
                self.lint_expr(object);
                for arg in arguments {
                    self.lint_expr(arg);
                }
            }
            
            Expr::Index { object, index, .. } => {
                self.lint_expr(object);
                self.lint_expr(index);
            }
            
            Expr::Property { object, .. } => {
                self.lint_expr(object);
            }
            
            Expr::OptionalChain { object, .. } => {
                self.lint_expr(object);
            }
            
            Expr::Assign { target, value, .. } => {
                self.lint_expr(value);
                
                if let Expr::Identifier { name, .. } = target.as_ref() {
                    self.mark_variable_reassigned(name.as_str());
                }
                
                self.lint_expr(target);
            }
            
            Expr::CompoundAssign { target, value, .. } => {
                self.lint_expr(value);
                
                if let Expr::Identifier { name, .. } = target.as_ref() {
                    self.mark_variable_reassigned(name.as_str());
                }
                
                self.lint_expr(target);
            }
            
            Expr::Array { elements, .. } => {
                for elem in elements {
                    self.lint_expr(elem);
                }
            }
            
            Expr::Object { properties, .. } => {
                for prop in properties {
                    self.lint_expr(&prop.value);
                }
            }
            
            Expr::Lambda { params, body, .. } => {
                self.push_scope();
                
                for param in params {
                    let info = VariableInfo {
                        name: param.name,
                        span: SourceSpan::point(0, 0, 0),
                        used: false,
                        is_param: true,
                        is_mutable: true,
                        was_reassigned: false,
                    };
                    self.declare_variable(param.name.as_str().to_string(), info);
                }
                
                self.lint_expr(body);
                
                self.check_unused_variables();
                self.pop_scope();
            }
            
            Expr::IfExpr { condition, then_branch, else_branch, .. } => {
                self.lint_expr(condition);
                self.lint_expr(then_branch);
                self.lint_expr(else_branch);
                self.current_complexity += 1;
            }
            
            Expr::Match { value, arms, .. } => {
                self.lint_expr(value);
                
                // Each arm adds to complexity
                self.current_complexity += arms.len().saturating_sub(1);
                
                for arm in arms {
                    self.push_scope();
                    
                    // Extract bindings from pattern
                    self.extract_pattern_bindings(&arm.pattern);
                    
                    if let Some(guard) = &arm.guard {
                        self.lint_expr(guard);
                    }
                    
                    self.lint_expr(&arm.body);
                    
                    self.check_unused_variables();
                    self.pop_scope();
                }
            }
            
            Expr::Block { statements, expression, .. } => {
                self.push_scope();
                for s in statements {
                    self.lint_stmt(s);
                }
                if let Some(expr) = expression {
                    self.lint_expr(expr);
                }
                self.check_unused_variables();
                self.pop_scope();
            }
            
            Expr::Await { expr, .. } => {
                self.lint_expr(expr);
            }
            
            Expr::Range { start, end, .. } => {
                self.lint_expr(start);
                self.lint_expr(end);
            }
            
            Expr::Spread { expr, .. } => {
                self.lint_expr(expr);
            }
            
            Expr::NullCoalesce { left, right, .. } => {
                self.lint_expr(left);
                self.lint_expr(right);
            }
            
            Expr::Grouping { expr, .. } => {
                self.lint_expr(expr);
            }
            
            _ => {}
        }
    }
    
    fn push_scope(&mut self) {
        let new_scope = Scope::new(Some(self.current_scope));
        self.scopes.push(new_scope);
        self.current_scope = self.scopes.len() - 1;
    }
    
    fn pop_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
            self.current_scope = parent;
        }
    }
    
    fn declare_variable(&mut self, name: String, info: VariableInfo) {
        self.scopes[self.current_scope].variables.insert(name, info);
    }
    
    fn lookup_variable(&self, name: &str) -> Option<&VariableInfo> {
        let mut scope_idx = Some(self.current_scope);
        
        while let Some(idx) = scope_idx {
            if let Some(info) = self.scopes[idx].variables.get(name) {
                return Some(info);
            }
            scope_idx = self.scopes[idx].parent;
        }
        
        None
    }
    
    fn mark_variable_used(&mut self, name: &str) {
        let mut scope_idx = Some(self.current_scope);
        
        while let Some(idx) = scope_idx {
            if let Some(info) = self.scopes[idx].variables.get_mut(name) {
                info.used = true;
                return;
            }
            scope_idx = self.scopes[idx].parent;
        }
    }
    
    fn mark_variable_reassigned(&mut self, name: &str) {
        let mut scope_idx = Some(self.current_scope);
        
        while let Some(idx) = scope_idx {
            if let Some(info) = self.scopes[idx].variables.get_mut(name) {
                info.was_reassigned = true;
                return;
            }
            scope_idx = self.scopes[idx].parent;
        }
    }
    
    fn check_unused_variables(&mut self) {
        // Collect diagnostic info first to avoid borrow issues
        let mut diagnostics_to_add: Vec<(LintRule, String, SourceSpan)> = Vec::new();
        
        {
            let scope = &self.scopes[self.current_scope];
            
            for (name, info) in &scope.variables {
                if !info.used && !name.starts_with('_') {
                    let rule = if info.is_param {
                        LintRule::UnusedParameter
                    } else {
                        LintRule::UnusedVariable
                    };
                    
                    if self.rule_enabled(rule) {
                        diagnostics_to_add.push((
                            rule,
                            format!(
                                "unused {} `{}`",
                                if info.is_param { "parameter" } else { "variable" },
                                name
                            ),
                            info.span,
                        ));
                    }
                }
                
                // Check for prefer-const
                if info.is_mutable 
                    && !info.is_param 
                    && !info.was_reassigned 
                    && self.rule_enabled(LintRule::PreferConst)
                {
                    diagnostics_to_add.push((
                        LintRule::PreferConst,
                        format!(
                            "`{}` is never reassigned. Use `let` without `mut`",
                            name
                        ),
                        info.span,
                    ));
                }
            }
        }
        
        // Now add all diagnostics
        for (rule, message, span) in diagnostics_to_add {
            self.add_diagnostic(rule, message, span);
        }
    }
    
    fn is_constant_bool(&self, expr: &Expr) -> bool {
        matches!(expr, Expr::Literal { value: crate::ast::Literal::Bool(_), .. })
    }
    
    fn is_side_effect_expr(&self, expr: &Expr) -> bool {
        matches!(
            expr,
            Expr::Call { .. } | Expr::MethodCall { .. } | Expr::Assign { .. } | 
            Expr::CompoundAssign { .. } | Expr::Await { .. }
        )
    }
    
    fn extract_pattern_bindings(&mut self, pattern: &Pattern) {
        // Extract variable bindings from pattern and declare them
        match pattern {
            Pattern::Binding { name, span } => {
                let info = VariableInfo {
                    name: *name,
                    span: SourceSpan::point(0, span.line as u32, span.column as u32),
                    used: false,
                    is_param: false,
                    is_mutable: true,
                    was_reassigned: false,
                };
                self.declare_variable(name.as_str().to_string(), info);
            }
            Pattern::Constructor { args, .. } => {
                for arg in args {
                    self.extract_pattern_bindings(arg);
                }
            }
            Pattern::Array { elements, rest, span } => {
                for elem in elements {
                    self.extract_pattern_bindings(elem);
                }
                if let Some(rest_name) = rest {
                    let info = VariableInfo {
                        name: *rest_name,
                        span: SourceSpan::point(0, span.line as u32, span.column as u32),
                        used: false,
                        is_param: false,
                        is_mutable: true,
                        was_reassigned: false,
                    };
                    self.declare_variable(rest_name.as_str().to_string(), info);
                }
            }
            Pattern::Object { properties, rest, span } => {
                for (_, pattern) in properties {
                    self.extract_pattern_bindings(pattern);
                }
                if let Some(rest_name) = rest {
                    let info = VariableInfo {
                        name: *rest_name,
                        span: SourceSpan::point(0, span.line as u32, span.column as u32),
                        used: false,
                        is_param: false,
                        is_mutable: true,
                        was_reassigned: false,
                    };
                    self.declare_variable(rest_name.as_str().to_string(), info);
                }
            }
            Pattern::Or { patterns, .. } => {
                // Only need to declare from first pattern (all should have same bindings)
                if let Some(first) = patterns.first() {
                    self.extract_pattern_bindings(first);
                }
            }
            _ => {} // Wildcard, Literal, Range don't introduce bindings
        }
    }
    
    fn rule_enabled(&self, rule: LintRule) -> bool {
        self.config.rules.get(&rule).copied() != Some(RuleSeverity::Off)
    }
    
    fn add_diagnostic(&mut self, rule: LintRule, message: String, span: SourceSpan) {
        let severity = self.config.rules.get(&rule).copied().unwrap_or(RuleSeverity::Warning);
        
        let diag_severity = match severity {
            RuleSeverity::Error => Severity::Error,
            RuleSeverity::Warning => Severity::Warning,
            RuleSeverity::Off => return,
        };
        
        self.diagnostics.push(Diagnostic {
            severity: diag_severity,
            code: ErrorCode::InternalError, // Use appropriate code
            message: format!("[{}] {}", rule.name(), message),
            span: Some(span),
            file: None,
            source: None,
            labels: Vec::new(),
            help: Vec::new(),
            notes: Vec::new(),
        });
    }
}

impl Default for Linter {
    fn default() -> Self {
        Self::new(LintConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lint_config_default() {
        let config = LintConfig::default();
        assert_eq!(
            config.rules.get(&LintRule::UnusedVariable),
            Some(&RuleSeverity::Warning)
        );
    }
    
    #[test]
    fn test_rule_names() {
        assert_eq!(LintRule::UnusedVariable.name(), "unused-variable");
        assert_eq!(LintRule::ShadowedVariable.name(), "shadowed-variable");
    }
}
