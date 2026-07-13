//! Runtime Chora navigation state (active world + temporal scrub position).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const STATE_FILE: &str = "wellfair/canvas_state.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanvasNavigationState {
    pub active_world_id: String,
    /// Current temporal scrub position (unix seconds).
    pub temporal_t: u64,
    /// Spawn/decay ramp duration (seconds).
    #[serde(default = "default_ramp")]
    pub ramp_secs: u64,
}

fn default_ramp() -> u64 {
    86_400 // one day fade
}

impl Default for CanvasNavigationState {
    fn default() -> Self {
        Self {
            active_world_id: "q42:world:demo-offline".to_string(),
            temporal_t: 1_750_000_000,
            ramp_secs: default_ramp(),
        }
    }
}

pub fn load(storage_root: impl AsRef<Path>) -> CanvasNavigationState {
    let path = storage_root.as_ref().join(STATE_FILE);
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => CanvasNavigationState::default(),
    }
}

pub fn save(storage_root: impl AsRef<Path>, state: &CanvasNavigationState) -> std::io::Result<()> {
    let path: PathBuf = storage_root.as_ref().join(STATE_FILE);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(state).map_err(|e| std::io::Error::other(e.to_string()))?;
    fs::write(&tmp, &json)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let dir = std::env::temp_dir().join(format!("chora-state-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut s = CanvasNavigationState::default();
        s.temporal_t = 999;
        save(&dir, &s).unwrap();
        assert_eq!(load(&dir).temporal_t, 999);
        let _ = fs::remove_dir_all(&dir);
    }
}