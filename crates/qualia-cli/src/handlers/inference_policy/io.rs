//! Bounded operator input. Parse diagnostics do not echo file content.
use serde::de::DeserializeOwned;
use std::{io::Read, path::Path};
pub(super) fn bytes(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Cannot open operator input: {e}"))?;
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > limit as u64 {
        return Err("Operator input exceeds its byte limit or is not a regular file".into());
    }
    let mut result = Vec::with_capacity(limit + 1);
    file.take(limit as u64 + 1)
        .read_to_end(&mut result)
        .map_err(|e| e.to_string())?;
    if result.len() > limit {
        return Err("Operator input grew beyond its byte limit".into());
    }
    Ok(result)
}
pub(super) fn json<T: DeserializeOwned>(path: &Path, limit: usize) -> Result<T, String> {
    serde_json::from_slice(&bytes(path, limit)?).map_err(|e| {
        format!(
            "Invalid operator JSON at line {}, column {}",
            e.line(),
            e.column()
        )
    })
}
pub(super) fn policy(path: &Path) -> Result<String, String> {
    // Parse here so serde's unknown-field/variant diagnostics cannot echo input
    // through the client API. Lists use the real bounded policy deserializer.
    let rules: qualia_core_db::llm_agent::governed::PolicyRules = json(path, 128 * 1024)?;
    let encoded = serde_json::to_string(&rules)
        .map_err(|_| "Cannot encode validated operator policy".to_string())?;
    if encoded.len() > 128 * 1024 {
        return Err("Encoded operator policy exceeds its byte limit".into());
    }
    Ok(encoded)
}
