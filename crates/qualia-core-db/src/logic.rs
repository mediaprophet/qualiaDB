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
                }
            }
        }
        
        condition_flag
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
}
