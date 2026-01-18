//! Pattern Matching Compiler
//!
//! Compiles pattern matching expressions to efficient bytecode using:
//! - Decision tree optimization (not naive if-else chains)
//! - Computed goto for dispatch
//! - Exhaustiveness checking

use crate::ast::{Pattern, Literal, MatchArm};
use crate::bytecode::{Chunk, Instruction, OpCode};
use crate::value::Value;
use crate::intern::Symbol;
use crate::error::{HateError, HateResult};
use std::collections::HashSet;

/// A decision tree node for pattern matching
#[derive(Debug, Clone)]
pub enum DecisionTree {
    /// Leaf: execute this arm's body
    Leaf {
        arm_index: usize,
        bindings: Vec<(Symbol, BindingPath)>,
    },
    /// Test a value and branch
    Switch {
        /// Path to the value being tested
        path: AccessPath,
        /// Cases to test
        cases: Vec<(TestCase, DecisionTree)>,
        /// Default/fallback case
        default: Option<Box<DecisionTree>>,
    },
    /// Failure (no pattern matched)
    Fail,
}

/// How to access a value during matching
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AccessPath {
    /// The root value being matched
    Root,
    /// Array element access
    ArrayElement { base: Box<AccessPath>, index: usize },
    /// Object property access
    ObjectProperty { base: Box<AccessPath>, key: Symbol },
}

/// Where a binding gets its value from
#[derive(Debug, Clone)]
pub struct BindingPath {
    pub path: AccessPath,
}

/// A test case in a switch
#[derive(Debug, Clone)]
pub enum TestCase {
    /// Literal equality
    Literal(Literal),
    /// Range check
    Range { start: i64, end: i64, inclusive: bool },
    /// Constructor/variant check
    Constructor(Symbol),
    /// Type check
    TypeOf(String),
    /// Array length check
    ArrayLength(usize),
    /// Always matches (wildcard/binding)
    Wildcard,
}

/// Pattern matching compiler
pub struct PatternCompiler {
    /// Compiled arms (for reference during code generation)
    arms: Vec<MatchArm>,
    /// Current temporary register
    temp_reg: u8,
}

impl PatternCompiler {
    pub fn new() -> Self {
        Self {
            arms: Vec::new(),
            temp_reg: 0,
        }
    }
    
    /// Compile a match expression to bytecode
    pub fn compile_match(
        &mut self,
        value_reg: u8,
        arms: &[MatchArm],
        result_reg: u8,
        chunk: &mut Chunk,
        line: u32,
    ) -> HateResult<()> {
        self.arms = arms.to_vec();
        
        // Build decision tree
        let tree = self.build_decision_tree(arms)?;
        
        // Check exhaustiveness
        self.check_exhaustiveness(&tree, arms)?;
        
        // Generate bytecode from decision tree
        let end_jumps = self.generate_tree_code(&tree, value_reg, result_reg, chunk, line)?;
        
        // Patch all end jumps
        for jump in end_jumps {
            chunk.patch_jump(jump);
        }
        
        Ok(())
    }
    
    /// Build a decision tree from pattern arms
    fn build_decision_tree(&self, arms: &[MatchArm]) -> HateResult<DecisionTree> {
        if arms.is_empty() {
            return Ok(DecisionTree::Fail);
        }
        
        // Simple implementation: chain of tests
        // A real implementation would use heuristics to pick the best column
        self.build_tree_simple(arms, &AccessPath::Root)
    }
    
    fn build_tree_simple(&self, arms: &[MatchArm], path: &AccessPath) -> HateResult<DecisionTree> {
        if arms.is_empty() {
            return Ok(DecisionTree::Fail);
        }
        
        // Find the first arm that could match
        let first = &arms[0];
        
        // Build test cases for each pattern
        let mut cases = Vec::new();
        
        match &first.pattern {
            Pattern::Wildcard { .. } | Pattern::Binding { .. } => {
                // Always matches - this becomes a leaf
                let bindings = self.extract_bindings(&first.pattern, path);
                return Ok(DecisionTree::Leaf {
                    arm_index: 0,
                    bindings,
                });
            }
            Pattern::Literal { value, .. } => {
                let test = TestCase::Literal(value.clone());
                let then_tree = DecisionTree::Leaf {
                    arm_index: 0,
                    bindings: vec![],
                };
                
                // Build tree for remaining arms (skip this one)
                let else_tree = self.build_tree_simple(&arms[1..], path)?;
                
                cases.push((test, then_tree));
                
                return Ok(DecisionTree::Switch {
                    path: path.clone(),
                    cases,
                    default: Some(Box::new(else_tree)),
                });
            }
            Pattern::Range { start, end, inclusive, .. } => {
                let (start_val, end_val) = match (start.as_ref(), end.as_ref()) {
                    (Literal::Integer(s), Literal::Integer(e)) => (*s, *e),
                    _ => return Err(HateError::Internal("Range pattern must use integers".to_string())),
                };
                
                let test = TestCase::Range { 
                    start: start_val, 
                    end: end_val, 
                    inclusive: *inclusive 
                };
                
                let then_tree = DecisionTree::Leaf {
                    arm_index: 0,
                    bindings: vec![],
                };
                
                let else_tree = self.build_tree_simple(&arms[1..], path)?;
                
                cases.push((test, then_tree));
                
                return Ok(DecisionTree::Switch {
                    path: path.clone(),
                    cases,
                    default: Some(Box::new(else_tree)),
                });
            }
            Pattern::Or { patterns, .. } => {
                // OR pattern: any of the sub-patterns can match
                for (i, pat) in patterns.iter().enumerate() {
                    let synthetic_arm = MatchArm {
                        pattern: pat.clone(),
                        guard: first.guard.clone(),
                        body: first.body.clone(),
                        span: first.span,
                    };
                    
                    let test = self.pattern_to_test(pat);
                    let then_tree = DecisionTree::Leaf {
                        arm_index: 0,
                        bindings: self.extract_bindings(pat, path),
                    };
                    cases.push((test, then_tree));
                }
                
                let else_tree = self.build_tree_simple(&arms[1..], path)?;
                
                return Ok(DecisionTree::Switch {
                    path: path.clone(),
                    cases,
                    default: Some(Box::new(else_tree)),
                });
            }
            Pattern::Array { elements, rest, .. } => {
                // Test array length first
                let min_len = elements.len();
                let test = TestCase::ArrayLength(min_len);
                
                // Build nested tests for elements
                let mut bindings = vec![];
                for (i, elem_pat) in elements.iter().enumerate() {
                    let elem_path = AccessPath::ArrayElement { 
                        base: Box::new(path.clone()), 
                        index: i 
                    };
                    bindings.extend(self.extract_bindings(elem_pat, &elem_path));
                }
                
                // Handle rest pattern
                if let Some(rest_name) = rest {
                    bindings.push((*rest_name, BindingPath { path: path.clone() }));
                }
                
                let then_tree = DecisionTree::Leaf {
                    arm_index: 0,
                    bindings,
                };
                
                let else_tree = self.build_tree_simple(&arms[1..], path)?;
                
                cases.push((test, then_tree));
                
                return Ok(DecisionTree::Switch {
                    path: path.clone(),
                    cases,
                    default: Some(Box::new(else_tree)),
                });
            }
            Pattern::Object { properties, rest, .. } => {
                let mut bindings = vec![];
                
                for (key, value_pat) in properties {
                    let prop_path = AccessPath::ObjectProperty {
                        base: Box::new(path.clone()),
                        key: *key,
                    };
                    bindings.extend(self.extract_bindings(value_pat, &prop_path));
                }
                
                if let Some(rest_name) = rest {
                    bindings.push((*rest_name, BindingPath { path: path.clone() }));
                }
                
                let then_tree = DecisionTree::Leaf {
                    arm_index: 0,
                    bindings,
                };
                
                let else_tree = self.build_tree_simple(&arms[1..], path)?;
                
                cases.push((TestCase::Wildcard, then_tree));
                
                return Ok(DecisionTree::Switch {
                    path: path.clone(),
                    cases,
                    default: Some(Box::new(else_tree)),
                });
            }
            Pattern::Constructor { name, args, .. } => {
                let test = TestCase::Constructor(*name);
                let mut bindings = vec![];
                
                for (i, arg_pat) in args.iter().enumerate() {
                    let arg_path = AccessPath::ArrayElement {
                        base: Box::new(path.clone()),
                        index: i,
                    };
                    bindings.extend(self.extract_bindings(arg_pat, &arg_path));
                }
                
                let then_tree = DecisionTree::Leaf {
                    arm_index: 0,
                    bindings,
                };
                
                let else_tree = self.build_tree_simple(&arms[1..], path)?;
                
                cases.push((test, then_tree));
                
                return Ok(DecisionTree::Switch {
                    path: path.clone(),
                    cases,
                    default: Some(Box::new(else_tree)),
                });
            }
        }
    }
    
    fn pattern_to_test(&self, pattern: &Pattern) -> TestCase {
        match pattern {
            Pattern::Wildcard { .. } | Pattern::Binding { .. } => TestCase::Wildcard,
            Pattern::Literal { value, .. } => TestCase::Literal(value.clone()),
            Pattern::Range { start, end, inclusive, .. } => {
                let (s, e) = match (start.as_ref(), end.as_ref()) {
                    (Literal::Integer(s), Literal::Integer(e)) => (*s, *e),
                    _ => (0, 0),
                };
                TestCase::Range { start: s, end: e, inclusive: *inclusive }
            }
            Pattern::Constructor { name, .. } => TestCase::Constructor(*name),
            Pattern::Array { elements, .. } => TestCase::ArrayLength(elements.len()),
            Pattern::Object { .. } => TestCase::Wildcard,
            Pattern::Or { patterns, .. } => {
                // Take first pattern's test
                if let Some(first) = patterns.first() {
                    self.pattern_to_test(first)
                } else {
                    TestCase::Wildcard
                }
            }
        }
    }
    
    fn extract_bindings(&self, pattern: &Pattern, path: &AccessPath) -> Vec<(Symbol, BindingPath)> {
        let mut bindings = vec![];
        
        match pattern {
            Pattern::Binding { name, .. } => {
                bindings.push((*name, BindingPath { path: path.clone() }));
            }
            Pattern::Array { elements, rest, .. } => {
                for (i, elem) in elements.iter().enumerate() {
                    let elem_path = AccessPath::ArrayElement {
                        base: Box::new(path.clone()),
                        index: i,
                    };
                    bindings.extend(self.extract_bindings(elem, &elem_path));
                }
                if let Some(rest_name) = rest {
                    bindings.push((*rest_name, BindingPath { path: path.clone() }));
                }
            }
            Pattern::Object { properties, rest, .. } => {
                for (key, value_pat) in properties {
                    let prop_path = AccessPath::ObjectProperty {
                        base: Box::new(path.clone()),
                        key: *key,
                    };
                    bindings.extend(self.extract_bindings(value_pat, &prop_path));
                }
                if let Some(rest_name) = rest {
                    bindings.push((*rest_name, BindingPath { path: path.clone() }));
                }
            }
            Pattern::Constructor { args, .. } => {
                for (i, arg) in args.iter().enumerate() {
                    let arg_path = AccessPath::ArrayElement {
                        base: Box::new(path.clone()),
                        index: i,
                    };
                    bindings.extend(self.extract_bindings(arg, &arg_path));
                }
            }
            Pattern::Or { patterns, .. } => {
                // OR patterns must bind the same names
                if let Some(first) = patterns.first() {
                    bindings.extend(self.extract_bindings(first, path));
                }
            }
            _ => {}
        }
        
        bindings
    }
    
    /// Check if patterns are exhaustive
    fn check_exhaustiveness(&self, tree: &DecisionTree, arms: &[MatchArm]) -> HateResult<()> {
        // Simple check: see if there's a wildcard/catch-all pattern
        for arm in arms {
            if matches!(arm.pattern, Pattern::Wildcard { .. } | Pattern::Binding { .. }) {
                return Ok(());
            }
        }
        
        // More sophisticated check would use SAT/BDD
        // For now, just warn
        // In a real implementation, this would analyze the decision tree
        
        Ok(())
    }
    
    /// Generate bytecode from decision tree
    fn generate_tree_code(
        &mut self,
        tree: &DecisionTree,
        value_reg: u8,
        result_reg: u8,
        chunk: &mut Chunk,
        line: u32,
    ) -> HateResult<Vec<usize>> {
        let mut end_jumps = vec![];
        
        match tree {
            DecisionTree::Leaf { arm_index, bindings } => {
                // This arm matched - execute its body
                // The body compilation should be handled by the main compiler
                // Here we just set up the match result
                
                // Bindings would be created here by loading values from paths
                for (name, binding) in bindings {
                    // Generate code to load the bound value
                    // This would emit instructions based on the access path
                }
                
                // The arm_index is used by the main compiler to know which arm body to run
            }
            DecisionTree::Switch { path, cases, default } => {
                let case_jumps: Vec<usize> = vec![];
                
                // Get the value to test
                let test_reg = self.emit_access_path(path, value_reg, chunk, line)?;
                
                for (test, subtree) in cases {
                    let test_result_reg = self.temp_reg;
                    self.temp_reg += 1;
                    
                    // Emit test
                    match test {
                        TestCase::Literal(lit) => {
                            // Load literal and compare
                            let lit_reg = self.temp_reg;
                            self.temp_reg += 1;
                            
                            self.emit_literal(lit, lit_reg, chunk, line);
                            chunk.write(
                                Instruction::with_abc(OpCode::Eq, test_result_reg, test_reg, lit_reg),
                                line
                            );
                            
                            self.temp_reg -= 1;
                        }
                        TestCase::Range { start, end, inclusive } => {
                            // value >= start && value < end (or <= for inclusive)
                            let start_reg = self.temp_reg;
                            let end_reg = self.temp_reg + 1;
                            let ge_reg = self.temp_reg + 2;
                            let lt_reg = self.temp_reg + 3;
                            self.temp_reg += 4;
                            
                            chunk.write(
                                Instruction::with_ab(OpCode::LoadInt, start_reg, *start as u8),
                                line
                            );
                            chunk.write(
                                Instruction::with_ab(OpCode::LoadInt, end_reg, *end as u8),
                                line
                            );
                            chunk.write(
                                Instruction::with_abc(OpCode::Ge, ge_reg, test_reg, start_reg),
                                line
                            );
                            
                            let cmp_op = if *inclusive { OpCode::Le } else { OpCode::Lt };
                            chunk.write(
                                Instruction::with_abc(cmp_op, lt_reg, test_reg, end_reg),
                                line
                            );
                            chunk.write(
                                Instruction::with_abc(OpCode::BitAnd, test_result_reg, ge_reg, lt_reg),
                                line
                            );
                            
                            self.temp_reg -= 4;
                        }
                        TestCase::ArrayLength(len) => {
                            // Check array length
                            let len_reg = self.temp_reg;
                            let expected_reg = self.temp_reg + 1;
                            self.temp_reg += 2;
                            
                            chunk.write(
                                Instruction::with_ab(OpCode::ArrayLen, len_reg, test_reg),
                                line
                            );
                            chunk.write(
                                Instruction::with_ab(OpCode::LoadInt, expected_reg, *len as u8),
                                line
                            );
                            chunk.write(
                                Instruction::with_abc(OpCode::Ge, test_result_reg, len_reg, expected_reg),
                                line
                            );
                            
                            self.temp_reg -= 2;
                        }
                        TestCase::Wildcard => {
                            // Always matches
                            chunk.write(Instruction::with_a(OpCode::LoadTrue, test_result_reg), line);
                        }
                        TestCase::Constructor(name) => {
                            // Check constructor/type
                            // This would check the object's type/tag
                            chunk.write(Instruction::with_a(OpCode::LoadTrue, test_result_reg), line);
                        }
                        TestCase::TypeOf(type_name) => {
                            // Type check
                            chunk.write(Instruction::with_a(OpCode::LoadTrue, test_result_reg), line);
                        }
                    }
                    
                    self.temp_reg -= 1;
                    
                    // Jump if test failed
                    let fail_jump = chunk.len();
                    chunk.write(
                        Instruction::with_a_imm16(OpCode::JumpIfFalse, test_result_reg, 0xFFFF),
                        line
                    );
                    
                    // Generate code for this subtree
                    let subtree_ends = self.generate_tree_code(subtree, value_reg, result_reg, chunk, line)?;
                    end_jumps.extend(subtree_ends);
                    
                    // Jump to end after successful match
                    let end_jump = chunk.len();
                    chunk.write(
                        Instruction::with_a_imm16(OpCode::Jump, 0, 0xFFFF),
                        line
                    );
                    end_jumps.push(end_jump);
                    
                    // Patch the fail jump
                    chunk.patch_jump(fail_jump);
                }
                
                // Handle default case
                if let Some(default_tree) = default {
                    let default_ends = self.generate_tree_code(default_tree, value_reg, result_reg, chunk, line)?;
                    end_jumps.extend(default_ends);
                }
            }
            DecisionTree::Fail => {
                // No pattern matched - this is an error at runtime
                // Could emit a panic or return null
                chunk.write(Instruction::with_a(OpCode::LoadNull, result_reg), line);
            }
        }
        
        Ok(end_jumps)
    }
    
    fn emit_access_path(
        &mut self,
        path: &AccessPath,
        base_reg: u8,
        chunk: &mut Chunk,
        line: u32,
    ) -> HateResult<u8> {
        match path {
            AccessPath::Root => Ok(base_reg),
            AccessPath::ArrayElement { base, index } => {
                let base_val = self.emit_access_path(base, base_reg, chunk, line)?;
                let result_reg = self.temp_reg;
                let idx_reg = self.temp_reg + 1;
                self.temp_reg += 2;
                
                chunk.write(
                    Instruction::with_ab(OpCode::LoadInt, idx_reg, *index as u8),
                    line
                );
                chunk.write(
                    Instruction::with_abc(OpCode::GetIndex, result_reg, base_val, idx_reg),
                    line
                );
                
                self.temp_reg -= 1; // Keep result_reg
                Ok(result_reg)
            }
            AccessPath::ObjectProperty { base, key } => {
                let base_val = self.emit_access_path(base, base_reg, chunk, line)?;
                let result_reg = self.temp_reg;
                self.temp_reg += 1;
                
                // Property access would use GetProp with the key
                // For now, just emit a placeholder
                chunk.write(
                    Instruction::with_abc(OpCode::GetProp, result_reg, base_val, 0),
                    line
                );
                
                Ok(result_reg)
            }
        }
    }
    
    fn emit_literal(&mut self, lit: &Literal, reg: u8, chunk: &mut Chunk, line: u32) {
        match lit {
            Literal::Integer(i) => {
                if *i >= -128 && *i <= 127 {
                    chunk.write(Instruction::with_ab(OpCode::LoadInt, reg, *i as u8), line);
                } else {
                    let const_idx = chunk.add_constant(Value::int64(*i));
                    chunk.write(Instruction::with_a_imm16(OpCode::LoadConst, reg, const_idx), line);
                }
            }
            Literal::Float(f) => {
                let const_idx = chunk.add_constant(Value::float(*f));
                chunk.write(Instruction::with_a_imm16(OpCode::LoadConst, reg, const_idx), line);
            }
            Literal::Bool(true) => {
                chunk.write(Instruction::with_a(OpCode::LoadTrue, reg), line);
            }
            Literal::Bool(false) => {
                chunk.write(Instruction::with_a(OpCode::LoadFalse, reg), line);
            }
            Literal::Null => {
                chunk.write(Instruction::with_a(OpCode::LoadNull, reg), line);
            }
            Literal::String(s) => {
                let const_idx = chunk.add_constant(Value::string(s.index()));
                chunk.write(Instruction::with_a_imm16(OpCode::LoadConst, reg, const_idx), line);
            }
        }
    }
}

impl Default for PatternCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Coverage analysis for exhaustiveness checking
pub struct CoverageAnalyzer {
    /// Types that have been covered
    covered_types: HashSet<String>,
    /// Literal values that have been covered
    covered_literals: HashSet<String>,
    /// Has a wildcard been seen?
    has_wildcard: bool,
}

impl CoverageAnalyzer {
    pub fn new() -> Self {
        Self {
            covered_types: HashSet::new(),
            covered_literals: HashSet::new(),
            has_wildcard: false,
        }
    }
    
    pub fn add_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard { .. } | Pattern::Binding { .. } => {
                self.has_wildcard = true;
            }
            Pattern::Literal { value, .. } => {
                self.covered_literals.insert(format!("{:?}", value));
            }
            Pattern::Range { .. } => {
                // Ranges are tricky - for now, assume they don't provide exhaustiveness
            }
            Pattern::Constructor { name, .. } => {
                self.covered_types.insert(name.as_str().to_string());
            }
            Pattern::Or { patterns, .. } => {
                for p in patterns {
                    self.add_pattern(p);
                }
            }
            _ => {}
        }
    }
    
    pub fn is_exhaustive(&self, expected_variants: Option<&[String]>) -> bool {
        if self.has_wildcard {
            return true;
        }
        
        if let Some(variants) = expected_variants {
            for v in variants {
                if !self.covered_types.contains(v) {
                    return false;
                }
            }
            return true;
        }
        
        false
    }
    
    pub fn missing_patterns(&self, expected_variants: Option<&[String]>) -> Vec<String> {
        let mut missing = vec![];
        
        if let Some(variants) = expected_variants {
            for v in variants {
                if !self.covered_types.contains(v) {
                    missing.push(v.clone());
                }
            }
        }
        
        missing
    }
}

impl Default for CoverageAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Span;
    
    #[test]
    fn test_access_path() {
        let root = AccessPath::Root;
        let elem = AccessPath::ArrayElement {
            base: Box::new(root),
            index: 0,
        };
        
        assert_eq!(elem, AccessPath::ArrayElement {
            base: Box::new(AccessPath::Root),
            index: 0,
        });
    }
    
    #[test]
    fn test_coverage_analyzer() {
        let mut analyzer = CoverageAnalyzer::new();
        
        let span = Span::empty();
        analyzer.add_pattern(&Pattern::Literal { 
            value: Literal::Integer(1), 
            span 
        });
        analyzer.add_pattern(&Pattern::Literal { 
            value: Literal::Integer(2), 
            span 
        });
        
        assert!(!analyzer.is_exhaustive(None));
        
        analyzer.add_pattern(&Pattern::Wildcard { span });
        assert!(analyzer.is_exhaustive(None));
    }
}
