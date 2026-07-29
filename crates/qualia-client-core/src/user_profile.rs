//! Local user profile and sharing policy for chats and connect invites.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Per-flag sharing policy for chats and connect invites.
///
/// `#[serde(default)]` on the struct (and the `Default` impl) means older on-disk
/// `profile.json` files and partial UI patches can omit fields without hard-failing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SharingPolicy {
    pub share_display_name: bool,
    pub share_public_did: bool,
    pub share_active_model: bool,
    /// Allow sharing Webizen-processed outcomes (not raw prompts) with group chat peers.
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

/// Apply a partial profile JSON patch onto `base` (load-merge-save semantics).
///
/// Used by the People tab "Save + enable invites" path, which intentionally sends only
/// `display_name` + a subset of `sharing` flags. Unknown top-level keys are ignored.
/// Nested `sharing` objects are field-wise merged so omitted flags keep their current values.
pub fn apply_profile_patch(
    base: &UserProfile,
    patch: &serde_json::Value,
) -> Result<UserProfile, String> {
    if !patch.is_object() {
        return Err("profile patch must be a JSON object".into());
    }
    let mut out = base.clone();

    if let Some(v) = patch.get("display_name") {
        out.display_name = v
            .as_str()
            .ok_or_else(|| "display_name must be a string".to_string())?
            .to_string();
    }
    if let Some(v) = patch.get("bio") {
        out.bio = match v {
            serde_json::Value::Null => None,
            serde_json::Value::String(s) => Some(s.clone()),
            _ => return Err("bio must be a string or null".into()),
        };
    }
    if let Some(v) = patch.get("public_did") {
        if let Some(s) = v.as_str() {
            if !s.is_empty() {
                out.public_did = s.to_string();
            }
        } else if !v.is_null() {
            return Err("public_did must be a string".into());
        }
    }
    if let Some(v) = patch.get("active_front_door_id") {
        out.active_front_door_id = match v {
            serde_json::Value::Null => None,
            serde_json::Value::String(s) => Some(s.clone()),
            _ => return Err("active_front_door_id must be a string or null".into()),
        };
    }
    if let Some(v) = patch.get("relay_base_url") {
        out.relay_base_url = match v {
            serde_json::Value::Null => None,
            serde_json::Value::String(s) => Some(s.clone()),
            _ => return Err("relay_base_url must be a string or null".into()),
        };
    }
    if let Some(sharing) = patch.get("sharing") {
        apply_sharing_patch(&mut out.sharing, sharing)?;
    }

    Ok(out)
}

fn apply_sharing_patch(
    policy: &mut SharingPolicy,
    patch: &serde_json::Value,
) -> Result<(), String> {
    let obj = patch
        .as_object()
        .ok_or_else(|| "sharing must be a JSON object".to_string())?;
    for (key, value) in obj {
        let Some(flag) = value.as_bool() else {
            return Err(format!("sharing.{key} must be a boolean"));
        };
        match key.as_str() {
            "share_display_name" => policy.share_display_name = flag,
            "share_public_did" => policy.share_public_did = flag,
            "share_active_model" => policy.share_active_model = flag,
            "share_llm_outcomes" => policy.share_llm_outcomes = flag,
            "share_ontology_scope" => policy.share_ontology_scope = flag,
            "share_installed_qapps" => policy.share_installed_qapps = flag,
            "share_daemon_status" => policy.share_daemon_status = flag,
            "allow_group_chat_invites" => policy.allow_group_chat_invites = flag,
            "allow_directory_lookup" => policy.allow_directory_lookup = flag,
            "allow_email_invites" => policy.allow_email_invites = flag,
            // Forward-compatible: ignore unknown sharing keys rather than fail closed.
            _ => {}
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn partial_people_tab_patch_enables_invites_without_wiping_fields() {
        let base = UserProfile {
            display_name: "Old Name".into(),
            bio: Some("keeps bio".into()),
            public_did: "did:qualia:test:abc".into(),
            active_front_door_id: Some("door-1".into()),
            relay_base_url: Some("https://relay.example".into()),
            sharing: SharingPolicy {
                allow_group_chat_invites: false,
                share_active_model: true, // must survive a partial sharing patch
                ..SharingPolicy::default()
            },
            updated_at: 1,
        };
        // Exact shape the People tab "Save + enable invites" button used to send
        // (and that previously failed with missing field `share_display_name`).
        let patch = json!({
            "display_name": "Timothy",
            "sharing": { "allow_group_chat_invites": true }
        });
        let out = apply_profile_patch(&base, &patch).expect("partial patch must apply");
        assert_eq!(out.display_name, "Timothy");
        assert!(out.sharing.allow_group_chat_invites);
        assert!(
            out.sharing.share_active_model,
            "unmentioned sharing flags must be preserved"
        );
        assert_eq!(out.bio.as_deref(), Some("keeps bio"));
        assert_eq!(out.public_did, "did:qualia:test:abc");
        assert_eq!(out.active_front_door_id.as_deref(), Some("door-1"));
        assert_eq!(out.relay_base_url.as_deref(), Some("https://relay.example"));
    }

    #[test]
    fn sharing_policy_deserializes_when_fields_are_omitted() {
        let p: SharingPolicy = serde_json::from_str(r#"{"allow_group_chat_invites":false}"#)
            .expect("#[serde(default)] must fill omitted SharingPolicy fields");
        assert!(!p.allow_group_chat_invites);
        assert!(p.share_display_name);
        assert!(p.share_public_did);
    }
}
