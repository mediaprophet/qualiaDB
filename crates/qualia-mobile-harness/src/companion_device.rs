//! Persistent companion device identity + Ed25519 challenge signing.

use ed25519_dalek::{Signer, SigningKey};
use wellfare_core::companion_pairing::{
    CompanionChallenge, CompanionPairingResponse, COMPANION_PAIRING_CONTEXT,
};

const DEVICE_ID_KEY: &str = "qualia-companion-device-id";
const DEVICE_SEED_KEY: &str = "qualia-companion-device-seed";

#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
    export function persistCompanionValue(key, value) {
        localStorage.setItem(key, value);
    }
    export function loadCompanionValue(key) {
        return localStorage.getItem(key) || "";
    }
"#)]
extern "C" {
    fn persistCompanionValue(key: &str, value: &str);
    fn loadCompanionValue(key: &str) -> String;
}

pub fn device_id() -> String {
    let stored = loadCompanionValue(DEVICE_ID_KEY);
    if !stored.is_empty() {
        return stored;
    }
    let id = format!("phone-{}", js_sys::Math::random());
    persistCompanionValue(DEVICE_ID_KEY, &id);
    id
}

fn signing_key() -> SigningKey {
    let stored = loadCompanionValue(DEVICE_SEED_KEY);
    if stored.len() == 64 {
        if let Ok(bytes) = hex::decode(&stored) {
            if bytes.len() == 32 {
                let mut seed = [0u8; 32];
                seed.copy_from_slice(&bytes);
                return SigningKey::from_bytes(&seed);
            }
        }
    }

    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed).expect("OS random for companion device seed");
    persistCompanionValue(DEVICE_SEED_KEY, &hex::encode(seed));
    SigningKey::from_bytes(&seed)
}

pub fn build_pairing_response(challenge_json: &str) -> Result<String, String> {
    let challenge: CompanionChallenge =
        serde_json::from_str(challenge_json).map_err(|e| format!("invalid challenge: {e}"))?;
    if challenge.context != COMPANION_PAIRING_CONTEXT {
        return Err("unexpected pairing context".into());
    }
    let nonce = hex::decode(&challenge.nonce_hex).map_err(|e| format!("bad nonce: {e}"))?;
    if nonce.len() != 32 {
        return Err("challenge nonce must be 32 bytes".into());
    }

    let key = signing_key();
    let mut payload = Vec::with_capacity(challenge.context.len() + 1 + nonce.len());
    payload.extend_from_slice(challenge.context.as_bytes());
    payload.push(0);
    payload.extend_from_slice(&nonce);
    let signature = key.sign(&payload);

    let response = CompanionPairingResponse::new(
        device_id(),
        hex::encode(key.verifying_key().to_bytes()),
        hex::encode(signature.to_bytes()),
    );
    serde_json::to_string(&response).map_err(|e| e.to_string())
}