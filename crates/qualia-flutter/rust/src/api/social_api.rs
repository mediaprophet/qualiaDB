//! Flutter FRB — profile, connect invites, and chat contacts.

use flutter_rust_bridge::frb;
use qualia_client_core::api as core;

#[frb]
#[derive(Debug, Clone)]
pub struct SharingPolicy {
    pub share_display_name: bool,
    pub share_public_did: bool,
    pub share_active_model: bool,
    pub share_llm_outcomes: bool,
    pub share_ontology_scope: bool,
    pub share_installed_qapps: bool,
    pub share_daemon_status: bool,
    pub allow_group_chat_invites: bool,
    pub allow_directory_lookup: bool,
    pub allow_email_invites: bool,
}

#[frb]
#[derive(Debug, Clone)]
pub struct UserProfile {
    pub display_name: String,
    pub bio: Option<String>,
    pub public_did: String,
    pub active_front_door_id: Option<String>,
    pub relay_base_url: Option<String>,
    pub sharing: SharingPolicy,
    pub updated_at: u64,
}

#[frb]
#[derive(Debug, Clone)]
pub struct ConnectInviteSummary {
    pub code: String,
    pub invite_json: String,
    pub mailto_url: String,
    pub inviter_did: String,
    pub expires_at: u64,
}

#[frb]
#[derive(Debug, Clone)]
pub struct ChatContact {
    pub actor_id: String,
    pub display_name: String,
    pub did: String,
    pub source: String,
    pub added_at: u64,
    pub categories: Vec<String>,
}

fn parse_sharing(v: &serde_json::Value) -> SharingPolicy {
    SharingPolicy {
        share_display_name: v["share_display_name"].as_bool().unwrap_or(true),
        share_public_did: v["share_public_did"].as_bool().unwrap_or(true),
        share_active_model: v["share_active_model"].as_bool().unwrap_or(false),
        share_llm_outcomes: v["share_llm_outcomes"].as_bool().unwrap_or(false),
        share_ontology_scope: v["share_ontology_scope"].as_bool().unwrap_or(false),
        share_installed_qapps: v["share_installed_qapps"].as_bool().unwrap_or(false),
        share_daemon_status: v["share_daemon_status"].as_bool().unwrap_or(false),
        allow_group_chat_invites: v["allow_group_chat_invites"].as_bool().unwrap_or(true),
        allow_directory_lookup: v["allow_directory_lookup"].as_bool().unwrap_or(true),
        allow_email_invites: v["allow_email_invites"].as_bool().unwrap_or(true),
    }
}

fn parse_profile(v: serde_json::Value) -> UserProfile {
    UserProfile {
        display_name: v["display_name"].as_str().unwrap_or("Qualia User").to_string(),
        bio: v["bio"].as_str().map(|s| s.to_string()),
        public_did: v["public_did"].as_str().unwrap_or_default().to_string(),
        active_front_door_id: v["active_front_door_id"].as_str().map(|s| s.to_string()),
        relay_base_url: v["relay_base_url"].as_str().map(|s| s.to_string()),
        sharing: parse_sharing(&v["sharing"]),
        updated_at: v["updated_at"].as_u64().unwrap_or(0),
    }
}

#[frb]
pub fn get_user_profile() -> Result<UserProfile, String> {
    let json = core::get_user_profile()?;
    Ok(parse_profile(json))
}

#[frb]
pub fn save_user_profile(profile: UserProfile) -> Result<UserProfile, String> {
    let payload = serde_json::json!({
        "display_name": profile.display_name,
        "bio": profile.bio,
        "public_did": profile.public_did,
        "active_front_door_id": profile.active_front_door_id,
        "relay_base_url": profile.relay_base_url,
        "sharing": {
            "share_display_name": profile.sharing.share_display_name,
            "share_public_did": profile.sharing.share_public_did,
            "share_active_model": profile.sharing.share_active_model,
            "share_llm_outcomes": profile.sharing.share_llm_outcomes,
            "share_ontology_scope": profile.sharing.share_ontology_scope,
            "share_installed_qapps": profile.sharing.share_installed_qapps,
            "share_daemon_status": profile.sharing.share_daemon_status,
            "allow_group_chat_invites": profile.sharing.allow_group_chat_invites,
            "allow_directory_lookup": profile.sharing.allow_directory_lookup,
            "allow_email_invites": profile.sharing.allow_email_invites,
        },
        "updated_at": profile.updated_at,
    });
    let saved = core::save_user_profile(payload.to_string())?;
    Ok(parse_profile(saved))
}

#[frb]
pub fn generate_connect_invite(front_door_id: Option<String>) -> Result<ConnectInviteSummary, String> {
    let json = core::generate_connect_invite(front_door_id)?;
    Ok(ConnectInviteSummary {
        code: json["code"].as_str().unwrap_or_default().to_string(),
        invite_json: json["invite_json"].as_str().unwrap_or_default().to_string(),
        mailto_url: json["mailto_url"].as_str().unwrap_or_default().to_string(),
        inviter_did: json["inviter_did"].as_str().unwrap_or_default().to_string(),
        expires_at: json["expires_at"].as_u64().unwrap_or(0),
    })
}

#[frb]
pub fn accept_connect_invite(input: String) -> Result<ChatContact, String> {
    let json = core::accept_connect_invite(input)?;
    Ok(ChatContact {
        actor_id: json["actor_id"].as_str().unwrap_or_default().to_string(),
        display_name: json["display_name"].as_str().unwrap_or_default().to_string(),
        did: json["did"].as_str().unwrap_or_default().to_string(),
        source: json["source"].as_str().unwrap_or_default().to_string(),
        added_at: json["added_at"].as_u64().unwrap_or(0),
        categories: json["categories"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

#[frb]
pub fn update_chat_contact_categories(
    contact_did: String,
    categories: Vec<String>,
) -> Result<ChatContact, String> {
    let json = core::update_chat_contact_categories(contact_did, categories)?;
    Ok(ChatContact {
        actor_id: json["actor_id"].as_str().unwrap_or_default().to_string(),
        display_name: json["display_name"].as_str().unwrap_or_default().to_string(),
        did: json["did"].as_str().unwrap_or_default().to_string(),
        source: json["source"].as_str().unwrap_or_default().to_string(),
        added_at: json["added_at"].as_u64().unwrap_or(0),
        categories: json["categories"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

#[frb]
pub fn list_chat_contacts() -> Result<Vec<ChatContact>, String> {
    let json = core::list_chat_contacts()?;
    let arr = json.as_array().cloned().unwrap_or_default();
    Ok(arr
        .into_iter()
        .map(|c| ChatContact {
            actor_id: c["actor_id"].as_str().unwrap_or_default().to_string(),
            display_name: c["display_name"].as_str().unwrap_or_default().to_string(),
            did: c["did"].as_str().unwrap_or_default().to_string(),
            source: c["source"].as_str().unwrap_or_default().to_string(),
            added_at: c["added_at"].as_u64().unwrap_or(0),
            categories: c["categories"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect())
}
