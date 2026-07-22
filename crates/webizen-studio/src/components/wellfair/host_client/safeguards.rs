//! Safeguard switches (dead-man, incapacity)

use super::*;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;


/// Generic single-string-arg invoke returning parsed JSON `Value` (for the switch commands).
#[cfg(target_arch = "wasm32")]
async fn invoke_str_arg(cmd: &str, key: &str, val: &str) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &key.into(), &wasm_bindgen::JsValue::from_str(val))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke(cmd, args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

/// Arm a dead-man switch from primitive fields. `disposition` = `"make_public"` | `"release_to"`.
#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
pub async fn arm_dead_mans_switch(
    commitment_hex: &str,
    lapse_after_secs: u64,
    parties: Vec<String>,
    threshold: usize,
    disposition: &str,
    disposition_parties: Vec<String>,
) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"commitmentHex".into(), &wasm_bindgen::JsValue::from_str(commitment_hex))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"lapseAfterSecs".into(), &wasm_bindgen::JsValue::from_f64(lapse_after_secs as f64))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"parties".into(), &serde_wasm_bindgen::to_value(&parties).map_err(|e| e.to_string())?)
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"threshold".into(), &wasm_bindgen::JsValue::from_f64(threshold as f64))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"disposition".into(), &wasm_bindgen::JsValue::from_str(disposition))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"dispositionParties".into(), &serde_wasm_bindgen::to_value(&disposition_parties).map_err(|e| e.to_string())?)
        .map_err(|_| "args".to_string())?;
    tauri_invoke("wellfair_arm_dead_mans_switch", args.into()).await.map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn arm_dead_mans_switch(
    _commitment_hex: &str,
    _lapse_after_secs: u64,
    _parties: Vec<String>,
    _threshold: usize,
    _disposition: &str,
    _disposition_parties: Vec<String>,
) -> Result<(), String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// "I'm alive" â€” touch a dead-man switch's heartbeat.
#[cfg(target_arch = "wasm32")]
pub async fn dead_mans_alive(commitment_hex: &str) -> Result<bool, String> {
    let v = invoke_str_arg("wellfair_dead_mans_alive", "commitmentHex", commitment_hex).await?;
    Ok(v.get("found").and_then(|x| x.as_bool()).unwrap_or(false))
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn dead_mans_alive(_commitment_hex: &str) -> Result<bool, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Record a party attestation toward a dead-man switch. `kind` = `no_contact` | `believed_dead` | `abandon`.
#[cfg(target_arch = "wasm32")]
pub async fn attest_dead_mans(commitment_hex: &str, party_did: &str, kind: &str) -> Result<bool, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"commitmentHex".into(), &wasm_bindgen::JsValue::from_str(commitment_hex))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"partyDid".into(), &wasm_bindgen::JsValue::from_str(party_did))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"kind".into(), &wasm_bindgen::JsValue::from_str(kind))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_attest_dead_mans", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("found").and_then(|x| x.as_bool()).unwrap_or(false))
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn attest_dead_mans(_commitment_hex: &str, _party_did: &str, _kind: &str) -> Result<bool, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Enact a dead-man switch â€” returns the disposition JSON (or null).
#[cfg(target_arch = "wasm32")]
pub async fn enact_dead_mans(commitment_hex: &str) -> Result<serde_json::Value, String> {
    invoke_str_arg("wellfair_enact_dead_mans", "commitmentHex", commitment_hex).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn enact_dead_mans(_commitment_hex: &str) -> Result<serde_json::Value, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// List armed dead-man switches.
#[cfg(target_arch = "wasm32")]
pub async fn list_dead_mans_switches() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_list_dead_mans_switches", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "list not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn list_dead_mans_switches() -> Result<serde_json::Value, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Enact a dead-man switch AND release the keys to the disposition parties. `party_keys` = `(did, pubkey_hex)`.
#[cfg(target_arch = "wasm32")]
pub async fn enact_dead_mans_release(commitment_hex: &str, party_keys: Vec<(String, String)>) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"commitmentHex".into(), &wasm_bindgen::JsValue::from_str(commitment_hex)).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"partyKeys".into(), &serde_wasm_bindgen::to_value(&party_keys).map_err(|e| e.to_string())?).map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_enact_dead_mans_release", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn enact_dead_mans_release(_commitment_hex: &str, _party_keys: Vec<(String, String)>) -> Result<serde_json::Value, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Split a payload's DEK into Shamir social-recovery shares. Returns `{ threshold, shares: [{party, share}] }`.
#[cfg(target_arch = "wasm32")]
pub async fn split_dek_recovery(commitment_hex: &str, threshold: usize, parties: Vec<String>) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"commitmentHex".into(), &wasm_bindgen::JsValue::from_str(commitment_hex)).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"threshold".into(), &wasm_bindgen::JsValue::from_f64(threshold as f64)).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"parties".into(), &serde_wasm_bindgen::to_value(&parties).map_err(|e| e.to_string())?).map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_split_dek_recovery", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn split_dek_recovery(_c: &str, _t: usize, _p: Vec<String>) -> Result<serde_json::Value, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Social-recovery enactment: reconstruct from friends' `shares` (a JSON array of Shamir shares) and release.
#[cfg(target_arch = "wasm32")]
pub async fn reconstruct_and_release(commitment_hex: &str, shares: serde_json::Value, party_keys: Vec<(String, String)>) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"commitmentHex".into(), &wasm_bindgen::JsValue::from_str(commitment_hex)).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"shares".into(), &serde_wasm_bindgen::to_value(&shares).map_err(|e| e.to_string())?).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"partyKeys".into(), &serde_wasm_bindgen::to_value(&party_keys).map_err(|e| e.to_string())?).map_err(|_| "args".to_string())?;
    let js = tauri_invoke("wellfair_reconstruct_and_release", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn reconstruct_and_release(_c: &str, _s: serde_json::Value, _p: Vec<(String, String)>) -> Result<serde_json::Value, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Publish a peer's envelope (X25519) public key into their peer record (remote-key distribution).
#[cfg(target_arch = "wasm32")]
pub async fn set_peer_envelope_key(did: &str, pubkey_hex: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"did".into(), &wasm_bindgen::JsValue::from_str(did)).map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"pubkeyHex".into(), &wasm_bindgen::JsValue::from_str(pubkey_hex)).map_err(|_| "args".to_string())?;
    tauri_invoke("wellfair_set_peer_envelope_key", args.into()).await.map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn set_peer_envelope_key(_did: &str, _pubkey_hex: &str) -> Result<(), String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Enact + release resolving the disposition parties' keys from the peer store. `{ result, missing_keys_for }`.
#[cfg(target_arch = "wasm32")]
pub async fn enact_dead_mans_release_via_peers(commitment_hex: &str) -> Result<serde_json::Value, String> {
    invoke_str_arg("wellfair_enact_dead_mans_release_via_peers", "commitmentHex", commitment_hex).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn enact_dead_mans_release_via_peers(_commitment_hex: &str) -> Result<serde_json::Value, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Arm an incapacity switch from primitive fields. `kind` = `involuntary_psychiatric` | `serious_injury` | ...
#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
pub async fn arm_incapacity_switch(
    principal_did: &str,
    kind: &str,
    advocate_did: &str,
    parties: Vec<String>,
    threshold: usize,
    require_official_instrument: bool,
) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"principalDid".into(), &wasm_bindgen::JsValue::from_str(principal_did))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"kind".into(), &wasm_bindgen::JsValue::from_str(kind))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"advocateDid".into(), &wasm_bindgen::JsValue::from_str(advocate_did))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"parties".into(), &serde_wasm_bindgen::to_value(&parties).map_err(|e| e.to_string())?)
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"threshold".into(), &wasm_bindgen::JsValue::from_f64(threshold as f64))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"requireOfficialInstrument".into(), &wasm_bindgen::JsValue::from_bool(require_official_instrument))
        .map_err(|_| "args".to_string())?;
    tauri_invoke("wellfair_arm_incapacity_switch", args.into()).await.map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn arm_incapacity_switch(
    _principal_did: &str,
    _kind: &str,
    _advocate_did: &str,
    _parties: Vec<String>,
    _threshold: usize,
    _require_official_instrument: bool,
) -> Result<(), String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Activate advocacy on a validated incapacity trigger.
#[cfg(target_arch = "wasm32")]
pub async fn activate_incapacity(
    principal_did: &str,
    attesting_parties: Vec<String>,
    official_instrument: Option<String>,
) -> Result<bool, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"principalDid".into(), &wasm_bindgen::JsValue::from_str(principal_did))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let parties = serde_wasm_bindgen::to_value(&attesting_parties).map_err(|e| e.to_string())?;
    js_sys::Reflect::set(&args, &"attestingParties".into(), &parties)
        .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(instr) = official_instrument {
        js_sys::Reflect::set(&args, &"officialInstrument".into(), &wasm_bindgen::JsValue::from_str(&instr))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_activate_incapacity", args.into()).await.map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "response not JSON".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(v.get("activated").and_then(|x| x.as_bool()).unwrap_or(false))
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn activate_incapacity(
    _principal_did: &str,
    _attesting_parties: Vec<String>,
    _official_instrument: Option<String>,
) -> Result<bool, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// Regain capacity â€” the advocate stands down.
#[cfg(target_arch = "wasm32")]
pub async fn regain_capacity(principal_did: &str) -> Result<bool, String> {
    let v = invoke_str_arg("wellfair_regain_capacity", "principalDid", principal_did).await?;
    Ok(v.get("found").and_then(|x| x.as_bool()).unwrap_or(false))
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn regain_capacity(_principal_did: &str) -> Result<bool, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

/// List armed incapacity switches.
#[cfg(target_arch = "wasm32")]
pub async fn list_incapacity_switches() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("wellfair_list_incapacity_switches", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "list not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn list_incapacity_switches() -> Result<serde_json::Value, String> {
    Err("Safeguard switches require the Tauri desktop host".into())
}

