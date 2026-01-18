//! Debugger
//!
//! Implements Debug Adapter Protocol (DAP) compatible debugging:
//! - Breakpoints
//! - Step execution
//! - Variable inspection
//! - Call stack
//! - Watch expressions

use crate::value::Value;
use std::collections::HashMap;
use std::path::PathBuf;

/// Breakpoint types
#[derive(Debug, Clone)]
pub enum BreakpointKind {
    /// Line breakpoint
    Line,
    /// Conditional breakpoint
    Conditional { expression: String },
    /// Log point (doesn't stop, just logs)
    LogPoint { message: String },
    /// Hit count breakpoint
    HitCount { count: u32 },
    /// Function breakpoint
    Function { name: String },
}

/// A breakpoint
#[derive(Debug, Clone)]
pub struct Breakpoint {
    /// Unique ID
    pub id: u32,
    /// Source file
    pub file: PathBuf,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column (optional)
    pub column: Option<u32>,
    /// Kind of breakpoint
    pub kind: BreakpointKind,
    /// Is it enabled?
    pub enabled: bool,
    /// Is it verified (valid location)?
    pub verified: bool,
    /// Hit count
    pub hit_count: u32,
}

impl Breakpoint {
    pub fn line(id: u32, file: PathBuf, line: u32) -> Self {
        Self {
            id,
            file,
            line,
            column: None,
            kind: BreakpointKind::Line,
            enabled: true,
            verified: false,
            hit_count: 0,
        }
    }
    
    pub fn conditional(id: u32, file: PathBuf, line: u32, expr: String) -> Self {
        Self {
            id,
            file,
            line,
            column: None,
            kind: BreakpointKind::Conditional { expression: expr },
            enabled: true,
            verified: false,
            hit_count: 0,
        }
    }
    
    pub fn hit(&mut self) {
        self.hit_count += 1;
    }
}

/// Stepping mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    /// Continue until next breakpoint
    Continue,
    /// Step over (next line in current function)
    StepOver,
    /// Step into (enter function calls)
    StepInto,
    /// Step out (return from current function)
    StepOut,
    /// Single instruction
    StepInstruction,
}

/// Variable scope
#[derive(Debug, Clone)]
pub struct Scope {
    /// Scope name
    pub name: String,
    /// Scope type
    pub kind: ScopeKind,
    /// Variables in scope
    pub variables: Vec<Variable>,
    /// Variable reference for expensive scopes
    pub variables_reference: u32,
}

/// Scope types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Arguments,
    Locals,
    Globals,
    Registers,
}

/// A variable
#[derive(Debug, Clone)]
pub struct Variable {
    /// Variable name
    pub name: String,
    /// Value as string
    pub value: String,
    /// Type name
    pub type_name: String,
    /// Reference for structured types (to expand children)
    pub variables_reference: u32,
    /// Named children count
    pub named_children: u32,
    /// Indexed children count
    pub indexed_children: u32,
}

impl Variable {
    pub fn simple(name: impl Into<String>, value: impl Into<String>, type_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            type_name: type_name.into(),
            variables_reference: 0,
            named_children: 0,
            indexed_children: 0,
        }
    }
    
    pub fn with_children(
        name: impl Into<String>,
        value: impl Into<String>,
        type_name: impl Into<String>,
        ref_id: u32,
        named: u32,
        indexed: u32,
    ) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            type_name: type_name.into(),
            variables_reference: ref_id,
            named_children: named,
            indexed_children: indexed,
        }
    }
}

/// Stack frame
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Frame ID
    pub id: u32,
    /// Function name
    pub name: String,
    /// Source file
    pub file: Option<PathBuf>,
    /// Line number
    pub line: u32,
    /// Column
    pub column: u32,
    /// End line (for highlighting)
    pub end_line: Option<u32>,
    /// End column
    pub end_column: Option<u32>,
}

/// Debug event
#[derive(Debug, Clone)]
pub enum DebugEvent {
    /// Execution stopped
    Stopped {
        reason: StopReason,
        thread_id: u32,
        frame: Option<StackFrame>,
    },
    /// Execution continued
    Continued { thread_id: u32 },
    /// Thread started
    ThreadStarted { thread_id: u32 },
    /// Thread exited
    ThreadExited { thread_id: u32 },
    /// Output message
    Output { category: String, output: String },
    /// Breakpoint changed
    BreakpointChanged { breakpoint: Breakpoint },
    /// Process exited
    Exited { exit_code: i32 },
    /// Process terminated
    Terminated,
}

/// Stop reasons
#[derive(Debug, Clone)]
pub enum StopReason {
    Step,
    Breakpoint { ids: Vec<u32> },
    Exception { description: String },
    Pause,
    Entry,
    DataBreakpoint,
    FunctionBreakpoint,
}

/// Evaluate context
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluateContext {
    /// Evaluate in watch window
    Watch,
    /// Evaluate in REPL
    Repl,
    /// Evaluate for hover
    Hover,
    /// Evaluate for clipboard
    Clipboard,
}

/// Debug session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugState {
    /// Not started
    NotStarted,
    /// Running
    Running,
    /// Paused at breakpoint
    Paused,
    /// Terminated
    Terminated,
}

/// Debugger
pub struct Debugger {
    /// Current state
    state: DebugState,
    /// All breakpoints
    breakpoints: HashMap<u32, Breakpoint>,
    /// Breakpoints by file/line for fast lookup
    line_breakpoints: HashMap<(PathBuf, u32), Vec<u32>>,
    /// Next breakpoint ID
    next_bp_id: u32,
    /// Current step mode
    step_mode: Option<StepMode>,
    /// Step target frame depth (for step over/out)
    step_frame_depth: Option<usize>,
    /// Current stack frames
    stack_frames: Vec<StackFrame>,
    /// Scopes by frame ID
    scopes: HashMap<u32, Vec<Scope>>,
    /// Variables by reference
    variables: HashMap<u32, Vec<Variable>>,
    /// Next variable reference
    next_var_ref: u32,
    /// Watch expressions
    watches: Vec<String>,
    /// Event queue
    events: Vec<DebugEvent>,
}

impl Debugger {
    pub fn new() -> Self {
        Self {
            state: DebugState::NotStarted,
            breakpoints: HashMap::new(),
            line_breakpoints: HashMap::new(),
            next_bp_id: 1,
            step_mode: None,
            step_frame_depth: None,
            stack_frames: Vec::new(),
            scopes: HashMap::new(),
            variables: HashMap::new(),
            next_var_ref: 1,
            watches: Vec::new(),
            events: Vec::new(),
        }
    }
    
    /// Add a line breakpoint
    pub fn add_breakpoint(&mut self, file: PathBuf, line: u32) -> Breakpoint {
        let id = self.next_bp_id;
        self.next_bp_id += 1;
        
        let bp = Breakpoint::line(id, file.clone(), line);
        
        self.breakpoints.insert(id, bp.clone());
        self.line_breakpoints
            .entry((file, line))
            .or_default()
            .push(id);
        
        bp
    }
    
    /// Add a conditional breakpoint
    pub fn add_conditional_breakpoint(
        &mut self,
        file: PathBuf,
        line: u32,
        condition: String,
    ) -> Breakpoint {
        let id = self.next_bp_id;
        self.next_bp_id += 1;
        
        let bp = Breakpoint::conditional(id, file.clone(), line, condition);
        
        self.breakpoints.insert(id, bp.clone());
        self.line_breakpoints
            .entry((file, line))
            .or_default()
            .push(id);
        
        bp
    }
    
    /// Remove a breakpoint
    pub fn remove_breakpoint(&mut self, id: u32) -> Option<Breakpoint> {
        if let Some(bp) = self.breakpoints.remove(&id) {
            if let Some(ids) = self.line_breakpoints.get_mut(&(bp.file.clone(), bp.line)) {
                ids.retain(|&i| i != id);
            }
            Some(bp)
        } else {
            None
        }
    }
    
    /// Enable/disable a breakpoint
    pub fn set_breakpoint_enabled(&mut self, id: u32, enabled: bool) {
        if let Some(bp) = self.breakpoints.get_mut(&id) {
            bp.enabled = enabled;
        }
    }
    
    /// Get all breakpoints
    pub fn get_breakpoints(&self) -> Vec<&Breakpoint> {
        self.breakpoints.values().collect()
    }
    
    /// Check if we should break at a location
    pub fn should_break(&mut self, file: &PathBuf, line: u32, current_depth: usize) -> bool {
        // Check step mode first
        if let Some(mode) = self.step_mode {
            match mode {
                StepMode::StepInto => return true,
                StepMode::StepOver => {
                    if let Some(target_depth) = self.step_frame_depth {
                        if current_depth <= target_depth {
                            return true;
                        }
                    }
                }
                StepMode::StepOut => {
                    if let Some(target_depth) = self.step_frame_depth {
                        if current_depth < target_depth {
                            return true;
                        }
                    }
                }
                StepMode::StepInstruction => return true,
                StepMode::Continue => {}
            }
        }
        
        // Check breakpoints
        if let Some(bp_ids) = self.line_breakpoints.get(&(file.clone(), line)) {
            for &id in bp_ids {
                if let Some(bp) = self.breakpoints.get_mut(&id) {
                    if bp.enabled {
                        bp.hit();
                        return true;
                    }
                }
            }
        }
        
        false
    }
    
    /// Pause execution
    pub fn pause(&mut self) {
        self.state = DebugState::Paused;
        self.step_mode = None;
    }
    
    /// Continue execution
    pub fn continue_execution(&mut self) {
        self.state = DebugState::Running;
        self.step_mode = Some(StepMode::Continue);
        self.step_frame_depth = None;
    }
    
    /// Step over
    pub fn step_over(&mut self, current_depth: usize) {
        self.state = DebugState::Running;
        self.step_mode = Some(StepMode::StepOver);
        self.step_frame_depth = Some(current_depth);
    }
    
    /// Step into
    pub fn step_into(&mut self) {
        self.state = DebugState::Running;
        self.step_mode = Some(StepMode::StepInto);
        self.step_frame_depth = None;
    }
    
    /// Step out
    pub fn step_out(&mut self, current_depth: usize) {
        self.state = DebugState::Running;
        self.step_mode = Some(StepMode::StepOut);
        self.step_frame_depth = Some(current_depth);
    }
    
    /// Set stack frames
    pub fn set_stack_frames(&mut self, frames: Vec<StackFrame>) {
        self.stack_frames = frames;
    }
    
    /// Get stack frames
    pub fn get_stack_frames(&self) -> &[StackFrame] {
        &self.stack_frames
    }
    
    /// Set scopes for a frame
    pub fn set_scopes(&mut self, frame_id: u32, scopes: Vec<Scope>) {
        self.scopes.insert(frame_id, scopes);
    }
    
    /// Get scopes for a frame
    pub fn get_scopes(&self, frame_id: u32) -> Option<&Vec<Scope>> {
        self.scopes.get(&frame_id)
    }
    
    /// Register variables and get a reference
    pub fn register_variables(&mut self, vars: Vec<Variable>) -> u32 {
        let ref_id = self.next_var_ref;
        self.next_var_ref += 1;
        self.variables.insert(ref_id, vars);
        ref_id
    }
    
    /// Get variables by reference
    pub fn get_variables(&self, ref_id: u32) -> Option<&Vec<Variable>> {
        self.variables.get(&ref_id)
    }
    
    /// Add watch expression
    pub fn add_watch(&mut self, expression: String) {
        if !self.watches.contains(&expression) {
            self.watches.push(expression);
        }
    }
    
    /// Remove watch expression
    pub fn remove_watch(&mut self, expression: &str) {
        self.watches.retain(|w| w != expression);
    }
    
    /// Get watch expressions
    pub fn get_watches(&self) -> &[String] {
        &self.watches
    }
    
    /// Queue an event
    pub fn queue_event(&mut self, event: DebugEvent) {
        self.events.push(event);
    }
    
    /// Get pending events
    pub fn drain_events(&mut self) -> Vec<DebugEvent> {
        std::mem::take(&mut self.events)
    }
    
    /// Get current state
    pub fn state(&self) -> DebugState {
        self.state
    }
    
    /// Convert a Value to a Variable for display
    pub fn value_to_variable(&mut self, name: &str, value: Value) -> Variable {
        if let Some(n) = value.as_int() {
            return Variable::simple(name, n.to_string(), "int");
        }
        
        if let Some(b) = value.as_bool() {
            return Variable::simple(name, b.to_string(), "bool");
        }
        
        if value.is_null() {
            return Variable::simple(name, "null", "null");
        }
        
        // For complex types, we'd need to look up in the heap
        Variable::simple(name, "<value>", "unknown")
    }
}

impl Default for Debugger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_breakpoint_creation() {
        let mut debugger = Debugger::new();
        
        let bp = debugger.add_breakpoint(PathBuf::from("test.hate"), 10);
        
        assert_eq!(bp.id, 1);
        assert_eq!(bp.line, 10);
        assert!(bp.enabled);
    }
    
    #[test]
    fn test_breakpoint_removal() {
        let mut debugger = Debugger::new();
        
        let bp = debugger.add_breakpoint(PathBuf::from("test.hate"), 10);
        let removed = debugger.remove_breakpoint(bp.id);
        
        assert!(removed.is_some());
        assert_eq!(debugger.get_breakpoints().len(), 0);
    }
    
    #[test]
    fn test_should_break() {
        let mut debugger = Debugger::new();
        
        debugger.add_breakpoint(PathBuf::from("test.hate"), 10);
        
        assert!(debugger.should_break(&PathBuf::from("test.hate"), 10, 0));
        assert!(!debugger.should_break(&PathBuf::from("test.hate"), 11, 0));
    }
    
    #[test]
    fn test_step_modes() {
        let mut debugger = Debugger::new();
        
        debugger.step_into();
        assert!(debugger.should_break(&PathBuf::from("any.hate"), 1, 0));
        
        debugger.step_over(2);
        // At same depth, should break
        assert!(debugger.should_break(&PathBuf::from("any.hate"), 1, 2));
        // Deeper, should not break
        assert!(!debugger.should_break(&PathBuf::from("any.hate"), 1, 3));
    }
    
    #[test]
    fn test_variables() {
        let mut debugger = Debugger::new();
        
        let vars = vec![
            Variable::simple("x", "42", "int"),
            Variable::simple("name", "\"hello\"", "string"),
        ];
        
        let ref_id = debugger.register_variables(vars);
        
        let retrieved = debugger.get_variables(ref_id).unwrap();
        assert_eq!(retrieved.len(), 2);
        assert_eq!(retrieved[0].name, "x");
    }
}
