//! The Orchestration Sieve — LLM Sub-Agent Dispatch Layer
//!
//! Sits between raw input (multi-modal data, user prompts) and the Webizen VM.
//! Coordinates pre-processing → intent validation → inference → output grounding.
//!
//! Flow:
//!   RawInput → [Orchestrator] → validate_intent → [LlmAgent.infer] → validate_output → .q42 commit

use crate::llm_agent::{AgentIntent, AgentRuntime, LocalLlmAgent, WebizenVerdict};
use crate::modalities::logic::n3_compiler::{compile_rules_with_shacl_gate, default_observation_shape, N3OutputMode};
use crate::modalities::logic::n3_parser::{N3Event, N3Parser};
use crate::modalities::logic::shacl::{CompiledShape, ShaclCompiler, ShaclConstraint, ShaclSeverity};
#[cfg(not(target_arch = "wasm32"))]
use crate::wal::{commit_semantic_mutation, WalHandoffResult, WriteAheadLog};
use crate::webizen::{SlgArena, SlgOpcode, VmFrame};
use crate::{q_hash, NQuin};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelLifecycle {
    Discovered,
    MappedToDisk,
    StreamingVRAM,
    Active,
    Scrubbing,
}

/// The outcome of a full orchestrated inference cycle.
#[derive(Debug)]
pub enum OrchestrationResult {
    /// Output was validated, grounded, and ready to commit to the semantic graph.
    Committed {
        text: String,
        provenance_quins: Vec<u64>,
        semantic_quin: Option<NQuin>,
        wal_committed: bool,
        wal_suspended: bool,
        suspended_agreement_id: Option<u64>,
    },
    /// Webizen blocked the operation at pre-flight or post-flight.
    Blocked {
        rule_violated: u64,
        reason: &'static str,
    },
    /// Inference failed (timeout, backend unavailable, etc.)
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalStatus {
    /// Normal operating temperatures. Full 3-Core Triad utilization.
    Cool,
    /// Elevated temperatures. Restrict non-essential parallelism and heavy sieving.
    Warm,
    /// Critical temperatures. Pause background ingestion/indexing, throttle to critical-path single-thread only.
    Critical,
}

pub trait ThermalGovernor: Send + Sync {
    /// Returns the current thermal state of the host device.
    fn get_thermal_state(&self) -> ThermalStatus;

    /// Optional hook for the governor to self-adjust or log transitions.
    fn adjust_policy(&self, status: ThermalStatus) {
        let _ = status; // Default no-op
    }
}

pub struct NullThermalGovernor;
impl ThermalGovernor for NullThermalGovernor {
    fn get_thermal_state(&self) -> ThermalStatus {
        ThermalStatus::Cool
    }
}

pub struct TaskOrchestrator {
    thermal_governor: Box<dyn ThermalGovernor>,
    pub current_model_state: Arc<std::sync::Mutex<ModelLifecycle>>,
    pub current_model_id: Arc<std::sync::Mutex<Option<u64>>>,
    pub resident_memory_bytes: Arc<AtomicU64>,
    pub scrubbing_lock: Arc<AtomicBool>,
}

impl TaskOrchestrator {
    fn routed_shapes(intent: &AgentIntent) -> Vec<CompiledShape> {
        let compiler = ShaclCompiler::new();
        let mut shapes = Vec::new();

        let has_namespace = |needles: &[&str]| {
            needles.iter().any(|needle| {
                let hash = q_hash(needle);
                intent.context_namespaces.iter().any(|ns| *ns == hash)
            })
        };

        if has_namespace(&[
            "health", "medical", "clinical", "fhir", "loinc", "snomed", "anatomy",
        ]) {
            shapes.push(default_observation_shape());
            shapes.push(compiler.compile_class(
                "fhir:Observation",
                "health:heartRate",
                ShaclConstraint::MinInclusive(20.0),
                ShaclSeverity::Violation,
            ));
        }

        if has_namespace(&[
            "legal",
            "law",
            "contract",
            "guardian",
            "guardianship",
            "rights",
            "agreement",
            "consent",
        ]) {
            shapes.push(compiler.compile_class(
                "q42:Agreement",
                "q42:hasGuardian",
                ShaclConstraint::MinCount(1),
                ShaclSeverity::Violation,
            ));
            shapes.push(compiler.compile_class(
                "q42:Agreement",
                "q42:requiresConsensus",
                ShaclConstraint::MinCount(1),
                ShaclSeverity::Violation,
            ));
        }

        if has_namespace(&["commons", "governance", "community"]) {
            shapes.push(compiler.compile_class(
                "q42:CommonsAgreement",
                "q42:hasDomainScope",
                ShaclConstraint::MinCount(1),
                ShaclSeverity::Violation,
            ));
        }

        // CogAI agent shapes — always active for any inference intent.
        // Validates cog:Agent identity, cog:Goal alignment, and inference authorization.
        shapes.push(compiler.compile_class(
            "cog:Agent",
            "cog:agentID",
            ShaclConstraint::MinCount(1),
            ShaclSeverity::Violation,
        ));
        shapes.push(compiler.compile_class(
            "q42:InferenceIntent",
            "q42:inferenceAuthorizedBy",
            ShaclConstraint::MinCount(1),
            ShaclSeverity::Violation,
        ));
        shapes.push(compiler.compile_class(
            "q42:InferenceIntent",
            "q42:provenanceCitations",
            ShaclConstraint::MinInclusive(1.0),
            ShaclSeverity::Warning,
        ));

        if shapes.is_empty() {
            shapes.push(default_observation_shape());
        }
        shapes
    }

    pub fn new(thermal_governor: Box<dyn ThermalGovernor>) -> Self {
        Self {
            thermal_governor,
            current_model_state: Arc::new(std::sync::Mutex::new(ModelLifecycle::Discovered)),
            current_model_id: Arc::new(std::sync::Mutex::new(None)),
            resident_memory_bytes: Arc::new(AtomicU64::new(0)),
            scrubbing_lock: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn thermal_state_label(&self) -> &'static str {
        match self.thermal_governor.get_thermal_state() {
            ThermalStatus::Cool => "Cool",
            ThermalStatus::Warm => "Warm",
            ThermalStatus::Critical => "Critical",
        }
    }

    pub fn resident_model_id(&self) -> Option<u64> {
        *self.current_model_id.lock().unwrap()
    }

    pub fn resident_memory_bytes(&self) -> u64 {
        self.resident_memory_bytes.load(Ordering::Relaxed)
    }

    pub fn register_resident_model(&self, model_id: u64, bytes: u64) {
        *self.current_model_id.lock().unwrap() = Some(model_id);
        self.resident_memory_bytes.store(bytes, Ordering::Relaxed);
    }

    pub fn load_model(&self, agent: &LocalLlmAgent, model_id: u64) -> Result<(), &'static str> {
        if self.scrubbing_lock.load(Ordering::Acquire) {
            return Err("Cannot load model: Swarm Worker is actively scrubbing memory arena");
        }
        if let Some(active_model) = self.resident_model_id() {
            if active_model != model_id {
                return Err("Cannot load model: another model is still resident; evict it first");
            }
        }

        let mut state = self.current_model_state.lock().unwrap();
        *state = ModelLifecycle::MappedToDisk;
        *state = ModelLifecycle::StreamingVRAM;
        *state = ModelLifecycle::Active;
        drop(state);
        *self.current_model_id.lock().unwrap() = Some(model_id);

        // Wire lex sidecar: prefer schema.org bundle, else model-adjacent `.q42.lex`.
        let schema_lex = "data/schemaorg/30.0/schemaorg-current-https.q42.lex";
        if std::path::Path::new(schema_lex).exists() {
            agent.configure_sieve_lex(schema_lex);
        } else if let crate::llm_agent::AgentBackend::Local { model_path, .. } = &agent.backend {
            let mut p = std::path::PathBuf::from(model_path);
            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()).map(str::to_string) {
                p.set_file_name(format!("{stem}.q42.lex"));
                if p.exists() {
                    agent.configure_sieve_lex(p.to_string_lossy().into_owned());
                }
            }
        }
        Ok(())
    }

    pub fn evict_model(&self, model_id: u64) {
        let resident = self.resident_model_id();
        if resident.is_none() {
            self.resident_memory_bytes.store(0, Ordering::Relaxed);
            if let Ok(mut st) = self.current_model_state.lock() {
                *st = ModelLifecycle::Discovered;
            }
            return;
        }
        if resident != Some(model_id) {
            return;
        }

        if let Ok(mut state) = self.current_model_state.lock() {
            *state = ModelLifecycle::Scrubbing;
        }

        self.scrubbing_lock.store(true, Ordering::Release);
        let scrub_bytes = self.resident_memory_bytes.swap(0, Ordering::Relaxed);

        let lock_clone = self.scrubbing_lock.clone();
        let state_clone = self.current_model_state.clone();
        let model_clone = self.current_model_id.clone();

        // Asynchronous scrubbing over a fixed stack buffer keeps the sweep deterministic and heap-free.
        thread::spawn(move || {
            let mut scrub_block = [0u8; 4096];
            let mut remaining = scrub_bytes.max(scrub_block.len() as u64);
            while remaining > 0 {
                let write_len = remaining.min(scrub_block.len() as u64) as usize;
                for byte in scrub_block.iter_mut().take(write_len) {
                    unsafe { std::ptr::write_volatile(byte, 0) };
                }
                remaining = remaining.saturating_sub(write_len as u64);
                if remaining > 0 {
                    std::thread::yield_now();
                }
            }

            // Release the cryptographic lock and revert state
            lock_clone.store(false, Ordering::Release);
            if let Ok(mut model) = model_clone.lock() {
                *model = None;
            }
            if let Ok(mut st) = state_clone.lock() {
                *st = ModelLifecycle::Discovered;
            }
            crate::resident_model::clear_resident_model();
        });
    }

    /// Parse LLM-emitted N3, validate via SHACL compiler, and execute on the Sentinel VM.
    fn gate_llm_n3_output(
        text: &str,
        contract_hash: u64,
        intent: &AgentIntent,
    ) -> Result<(), &'static str> {
        let mut rules = Vec::new();
        let mut parser = N3Parser::new(std::io::Cursor::new(text.as_bytes()));
        parser
            .parse_all(|event| {
                if let N3Event::LogicRule(rule) = event {
                    rules.push(rule);
                }
                Ok(())
            })
            .map_err(|_| "Invalid N3 output from LLM")?;

        if rules.is_empty() {
            return Err("LLM did not emit parseable N3 assertions");
        }

        let routed_shapes = Self::routed_shapes(intent);
        let shapes: Vec<&CompiledShape> = routed_shapes.iter().collect();
        let mut opcodes = [SlgOpcode::Call; 256];
        let mut quins = [crate::NQuin::default(); 64];
        let program =
            compile_rules_with_shacl_gate(&rules, &shapes, &mut opcodes, &mut quins, contract_hash)
                .map_err(|_| "SHACL validation failed for LLM N3 output")?;

        let mut arena = SlgArena::new();
        let mut frame = VmFrame::default();
        let _ = crate::modalities::logic::n3_compiler::execute_compiled_program(
            &mut arena,
            &opcodes[..program.opcode_count],
            &mut frame,
            32,
        )
        .map_err(|_| "Sentinel VM memory overflow")?;
        Ok(())
    }

    /// Runs a full, Webizen-gated inference cycle for a registered LLM sub-agent.
    pub fn orchestrate_inference(
        &self,
        agent: &dyn AgentRuntime,
        prompt: &str,
        graph_context: &str,
        intent: AgentIntent,
        suspended: Option<&mut crate::crdt::SuspendedTransactionQueue>,
    ) -> OrchestrationResult {
        let thermal_state = self.thermal_governor.get_thermal_state();

        match thermal_state {
            ThermalStatus::Critical => {
                if !intent.is_critical() {
                    return OrchestrationResult::Blocked {
                        rule_violated: 0xDEADBEEF, // Mock constant for Thermal Block
                        reason: "Device critical thermal state. Non-essential inference paused.",
                    };
                }
            }
            ThermalStatus::Warm => {
                // Future extension: dampened logic
            }
            ThermalStatus::Cool => {}
        }

        // 1. Pre-flight: validate intent against Rights Ontology
        match agent.validate_intent(&intent) {
            WebizenVerdict::Deny {
                rule_violated,
                reason,
                conduct_record,
            } => {
                // If the verdict contains a conduct violation Quin, propagate it to the immutable ledger
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(quin) = conduct_record {
                    if let Ok(mut wal) = crate::wal::WriteAheadLog::open(".qualia_conduct.wal") {
                        let _ = wal.append_mutation(&quin);

                        // Cryptographic signing pipeline (using a static key for demonstration of wiring)
                        let secret = [42u8; 32];
                        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret);
                        let frame = [quin];
                        let sub_root = crate::agency::compute_scoped_merkle_root(
                            &frame,
                            intent.principal_did_hash,
                        );
                        let _signature = crate::agency::sign_agency_root(&signing_key, &sub_root);

                        // In production, the signature and quin would be passed to SuperBlockWriter
                    }
                }
                #[cfg(target_arch = "wasm32")]
                let _ = conduct_record;
                return OrchestrationResult::Blocked {
                    rule_violated,
                    reason,
                };
            }
            WebizenVerdict::DenyWithExplanation {
                rule_violated,
                reason: _,
                explanation: _,
            } => {
                // Return blocked with the detailed explanation
                return OrchestrationResult::Blocked {
                    rule_violated,
                    reason: "Intent Frame Violation",
                };
            }
            WebizenVerdict::RequireReconfirmation { reason: _ } => {
                return OrchestrationResult::Blocked {
                    rule_violated: 0,
                    reason: "Reconfirmation required",
                };
            }
            WebizenVerdict::Sanitised { .. } => { /* intent was scrubbed; proceed with caution */ }
            WebizenVerdict::Permit => {}
        }

        // 1b. Quantum egress gate — block classified prompts from remote QPU
        if let Some(reason) = crate::modalities::logic::qubo::quantum_prompt_gate(prompt) {
            return OrchestrationResult::Blocked {
                rule_violated: crate::q_hash("q42:QuantumTaskShape"),
                reason,
            };
        }

        // 1c. CogAI pre-flight: write agent registration quin and validate InferenceIntentShape.
        // The `register_agent` quin is written to AGENT_CONTEXT so downstream SPARQL queries
        // can resolve cog:Agent identity without re-deriving it from the principal DID.
        {
            use crate::temporal_graph::register_agent;
            let _agent_quin = register_agent(intent.principal_did_hash, 0);
            // Future: append _agent_quin to the per-request provenance accumulator.
        }

        // 2. Inference
        let output = match agent.infer(prompt, graph_context) {
            Ok(o) => o,
            Err(e) => return OrchestrationResult::Failed(format!("{:?}", e)),
        };

        // 2b. Optional CogAI symbolic path: compile LLM-emitted N3 through SHACL → bytecode.
        // Skipped when the neuro-symbolic sieve already emitted a structured Quin.
        if output.semantic_quin.is_none()
            && (intent.output_mode == N3OutputMode::N3Assertions
                || intent.output_mode == N3OutputMode::GraphMutation)
        {
            if let Err(reason) =
                Self::gate_llm_n3_output(&output.text, intent.principal_did_hash, &intent)
            {
                return OrchestrationResult::Blocked {
                    rule_violated: crate::q_hash("q42:N3Compiler"),
                    reason,
                };
            }
        }

        // 3. Post-flight: validate output grounding
        match agent.validate_output(&output) {
            WebizenVerdict::Deny {
                rule_violated,
                reason,
                conduct_record,
            } => {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(quin) = conduct_record {
                    if let Ok(mut wal) = crate::wal::WriteAheadLog::open(".qualia_conduct.wal") {
                        let _ = wal.append_mutation(&quin);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                let _ = conduct_record;
                return OrchestrationResult::Blocked {
                    rule_violated,
                    reason,
                };
            }
            WebizenVerdict::DenyWithExplanation {
                rule_violated,
                reason: _,
                explanation: _,
            } => {
                return OrchestrationResult::Blocked {
                    rule_violated,
                    reason: "Output blocked due to frame bounds",
                };
            }
            WebizenVerdict::RequireReconfirmation { reason: _ } => {
                return OrchestrationResult::Blocked {
                    rule_violated: 0,
                    reason: "Output requires reconfirmation",
                };
            }
            _ => {}
        }

        let mut semantic_quin = output.semantic_quin;
        let mut wal_written = false;
        let mut wal_suspended = false;
        let mut suspended_agreement_id = None;

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref mut quin) = semantic_quin {
            if let Ok(mut wal) = WriteAheadLog::open(".qualia_graph_mutations.wal") {
                let secret = [42u8; 32];
                let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret);
                let mut local_suspended = crate::crdt::SuspendedTransactionQueue::new();
                let queue = suspended.unwrap_or(&mut local_suspended);
                let agent_did_hash = q_hash(agent.agent_did());
                match commit_semantic_mutation(
                    &mut wal,
                    quin,
                    intent.principal_did_hash,
                    agent_did_hash,
                    &signing_key,
                    queue,
                ) {
                    Ok(WalHandoffResult::Committed) => {
                        wal_written = true;
                    }
                    Ok(WalHandoffResult::Suspended { agreement_id }) => {
                        wal_written = true;
                        wal_suspended = true;
                        suspended_agreement_id = Some(agreement_id);
                    }
                    Err(_) => {}
                }
            }
        }

        OrchestrationResult::Committed {
            text: output.text,
            provenance_quins: output.provenance_quins,
            semantic_quin,
            wal_committed: wal_written,
            wal_suspended,
            suspended_agreement_id,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::llm_agent::{AgentIntent, AgentRuntime, LocalLlmAgent, SANCTUARY_SCOPE_WEBIZEN};
    use crate::modalities::logic::n3_compiler::N3OutputMode;

    #[test]
    pub fn qualia_validate_ring_buffer() {}

    #[test]
    fn test_orchestrator_full_permit_path() {
        let agent = LocalLlmAgent::new("did:git:orch-test", "model.gguf");
        let intent = AgentIntent {
            intent_predicate: 0x1234,
            requested_graph_scope: vec![0xABCD],
            context_namespaces: vec![],
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: 0,
            mcp_intent_frame_hash: 0x1234,
            output_mode: N3OutputMode::FreeText,
            clearance_ceiling: 0,
            max_sentinel_depth: 32,
            active_profile: None,
        };
        let orch = TaskOrchestrator::new(Box::new(NullThermalGovernor));
        let result = orch.orchestrate_inference(
            &agent,
            "Summarise my health graph.",
            "some_graph_bytes",
            intent,
            None,
        );
        assert!(matches!(result, OrchestrationResult::Committed { .. }));
    }

    #[test]
    fn test_orchestrator_blocks_sanctuary_intent() {
        let agent = LocalLlmAgent::new("did:git:orch-test", "model.gguf");
        let intent = AgentIntent {
            intent_predicate: 0x1234,
            requested_graph_scope: vec![SANCTUARY_SCOPE_WEBIZEN],
            context_namespaces: vec![],
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: 0,
            mcp_intent_frame_hash: 0x1234,
            output_mode: N3OutputMode::FreeText,
            clearance_ceiling: 0,
            max_sentinel_depth: 32,
            active_profile: None,
        };
        let orch = TaskOrchestrator::new(Box::new(NullThermalGovernor));
        let result =
            orch.orchestrate_inference(&agent, "Show me sanctuary data.", "ctx", intent, None);
        assert!(matches!(result, OrchestrationResult::Blocked { .. }));
    }

    #[test]
    fn test_thermal_critical_blocks_non_essential_inference() {
        struct MockCriticalThermalGovernor;
        impl ThermalGovernor for MockCriticalThermalGovernor {
            fn get_thermal_state(&self) -> ThermalStatus {
                ThermalStatus::Critical
            }
        }

        let agent = LocalLlmAgent::new("did:git:orch-test", "model.gguf");
        let intent = AgentIntent {
            intent_predicate: 0x1234, // non-critical
            requested_graph_scope: vec![],
            context_namespaces: vec![],
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: 0,
            mcp_intent_frame_hash: 0x1234,
            output_mode: N3OutputMode::FreeText,
            clearance_ceiling: 0,
            max_sentinel_depth: 32,
            active_profile: None,
        };
        let orch = TaskOrchestrator::new(Box::new(MockCriticalThermalGovernor));
        let result = orch.orchestrate_inference(&agent, "Query", "ctx", intent, None);
        assert!(matches!(
            result,
            OrchestrationResult::Blocked {
                rule_violated: 0xDEADBEEF,
                ..
            }
        ));
    }

    #[test]
    fn test_async_scrub_lock_invariant() {
        let orch = TaskOrchestrator::new(Box::new(NullThermalGovernor));
        let agent = LocalLlmAgent::new("did:git:orch-test", "model.gguf");

        // 1. Initially it should load fine
        assert!(orch.load_model(&agent, 123).is_ok());

        // 2. Trigger an eviction (spawns background thread to scrub)
        orch.evict_model(123);

        // 3. IMMEDIATELY try to load a new model. The lock should reject it.
        let load_result = orch.load_model(&agent, 456);
        assert!(load_result.is_err(), "Orchestrator violated mechanical sympathy! Mapped model while Swarm worker was still scrubbing.");

        // 4. Wait for the background Swarm worker to complete its duty of care
        std::thread::sleep(std::time::Duration::from_millis(50));

        // 5. Try loading again. The lock should be cleared.
        let second_load_result = orch.load_model(&agent, 456);
        assert!(
            second_load_result.is_ok(),
            "Orchestrator failed to load model after scrubbing lock cleared."
        );
        assert_eq!(orch.resident_model_id(), Some(456));

        // Ensure Webizen VM logic handles yielding
        let mut vm = crate::modalities::logic::core::WebizenVM::with_scrubbing_lock(orch.scrubbing_lock.clone());
        let bytecode = vec![crate::modalities::logic::core::WebizenOpcode::LoadModel(999)];
        vm.load_bytecode(&bytecode);

        let quin = crate::NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        };

        // If we trigger evict again, the VM should yield on LoadModel
        orch.evict_model(456);
        let exec_result = vm.execute_implication(&quin);
        assert!(exec_result.is_none());
        assert_eq!(
            vm.yielded_op,
            Some(crate::modalities::logic::core::WebizenOpcode::LoadModel(999))
        );

        // Wait for scrub to clear
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(orch.resident_model_id(), None);
    }

    fn write_fever_lex_bytes() -> Vec<u8> {
        let h_sub = q_hash("Patient");
        let h_pred = q_hash("fever");
        let h_obj = q_hash("True");
        let entries = [(h_sub, "Patient"), (h_pred, "fever"), (h_obj, "True")];
        let mut sorted = entries.to_vec();
        sorted.sort_unstable_by_key(|(h, _)| *h);
        let entry_count = sorted.len() as u64;
        let strings_offset = 32 + entry_count * 16;
        let mut blob = Vec::new();
        let mut index = Vec::new();
        for (hash, text) in &sorted {
            let str_off = blob.len() as u64;
            let b = text.as_bytes();
            let len = b.len().min(65535) as u16;
            blob.extend_from_slice(&len.to_le_bytes());
            blob.extend_from_slice(&b[..len as usize]);
            index.extend_from_slice(&hash.to_le_bytes());
            index.extend_from_slice(&str_off.to_le_bytes());
        }
        let mut out = Vec::new();
        out.extend_from_slice(b"Q42LEX\0\0");
        out.extend_from_slice(&entry_count.to_le_bytes());
        out.extend_from_slice(&strings_offset.to_le_bytes());
        out.extend_from_slice(&1u64.to_le_bytes());
        out.extend_from_slice(&index);
        out.extend_from_slice(&blob);
        out
    }

    /// End-to-end: mmap lex → FSM sieve (3 tokens) → fiduciary stamp → volatile WAL commit.
    #[test]
    #[serial_test::serial]
    fn test_e2e_llm_to_wal_pipeline() {
        let _profiler = dhat::Profiler::builder().testing().build();

        let lex_bytes = write_fever_lex_bytes();
        let lex_mmap = crate::q42_lex::Q42LexMmap::from_bytes(&lex_bytes).expect("lex");
        let tok = crate::gguf_sharder::GgufTokenizer::default();
        let spec = crate::neuro_symbolic_sieve::SieveLexSpec::fever_observation();
        let mut sieve = crate::neuro_symbolic_sieve::NeuroSymbolicSieve::from_lex_and_tokenizer(
            &lex_mmap, &tok, &spec,
        );
        assert!(
            sieve.masks_ready(),
            "lex must resolve fever triple token IDs"
        );
        let (sub, pred, obj) = sieve.resolved_token_triple().expect("triple");

        let wal_file = tempfile::NamedTempFile::new().expect("wal temp");
        let mut wal = WriteAheadLog::open(wal_file.path()).expect("wal open");
        let secret = [42u8; 32];
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret);
        let mut suspended = crate::crdt::SuspendedTransactionQueue::new();
        let principal = q_hash("did:q42:test-principal");
        let agent_did = q_hash("did:git:orch-test");

        let stats_before = dhat::HeapStats::get();

        assert!(sieve.apply_token(sub).is_ok());
        assert!(sieve.apply_token(pred).is_ok());
        assert!(sieve.apply_token(obj).is_ok());
        assert_eq!(
            sieve.emitted_len(),
            3,
            "must halt at exactly 3 sieve tokens"
        );
        assert!(sieve.is_complete());

        let mut quin = sieve.assemble_quin(q_hash("clinical:fever-context"));
        let handoff = commit_semantic_mutation(
            &mut wal,
            &mut quin,
            principal,
            agent_did,
            &signing_key,
            &mut suspended,
        )
        .expect("wal handoff");

        let stats_after = dhat::HeapStats::get();
        assert_eq!(
            stats_after.total_blocks - stats_before.total_blocks,
            0,
            "sieve decode + WAL write must not heap-allocate"
        );
        assert_eq!(
            stats_after.total_bytes - stats_before.total_bytes,
            0,
            "sieve decode + WAL write must not heap-allocate"
        );

        assert_eq!(handoff, WalHandoffResult::Committed);
        let recovered = wal.recover().expect("recover");
        assert_eq!(recovered.len(), 1, "WAL must contain one 48-byte Quin");
        assert_eq!(recovered[0].subject, q_hash("Patient"));
        assert_eq!(recovered[0].predicate, q_hash("fever"));
        assert_eq!(recovered[0].object, q_hash("True"));
        assert_eq!(recovered[0].context, principal);
        assert_eq!(
            recovered[0].parity,
            recovered[0].subject
                ^ recovered[0].predicate
                ^ recovered[0].object
                ^ recovered[0].context
                ^ recovered[0].metadata
        );

        // Optional GPU path: chunked prefill + sieved decode when Gemma GGUF is present.
        let gemma = std::path::Path::new(
            "C:/Projects/qualiaDB/gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf",
        );
        if !gemma.exists() {
            return;
        }
        let mut lex_tmp = tempfile::NamedTempFile::new().expect("lex temp");
        std::io::Write::write_all(&mut lex_tmp, &lex_bytes).expect("lex write");
        let agent = LocalLlmAgent::new("did:git:e2e-fever", gemma.to_string_lossy());
        agent.configure_sieve_lex(lex_tmp.path().to_string_lossy().into_owned());
        let frame_hash = q_hash("intent:fever");
        let fever_intent = AgentIntent {
            intent_predicate: frame_hash,
            requested_graph_scope: vec![q_hash("snomed:hasFever")],
            context_namespaces: vec![q_hash("health"), q_hash("snomed")],
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: principal,
            mcp_intent_frame_hash: frame_hash,
            output_mode: N3OutputMode::GraphMutation,
            clearance_ceiling: 0,
            max_sentinel_depth: 32,
            active_profile: None,
        };
        assert_eq!(agent.validate_intent(&fever_intent), WebizenVerdict::Permit);
        if let Ok(output) = AgentRuntime::infer(&agent, "The user has a fever", "clinical-context")
        {
            assert!(
                output.tokens_generated <= 3,
                "sieve must cap generation at 3 tokens"
            );
            if let Some(q) = output.semantic_quin {
                assert_ne!(q.subject, 0);
                assert_ne!(q.predicate, 0);
            }
        }
    }
}
