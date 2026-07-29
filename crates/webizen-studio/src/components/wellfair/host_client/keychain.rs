//! OS-keychain vault wrapping

use super::*;

#[cfg(target_arch = "wasm32")]
use super::guardianship::SanctuaryVaultListDto;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;

#[cfg(target_arch = "wasm32")]
pub async fn sanctuary_vault_is_keychain_wrapped() -> Result<bool, String> {
    let js = tauri_invoke(
        "wellfair_sanctuary_vault_is_keychain_wrapped",
        wasm_bindgen::JsValue::NULL,
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("wrapped").and_then(|b| b.as_bool()).unwrap_or(false))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sanctuary_vault_is_keychain_wrapped() -> Result<bool, String> {
    Ok(false)
}

/// Create a keychain-wrapped vault; returns the one-time recovery code to display to the user.
#[cfg(target_arch = "wasm32")]
pub async fn setup_sanctuary_vault_wrapped(
    real_pin: &str,
    decoy_pin: &str,
) -> Result<String, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"realPin".into(),
        &wasm_bindgen::JsValue::from_str(real_pin),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(
        &args,
        &"decoyPin".into(),
        &wasm_bindgen::JsValue::from_str(decoy_pin),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_setup_sanctuary_vault_wrapped", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("recovery_code")
        .and_then(|c| c.as_str())
        .unwrap_or_default()
        .to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn setup_sanctuary_vault_wrapped(
    _real_pin: &str,
    _decoy_pin: &str,
) -> Result<String, String> {
    Err("Sanctuary vault requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn sanctuary_vault_unlock_with_recovery(
    pin: &str,
    recovery_code: &str,
) -> Result<String, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"pin".into(), &wasm_bindgen::JsValue::from_str(pin))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(
        &args,
        &"recoveryCode".into(),
        &wasm_bindgen::JsValue::from_str(recovery_code),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_sanctuary_vault_unlock_with_recovery", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("lane")
        .and_then(|l| l.as_str())
        .unwrap_or("real")
        .to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sanctuary_vault_unlock_with_recovery(
    _pin: &str,
    _recovery_code: &str,
) -> Result<String, String> {
    Err("Sanctuary vault requires the Tauri desktop host".into())
}

/// Returns the lane label ("real" | "decoy") the PIN opened.
#[cfg(target_arch = "wasm32")]
pub async fn sanctuary_vault_add_note(pin: &str, body: &str) -> Result<String, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"pin".into(), &wasm_bindgen::JsValue::from_str(pin))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(
        &args,
        &"body".into(),
        &wasm_bindgen::JsValue::from_str(body),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_sanctuary_vault_add_note", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "add note response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("lane")
        .and_then(|l| l.as_str())
        .unwrap_or("real")
        .to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sanctuary_vault_add_note(_pin: &str, _body: &str) -> Result<String, String> {
    Err("Sanctuary vault requires the Tauri desktop host".into())
}

/// Returns (lane label, notes) for whichever lane the PIN opens.
#[cfg(target_arch = "wasm32")]
pub async fn sanctuary_vault_list_notes(
    pin: &str,
) -> Result<(String, Vec<SanctuaryVaultNoteDto>), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"pin".into(), &wasm_bindgen::JsValue::from_str(pin))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_sanctuary_vault_list_notes", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "list notes response not JSON".to_string())?;
    let dto: SanctuaryVaultListDto = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok((dto.lane, dto.notes))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sanctuary_vault_list_notes(
    _pin: &str,
) -> Result<(String, Vec<SanctuaryVaultNoteDto>), String> {
    Err("Sanctuary vault requires the Tauri desktop host".into())
}
