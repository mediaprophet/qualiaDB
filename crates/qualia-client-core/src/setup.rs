//! Persisted first-run setup state for the desktop shell.
//!
//! Setup is intentionally separate from `AgentConfig`: configuration may be
//! saved more than once during onboarding, while the gate must only disappear
//! after the required choices have been reviewed.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::state::{app_meta_dir, config_file_path};

pub const SETUP_STATE_VERSION: u32 = 5;

/// First-run gate: local foundations that do **not** require peers, domains, or live mesh.
///
/// Relational and network configuration is progressive — see [`PROGRESSIVE_SETUP_PATHS`].
pub const REQUIRED_SETUP_STEPS: [&str; 8] = [
    "welcome",
    "storage",
    "control",
    "device",
    "inference",
    "relations", // how you want to be known (local profile only)
    "care",
    "ready",
];

/// Paths that become meaningful after the apparatus is running and people can connect.
/// These never block the first-run gate; they surface in Setup Health / Relations over time.
pub const PROGRESSIVE_SETUP_PATHS: [&str; 6] = [
    "reachability",      // private / mesh / public posture
    "assurance",         // backup destination + verified restore
    "people_connections", // invites, contacts, groups
    "domains_mail",      // front door, DNS, purpose mailboxes
    "care_records",      // provenance-backed health material
    "peer_agreements",   // multi-party norms once peers exist
];

/// Social and tenure context for the machine Webizen is being installed on.
///
/// This is not hardware telemetry. It answers: whose machine, only machine or not,
/// one person or several, and (if several) what kind of shared setting. All fields
/// are optional plain tokens; empty means “prefer not to say / not set yet”.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceContext {
    /// e.g. `owned_by_me` | `employer` | `school` | `organisation` | `shared_household`
    /// | `borrowed_or_public` | `prefer_not_say` | `other`
    #[serde(default)]
    pub ownership: String,
    /// `only_machine` | `one_of_several` | `prefer_not_say`
    #[serde(default)]
    pub machine_fleet: String,
    /// `just_me` | `more_than_one` | `prefer_not_say`
    #[serde(default)]
    pub user_scope: String,
    /// When `user_scope` is multi-person: `family` | `household` | `work` | `school`
    /// | `organisation` | `public_shared` | `mixed` | `other` | `prefer_not_say`
    #[serde(default)]
    pub shared_setting: String,
    /// Free-text clarification the person chooses to add (optional).
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetupProfile {
    #[serde(default)]
    pub preferred_name: String,
    #[serde(default)]
    pub locale: String,
    #[serde(default)]
    pub timezone: String,
    #[serde(default)]
    pub accessibility_needs: Vec<String>,
    #[serde(default)]
    pub interests: Vec<String>,
    #[serde(default)]
    pub preferred_ontologies: Vec<String>,
    #[serde(default)]
    pub care_priorities: Vec<String>,
    #[serde(default)]
    pub qapp_goals: Vec<String>,
    /// Situation of this machine (ownership, sole vs fleet, single vs multi-user).
    #[serde(default)]
    pub device_context: DeviceContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetupState {
    pub version: u32,
    #[serde(default)]
    pub completed: bool,
    #[serde(default = "default_current_step")]
    pub current_step: String,
    #[serde(default)]
    pub completed_steps: Vec<String>,
    #[serde(default)]
    pub migrated_from_legacy_config: bool,
    #[serde(default)]
    pub profile: SetupProfile,
}

fn default_current_step() -> String {
    "welcome".to_string()
}

impl Default for SetupState {
    fn default() -> Self {
        Self {
            version: SETUP_STATE_VERSION,
            completed: false,
            current_step: default_current_step(),
            completed_steps: Vec::new(),
            migrated_from_legacy_config: false,
            profile: SetupProfile::default(),
        }
    }
}

pub fn update_setup_profile(profile: SetupProfile) -> Result<SetupState, String> {
    let mut state = get_setup_state()?;
    state.profile = profile;
    save_setup_state_to(&setup_state_path(), &state)?;
    // Keep apparatus device_context in the fleet identity plane (person ≠ machine).
    let _ = crate::identity_plane::sync_local_device_context(&state.profile.device_context);
    Ok(state)
}

pub fn setup_state_path() -> PathBuf {
    app_meta_dir().join("setup-state.json")
}

pub fn get_setup_state() -> Result<SetupState, String> {
    load_setup_state_from(&setup_state_path(), config_file_path().exists())
}

pub fn complete_setup_step(step: String) -> Result<SetupState, String> {
    let step = step.trim();
    let is_required = REQUIRED_SETUP_STEPS.contains(&step);
    let is_progressive = PROGRESSIVE_SETUP_PATHS.contains(&step);
    if !is_required && !is_progressive {
        return Err(format!("Unknown setup step: {step}"));
    }

    let mut state = get_setup_state()?;
    // Progressive paths may be recorded after first-run is already complete.
    if state.completed && is_required {
        return Ok(state);
    }
    if !state.completed_steps.iter().any(|done| done == step) {
        state.completed_steps.push(step.to_string());
    }
    if !state.completed {
        state.current_step = next_incomplete_step(&state).unwrap_or("ready").to_string();
    }
    save_setup_state_to(&setup_state_path(), &state)?;
    Ok(state)
}

pub fn finish_setup() -> Result<SetupState, String> {
    let mut state = get_setup_state()?;
    let missing: Vec<&str> = REQUIRED_SETUP_STEPS
        .iter()
        .copied()
        .filter(|required| {
            !state
                .completed_steps
                .iter()
                .any(|completed| completed == required)
        })
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "Complete the required setup steps first: {}",
            missing.join(", ")
        ));
    }

    state.completed = true;
    state.current_step = "complete".to_string();
    save_setup_state_to(&setup_state_path(), &state)?;
    // Mint / refresh person + local apparatus under the fleet plane so multi-device
    // job targeting has a real local device_id (not OS account, not “the person”).
    crate::identity_plane::ensure_local_apparatus(Some(state.profile.device_context.clone()))?;
    Ok(state)
}

fn next_incomplete_step(state: &SetupState) -> Option<&'static str> {
    REQUIRED_SETUP_STEPS.iter().copied().find(|required| {
        !state
            .completed_steps
            .iter()
            .any(|completed| completed == required)
    })
}

fn load_setup_state_from(path: &Path, legacy_config_exists: bool) -> Result<SetupState, String> {
    match fs::read_to_string(path) {
        Ok(text) => {
            let mut state: SetupState = serde_json::from_str(&text)
                .map_err(|error| format!("Could not read setup state: {error}"))?;
            state.version = SETUP_STATE_VERSION;
            Ok(state)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if legacy_config_exists {
                let state = SetupState {
                    completed: true,
                    current_step: "complete".to_string(),
                    completed_steps: REQUIRED_SETUP_STEPS
                        .iter()
                        .map(|step| (*step).to_string())
                        .collect(),
                    migrated_from_legacy_config: true,
                    ..SetupState::default()
                };
                save_setup_state_to(path, &state)?;
                Ok(state)
            } else {
                Ok(SetupState::default())
            }
        }
        Err(error) => Err(format!("Could not read setup state: {error}")),
    }
}

fn save_setup_state_to(path: &Path, state: &SetupState) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create setup directory: {error}"))?;
    }
    let json = serde_json::to_string_pretty(state)
        .map_err(|error| format!("Could not encode setup state: {error}"))?;
    fs::write(path, json).map_err(|error| format!("Could not save setup state: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_install_starts_at_welcome() {
        let dir = tempfile::tempdir().unwrap();
        let state = load_setup_state_from(&dir.path().join("setup.json"), false).unwrap();
        assert!(!state.completed);
        assert_eq!(state.current_step, "welcome");
        assert!(state.completed_steps.is_empty());
    }

    #[test]
    fn legacy_config_is_migrated_without_showing_the_gate() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("setup.json");
        let state = load_setup_state_from(&path, true).unwrap();
        assert!(state.completed);
        assert!(state.migrated_from_legacy_config);
        assert!(path.exists());
    }

    #[test]
    fn required_step_order_is_deterministic() {
        let mut state = SetupState::default();
        assert_eq!(next_incomplete_step(&state), Some("welcome"));
        state.completed_steps.push("welcome".into());
        assert_eq!(next_incomplete_step(&state), Some("storage"));
        state.completed_steps.push("storage".into());
        for step in REQUIRED_SETUP_STEPS.iter().skip(2) {
            state.completed_steps.push((*step).into());
        }
        assert_eq!(next_incomplete_step(&state), None);
    }

    #[test]
    fn first_run_gate_does_not_require_relational_paths() {
        assert!(!REQUIRED_SETUP_STEPS.contains(&"reachability"));
        assert!(!REQUIRED_SETUP_STEPS.contains(&"assurance"));
        assert!(PROGRESSIVE_SETUP_PATHS.contains(&"reachability"));
        assert!(PROGRESSIVE_SETUP_PATHS.contains(&"people_connections"));
        assert_eq!(REQUIRED_SETUP_STEPS.len(), 8);
    }

    #[test]
    fn older_setup_state_gets_an_empty_profile() {
        let state: SetupState = serde_json::from_str(
            r#"{"version":2,"completed":false,"current_step":"care","completed_steps":[]}"#,
        )
        .unwrap();
        assert_eq!(state.profile, SetupProfile::default());
        assert_eq!(state.profile.device_context, DeviceContext::default());
    }

    #[test]
    fn device_context_deserializes_on_v5_profile() {
        let state: SetupState = serde_json::from_str(
            r#"{
                "version":5,
                "completed":false,
                "current_step":"device",
                "completed_steps":[],
                "profile":{
                    "device_context":{
                        "ownership":"owned_by_me",
                        "machine_fleet":"one_of_several",
                        "user_scope":"more_than_one",
                        "shared_setting":"family",
                        "notes":""
                    }
                }
            }"#,
        )
        .unwrap();
        assert_eq!(state.profile.device_context.ownership, "owned_by_me");
        assert_eq!(state.profile.device_context.shared_setting, "family");
    }
}
