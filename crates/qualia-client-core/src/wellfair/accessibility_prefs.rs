//! Owner accessibility preferences — persisted under storage root.

use std::fs;
use std::path::Path;

use super::host_state::AccessibilityPreferences;

pub const PREFS_FILE: &str = "wellfair/accessibility.json";

pub fn load(storage_root: impl AsRef<Path>) -> AccessibilityPreferences {
    let path = storage_root.as_ref().join(PREFS_FILE);
    if !path.exists() {
        return AccessibilityPreferences::default();
    }
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => AccessibilityPreferences::default(),
    }
}

pub fn save(
    storage_root: impl AsRef<Path>,
    prefs: &AccessibilityPreferences,
) -> std::io::Result<()> {
    let path = storage_root.as_ref().join(PREFS_FILE);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(prefs)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    fs::write(&path, text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessibility_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let prefs = AccessibilityPreferences {
            high_contrast: true,
            reduced_motion: true,
            text_scale_percent: 125,
            screen_reader_hints: false,
        };
        save(dir.path(), &prefs).unwrap();
        let loaded = load(dir.path());
        assert_eq!(loaded, prefs);
    }
}