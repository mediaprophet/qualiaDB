//! **The person's own score-card weight model** — their authorship of *how their body is read*.
//!
//! The `WeightModel` is the interpretive lens the score-card uses (which systems carry "stress", which carry
//! "resilience", …). The software must not *define* the person through a fixed lens; it offers a **seed
//! suggestion** the person can adopt, edit, or replace, and stores **their** model here. Absence of a stored
//! model means "the person has not authored one yet" — the caller falls back to the seed *suggestion*, never
//! an imposed definition. Forum-internum / Sanctuary-class selfhood config; the person's alone.

use std::path::Path;

use wellfare_core::anatomy::WeightModel;

/// Where the person's authored weight model lives (prefs-style, under the `wellfair/` prefix).
pub const WEIGHT_MODEL_FILE: &str = "wellfair/weight_model.json";

/// Load the person's **authored** weight model, or `None` if they have not authored one (→ the caller uses
/// the seed *suggestion*).
pub fn load(storage_root: impl AsRef<Path>) -> Option<WeightModel> {
    let path = storage_root.as_ref().join(WEIGHT_MODEL_FILE);
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Persist the person's own weight model — their authorship of the interpretation.
pub fn save(storage_root: impl AsRef<Path>, model: &WeightModel) -> Result<(), String> {
    let path = storage_root.as_ref().join(WEIGHT_MODEL_FILE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(model).map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

/// Clear the person's authored model — revert to the seed *suggestion*. Idempotent (no-op if none exists).
pub fn clear(storage_root: impl AsRef<Path>) -> Result<(), String> {
    let path = storage_root.as_ref().join(WEIGHT_MODEL_FILE);
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_model_persists_and_clears_back_to_none() {
        let dir = tempfile::tempdir().unwrap();
        // Nothing authored yet → None (caller uses the seed suggestion).
        assert!(load(dir.path()).is_none());

        // The person authors their own model.
        let mut model = wellfare_core::anatomy::seed_weight_model();
        model.system_weights.push(wellfare_core::anatomy::SystemAspectWeight {
            system_id: "nervous".into(),
            aspect: wellfare_core::anatomy::Aspect::Stress,
            weight_pct: 42,
        });
        save(dir.path(), &model).unwrap();
        assert_eq!(load(dir.path()).as_ref(), Some(&model), "the person's model is theirs, persisted");

        // They can revert to the suggestion.
        clear(dir.path()).unwrap();
        assert!(load(dir.path()).is_none(), "reset returns to the seed suggestion");
    }
}
