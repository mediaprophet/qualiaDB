//! Connect codes, signed invites, and friend contacts for group chat.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::state::Actor;
use crate::user_profile::{load_profile, public_profile_card};

const INVITE_TTL_SECS: u64 = 7 * 24 * 3600;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectInvitePayload {
    pub version: u8,
    pub code: String,
    pub inviter_name: String,
    pub inviter_did: String,
    pub inviter_pubkey_hex: String,
    #[serde(default)]
    pub relay_endpoint: String,
    pub front_door_did: String,
    pub profile_card: serde_json::Value,
    pub created_at: u64,
    pub expires_at: u64,
    pub signature_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectInviteSummary {
    pub code: String,
    pub invite_json: String,
    pub mailto_url: String,
    pub inviter_did: String,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContact {
    pub actor_id: String,
    pub display_name: String,
    pub did: String,
    pub source: String,
    pub added_at: u64,
    #[serde(default)]
    pub relay_endpoint: Option<String>,
    /// Optional tags for ontology / torrent sharing filters (e.g. `health`, `research`).
    #[serde(default)]
    pub categories: Vec<String>,
}

fn contacts_path() -> PathBuf {
    crate::state::app_meta_dir().join("chat_contacts.json")
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn format_connect_code(raw: u64) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut n = raw;
    let mut chars = [0u8; 8];
    for c in &mut chars {
        *c = ALPHABET[(n % 32) as usize];
        n /= 32;
    }
    format!(
        "QUALIA-{}-{}",
        std::str::from_utf8(&chars[0..4]).unwrap_or("XXXX"),
        std::str::from_utf8(&chars[4..8]).unwrap_or("XXXX")
    )
}

fn pct_encode(input: &str) -> String {
    let mut out = String::new();
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn resolve_front_door_did(profile: &crate::user_profile::UserProfile) -> String {
    let state = match crate::state::APP_STATE.get() {
        Some(s) => s,
        None => return profile.public_did.clone(),
    };
    let doors = state.front_doors.lock().unwrap();
    if let Some(ref fd_id) = profile.active_front_door_id {
        if let Some(door) = doors.iter().find(|d| d.id == *fd_id) {
            return door.did_uri.clone();
        }
    }
    doors
        .first()
        .map(|d| d.did_uri.clone())
        .unwrap_or_else(|| profile.public_did.clone())
}

pub fn generate_connect_invite(front_door_id: Option<String>) -> Result<ConnectInviteSummary, String> {
    let profile = load_profile();
    if !profile.sharing.allow_group_chat_invites {
        return Err("Group chat invites are disabled in your profile sharing settings.".to_string());
    }

    let state = crate::state::APP_STATE.get().ok_or("APP_STATE not initialized")?;

    let mut profile = profile;
    if let Some(id) = front_door_id {
        profile.active_front_door_id = Some(id);
    }
    profile.public_did = crate::user_profile::resolve_public_did(&profile);
    let front_door_did = resolve_front_door_did(&profile);

    let created = unix_now();
    let expires = created + INVITE_TTL_SECS;
    let code = format_connect_code(created ^ profile.public_did.len() as u64);

    let vault = state.key_vault.lock().unwrap();
    let signing_key = vault.derive_key("connect-invite");
    let inviter_pubkey_hex = hex::encode(ed25519_dalek::VerifyingKey::from(&signing_key).as_bytes());

    let relay_endpoint = profile
        .relay_base_url
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(crate::chat_relay::local_relay_base_url);

    let payload_unsigned = serde_json::json!({
        "version": 1,
        "code": code,
        "inviter_name": profile.display_name,
        "inviter_did": profile.public_did,
        "inviter_pubkey_hex": inviter_pubkey_hex,
        "relay_endpoint": relay_endpoint,
        "front_door_did": front_door_did,
        "profile_card": public_profile_card(&profile),
        "created_at": created,
        "expires_at": expires,
    });

    let payload_str = serde_json::to_string(&payload_unsigned).map_err(|e| e.to_string())?;
    let sig = vault.sign_payload(&signing_key, payload_str.as_bytes());
    let signature_hex = hex::encode(sig.to_bytes());
    drop(vault);

    let profile_card = public_profile_card(&profile);
    let invite = ConnectInvitePayload {
        version: 1,
        code: code.clone(),
        inviter_name: profile.display_name.clone(),
        inviter_did: profile.public_did.clone(),
        inviter_pubkey_hex,
        relay_endpoint: relay_endpoint.clone(),
        front_door_did,
        profile_card,
        created_at: created,
        expires_at: expires,
        signature_hex,
    };

    let invite_json = serde_json::to_string(&invite).map_err(|e| e.to_string())?;
    let mailto = if profile.sharing.allow_email_invites {
        let subject = pct_encode("Join my Qualia chat");
        let body = pct_encode(&format!(
            "Connect with me on Qualia.\n\nInvite code: {code}\n\nOr paste this invite JSON in Qualia → Profile → Add Friend:\n{invite_json}"
        ));
        format!("mailto:?subject={subject}&body={body}")
    } else {
        String::new()
    };

    Ok(ConnectInviteSummary {
        code,
        invite_json,
        mailto_url: mailto,
        inviter_did: invite.inviter_did,
        expires_at: expires,
    })
}

pub fn accept_connect_invite(input: &str) -> Result<ChatContact, String> {
    let invite: ConnectInvitePayload = if input.trim().starts_with('{') {
        serde_json::from_str(input.trim()).map_err(|e| format!("Invalid invite JSON: {e}"))?
    } else {
        return Err(
            "Paste the full invite JSON from your friend (Profile → Share connect code). Short codes alone are not yet supported for remote lookup.".to_string(),
        );
    };

    if unix_now() > invite.expires_at {
        return Err("This connect invite has expired.".to_string());
    }

    if !invite.inviter_pubkey_hex.is_empty() && !invite.signature_hex.is_empty() {
        let pk_bytes = hex::decode(&invite.inviter_pubkey_hex)
            .map_err(|e| format!("Invalid invite public key: {e}"))?;
        if pk_bytes.len() == 32 {
            let sig_bytes = hex::decode(&invite.signature_hex)
                .map_err(|e| format!("Invalid invite signature: {e}"))?;
            if sig_bytes.len() == 64 {
                let mut pk_arr = [0u8; 32];
                pk_arr.copy_from_slice(&pk_bytes);
                let mut sig_arr = [0u8; 64];
                sig_arr.copy_from_slice(&sig_bytes);
                let payload_unsigned = serde_json::json!({
                    "version": invite.version,
                    "code": invite.code,
                    "inviter_name": invite.inviter_name,
                    "inviter_did": invite.inviter_did,
                    "inviter_pubkey_hex": invite.inviter_pubkey_hex,
                    "relay_endpoint": invite.relay_endpoint,
                    "front_door_did": invite.front_door_did,
                    "profile_card": invite.profile_card,
                    "created_at": invite.created_at,
                    "expires_at": invite.expires_at,
                });
                let payload_str =
                    serde_json::to_string(&payload_unsigned).map_err(|e| e.to_string())?;
                if qualia_core_db::key_vault::KeyVault::verify_signature(
                    &pk_arr,
                    payload_str.as_bytes(),
                    &sig_arr,
                )
                .is_err()
                {
                    return Err("Invite signature verification failed.".to_string());
                }
            }
        }
    }

    let display_name = invite
        .profile_card
        .get("display_name")
        .and_then(|v| v.as_str())
        .unwrap_or(&invite.inviter_name)
        .to_string();

    let short_id = invite
        .inviter_did
        .rsplit(':')
        .next()
        .unwrap_or("peer")
        .chars()
        .take(12)
        .collect::<String>();
    let actor = Actor {
        id: format!("contact-{short_id}"),
        actor_type: "FRIEND".to_string(),
        name: display_name.clone(),
        organization: None,
        qualifications: vec![],
        roles: vec!["chat_participant".to_string()],
        verification_status: "INVITE_ACCEPTED".to_string(),
        pairwise_did: invite.inviter_did.clone(),
        root_did_uri: Some(invite.front_door_did.clone()),
        routing_hints: vec![],
    };

    crate::api::add_directory_actor(actor)?;

    let contact = ChatContact {
        actor_id: format!("contact-{short_id}"),
        display_name,
        did: invite.inviter_did,
        source: format!("connect:{}", invite.code),
        added_at: unix_now(),
        relay_endpoint: if invite.relay_endpoint.is_empty() {
            None
        } else {
            Some(invite.relay_endpoint)
        },
        categories: vec![],
    };

    let mut contacts = load_contacts();
    contacts.retain(|c| c.did != contact.did);
    contacts.push(contact.clone());
    save_contacts(&contacts)?;

    Ok(contact)
}

pub fn load_contacts() -> Vec<ChatContact> {
    let path = contacts_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_contacts(contacts: &[ChatContact]) -> Result<(), String> {
    let path = contacts_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(contacts).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

pub fn find_contact_by_did(did: &str) -> Option<ChatContact> {
    list_chat_contacts().into_iter().find(|c| c.did == did)
}

pub fn update_contact_categories(did: &str, categories: Vec<String>) -> Result<ChatContact, String> {
    let mut contacts = load_contacts();
    let idx = contacts
        .iter()
        .position(|c| c.did == did)
        .ok_or_else(|| format!("Contact not found: {did}"))?;
    contacts[idx].categories = categories
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    save_contacts(&contacts)?;
    Ok(contacts[idx].clone())
}

pub fn list_chat_contacts() -> Vec<ChatContact> {
    let mut contacts = load_contacts();
    if contacts.is_empty() {
        if let Ok(actors) = crate::api::get_directory_actors() {
            contacts = actors
                .into_iter()
                .filter(|a| a.actor_type == "FRIEND" || a.roles.iter().any(|r| r == "chat_participant"))
                .map(|a| ChatContact {
                    actor_id: a.id,
                    display_name: a.name,
                    did: a.pairwise_did,
                    source: "directory".to_string(),
                    added_at: unix_now(),
                    relay_endpoint: None,
                    categories: vec![],
                })
                .collect();
        }
    }
    contacts
}
