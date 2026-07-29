//! Hypermedia bookmarks (QLink index + Library purpose filter).
//!
//! Bookmarks are dual-written:
//! 1. `{storage}/qlinks/{uuid}.json` — always (offline-safe JSON-LD Bookmark)
//! 2. Hypermedia library entry with `purposes: ["bookmark"]` when the vault host can ingest
//!
//! Listing prefers the qlinks directory (complete for browser saves) and merges
//! library entries that already carry purpose `bookmark`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::hypermedia_store::{FacetFilter, HypermediaStore, LibrarySort};

pub const QLINKS_DIR: &str = "qlinks";
pub const PURPOSE_BOOKMARK: &str = "bookmark";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkRecord {
    pub id: String,
    pub url: String,
    pub name: String,
    pub description: String,
    pub date_created: String,
    pub ingested_to_library: bool,
    pub source: String,
    /// Path relative to storage root when from qlinks JSON.
    pub path: Option<String>,
}

fn qlinks_dir(storage_root: &Path) -> PathBuf {
    storage_root.join(QLINKS_DIR)
}

/// List bookmark JSON files under `{storage}/qlinks/`.
pub fn list_qlink_files(storage_root: &Path) -> Result<Vec<BookmarkRecord>, String> {
    let dir = qlinks_dir(storage_root);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let rd = fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for ent in rd.flatten() {
        let path = ent.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let raw = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let url = v
            .get("url")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if url.is_empty() {
            continue;
        }
        let name = v
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or(&url)
            .to_string();
        let description = v
            .get("description")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let date_created = v
            .get("dateCreated")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let ingested = v
            .get("ingestedToLibrary")
            .and_then(|x| x.as_bool())
            .unwrap_or(false);
        let rel = path
            .strip_prefix(storage_root)
            .ok()
            .map(|p| p.to_string_lossy().replace('\\', "/"));
        out.push(BookmarkRecord {
            id,
            url,
            name,
            description,
            date_created,
            ingested_to_library: ingested,
            source: "qlinks".into(),
            path: rel,
        });
    }
    out.sort_by(|a, b| b.date_created.cmp(&a.date_created));
    Ok(out)
}

/// Library entries with purpose `bookmark` (faceted query).
pub fn list_library_bookmarks(storage_root: &Path) -> Result<Vec<BookmarkRecord>, String> {
    let store = HypermediaStore::open(storage_root).map_err(|e| e.to_string())?;
    let filter = FacetFilter {
        purposes: vec![PURPOSE_BOOKMARK.into()],
        ..Default::default()
    };
    let entries = store
        .query_faceted(&filter, LibrarySort::Newest)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for e in entries {
        out.push(BookmarkRecord {
            id: e.asset_uri.clone(),
            url: e.asset_uri.clone(),
            name: if e.excerpt.is_empty() {
                e.asset_uri.clone()
            } else {
                e.excerpt.chars().take(80).collect()
            },
            description: e.excerpt.clone(),
            date_created: e
                .occurred_at
                .map(|t| {
                    chrono::DateTime::from_timestamp(t, 0)
                        .map(|d| d.to_rfc3339())
                        .unwrap_or_default()
                })
                .unwrap_or_else(|| {
                    chrono::DateTime::from_timestamp(e.ingested_unix as i64, 0)
                        .map(|d| d.to_rfc3339())
                        .unwrap_or_default()
                }),
            ingested_to_library: true,
            source: "library".into(),
            path: None,
        });
    }
    Ok(out)
}

/// Merge qlinks files + library purpose=bookmark (dedupe by URL, prefer qlinks metadata).
pub fn list_all_bookmarks(storage_root: &Path) -> Result<Vec<BookmarkRecord>, String> {
    let mut by_url: std::collections::BTreeMap<String, BookmarkRecord> =
        std::collections::BTreeMap::new();
    for b in list_library_bookmarks(storage_root)? {
        by_url.insert(b.url.clone(), b);
    }
    for b in list_qlink_files(storage_root)? {
        by_url.insert(b.url.clone(), b);
    }
    let mut out: Vec<_> = by_url.into_values().collect();
    out.sort_by(|a, b| b.date_created.cmp(&a.date_created));
    Ok(out)
}

/// Persist a qlink JSON document (always succeeds if disk allows).
pub fn write_qlink_json(
    storage_root: &Path,
    url: &str,
    name: &str,
    description: &str,
    ingested_to_library: bool,
    context_assertions: Option<Vec<Value>>,
) -> Result<(String, PathBuf), String> {
    let dir = qlinks_dir(storage_root);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let mut doc = serde_json::json!({
        "@context": ["http://schema.org", "http://www.w3.org/ns/anno.jsonld"],
        "@type": "Bookmark",
        "url": url,
        "name": name,
        "description": description,
        "dateCreated": chrono::Utc::now().to_rfc3339(),
        "ingestedToLibrary": ingested_to_library,
        "purpose": PURPOSE_BOOKMARK,
    });
    if let Some(assertions) = context_assertions {
        if let Some(obj) = doc.as_object_mut() {
            obj.insert("cml:contextAssertions".into(), Value::Array(assertions));
        }
    }
    let path = dir.join(format!("{id}.json"));
    let json_str = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    fs::write(&path, json_str).map_err(|e| e.to_string())?;
    Ok((id, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_list_qlink() {
        let dir = tempfile::tempdir().unwrap();
        let (id, path) = write_qlink_json(
            dir.path(),
            "https://example.org/a",
            "Example",
            "desc",
            false,
            None,
        )
        .unwrap();
        assert!(path.exists());
        assert!(!id.is_empty());
        let list = list_qlink_files(dir.path()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].url, "https://example.org/a");
        assert_eq!(list[0].name, "Example");
        assert!(!list[0].ingested_to_library);
    }

    #[test]
    fn list_empty_ok() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_all_bookmarks(dir.path()).unwrap().is_empty());
    }
}
