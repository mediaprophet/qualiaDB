//! Persisted first-run setup state for the desktop shell.
//!
//! Setup is intentionally separate from `AgentConfig`: configuration may be
//! saved more than once during onboarding, while the gate must only disappear
//! after the required choices have been reviewed.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::state::{app_meta_dir, config_file_path};

pub const SETUP_STATE_VERSION: u32 = 1;
pub const REQUIRED_SETUP_STEPS: [&str; 3] = ["welcome", "storage", "inference"];

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
        }
    }
}

pub fn setup_state_path() -> PathBuf {
    app_meta_dir().join("setup-state.json")
}

pub fn get_setup_state() -> Result<SetupState, String> {
    load_setup_state_from(&setup_state_path(), config_file_path().exists())
}

pub fn complete_setup_step(step: String) -> Result<SetupState, String> {
    let step = step.trim();
    if !REQUIRED_SETUP_STEPS.contains(&step) {
        return Err(format!("Unknown setup step: {step}"));
    }

    let mut state = get_setup_state()?;
    if state.completed {
        return Ok(state);
    }
    if !state.completed_steps.iter().any(|done| done == step) {
        state.completed_steps.push(step.to_string());
    }
    state.current_step = next_incomplete_step(&state).unwrap_or("ready").to_string();
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
        state.completed_steps.push("inference".into());
        assert_eq!(next_incomplete_step(&state), None);
    }
}
