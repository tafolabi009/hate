//! Async/Await Support
//!
//! Implements async functions using CPS transformation:
//! - Async functions are transformed into state machines
//! - Each await point becomes a state
//! - Promises for cooperative multitasking
//! - Microtask queue for promise resolution

use crate::value::Value;
use crate::bytecode::{Chunk, Instruction, OpCode};
use std::collections::VecDeque;

/// State machine states for async functions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncFunctionState {
    /// Initial state, not started
    Created,
    /// Running, not suspended
    Running,
    /// Suspended at an await point
    Suspended { await_point: u32 },
    /// Completed successfully
    Resolved,
    /// Completed with error
    Rejected,
}

/// An async function execution context
#[derive(Debug, Clone)]
pub struct AsyncContext {
    /// Current state
    pub state: AsyncFunctionState,
    /// The function chunk
    pub chunk_idx: usize,
    /// Instruction pointer (saved at suspend)
    pub ip: usize,
    /// Stack snapshot (saved at suspend)
    pub stack: Vec<Value>,
    /// Local variables (saved at suspend)
    pub locals: Vec<Value>,
    /// Result value (when resolved)
    pub result: Value,
    /// Error value (when rejected)
    pub error: Option<String>,
    /// Await point counter
    pub await_counter: u32,
    /// Associated promise index
    pub promise_idx: u32,
}

impl AsyncContext {
    pub fn new(chunk_idx: usize, promise_idx: u32) -> Self {
        Self {
            state: AsyncFunctionState::Created,
            chunk_idx,
            ip: 0,
            stack: Vec::new(),
            locals: Vec::new(),
            result: Value::null(),
            error: None,
            await_counter: 0,
            promise_idx,
        }
    }
    
    /// Suspend at an await point
    pub fn suspend(&mut self, ip: usize, stack: Vec<Value>, locals: Vec<Value>) {
        self.state = AsyncFunctionState::Suspended { 
            await_point: self.await_counter 
        };
        self.ip = ip;
        self.stack = stack;
        self.locals = locals;
        self.await_counter += 1;
    }
    
    /// Resume execution with a value
    pub fn resume(&mut self, value: Value) {
        self.state = AsyncFunctionState::Running;
        self.stack.push(value);
    }
    
    /// Resolve with a final value
    pub fn resolve(&mut self, value: Value) {
        self.state = AsyncFunctionState::Resolved;
        self.result = value;
    }
    
    /// Reject with an error
    pub fn reject(&mut self, error: String) {
        self.state = AsyncFunctionState::Rejected;
        self.error = Some(error);
    }
}

/// Promise states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromiseState {
    Pending,
    Fulfilled,
    Rejected,
}

/// A promise
#[derive(Debug, Clone)]
pub struct Promise {
    /// Current state
    pub state: PromiseState,
    /// Fulfilled value
    pub value: Value,
    /// Rejection reason
    pub reason: Option<String>,
    /// On-fulfill handlers (async context indices)
    pub fulfill_handlers: Vec<u32>,
    /// On-reject handlers (async context indices)
    pub reject_handlers: Vec<u32>,
}

impl Promise {
    pub fn new() -> Self {
        Self {
            state: PromiseState::Pending,
            value: Value::null(),
            reason: None,
            fulfill_handlers: Vec::new(),
            reject_handlers: Vec::new(),
        }
    }
    
    pub fn is_pending(&self) -> bool {
        self.state == PromiseState::Pending
    }
    
    pub fn is_fulfilled(&self) -> bool {
        self.state == PromiseState::Fulfilled
    }
    
    pub fn is_rejected(&self) -> bool {
        self.state == PromiseState::Rejected
    }
}

impl Default for Promise {
    fn default() -> Self {
        Self::new()
    }
}

/// Microtask for the event loop
#[derive(Debug, Clone)]
pub enum Microtask {
    /// Resume an async context with a value
    Resume { context_idx: u32, value: Value },
    /// Reject an async context with an error
    Reject { context_idx: u32, error: String },
    /// Run a then/catch callback
    Callback { callback_idx: u32, value: Value },
}

/// Microtask queue (FIFO)
#[derive(Debug, Default)]
pub struct MicrotaskQueue {
    tasks: VecDeque<Microtask>,
}

impl MicrotaskQueue {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
        }
    }
    
    pub fn enqueue(&mut self, task: Microtask) {
        self.tasks.push_back(task);
    }
    
    pub fn dequeue(&mut self) -> Option<Microtask> {
        self.tasks.pop_front()
    }
    
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
    
    pub fn len(&self) -> usize {
        self.tasks.len()
    }
}

/// Async runtime state
#[derive(Debug, Default)]
pub struct AsyncRuntime {
    /// All async contexts
    contexts: Vec<AsyncContext>,
    /// All promises
    promises: Vec<Promise>,
    /// Microtask queue
    microtasks: MicrotaskQueue,
    /// Free list for context reuse
    free_contexts: Vec<u32>,
    /// Free list for promise reuse
    free_promises: Vec<u32>,
}

impl AsyncRuntime {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a new async context
    pub fn create_context(&mut self, chunk_idx: usize) -> u32 {
        let promise_idx = self.create_promise();
        
        if let Some(idx) = self.free_contexts.pop() {
            self.contexts[idx as usize] = AsyncContext::new(chunk_idx, promise_idx);
            idx
        } else {
            let idx = self.contexts.len() as u32;
            self.contexts.push(AsyncContext::new(chunk_idx, promise_idx));
            idx
        }
    }
    
    /// Get an async context
    pub fn get_context(&self, idx: u32) -> Option<&AsyncContext> {
        self.contexts.get(idx as usize)
    }
    
    /// Get a mutable async context
    pub fn get_context_mut(&mut self, idx: u32) -> Option<&mut AsyncContext> {
        self.contexts.get_mut(idx as usize)
    }
    
    /// Create a new promise
    pub fn create_promise(&mut self) -> u32 {
        if let Some(idx) = self.free_promises.pop() {
            self.promises[idx as usize] = Promise::new();
            idx
        } else {
            let idx = self.promises.len() as u32;
            self.promises.push(Promise::new());
            idx
        }
    }
    
    /// Get a promise
    pub fn get_promise(&self, idx: u32) -> Option<&Promise> {
        self.promises.get(idx as usize)
    }
    
    /// Get a mutable promise
    pub fn get_promise_mut(&mut self, idx: u32) -> Option<&mut Promise> {
        self.promises.get_mut(idx as usize)
    }
    
    /// Fulfill a promise
    pub fn fulfill_promise(&mut self, idx: u32, value: Value) {
        if let Some(promise) = self.promises.get_mut(idx as usize) {
            if promise.state != PromiseState::Pending {
                return; // Already settled
            }
            
            promise.state = PromiseState::Fulfilled;
            promise.value = value;
            
            // Schedule handlers
            for &handler_idx in &promise.fulfill_handlers.clone() {
                self.microtasks.enqueue(Microtask::Resume {
                    context_idx: handler_idx,
                    value,
                });
            }
        }
    }
    
    /// Reject a promise
    pub fn reject_promise(&mut self, idx: u32, reason: String) {
        if let Some(promise) = self.promises.get_mut(idx as usize) {
            if promise.state != PromiseState::Pending {
                return; // Already settled
            }
            
            promise.state = PromiseState::Rejected;
            promise.reason = Some(reason.clone());
            
            // Schedule handlers
            for &handler_idx in &promise.reject_handlers.clone() {
                self.microtasks.enqueue(Microtask::Reject {
                    context_idx: handler_idx,
                    error: reason.clone(),
                });
            }
        }
    }
    
    /// Add a then handler to a promise
    pub fn add_then_handler(&mut self, promise_idx: u32, context_idx: u32) {
        if let Some(promise) = self.promises.get_mut(promise_idx as usize) {
            match promise.state {
                PromiseState::Pending => {
                    promise.fulfill_handlers.push(context_idx);
                }
                PromiseState::Fulfilled => {
                    // Already fulfilled, schedule immediately
                    self.microtasks.enqueue(Microtask::Resume {
                        context_idx,
                        value: promise.value,
                    });
                }
                PromiseState::Rejected => {
                    // Rejected, don't call then handler
                }
            }
        }
    }
    
    /// Add a catch handler to a promise
    pub fn add_catch_handler(&mut self, promise_idx: u32, context_idx: u32) {
        if let Some(promise) = self.promises.get_mut(promise_idx as usize) {
            match promise.state {
                PromiseState::Pending => {
                    promise.reject_handlers.push(context_idx);
                }
                PromiseState::Fulfilled => {
                    // Already fulfilled, don't call catch handler
                }
                PromiseState::Rejected => {
                    // Already rejected, schedule immediately
                    let reason = promise.reason.clone().unwrap_or_default();
                    self.microtasks.enqueue(Microtask::Reject {
                        context_idx,
                        error: reason,
                    });
                }
            }
        }
    }
    
    /// Process one microtask
    pub fn process_microtask(&mut self) -> Option<Microtask> {
        self.microtasks.dequeue()
    }
    
    /// Check if there are pending microtasks
    pub fn has_microtasks(&self) -> bool {
        !self.microtasks.is_empty()
    }
    
    /// Run all microtasks until empty
    pub fn drain_microtasks<F>(&mut self, mut handler: F)
    where
        F: FnMut(&mut Self, Microtask),
    {
        while let Some(task) = self.microtasks.dequeue() {
            handler(self, task);
        }
    }
    
    /// Release an async context for reuse
    pub fn release_context(&mut self, idx: u32) {
        self.free_contexts.push(idx);
    }
    
    /// Release a promise for reuse
    pub fn release_promise(&mut self, idx: u32) {
        self.free_promises.push(idx);
    }
}

/// CPS Transformer for async functions
///
/// Transforms:
/// ```text
/// async fn fetchData() {
///     let a = await fetch(url);
///     let b = await process(a);
///     return b;
/// }
/// ```
///
/// Into a state machine with states:
/// - State 0: Call fetch(url), suspend, return promise
/// - State 1: Receive a, call process(a), suspend
/// - State 2: Receive b, resolve promise with b
pub struct CpsTransformer {
    /// Current state counter
    state_counter: u32,
    /// Await point to state mapping
    await_states: Vec<u32>,
}

impl CpsTransformer {
    pub fn new() -> Self {
        Self {
            state_counter: 0,
            await_states: Vec::new(),
        }
    }
    
    /// Mark an await point
    pub fn mark_await_point(&mut self) -> u32 {
        let state = self.state_counter;
        self.await_states.push(state);
        self.state_counter += 1;
        state
    }
    
    /// Get number of states
    pub fn state_count(&self) -> u32 {
        self.state_counter
    }
    
    /// Generate state machine bytecode for async function
    pub fn generate_state_machine(&self, original: &Chunk) -> Chunk {
        // For now, return a simple state machine structure
        // Full implementation would transform the bytecode
        let chunk = Chunk::new();
        
        // State dispatch table
        // switch(state) {
        //   case 0: goto initial
        //   case 1: goto after_await_1
        //   ...
        // }
        
        // For each await point:
        // 1. Save current state
        // 2. Emit AsyncSuspend with state number
        // 3. After suspend point, emit state resume code
        
        chunk
    }
}

impl Default for CpsTransformer {
    fn default() -> Self {
        Self::new()
    }
}

/// Await expression compiler
pub struct AwaitCompiler {
    /// Whether we're inside an async function
    in_async: bool,
    /// Current async function's context
    context_register: Option<u8>,
}

impl AwaitCompiler {
    pub fn new() -> Self {
        Self {
            in_async: false,
            context_register: None,
        }
    }
    
    pub fn enter_async(&mut self, context_reg: u8) {
        self.in_async = true;
        self.context_register = Some(context_reg);
    }
    
    pub fn exit_async(&mut self) {
        self.in_async = false;
        self.context_register = None;
    }
    
    pub fn is_in_async(&self) -> bool {
        self.in_async
    }
    
    /// Compile an await expression
    /// 
    /// Generated bytecode:
    /// 1. Evaluate the promise expression (already on stack)
    /// 2. Check if promise is already resolved
    /// 3. If resolved, unwrap value and continue
    /// 4. If pending, suspend and register as handler
    pub fn compile_await(&self, chunk: &mut Chunk, promise_reg: u8, result_reg: u8, state: u32, line: u32) {
        // AsyncSuspend: suspend execution and wait for promise
        chunk.write(Instruction {
            opcode: OpCode::AsyncSuspend,
            a: result_reg,
            b: promise_reg,
            c: state as u8,
        }, line);
    }
}

impl Default for AwaitCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_async_context_lifecycle() {
        let mut ctx = AsyncContext::new(0, 0);
        
        assert_eq!(ctx.state, AsyncFunctionState::Created);
        
        ctx.suspend(10, vec![Value::int(1)], vec![Value::int(2)]);
        assert!(matches!(ctx.state, AsyncFunctionState::Suspended { await_point: 0 }));
        
        ctx.resume(Value::int(42));
        assert_eq!(ctx.state, AsyncFunctionState::Running);
        assert_eq!(ctx.stack.len(), 2);
        
        ctx.resolve(Value::int(100));
        assert_eq!(ctx.state, AsyncFunctionState::Resolved);
        assert_eq!(ctx.result.as_int(), Some(100));
    }
    
    #[test]
    fn test_promise_lifecycle() {
        let mut promise = Promise::new();
        
        assert!(promise.is_pending());
        
        promise.state = PromiseState::Fulfilled;
        promise.value = Value::int(42);
        
        assert!(promise.is_fulfilled());
        assert_eq!(promise.value.as_int(), Some(42));
    }
    
    #[test]
    fn test_microtask_queue() {
        let mut queue = MicrotaskQueue::new();
        
        queue.enqueue(Microtask::Resume { context_idx: 0, value: Value::int(1) });
        queue.enqueue(Microtask::Resume { context_idx: 1, value: Value::int(2) });
        
        assert_eq!(queue.len(), 2);
        
        let task1 = queue.dequeue().unwrap();
        assert!(matches!(task1, Microtask::Resume { context_idx: 0, .. }));
        
        let task2 = queue.dequeue().unwrap();
        assert!(matches!(task2, Microtask::Resume { context_idx: 1, .. }));
        
        assert!(queue.is_empty());
    }
    
    #[test]
    fn test_async_runtime_promise_fulfillment() {
        let mut runtime = AsyncRuntime::new();
        
        let promise_idx = runtime.create_promise();
        let context_idx = runtime.create_context(0);
        
        // Add handler before fulfillment
        runtime.add_then_handler(promise_idx, context_idx);
        
        // Fulfill the promise
        runtime.fulfill_promise(promise_idx, Value::int(42));
        
        // Check that microtask was scheduled
        assert!(runtime.has_microtasks());
        
        let task = runtime.process_microtask().unwrap();
        assert!(matches!(task, Microtask::Resume { context_idx: _, value } if value.as_int() == Some(42)));
    }
    
    #[test]
    fn test_async_runtime_already_fulfilled() {
        let mut runtime = AsyncRuntime::new();
        
        let promise_idx = runtime.create_promise();
        runtime.fulfill_promise(promise_idx, Value::int(42));
        
        // Add handler after fulfillment
        let context_idx = runtime.create_context(0);
        runtime.add_then_handler(promise_idx, context_idx);
        
        // Should immediately schedule
        assert!(runtime.has_microtasks());
    }
}
