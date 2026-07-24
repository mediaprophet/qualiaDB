//! Personal directory and agreements

#![allow(non_snake_case)]

use qualia_client_core::api;
use tauri::command;

// ── Personal directory (AD-like): categorised addressbook + agreement slots ─────

/// The unified, categorised personal directory: the addressbook (Parties joined by DID across the
/// directory-actor + chat-contact stores) grouped into categories, each entry carrying a slot for the
/// agreements governing that relationship.
#[command]
pub fn list_directory() -> Result<serde_json::Value, String> {
    api::list_directory()
}

/// The directory categories (built-in + user-created).
#[command]
pub fn list_directory_categories() -> Result<serde_json::Value, String> {
    api::list_directory_categories()
}

/// Create a custom directory category.
#[command]
pub fn create_directory_category(label: String) -> Result<serde_json::Value, String> {
    api::create_directory_category(label)
}

/// Set which categories a directory entry (by DID) belongs to; returns the refreshed directory.
#[command]
pub fn set_directory_entry_categories(
    did: String,
    categories: Vec<String>,
) -> Result<serde_json::Value, String> {
    api::set_directory_entry_categories(did, categories)
}

/// Faceted + concept-aware search over the directory. `query` is meaning-aware; `facets_json` is a JSON
/// object of `{facet_id: [values]}`. Returns ranked entries + drill-down facet counts.
#[command]
pub fn search_directory(query: String, facets_json: String) -> Result<serde_json::Value, String> {
    api::search_directory(query, facets_json)
}

