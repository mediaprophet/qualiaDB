//! Profile + social connect, personal directory

#![allow(non_snake_case)]

use super::*;

pub fn get_user_profile() -> Result<serde_json::Value, String> {
    let profile = crate::user_profile::load_profile();
    serde_json::to_value(profile).map_err(|e| e.to_string())
}

pub fn save_user_profile(profile_json: String) -> Result<serde_json::Value, String> {
    // People tab (and other callers) may send a *partial* profile JSON — e.g. only
    // `display_name` + `sharing.allow_group_chat_invites`. Full-struct deserialize used
    // to fail with `missing field share_display_name`. Merge onto the loaded profile.
    let patch: serde_json::Value =
        serde_json::from_str(&profile_json).map_err(|e| format!("invalid profile json: {e}"))?;
    let base = crate::user_profile::load_profile();
    let mut profile = crate::user_profile::apply_profile_patch(&base, &patch)?;
    profile.public_did = crate::user_profile::resolve_public_did(&profile);
    crate::user_profile::save_profile(&profile)?;
    serde_json::to_value(profile).map_err(|e| e.to_string())
}

pub fn generate_connect_invite(front_door_id: Option<String>) -> Result<serde_json::Value, String> {
    let invite = crate::social_connect::generate_connect_invite(front_door_id)?;
    serde_json::to_value(invite).map_err(|e| e.to_string())
}

pub fn accept_connect_invite(input: String) -> Result<serde_json::Value, String> {
    let contact = crate::social_connect::accept_connect_invite(&input)?;
    serde_json::to_value(contact).map_err(|e| e.to_string())
}

pub fn list_chat_contacts() -> Result<serde_json::Value, String> {
    let contacts = crate::social_connect::list_chat_contacts();
    serde_json::to_value(contacts).map_err(|e| e.to_string())
}

// ── Personal directory (AD-like): categorised addressbook + agreement slots ─────

/// The unified, categorised personal directory — the addressbook (Parties joined across the directory-actor
/// + chat-contact stores by DID) grouped into categories, with a per-entry slot for the agreements
/// governing that relationship. See `docs/plans/rights-aware-peer-agreement-addressbook.md`.
pub fn list_directory() -> Result<serde_json::Value, String> {
    let actors = get_directory_actors()?;
    let contacts = crate::social_connect::list_chat_contacts();
    let view = crate::directory::build_view(&actors, &contacts);
    serde_json::to_value(view).map_err(|e| e.to_string())
}

/// The directory categories (built-in + user-created).
pub fn list_directory_categories() -> Result<serde_json::Value, String> {
    serde_json::to_value(crate::directory::list_categories()).map_err(|e| e.to_string())
}

/// Create a custom directory category. Returns the new category.
pub fn create_directory_category(label: String) -> Result<serde_json::Value, String> {
    let cat = crate::directory::create_category(&label)?;
    serde_json::to_value(cat).map_err(|e| e.to_string())
}

/// Set the categories a directory entry (by DID) belongs to; returns the refreshed directory.
pub fn set_directory_entry_categories(
    did: String,
    categories: Vec<String>,
) -> Result<serde_json::Value, String> {
    crate::directory::set_entry_categories(&did, categories)?;
    list_directory()
}

/// Faceted + concept-aware search over the directory. `query` is meaning-aware (a token expands across a
/// concept cluster, so "doctor" finds a "clinician"); `facets_json` is a JSON object of
/// `{facet_id: [selected values]}` (AND across facets, OR within). Returns ranked entries + drill-down
/// facet counts. Both empty → the whole directory with all facet counts.
pub fn search_directory(query: String, facets_json: String) -> Result<serde_json::Value, String> {
    let selected: std::collections::BTreeMap<String, Vec<String>> = if facets_json.trim().is_empty()
    {
        std::collections::BTreeMap::new()
    } else {
        serde_json::from_str(&facets_json).map_err(|e| format!("bad facets json: {e}"))?
    };
    let actors = get_directory_actors()?;
    let contacts = crate::social_connect::list_chat_contacts();
    let result = crate::directory::search(&actors, &contacts, &query, &selected);
    serde_json::to_value(result).map_err(|e| e.to_string())
}
