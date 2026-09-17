//! Agent characteristics KB — behaviour-based profiling (T59).
//!
//! Logs characteristics of agents (including AI agents) from behaviour.
//! Local inference first, then packs for jurisdictions. The profile is
//! built from observed turns, not from self-report.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Observed characteristics of an agent (T59). Built from behaviour,
/// not from self-report. Used for jurisdictional compliance and audit.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentCharacteristics {
    /// Agent slug this profile describes.
    pub agent_slug: String,
    /// Number of inference turns observed.
    pub turns_observed: u64,
    /// Average response length in characters.
    pub avg_response_chars: f64,
    /// Total characters across all responses (for running average).
    pub total_response_chars: u64,
    /// Capabilities most frequently invoked (top 5).
    pub frequent_capabilities: Vec<String>,
    /// Capability invocation counts (for computing top 5).
    #[serde(default)]
    pub capability_counts: HashMap<String, u64>,
    /// Whether this agent has ever produced a disclosure denial.
    pub has_denied_disclosure: bool,
    /// Whether this agent has ever triggered a deontic interrupt.
    pub has_triggered_interrupt: bool,
    /// Unix timestamp of last observation.
    pub last_observed_unix: u64,
}

impl AgentCharacteristics {
    /// Create a fresh profile for the given agent slug.
    pub fn new(slug: &str) -> Self {
        Self {
            agent_slug: slug.to_string(),
            ..Self::default()
        }
    }

    /// Record an inference turn: updates turn count, response length
    /// average, and capability frequency.
    pub fn record_turn(&mut self, capability: &str, response_len: usize) {
        self.turns_observed += 1;
        self.total_response_chars += response_len as u64;
        self.avg_response_chars = self.total_response_chars as f64 / self.turns_observed as f64;
        *self
            .capability_counts
            .entry(capability.to_string())
            .or_insert(0) += 1;
        self.recompute_frequent_capabilities();
        self.last_observed_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }

    /// Record that this agent produced a disclosure denial.
    pub fn record_denial(&mut self) {
        self.has_denied_disclosure = true;
    }

    /// Record that this agent triggered a deontic interrupt.
    pub fn record_interrupt(&mut self) {
        self.has_triggered_interrupt = true;
    }

    /// Recompute the top-5 frequent capabilities from the counts map.
    fn recompute_frequent_capabilities(&mut self) {
        let mut entries: Vec<(String, u64)> = self
            .capability_counts
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        self.frequent_capabilities = entries.into_iter().take(5).map(|(k, _)| k).collect();
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// A store that persists agent characteristics to a JSON file.
pub struct CharacteristicsStore {
    dir: std::path::PathBuf,
}

impl CharacteristicsStore {
    /// Create a store rooted at the given directory.
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
        }
    }

    /// Path for a given agent slug's profile.
    fn path_for(&self, slug: &str) -> std::path::PathBuf {
        self.dir.join(format!("{slug}.json"))
    }

    /// Save a profile to disk.
    pub fn save(&self, profile: &AgentCharacteristics) -> std::io::Result<()> {
        let path = self.path_for(&profile.agent_slug);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = profile
            .to_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Load a profile from disk. Returns None if no profile exists.
    pub fn load(&self, slug: &str) -> std::io::Result<Option<AgentCharacteristics>> {
        let path = self.path_for(slug);
        if !path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(path)?;
        let profile = AgentCharacteristics::from_json(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(profile))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_characteristics_empty() {
        let c = AgentCharacteristics::new("test-agent");
        assert_eq!(c.agent_slug, "test-agent");
        assert_eq!(c.turns_observed, 0);
        assert_eq!(c.avg_response_chars, 0.0);
        assert!(c.frequent_capabilities.is_empty());
        assert!(!c.has_denied_disclosure);
        assert!(!c.has_triggered_interrupt);
    }

    #[test]
    fn record_turn_updates_stats() {
        let mut c = AgentCharacteristics::new("agent-a");
        c.record_turn("graph.query", 100);
        c.record_turn("graph.query", 200);
        c.record_turn("pulse.publish", 50);
        assert_eq!(c.turns_observed, 3);
        assert!((c.avg_response_chars - 116.666).abs() < 0.01);
        assert_eq!(c.frequent_capabilities[0], "graph.query");
        assert!(c
            .frequent_capabilities
            .contains(&"pulse.publish".to_string()));
    }

    #[test]
    fn record_denial_sets_flag() {
        let mut c = AgentCharacteristics::new("agent-b");
        assert!(!c.has_denied_disclosure);
        c.record_denial();
        assert!(c.has_denied_disclosure);
    }

    #[test]
    fn record_interrupt_sets_flag() {
        let mut c = AgentCharacteristics::new("agent-c");
        assert!(!c.has_triggered_interrupt);
        c.record_interrupt();
        assert!(c.has_triggered_interrupt);
    }

    #[test]
    fn json_roundtrip() {
        let mut c = AgentCharacteristics::new("agent-d");
        c.record_turn("math.abs", 42);
        c.record_denial();
        let json = c.to_json().unwrap();
        let restored = AgentCharacteristics::from_json(&json).unwrap();
        assert_eq!(restored.agent_slug, "agent-d");
        assert_eq!(restored.turns_observed, 1);
        assert!(restored.has_denied_disclosure);
        assert_eq!(restored.frequent_capabilities, vec!["math.abs"]);
    }

    #[test]
    fn store_persists_and_loads() {
        let tmp =
            std::env::temp_dir().join(format!("qualia_agent_char_test_{}", std::process::id()));
        let store = CharacteristicsStore::new(&tmp);
        let mut c = AgentCharacteristics::new("agent-e");
        c.record_turn("graph.query", 100);
        c.record_interrupt();
        store.save(&c).unwrap();
        let loaded = store.load("agent-e").unwrap().unwrap();
        assert_eq!(loaded.agent_slug, "agent-e");
        assert_eq!(loaded.turns_observed, 1);
        assert!(loaded.has_triggered_interrupt);
        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn store_load_nonexistent_returns_none() {
        let tmp =
            std::env::temp_dir().join(format!("qualia_agent_char_none_{}", std::process::id()));
        let store = CharacteristicsStore::new(&tmp);
        let result = store.load("nonexistent").unwrap();
        assert!(result.is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn frequent_capabilities_top_5() {
        let mut c = AgentCharacteristics::new("agent-f");
        // Add 7 distinct capabilities with different frequencies.
        for (cap, count) in [
            ("a", 10),
            ("b", 8),
            ("c", 6),
            ("d", 4),
            ("e", 2),
            ("f", 1),
            ("g", 1),
        ] {
            for _ in 0..count {
                c.record_turn(cap, 10);
            }
        }
        // Only top 5 should be in frequent_capabilities.
        assert_eq!(c.frequent_capabilities.len(), 5);
        assert_eq!(c.frequent_capabilities[0], "a");
        assert_eq!(c.frequent_capabilities[1], "b");
        assert_eq!(c.frequent_capabilities[4], "e");
        assert!(!c.frequent_capabilities.contains(&"f".to_string()));
    }
}
