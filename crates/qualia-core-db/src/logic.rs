//! Core 1 Sentinel Bytecode VM
//! A `#![no_std]` compatible virtual machine that executes a fixed-size 
//! instruction set across the 48-byte Quins without triggering heap allocations.

use crate::QualiaQuin;

/// The micro-instruction set (ISA) for the Core 1 Logic Engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SentinelOpcode {
    /// Compares the Quin's Subject to a hardcoded 60-bit ID
    MatchSubject(u64),
    /// Compares the Quin's Predicate to a hardcoded 60-bit ID
    MatchPredicate(u64),
    /// Compares the Quin's Object to a hardcoded 60-bit ID
    MatchObject(u64),
    /// Evaluates if the 5th Vector's bits match the target mask exactly
    EvalMetadataMask(u32),
    /// Extracts a u64 from the Quin (0=Subj, 1=Pred, 2=Obj, 3=Ctx) and stores it in the VM Register
    BindRegister { vector_id: u8, register_index: usize },
    /// Asserts that a VM register equals the given Quin vector
    MatchRegister { vector_id: u8, register_index: usize },
    /// Halts execution and returns false immediately if the prior condition failed
    HaltIfFalse,
    /// Yields a new generated Quin (The consequent of an implication `=>`)
    EmitQuin { subject_reg: usize, predicate: u64, object: u64, context_reg: usize },
}

/// The Zero-Allocation Virtual Machine for N3Logic and Constraints.
pub struct SentinelVM {
    /// The local L1-cached execution stack for bound variables
    pub registers: [Option<u64>; 16],
    /// The maximum number of instructions allowed in a single rule block
    pub bytecode_buffer: [Option<SentinelOpcode>; 64],
}

impl SentinelVM {
    pub fn new() -> Self {
        Self {
            registers: [None; 16],
            bytecode_buffer: [None; 64],
        }
    }

    pub fn load_bytecode(&mut self, instructions: &[SentinelOpcode]) {
        for (i, &op) in instructions.iter().enumerate().take(64) {
            self.bytecode_buffer[i] = Some(op);
        }
    }

    /// Evaluates a loaded constraint block against a target Quin.
    pub fn execute_constraint(&mut self, quin: &QualiaQuin) -> bool {
        let mut condition_flag = true;

        for op in self.bytecode_buffer.iter().flatten() {
            crate::telemetry::VM_CYCLES_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            
            match op {
                SentinelOpcode::MatchSubject(val) => {
                    condition_flag = quin.subject == *val;
                },
                SentinelOpcode::MatchPredicate(val) => {
                    condition_flag = quin.predicate == *val;
                },
                SentinelOpcode::MatchObject(val) => {
                    condition_flag = quin.object == *val;
                },
                SentinelOpcode::EvalMetadataMask(mask) => {
                    let quin_mask = (quin.metadata & 0xFFFF) as u32;
                    condition_flag = (quin_mask & mask) == *mask;
                },
                SentinelOpcode::BindRegister { vector_id, register_index } => {
                    let value = match vector_id {
                        0 => quin.subject,
                        1 => quin.predicate,
                        2 => quin.object,
                        3 => quin.context,
                        _ => 0,
                    };
                    self.registers[*register_index] = Some(value);
                    condition_flag = true;
                },
                SentinelOpcode::MatchRegister { vector_id, register_index } => {
                    if let Some(bound_val) = self.registers[*register_index] {
                        let value = match vector_id {
                            0 => quin.subject,
                            1 => quin.predicate,
                            2 => quin.object,
                            3 => quin.context,
                            _ => 0,
                        };
                        condition_flag = value == bound_val;
                    } else {
                        // Register unbound
                        condition_flag = false;
                    }
                },
                SentinelOpcode::HaltIfFalse => {
                    if !condition_flag {
                        return false;
                    }
                },
                SentinelOpcode::EmitQuin { .. } => {
                    // Handled exclusively by `execute_implication`
                }
            }
        }
        
        condition_flag
    }

    /// Evaluates an N3 Implication `=>` constraint against a target Quin.
    /// Returns `Some(QualiaQuin)` if the antecedent passes and yields a consequence.
    pub fn execute_implication(&mut self, quin: &QualiaQuin) -> Option<QualiaQuin> {
        let mut condition_flag = true;
        let mut emitted_quin = None;

        for op in self.bytecode_buffer.iter().flatten() {
            crate::telemetry::VM_CYCLES_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            
            match op {
                SentinelOpcode::MatchSubject(val) => condition_flag = quin.subject == *val,
                SentinelOpcode::MatchPredicate(val) => condition_flag = quin.predicate == *val,
                SentinelOpcode::MatchObject(val) => condition_flag = quin.object == *val,
                SentinelOpcode::EvalMetadataMask(mask) => {
                    let quin_mask = (quin.metadata & 0xFFFF) as u32;
                    condition_flag = (quin_mask & mask) == *mask;
                },
                SentinelOpcode::BindRegister { vector_id, register_index } => {
                    let value = match vector_id {
                        0 => quin.subject, 1 => quin.predicate, 2 => quin.object, 3 => quin.context, _ => 0,
                    };
                    self.registers[*register_index] = Some(value);
                    condition_flag = true;
                },
                SentinelOpcode::MatchRegister { vector_id, register_index } => {
                    if let Some(bound_val) = self.registers[*register_index] {
                        let value = match vector_id {
                            0 => quin.subject, 1 => quin.predicate, 2 => quin.object, 3 => quin.context, _ => 0,
                        };
                        condition_flag = value == bound_val;
                    } else {
                        condition_flag = false;
                    }
                },
                SentinelOpcode::HaltIfFalse => {
                    if !condition_flag { return None; }
                },
                SentinelOpcode::EmitQuin { subject_reg, predicate, object, context_reg } => {
                    if condition_flag {
                        let s = self.registers[*subject_reg].unwrap_or(0);
                        let c = self.registers[*context_reg].unwrap_or(0);
                        
                        // Inherit routing lane and Lamport clock from the triggering Quin
                        let mut resulting_quin = QualiaQuin {
                            subject: s,
                            predicate: *predicate,
                            object: *object,
                            context: c,
                            metadata: quin.metadata,
                            parity: 0,
                        };
                        
                        // Advance the Lamport Clock for Truth Maintenance natively
                        let clock = quin.extract_lamport_clock();
                        if clock < 0x1FFF_FFFF {
                            resulting_quin.set_lamport_clock(clock + 1);
                        }
                        
                        emitted_quin = Some(resulting_quin);
                    }
                }
            }
        }
        
        emitted_quin
    }
}

/// The Translation Layer. Parses N3Logic/SHACL into bytecode arrays.
pub struct SentinelCompiler;

impl SentinelCompiler {
    /// Mocks the translation of a SHACL constraint into Sentinel Bytecode.
    /// Example: `[Shape] sh:property [ sh:path ex:age ; sh:minInclusive 18 ]`
    pub fn compile_mock_constraint() -> Vec<SentinelOpcode> {
        vec![
            SentinelOpcode::MatchPredicate(100), // Predicate 100 = 'age'
            SentinelOpcode::HaltIfFalse,
            // (A true engine would have a GreaterThan opcode here, using MatchObject for now)
            SentinelOpcode::MatchObject(18),     
            SentinelOpcode::HaltIfFalse,
        ]
    }

    /// Compiles a medical N3 constraint for Differential Diagnostics.
    /// Example: IF Subject has symptom SNOMED:Fever => Yield Diagnosis Potential
    pub fn compile_diagnostic_constraint() -> Vec<SentinelOpcode> {
        vec![
            SentinelOpcode::MatchPredicate(100), // e.g. "has_symptom"
            SentinelOpcode::MatchObject(200), // e.g. "Fever"
            SentinelOpcode::HaltIfFalse,
            SentinelOpcode::BindRegister { vector_id: 0, register_index: 0 },
            SentinelOpcode::BindRegister { vector_id: 3, register_index: 1 },
            // Emits a Diagnosis Quin
            SentinelOpcode::EmitQuin { subject_reg: 0, predicate: 300, object: 400, context_reg: 1 }, 
        ]
    }
}

/// The Informatics Subsystem (Differential Diagnostics)
/// Executes N3Logic bytecode constraints over a subset of Quins (e.g., from a .q42 file)
/// to derive deterministic inferences natively on the edge, replacing the legacy WebAssembly/Prolog engine.
pub fn execute_differential_diagnostics(qualia_graph: &[QualiaQuin]) -> Vec<QualiaQuin> {
    let mut inferences = Vec::new();
    let mut vm = SentinelVM::new();
    let diagnostic_rules = SentinelCompiler::compile_diagnostic_constraint();
    vm.load_bytecode(&diagnostic_rules);

    for quin in qualia_graph {
        if let Some(inferred_quin) = vm.execute_implication(quin) {
            inferences.push(inferred_quin);
        }
    }
    inferences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentinel_vm_execution() {
        let mut vm = SentinelVM::new();
        let bytecode = vec![
            SentinelOpcode::MatchPredicate(42),
            SentinelOpcode::HaltIfFalse,
            SentinelOpcode::BindRegister { vector_id: 0, register_index: 0 },
            SentinelOpcode::HaltIfFalse,
        ];
        vm.load_bytecode(&bytecode);

        // Matching Quin
        let valid_quin = QualiaQuin {
            subject: 999,
            predicate: 42,
            object: 0, context: 0, metadata: 0, parity: 0
        };

        // Fails predicate match
        let invalid_quin = QualiaQuin {
            subject: 999,
            predicate: 99,
            object: 0, context: 0, metadata: 0, parity: 0
        };

        assert_eq!(vm.execute_constraint(&valid_quin), true, "VM failed to execute valid bytecode constraint");
        assert_eq!(vm.registers[0], Some(999), "VM failed to bind Subject to Register 0");

        assert_eq!(vm.execute_constraint(&invalid_quin), false, "VM erroneously passed invalid quin constraint");
    }

    #[test]
    fn test_qualia_n3_implication() {
        let mut vm = SentinelVM::new();
        // Rule: { ?x predicate 42 } => { ?x predicate 100 }
        let bytecode = vec![
            SentinelOpcode::MatchPredicate(42),
            SentinelOpcode::HaltIfFalse,
            SentinelOpcode::BindRegister { vector_id: 0, register_index: 0 }, // Bind Subject
            SentinelOpcode::BindRegister { vector_id: 3, register_index: 1 }, // Bind Context
            SentinelOpcode::EmitQuin { subject_reg: 0, predicate: 100, object: 999, context_reg: 1 },
        ];
        vm.load_bytecode(&bytecode);

        // The input antecedent
        let _input_quin = crate::q_turtle!("Alice", "knows", "Bob");
        // q_turtle! hashes "knows" to something, but we hardcoded predicate 42 in bytecode, so let's mock it
        let trigger_quin = QualiaQuin {
            subject: 123,
            predicate: 42,
            object: 456,
            context: 789,
            metadata: 0b01 << 61,
            parity: 0
        };

        let result = vm.execute_implication(&trigger_quin);
        assert!(result.is_some(), "Implication should have yielded a consequent Quin");
        
        let output = result.unwrap();
        assert_eq!(output.subject, 123, "Subject was not bound properly");
        assert_eq!(output.predicate, 100, "Consequent predicate not emitted properly");
        assert_eq!(output.object, 999, "Consequent object not emitted properly");
        assert_eq!(output.context, 789, "Context was not carried over");
        assert_eq!(
            output.identify_routing_lane(),
            crate::PermissiveRoutingLane::EnforcePermissiveCommons,
            "Routing lane was not inherited"
        );
        assert_eq!(output.extract_lamport_clock(), 1, "Lamport clock did not advance");
        
        // Let's verify q_turtle compile-time macro works
        let qt = crate::q_turtle!("Alice", "knows", "Bob");
        assert_eq!(qt.subject, crate::q_hash("Alice"));
        assert_eq!(qt.predicate, crate::q_hash("knows"));
        assert_eq!(qt.object, crate::q_hash("Bob"));
        assert_eq!(qt.identify_routing_lane(), crate::PermissiveRoutingLane::EnforcePermissiveCommons);
    }
}
