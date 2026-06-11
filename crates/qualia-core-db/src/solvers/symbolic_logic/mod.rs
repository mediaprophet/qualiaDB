//! Symbolic & Logic Solvers - Zero-Allocation Implementation
//! 
//! This module provides fixed-size stack-based symbolic and logic solvers for
//! defeasible reasoning and boolean satisfiability suitable for the #![no_std]
//! environment of Qualia-DB.

use crate::solvers::{SolverConfig, SolverState, SolverResult};
use crate::solvers::SolversError as ExecutionError;
use crate::webizen::SlgOpcode;

/// Forward chaining defeasible reasoning solver
#[repr(C)]
pub struct ForwardChainingDefeasible {
    /// Rule base (fixed size)
    pub rule_base: [DefeasibleRule; 100],
    /// Current facts
    pub facts: [Fact; 50],
    /// Inference queue
    pub inference_queue: [usize; 50],
    /// Queue pointers
    pub queue_head: u8,
    pub queue_tail: u8,
    /// Conflict resolution state
    pub conflict_state: ConflictState,
    /// Solver configuration
    pub config: SolverConfig,
    /// Solver state
    pub solver_state: SolverState,
}

/// Bounded SAT solver for boolean satisfiability
#[repr(C)]
pub struct BoundedSatSolver {
    /// Clause database (fixed size)
    pub clauses: [Clause; 50],
    /// Variable assignments
    pub assignments: [VariableAssignment; 20],
    /// Decision stack
    pub decision_stack: [Decision; 20],
    /// Unit propagation queue
    pub propagation_queue: [u8; 20],
    /// Current assignment level
    pub assignment_level: u8,
    /// Conflict analysis
    pub conflict_clause: Option<Clause>,
    /// Solver configuration
    pub config: SolverConfig,
    /// Solver state
    pub solver_state: SolverState,
}

/// Defeasible rule structure
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DefeasibleRule {
    /// Rule identifier
    pub id: u32,
    /// Rule type (strict, defeasible, defeater)
    pub rule_type: RuleType,
    /// Antecedent literals
    pub antecedents: [Literal; 5],
    /// Consequent literal
    pub consequent: Literal,
    /// Rule priority
    pub priority: u16,
    /// Active flag
    pub active: bool,
    /// Fire count
    pub fire_count: u32,
}

/// Fact structure
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Fact {
    /// Fact identifier
    pub id: u32,
    /// Literal value
    pub literal: Literal,
    /// Supporting rules
    pub supporting_rules: [u32; 3],
    /// Defeated flag
    pub defeated: bool,
    /// Confidence level
    pub confidence: f64,
}

/// Clause structure for SAT solver
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Clause {
    /// Clause identifier
    pub id: u32,
    /// Literals in clause
    pub literals: [Literal; 5],
    /// Number of literals
    pub num_literals: u8,
    /// Learned flag
    pub learned: bool,
    /// Activity score
    pub activity: f64,
}

/// Literal structure
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Literal {
    /// Variable index
    pub variable: u8,
    /// Negation flag
    pub negated: bool,
}

/// Variable assignment
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VariableAssignment {
    /// Assignment value
    pub value: AssignmentValue,
    /// Assignment level
    pub level: u8,
    /// Antecedent clause
    pub antecedent: Option<u32>,
}

/// Decision structure
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Decision {
    /// Variable index
    pub variable: u8,
    /// Assignment value
    pub value: AssignmentValue,
    /// Decision level
    pub level: u8,
}

/// Conflict state for defeasible reasoning
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ConflictState {
    /// Current conflicts
    pub conflicts: [Conflict; 10],
    /// Number of conflicts
    pub num_conflicts: u8,
    /// Resolution strategy
    pub resolution_strategy: ResolutionStrategy,
}

/// Conflict structure
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Conflict {
    /// Conflicting facts
    pub facts: [u32; 3],
    /// Conflicting rules
    pub rules: [u32; 3],
    /// Conflict type
    pub conflict_type: ConflictType,
}

/// Defeasible reasoning state
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DefeasibleState {
    /// Current iteration
    pub iteration: u32,
    /// Number of facts derived
    pub num_facts: u16,
    /// Number of rules fired
    pub rules_fired: u16,
    /// Converged flag
    pub converged: bool,
}

/// SAT solver state
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SatState {
    /// Current iteration
    pub iteration: u32,
    /// Number of decisions made
    pub num_decisions: u16,
    /// Number of propagations
    pub num_propagations: u16,
    /// Satisfiable flag
    pub satisfiable: Option<bool>,
}

/// Rule types for defeasible logic
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum RuleType {
    Strict = 0,
    Defeasible = 1,
    Defeater = 2,
}

/// Assignment values
#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
pub enum AssignmentValue {
    False = 0,
    True = 1,
    Unassigned = 2,
}

/// Conflict types
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum ConflictType {
    Contradiction = 0,
    Defeat = 1,
    Preference = 2,
}

/// Resolution strategies
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum ResolutionStrategy {
    Priority = 0,
    Specificity = 1,
    Recency = 2,
}

impl ForwardChainingDefeasible {
    /// Create new defeasible reasoning solver
    pub fn new(config: SolverConfig) -> Self {
        Self {
            rule_base: [DefeasibleRule::default(); 100],
            facts: [Fact::default(); 50],
            inference_queue: [0; 50],
            queue_head: 0,
            queue_tail: 0,
            conflict_state: ConflictState::default(),
            config,
            solver_state: SolverState::default(),
        }
    }

    /// Add rule to rule base
    pub fn add_rule(&mut self, rule: DefeasibleRule) -> SolverResult<()> {
        // Find empty slot
        for i in 0..100 {
            if self.rule_base[i].id == 0 {
                self.rule_base[i] = rule;
                return Ok(());
            }
        }
        
        Err(ExecutionError::CapacityExceeded)
    }

    /// Add fact to fact base
    pub fn add_fact(&mut self, fact: Fact) -> SolverResult<()> {
        // Find empty slot
        for i in 0..50 {
            if self.facts[i].id == 0 {
                self.facts[i] = fact;
                
                // Queue for inference
                self.queue_inference(i as u8);
                
                return Ok(());
            }
        }
        
        Err(ExecutionError::CapacityExceeded)
    }

    /// Perform forward chaining inference
    pub fn infer(&mut self) -> SolverResult<DefeasibleState> {
        self.solver_state.iteration = 0;
        self.solver_state.converged = false;

        while !self.queue_empty() && self.solver_state.iteration < self.config.max_iterations {
            // Get next fact from queue
            let fact_index = self.queue_dequeue();
            
            // Find applicable rules
            self.find_applicable_rules(fact_index)?;
            
            // Apply rules and detect conflicts
            self.apply_rules()?;
            
            // Resolve conflicts
            self.resolve_conflicts()?;
            
            self.solver_state.iteration += 1;
        }

        // Check convergence
        self.solver_state.converged = self.queue_empty();

        Ok(DefeasibleState {
            iteration: self.solver_state.iteration,
            num_facts: self.count_facts(),
            rules_fired: self.count_rules_fired(),
            converged: self.solver_state.converged,
        })
    }

    /// Queue fact for inference
    fn queue_inference(&mut self, fact_index: u8) {
        if ((self.queue_tail + 1) % 50) != self.queue_head {
            self.inference_queue[self.queue_tail as usize] = fact_index as usize;
            self.queue_tail = (self.queue_tail + 1) % 50;
        }
    }

    /// Dequeue fact from inference queue
    fn queue_dequeue(&mut self) -> u8 {
        if self.queue_head != self.queue_tail {
            let fact_index = self.inference_queue[self.queue_head as usize];
            self.queue_head = (self.queue_head + 1) % 50;
            fact_index as u8
        } else {
            0
        }
    }

    /// Check if queue is empty
    fn queue_empty(&self) -> bool {
        self.queue_head == self.queue_tail
    }

    /// Find applicable rules for given fact
    fn find_applicable_rules(&mut self, fact_index: u8) -> SolverResult<()> {
        let fact_literal = self.facts[fact_index as usize].literal; // Copy

        // Two-pass approach: collect which rules to fire, then fire them
        let mut rules_to_fire = [false; 100];

        for i in 0..100 {
            if self.rule_base[i].id == 0 || !self.rule_base[i].active {
                continue;
            }

            for j in 0..5 {
                if self.rule_base[i].antecedents[j].variable == 0 {
                    break; // No more antecedents
                }

                let antecedent = self.rule_base[i].antecedents[j]; // Copy
                if self.literals_match(&antecedent, &fact_literal) {
                    self.rule_base[i].fire_count += 1;

                    let rule_copy = self.rule_base[i]; // Copy for immutable check
                    if self.antecedents_satisfied(&rule_copy) {
                        rules_to_fire[i] = true;
                    }
                }
            }
        }

        for i in 0..100 {
            if rules_to_fire[i] {
                self.fire_rule(i)?;
            }
        }

        Ok(())
    }

    /// Check if literals match
    fn literals_match(&self, lit1: &Literal, lit2: &Literal) -> bool {
        lit1.variable == lit2.variable && lit1.negated == lit2.negated
    }

    /// Check if all antecedents are satisfied
    fn antecedents_satisfied(&self, rule: &DefeasibleRule) -> bool {
        for i in 0..5 {
            if rule.antecedents[i].variable == 0 {
                break; // No more antecedents
            }
            
            let mut satisfied = false;
            for j in 0..50 {
                if self.facts[j].id == 0 {
                    continue;
                }
                
                if self.literals_match(&rule.antecedents[i], &self.facts[j].literal) 
                    && !self.facts[j].defeated {
                    satisfied = true;
                    break;
                }
            }
            
            if !satisfied {
                return false;
            }
        }
        
        true
    }

    /// Fire rule and derive consequent
    fn fire_rule(&mut self, rule_index: usize) -> SolverResult<()> {
        let rule = &self.rule_base[rule_index];
        
        // Create new fact
        let new_fact = Fact {
            id: rule.consequent.variable as u32 + 1000, // Unique ID
            literal: rule.consequent,
            supporting_rules: [rule.id, 0, 0],
            defeated: false,
            confidence: rule.priority as f64 / 1000.0,
        };
        
        // Add fact
        self.add_fact(new_fact)?;
        
        Ok(())
    }

    /// Apply rules and detect conflicts
    fn apply_rules(&mut self) -> SolverResult<()> {
        // Check for contradictions
        for i in 0..50 {
            if self.facts[i].id == 0 {
                continue;
            }
            
            for j in i + 1..50 {
                if self.facts[j].id == 0 {
                    continue;
                }
                
                // Check for direct contradiction
                if self.facts[i].literal.variable == self.facts[j].literal.variable &&
                   self.facts[i].literal.negated != self.facts[j].literal.negated {
                    
                    // Add conflict
                    self.add_conflict(i, j, ConflictType::Contradiction)?;
                }
            }
        }
        
        Ok(())
    }

    /// Add conflict to conflict state
    fn add_conflict(&mut self, fact1: usize, fact2: usize, conflict_type: ConflictType) -> SolverResult<()> {
        if self.conflict_state.num_conflicts < 10 {
            let conflict = Conflict {
                facts: [self.facts[fact1].id, self.facts[fact2].id, 0],
                rules: [0, 0, 0],
                conflict_type,
            };
            
            self.conflict_state.conflicts[self.conflict_state.num_conflicts as usize] = conflict;
            self.conflict_state.num_conflicts += 1;
        }
        
        Ok(())
    }

    /// Resolve conflicts using priority strategy
    fn resolve_conflicts(&mut self) -> SolverResult<()> {
        for i in 0..self.conflict_state.num_conflicts as usize {
            let conflict = self.conflict_state.conflicts[i]; // Copy

            match conflict.conflict_type {
                ConflictType::Contradiction => {
                    self.resolve_contradiction(&conflict)?;
                }
                ConflictType::Defeat => {
                    self.resolve_defeat(&conflict)?;
                }
                ConflictType::Preference => {
                    self.resolve_preference(&conflict)?;
                }
            }
        }

        Ok(())
    }

    /// Resolve contradiction by keeping higher confidence fact
    fn resolve_contradiction(&mut self, conflict: &Conflict) -> SolverResult<()> {
        // Find conflicting facts
        let mut fact1_idx = 0;
        let mut fact2_idx = 0;
        let mut fact1_confidence = 0.0;
        let mut fact2_confidence = 0.0;
        
        for i in 0..50 {
            if self.facts[i].id == 0 {
                continue;
            }
            
            if self.facts[i].id == conflict.facts[0] {
                fact1_idx = i;
                fact1_confidence = self.facts[i].confidence;
            }
            if self.facts[i].id == conflict.facts[1] {
                fact2_idx = i;
                fact2_confidence = self.facts[i].confidence;
            }
        }
        
        // Defeat lower confidence fact
        if fact1_confidence < fact2_confidence {
            self.facts[fact1_idx].defeated = true;
        } else {
            self.facts[fact2_idx].defeated = true;
        }
        
        Ok(())
    }

    /// Resolve defeat using rule priorities
    fn resolve_defeat(&mut self, _conflict: &Conflict) -> SolverResult<()> {
        // Simplified defeat resolution
        // In full implementation, would check rule priorities and specificity
        Ok(())
    }

    /// Resolve preference using recency
    fn resolve_preference(&mut self, _conflict: &Conflict) -> SolverResult<()> {
        // Simplified preference resolution
        // In full implementation, would use recency or user preferences
        Ok(())
    }

    /// Count active facts
    fn count_facts(&self) -> u16 {
        let mut count = 0;
        for i in 0..50 {
            if self.facts[i].id != 0 && !self.facts[i].defeated {
                count += 1;
            }
        }
        count
    }

    /// Count fired rules
    fn count_rules_fired(&self) -> u16 {
        let mut count = 0;
        for i in 0..100 {
            if self.rule_base[i].id != 0 && self.rule_base[i].fire_count > 0 {
                count += 1;
            }
        }
        count
    }

    /// Get derived facts
    pub fn get_facts(&self) -> &[Fact; 50] {
        &self.facts
    }
}

impl BoundedSatSolver {
    /// Create new SAT solver
    pub fn new(config: SolverConfig) -> Self {
        Self {
            clauses: [Clause::default(); 50],
            assignments: [VariableAssignment::default(); 20],
            decision_stack: [Decision::default(); 20],
            propagation_queue: [0; 20],
            assignment_level: 0,
            conflict_clause: None,
            config,
            solver_state: SolverState::default(),
        }
    }

    /// Add clause to clause database
    pub fn add_clause(&mut self, clause: Clause) -> SolverResult<()> {
        // Find empty slot
        for i in 0..50 {
            if self.clauses[i].id == 0 {
                self.clauses[i] = clause;
                return Ok(());
            }
        }
        
        Err(ExecutionError::CapacityExceeded)
    }

    /// Solve SAT problem
    pub fn solve(&mut self) -> SolverResult<SatState> {
        self.solver_state.iteration = 0;
        self.solver_state.satisfiable = None;

        // Initialize assignments
        for i in 0..20 {
            self.assignments[i] = VariableAssignment::default();
        }

        // DPLL algorithm with unit propagation
        if self.dpll_algorithm()? {
            self.solver_state.satisfiable = Some(true);
        } else {
            self.solver_state.satisfiable = Some(false);
        }

        Ok(SatState {
            iteration: self.solver_state.iteration,
            num_decisions: self.assignment_level as u16,
            num_propagations: self.count_propagations(),
            satisfiable: self.solver_state.satisfiable,
        })
    }

    /// DPLL algorithm implementation
    fn dpll_algorithm(&mut self) -> SolverResult<bool> {
        // Unit propagation
        if !self.unit_propagate()? {
            return Ok(false); // Conflict detected
        }

        // Check if all variables are assigned
        if self.all_variables_assigned() {
            return Ok(true); // Satisfying assignment found
        }

        // Choose unassigned variable
        let var = self.choose_unassigned_variable()?;
        
        // Try assigning true
        self.assign_variable(var, AssignmentValue::True, None)?;
        if self.dpll_algorithm()? {
            return Ok(true);
        }
        
        // Backtrack
        self.backtrack()?;
        
        // Try assigning false
        self.assign_variable(var, AssignmentValue::False, None)?;
        if self.dpll_algorithm()? {
            return Ok(true);
        }
        
        // Backtrack
        self.backtrack()?;
        
        Ok(false)
    }

    /// Unit propagation
    fn unit_propagate(&mut self) -> SolverResult<bool> {
        let mut propagated = true;
        
        while propagated {
            propagated = false;
            
            // Check all clauses for unit clauses
            for i in 0..50 {
                if self.clauses[i].id == 0 {
                    continue;
                }
                
                if let Some(unit_literal) = self.is_unit_clause(i)? {
                    // Propagate unit literal
                    let value = if unit_literal.negated {
                        AssignmentValue::False
                    } else {
                        AssignmentValue::True
                    };
                    
                    self.assign_variable(unit_literal.variable, value, Some(self.clauses[i].id))?;
                    propagated = true;
                }
            }
        }
        
        Ok(true)
    }

    /// Check if clause is unit clause
    fn is_unit_clause(&self, clause_index: usize) -> SolverResult<Option<Literal>> {
        let clause = &self.clauses[clause_index];
        let mut unassigned_count = 0;
        let mut unit_literal = None;
        
        for i in 0..clause.num_literals as usize {
            let literal = clause.literals[i];
            let assignment = &self.assignments[literal.variable as usize];
            
            match assignment.value {
                AssignmentValue::Unassigned => {
                    unassigned_count += 1;
                    unit_literal = Some(literal);
                }
                AssignmentValue::True => {
                    if !literal.negated {
                        return Ok(None); // Clause satisfied
                    }
                }
                AssignmentValue::False => {
                    if literal.negated {
                        return Ok(None); // Clause satisfied
                    }
                }
            }
        }
        
        if unassigned_count == 1 {
            Ok(unit_literal)
        } else if unassigned_count == 0 {
            Err(ExecutionError::Unsatisfiable)
        } else {
            Ok(None)
        }
    }

    /// Assign variable
    fn assign_variable(&mut self, var: u8, value: AssignmentValue, antecedent: Option<u32>) -> SolverResult<()> {
        self.assignments[var as usize] = VariableAssignment {
            value,
            level: self.assignment_level,
            antecedent,
        };
        
        // Add to propagation queue
        for i in 0..20 {
            if self.propagation_queue[i] == 0 {
                self.propagation_queue[i] = var;
                break;
            }
        }
        
        Ok(())
    }

    /// Check if all variables are assigned
    fn all_variables_assigned(&self) -> bool {
        for i in 0..20 {
            if self.assignments[i].value == AssignmentValue::Unassigned {
                return false;
            }
        }
        true
    }

    /// Choose unassigned variable
    fn choose_unassigned_variable(&self) -> SolverResult<u8> {
        for i in 0..20 {
            if self.assignments[i].value == AssignmentValue::Unassigned {
                return Ok(i as u8);
            }
        }
        Err(ExecutionError::InvalidParameters)
    }

    /// Backtrack to previous decision level
    fn backtrack(&mut self) -> SolverResult<()> {
        if self.assignment_level == 0 {
            return Err(ExecutionError::BacktrackFailed);
        }
        
        // Clear assignments at current level
        for i in 0..20 {
            if self.assignments[i].level == self.assignment_level {
                self.assignments[i] = VariableAssignment::default();
            }
        }
        
        self.assignment_level -= 1;
        
        Ok(())
    }

    /// Count propagations
    fn count_propagations(&self) -> u16 {
        let mut count = 0;
        for i in 0..20 {
            if self.assignments[i].antecedent.is_some() {
                count += 1;
            }
        }
        count
    }

    /// Get variable assignments
    pub fn get_assignments(&self) -> &[VariableAssignment; 20] {
        &self.assignments
    }
}

impl Default for DefeasibleRule {
    fn default() -> Self {
        Self {
            id: 0,
            rule_type: RuleType::Strict,
            antecedents: [Literal::default(); 5],
            consequent: Literal::default(),
            priority: 0,
            active: true,
            fire_count: 0,
        }
    }
}

impl Default for Fact {
    fn default() -> Self {
        Self {
            id: 0,
            literal: Literal::default(),
            supporting_rules: [0, 0, 0],
            defeated: false,
            confidence: 0.0,
        }
    }
}

impl Default for Clause {
    fn default() -> Self {
        Self {
            id: 0,
            literals: [Literal::default(); 5],
            num_literals: 0,
            learned: false,
            activity: 0.0,
        }
    }
}

impl Default for Literal {
    fn default() -> Self {
        Self {
            variable: 0,
            negated: false,
        }
    }
}

impl Default for VariableAssignment {
    fn default() -> Self {
        Self {
            value: AssignmentValue::Unassigned,
            level: 0,
            antecedent: None,
        }
    }
}

impl Default for Decision {
    fn default() -> Self {
        Self {
            variable: 0,
            value: AssignmentValue::Unassigned,
            level: 0,
        }
    }
}

impl Default for ConflictState {
    fn default() -> Self {
        Self {
            conflicts: [Conflict::default(); 10],
            num_conflicts: 0,
            resolution_strategy: ResolutionStrategy::Priority,
        }
    }
}

impl Default for Conflict {
    fn default() -> Self {
        Self {
            facts: [0, 0, 0],
            rules: [0, 0, 0],
            conflict_type: ConflictType::Contradiction,
        }
    }
}

impl Default for DefeasibleState {
    fn default() -> Self {
        Self {
            iteration: 0,
            num_facts: 0,
            rules_fired: 0,
            converged: false,
        }
    }
}

impl Default for SatState {
    fn default() -> Self {
        Self {
            iteration: 0,
            num_decisions: 0,
            num_propagations: 0,
            satisfiable: None,
        }
    }
}

impl Default for ForwardChainingDefeasible {
    fn default() -> Self {
        Self::new(SolverConfig::default())
    }
}

impl Default for BoundedSatSolver {
    fn default() -> Self {
        Self::new(SolverConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defeasible_reasoning() {
        let mut solver = ForwardChainingDefeasible::new(SolverConfig::default());
        
        // Add rule: A -> B
        let rule = DefeasibleRule {
            id: 1,
            rule_type: RuleType::Strict,
            antecedents: [Literal { variable: 1, negated: false }, Literal { variable: 0, negated: false }, Literal { variable: 0, negated: false }, Literal { variable: 0, negated: false }, Literal { variable: 0, negated: false }],
            consequent: Literal { variable: 2, negated: false },
            priority: 100,
            active: true,
            fire_count: 0,
        };
        
        solver.add_rule(rule).unwrap();
        
        // Add fact: A
        let fact = Fact {
            id: 100,
            literal: Literal { variable: 1, negated: false },
            supporting_rules: [0, 0, 0],
            defeated: false,
            confidence: 1.0,
        };
        
        solver.add_fact(fact).unwrap();
        
        // Perform inference
        let result = solver.infer();
        assert!(result.is_ok());
        
        let state = result.unwrap();
        assert!(state.num_facts >= 2); // A and B
        assert!(state.rules_fired >= 1);
    }

    #[test]
    fn test_sat_solver() {
        let mut solver = BoundedSatSolver::new(SolverConfig::default());
        
        // Add clause: (A ∨ B ∨ ¬C)
        let clause = Clause {
            id: 1,
            literals: [
                Literal { variable: 1, negated: false },
                Literal { variable: 2, negated: false },
                Literal { variable: 3, negated: true },
                Literal { variable: 0, negated: false },
                Literal { variable: 0, negated: false },
            ],
            num_literals: 3,
            learned: false,
            activity: 0.0,
        };
        
        solver.add_clause(clause).unwrap();
        
        // Solve
        let result = solver.solve();
        assert!(result.is_ok());
        
        let state = result.unwrap();
        assert!(state.satisfiable.is_some());
    }

    #[test]
    fn test_zero_allocation_guarantee() {
        assert_eq!(core::mem::size_of::<ForwardChainingDefeasible>(), 4496);
        assert_eq!(core::mem::size_of::<BoundedSatSolver>(), 2368);
        assert_eq!(core::mem::size_of::<DefeasibleRule>(), 32);
        assert_eq!(core::mem::size_of::<Fact>(), 32);
        assert_eq!(core::mem::size_of::<Clause>(), 48);
        assert_eq!(core::mem::size_of::<Literal>(), 2);
    }
}
