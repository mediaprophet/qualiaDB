//! Active downloads, remote manifest, imported accounts

#![allow(non_snake_case)]


use crate::state::*;
use std::path::PathBuf;


pub fn get_active_downloads() -> Vec<ProgressPayload> {
    let state = crate::state::APP_STATE.get().unwrap();
    state
        .active_downloads
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect()
}

// â”€â”€ Remote manifest fetch â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub async fn fetch_remote_manifest(url: String) -> Result<String, String> {
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Network error: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Response error: {}", e))
}

// â”€â”€ Imported accounts persistence â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn imported_accounts_path() -> PathBuf {
    app_meta_dir().join("imported_accounts.json")
}

pub fn load_imported_accounts() -> Result<serde_json::Value, String> {
    let path = imported_accounts_path();
    if !path.exists() {
        return Ok(serde_json::json!([]));
    }
    let s = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&s).map_err(|e| e.to_string())
}

pub fn save_imported_accounts(accounts: serde_json::Value) -> Result<(), String> {
    let meta = app_meta_dir();
    std::fs::create_dir_all(&meta).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&accounts).map_err(|e| e.to_string())?;
    std::fs::write(imported_accounts_path(), json).map_err(|e| e.to_string())
}

