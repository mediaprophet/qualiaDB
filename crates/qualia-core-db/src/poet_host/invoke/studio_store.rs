//! Stateful Studio Engine Store & QualiaDB NQuin/Format Import-Export Bridge.
//!
//! Provides a thread-safe registry of `StudioArtifact`s across VibeScript invoke
//! calls. Emits bit-exact 48-byte `NQuin` structures with MSB=1 did:q42 pointers,
//! 29-bit Lamport clocks, permissive routing lanes, sensitivity classifications,
//! and parity checks following AGENTS.md §1.
//! Follows AGENTS.md §0-B (< 480 lines).

use crate::q_hash;
use crate::NQuin;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

#[path = "studio_journal.rs"]
mod journal;
pub use journal::{EditableSourceSnapshot, SourceJournalLimits};
#[path = "studio_stamped.rs"]
mod stamped;
pub use stamped::{artifact_stamp_is_current, create_artifact_if_absent, get_artifact_stamped, mutate_artifact_stamped,
    redo_artifact_stamped, undo_artifact_stamped, previous_source_snapshot, StoreStamp};

#[path = "studio_checkpoint/mod.rs"]
pub mod checkpoint;
#[path = "studio_cml/mod.rs"]
pub mod cml;

static STUDIO_STORE: OnceLock<Mutex<BTreeMap<String, StoredArtifact>>> = OnceLock::new();

fn store() -> &'static Mutex<BTreeMap<String, StoredArtifact>> {
    STUDIO_STORE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// In-memory stateful Studio Artifact.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudioArtifact {
    pub id: String,
    pub did_q42: u64,
    pub domain: String,
    pub title: String,
    pub lamport_clock: u32,
    pub routing_lane: u8,
    pub sensitivity: u8,
    pub context_did_hash: u64,
    pub nodes: BTreeMap<String, BTreeMap<String, String>>,
    pub properties: BTreeMap<String, String>,
    pub history: Vec<String>,
    pub redo_stack: Vec<String>,
}

// Keep the public StudioArtifact source shape unchanged. Journal state lives
// beside its artifact in the same locked store entry, never in a second map.
struct StoredArtifact {
    artifact: StudioArtifact,
    journal: journal::SourceJournal,
    incarnation: Arc<()>,
}

impl std::ops::Deref for StoredArtifact {
    type Target = StudioArtifact;
    fn deref(&self) -> &Self::Target { &self.artifact }
}

impl std::ops::DerefMut for StoredArtifact {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.artifact }
}

#[inline]
fn encode_val(v: &str) -> u64 {
    if let Ok(i) = v.parse::<i64>() {
        (0b001u64 << 60) | ((i as u64) & 0x0FFF_FFFF_FFFF_FFFF)
    } else if let Ok(b) = v.parse::<bool>() {
        (0b011u64 << 60) | (if b { 1 } else { 0 })
    } else {
        q_hash(v) & 0x0FFF_FFFF_FFFF_FFFF
    }
}

impl StudioArtifact {
    /// Creates a new Studio Artifact with did:q42 topological pointer.
    pub fn new(id: &str, domain: &str, title: &str) -> Self {
        let did_q42 = q_hash(id) | (1u64 << 63);
        Self {
            id: id.to_string(),
            did_q42,
            domain: domain.to_string(),
            title: title.to_string(),
            lamport_clock: 1,
            routing_lane: 0,
            sensitivity: 0,
            context_did_hash: q_hash("qualia:world:studio"),
            nodes: BTreeMap::new(),
            properties: BTreeMap::new(),
            history: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Emits bit-exact 48-byte `NQuin`s following AGENTS.md §1.
    pub fn compile_to_quins(&self) -> Vec<NQuin> {
        let subject = self.did_q42;
        let mut quins = Vec::new();
        let context = ((self.sensitivity as u64) << 56) | (self.context_did_hash & 0x00FF_FFFF_FFFF_FFFF);
        let metadata = (((self.routing_lane as u64) & 0b11) << 61)
            | (((self.lamport_clock as u64) & 0x1FFF_FFFF) << 32)
            | (self.nodes.len() as u64 & 0xFFFF_FFFF);

        let make_quin = |pred_iri: &str, opcode: u8, obj_val: u64| -> NQuin {
            let predicate = (q_hash(pred_iri) << 8) | (opcode as u64);
            let parity = subject ^ predicate ^ obj_val ^ context;
            NQuin { subject, predicate, object: obj_val, context, metadata, parity }
        };

        // Domain identity
        quins.push(make_quin("q42:hasStudioDomain", 0x10, q_hash(&self.domain) & 0x0FFF_FFFF_FFFF_FFFF));
        // Title
        quins.push(make_quin("q42:hasTitle", 0x11, q_hash(&self.title) & 0x0FFF_FFFF_FFFF_FFFF));

        // Properties
        for (k, v) in &self.properties {
            let pred = format!("q42:prop:{}", k);
            quins.push(make_quin(&pred, 0x12, encode_val(v)));
        }

        // Nodes & Node attributes
        for (node_id, attrs) in &self.nodes {
            let node_pred = format!("q42:node:{}", node_id);
            quins.push(make_quin(&node_pred, 0x13, q_hash(node_id) & 0x0FFF_FFFF_FFFF_FFFF));
            for (ak, av) in attrs {
                let attr_pred = format!("q42:nodeAttr:{}:{}", node_id, ak);
                quins.push(make_quin(&attr_pred, 0x14, encode_val(av)));
            }
        }
        quins
    }

    /// Exports artifact to native or industry interchange format.
    pub fn export_format(&self, format: &str) -> Result<(String, String), String> {
        match format.to_ascii_lowercase().as_str() {
            "q42" | "quins" => {
                let quins = self.compile_to_quins();
                let entries: Vec<String> = quins.iter().map(|q| {
                    let ok = q.parity == (q.subject ^ q.predicate ^ q.object ^ q.context);
                    format!(r#"{{"subject":"0x{:016x}","predicate":"0x{:016x}","object":"0x{:016x}","context":"0x{:016x}","metadata":"0x{:016x}","parity":"0x{:016x}","parity_valid":{}}}"#, q.subject, q.predicate, q.object, q.context, q.metadata, q.parity, ok)
                }).collect();
                let json = format!(r#"{{"artifact_id":"{}","did_q42":"0x{:016x}","clock":{},"quin_count":{},"quins":[{}]}}"#, self.id, self.did_q42, self.lamport_clock, quins.len(), entries.join(","));
                Ok(("application/x-qualia-quins+json".into(), json))
            }
            "10d" | "e64" => {
                let json = format!(r#"{{"envelope":"Tensor10D","did_q42":"0x{:016x}","lamport_clock":{},"domain":"{}","title":"{}","node_count":{},"axes":[1.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]}}"#, self.did_q42, self.lamport_clock, self.domain, self.title, self.nodes.len());
                Ok(("application/x-qualia-10d+json".into(), json))
            }
            "cml" => {
                Ok(("application/x-qualia-cml+xml".into(), cml::export(self, cml::CmlLimits::default())?))
            }
            "ttl" | "rdf" => {
                let mut s = format!("@prefix qualia: <https://qualia.dev/ns#> .\n\n<did:q42:0x{:016x}> a qualia:StudioArtifact ;\n  qualia:domain \"{}\" ;\n  qualia:title \"{}\" ;\n  qualia:clock {} .\n", self.did_q42, self.domain, self.title, self.lamport_clock);
                for (k, v) in &self.properties { s.push_str(&format!("<did:q42:0x{:016x}> qualia:prop_{} \"{}\" .\n", self.did_q42, k, v)); }
                for (nid, attrs) in &self.nodes {
                    s.push_str(&format!("<did:q42:0x{:016x}> qualia:hasNode <did:q42:node:{}> .\n", self.did_q42, nid));
                    for (ak, av) in attrs { s.push_str(&format!("<did:q42:node:{}> qualia:{} \"{}\" .\n", nid, ak, av)); }
                }
                Ok(("text/turtle".into(), s))
            }
            "hcf" => {
                let json = format!(r#"{{"version":"webizen.hcf/1","artifact_id":"{}","did_q42":"0x{:016x}","domain":"{}","title":"{}","clock":{},"node_count":{},"prop_count":{}}}"#, self.id, self.did_q42, self.domain, self.title, self.lamport_clock, self.nodes.len(), self.properties.len());
                Ok(("application/vnd.webizen.hcf+json".into(), json))
            }
            "docx" | "odt" | "md" => {
                let mut out = format!("# {}\n\n*Domain: {} | Lamport: {}*\n\n", self.title, self.domain, self.lamport_clock);
                for (nid, attrs) in &self.nodes {
                    let text = attrs.get("text").cloned().unwrap_or_else(|| format!("{}: {:?}", nid, attrs));
                    out.push_str(&format!("{}\n\n", text));
                }
                let mime = if format == "md" { "text/markdown" } else { "application/vnd.openxmlformats-officedocument.wordprocessingml.document" };
                Ok((mime.into(), out))
            }
            "xlsx" | "csv" => {
                let mut out = "node_id,attribute,value\n".to_string();
                for (nid, attrs) in &self.nodes {
                    for (k, v) in attrs { out.push_str(&format!("{},{},{}\n", nid, k, v)); }
                }
                let mime = if format == "csv" { "text/csv" } else { "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" };
                Ok((mime.into(), out))
            }
            "pptx" => {
                let mut out = format!("Presentation: {}\nSlides:\n", self.title);
                for (nid, attrs) in &self.nodes { out.push_str(&format!("- Slide {}: {:?}\n", nid, attrs)); }
                Ok(("application/vnd.openxmlformats-officedocument.presentationml.presentation".into(), out))
            }
            "pdf" | "xfdf" => {
                let mut out = format!("%PDF-1.7\n% StudioArtifact: {} (clock {})\n", self.title, self.lamport_clock);
                for (nid, attrs) in &self.nodes { out.push_str(&format!("/COS_{} << {:?} >>\n", nid, attrs)); }
                Ok(("application/pdf".into(), out))
            }
            "svg" => {
                let mut out = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 600" title="{}">"#, self.title);
                for (nid, attrs) in &self.nodes {
                    let d = attrs.get("d").cloned().unwrap_or_else(|| "M 0 0 L 100 100".into());
                    out.push_str(&format!(r#"<path id="{}" d="{}" />"#, nid, d));
                }
                out.push_str("</svg>");
                Ok(("image/svg+xml".into(), out))
            }
            "dxf" | "step" => {
                let mut out = "0\nSECTION\n2\nENTITIES\n".to_string();
                for (nid, _) in &self.nodes { out.push_str(&format!("0\nLINE\n8\n{}\n", nid)); }
                out.push_str("0\nENDSEC\n0\nEOF\n");
                Ok(("image/vnd.dxf".into(), out))
            }
            "fcpxml" | "edl" => {
                let mut out = format!(r#"<?xml version="1.0" encoding="UTF-8"?><fcpxml version="1.9"><project name="{}"><sequence>"#, self.title);
                for (nid, attrs) in &self.nodes { out.push_str(&format!(r#"<clip name="{}" offset="{}" />"#, nid, attrs.get("offset").cloned().unwrap_or_default())); }
                out.push_str("</sequence></project></fcpxml>");
                Ok(("application/xml".into(), out))
            }
            "psd" | "dng" => Ok(("image/vnd.adobe.photoshop".into(), format!("RasterContainer: title={}, nodes={}", self.title, self.nodes.len()))),
            "wav" | "mid" => Ok(("audio/wav".into(), format!("RIFF_AUDIO: title={}, tracks={}", self.title, self.nodes.len()))),
            other => Err(format!("Unsupported export format: {other}")),
        }
    }

    /// Imports external format payload into a populated `StudioArtifact`.
    pub fn import_format(id: &str, domain: &str, format: &str, payload: &str) -> Result<Self, String> {
        let mut art = StudioArtifact::new(id, domain, "Imported Artifact");
        match format.to_ascii_lowercase().as_str() {
            "cml" => {
                cml::import_into(&mut art, payload, cml::CmlLimits::default())?;
            }
            "csv" => {
                for line in payload.lines().skip(1) {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 3 {
                        art.nodes.entry(parts[0].to_string()).or_default().insert(parts[1].to_string(), parts[2].to_string());
                    }
                }
            }
            "md" | "markdown" => {
                for line in payload.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("# ") {
                        art.title = trimmed[2..].trim().to_string();
                    } else if !trimmed.is_empty() {
                        let nid = format!("p_{}", art.nodes.len() + 1);
                        let mut m = BTreeMap::new();
                        m.insert("text".to_string(), trimmed.to_string());
                        art.nodes.insert(nid, m);
                    }
                }
            }
            "svg" => {
                for line in payload.lines() {
                    let trimmed = line.trim();
                    if trimmed.contains("<path") {
                        let nid = extract_attr(trimmed, "id").unwrap_or_else(|| format!("path_{}", art.nodes.len() + 1));
                        let d = extract_attr(trimmed, "d").unwrap_or_default();
                        let mut m = BTreeMap::new();
                        m.insert("d".to_string(), d);
                        art.nodes.insert(nid, m);
                    }
                }
            }
            _ => {
                for line in payload.lines() {
                    if let Some((k, v)) = line.split_once('=') {
                        art.properties.insert(k.trim().to_string(), v.trim().to_string());
                    }
                }
            }
        }
        art.history.push(format!("import:{}", format));
        Ok(art)
    }
}

fn extract_attr(s: &str, attr: &str) -> Option<String> {
    let needle = format!("{}=\"", attr);
    let start = s.find(&needle)? + needle.len();
    let end = s[start..].find('"')? + start;
    Some(s[start..end].to_string())
}

// ── Public Store API ─────────────────────────────────────────────────────────

/// Persists or updates an artifact in the studio store.
/// Compatibility replacement API; it does not perform revision/authority checks.
/// Replacing source invalidates its in-memory undo snapshots. Cooperative edits
/// must use the checked mutation API rather than get/edit/upsert.
pub fn upsert_artifact(mut art: StudioArtifact) {
    art.redo_stack.clear();
    let mut s = store().lock().expect("studio store poisoned");
    s.insert(art.id.clone(), StoredArtifact { artifact: art, journal: journal::SourceJournal::default(), incarnation: Arc::new(()) });
}

/// Retrieves an artifact by ID from the studio store.
pub fn get_artifact(id: &str) -> Option<StudioArtifact> {
    let s = store().lock().expect("studio store poisoned");
    s.get(id).map(|stored| stored.artifact.clone())
}

/// Mutates an existing artifact in place, bumping its Lamport clock and recording history.
pub fn mutate_artifact<F>(id: &str, f: F) -> Result<StudioArtifact, String>
where
    F: FnOnce(&mut StudioArtifact),
{
    mutate_artifact_at(id, None, f)
}

/// Atomic revision-checked source edit. `None` preserves the legacy call shape;
/// it supplies no conflict protection. This is not an authorization check.
pub fn mutate_artifact_at<F>(id: &str, expected: Option<u32>, f: F) -> Result<StudioArtifact, String>
where
    F: FnOnce(&mut StudioArtifact),
{
    let mut s = store().lock().expect("studio store poisoned");
    let art = s.get_mut(id).ok_or_else(|| format!("Artifact '{id}' not found in studio store"))?;
    journal::edit(art, expected, f)?;
    Ok(art.artifact.clone())
}

/// Lists all artifacts in the studio store.
pub fn list_artifacts() -> Vec<StudioArtifact> {
    let s = store().lock().expect("studio store poisoned");
    s.values().map(|stored| stored.artifact.clone()).collect()
}

/// Imports external data into the store as an artifact.
pub fn import_into_store(id: &str, domain: &str, format: &str, payload: &str) -> Result<StudioArtifact, String> {
    let art = StudioArtifact::import_format(id, domain, format, payload)?;
    upsert_artifact(art.clone());
    Ok(art)
}

/// Exports an artifact from the store to the specified format.
pub fn export_from_store(id: &str, format: &str) -> Result<(String, String), String> {
    let art = get_artifact(id).ok_or_else(|| format!("Artifact '{id}' not found in studio store"))?;
    art.export_format(format)
}

/// Undoes the last mutation on an artifact.
pub fn undo_artifact(id: &str) -> Result<StudioArtifact, String> {
    undo_artifact_at(id, None)
}

/// Redoes the last undone mutation on an artifact.
pub fn redo_artifact(id: &str) -> Result<StudioArtifact, String> {
    redo_artifact_at(id, None)
}

/// Restore source while advancing revision; an old revision cannot be reused.
pub fn undo_artifact_at(id: &str, expected: Option<u32>) -> Result<StudioArtifact, String> {
    restore_artifact_at(id, expected, false)
}

/// Reapply source while advancing revision and preserving append-only history.
pub fn redo_artifact_at(id: &str, expected: Option<u32>) -> Result<StudioArtifact, String> {
    restore_artifact_at(id, expected, true)
}

fn restore_artifact_at(id: &str, expected: Option<u32>, redo: bool) -> Result<StudioArtifact, String> {
    let mut s = store().lock().expect("studio store poisoned");
    let art = s.get_mut(id).ok_or_else(|| format!("Artifact '{id}' not found in studio store"))?;
    journal::restore(art, expected, redo)?;
    Ok(art.artifact.clone())
}

/// Host-controlled cold-state budget admission. Not exposed as a Vibe edit and
/// not an authorization grant. A caller may deliberately admit larger masters.
pub fn configure_artifact_journal(id: &str, expected: Option<u32>, limits: SourceJournalLimits) -> Result<StudioArtifact, String> {
    let mut s = store().lock().expect("studio store poisoned");
    let art = s.get_mut(id).ok_or_else(|| format!("Artifact '{id}' not found in studio store"))?;
    journal::configure(art, expected, limits)?;
    Ok(art.artifact.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_lifecycle() {
        let mut art = StudioArtifact::new("art-001", "Document", "Annual Report");
        assert_eq!(art.did_q42 & (1u64 << 63), 1u64 << 63);
        art.properties.insert("author".into(), "did:qualia:01".into());
        let mut n1 = BTreeMap::new();
        n1.insert("text".into(), "Introduction section".into());
        art.nodes.insert("p1".into(), n1);

        let quins = art.compile_to_quins();
        assert!(!quins.is_empty());
        for q in &quins {
            assert_eq!(q.parity, q.subject ^ q.predicate ^ q.object ^ q.context);
        }

        upsert_artifact(art);
        let loaded = get_artifact("art-001").expect("load");
        assert_eq!(loaded.title, "Annual Report");
        assert_eq!(loaded.nodes.len(), 1);
    }

    #[test]
    fn test_export_native_formats() {
        let mut art = StudioArtifact::new("sheet-001", "Sheet", "Q3 Financials");
        let mut cell = BTreeMap::new();
        cell.insert("formula".into(), "=SUM(A1:A10)".into());
        art.nodes.insert("A11".into(), cell);

        let (mime_q42, q42) = art.export_format("q42").expect("q42");
        assert!(mime_q42.contains("quins"));
        assert!(q42.contains("parity_valid"));

        let (mime_10d, d10) = art.export_format("10d").expect("10d");
        assert!(mime_10d.contains("10d"));
        assert!(d10.contains("Tensor10D"));

        let (mime_cml, cml) = art.export_format("cml").expect("cml");
        assert!(mime_cml.contains("cml"));
        assert!(cml.contains("<q-artifact"));

        let (mime_ttl, ttl) = art.export_format("ttl").expect("ttl");
        assert!(mime_ttl.contains("turtle"));
        assert!(ttl.contains("@prefix qualia:"));
    }

    #[test]
    fn test_import_cml_and_csv() {
        let cml = r##"<q-artifact domain="Vector" did="did:q42:0x1" clock="1" title="Logo"><q-prop key="stroke" value="#000" /><q-node id="path_1"><q-attr key="d" value="M 0 0 L 50 50" /></q-node></q-artifact>"##;
        let art = StudioArtifact::import_format("vec-001", "Vector", "cml", cml).expect("cml import");
        assert_eq!(art.properties.get("stroke").map(|s| s.as_str()), Some("#000"));
        assert!(art.nodes.contains_key("path_1"));

        let csv = "node_id,attribute,value\ncell_1,val,42\ncell_2,val,99\n";
        let art_csv = StudioArtifact::import_format("sheet-csv", "Sheet", "csv", csv).expect("csv import");
        assert_eq!(art_csv.nodes.len(), 2);
    }

    #[test]
    fn test_undo_redo() {
        let art = StudioArtifact::new("art-undo", "Document", "Draft");
        upsert_artifact(art);

        let mut1 = mutate_artifact("art-undo", |a| {
            a.title = "Draft 2".into();
        }).expect("mutate");
        assert_eq!(mut1.lamport_clock, 2);

        let undid = undo_artifact("art-undo").expect("undo");
        assert_eq!(undid.lamport_clock, 3);
        assert_eq!(undid.title, "Draft");

        let redid = redo_artifact("art-undo").expect("redo");
        assert_eq!(redid.lamport_clock, 4);
        assert_eq!(redid.title, "Draft 2");
    }
}

#[cfg(test)]
#[path = "studio_journal_tests.rs"]
mod journal_tests;

#[cfg(test)]
#[path = "studio_stamped_tests.rs"]
mod stamped_tests;
