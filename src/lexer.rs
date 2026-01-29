//! Lexer for the Hate programming language
//!
//! Hand-written lexer optimized for speed with:
//! - Zero-copy string handling via string interning
//! - Single-pass tokenization
//! - Line and column tracking for error reporting

use crate::intern::{Symbol, intern};
use crate::error::{HateError, HateResult, Span};

/// Token types in the Hate language
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // Literals
    Integer,        // 42
    Float,          // 3.14
    String,         // "hello"
    True,           // true
    False,          // false
    Null,           // null
    
    // Identifiers and keywords
    Identifier,     // foo, bar
    Let,            // let
    Mut,            // mut
    Fn,             // fn
    Return,         // return
    If,             // if
    Else,           // else
    While,          // while
    For,            // for
    In,             // in
    Break,          // break
    Continue,       // continue
    Match,          // match
    Class,          // class
    New,            // new
    Import,         // import
    Export,         // export
    From,           // from
    As,             // as
    Async,          // async
    Await,          // await
    Impl,           // impl
    Self_,          // self
    
    // Type keywords
    I32,            // i32
    I64,            // i64
    U32,            // u32
    U64,            // u64
    F64,            // f64
    Bool,           // bool
    Str,            // str
    
    // Operators - Arithmetic
    Plus,           // +
    Minus,          // -
    Star,           // *
    Slash,          // /
    Percent,        // %
    
    // Operators - Comparison
    EqualEqual,     // ==
    BangEqual,      // !=
    Less,           // <
    LessEqual,      // <=
    Greater,        // >
    GreaterEqual,   // >=
    
    // Operators - Logical
    And,            // &&
    Or,             // ||
    Bang,           // !
    
    // Operators - Bitwise
    Ampersand,      // &
    Pipe,           // |
    Caret,          // ^
    Tilde,          // ~
    LessLess,       // <<
    GreaterGreater, // >>
    
    // Operators - Assignment
    Equal,          // =
    PlusEqual,      // +=
    MinusEqual,     // -=
    StarEqual,      // *=
    SlashEqual,     // /=
    
    // Delimiters
    LeftParen,      // (
    RightParen,     // )
    LeftBrace,      // {
    RightBrace,     // }
    LeftBracket,    // [
    RightBracket,   // ]
    
    // Punctuation
    Comma,          // ,
    Dot,            // .
    Colon,          // :
    Semicolon,      // ;
    Arrow,          // =>
    ThinArrow,      // ->
    Question,       // ?
    QuestionDot,    // ?.
    DoubleQuestion, // ??
    DotDot,         // ..
    DotDotDot,      // ...
    
    // Special
    Newline,        // \n (significant in some contexts)
    Eof,            // End of file
}

impl TokenKind {
    /// Check if this token is a keyword
    pub fn is_keyword(&self) -> bool {
        matches!(self,
            TokenKind::Let | TokenKind::Mut | TokenKind::Fn | TokenKind::Return |
            TokenKind::If | TokenKind::Else | TokenKind::While | TokenKind::For |
            TokenKind::In | TokenKind::Break | TokenKind::Continue | TokenKind::Match | 
            TokenKind::Class | TokenKind::Import |
            TokenKind::Export | TokenKind::From | TokenKind::As | TokenKind::Async |
            TokenKind::Await | TokenKind::Impl | TokenKind::Self_ | TokenKind::True |
            TokenKind::False | TokenKind::Null
        )
    }
    
    /// Check if this token is a literal
    pub fn is_literal(&self) -> bool {
        matches!(self,
            TokenKind::Integer | TokenKind::Float | TokenKind::String |
            TokenKind::True | TokenKind::False | TokenKind::Null
        )
    }
    
    /// Check if this token is an operator
    pub fn is_operator(&self) -> bool {
        matches!(self,
            TokenKind::Plus | TokenKind::Minus | TokenKind::Star | TokenKind::Slash |
            TokenKind::Percent | TokenKind::EqualEqual | TokenKind::BangEqual |
            TokenKind::Less | TokenKind::LessEqual | TokenKind::Greater |
            TokenKind::GreaterEqual | TokenKind::And | TokenKind::Or | TokenKind::Bang
        )
    }
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Integer => write!(f, "integer"),
            TokenKind::Float => write!(f, "float"),
            TokenKind::String => write!(f, "string"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Null => write!(f, "null"),
            TokenKind::Identifier => write!(f, "identifier"),
            TokenKind::Let => write!(f, "let"),
            TokenKind::Mut => write!(f, "mut"),
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::While => write!(f, "while"),
            TokenKind::For => write!(f, "for"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::Class => write!(f, "class"),
            TokenKind::New => write!(f, "new"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::Export => write!(f, "export"),
            TokenKind::From => write!(f, "from"),
            TokenKind::As => write!(f, "as"),
            TokenKind::Async => write!(f, "async"),
            TokenKind::Await => write!(f, "await"),
            TokenKind::Impl => write!(f, "impl"),
            TokenKind::Self_ => write!(f, "self"),
            TokenKind::I32 => write!(f, "i32"),
            TokenKind::I64 => write!(f, "i64"),
            TokenKind::U32 => write!(f, "u32"),
            TokenKind::U64 => write!(f, "u64"),
            TokenKind::F64 => write!(f, "f64"),
            TokenKind::Bool => write!(f, "bool"),
            TokenKind::Str => write!(f, "str"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::EqualEqual => write!(f, "=="),
            TokenKind::BangEqual => write!(f, "!="),
            TokenKind::Less => write!(f, "<"),
            TokenKind::LessEqual => write!(f, "<="),
            TokenKind::Greater => write!(f, ">"),
            TokenKind::GreaterEqual => write!(f, ">="),
            TokenKind::And => write!(f, "&&"),
            TokenKind::Or => write!(f, "||"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::Ampersand => write!(f, "&"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::Tilde => write!(f, "~"),
            TokenKind::LessLess => write!(f, "<<"),
            TokenKind::GreaterGreater => write!(f, ">>"),
            TokenKind::Equal => write!(f, "="),
            TokenKind::PlusEqual => write!(f, "+="),
            TokenKind::MinusEqual => write!(f, "-="),
            TokenKind::StarEqual => write!(f, "*="),
            TokenKind::SlashEqual => write!(f, "/="),
            TokenKind::LeftParen => write!(f, "("),
            TokenKind::RightParen => write!(f, ")"),
            TokenKind::LeftBrace => write!(f, "{{"),
            TokenKind::RightBrace => write!(f, "}}"),
            TokenKind::LeftBracket => write!(f, "["),
            TokenKind::RightBracket => write!(f, "]"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Arrow => write!(f, "=>"),
            TokenKind::ThinArrow => write!(f, "->"),
            TokenKind::Question => write!(f, "?"),
            TokenKind::QuestionDot => write!(f, "?."),
            TokenKind::DoubleQuestion => write!(f, "??"),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::DotDotDot => write!(f, "..."),
            TokenKind::Newline => write!(f, "newline"),
            TokenKind::Eof => write!(f, "end of file"),
        }
    }
}

/// A token in the Hate language
#[derive(Debug, Clone, Copy)]
pub struct Token {
    /// The type of token
    pub kind: TokenKind,
    /// The interned string value (for identifiers and strings)
    pub symbol: Option<Symbol>,
    /// Integer value (for integer literals)
    pub int_value: i64,
    /// Float value (for float literals)
    pub float_value: f64,
    /// Source location
    pub span: Span,
}

impl Token {
    /// Create a new token
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self {
            kind,
            symbol: None,
            int_value: 0,
            float_value: 0.0,
            span,
        }
    }
    
    /// Create a token with a symbol
    pub fn with_symbol(kind: TokenKind, symbol: Symbol, span: Span) -> Self {
        Self {
            kind,
            symbol: Some(symbol),
            int_value: 0,
            float_value: 0.0,
            span,
        }
    }
    
    /// Create a token with an integer value
    pub fn with_int(int_value: i64, span: Span) -> Self {
        Self {
            kind: TokenKind::Integer,
            symbol: None,
            int_value,
            float_value: 0.0,
            span,
        }
    }
    
    /// Create a token with a float value
    pub fn with_float(float_value: f64, span: Span) -> Self {
        Self {
            kind: TokenKind::Float,
            symbol: None,
            int_value: 0,
            float_value,
            span,
        }
    }
    
    /// Get the lexeme as a string (for error messages)
    pub fn lexeme(&self) -> String {
        if let Some(sym) = self.symbol {
            sym.as_str().to_string()
        } else {
            self.kind.to_string()
        }
    }
}

/// Lexer for the Hate language
pub struct Lexer<'a> {
    /// Source code being tokenized
    source: &'a str,
    /// Source as bytes for faster access
    bytes: &'a [u8],
    /// Current position in the source
    pos: usize,
    /// Current line number (1-indexed)
    line: usize,
    /// Current column number (1-indexed)
    column: usize,
    /// Start position of current token
    token_start: usize,
    /// Start line of current token
    token_line: usize,
    /// Start column of current token
    token_column: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            line: 1,
            column: 1,
            token_start: 0,
            token_line: 1,
            token_column: 1,
        }
    }
    
    /// Tokenize the entire source, returning all tokens
    pub fn tokenize(&mut self) -> HateResult<Vec<Token>> {
        let mut tokens = Vec::with_capacity(self.source.len() / 4); // Estimate
        
        loop {
            let token = self.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        
        Ok(tokens)
    }
    
    /// Get the next token
    pub fn next_token(&mut self) -> HateResult<Token> {
        self.skip_whitespace_and_comments();
        
        self.token_start = self.pos;
        self.token_line = self.line;
        self.token_column = self.column;
        
        if self.is_at_end() {
            return Ok(self.make_token(TokenKind::Eof));
        }
        
        let c = self.advance();
        
        // Identifiers and keywords
        if is_alpha(c) {
            return self.identifier();
        }
        
        // Numbers
        if is_digit(c) {
            return self.number();
        }
        
        // Single character tokens
        match c {
            b'(' => Ok(self.make_token(TokenKind::LeftParen)),
            b')' => Ok(self.make_token(TokenKind::RightParen)),
            b'{' => Ok(self.make_token(TokenKind::LeftBrace)),
            b'}' => Ok(self.make_token(TokenKind::RightBrace)),
            b'[' => Ok(self.make_token(TokenKind::LeftBracket)),
            b']' => Ok(self.make_token(TokenKind::RightBracket)),
            b',' => Ok(self.make_token(TokenKind::Comma)),
            b':' => Ok(self.make_token(TokenKind::Colon)),
            b';' => Ok(self.make_token(TokenKind::Semicolon)),
            b'~' => Ok(self.make_token(TokenKind::Tilde)),
            b'^' => Ok(self.make_token(TokenKind::Caret)),
            
            // Multi-character tokens
            b'.' => {
                if self.match_char(b'.') {
                    if self.match_char(b'.') {
                        Ok(self.make_token(TokenKind::DotDotDot))
                    } else {
                        Ok(self.make_token(TokenKind::DotDot))
                    }
                } else {
                    Ok(self.make_token(TokenKind::Dot))
                }
            }
            
            b'+' => {
                if self.match_char(b'=') {
                    Ok(self.make_token(TokenKind::PlusEqual))
                } else {
                    Ok(self.make_token(TokenKind::Plus))
                }
            }
            
            b'-' => {
                if self.match_char(b'=') {
                    Ok(self.make_token(TokenKind::MinusEqual))
                } else if self.match_char(b'>') {
                    Ok(self.make_token(TokenKind::ThinArrow))
                } else {
                    Ok(self.make_token(TokenKind::Minus))
                }
            }
            
            b'*' => {
                if self.match_char(b'=') {
                    Ok(self.make_token(TokenKind::StarEqual))
                } else {
                    Ok(self.make_token(TokenKind::Star))
                }
            }
            
            b'/' => {
                if self.match_char(b'=') {
                    Ok(self.make_token(TokenKind::SlashEqual))
                } else {
                    Ok(self.make_token(TokenKind::Slash))
                }
            }
            
            b'%' => Ok(self.make_token(TokenKind::Percent)),
            
            b'=' => {
                if self.match_char(b'=') {
                    Ok(self.make_token(TokenKind::EqualEqual))
                } else if self.match_char(b'>') {
                    Ok(self.make_token(TokenKind::Arrow))
                } else {
                    Ok(self.make_token(TokenKind::Equal))
                }
            }
            
            b'!' => {
                if self.match_char(b'=') {
                    Ok(self.make_token(TokenKind::BangEqual))
                } else {
                    Ok(self.make_token(TokenKind::Bang))
                }
            }
            
            b'<' => {
                if self.match_char(b'=') {
                    Ok(self.make_token(TokenKind::LessEqual))
                } else if self.match_char(b'<') {
                    Ok(self.make_token(TokenKind::LessLess))
                } else {
                    Ok(self.make_token(TokenKind::Less))
                }
            }
            
            b'>' => {
                if self.match_char(b'=') {
                    Ok(self.make_token(TokenKind::GreaterEqual))
                } else if self.match_char(b'>') {
                    Ok(self.make_token(TokenKind::GreaterGreater))
                } else {
                    Ok(self.make_token(TokenKind::Greater))
                }
            }
            
            b'&' => {
                if self.match_char(b'&') {
                    Ok(self.make_token(TokenKind::And))
                } else {
                    Ok(self.make_token(TokenKind::Ampersand))
                }
            }
            
            b'|' => {
                if self.match_char(b'|') {
                    Ok(self.make_token(TokenKind::Or))
                } else {
                    Ok(self.make_token(TokenKind::Pipe))
                }
            }
            
            b'?' => {
                if self.match_char(b'.') {
                    Ok(self.make_token(TokenKind::QuestionDot))
                } else if self.match_char(b'?') {
                    Ok(self.make_token(TokenKind::DoubleQuestion))
                } else {
                    Ok(self.make_token(TokenKind::Question))
                }
            }
            
            b'"' => self.string(),
            
            _ => Err(HateError::UnexpectedChar {
                ch: c as char,
                span: self.current_span(),
            }),
        }
    }
    
    // ==================== Helper Methods ====================
    
    /// Check if we've reached the end of the source
    #[inline(always)]
    fn is_at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }
    
    /// Peek at the current character without consuming it
    #[inline(always)]
    fn peek(&self) -> u8 {
        if self.is_at_end() { 0 } else { self.bytes[self.pos] }
    }
    
    /// Peek at the next character without consuming it
    #[inline(always)]
    fn peek_next(&self) -> u8 {
        if self.pos + 1 >= self.bytes.len() { 0 } else { self.bytes[self.pos + 1] }
    }
    
    /// Advance to the next character
    #[inline(always)]
    fn advance(&mut self) -> u8 {
        let c = self.bytes[self.pos];
        self.pos += 1;
        if c == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        c
    }
    
    /// Match and consume a character if it matches
    #[inline(always)]
    fn match_char(&mut self, expected: u8) -> bool {
        if self.is_at_end() || self.bytes[self.pos] != expected {
            return false;
        }
        self.advance();
        true
    }
    
    /// Skip whitespace and comments
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            if self.is_at_end() {
                return;
            }
            
            match self.peek() {
                b' ' | b'\t' | b'\r' | b'\n' => {
                    self.advance();
                }
                b'/' => {
                    if self.peek_next() == b'/' {
                        // Single-line comment
                        while !self.is_at_end() && self.peek() != b'\n' {
                            self.advance();
                        }
                    } else if self.peek_next() == b'*' {
                        // Multi-line comment
                        self.advance(); // consume /
                        self.advance(); // consume *
                        while !self.is_at_end() {
                            if self.peek() == b'*' && self.peek_next() == b'/' {
                                self.advance(); // consume *
                                self.advance(); // consume /
                                break;
                            }
                            self.advance();
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            }
        }
    }
    
    /// Get the current span
    fn current_span(&self) -> Span {
        Span::new(self.token_start, self.pos, self.token_line, self.token_column)
    }
    
    /// Make a token with the current span
    fn make_token(&self, kind: TokenKind) -> Token {
        Token::new(kind, self.current_span())
    }
    
    /// Get the current lexeme
    fn current_lexeme(&self) -> &str {
        &self.source[self.token_start..self.pos]
    }
    
    // ==================== Token Scanners ====================
    
    /// Scan an identifier or keyword
    fn identifier(&mut self) -> HateResult<Token> {
        while !self.is_at_end() && is_alphanumeric(self.peek()) {
            self.advance();
        }
        
        let lexeme = self.current_lexeme();
        let kind = match lexeme {
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "fn" => TokenKind::Fn,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "match" => TokenKind::Match,
            "class" => TokenKind::Class,
            "new" => TokenKind::New,
            "import" => TokenKind::Import,
            "export" => TokenKind::Export,
            "from" => TokenKind::From,
            "as" => TokenKind::As,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "impl" => TokenKind::Impl,
            "self" => TokenKind::Self_,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            "i32" => TokenKind::I32,
            "i64" => TokenKind::I64,
            "u32" => TokenKind::U32,
            "u64" => TokenKind::U64,
            "f64" => TokenKind::F64,
            "bool" => TokenKind::Bool,
            "str" => TokenKind::Str,
            _ => TokenKind::Identifier,
        };
        
        if kind == TokenKind::Identifier {
            let symbol = intern(lexeme);
            Ok(Token::with_symbol(TokenKind::Identifier, symbol, self.current_span()))
        } else {
            Ok(self.make_token(kind))
        }
    }
    
    /// Scan a number (integer or float)
    fn number(&mut self) -> HateResult<Token> {
        // Scan integer part
        while !self.is_at_end() && is_digit(self.peek()) {
            self.advance();
        }
        
        // Check for fractional part
        if self.peek() == b'.' && is_digit(self.peek_next()) {
            self.advance(); // consume '.'
            while !self.is_at_end() && is_digit(self.peek()) {
                self.advance();
            }
            
            // Check for exponent
            if self.peek() == b'e' || self.peek() == b'E' {
                self.advance();
                if self.peek() == b'+' || self.peek() == b'-' {
                    self.advance();
                }
                while !self.is_at_end() && is_digit(self.peek()) {
                    self.advance();
                }
            }
            
            let lexeme = self.current_lexeme();
            let value: f64 = lexeme.parse().map_err(|_| HateError::InvalidNumber {
                span: self.current_span(),
            })?;
            
            Ok(Token::with_float(value, self.current_span()))
        } else {
            // Integer
            let lexeme = self.current_lexeme();
            let value: i64 = lexeme.parse().map_err(|_| HateError::InvalidNumber {
                span: self.current_span(),
            })?;
            
            Ok(Token::with_int(value, self.current_span()))
        }
    }
    
    /// Scan a string literal
    fn string(&mut self) -> HateResult<Token> {
        let mut value = String::new();
        
        while !self.is_at_end() && self.peek() != b'"' {
            if self.peek() == b'\\' {
                self.advance(); // consume backslash
                if self.is_at_end() {
                    return Err(HateError::UnterminatedString {
                        span: self.current_span(),
                    });
                }
                
                let escaped = self.advance();
                match escaped {
                    b'n' => value.push('\n'),
                    b't' => value.push('\t'),
                    b'r' => value.push('\r'),
                    b'\\' => value.push('\\'),
                    b'"' => value.push('"'),
                    b'0' => value.push('\0'),
                    _ => {
                        return Err(HateError::InvalidEscape {
                            ch: escaped as char,
                            span: self.current_span(),
                        });
                    }
                }
            } else if self.peek() == b'\n' {
                // Allow multi-line strings
                value.push('\n');
                self.advance();
            } else {
                value.push(self.advance() as char);
            }
        }
        
        if self.is_at_end() {
            return Err(HateError::UnterminatedString {
                span: self.current_span(),
            });
        }
        
        self.advance(); // consume closing quote
        
        let symbol = intern(&value);
        Ok(Token::with_symbol(TokenKind::String, symbol, self.current_span()))
    }
}

// ==================== Character Classification ====================

#[inline(always)]
fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

#[inline(always)]
fn is_alpha(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

#[inline(always)]
fn is_alphanumeric(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn tokenize(source: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(source);
        lexer.tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }
    
    #[test]
    fn test_empty() {
        assert_eq!(tokenize(""), vec![TokenKind::Eof]);
    }
    
    #[test]
    fn test_whitespace() {
        assert_eq!(tokenize("   \t\n  "), vec![TokenKind::Eof]);
    }
    
    #[test]
    fn test_comments() {
        assert_eq!(tokenize("// comment\n42"), vec![TokenKind::Integer, TokenKind::Eof]);
        assert_eq!(tokenize("/* multi\nline */ 42"), vec![TokenKind::Integer, TokenKind::Eof]);
    }
    
    #[test]
    fn test_integers() {
        let mut lexer = Lexer::new("42 0 999");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].kind, TokenKind::Integer);
        assert_eq!(tokens[0].int_value, 42);
        assert_eq!(tokens[1].int_value, 0);
        assert_eq!(tokens[2].int_value, 999);
    }
    
    #[test]
    fn test_floats() {
        let mut lexer = Lexer::new("3.14 0.5 1e10");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].kind, TokenKind::Float);
        assert!((tokens[0].float_value - 3.14).abs() < f64::EPSILON);
    }
    
    #[test]
    fn test_strings() {
        let mut lexer = Lexer::new("\"hello\" \"with\\nnewline\"");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].kind, TokenKind::String);
        assert_eq!(tokens[0].symbol.unwrap().as_str(), "hello");
        assert_eq!(tokens[1].symbol.unwrap().as_str(), "with\nnewline");
    }
    
    #[test]
    fn test_keywords() {
        assert_eq!(
            tokenize("let mut fn return if else while for in match"),
            vec![
                TokenKind::Let, TokenKind::Mut, TokenKind::Fn, TokenKind::Return,
                TokenKind::If, TokenKind::Else, TokenKind::While, TokenKind::For,
                TokenKind::In, TokenKind::Match, TokenKind::Eof
            ]
        );
    }
    
    #[test]
    fn test_identifiers() {
        let mut lexer = Lexer::new("foo bar_baz _private");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].symbol.unwrap().as_str(), "foo");
    }
    
    #[test]
    fn test_operators() {
        assert_eq!(
            tokenize("+ - * / % == != < <= > >= && || !"),
            vec![
                TokenKind::Plus, TokenKind::Minus, TokenKind::Star, TokenKind::Slash,
                TokenKind::Percent, TokenKind::EqualEqual, TokenKind::BangEqual,
                TokenKind::Less, TokenKind::LessEqual, TokenKind::Greater,
                TokenKind::GreaterEqual, TokenKind::And, TokenKind::Or, TokenKind::Bang,
                TokenKind::Eof
            ]
        );
    }
    
    #[test]
    fn test_delimiters() {
        assert_eq!(
            tokenize("( ) { } [ ] , . : ; => ->"),
            vec![
                TokenKind::LeftParen, TokenKind::RightParen,
                TokenKind::LeftBrace, TokenKind::RightBrace,
                TokenKind::LeftBracket, TokenKind::RightBracket,
                TokenKind::Comma, TokenKind::Dot, TokenKind::Colon,
                TokenKind::Semicolon, TokenKind::Arrow, TokenKind::ThinArrow,
                TokenKind::Eof
            ]
        );
    }
    
    #[test]
    fn test_optional_chaining() {
        assert_eq!(
            tokenize("?. ??"),
            vec![TokenKind::QuestionDot, TokenKind::DoubleQuestion, TokenKind::Eof]
        );
    }
    
    #[test]
    fn test_spread() {
        assert_eq!(
            tokenize(".. ..."),
            vec![TokenKind::DotDot, TokenKind::DotDotDot, TokenKind::Eof]
        );
    }
    
    #[test]
    fn test_complex_expression() {
        assert_eq!(
            tokenize("let x = 10 + 20;"),
            vec![
                TokenKind::Let, TokenKind::Identifier, TokenKind::Equal,
                TokenKind::Integer, TokenKind::Plus, TokenKind::Integer,
                TokenKind::Semicolon, TokenKind::Eof
            ]
        );
    }
    
    #[test]
    fn test_function() {
        assert_eq!(
            tokenize("fn add(a: i64, b: i64): i64 => a + b;"),
            vec![
                TokenKind::Fn, TokenKind::Identifier, TokenKind::LeftParen,
                TokenKind::Identifier, TokenKind::Colon, TokenKind::I64,
                TokenKind::Comma, TokenKind::Identifier, TokenKind::Colon,
                TokenKind::I64, TokenKind::RightParen, TokenKind::Colon,
                TokenKind::I64, TokenKind::Arrow, TokenKind::Identifier,
                TokenKind::Plus, TokenKind::Identifier, TokenKind::Semicolon,
                TokenKind::Eof
            ]
        );
    }
    
    #[test]
    fn test_span_tracking() {
        let mut lexer = Lexer::new("let x = 10;");
        let tokens = lexer.tokenize().unwrap();
        
        // 'let' starts at column 1
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.column, 1);
        
        // 'x' starts at column 5
        assert_eq!(tokens[1].span.column, 5);
    }
    
    #[test]
    fn test_error_recovery() {
        let mut lexer = Lexer::new("@");
        let result = lexer.next_token();
        assert!(result.is_err());
    }
}
