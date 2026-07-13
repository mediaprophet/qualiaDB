//! **Persistent hypermedia asset library** — ingest documents/assets, and find them by *meaning* (topic /
//! depiction / time / place / project / purpose), never by folder.
//!
//! Each ingested asset is stored as a [`LibraryEntry`] carrying the **container's NQuin edge-graph** (from
//! `qualia_core_db::hypermedia`) — the canonical semantic form. Search is a **real graph query** over the
//! union of all entries' quins (`by_topic`, `by_place`, `in_project`, `for_purpose`, `in_time_range`, …),
//! mapping matched subjects back to entries. So "find my files about biology / at Sydney / for the tax claim
//! / on this day" is one identity space of edges, not a directory walk. (A later step folds these quins into
//! the core graph store / daemon `/query`; this desktop store makes it usable end-to-end now.)

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use qualia_core_db::hypermedia;
use qualia_core_db::NQuin;
use serde::{Deserialize, Serialize};

pub const LIBRARY_FILE: &str = "wellfair/hypermedia_library.json";

/// A summarised flag on an ingested asset (for display / the guardian path).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryFlag {
    pub kind: String,
    pub severity_level: u64,
    pub detail: String,
}

/// One ingested asset in the person's library — its identity + the container's semantic edge-graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryEntry {
    pub asset_uri: String,
    /// The container primary's subject (`= fnv60(asset_uri)`) — the join key for search results.
    pub primary_subject: u64,
    pub media_type: String,
    /// The container's edge-graph (container + descriptor + flag quins) — the searchable semantic form.
    pub quins: Vec<NQuin>,
    /// Display facets (the string forms of the descriptors; search itself runs over the quins above).
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default)]
    pub place: Option<String>,
    /// Event instant (unix seconds) if the asset carries one — the timeline anchor
    /// (e.g. a photo's EXIF capture time). `None` = no dated event.
    #[serde(default)]
    pub occurred_at: Option<i64>,
    /// Geographic coordinates if the asset carries them (e.g. a photo's GPS) — the
    /// map pin. Both present together or both `None`.
    #[serde(default)]
    pub lat: Option<f32>,
    #[serde(default)]
    pub lon: Option<f32>,
    #[serde(default)]
    pub flags: Vec<LibraryFlag>,
    pub ingested_unix: u64,
    /// A short excerpt for display in results (never the whole asset).
    #[serde(default)]
    pub excerpt: String,
}

/// On-disk library store (whole-file JSON, write-temp-then-rename), matching the sibling-store convention.
pub struct HypermediaStore {
    path: PathBuf,
}

impl HypermediaStore {
    pub fn open(storage_root: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = storage_root.as_ref().join(LIBRARY_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self { path })
    }

    pub fn load(&self) -> std::io::Result<Vec<LibraryEntry>> {
        match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| std::io::Error::other(e.to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    fn save(&self, entries: &[LibraryEntry]) -> std::io::Result<()> {
        let bytes = serde_json::to_vec_pretty(entries).map_err(|e| std::io::Error::other(e.to_string()))?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, &bytes)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Add (or replace by `asset_uri`) an entry.
    pub fn add(&self, entry: LibraryEntry) -> std::io::Result<()> {
        let mut entries = self.load()?;
        entries.retain(|e| e.asset_uri != entry.asset_uri);
        entries.push(entry);
        self.save(&entries)
    }

    /// Everything in the library (newest first).
    pub fn all(&self) -> std::io::Result<Vec<LibraryEntry>> {
        let mut entries = self.load()?;
        entries.sort_by(|a, b| b.ingested_unix.cmp(&a.ingested_unix));
        Ok(entries)
    }

    /// Return the entries whose primary subject is in `subjects` (the join back from a graph query).
    fn entries_for(&self, subjects: &HashSet<u64>) -> std::io::Result<Vec<LibraryEntry>> {
        Ok(self
            .load()?
            .into_iter()
            .filter(|e| subjects.contains(&e.primary_subject))
            .collect())
    }

    /// Run a graph query over the union of all entries' quins, returning matching entries. `facet` is one of
    /// `topic` | `depicts` | `place` | `project` | `purpose`.
    pub fn search(&self, facet: &str, value: &str) -> std::io::Result<Vec<LibraryEntry>> {
        let entries = self.load()?;
        let all: Vec<NQuin> = entries.iter().flat_map(|e| e.quins.iter().cloned()).collect();
        let subjects: HashSet<u64> = match facet {
            "topic" => hypermedia::by_topic(&all, value),
            "depicts" => hypermedia::by_depiction(&all, value),
            "place" => hypermedia::by_place(&all, value),
            "project" => hypermedia::in_project(&all, value),
            "purpose" => hypermedia::for_purpose(&all, value),
            "target" => hypermedia::analytics_for(&all, hypermedia::fnv60(value.as_bytes())),
            _ => Vec::new(),
        }
        .into_iter()
        .collect();
        self.entries_for(&subjects)
    }

    /// The **timeline** query — entries whose event instant is within `[start, end]` (unix seconds).
    pub fn search_time_range(&self, start: i64, end: i64) -> std::io::Result<Vec<LibraryEntry>> {
        let entries = self.load()?;
        let all: Vec<NQuin> = entries.iter().flat_map(|e| e.quins.iter().cloned()).collect();
        let subjects: HashSet<u64> = hypermedia::in_time_range(&all, start, end).into_iter().collect();
        self.entries_for(&subjects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::hypermedia::{ingest_with, TextProcessor};

    fn ingest(store: &HypermediaStore, uri: &str, text: &str, now: u64) {
        let proc = TextProcessor::default();
        let r = ingest_with(&proc, uri, "text/markdown", 1, text.as_bytes());
        let entry = LibraryEntry {
            asset_uri: uri.to_string(),
            primary_subject: r.container.primary.subject(),
            media_type: "text/markdown".to_string(),
            quins: r.quins,
            topics: Vec::new(),
            projects: Vec::new(),
            place: None,
            occurred_at: None,
            lat: None,
            lon: None,
            flags: Vec::new(),
            ingested_unix: now,
            excerpt: text.chars().take(40).collect(),
        };
        store.add(entry).unwrap();
    }

    #[test]
    fn ingested_documents_are_findable_by_topic_across_the_library() {
        let dir = tempfile::tempdir().unwrap();
        let store = HypermediaStore::open(dir.path()).unwrap();
        ingest(&store, "urn:doc:liver", "The liver is an organ; hepatocytes secrete bile.", 1_000);
        ingest(&store, "urn:doc:contract", "This contract is governed by statute and jurisdiction.", 1_100);
        ingest(&store, "urn:doc:receipt", "Invoice for a tax-deductible expense; keep this receipt.", 1_200);

        // Search the WHOLE library by meaning — biology finds the liver, law finds the contract, finance the receipt.
        let bio = store.search("topic", "biology").unwrap();
        assert_eq!(bio.len(), 1);
        assert_eq!(bio[0].asset_uri, "urn:doc:liver");
        let law = store.search("topic", "law").unwrap();
        assert_eq!(law.len(), 1);
        assert_eq!(law[0].asset_uri, "urn:doc:contract");
        // The tax/expenses use case: finance topic finds the receipt.
        let fin = store.search("topic", "finance").unwrap();
        assert_eq!(fin.len(), 1);
        assert_eq!(fin[0].asset_uri, "urn:doc:receipt");
        // A topic no doc has returns nothing; the whole library lists all three.
        assert!(store.search("topic", "astronomy").unwrap().is_empty());
        assert_eq!(store.all().unwrap().len(), 3);
    }
}
