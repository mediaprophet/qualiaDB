#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

#[command]
pub fn wellfair_add_ledger_entry(
    app: AppHandle,
    description: String,
    amount_cents: f64,
    currency: String,
    category: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let mut entry = wellfare_core::finance::LedgerEntry::new(
            description,
            amount_cents.round() as i64,
            currency,
            wellfair_now_unix(),
        );
        entry.category = category.filter(|s| !s.is_empty());
        let committed = host.add_ledger_entry(&entry)?;
        serde_json::to_string(&committed).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_ledger_balance(app: AppHandle, limit: usize) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let balance = host.ledger_balance(limit)?;
        serde_json::to_string(&balance).map_err(|e| e.to_string())
    })?
}

