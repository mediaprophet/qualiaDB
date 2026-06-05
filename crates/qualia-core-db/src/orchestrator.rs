//! The Orchestration Sieve — LLM Sub-Agent Dispatch Layer
//!
//! Sits between raw input (multi-modal data, user prompts) and the Webizen VM.
//! Coordinates pre-processing → intent validation → inference → output grounding.
//!
//! Flow:
//!   RawInput → [Orchestrator] → validate_intent → [LlmAgent.infer] → validate_output → .q42 commit

use crate::llm_agent::{AgentIntent, AgentRuntime, WebizenVerdict};
use std::sync::atomic::{AtomicBool, Ordering};
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
    Committed { text: String, provenance_quins: Vec<u64> },
    /// Webizen blocked the operation at pre-flight or post-flight.
    Blocked { rule_violated: u64, reason: &'static str },
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
    pub scrubbing_lock: Arc<AtomicBool>,
}

impl TaskOrchestrator {
    pub fn new(thermal_governor: Box<dyn ThermalGovernor>) -> Self {
        Self { 
            thermal_governor,
            current_model_state: Arc::new(std::sync::Mutex::new(ModelLifecycle::Discovered)),
            scrubbing_lock: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn load_model(&self, _model_id: u64) -> Result<(), &'static str> {
        if self.scrubbing_lock.load(Ordering::Acquire) {
            return Err("Cannot load model: Swarm Worker is actively scrubbing memory arena");
        }
        
        let mut state = self.current_model_state.lock().unwrap();
        *state = ModelLifecycle::MappedToDisk;
        *state = ModelLifecycle::StreamingVRAM;
        *state = ModelLifecycle::Active;
        Ok(())
    }

    pub fn evict_model(&self, _model_id: u64) {
        let mut state = self.current_model_state.lock().unwrap();
        *state = ModelLifecycle::Scrubbing;
        
        self.scrubbing_lock.store(true, Ordering::Release);
        
        let lock_clone = self.scrubbing_lock.clone();
        let state_clone = self.current_model_state.clone();
        
        // Asynchronous scrubbing via Swarm Worker
        thread::spawn(move || {
            // Mocking a 512MB allocation that needs to be scrubbed
            let mut mock_memory = vec![0u8; 1024]; 
            let ptr = mock_memory.as_mut_ptr();
            
            unsafe {
                // Cryptographic flush of the memory boundaries to zero
                std::ptr::write_volatile(ptr, 0);
            }
            
            // Artificial delay to simulate large memory sweep taking 15ms
            thread::sleep(std::time::Duration::from_millis(15));
            
            // Release the cryptographic lock and revert state
            lock_clone.store(false, Ordering::Release);
            if let Ok(mut st) = state_clone.lock() {
                *st = ModelLifecycle::Discovered; // Ready to be loaded again or remains unloaded
            }
        });
    }

    /// Runs a full, Webizen-gated inference cycle for a registered LLM sub-agent.
    pub fn orchestrate_inference(
        &self,
        agent: &dyn AgentRuntime,
        prompt: &str,
        graph_context: &str,
        intent: AgentIntent,
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
            WebizenVerdict::Deny { rule_violated, reason, conduct_record } => {
                // If the verdict contains a conduct violation Quin, propagate it to the immutable ledger
                if let Some(quin) = conduct_record {
                    if let Ok(mut wal) = crate::wal::WriteAheadLog::open(".qualia_conduct.wal") {
                        let _ = wal.append_mutation(&quin);
                        
                        // Cryptographic signing pipeline (using a static key for demonstration of wiring)
                        let secret = [42u8; 32];
                        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret);
                        let frame = [quin];
                        let sub_root = crate::agency::compute_scoped_merkle_root(&frame, intent.principal_did_hash);
                        let _signature = crate::agency::sign_agency_root(&signing_key, &sub_root);
                        
                        // In production, the signature and quin would be passed to SuperBlockWriter
                    }
                }
                return OrchestrationResult::Blocked { rule_violated, reason };
            }
            WebizenVerdict::Sanitised { .. } => { /* intent was scrubbed; proceed with caution */ }
            WebizenVerdict::Permit => {}
        }

        // 2. Inference
        let output = match agent.infer(prompt, graph_context) {
            Ok(o) => o,
            Err(e) => return OrchestrationResult::Failed(format!("{:?}", e)),
        };

        // 3. Post-flight: validate output grounding
        match agent.validate_output(&output) {
            WebizenVerdict::Deny { rule_violated, reason, conduct_record } => {
                if let Some(quin) = conduct_record {
                    if let Ok(mut wal) = crate::wal::WriteAheadLog::open(".qualia_conduct.wal") {
                        let _ = wal.append_mutation(&quin);
                    }
                }
                OrchestrationResult::Blocked { rule_violated, reason }
            }
            _ => OrchestrationResult::Committed {
                text: output.text,
                provenance_quins: output.provenance_quins,
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::llm_agent::{AgentIntent, LocalLlmAgent, SANCTUARY_SCOPE_WEBIZEN};

    #[test]
    pub fn qualia_validate_ring_buffer() {}

    #[test]
    fn test_orchestrator_full_permit_path() {
        let agent = LocalLlmAgent::new("did:git:orch-test", "model.gguf");
        let intent = AgentIntent {
            intent_predicate: 0x1234,
            requested_graph_scope: vec![0xABCD],
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: 0,
        };
        let orch = TaskOrchestrator::new(Box::new(NullThermalGovernor));
        let result = orch.orchestrate_inference(&agent, "Summarise my health graph.", "some_graph_bytes", intent);
        assert!(matches!(result, OrchestrationResult::Committed { .. }));
    }

    #[test]
    fn test_orchestrator_blocks_sanctuary_intent() {
        let agent = LocalLlmAgent::new("did:git:orch-test", "model.gguf");
        let intent = AgentIntent {
            intent_predicate: 0x1234,
            requested_graph_scope: vec![SANCTUARY_SCOPE_WEBIZEN],
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: 0,
        };
        let orch = TaskOrchestrator::new(Box::new(NullThermalGovernor));
        let result = orch.orchestrate_inference(&agent, "Show me sanctuary data.", "ctx", intent);
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
            requires_network: false,
            ilp_offer_micro_cents: 0,
            principal_did_hash: 0,
        };
        let orch = TaskOrchestrator::new(Box::new(MockCriticalThermalGovernor));
        let result = orch.orchestrate_inference(&agent, "Query", "ctx", intent);
        assert!(matches!(result, OrchestrationResult::Blocked { rule_violated: 0xDEADBEEF, .. }));
    }

    #[test]
    fn test_async_scrub_lock_invariant() {
        let orch = TaskOrchestrator::new(Box::new(NullThermalGovernor));
        
        // 1. Initially it should load fine
        assert!(orch.load_model(123).is_ok());
        
        // 2. Trigger an eviction (spawns background thread to scrub)
        orch.evict_model(123);
        
        // 3. IMMEDIATELY try to load a new model. The lock should reject it.
        let load_result = orch.load_model(456);
        assert!(load_result.is_err(), "Orchestrator violated mechanical sympathy! Mapped model while Swarm worker was still scrubbing.");
        
        // 4. Wait for the background Swarm worker to complete its duty of care
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        // 5. Try loading again. The lock should be cleared.
        let second_load_result = orch.load_model(456);
        assert!(second_load_result.is_ok(), "Orchestrator failed to load model after scrubbing lock cleared.");
        
        // Ensure Webizen VM logic handles yielding
        let mut vm = crate::logic::WebizenVM::with_scrubbing_lock(orch.scrubbing_lock.clone());
        let bytecode = vec![crate::logic::WebizenOpcode::LoadModel(999)];
        vm.load_bytecode(&bytecode);
        
        let quin = crate::QualiaQuin { subject: 0, predicate: 0, object: 0, context: 0, metadata: 0, parity: 0 };
        
        // If we trigger evict again, the VM should yield on LoadModel
        orch.evict_model(999);
        let exec_result = vm.execute_implication(&quin);
        assert!(exec_result.is_none());
        assert_eq!(vm.yielded_op, Some(crate::logic::WebizenOpcode::LoadModel(999)));
        
        // Wait for scrub to clear
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
