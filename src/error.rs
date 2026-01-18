//! Error types and result handling for the Hate language
//!
//! Provides rich error messages with source locations and suggestions.

use std::fmt;
use thiserror::Error;

/// Source location information for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Start byte offset in source
    pub start: usize,
    /// End byte offset in source
    pub end: usize,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
}

impl Span {
    /// Create a new span
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self { start, end, line, column }
    }
    
    /// Create an empty span at position 0
    pub fn empty() -> Self {
        Self { start: 0, end: 0, line: 1, column: 1 }
    }
    
    /// Merge two spans into one that covers both
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line: self.line.min(other.line),
            column: if self.line <= other.line { self.column } else { other.column },
        }
    }
}

impl Default for Span {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// All possible errors in the Hate language
#[derive(Error, Debug)]
pub enum HateError {
    // ==================== Lexer Errors ====================
    #[error("Unexpected character '{ch}' at {span}")]
    UnexpectedChar { ch: char, span: Span },
    
    #[error("Unterminated string at {span}")]
    UnterminatedString { span: Span },
    
    #[error("Invalid number format at {span}")]
    InvalidNumber { span: Span },
    
    #[error("Invalid escape sequence '\\{ch}' at {span}")]
    InvalidEscape { ch: char, span: Span },
    
    // ==================== Parser Errors ====================
    #[error("Expected {expected}, found {found} at {span}")]
    ExpectedToken { expected: String, found: String, span: Span },
    
    #[error("Unexpected token '{token}' at {span}")]
    UnexpectedToken { token: String, span: Span },
    
    #[error("Expected expression at {span}")]
    ExpectedExpression { span: Span },
    
    #[error("Invalid assignment target at {span}")]
    InvalidAssignment { span: Span },
    
    #[error("Too many parameters (max 255) at {span}")]
    TooManyParams { span: Span },
    
    #[error("Too many arguments (max 255) at {span}")]
    TooManyArgs { span: Span },
    
    // ==================== Type Errors ====================
    #[error("Type mismatch: expected {expected}, found {found} at {span}")]
    TypeMismatch { expected: String, found: String, span: Span },
    
    #[error("Cannot apply operator '{op}' to types {left} and {right} at {span}")]
    InvalidOperator { op: String, left: String, right: String, span: Span },
    
    #[error("Undefined variable '{name}' at {span}")]
    UndefinedVariable { name: String, span: Span },
    
    #[error("Cannot reassign immutable variable '{name}' at {span}")]
    ImmutableReassign { name: String, span: Span },
    
    // ==================== Runtime Errors ====================
    #[error("Division by zero at {span}")]
    DivisionByZero { span: Span },
    
    #[error("Stack overflow")]
    StackOverflow,
    
    #[error("Index {index} out of bounds (length {length})")]
    IndexOutOfBounds { index: i64, length: usize },
    
    #[error("Property '{name}' not found on object")]
    PropertyNotFound { name: String },
    
    #[error("Value is not callable")]
    NotCallable,
    
    #[error("Wrong number of arguments: expected {expected}, got {got}")]
    WrongArity { expected: usize, got: usize },
    
    // ==================== Compiler Errors ====================
    #[error("Too many constants in chunk (max 65536)")]
    TooManyConstants,
    
    #[error("Too many local variables (max 256)")]
    TooManyLocals,
    
    #[error("Too many upvalues (max 256)")]
    TooManyUpvalues,
    
    #[error("Jump offset too large")]
    JumpTooLarge,
    
    #[error("Break outside of loop at {span}")]
    BreakOutsideLoop { span: Span },
    
    #[error("Continue outside of loop at {span}")]
    ContinueOutsideLoop { span: Span },
    
    #[error("Return outside of function at {span}")]
    ReturnOutsideFunction { span: Span },
    
    // ==================== IO Errors ====================
    #[error("IO error: {0}")]
    IO(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    // ==================== Internal Errors ====================
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for Hate operations
pub type HateResult<T> = Result<T, HateError>;

/// A diagnostic message with source context
pub struct Diagnostic {
    pub error: HateError,
    pub source: String,
    pub filename: Option<String>,
    pub help: Option<String>,
    pub note: Option<String>,
}

impl Diagnostic {
    /// Create a new diagnostic
    pub fn new(error: HateError, source: &str) -> Self {
        Self {
            error,
            source: source.to_string(),
            filename: None,
            help: None,
            note: None,
        }
    }
    
    /// Add a filename to the diagnostic
    pub fn with_filename(mut self, filename: &str) -> Self {
        self.filename = Some(filename.to_string());
        self
    }
    
    /// Add a help message
    pub fn with_help(mut self, help: &str) -> Self {
        self.help = Some(help.to_string());
        self
    }
    
    /// Add a note
    pub fn with_note(mut self, note: &str) -> Self {
        self.note = Some(note.to_string());
        self
    }
    
    /// Format the diagnostic with source context
    pub fn format(&self) -> String {
        let mut output = String::new();
        
        // Error header
        output.push_str(&format!("\x1b[1;31mError\x1b[0m: {}\n", self.error));
        
        // Get span from error if available
        let span = self.get_span();
        
        if let Some(span) = span {
            let filename = self.filename.as_deref().unwrap_or("<input>");
            output.push_str(&format!("  \x1b[1;34m┌─\x1b[0m {}:{}:{}\n", 
                filename, span.line, span.column));
            
            // Get the relevant source line
            if let Some(line_str) = self.source.lines().nth(span.line - 1) {
                let line_num = format!("{}", span.line);
                let padding = " ".repeat(line_num.len());
                
                output.push_str(&format!("  \x1b[1;34m{} │\x1b[0m\n", padding));
                output.push_str(&format!("  \x1b[1;34m{} │\x1b[0m {}\n", line_num, line_str));
                
                // Underline the error
                let underline_start = span.column.saturating_sub(1);
                let underline_len = (span.end - span.start).max(1);
                let underline = format!(
                    "{}{}",
                    " ".repeat(underline_start),
                    "\x1b[1;31m^\x1b[0m".repeat(underline_len.min(line_str.len() - underline_start))
                );
                output.push_str(&format!("  \x1b[1;34m{} │\x1b[0m {}\n", padding, underline));
            }
        }
        
        // Help message
        if let Some(help) = &self.help {
            output.push_str(&format!("  \x1b[1;32m= help:\x1b[0m {}\n", help));
        }
        
        // Note
        if let Some(note) = &self.note {
            output.push_str(&format!("  \x1b[1;36m= note:\x1b[0m {}\n", note));
        }
        
        output
    }
    
    /// Extract span from error
    fn get_span(&self) -> Option<Span> {
        match &self.error {
            HateError::UnexpectedChar { span, .. } => Some(*span),
            HateError::UnterminatedString { span } => Some(*span),
            HateError::InvalidNumber { span } => Some(*span),
            HateError::InvalidEscape { span, .. } => Some(*span),
            HateError::ExpectedToken { span, .. } => Some(*span),
            HateError::UnexpectedToken { span, .. } => Some(*span),
            HateError::ExpectedExpression { span } => Some(*span),
            HateError::InvalidAssignment { span } => Some(*span),
            HateError::TooManyParams { span } => Some(*span),
            HateError::TooManyArgs { span } => Some(*span),
            HateError::TypeMismatch { span, .. } => Some(*span),
            HateError::InvalidOperator { span, .. } => Some(*span),
            HateError::UndefinedVariable { span, .. } => Some(*span),
            HateError::ImmutableReassign { span, .. } => Some(*span),
            HateError::DivisionByZero { span } => Some(*span),
            HateError::BreakOutsideLoop { span } => Some(*span),
            HateError::ContinueOutsideLoop { span } => Some(*span),
            HateError::ReturnOutsideFunction { span } => Some(*span),
            _ => None,
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_span_merge() {
        let s1 = Span::new(0, 5, 1, 1);
        let s2 = Span::new(10, 15, 1, 11);
        let merged = s1.merge(s2);
        
        assert_eq!(merged.start, 0);
        assert_eq!(merged.end, 15);
    }
    
    #[test]
    fn test_diagnostic_format() {
        let error = HateError::UnexpectedChar { 
            ch: '@', 
            span: Span::new(5, 6, 1, 6) 
        };
        let diag = Diagnostic::new(error, "let x @= 10;")
            .with_filename("test.hate")
            .with_help("Remove the '@' character");
        
        let output = diag.format();
        assert!(output.contains("Error"));
        assert!(output.contains("test.hate"));
    }
}
