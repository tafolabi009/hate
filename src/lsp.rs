//! Language Server Protocol Implementation
//!
//! Provides IDE features for Hate:
//! - Code completion
//! - Go to definition
//! - Find references
//! - Hover information
//! - Diagnostics
//! - Code actions

use crate::diagnostic::Diagnostic;
use std::collections::HashMap;
use std::path::PathBuf;

/// Position in a document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// Range in a document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
    
    pub fn point(line: u32, character: u32) -> Self {
        let pos = Position::new(line, character);
        Self { start: pos, end: pos }
    }
    
    pub fn contains(&self, pos: Position) -> bool {
        if pos.line < self.start.line || pos.line > self.end.line {
            return false;
        }
        if pos.line == self.start.line && pos.character < self.start.character {
            return false;
        }
        if pos.line == self.end.line && pos.character > self.end.character {
            return false;
        }
        true
    }
}

/// Location (file + range)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// Symbol information
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub container: Option<String>,
    pub detail: Option<String>,
}

/// Symbol kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    File,
    Module,
    Namespace,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    Enum,
    Interface,
    TypeParameter,
}

impl SymbolKind {
    pub fn to_lsp_kind(&self) -> u32 {
        match self {
            SymbolKind::File => 1,
            SymbolKind::Module => 2,
            SymbolKind::Namespace => 3,
            SymbolKind::Class => 5,
            SymbolKind::Method => 6,
            SymbolKind::Property => 7,
            SymbolKind::Field => 8,
            SymbolKind::Constructor => 9,
            SymbolKind::Function => 12,
            SymbolKind::Variable => 13,
            SymbolKind::Constant => 14,
            SymbolKind::String => 15,
            SymbolKind::Number => 16,
            SymbolKind::Boolean => 17,
            SymbolKind::Array => 18,
            SymbolKind::Object => 19,
            SymbolKind::Key => 20,
            SymbolKind::Null => 21,
            SymbolKind::Enum => 10,
            SymbolKind::Interface => 11,
            SymbolKind::TypeParameter => 26,
        }
    }
}

/// Completion item
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
    pub filter_text: Option<String>,
}

/// Completion item kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Keyword,
    Snippet,
    Constant,
    Enum,
}

impl CompletionKind {
    pub fn to_lsp_kind(&self) -> u32 {
        match self {
            CompletionKind::Text => 1,
            CompletionKind::Method => 2,
            CompletionKind::Function => 3,
            CompletionKind::Constructor => 4,
            CompletionKind::Field => 5,
            CompletionKind::Variable => 6,
            CompletionKind::Class => 7,
            CompletionKind::Interface => 8,
            CompletionKind::Module => 9,
            CompletionKind::Property => 10,
            CompletionKind::Keyword => 14,
            CompletionKind::Snippet => 15,
            CompletionKind::Constant => 21,
            CompletionKind::Enum => 13,
        }
    }
}

/// Hover information
#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub contents: String,
    pub range: Option<Range>,
}

/// Code action
#[derive(Debug, Clone)]
pub struct CodeAction {
    pub title: String,
    pub kind: CodeActionKind,
    pub diagnostics: Vec<Diagnostic>,
    pub edit: Option<WorkspaceEdit>,
    pub command: Option<Command>,
}

/// Code action kinds
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeActionKind {
    QuickFix,
    Refactor,
    RefactorExtract,
    RefactorInline,
    RefactorRewrite,
    Source,
    SourceOrganizeImports,
}

impl CodeActionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            CodeActionKind::QuickFix => "quickfix",
            CodeActionKind::Refactor => "refactor",
            CodeActionKind::RefactorExtract => "refactor.extract",
            CodeActionKind::RefactorInline => "refactor.inline",
            CodeActionKind::RefactorRewrite => "refactor.rewrite",
            CodeActionKind::Source => "source",
            CodeActionKind::SourceOrganizeImports => "source.organizeImports",
        }
    }
}

/// Workspace edit
#[derive(Debug, Clone, Default)]
pub struct WorkspaceEdit {
    pub changes: HashMap<String, Vec<TextEdit>>,
}

/// Text edit
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

/// Command
#[derive(Debug, Clone)]
pub struct Command {
    pub title: String,
    pub command: String,
    pub arguments: Vec<String>,
}

/// Document symbol table
#[derive(Debug, Default)]
pub struct DocumentSymbols {
    /// All symbols in the document
    pub symbols: Vec<SymbolInfo>,
    /// Symbol name to indices
    pub name_to_indices: HashMap<String, Vec<usize>>,
    /// Definition locations
    pub definitions: HashMap<String, Location>,
    /// Reference locations
    pub references: HashMap<String, Vec<Location>>,
}

impl DocumentSymbols {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a symbol
    pub fn add_symbol(&mut self, symbol: SymbolInfo) {
        let name = symbol.name.clone();
        let idx = self.symbols.len();
        
        self.name_to_indices
            .entry(name.clone())
            .or_default()
            .push(idx);
        
        self.definitions.insert(name, symbol.location.clone());
        self.symbols.push(symbol);
    }
    
    /// Add a reference
    pub fn add_reference(&mut self, name: &str, location: Location) {
        self.references
            .entry(name.to_string())
            .or_default()
            .push(location);
    }
    
    /// Find symbol at position
    pub fn symbol_at(&self, pos: Position) -> Option<&SymbolInfo> {
        self.symbols.iter().find(|s| s.location.range.contains(pos))
    }
    
    /// Find definition
    pub fn find_definition(&self, name: &str) -> Option<&Location> {
        self.definitions.get(name)
    }
    
    /// Find references
    pub fn find_references(&self, name: &str) -> Vec<&Location> {
        let mut refs = Vec::new();
        
        // Include definition
        if let Some(def) = self.definitions.get(name) {
            refs.push(def);
        }
        
        // Include references
        if let Some(r) = self.references.get(name) {
            refs.extend(r);
        }
        
        refs
    }
}

/// Language server
pub struct LanguageServer {
    /// Open documents
    documents: HashMap<String, String>,
    /// Document symbols
    symbols: HashMap<String, DocumentSymbols>,
    /// Workspace root
    root: Option<PathBuf>,
}

impl LanguageServer {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            symbols: HashMap::new(),
            root: None,
        }
    }
    
    /// Set workspace root
    pub fn set_root(&mut self, root: PathBuf) {
        self.root = Some(root);
    }
    
    /// Open a document
    pub fn open_document(&mut self, uri: &str, content: &str) {
        self.documents.insert(uri.to_string(), content.to_string());
        self.analyze_document(uri);
    }
    
    /// Update a document
    pub fn update_document(&mut self, uri: &str, content: &str) {
        self.documents.insert(uri.to_string(), content.to_string());
        self.analyze_document(uri);
    }
    
    /// Close a document
    pub fn close_document(&mut self, uri: &str) {
        self.documents.remove(uri);
        self.symbols.remove(uri);
    }
    
    /// Analyze a document and build symbol table
    fn analyze_document(&mut self, uri: &str) {
        let content = match self.documents.get(uri) {
            Some(c) => c.clone(),
            None => return,
        };
        
        let symbols = DocumentSymbols::new();
        
        // In a full implementation, we would:
        // 1. Parse the document
        // 2. Walk the AST to collect symbols
        // 3. Build the symbol table
        
        self.symbols.insert(uri.to_string(), symbols);
    }
    
    /// Get completions at position
    pub fn completions(&self, uri: &str, pos: Position) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        // Add keywords
        let keywords = [
            "fn", "let", "const", "if", "else", "while", "for", "in",
            "return", "break", "continue", "class", "extends", "new",
            "this", "super", "import", "export", "from", "as",
            "async", "await", "try", "catch", "finally", "throw",
            "match", "true", "false", "null",
        ];
        
        for kw in keywords {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: Some("keyword".to_string()),
                documentation: None,
                insert_text: None,
                filter_text: None,
            });
        }
        
        // Add symbols from current document
        if let Some(syms) = self.symbols.get(uri) {
            for sym in &syms.symbols {
                let kind = match sym.kind {
                    SymbolKind::Function => CompletionKind::Function,
                    SymbolKind::Variable => CompletionKind::Variable,
                    SymbolKind::Constant => CompletionKind::Constant,
                    SymbolKind::Class => CompletionKind::Class,
                    SymbolKind::Method => CompletionKind::Method,
                    SymbolKind::Property => CompletionKind::Property,
                    _ => CompletionKind::Text,
                };
                
                items.push(CompletionItem {
                    label: sym.name.clone(),
                    kind,
                    detail: sym.detail.clone(),
                    documentation: None,
                    insert_text: None,
                    filter_text: None,
                });
            }
        }
        
        // Add snippets
        items.push(CompletionItem {
            label: "fn".to_string(),
            kind: CompletionKind::Snippet,
            detail: Some("function definition".to_string()),
            documentation: None,
            insert_text: Some("fn ${1:name}(${2:params}) {\n\t$0\n}".to_string()),
            filter_text: Some("fn".to_string()),
        });
        
        items.push(CompletionItem {
            label: "for".to_string(),
            kind: CompletionKind::Snippet,
            detail: Some("for loop".to_string()),
            documentation: None,
            insert_text: Some("for ${1:item} in ${2:iterable} {\n\t$0\n}".to_string()),
            filter_text: Some("for".to_string()),
        });
        
        items.push(CompletionItem {
            label: "if".to_string(),
            kind: CompletionKind::Snippet,
            detail: Some("if statement".to_string()),
            documentation: None,
            insert_text: Some("if ${1:condition} {\n\t$0\n}".to_string()),
            filter_text: Some("if".to_string()),
        });
        
        items
    }
    
    /// Get hover info at position
    pub fn hover(&self, uri: &str, pos: Position) -> Option<HoverInfo> {
        let syms = self.symbols.get(uri)?;
        let sym = syms.symbol_at(pos)?;
        
        let kind_str = match sym.kind {
            SymbolKind::Function => "function",
            SymbolKind::Variable => "variable",
            SymbolKind::Constant => "constant",
            SymbolKind::Class => "class",
            SymbolKind::Method => "method",
            SymbolKind::Property => "property",
            _ => "symbol",
        };
        
        let mut contents = format!("**{}** _{}_\n\n", sym.name, kind_str);
        
        if let Some(detail) = &sym.detail {
            contents.push_str(detail);
        }
        
        Some(HoverInfo {
            contents,
            range: Some(sym.location.range),
        })
    }
    
    /// Go to definition
    pub fn definition(&self, uri: &str, pos: Position) -> Option<Location> {
        let syms = self.symbols.get(uri)?;
        let sym = syms.symbol_at(pos)?;
        syms.find_definition(&sym.name).cloned()
    }
    
    /// Find references
    pub fn references(&self, uri: &str, pos: Position) -> Vec<Location> {
        let syms = match self.symbols.get(uri) {
            Some(s) => s,
            None => return Vec::new(),
        };
        
        let sym = match syms.symbol_at(pos) {
            Some(s) => s,
            None => return Vec::new(),
        };
        
        syms.find_references(&sym.name)
            .into_iter()
            .cloned()
            .collect()
    }
    
    /// Get document symbols
    pub fn document_symbols(&self, uri: &str) -> Vec<SymbolInfo> {
        self.symbols
            .get(uri)
            .map(|s| s.symbols.clone())
            .unwrap_or_default()
    }
    
    /// Get code actions at range
    pub fn code_actions(&self, _uri: &str, _range: Range, diagnostics: &[Diagnostic]) -> Vec<CodeAction> {
        let mut actions = Vec::new();
        
        for diag in diagnostics {
            // Quick fix for undefined variable
            if diag.message.contains("undefined variable") {
                if let Some(var_name) = extract_quoted(&diag.message) {
                    // Suggest creating the variable
                    let edit = WorkspaceEdit::default();
                    // In production, calculate the correct insertion point
                    
                    actions.push(CodeAction {
                        title: format!("Create variable `{}`", var_name),
                        kind: CodeActionKind::QuickFix,
                        diagnostics: vec![diag.clone()],
                        edit: Some(edit),
                        command: None,
                    });
                }
            }
            
            // Quick fix for unused variable
            if diag.message.contains("unused variable") {
                if let Some(var_name) = extract_quoted(&diag.message) {
                    actions.push(CodeAction {
                        title: format!("Prefix with underscore: `_{}`", var_name),
                        kind: CodeActionKind::QuickFix,
                        diagnostics: vec![diag.clone()],
                        edit: None, // Would include the edit
                        command: None,
                    });
                    
                    actions.push(CodeAction {
                        title: format!("Remove variable `{}`", var_name),
                        kind: CodeActionKind::QuickFix,
                        diagnostics: vec![diag.clone()],
                        edit: None,
                        command: None,
                    });
                }
            }
        }
        
        // Always available refactorings
        actions.push(CodeAction {
            title: "Extract to function".to_string(),
            kind: CodeActionKind::RefactorExtract,
            diagnostics: vec![],
            edit: None,
            command: Some(Command {
                title: "Extract to function".to_string(),
                command: "hate.extractFunction".to_string(),
                arguments: vec![],
            }),
        });
        
        actions
    }
}

impl Default for LanguageServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract a quoted string from a message
fn extract_quoted(s: &str) -> Option<String> {
    let start = s.find('`')? + 1;
    let end = s[start..].find('`')? + start;
    Some(s[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_position_range() {
        let range = Range::new(
            Position::new(1, 5),
            Position::new(1, 10),
        );
        
        assert!(range.contains(Position::new(1, 5)));
        assert!(range.contains(Position::new(1, 7)));
        assert!(range.contains(Position::new(1, 10)));
        assert!(!range.contains(Position::new(1, 4)));
        assert!(!range.contains(Position::new(1, 11)));
        assert!(!range.contains(Position::new(0, 7)));
    }
    
    #[test]
    fn test_document_symbols() {
        let mut symbols = DocumentSymbols::new();
        
        symbols.add_symbol(SymbolInfo {
            name: "foo".to_string(),
            kind: SymbolKind::Function,
            location: Location {
                uri: "test.hate".to_string(),
                range: Range::point(1, 0),
            },
            container: None,
            detail: Some("fn foo()".to_string()),
        });
        
        assert!(symbols.find_definition("foo").is_some());
        assert!(symbols.find_definition("bar").is_none());
    }
    
    #[test]
    fn test_language_server_completions() {
        let server = LanguageServer::new();
        let completions = server.completions("test.hate", Position::new(0, 0));
        
        // Should have keywords
        assert!(completions.iter().any(|c| c.label == "fn"));
        assert!(completions.iter().any(|c| c.label == "let"));
    }
    
    #[test]
    fn test_extract_quoted() {
        assert_eq!(extract_quoted("undefined variable `foo`"), Some("foo".to_string()));
        assert_eq!(extract_quoted("no quotes here"), None);
    }
}
