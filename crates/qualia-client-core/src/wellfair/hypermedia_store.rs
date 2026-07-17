//! **Persistent hypermedia asset library** — ingest documents/assets, and find them by *meaning* (topic /
//! depiction / time / place / project / purpose), never by folder.
//!
//! # Sections (product lanes)
//! The library is one store, many **sections** — purpose-shaped views, not folders:
//! - **Secret** — sanctuary / restricted / classified / Wellfair-private health
//! - **Wellfair** — health & welfare purposes (can also force secret when sensitivity is high)
//! - **Personal** — private life, default home shelf
//! - **Work** — project-scoped labour
//! - **Tools** — logs, telemetry, technical artefacts, agent/tool output
//! - **Commons** — permissive share surface (peers / micro-commons via social networking)
//!
//! Sensitivity (`public` | `restricted` | `classified` | `sanctuary`) is orthogonal: high sensitivity
//! always routes into **Secret** even if the purpose is Wellfair or Work.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use qualia_core_db::hypermedia;
use qualia_core_db::NQuin;
use serde::{Deserialize, Serialize};

pub const LIBRARY_FILE: &str = "wellfair/hypermedia_library.json";

/// Product section id for the Library chrome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibrarySection {
    /// All items (except those filtered by UI for secret gate).
    All,
    /// High-sensitivity / sanctuary / private health — the secret shelf.
    Secret,
    /// Health, welfare, care (Wellfair).
    Wellfair,
    /// Default personal shelf.
    Personal,
    /// Project / cooperative work.
    Work,
    /// Logs, telemetry, agent/tool output, technical diagnostics.
    Tools,
    /// Permissive commons — shareable with peers / social networking layers.
    Commons,
}

impl LibrarySection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Secret => "secret",
            Self::Wellfair => "wellfair",
            Self::Personal => "personal",
            Self::Work => "work",
            Self::Tools => "tools",
            Self::Commons => "commons",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "secret" | "sanctuary" | "private" => Self::Secret,
            "wellfair" | "health" | "welfare" => Self::Wellfair,
            "personal" | "home" => Self::Personal,
            "work" | "project" | "coop" => Self::Work,
            "tools" | "tool" | "logs" | "log" | "tech" | "technical" | "ops" | "debug"
            | "telemetry" | "agent" => Self::Tools,
            "commons" | "public" | "share" | "permissive" => Self::Commons,
            _ => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Secret => "Secret",
            Self::Wellfair => "Wellfair",
            Self::Personal => "Personal",
            Self::Work => "Work",
            Self::Tools => "Tools",
            Self::Commons => "Commons",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Self::All => "Everything on this device you have rights to see.",
            Self::Secret => "Sanctuary & high-sensitivity — Wellfair-private health and other secrets. Not for commons.",
            Self::Wellfair => "Health, care, and welfare records — may also live under Secret when classified.",
            Self::Personal => "Your private shelf — notes, life admin, unshared research.",
            Self::Work => "Project-scoped material for cooperative labour.",
            Self::Tools => "Logs, telemetry, agent/tool output, technical diagnostics — the machine's paper trail.",
            Self::Commons => "Permissive share surface — peers and micro-commons via Talk social networking.",
        }
    }
}

/// How far an item may travel on social / commons layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CommonsVisibility {
    /// Device-local only (default for secret).
    #[default]
    None,
    /// Visible to accepted social peers (bilateral / mesh).
    Peers,
    /// Permissive commons — intended for broader micro-commons replication.
    Commons,
}

impl CommonsVisibility {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "peers" | "peer" | "bilateral" => Self::Peers,
            "commons" | "public" | "permissive" => Self::Commons,
            _ => Self::None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Peers => "peers",
            Self::Commons => "commons",
        }
    }
}

/// Normalize sensitivity tokens used at ingest / UI.
pub fn normalize_sensitivity(s: &str) -> String {
    match s.trim().to_ascii_lowercase().as_str() {
        "restricted" => "restricted".into(),
        "classified" | "sanctuary" => "classified".into(),
        "secret" => "classified".into(),
        _ => "public".into(),
    }
}

/// Resolve the product section for an entry (secret always wins on high sensitivity).
pub fn resolve_section(
    sensitivity: &str,
    purposes: &[String],
    projects: &[String],
    commons: CommonsVisibility,
    section_hint: Option<&str>,
) -> LibrarySection {
    let sens = normalize_sensitivity(sensitivity);
    if sens == "restricted" || sens == "classified" {
        return LibrarySection::Secret;
    }
    if commons == CommonsVisibility::Commons || commons == CommonsVisibility::Peers {
        // Explicit share lane — still never secret.
        if let Some(h) = section_hint {
            let p = LibrarySection::parse(h);
            if p != LibrarySection::Secret {
                return if commons == CommonsVisibility::Commons {
                    LibrarySection::Commons
                } else {
                    p
                };
            }
        }
        return LibrarySection::Commons;
    }
    if let Some(h) = section_hint {
        let p = LibrarySection::parse(h);
        if p != LibrarySection::All {
            return p;
        }
    }
    let purpose_blob = purposes
        .iter()
        .chain(projects.iter())
        .map(|s| s.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    if purpose_blob.contains("health")
        || purpose_blob.contains("wellfair")
        || purpose_blob.contains("welfare")
        || purpose_blob.contains("medical")
        || purpose_blob.contains("care")
    {
        return LibrarySection::Wellfair;
    }
    if purpose_blob.contains("log")
        || purpose_blob.contains("telemetry")
        || purpose_blob.contains("debug")
        || purpose_blob.contains("trace")
        || purpose_blob.contains("tool")
        || purpose_blob.contains("agent")
        || purpose_blob.contains("ops")
        || purpose_blob.contains("technical")
        || purpose_blob.contains("diagnostic")
        || purpose_blob.contains("build")
        || purpose_blob.contains("ci")
    {
        return LibrarySection::Tools;
    }
    if !projects.is_empty()
        || purpose_blob.contains("work")
        || purpose_blob.contains("project")
        || purpose_blob.contains("coop")
    {
        return LibrarySection::Work;
    }
    LibrarySection::Personal
}

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
    /// Purpose tags (tax, health, research, …) — power purpose-shaped sections.
    #[serde(default)]
    pub purposes: Vec<String>,
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
    /// `public` | `restricted` | `classified` — high sensitivity forces Secret section.
    #[serde(default = "default_sensitivity_public")]
    pub sensitivity: String,
    /// Product section lane (secret / wellfair / personal / work / commons).
    #[serde(default = "default_section_personal")]
    pub section: String,
    /// How far this may travel on social / commons layers.
    #[serde(default)]
    pub commons_visibility: CommonsVisibility,
}

fn default_sensitivity_public() -> String {
    "public".into()
}
fn default_section_personal() -> String {
    LibrarySection::Personal.as_str().into()
}

impl LibraryEntry {
    /// Recompute section from sensitivity / purposes / commons (call after mutate).
    pub fn recompute_section(&mut self) {
        self.section = resolve_section(
            &self.sensitivity,
            &self.purposes,
            &self.projects,
            self.commons_visibility,
            Some(&self.section),
        )
        .as_str()
        .into();
        // Secret can never be commons-visible.
        if self.section == LibrarySection::Secret.as_str() {
            self.commons_visibility = CommonsVisibility::None;
        }
    }

    pub fn is_secret(&self) -> bool {
        self.section == LibrarySection::Secret.as_str()
            || matches!(
                normalize_sensitivity(&self.sensitivity).as_str(),
                "restricted" | "classified"
            )
    }
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
        // Backfill section/sensitivity on old entries.
        for e in &mut entries {
            if e.section.is_empty() || e.sensitivity.is_empty() {
                e.sensitivity = normalize_sensitivity(&e.sensitivity);
                e.recompute_section();
            }
        }
        entries.sort_by(|a, b| b.ingested_unix.cmp(&a.ingested_unix));
        Ok(entries)
    }

    /// Entries in one product section (`all` = everything).
    pub fn by_section(&self, section: LibrarySection) -> std::io::Result<Vec<LibraryEntry>> {
        let mut entries = self.all()?;
        if section != LibrarySection::All {
            let want = section.as_str();
            entries.retain(|e| e.section == want);
        }
        Ok(entries)
    }

    /// Counts per section for the section rail UI.
    pub fn section_counts(&self) -> std::io::Result<BTreeMap<String, usize>> {
        let entries = self.all()?;
        let mut m = BTreeMap::new();
        m.insert(LibrarySection::All.as_str().into(), entries.len());
        for sec in [
            LibrarySection::Secret,
            LibrarySection::Wellfair,
            LibrarySection::Personal,
            LibrarySection::Work,
            LibrarySection::Tools,
            LibrarySection::Commons,
        ] {
            m.insert(
                sec.as_str().into(),
                entries.iter().filter(|e| e.section == sec.as_str()).count(),
            );
        }
        Ok(m)
    }

    /// Publish (or revoke) commons visibility — never allowed for Secret / high sensitivity.
    pub fn set_commons_visibility(
        &self,
        asset_uri: &str,
        visibility: CommonsVisibility,
    ) -> std::io::Result<LibraryEntry> {
        let mut entries = self.load()?;
        let e = entries
            .iter_mut()
            .find(|e| e.asset_uri == asset_uri)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "unknown asset"))?;
        if e.is_secret() && visibility != CommonsVisibility::None {
            return Err(std::io::Error::other(
                "secret / high-sensitivity items cannot join the commons or peer share lane",
            ));
        }
        e.commons_visibility = visibility;
        if visibility == CommonsVisibility::Commons {
            e.section = LibrarySection::Commons.as_str().into();
        } else if visibility == CommonsVisibility::None
            && e.section == LibrarySection::Commons.as_str()
        {
            e.recompute_section();
        }
        let out = e.clone();
        self.save(&entries)?;
        Ok(out)
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

    /// Free-text filter over uri, excerpt, topics, projects, place (case-insensitive).
    pub fn search_text(&self, query: &str) -> std::io::Result<Vec<LibraryEntry>> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return self.all();
        }
        let mut entries = self.load()?;
        entries.retain(|e| {
            e.asset_uri.to_lowercase().contains(&q)
                || e.excerpt.to_lowercase().contains(&q)
                || e.media_type.to_lowercase().contains(&q)
                || e.topics.iter().any(|t| t.to_lowercase().contains(&q))
                || e.projects.iter().any(|t| t.to_lowercase().contains(&q))
                || e.place
                    .as_ref()
                    .map(|p| p.to_lowercase().contains(&q))
                    .unwrap_or(false)
        });
        entries.sort_by(|a, b| b.ingested_unix.cmp(&a.ingested_unix));
        Ok(entries)
    }

    /// Remove an entry by asset_uri.
    pub fn remove(&self, asset_uri: &str) -> std::io::Result<bool> {
        let mut entries = self.load()?;
        let before = entries.len();
        entries.retain(|e| e.asset_uri != asset_uri);
        if entries.len() == before {
            return Ok(false);
        }
        self.save(&entries)?;
        Ok(true)
    }

    /// Library stats for the UI chrome.
    pub fn stats(&self) -> std::io::Result<LibraryStats> {
        let entries = self.load()?;
        let mut topics: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        let mut projects: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        let mut with_date = 0usize;
        let mut with_place = 0usize;
        let mut flags = 0usize;
        let mut quins = 0usize;
        for e in &entries {
            for t in &e.topics {
                *topics.entry(t.clone()).or_default() += 1;
            }
            for p in &e.projects {
                *projects.entry(p.clone()).or_default() += 1;
            }
            if e.occurred_at.is_some() {
                with_date += 1;
            }
            if e.lat.is_some() && e.lon.is_some() {
                with_place += 1;
            }
            flags += e.flags.len();
            quins += e.quins.len();
        }
        Ok(LibraryStats {
            total: entries.len(),
            with_date,
            with_place,
            flags,
            quins,
            topics,
            projects,
        })
    }

    /// Flatten all library quins for graph export / daemon inject (caller owns the slice).
    pub fn all_quins(&self) -> std::io::Result<Vec<NQuin>> {
        Ok(self
            .load()?
            .into_iter()
            .flat_map(|e| e.quins)
            .collect())
    }
}

/// Aggregate counts for the Library UI header.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LibraryStats {
    pub total: usize,
    pub with_date: usize,
    pub with_place: usize,
    pub flags: usize,
    /// Total NQuins across all containers — the semantic graph mass.
    pub quins: usize,
    pub topics: std::collections::BTreeMap<String, usize>,
    pub projects: std::collections::BTreeMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::hypermedia::{ingest_with, TextProcessor};

    fn ingest(store: &HypermediaStore, uri: &str, text: &str, now: u64) {
        let proc = TextProcessor::default();
        let r = ingest_with(&proc, uri, "text/markdown", 1, text.as_bytes());
        let mut entry = LibraryEntry {
            asset_uri: uri.to_string(),
            primary_subject: r.container.primary.subject(),
            media_type: "text/markdown".to_string(),
            quins: r.quins,
            topics: Vec::new(),
            projects: Vec::new(),
            purposes: Vec::new(),
            place: None,
            occurred_at: None,
            lat: None,
            lon: None,
            flags: Vec::new(),
            ingested_unix: now,
            excerpt: text.chars().take(40).collect(),
            sensitivity: "public".into(),
            section: "personal".into(),
            commons_visibility: CommonsVisibility::None,
        };
        entry.recompute_section();
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

    #[test]
    fn secret_section_forced_by_classified_sensitivity() {
        assert_eq!(
            resolve_section(
                "classified",
                &["health".into()],
                &[],
                CommonsVisibility::Commons,
                Some("commons")
            ),
            LibrarySection::Secret
        );
    }

    #[test]
    fn wellfair_purpose_routes_to_wellfair_when_public() {
        assert_eq!(
            resolve_section(
                "public",
                &["health-record".into()],
                &[],
                CommonsVisibility::None,
                None
            ),
            LibrarySection::Wellfair
        );
    }

    #[test]
    fn commons_visibility_routes_to_commons() {
        assert_eq!(
            resolve_section("public", &[], &[], CommonsVisibility::Commons, None),
            LibrarySection::Commons
        );
    }

    #[test]
    fn tools_purpose_routes_to_tools() {
        assert_eq!(
            resolve_section(
                "public",
                &["agent-log".into(), "telemetry".into()],
                &[],
                CommonsVisibility::None,
                None
            ),
            LibrarySection::Tools
        );
        assert_eq!(
            resolve_section("public", &[], &[], CommonsVisibility::None, Some("tools")),
            LibrarySection::Tools
        );
    }
}
