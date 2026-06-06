
extern crate alloc;

use alloc::vec::Vec;
use crate::llm_agent::AgentBackend;
use crate::webizen::SlgOpcode;

/// Defines a specialized capability environment.
/// Loaded via external `.chk` files or defined natively.
#[derive(Debug, Clone)]
pub struct CapabilityProfile {
    pub profile_id: u64, // e.g., q_hash("profile:health")
    
    // Active Native Engines. If empty, all are theoretically active unless masked by other rules.
    // If populated, acts as an allow-list mask over the CAPABILITY_REGISTRY.
    pub active_engines: Vec<SlgOpcode>,
    
    // Ontology namespaces mapped into the active LLM context
    pub loaded_ontologies: Vec<u64>, // e.g., q_hash("namespace:Bio2RDF")
    
    // LLM Backend Requirements
    pub preferred_backend: AgentBackend,
    
    // Hard Intent Frame Constraints. If the LLM agent declares an intent outside this list,
    // the Webizen VM will instantly Deny the request.
    pub permitted_intent_frames: Vec<u64>, 
}

impl CapabilityProfile {
    /// Returns true if the requested engine opcode is permitted by this profile.
    pub fn allows_engine(&self, opcode: &SlgOpcode) -> bool {
        // Empty allow-list means no engine restrictions at the profile layer
        if self.active_engines.is_empty() {
            return true;
        }
        
        // Basic equality check. Note: For opcodes with data payloads (like NativeEpistemicEval(u8)),
        // this requires an exact match or a custom discriminant matcher.
        // For our zero-heap constraint, we do a simple byte-level discriminant check if needed,
        // but exact `PartialEq` is implemented for `SlgOpcode`.
        self.active_engines.contains(opcode)
    }

    /// Returns true if the requested intent frame is permitted.
    pub fn allows_intent(&self, intent_hash: u64) -> bool {
        if self.permitted_intent_frames.is_empty() {
            return true;
        }
        self.permitted_intent_frames.contains(&intent_hash)
    }

    /// Returns true if the requested ontology is actively loaded.
    pub fn has_ontology(&self, namespace_hash: u64) -> bool {
        self.loaded_ontologies.contains(&namespace_hash)
    }
}
