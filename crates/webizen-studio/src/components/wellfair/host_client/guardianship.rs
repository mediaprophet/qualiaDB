//! Guardianship approval escrow

use super::*;
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;


#[derive(Debug, Clone, Deserialize)]
pub struct GuardianshipProposalDto {
    pub proposal_id: String,
    pub principal_did: String,
    pub proxy_did: String,
    pub escrowed_kind: String,
    pub reason: String,
    pub created_unix: u32,
    /// "pending" | "ratified" | "denied".
    pub state: String,
    pub approvals: u8,
    pub threshold: u8,
    pub denied_by: Option<String>,
    pub denial_reason: Option<String>,
    pub committed: bool,
}

#[cfg(target_arch = "wasm32")]
pub async fn propose_proxy_condition(proxy_did: &str, label: &str) -> Result<String, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"proxyDid".into(), &wasm_bindgen::JsValue::from_str(proxy_did))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"label".into(), &wasm_bindgen::JsValue::from_str(label))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_propose_proxy_condition", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    js.as_string().ok_or_else(|| "proxy proposal response not JSON".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn propose_proxy_condition(_proxy_did: &str, _label: &str) -> Result<String, String> {
    Err("Guardianship requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_guardianship_proposals(
    limit: usize,
) -> Result<Vec<GuardianshipProposalDto>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"limit".into(), &wasm_bindgen::JsValue::from(limit as u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_list_guardianship_proposals", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "proposals response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_guardianship_proposals(
    _limit: usize,
) -> Result<Vec<GuardianshipProposalDto>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
pub async fn vote_guardianship_proposal(
    proposal_id: &str,
    guardian_did: &str,
    approve: bool,
    reason: Option<&str>,
) -> Result<GuardianshipProposalDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"proposalId".into(), &wasm_bindgen::JsValue::from_str(proposal_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"guardianDid".into(), &wasm_bindgen::JsValue::from_str(guardian_did))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"approve".into(), &wasm_bindgen::JsValue::from_bool(approve))
        .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(r) = reason {
        js_sys::Reflect::set(&args, &"reason".into(), &wasm_bindgen::JsValue::from_str(r))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_vote_guardianship_proposal", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "vote response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn vote_guardianship_proposal(
    _proposal_id: &str,
    _guardian_did: &str,
    _approve: bool,
    _reason: Option<&str>,
) -> Result<GuardianshipProposalDto, String> {
    Err("Guardianship requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_assistance_need(
    category: &str,
    description: &str,
    urgency: &str,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"category".into(), &wasm_bindgen::JsValue::from_str(category))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"description".into(), &wasm_bindgen::JsValue::from_str(description))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"urgency".into(), &wasm_bindgen::JsValue::from_str(urgency))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_assistance_need", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "assistance response was not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_assistance_need(
    _category: &str,
    _description: &str,
    _urgency: &str,
) -> Result<HealthRecordDto, String> {
    Err("Assistance needs require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_welfare_stream(
    program_name: &str,
    reference: Option<&str>,
    status: &str,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"programName".into(), &wasm_bindgen::JsValue::from_str(program_name))
        .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(r) = reference {
        js_sys::Reflect::set(&args, &"reference".into(), &wasm_bindgen::JsValue::from_str(r))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    js_sys::Reflect::set(&args, &"status".into(), &wasm_bindgen::JsValue::from_str(status))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_welfare_stream", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "welfare stream response was not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_welfare_stream(
    _program_name: &str,
    _reference: Option<&str>,
    _status: &str,
) -> Result<HealthRecordDto, String> {
    Err("Welfare streams require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_government_letter(
    sender: &str,
    subject: &str,
    action_required: bool,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"sender".into(), &wasm_bindgen::JsValue::from_str(sender))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"subject".into(), &wasm_bindgen::JsValue::from_str(subject))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"actionRequired".into(), &wasm_bindgen::JsValue::from(action_required))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_government_letter", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "letter response was not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_government_letter(
    _sender: &str,
    _subject: &str,
    _action_required: bool,
) -> Result<HealthRecordDto, String> {
    Err("Government letters require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_government_letter_attachment_from_path(
    sender: &str,
    subject: &str,
    action_required: bool,
    path: &str,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"sender".into(), &wasm_bindgen::JsValue::from_str(sender))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"subject".into(), &wasm_bindgen::JsValue::from_str(subject))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"actionRequired".into(), &wasm_bindgen::JsValue::from(action_required))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"path".into(), &wasm_bindgen::JsValue::from_str(path))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_government_letter_attachment_from_path", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "letter attachment response not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_government_letter_attachment_from_path(
    _sender: &str,
    _subject: &str,
    _action_required: bool,
    _path: &str,
) -> Result<HealthRecordDto, String> {
    Err("Government letter attachments require the Tauri desktop host".into())
}

/// One quarantined-inbox row (subset of the host `InboxRecord`).
#[derive(Debug, Clone, Deserialize)]
pub struct SyncInboxOpDto {
    pub operation_id: String,
    pub kind: String,
    pub lamport: u64,
    pub sensitivity: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyncInboxOutcomeDto {
    pub state: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyncInboxRecordDto {
    pub operation: SyncInboxOpDto,
    pub outcome: SyncInboxOutcomeDto,
    pub admitted_unix: u32,
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_sync_inbox(limit: usize) -> Result<Vec<SyncInboxRecordDto>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"limit".into(), &wasm_bindgen::JsValue::from(limit as u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_list_sync_inbox", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "sync inbox response was not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_sync_inbox(_limit: usize) -> Result<Vec<SyncInboxRecordDto>, String> {
    Ok(vec![])
}

/// A note held in the encrypted Sanctuary vault (never leaves the desktop unencrypted).
#[derive(Debug, Clone, Deserialize)]
pub struct SanctuaryVaultNoteDto {
    pub id: String,
    pub body: String,
    pub created_at_unix: u32,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Deserialize)]
struct SanctuaryVaultListDto {
    lane: String,
    notes: Vec<SanctuaryVaultNoteDto>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Deserialize)]
struct SanctuaryVaultConfiguredDto {
    configured: bool,
}

#[cfg(target_arch = "wasm32")]
pub async fn sanctuary_vault_configured() -> Result<bool, String> {
    let js = tauri_invoke("wellfair_sanctuary_vault_configured", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "vault status not JSON".to_string())?;
    let dto: SanctuaryVaultConfiguredDto = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(dto.configured)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sanctuary_vault_configured() -> Result<bool, String> {
    Ok(false)
}

#[cfg(target_arch = "wasm32")]
pub async fn setup_sanctuary_vault(real_pin: &str, decoy_pin: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"realPin".into(), &wasm_bindgen::JsValue::from_str(real_pin))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"decoyPin".into(), &wasm_bindgen::JsValue::from_str(decoy_pin))
        .map_err(|_| "failed to build invoke args".to_string())?;
    tauri_invoke("wellfair_setup_sanctuary_vault", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn setup_sanctuary_vault(_real_pin: &str, _decoy_pin: &str) -> Result<(), String> {
    Err("Sanctuary vault requires the Tauri desktop host".into())
}

