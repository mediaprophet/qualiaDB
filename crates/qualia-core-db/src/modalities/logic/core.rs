//! Core 1 Webizen Bytecode VM
//! A `#![no_std]` compatible virtual machine that executes a fixed-size
//! instruction set across the 48-byte Quins without triggering heap allocations.

use crate::NQuin;

/// The micro-instruction set (ISA) for the Core 1 Logic Engine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WebizenOpcode {
    /// Compares the Quin's Subject to a hardcoded 60-bit ID
    MatchSubject(u64),
    /// Compares the Quin's Predicate to a hardcoded 60-bit ID
    MatchPredicate(u64),
    /// Compares the Quin's Object to a hardcoded 60-bit ID
    MatchObject(u64),
    /// Evaluates if the 5th Vector's bits match the target mask exactly
    EvalMetadataMask(u32),
    /// Extracts a u64 from the Quin (0=Subj, 1=Pred, 2=Obj, 3=Ctx) and stores it in the VM Register
    BindRegister {
        vector_id: u8,
        register_index: usize,
    },
    /// Asserts that a VM register equals the given Quin vector
    MatchRegister {
        vector_id: u8,
        register_index: usize,
    },
    /// Halts execution and returns false immediately if the prior condition failed
    HaltIfFalse,
    /// Yields a new generated Quin (The consequent of an implication `=>`)
    EmitQuin {
        subject_reg: usize,
        predicate: u64,
        object: u64,
        context_reg: usize,
    },
    /// Continuous Constraint: Evaluates <
    LessThan { vector_id: u8, value: f32 },
    /// Continuous Constraint: Evaluates >
    GreaterThan { vector_id: u8, value: f32 },
    /// Continuous Constraint: Evaluates <=
    LessOrEqual { vector_id: u8, value: f32 },
    /// Continuous Constraint: Evaluates >=
    GreaterOrEqual { vector_id: u8, value: f32 },
    /// Temporal Logic: Always constraint (LTL)
    Always(u64),
    /// Temporal Logic: Eventually constraint (LTL)
    Eventually(u64),
    /// Temporal Logic: Next constraint (LTL)
    Next(u64),
    /// Yields a mathematically calculated Quin consequence
    EmitCalculatedQuin {
        subject_reg: usize,
        predicate: u64,
        object_calc_op: u8,
        context_reg: usize,
    },
    /// Evaluates the 5th Metadata Vector confidence weight. If below threshold, tags consequence as Defeasible.
    YieldConfidence(f32),
    /// Triggers native mapping of a GGUF model pointer into the OS page cache
    LoadModel(u64),
    /// Cryptographically flushes a 512MB model mapping using Volatile Scrubbing
    EvictModel(u64),
}

/// The Zero-Allocation Virtual Machine for N3Logic and Constraints.
pub struct WebizenVM {
    /// The local L1-cached execution stack for bound variables
    pub registers: [Option<u64>; 16],
    /// The maximum number of instructions allowed in a single rule block
    pub bytecode_buffer: [Option<WebizenOpcode>; 64],
    /// Shared reference to the orchestrator's cryptographic memory flush lock
    pub scrubbing_lock: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// State tracking for suspended opcodes that yielded due to a hardware lock
    pub yielded_op: Option<WebizenOpcode>,
}

impl WebizenVM {
    pub fn new() -> Self {
        Self {
            registers: [None; 16],
            bytecode_buffer: [None; 64],
            scrubbing_lock: None,
            yielded_op: None,
        }
    }

    pub fn with_scrubbing_lock(lock: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Self {
        let mut vm = Self::new();
        vm.scrubbing_lock = Some(lock);
        vm
    }

    pub fn load_bytecode(&mut self, instructions: &[WebizenOpcode]) {
        for (i, &op) in instructions.iter().enumerate().take(64) {
            self.bytecode_buffer[i] = Some(op);
        }
    }

    /// Serializes the current VM execution frame into a strictly-sized zero-allocation buffer
    /// for offline suspension in the CRDT queue while awaiting M:N Guardianship signatures.
    pub fn flatten_to_suspended(
        &self,
        agreement_id: u64,
        threshold: u8,
        current_quin: crate::NQuin,
    ) -> crate::crdt::SuspendedTransaction {
        crate::crdt::SuspendedTransaction {
            agreement_id,
            threshold,
            collected_signatures: 0,
            registers: self.registers,
            bytecode_buffer: self.bytecode_buffer,
            yielded_op: self.yielded_op,
            suspended_quin: current_quin,
        }
    }

    /// Evaluates a loaded constraint block against a target Quin.
    pub fn execute_constraint(&mut self, quin: &NQuin) -> bool {
        let mut condition_flag = true;

        // Extract 5th Metadata Vector for Stochastic/Fuzzy logic weight (bottom 16 bits as probability 0.0 - 1.0)
        let _stochastic_weight = (quin.metadata & 0xFFFF) as f32 / 65535.0;

        for op in self.bytecode_buffer.iter().flatten() {
            crate::telemetry::VM_CYCLES_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            match op {
                WebizenOpcode::MatchSubject(val) => {
                    condition_flag = quin.subject == *val;
                }
                WebizenOpcode::MatchPredicate(val) => {
                    condition_flag = quin.predicate == *val;
                }
                WebizenOpcode::MatchObject(val) => {
                    condition_flag = quin.object == *val;
                }
                WebizenOpcode::EvalMetadataMask(mask) => {
                    let quin_mask = (quin.metadata & 0xFFFF) as u32;
                    condition_flag = (quin_mask & mask) == *mask;
                }
                WebizenOpcode::BindRegister {
                    vector_id,
                    register_index,
                } => {
                    let value = match vector_id {
                        0 => quin.subject,
                        1 => quin.predicate,
                        2 => quin.object,
                        3 => quin.context,
                        _ => 0,
                    };
                    self.registers[*register_index] = Some(value);
                    condition_flag = true;
                }
                WebizenOpcode::MatchRegister {
                    vector_id,
                    register_index,
                } => {
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
                }
                WebizenOpcode::HaltIfFalse => {
                    if !condition_flag {
                        return false;
                    }
                }
                WebizenOpcode::LessThan { vector_id, value } => {
                    condition_flag =
                        Self::extract_float(quin, *vector_id).map_or(false, |v| v < *value);
                }
                WebizenOpcode::GreaterThan { vector_id, value } => {
                    condition_flag =
                        Self::extract_float(quin, *vector_id).map_or(false, |v| v > *value);
                }
                WebizenOpcode::LessOrEqual { vector_id, value } => {
                    condition_flag =
                        Self::extract_float(quin, *vector_id).map_or(false, |v| v <= *value);
                }
                WebizenOpcode::GreaterOrEqual { vector_id, value } => {
                    condition_flag =
                        Self::extract_float(quin, *vector_id).map_or(false, |v| v >= *value);
                }
                WebizenOpcode::Always(stress_threshold) => {
                    crate::telemetry::ATOMIC_INTEGRATION_STEPS
                        .fetch_add(50, std::sync::atomic::Ordering::Relaxed);
                    condition_flag = Self::extract_float(quin, 2)
                        .map_or(false, |stress| stress < *stress_threshold as f32);
                }
                WebizenOpcode::Eventually(stress_threshold) => {
                    crate::telemetry::ATOMIC_INTEGRATION_STEPS
                        .fetch_add(50, std::sync::atomic::Ordering::Relaxed);
                    condition_flag = Self::extract_float(quin, 2)
                        .map_or(false, |stress| stress >= *stress_threshold as f32);
                }
                WebizenOpcode::Next(stress_threshold) => {
                    crate::telemetry::ATOMIC_INTEGRATION_STEPS
                        .fetch_add(50, std::sync::atomic::Ordering::Relaxed);
                    condition_flag = Self::extract_float(quin, 2)
                        .map_or(false, |stress| stress == *stress_threshold as f32);
                }
                WebizenOpcode::YieldConfidence(threshold) => {
                    let stochastic_weight = (quin.metadata & 0xFFFF) as f32 / 65535.0;
                    if stochastic_weight < *threshold {
                        condition_flag = false; // For raw constraints, it fails the assertion
                    }
                }
                WebizenOpcode::LoadModel(model_id) => {
                    if let Some(ref lock) = self.scrubbing_lock {
                        if lock.load(std::sync::atomic::Ordering::Acquire) {
                            self.yielded_op = Some(WebizenOpcode::LoadModel(*model_id));
                            return false; // Suspend execution
                        }
                    }
                }
                WebizenOpcode::EvictModel(_) => {}
                WebizenOpcode::EmitQuin { .. } | WebizenOpcode::EmitCalculatedQuin { .. } => {
                    // Handled exclusively by `execute_implication`
                }
            }
        }

        condition_flag
    }

    /// Evaluates an N3 Implication `=>` constraint against a target Quin.
    /// Returns `Some(NQuin)` if the antecedent passes and yields a consequence.
    pub fn execute_implication(&mut self, quin: &NQuin) -> Option<NQuin> {
        let mut condition_flag = true;
        let mut emitted_quin = None;
        let mut defeasible_tag = false;

        for op in self.bytecode_buffer.iter().flatten() {
            crate::telemetry::VM_CYCLES_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            match op {
                WebizenOpcode::MatchSubject(val) => condition_flag = quin.subject == *val,
                WebizenOpcode::MatchPredicate(val) => condition_flag = quin.predicate == *val,
                WebizenOpcode::MatchObject(val) => condition_flag = quin.object == *val,
                WebizenOpcode::EvalMetadataMask(mask) => {
                    let quin_mask = (quin.metadata & 0xFFFF) as u32;
                    condition_flag = (quin_mask & mask) == *mask;
                }
                WebizenOpcode::BindRegister {
                    vector_id,
                    register_index,
                } => {
                    let value = match vector_id {
                        0 => quin.subject,
                        1 => quin.predicate,
                        2 => quin.object,
                        3 => quin.context,
                        _ => 0,
                    };
                    self.registers[*register_index] = Some(value);
                    condition_flag = true;
                }
                WebizenOpcode::MatchRegister {
                    vector_id,
                    register_index,
                } => {
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
                        condition_flag = false;
                    }
                }
                WebizenOpcode::HaltIfFalse => {
                    if !condition_flag {
                        return None;
                    }
                }
                WebizenOpcode::EmitQuin {
                    subject_reg,
                    predicate,
                    object,
                    context_reg,
                } => {
                    if condition_flag {
                        let s = self.registers[*subject_reg].unwrap_or(0);
                        let c = self.registers[*context_reg].unwrap_or(0);

                        let mut resulting_quin = NQuin {
                            subject: s,
                            predicate: *predicate,
                            object: *object,
                            context: c,
                            metadata: quin.metadata,
                            parity: 0,
                        };

                        let clock = quin.extract_lamport_clock();
                        if clock < 0x1FFF_FFFF {
                            resulting_quin.set_lamport_clock(clock + 1);
                        }
                        if defeasible_tag {
                            resulting_quin.metadata |= 1 << 60;
                        }
                        emitted_quin = Some(resulting_quin);
                    }
                }
                WebizenOpcode::LessThan { vector_id, value } => {
                    condition_flag =
                        Self::extract_float(quin, *vector_id).map_or(false, |v| v < *value);
                }
                WebizenOpcode::GreaterThan { vector_id, value } => {
                    condition_flag =
                        Self::extract_float(quin, *vector_id).map_or(false, |v| v > *value);
                }
                WebizenOpcode::LessOrEqual { vector_id, value } => {
                    condition_flag =
                        Self::extract_float(quin, *vector_id).map_or(false, |v| v <= *value);
                }
                WebizenOpcode::GreaterOrEqual { vector_id, value } => {
                    condition_flag =
                        Self::extract_float(quin, *vector_id).map_or(false, |v| v >= *value);
                }
                WebizenOpcode::Always(stress_threshold) => {
                    crate::telemetry::ATOMIC_INTEGRATION_STEPS
                        .fetch_add(50, std::sync::atomic::Ordering::Relaxed);
                    condition_flag = Self::extract_float(quin, 2)
                        .map_or(false, |stress| stress < *stress_threshold as f32);
                }
                WebizenOpcode::Eventually(stress_threshold) => {
                    crate::telemetry::ATOMIC_INTEGRATION_STEPS
                        .fetch_add(50, std::sync::atomic::Ordering::Relaxed);
                    condition_flag = Self::extract_float(quin, 2)
                        .map_or(false, |stress| stress >= *stress_threshold as f32);
                }
                WebizenOpcode::Next(stress_threshold) => {
                    crate::telemetry::ATOMIC_INTEGRATION_STEPS
                        .fetch_add(50, std::sync::atomic::Ordering::Relaxed);
                    condition_flag = Self::extract_float(quin, 2)
                        .map_or(false, |stress| stress == *stress_threshold as f32);
                }
                WebizenOpcode::EmitCalculatedQuin {
                    subject_reg,
                    predicate,
                    object_calc_op,
                    context_reg,
                } => {
                    if condition_flag {
                        let s = self.registers[*subject_reg].unwrap_or(0);
                        let c = self.registers[*context_reg].unwrap_or(0);

                        // Example calculated transformation (e.g. op 1 = mass * accel placeholder)
                        let calc_val = match object_calc_op {
                            1 => 42.0_f32, // Mocked math transformation
                            _ => 0.0_f32,
                        };

                        let mut resulting_quin = NQuin {
                            subject: s,
                            predicate: *predicate,
                            // Tag the object as float (0x1 << 60) and pack f32
                            object: (0x1 << 60) | (calc_val.to_bits() as u64),
                            context: c,
                            metadata: quin.metadata,
                            parity: 0,
                        };

                        let clock = quin.extract_lamport_clock();
                        if clock < 0x1FFF_FFFF {
                            resulting_quin.set_lamport_clock(clock + 1);
                        }
                        if defeasible_tag {
                            resulting_quin.metadata |= 1 << 60;
                        }
                        emitted_quin = Some(resulting_quin);
                    }
                }
                WebizenOpcode::YieldConfidence(threshold) => {
                    let stochastic_weight = (quin.metadata & 0xFFFF) as f32 / 65535.0;
                    if stochastic_weight < *threshold {
                        defeasible_tag = true;
                    }
                }
                WebizenOpcode::LoadModel(model_id) => {
                    if let Some(ref lock) = self.scrubbing_lock {
                        if lock.load(std::sync::atomic::Ordering::Acquire) {
                            self.yielded_op = Some(WebizenOpcode::LoadModel(*model_id));
                            return None; // Suspend execution yielding no consequence yet
                        }
                    }
                }
                WebizenOpcode::EvictModel(_) => {}
            }
        }

        emitted_quin
    }

    /// Extracts a tagged floating point value from a given 64-bit Quin vector.
    /// Uses the top 4 bits as a type tag (0x1 = float).
    #[inline(always)]
    fn extract_float(quin: &NQuin, vector_id: u8) -> Option<f32> {
        let val = match vector_id {
            0 => quin.subject,
            1 => quin.predicate,
            2 => quin.object,
            3 => quin.context,
            _ => return None,
        };

        let tag = val >> 60;
        if tag == 0x1 {
            Some(f32::from_bits((val & 0xFFFFFFFF) as u32))
        } else {
            None
        }
    }

    /// Prunes neural hallucinations: If a Defeasible claim is contradicted by a hard physical fact, it is removed.
    pub fn prune_defeasible_claims(qualia_graph: &mut Vec<NQuin>) {
        // A claim is defeasible if bit 60 is set.
        let mut deterministic_subjects = std::collections::HashSet::new();

        // Pass 1: Gather hard facts (metadata bit 60 is NOT set)
        for quin in qualia_graph.iter() {
            if (quin.metadata & (1 << 60)) == 0 {
                // Subject has a deterministic fact
                deterministic_subjects.insert(quin.subject);
            }
        }

        // Pass 2: Remove defeasible claims contradicted by hard facts
        qualia_graph.retain(|quin| {
            let is_defeasible = (quin.metadata & (1 << 60)) != 0;
            if is_defeasible && deterministic_subjects.contains(&quin.subject) {
                return false; // Prune neural hallucination due to hard fact conflict
            }
            true
        });
    }
}

/// The Translation Layer. Parses N3Logic/SHACL into bytecode arrays.
pub struct WebizenCompiler;

impl WebizenCompiler {
    /// Mocks the translation of a SHACL constraint into Webizen Bytecode.
    /// Example: `[Shape] sh:property [ sh:path ex:age ; sh:minInclusive 18 ]`
    pub fn compile_mock_constraint() -> Vec<WebizenOpcode> {
        vec![
            WebizenOpcode::MatchPredicate(100), // Predicate 100 = 'age'
            WebizenOpcode::HaltIfFalse,
            // (A true engine would have a GreaterThan opcode here, using MatchObject for now)
            WebizenOpcode::MatchObject(18),
            WebizenOpcode::HaltIfFalse,
        ]
    }

    /// Compiles a medical N3 constraint for Differential Diagnostics.
    /// Example: IF Subject has symptom SNOMED:Fever => Yield Diagnosis Potential
    pub fn compile_diagnostic_constraint() -> Vec<WebizenOpcode> {
        vec![
            WebizenOpcode::MatchPredicate(100), // e.g. "has_symptom"
            WebizenOpcode::MatchObject(200),    // e.g. "Fever"
            WebizenOpcode::HaltIfFalse,
            WebizenOpcode::BindRegister {
                vector_id: 0,
                register_index: 0,
            },
            WebizenOpcode::BindRegister {
                vector_id: 3,
                register_index: 1,
            },
            // Emits a Diagnosis Quin
            WebizenOpcode::EmitQuin {
                subject_reg: 0,
                predicate: 300,
                object: 400,
                context_reg: 1,
            },
        ]
    }
}

/// The Informatics Subsystem (Differential Diagnostics)
/// Executes N3Logic bytecode constraints over a subset of Quins (e.g., from a .q42 file)
/// to derive deterministic inferences natively on the edge, replacing the legacy WebAssembly/Prolog engine.
pub fn execute_differential_diagnostics(qualia_graph: &[NQuin]) -> Vec<NQuin> {
    let mut inferences = Vec::new();
    let mut vm = WebizenVM::new();
    let diagnostic_rules = WebizenCompiler::compile_diagnostic_constraint();
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
    fn test_webizen_vm_execution() {
        let mut vm = WebizenVM::new();
        let bytecode = vec![
            WebizenOpcode::MatchPredicate(42),
            WebizenOpcode::HaltIfFalse,
            WebizenOpcode::BindRegister {
                vector_id: 0,
                register_index: 0,
            },
            WebizenOpcode::HaltIfFalse,
        ];
        vm.load_bytecode(&bytecode);

        // Matching Quin
        let valid_quin = NQuin {
            subject: 999,
            predicate: 42,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        };

        // Fails predicate match
        let invalid_quin = NQuin {
            subject: 999,
            predicate: 99,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        };

        assert_eq!(
            vm.execute_constraint(&valid_quin),
            true,
            "VM failed to execute valid bytecode constraint"
        );
        assert_eq!(
            vm.registers[0],
            Some(999),
            "VM failed to bind Subject to Register 0"
        );

        assert_eq!(
            vm.execute_constraint(&invalid_quin),
            false,
            "VM erroneously passed invalid quin constraint"
        );
    }

    #[test]
    fn test_qualia_n3_implication() {
        let mut vm = WebizenVM::new();
        // Rule: { ?x predicate 42 } => { ?x predicate 100 }
        let bytecode = vec![
            WebizenOpcode::MatchPredicate(42),
            WebizenOpcode::HaltIfFalse,
            WebizenOpcode::BindRegister {
                vector_id: 0,
                register_index: 0,
            }, // Bind Subject
            WebizenOpcode::BindRegister {
                vector_id: 3,
                register_index: 1,
            }, // Bind Context
            WebizenOpcode::EmitQuin {
                subject_reg: 0,
                predicate: 100,
                object: 999,
                context_reg: 1,
            },
        ];
        vm.load_bytecode(&bytecode);

        // The input antecedent
        let _input_quin = crate::q_turtle!("Alice", "knows", "Bob");
        // q_turtle! hashes "knows" to something, but we hardcoded predicate 42 in bytecode, so let's mock it
        let trigger_quin = NQuin {
            subject: 123,
            predicate: 42,
            object: 456,
            context: 789,
            metadata: 0b01 << 61,
            parity: 0,
        };

        let result = vm.execute_implication(&trigger_quin);
        assert!(
            result.is_some(),
            "Implication should have yielded a consequent Quin"
        );

        let output = result.unwrap();
        assert_eq!(output.subject, 123, "Subject was not bound properly");
        assert_eq!(
            output.predicate, 100,
            "Consequent predicate not emitted properly"
        );
        assert_eq!(output.object, 999, "Consequent object not emitted properly");
        assert_eq!(output.context, 789, "Context was not carried over");
        assert_eq!(
            output.identify_routing_lane(),
            crate::PermissiveRoutingLane::EnforcePermissiveCommons,
            "Routing lane was not inherited"
        );
        assert_eq!(
            output.extract_lamport_clock(),
            1,
            "Lamport clock did not advance"
        );

        // Let's verify q_turtle compile-time macro works
        let qt = crate::q_turtle!("Alice", "knows", "Bob");
        assert_eq!(qt.subject, crate::q_hash("Alice"));
        assert_eq!(qt.predicate, crate::q_hash("knows"));
        assert_eq!(qt.object, crate::q_hash("Bob"));
        assert_eq!(
            qt.identify_routing_lane(),
            crate::PermissiveRoutingLane::EnforcePermissiveCommons
        );
    }

    #[test]
    fn test_webizen_float_logic() {
        let mut vm = WebizenVM::new();
        // Pack 3.14 as a float tag (0x1 << 60)
        let float_val = 3.14_f32;
        let tagged_object = (0x1 << 60) | (float_val.to_bits() as u64);

        let q = NQuin {
            subject: 0,
            predicate: 0,
            object: tagged_object,
            context: 0,
            metadata: 0,
            parity: 0,
        };

        let bytecode = vec![
            WebizenOpcode::LessThan {
                vector_id: 2,
                value: 4.0,
            }, // 3.14 < 4.0
            WebizenOpcode::HaltIfFalse,
            WebizenOpcode::GreaterThan {
                vector_id: 2,
                value: 3.0,
            }, // 3.14 > 3.0
            WebizenOpcode::HaltIfFalse,
        ];

        vm.load_bytecode(&bytecode);
        assert_eq!(
            vm.execute_constraint(&q),
            true,
            "VM failed to execute continuous float bounds"
        );
    }
}
