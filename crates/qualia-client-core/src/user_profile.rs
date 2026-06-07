//! Local user profile and sharing policy for chats and connect invites.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingPolicy {
    pub share_display_name: bool,
    pub share_public_did: bool,
    pub share_active_model: bool,
    /// Allow sharing Webizen-processed outcomes (not raw prompts) with group chat peers.
    #[serde(default)]
    pub share_llm_outcomes: bool,
    pub share_ontology_scope: bool,
    pub share_installed_qapps: bool,
    pub share_daemon_status: bool,
    pub allow_group_chat_invites: bool,
    pub allow_directory_lookup: bool,
    pub allow_email_invites: bool,
}

impl Default for SharingPolicy {
    fn default() -> Self {
        Self {
            share_display_name: true,
            share_public_did: true,
            share_active_model: false,
            share_llm_outcomes: false,
            share_ontology_scope: false,
            share_installed_qapps: false,
            share_daemon_status: false,
            allow_group_chat_invites: true,
            allow_directory_lookup: true,
            allow_email_invites: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub display_name: String,
    pub bio: Option<String>,
    pub public_did: String,
    pub active_front_door_id: Option<String>,
    #[serde(default)]
    pub relay_base_url: Option<String>,
    pub sharing: SharingPolicy,
    pub updated_at: u64,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            display_name: "Qualia User".to_string(),
            bio: None,
            public_did: String::new(),
            active_front_door_id: None,
            relay_base_url: None,
            sharing: SharingPolicy::default(),
            updated_at: 0,
        }
    }
}

pub fn profile_path() -> PathBuf {
    crate::state::app_meta_dir().join("profile.json")
}

pub fn load_profile() -> UserProfile {
    let path = profile_path();
    if let Ok(text) = fs::read_to_string(&path) {
        if let Ok(mut p) = serde_json::from_str::<UserProfile>(&text) {
            if p.public_did.is_empty() {
                p.public_did = resolve_public_did(&p);
            }
            return p;
        }
    }
    let mut profile = UserProfile::default();
    profile.public_did = resolve_public_did(&profile);
    profile.updated_at = unix_now();
    let _ = save_profile(&profile);
    profile
}

pub fn save_profile(profile: &UserProfile) -> Result<(), String> {
    let path = profile_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut p = profile.clone();
    p.updated_at = unix_now();
    if p.public_did.is_empty() {
        p.public_did = resolve_public_did(&p);
    }
    let text = serde_json::to_string_pretty(&p).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

pub fn resolve_public_did(profile: &UserProfile) -> String {
    let state = match crate::state::APP_STATE.get() {
        Some(s) => s,
        None => return format!("did:qualia:local:{}", unix_now()),
    };

    if let Some(ref fd_id) = profile.active_front_door_id {
        let doors = state.front_doors.lock().unwrap();
        if let Some(door) = doors.iter().find(|d| d.id == *fd_id) {
            return door.did_uri.clone();
        }
    }

    let doors = state.front_doors.lock().unwrap();
    if let Some(door) = doors.first() {
        return door.did_uri.clone();
    }

    let vault = state.key_vault.lock().unwrap();
    let key = vault.derive_key("profile-root");
    let pub_hex = hex::encode(ed25519_dalek::VerifyingKey::from(&key).as_bytes());
    format!("did:qualia:root:{pub_hex}")
}

pub fn public_profile_card(profile: &UserProfile) -> serde_json::Value {
    let mut card = serde_json::json!({
        "version": 1,
        "updated_at": profile.updated_at,
    });

    if profile.sharing.share_display_name {
        card["display_name"] = serde_json::Value::String(profile.display_name.clone());
    }
    if profile.sharing.share_public_did {
        card["public_did"] = serde_json::Value::String(profile.public_did.clone());
    }
    if let Some(ref bio) = profile.bio {
        if profile.sharing.share_display_name {
            card["bio"] = serde_json::Value::String(bio.clone());
        }
    }

    card
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
