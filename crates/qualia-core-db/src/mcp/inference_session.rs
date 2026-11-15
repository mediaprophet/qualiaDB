//! Local host delegation to the MCP inference surface. Tool arguments cannot
//! install, select or replace this authenticated authority.
use crate::llm_agent::{native_request::NativeProductSession, LocalLlmAgent};
use std::sync::{Arc, Mutex, OnceLock};

static SESSION: OnceLock<Mutex<Option<Arc<NativeProductSession>>>> = OnceLock::new();

/// Privileged in-process setup only; never register this as an MCP tool.
pub fn configure(session: Arc<NativeProductSession>) -> Result<(), String> {
    session.bind().map_err(|e| e.to_string())?;
    *SESSION.get_or_init(|| Mutex::new(None)).lock()
        .map_err(|_| "MCP inference session unavailable")? = Some(session);
    Ok(())
}

pub fn disconnect() {
    if let Some(slot) = SESSION.get() {
        *slot.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

/// Every tool invocation obtains a fresh, owner-validated single-use request.
pub fn bind(agent: &LocalLlmAgent) -> Result<(), String> {
    let session = SESSION.get().ok_or("Native host must bind MCP inference authority")?
        .lock().map_err(|_| "MCP inference session unavailable")?
        .clone().ok_or("Native host must bind MCP inference authority")?;
    agent.bind_native_governed_request(session.bind().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
