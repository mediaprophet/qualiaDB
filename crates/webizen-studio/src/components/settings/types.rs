use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsSection {
    #[default]
    Health,
    Data,
    Models,
    People,
    Privacy,
    Appearance,
    Device,
    Backup,
    Services,
    Technical,
}

impl SettingsSection {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Health => "Setup health",
            Self::Data => "Data & memory",
            Self::Models => "AI instruments",
            Self::People => "People & reachability",
            Self::Privacy => "Privacy & sanctuary",
            Self::Appearance => "Appearance & access",
            Self::Device => "Person & devices",
            Self::Backup => "Backup & recovery",
            Self::Services => "Services & updates",
            Self::Technical => "All technical settings",
        }
    }

    pub const fn search_terms(self) -> &'static str {
        match self {
            Self::Health => "setup ready attention repair status",
            Self::Data => "data storage path quota memory migrate disk",
            Self::Models => "ai model llm gguf p64 ollama inference",
            Self::People => "people network reachability mesh mail domain solid",
            Self::Privacy => "privacy sanctuary vault consent sharing keys",
            Self::Appearance => "appearance theme colour contrast accessibility",
            Self::Device => "person device apparatus fleet identity webid transfer bundle multi-machine job hardware gpu cpu",
            Self::Backup => "backup recovery restore export import",
            Self::Services => "services daemon updates logs ports",
            Self::Technical => "advanced technical raw configuration providers",
        }
    }
}

pub const ALL_SECTIONS: [SettingsSection; 10] = [
    SettingsSection::Health,
    SettingsSection::Data,
    SettingsSection::Models,
    SettingsSection::People,
    SettingsSection::Privacy,
    SettingsSection::Appearance,
    SettingsSection::Device,
    SettingsSection::Backup,
    SettingsSection::Services,
    SettingsSection::Technical,
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentConfigSnapshot {
    pub storage_path: String,
    pub storage_quota_gb: u64,
    pub base_connectivity_cost_ilp: u64,
    pub daemon_host: String,
    pub daemon_port: u16,
    pub inference_backend: String,
    pub settings_port: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentQaSnapshot {
    pub schema_version: u32,
    pub captured_at_unix: u64,
    pub setup: serde_json::Value,
    pub config: AgentConfigSnapshot,
    pub active_model: Option<String>,
    pub model_status: serde_json::Value,
    pub hardware: serde_json::Value,
    pub daemon_status: String,
    pub mail_receiver: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentQaModelProbe {
    pub schema_version: u32,
    pub passed: bool,
    pub active_model: Option<String>,
    pub committed: bool,
    pub duration_ms: u64,
    pub output_sample: String,
    pub block_reason: Option<String>,
    pub cleanup_succeeded: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_section_is_findable_by_its_label() {
        for section in ALL_SECTIONS {
            let haystack = format!(
                "{} {}",
                section.label().to_ascii_lowercase(),
                section.search_terms()
            );
            assert!(haystack.contains(
                section
                    .label()
                    .split_whitespace()
                    .next()
                    .unwrap()
                    .to_ascii_lowercase()
                    .as_str()
            ));
        }
    }
}
