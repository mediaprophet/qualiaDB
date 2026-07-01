//! JSON wire protocol for companion WebSocket pairing (Ed25519 challenge-response).

use serde::{Deserialize, Serialize};

pub const MSG_CHALLENGE: &str = "CHALLENGE";
pub const MSG_PAIRING_RESPONSE: &str = "PAIRING_RESPONSE";
pub const MSG_AUTH_SUCCESS: &str = "AUTH_SUCCESS";
pub const MSG_AUTH_DENIED: &str = "AUTH_DENIED";
pub const COMPANION_PAIRING_CONTEXT: &str = "wellfair:companion";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompanionChallenge {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub nonce_hex: String,
    pub context: String,
}

impl CompanionChallenge {
    pub fn new(nonce_hex: impl Into<String>) -> Self {
        Self {
            msg_type: MSG_CHALLENGE.into(),
            nonce_hex: nonce_hex.into(),
            context: COMPANION_PAIRING_CONTEXT.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompanionPairingResponse {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub device_id: String,
    pub public_key_hex: String,
    pub signature_hex: String,
}

impl CompanionPairingResponse {
    pub fn new(
        device_id: impl Into<String>,
        public_key_hex: impl Into<String>,
        signature_hex: impl Into<String>,
    ) -> Self {
        Self {
            msg_type: MSG_PAIRING_RESPONSE.into(),
            device_id: device_id.into(),
            public_key_hex: public_key_hex.into(),
            signature_hex: signature_hex.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompanionAuthResult {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl CompanionAuthResult {
    pub fn success() -> Self {
        Self {
            msg_type: MSG_AUTH_SUCCESS.into(),
            reason: None,
        }
    }

    pub fn denied(reason: impl Into<String>) -> Self {
        Self {
            msg_type: MSG_AUTH_DENIED.into(),
            reason: Some(reason.into()),
        }
    }
}