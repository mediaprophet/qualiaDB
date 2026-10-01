//! Persistent local notes and saved pages for the OS-shell Saved Items volume.
//!
//! This is a cold authoring store. It is intentionally separate from the graph
//! evaluator and Sanctuary records: ordinary notes must not silently enter a
//! protected or shared graph merely because a person saved them locally.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MAX_SAVED_ITEMS: usize = 1_024;
const MAX_ITEM_NAME_CHARS: usize = 160;
const MAX_ITEM_BODY_CHARS: usize = 256 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SavedItem {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct NewSavedItem {
    pub kind: String,
    pub name: String,
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct SavedItemChange {
    pub name: String,
    pub body: String,
}

/// Caller serializes access through a small mutex. Reads and writes are scoped
/// to the configured local storage directory and never traverse user paths.
pub struct SavedItemsStore {
    path: PathBuf,
}

impl SavedItemsStore {
    pub fn new(storage_root: impl AsRef<Path>) -> Self {
        Self {
            path: storage_root.as_ref().join("saved-items.json"),
        }
    }

    pub fn list(&self) -> Result<Vec<SavedItem>, String> {
        let mut items = self.read_items()?;
        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(items)
    }

    pub fn create(&self, input: NewSavedItem) -> Result<SavedItem, String> {
        validate_new_item(&input)?;
        let mut items = self.read_items()?;
        if items.len() >= MAX_SAVED_ITEMS {
            return Err(format!("Saved Items is limited to {MAX_SAVED_ITEMS} items."));
        }
        let now = Utc::now().to_rfc3339();
        let item = SavedItem {
            id: uuid::Uuid::new_v4().to_string(),
            kind: input.kind.trim().to_string(),
            name: input.name.trim().to_string(),
            body: input.body,
            created_at: now.clone(),
            updated_at: now,
        };
        items.push(item.clone());
        self.write_items(&items)?;
        Ok(item)
    }

    pub fn update(&self, id: &str, change: SavedItemChange) -> Result<SavedItem, String> {
        validate_change(&change)?;
        let mut items = self.read_items()?;
        let item = items
            .iter_mut()
            .find(|item| item.id == id)
            .ok_or_else(|| "Saved item was not found.".to_string())?;
        item.name = change.name.trim().to_string();
        item.body = change.body;
        item.updated_at = Utc::now().to_rfc3339();
        let updated = item.clone();
        self.write_items(&items)?;
        Ok(updated)
    }

    fn read_items(&self) -> Result<Vec<SavedItem>, String> {
        match std::fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| format!("Could not read Saved Items: {error}")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(format!("Could not read Saved Items: {error}")),
        }
    }

    fn write_items(&self, items: &[SavedItem]) -> Result<(), String> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| "Saved Items path has no parent directory.".to_string())?;
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Could not prepare Saved Items storage: {error}"))?;
        let encoded = serde_json::to_vec(items)
            .map_err(|error| format!("Could not encode Saved Items: {error}"))?;
        let temporary = self.path.with_extension("json.tmp");
        std::fs::write(&temporary, encoded)
            .map_err(|error| format!("Could not write Saved Items: {error}"))?;
        std::fs::rename(&temporary, &self.path)
            .map_err(|error| format!("Could not commit Saved Items: {error}"))
    }
}

fn validate_new_item(input: &NewSavedItem) -> Result<(), String> {
    if !matches!(input.kind.trim(), "note" | "page") {
        return Err("Saved item kind must be note or page.".to_string());
    }
    validate_text(&input.name, &input.body)
}

fn validate_change(change: &SavedItemChange) -> Result<(), String> {
    validate_text(&change.name, &change.body)
}

fn validate_text(name: &str, body: &str) -> Result<(), String> {
    if name.trim().is_empty() || name.chars().count() > MAX_ITEM_NAME_CHARS {
        return Err(format!(
            "Saved item name must contain 1 to {MAX_ITEM_NAME_CHARS} characters."
        ));
    }
    if body.chars().count() > MAX_ITEM_BODY_CHARS {
        return Err(format!(
            "Saved item content is limited to {MAX_ITEM_BODY_CHARS} characters."
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_notes_and_pages_are_valid_saved_item_kinds() {
        let note = NewSavedItem {
            kind: "note".to_string(),
            name: "A note".to_string(),
            body: String::new(),
        };
        assert!(validate_new_item(&note).is_ok());
        let other = NewSavedItem {
            kind: "graph".to_string(),
            name: "Not an ordinary saved item".to_string(),
            body: String::new(),
        };
        assert!(validate_new_item(&other).is_err());
    }

    #[test]
    fn saved_item_names_and_bodies_are_bounded() {
        assert!(validate_text(" ", "body").is_err());
        assert!(validate_text(&"n".repeat(MAX_ITEM_NAME_CHARS + 1), "body").is_err());
        assert!(validate_text("note", &"x".repeat(MAX_ITEM_BODY_CHARS + 1)).is_err());
    }
}
