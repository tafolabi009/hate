//! Enhanced Error Messages
//!
//! Provides rich, helpful error messages with:
//! - Source code context with line highlighting
//! - Colored output for terminals
//! - Suggestions for common mistakes
//! - Error codes for documentation
//! - Stack traces with function names


/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Hint,
}

impl Severity {
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Hint => "hint",
        }
    }
    
    pub fn color_code(&self) -> &'static str {
        match self {
            Severity::Error => "\x1b[31m",   // Red
            Severity::Warning => "\x1b[33m", // Yellow
            Severity::Hint => "\x1b[36m",    // Cyan
        }
    }
}

/// Source location
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    /// Start byte offset
    pub start: usize,
    /// End byte offset (exclusive)
    pub end: usize,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub column: u32,
}

impl SourceSpan {
    pub fn new(start: usize, end: usize, line: u32, column: u32) -> Self {
        Self { start, end, line, column }
    }
    
    pub fn point(offset: usize, line: u32, column: u32) -> Self {
        Self {
            start: offset,
            end: offset + 1,
            line,
            column,
        }
    }
}

/// Error code categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ErrorCode {
    // Syntax errors (1xxx)
    UnexpectedToken = 1001,
    UnterminatedString = 1002,
    UnterminatedComment = 1003,
    InvalidNumber = 1004,
    InvalidEscape = 1005,
    MissingCloseParen = 1006,
    MissingCloseBrace = 1007,
    MissingCloseBracket = 1008,
    ExpectedExpression = 1009,
    ExpectedIdentifier = 1010,
    
    // Type errors (2xxx)
    TypeMismatch = 2001,
    UndefinedVariable = 2002,
    UndefinedProperty = 2003,
    UndefinedMethod = 2004,
    NotCallable = 2005,
    NotIterable = 2006,
    InvalidOperands = 2007,
    NullReference = 2008,
    
    // Reference errors (3xxx)
    NotDefined = 3001,
    NotInitialized = 3002,
    AlreadyDeclared = 3003,
    ConstReassignment = 3004,
    
    // Runtime errors (4xxx)
    StackOverflow = 4001,
    OutOfMemory = 4002,
    DivisionByZero = 4003,
    IndexOutOfBounds = 4004,
    InvalidArguments = 4005,
    AssertionFailed = 4006,
    
    // Module errors (5xxx)
    ModuleNotFound = 5001,
    CircularDependency = 5002,
    ExportNotFound = 5003,
    
    // Internal errors (9xxx)
    InternalError = 9001,
}

impl ErrorCode {
    pub fn as_u16(self) -> u16 {
        self as u16
    }
    
    pub fn to_string_code(self) -> String {
        format!("E{:04}", self.as_u16())
    }
}

/// A diagnostic message
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity
    pub severity: Severity,
    /// Error code
    pub code: ErrorCode,
    /// Main message
    pub message: String,
    /// Primary source span
    pub span: Option<SourceSpan>,
    /// Source file path
    pub file: Option<String>,
    /// Source code (for context)
    pub source: Option<String>,
    /// Secondary labels
    pub labels: Vec<(SourceSpan, String)>,
    /// Help suggestions
    pub help: Vec<String>,
    /// Notes
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message: message.into(),
            span: None,
            file: None,
            source: None,
            labels: Vec::new(),
            help: Vec::new(),
            notes: Vec::new(),
        }
    }
    
    pub fn warning(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code,
            message: message.into(),
            span: None,
            file: None,
            source: None,
            labels: Vec::new(),
            help: Vec::new(),
            notes: Vec::new(),
        }
    }
    
    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }
    
    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }
    
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
    
    pub fn with_label(mut self, span: SourceSpan, label: impl Into<String>) -> Self {
        self.labels.push((span, label.into()));
        self
    }
    
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help.push(help.into());
        self
    }
    
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

/// Stack frame for error traces
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Function name
    pub function: String,
    /// File path
    pub file: Option<String>,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
}

impl StackFrame {
    pub fn new(function: impl Into<String>, line: u32, column: u32) -> Self {
        Self {
            function: function.into(),
            file: None,
            line,
            column,
        }
    }
    
    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }
}

/// Error reporter with rich output
pub struct ErrorReporter {
    /// Use colored output
    use_colors: bool,
    /// Terminal width for wrapping
    term_width: usize,
}

impl ErrorReporter {
    pub fn new() -> Self {
        Self {
            use_colors: true,
            term_width: 80,
        }
    }
    
    pub fn with_colors(mut self, use_colors: bool) -> Self {
        self.use_colors = use_colors;
        self
    }
    
    /// Format a diagnostic for display
    pub fn format(&self, diag: &Diagnostic) -> String {
        let mut output = String::new();
        
        // Header: error[E1001]: message
        let reset = if self.use_colors { "\x1b[0m" } else { "" };
        let bold = if self.use_colors { "\x1b[1m" } else { "" };
        let color = if self.use_colors { diag.severity.color_code() } else { "" };
        
        output.push_str(&format!(
            "{}{}{}[{}]{}: {}{}\n",
            bold,
            color,
            diag.severity.label(),
            diag.code.to_string_code(),
            reset,
            bold,
            diag.message,
        ));
        output.push_str(reset);
        
        // Location: --> file.hate:10:5
        if let Some(span) = &diag.span {
            let file = diag.file.as_deref().unwrap_or("<input>");
            let blue = if self.use_colors { "\x1b[34m" } else { "" };
            output.push_str(&format!(
                "  {}-->{} {}:{}:{}\n",
                blue, reset, file, span.line, span.column
            ));
        }
        
        // Source context
        if let (Some(source), Some(span)) = (&diag.source, &diag.span) {
            output.push_str(&self.format_source_context(source, span, &diag.labels));
        }
        
        // Help suggestions
        if !diag.help.is_empty() {
            let green = if self.use_colors { "\x1b[32m" } else { "" };
            for help in &diag.help {
                output.push_str(&format!("  {}help{}: {}\n", green, reset, help));
            }
        }
        
        // Notes
        if !diag.notes.is_empty() {
            let cyan = if self.use_colors { "\x1b[36m" } else { "" };
            for note in &diag.notes {
                output.push_str(&format!("  {}note{}: {}\n", cyan, reset, note));
            }
        }
        
        output
    }
    
    fn format_source_context(
        &self,
        source: &str,
        span: &SourceSpan,
        labels: &[(SourceSpan, String)],
    ) -> String {
        let mut output = String::new();
        
        let lines: Vec<&str> = source.lines().collect();
        let line_idx = span.line.saturating_sub(1) as usize;
        
        // Calculate gutter width
        let max_line = span.line + 2;
        let gutter_width = max_line.to_string().len();
        
        let blue = if self.use_colors { "\x1b[34m" } else { "" };
        let red = if self.use_colors { "\x1b[31m" } else { "" };
        let reset = if self.use_colors { "\x1b[0m" } else { "" };
        
        // Empty line before
        output.push_str(&format!("  {:width$} {}|{}\n", "", blue, reset, width = gutter_width));
        
        // Context lines before
        if line_idx > 0 {
            let prev_line = lines.get(line_idx - 1).unwrap_or(&"");
            output.push_str(&format!(
                "  {}{:width$}{} {}|{} {}\n",
                blue, span.line - 1, reset, blue, reset, prev_line,
                width = gutter_width
            ));
        }
        
        // Error line
        if let Some(error_line) = lines.get(line_idx) {
            output.push_str(&format!(
                "  {}{:width$}{} {}|{} {}\n",
                blue, span.line, reset, blue, reset, error_line,
                width = gutter_width
            ));
            
            // Underline
            let underline_start = span.column.saturating_sub(1) as usize;
            let underline_len = (span.end - span.start).max(1);
            
            output.push_str(&format!(
                "  {:width$} {}|{} {}{}^{}{}\n",
                "", blue, reset,
                " ".repeat(underline_start),
                red,
                "~".repeat(underline_len.saturating_sub(1)),
                reset,
                width = gutter_width
            ));
        }
        
        // Empty line after
        output.push_str(&format!("  {:width$} {}|{}\n", "", blue, reset, width = gutter_width));
        
        output
    }
    
    /// Format a stack trace
    pub fn format_stack_trace(&self, frames: &[StackFrame]) -> String {
        let mut output = String::new();
        
        let gray = if self.use_colors { "\x1b[90m" } else { "" };
        let reset = if self.use_colors { "\x1b[0m" } else { "" };
        let bold = if self.use_colors { "\x1b[1m" } else { "" };
        
        output.push_str(&format!("{}Stack trace:{}\n", bold, reset));
        
        for (i, frame) in frames.iter().enumerate() {
            let file = frame.file.as_deref().unwrap_or("<unknown>");
            output.push_str(&format!(
                "  {}{}. {}{} at {}{}:{}:{}{}\n",
                gray, i, reset,
                frame.function,
                gray, file, frame.line, frame.column, reset
            ));
        }
        
        output
    }
}

impl Default for ErrorReporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Common error suggestions
pub struct ErrorSuggestions;

impl ErrorSuggestions {
    /// Suggest corrections for undefined variables
    pub fn suggest_variable(name: &str, available: &[&str]) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Find similar names (Levenshtein distance <= 2)
        for &candidate in available {
            let dist = Self::levenshtein(name, candidate);
            if dist <= 2 && dist > 0 {
                suggestions.push(format!("did you mean `{}`?", candidate));
            }
        }
        
        suggestions
    }
    
    /// Suggest for common typos
    pub fn suggest_typo(found: &str) -> Option<String> {
        let typos: &[(&str, &str)] = &[
            ("fucntion", "function"),
            ("funcion", "function"),
            ("funciton", "function"),
            ("retrun", "return"),
            ("reutrn", "return"),
            ("ture", "true"),
            ("flase", "false"),
            ("nul", "null"),
            ("cosnt", "const"),
            ("lte", "let"),
            ("whlie", "while"),
            ("esle", "else"),
            ("elseif", "else if"),
            ("pritn", "print"),
            ("pirnt", "print"),
        ];
        
        for &(typo, correct) in typos {
            if found == typo {
                return Some(format!("did you mean `{}`?", correct));
            }
        }
        
        None
    }
    
    /// Simple Levenshtein distance
    fn levenshtein(a: &str, b: &str) -> usize {
        let a: Vec<char> = a.chars().collect();
        let b: Vec<char> = b.chars().collect();
        
        let m = a.len();
        let n = b.len();
        
        if m == 0 { return n; }
        if n == 0 { return m; }
        
        let mut matrix = vec![vec![0; n + 1]; m + 1];
        
        for i in 0..=m {
            matrix[i][0] = i;
        }
        for j in 0..=n {
            matrix[0][j] = j;
        }
        
        for i in 1..=m {
            for j in 1..=n {
                let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }
        
        matrix[m][n]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_diagnostic_builder() {
        let diag = Diagnostic::error(ErrorCode::UndefinedVariable, "undefined variable `foo`")
            .with_span(SourceSpan::new(10, 13, 2, 5))
            .with_file("test.hate")
            .with_help("did you mean `bar`?")
            .with_note("variables must be declared before use");
        
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.span.is_some());
        assert!(!diag.help.is_empty());
    }
    
    #[test]
    fn test_error_reporter() {
        let reporter = ErrorReporter::new().with_colors(false);
        
        let diag = Diagnostic::error(ErrorCode::UnexpectedToken, "unexpected token `}`")
            .with_span(SourceSpan::new(20, 21, 3, 5))
            .with_file("test.hate")
            .with_source("fn main() {\n  print(1)\n    }\n}")
            .with_help("remove the extra `}`");
        
        let output = reporter.format(&diag);
        assert!(output.contains("error[E1001]"));
        assert!(output.contains("unexpected token"));
        assert!(output.contains("test.hate:3:5"));
    }
    
    #[test]
    fn test_stack_trace() {
        let reporter = ErrorReporter::new().with_colors(false);
        
        let frames = vec![
            StackFrame::new("divide", 10, 5).with_file("math.hate"),
            StackFrame::new("calculate", 25, 10).with_file("calc.hate"),
            StackFrame::new("<main>", 5, 1).with_file("main.hate"),
        ];
        
        let output = reporter.format_stack_trace(&frames);
        assert!(output.contains("Stack trace"));
        assert!(output.contains("divide"));
        assert!(output.contains("calculate"));
    }
    
    #[test]
    fn test_levenshtein() {
        assert_eq!(ErrorSuggestions::levenshtein("kitten", "sitting"), 3);
        assert_eq!(ErrorSuggestions::levenshtein("foo", "foo"), 0);
        assert_eq!(ErrorSuggestions::levenshtein("bar", "baz"), 1);
    }
    
    #[test]
    fn test_typo_suggestions() {
        assert!(ErrorSuggestions::suggest_typo("fucntion").is_some());
        assert!(ErrorSuggestions::suggest_typo("function").is_none());
    }
    
    #[test]
    fn test_variable_suggestions() {
        let available = &["bar", "baz", "qux"];
        let suggestions = ErrorSuggestions::suggest_variable("bra", available);
        assert!(suggestions.iter().any(|s| s.contains("bar")));
    }
}
