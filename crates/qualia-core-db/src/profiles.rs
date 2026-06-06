use crate::llm_agent::AgentBackend;
use crate::webizen::SlgOpcode;

/// A declarative allow-list that constrains what the LLM and Webizen VM
/// are permitted to do within a given session or resource context.
///
/// Profiles are identified by a q_hash of their logical name
/// (e.g. `q_hash("profile:health")`) and are referenced from
/// `McpIntentFrame::active_profile_id`.
#[derive(Debug, Clone)]
pub struct CapabilityProfile {
    /// Stable identity hash — e.g. `q_hash("profile:phi3-mini-edge")`.
    pub profile_id: u64,

    /// If non-empty, acts as an allow-list mask over the CAPABILITY_REGISTRY.
    /// Only the listed `SlgOpcode` variants may be dispatched in this session.
    /// An empty vec means no engine restrictions apply at the profile layer.
    pub active_engines: Vec<SlgOpcode>,

    /// Ontology namespace hashes actively mapped into the LLM context window
    /// (e.g. `q_hash("namespace:Bio2RDF")`).
    pub loaded_ontologies: Vec<u64>,

    /// Preferred inference backend for this profile.
    pub preferred_backend: AgentBackend,

    /// If non-empty, any LLM intent declared outside this set is instantly
    /// denied by the Webizen VM before the model is invoked.
    pub permitted_intent_frames: Vec<u64>,
}

impl CapabilityProfile {
    /// Returns `true` if `opcode` is permitted by this profile's engine allow-list.
    pub fn allows_engine(&self, opcode: &SlgOpcode) -> bool {
        if self.active_engines.is_empty() {
            return true;
        }
        self.active_engines.contains(opcode)
    }

    /// Returns `true` if `intent_hash` is an allowed intent frame.
    pub fn allows_intent(&self, intent_hash: u64) -> bool {
        if self.permitted_intent_frames.is_empty() {
            return true;
        }
        self.permitted_intent_frames.contains(&intent_hash)
    }

    /// Returns `true` if the requested ontology namespace is actively loaded.
    pub fn has_ontology(&self, namespace_hash: u64) -> bool {
        self.loaded_ontologies.contains(&namespace_hash)
    }
}
