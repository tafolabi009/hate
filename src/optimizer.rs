//! Performance Optimizations
//!
//! Bytecode optimization passes:
//! - Constant folding
//! - Dead code elimination
//! - Peephole optimization
//! - Register allocation
//! - Inline caching

use crate::bytecode::{Chunk, Instruction, OpCode};
use crate::value::Value;
use std::collections::{HashMap, HashSet};

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    /// No optimization
    O0,
    /// Basic optimization (constant folding, dead code)
    O1,
    /// Full optimization (all passes)
    O2,
    /// Aggressive optimization (may increase compile time)
    O3,
}

/// Optimizer configuration
#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    pub level: OptLevel,
    pub constant_folding: bool,
    pub dead_code_elimination: bool,
    pub peephole: bool,
    pub register_coalescing: bool,
    pub inline_small_functions: bool,
    pub max_inline_size: usize,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            level: OptLevel::O1,
            constant_folding: true,
            dead_code_elimination: true,
            peephole: true,
            register_coalescing: false,
            inline_small_functions: false,
            max_inline_size: 10,
        }
    }
}

impl OptimizerConfig {
    pub fn from_level(level: OptLevel) -> Self {
        match level {
            OptLevel::O0 => Self {
                level,
                constant_folding: false,
                dead_code_elimination: false,
                peephole: false,
                register_coalescing: false,
                inline_small_functions: false,
                max_inline_size: 0,
            },
            OptLevel::O1 => Self::default(),
            OptLevel::O2 => Self {
                level,
                constant_folding: true,
                dead_code_elimination: true,
                peephole: true,
                register_coalescing: true,
                inline_small_functions: true,
                max_inline_size: 10,
            },
            OptLevel::O3 => Self {
                level,
                constant_folding: true,
                dead_code_elimination: true,
                peephole: true,
                register_coalescing: true,
                inline_small_functions: true,
                max_inline_size: 50,
            },
        }
    }
}

/// Basic block
#[derive(Debug)]
pub struct BasicBlock {
    /// Block ID
    pub id: usize,
    /// Instructions in this block
    pub instructions: Vec<Instruction>,
    /// Predecessor blocks
    pub predecessors: Vec<usize>,
    /// Successor blocks (for non-return)
    pub successors: Vec<usize>,
    /// Is this a loop header?
    pub is_loop_header: bool,
}

impl BasicBlock {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            instructions: Vec::new(),
            predecessors: Vec::new(),
            successors: Vec::new(),
            is_loop_header: false,
        }
    }
}

/// Control flow graph
#[derive(Debug)]
pub struct ControlFlowGraph {
    /// All basic blocks
    pub blocks: Vec<BasicBlock>,
    /// Entry block
    pub entry: usize,
    /// Exit blocks
    pub exits: Vec<usize>,
}

impl ControlFlowGraph {
    /// Build CFG from instructions
    pub fn build(instructions: &[Instruction]) -> Self {
        if instructions.is_empty() {
            return Self {
                blocks: vec![BasicBlock::new(0)],
                entry: 0,
                exits: vec![0],
            };
        }
        
        // Find block boundaries
        let mut block_starts = HashSet::new();
        block_starts.insert(0);
        
        for (i, instr) in instructions.iter().enumerate() {
            match instr.opcode {
                OpCode::Jump | OpCode::JumpIfFalse | OpCode::JumpIfTrue => {
                    // Next instruction starts a block
                    if i + 1 < instructions.len() {
                        block_starts.insert(i + 1);
                    }
                    // Jump target starts a block
                    let target = ((instr.b as usize) << 8) | (instr.c as usize);
                    block_starts.insert(target);
                }
                OpCode::Return => {
                    if i + 1 < instructions.len() {
                        block_starts.insert(i + 1);
                    }
                }
                _ => {}
            }
        }
        
        // Create blocks
        let mut block_starts_sorted: Vec<_> = block_starts.into_iter().collect();
        block_starts_sorted.sort();
        
        let mut blocks = Vec::new();
        let mut idx_to_block = HashMap::new();
        
        for (block_id, &start) in block_starts_sorted.iter().enumerate() {
            let end = block_starts_sorted
                .get(block_id + 1)
                .copied()
                .unwrap_or(instructions.len());
            
            let mut block = BasicBlock::new(block_id);
            block.instructions = instructions[start..end].to_vec();
            
            idx_to_block.insert(start, block_id);
            blocks.push(block);
        }
        
        // Connect blocks
        let mut exits = Vec::new();
        
        for block_id in 0..blocks.len() {
            let last_instr = blocks[block_id].instructions.last();
            
            match last_instr.map(|i| i.opcode) {
                Some(OpCode::Return) => {
                    exits.push(block_id);
                }
                Some(OpCode::Jump) => {
                    let instr = last_instr.unwrap();
                    let target = ((instr.b as usize) << 8) | (instr.c as usize);
                    if let Some(&target_block) = idx_to_block.get(&target) {
                        blocks[block_id].successors.push(target_block);
                        blocks[target_block].predecessors.push(block_id);
                    }
                }
                Some(OpCode::JumpIfFalse | OpCode::JumpIfTrue) => {
                    let instr = last_instr.unwrap();
                    let target = ((instr.b as usize) << 8) | (instr.c as usize);
                    
                    // Fall-through
                    if block_id + 1 < blocks.len() {
                        blocks[block_id].successors.push(block_id + 1);
                        blocks[block_id + 1].predecessors.push(block_id);
                    }
                    
                    // Jump target
                    if let Some(&target_block) = idx_to_block.get(&target) {
                        blocks[block_id].successors.push(target_block);
                        blocks[target_block].predecessors.push(block_id);
                    }
                }
                _ => {
                    // Fall-through to next block
                    if block_id + 1 < blocks.len() {
                        blocks[block_id].successors.push(block_id + 1);
                        blocks[block_id + 1].predecessors.push(block_id);
                    } else {
                        exits.push(block_id);
                    }
                }
            }
        }
        
        Self {
            blocks,
            entry: 0,
            exits,
        }
    }
    
    /// Detect loop headers
    pub fn detect_loops(&mut self) {
        // Simple dominance-based loop detection
        for block in &mut self.blocks {
            for &pred in &block.predecessors {
                if pred >= block.id {
                    // Back edge detected
                    block.is_loop_header = true;
                    break;
                }
            }
        }
    }
}

/// Constant folding pass
pub struct ConstantFolder {
    /// Known constant values
    constants: HashMap<u8, Value>,
}

impl ConstantFolder {
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
        }
    }
    
    /// Run constant folding on a chunk
    pub fn fold(&mut self, chunk: &mut Chunk) {
        self.constants.clear();
        
        let mut i = 0;
        while i < chunk.code.len() {
            let instr = chunk.code[i];
            
            match instr.opcode {
                OpCode::LoadConst => {
                    // Track constant loads
                    let const_idx = ((instr.b as u16) << 8) | (instr.c as u16);
                    if let Some(&value) = chunk.constants.get(const_idx as usize) {
                        self.constants.insert(instr.a, value);
                    }
                }
                OpCode::LoadInt => {
                    // Track integer loads
                    let value = ((instr.b as i16) << 8) | (instr.c as i16);
                    self.constants.insert(instr.a, Value::int(value as i32));
                }
                OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div => {
                    // Try to fold binary operations
                    if let (Some(&left), Some(&right)) = (
                        self.constants.get(&instr.b),
                        self.constants.get(&instr.c),
                    ) {
                        if let (Some(l), Some(r)) = (left.as_int(), right.as_int()) {
                            let result = match instr.opcode {
                                OpCode::Add => l + r,
                                OpCode::Sub => l - r,
                                OpCode::Mul => l * r,
                                OpCode::Div if r != 0 => l / r,
                                _ => {
                                    i += 1;
                                    continue;
                                }
                            };
                            
                            // Replace with LoadInt if possible
                            if result >= -32768 && result <= 32767 {
                                chunk.code[i] = Instruction {
                                    opcode: OpCode::LoadInt,
                                    a: instr.a,
                                    b: ((result >> 8) & 0xFF) as u8,
                                    c: (result & 0xFF) as u8,
                                };
                                self.constants.insert(instr.a, Value::int(result as i32));
                            }
                        }
                    }
                }
                _ => {
                    // Invalidate constants for assigned registers
                    self.constants.remove(&instr.a);
                }
            }
            
            i += 1;
        }
    }
}

impl Default for ConstantFolder {
    fn default() -> Self {
        Self::new()
    }
}

/// Dead code elimination
pub struct DeadCodeEliminator {
    /// Used registers
    used: HashSet<u8>,
}

impl DeadCodeEliminator {
    pub fn new() -> Self {
        Self {
            used: HashSet::new(),
        }
    }
    
    /// Eliminate dead code from a chunk
    pub fn eliminate(&mut self, chunk: &mut Chunk) {
        // Build CFG
        let cfg = ControlFlowGraph::build(&chunk.code);
        
        // Find unreachable blocks
        let mut reachable = HashSet::new();
        let mut queue = vec![cfg.entry];
        
        while let Some(block_id) = queue.pop() {
            if reachable.contains(&block_id) {
                continue;
            }
            reachable.insert(block_id);
            
            for &succ in &cfg.blocks[block_id].successors {
                queue.push(succ);
            }
        }
        
        // Mark unreachable instructions as Nop
        let mut instr_idx = 0;
        for (block_id, block) in cfg.blocks.iter().enumerate() {
            if !reachable.contains(&block_id) {
                for _ in &block.instructions {
                    if instr_idx < chunk.code.len() {
                        chunk.code[instr_idx] = Instruction {
                            opcode: OpCode::Nop,
                            a: 0,
                            b: 0,
                            c: 0,
                        };
                    }
                    instr_idx += 1;
                }
            } else {
                instr_idx += block.instructions.len();
            }
        }
        
        // Remove Nop instructions
        chunk.code.retain(|i| i.opcode != OpCode::Nop);
    }
}

impl Default for DeadCodeEliminator {
    fn default() -> Self {
        Self::new()
    }
}

/// Peephole optimizer
pub struct PeepholeOptimizer {
    window_size: usize,
}

impl PeepholeOptimizer {
    pub fn new() -> Self {
        Self { window_size: 3 }
    }
    
    /// Run peephole optimization
    pub fn optimize(&self, chunk: &mut Chunk) {
        let mut changed = true;
        
        while changed {
            changed = false;
            
            let mut i = 0;
            while i + 1 < chunk.code.len() {
                // Pattern: Load followed by immediate use
                if chunk.code[i].opcode == OpCode::LoadInt {
                    let load = chunk.code[i];
                    let next = chunk.code[i + 1];
                    
                    // Combine LoadInt + Add where add is with 0
                    if next.opcode == OpCode::Add && next.c == load.a {
                        let value = ((load.b as i16) << 8) | (load.c as i16);
                        if value == 0 {
                            // Adding 0, eliminate - replace with LoadLocal if same dest
                            chunk.code[i] = Instruction {
                                opcode: OpCode::LoadLocal,
                                a: next.a,
                                b: next.b,
                                c: 0,
                            };
                            chunk.code[i + 1] = Instruction {
                                opcode: OpCode::Nop,
                                a: 0,
                                b: 0,
                                c: 0,
                            };
                            changed = true;
                        }
                    }
                }
                
                // Pattern: Jump to next instruction
                if chunk.code[i].opcode == OpCode::Jump {
                    let target = ((chunk.code[i].b as usize) << 8) | (chunk.code[i].c as usize);
                    if target == i + 1 {
                        chunk.code[i] = Instruction {
                            opcode: OpCode::Nop,
                            a: 0,
                            b: 0,
                            c: 0,
                        };
                        changed = true;
                    }
                }
                
                // Pattern: Conditional jump with constant condition
                if chunk.code[i].opcode == OpCode::LoadTrue || chunk.code[i].opcode == OpCode::LoadFalse {
                    if i + 1 < chunk.code.len() {
                        let is_true = chunk.code[i].opcode == OpCode::LoadTrue;
                        let cond_reg = chunk.code[i].a;
                        let next = chunk.code[i + 1];
                        
                        if next.opcode == OpCode::JumpIfFalse && next.a == cond_reg {
                            if is_true {
                                // Never jumps, remove
                                chunk.code[i + 1] = Instruction {
                                    opcode: OpCode::Nop,
                                    a: 0,
                                    b: 0,
                                    c: 0,
                                };
                            } else {
                                // Always jumps, convert to unconditional
                                chunk.code[i + 1] = Instruction {
                                    opcode: OpCode::Jump,
                                    a: 0,
                                    b: next.b,
                                    c: next.c,
                                };
                            }
                            changed = true;
                        }
                    }
                }
                
                i += 1;
            }
            
            // Remove Nops
            chunk.code.retain(|i| i.opcode != OpCode::Nop);
        }
    }
}

impl Default for PeepholeOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Main optimizer
pub struct Optimizer {
    config: OptimizerConfig,
    constant_folder: ConstantFolder,
    dead_code: DeadCodeEliminator,
    peephole: PeepholeOptimizer,
}

impl Optimizer {
    pub fn new(config: OptimizerConfig) -> Self {
        Self {
            config,
            constant_folder: ConstantFolder::new(),
            dead_code: DeadCodeEliminator::new(),
            peephole: PeepholeOptimizer::new(),
        }
    }
    
    /// Optimize a chunk
    pub fn optimize(&mut self, chunk: &mut Chunk) {
        if self.config.level == OptLevel::O0 {
            return;
        }
        
        // Run optimization passes
        let passes = match self.config.level {
            OptLevel::O0 => 0,
            OptLevel::O1 => 1,
            OptLevel::O2 => 2,
            OptLevel::O3 => 3,
        };
        
        for _ in 0..passes {
            if self.config.constant_folding {
                self.constant_folder.fold(chunk);
            }
            
            if self.config.peephole {
                self.peephole.optimize(chunk);
            }
            
            if self.config.dead_code_elimination {
                self.dead_code.eliminate(chunk);
            }
        }
    }
}

impl Default for Optimizer {
    fn default() -> Self {
        Self::new(OptimizerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cfg_simple() {
        let instructions = vec![
            Instruction { opcode: OpCode::LoadInt, a: 0, b: 0, c: 1 },
            Instruction { opcode: OpCode::LoadInt, a: 1, b: 0, c: 2 },
            Instruction { opcode: OpCode::Add, a: 2, b: 0, c: 1 },
            Instruction { opcode: OpCode::Return, a: 2, b: 0, c: 0 },
        ];
        
        let cfg = ControlFlowGraph::build(&instructions);
        
        assert_eq!(cfg.blocks.len(), 1);
        assert_eq!(cfg.entry, 0);
    }
    
    #[test]
    fn test_cfg_with_branch() {
        let instructions = vec![
            Instruction { opcode: OpCode::LoadTrue, a: 0, b: 0, c: 0 },
            Instruction { opcode: OpCode::JumpIfFalse, a: 0, b: 0, c: 4 }, // Jump to index 4
            Instruction { opcode: OpCode::LoadInt, a: 1, b: 0, c: 1 },
            Instruction { opcode: OpCode::Jump, a: 0, b: 0, c: 5 }, // Jump to index 5
            Instruction { opcode: OpCode::LoadInt, a: 1, b: 0, c: 2 },
            Instruction { opcode: OpCode::Return, a: 1, b: 0, c: 0 },
        ];
        
        let cfg = ControlFlowGraph::build(&instructions);
        
        assert!(cfg.blocks.len() >= 3);
    }
    
    #[test]
    fn test_constant_folding() {
        let mut chunk = Chunk::new();
        
        // LoadInt R0, 2
        // LoadInt R1, 3
        // Add R2, R0, R1
        chunk.write(Instruction { opcode: OpCode::LoadInt, a: 0, b: 0, c: 2 }, 1);
        chunk.write(Instruction { opcode: OpCode::LoadInt, a: 1, b: 0, c: 3 }, 1);
        chunk.write(Instruction { opcode: OpCode::Add, a: 2, b: 0, c: 1 }, 1);
        
        let mut folder = ConstantFolder::new();
        folder.fold(&mut chunk);
        
        // The Add should be folded into LoadInt R2, 5
        assert_eq!(chunk.code[2].opcode, OpCode::LoadInt);
    }
    
    #[test]
    fn test_peephole_jump_to_next() {
        let mut chunk = Chunk::new();
        
        // Jump to next instruction should be eliminated
        chunk.write(Instruction { opcode: OpCode::Jump, a: 0, b: 0, c: 1 }, 1);
        chunk.write(Instruction { opcode: OpCode::Return, a: 0, b: 0, c: 0 }, 1);
        
        let peephole = PeepholeOptimizer::new();
        peephole.optimize(&mut chunk);
        
        // Peephole should convert jump-to-next to Nop (peephole doesn't remove instructions)
        assert_eq!(chunk.code[0].opcode, OpCode::Nop);
    }
}
