//! Pre-ingest source audit: triple counts, oversize terms, predicate histogram,
//! and RDF-vs-unsupported classification for ontology library roots.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::q_hash;
use crate::query::ingest_formats::{format_from_path, parse_triples_format, RawTriple};
use crate::query::ingest_job::IngestRdfFormat;
use crate::query::integrity_omit::MAX_LEX_TERM_BYTES;

/// How a path participates in Cross-Format Validation today.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceSupport {
    RdfImportVerify,
    ZipBag,
    UnsupportedTabular,
    UnsupportedOther,
    Missing,
}

#[derive(Clone, Debug, Serialize)]
pub struct PredicateCount {
    pub iri: String,
    pub count: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceAuditReport {
    pub path: String,
    pub format: String,
    pub support: SourceSupport,
    pub source_triple_count: u64,
    pub skipped_or_filtered: u64,
    pub oversize_literal_count: u64,
    pub first_oversize_preview: Option<String>,
    pub blank_node_triples: u64,
    pub top_predicates: Vec<PredicateCount>,
    pub comment_gloss_predicate_hits: u64,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LibraryEntry {
    pub path: String,
    pub bytes: u64,
    pub support: SourceSupport,
    pub format_hint: String,
    pub note: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct LibraryAuditMatrix {
    pub source_root: String,
    pub entries: Vec<LibraryEntry>,
}

fn support_for_path(path: &Path) -> (SourceSupport, String) {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !path.exists() {
        return (SourceSupport::Missing, "missing".into());
    }
    if name.ends_with(".zip") {
        return (SourceSupport::ZipBag, "zip".into());
    }
    if name.ends_with(".csv") || name.ends_with(".xlsx") || name.ends_with(".xls") {
        return (SourceSupport::UnsupportedTabular, "tabular".into());
    }
    let lower = path.to_string_lossy().to_ascii_lowercase();
    match format_from_path(&lower) {
        IngestRdfFormat::Auto
            if !(lower.ends_with(".owl")
                || lower.ends_with(".rdf")
                || lower.ends_with(".ttl")
                || lower.ends_with(".nt")
                || lower.ends_with(".nq")
                || lower.ends_with(".trig")
                || lower.ends_with(".jsonld")
                || lower.ends_with(".json-ld")) =>
        {
            (SourceSupport::UnsupportedOther, "unknown".into())
        }
        fmt => (SourceSupport::RdfImportVerify, format!("{fmt:?}")),
    }
}

/// Scan one RDF source the same way import would (Rio dispatch), without writing a volume.
pub fn audit_rdf_source(path: &Path, top_n: usize) -> io::Result<SourceAuditReport> {
    let (support, format_label) = support_for_path(path);
    let path_s = path.display().to_string();
    if support != SourceSupport::RdfImportVerify {
        return Ok(SourceAuditReport {
            path: path_s,
            format: format_label,
            support,
            source_triple_count: 0,
            skipped_or_filtered: 0,
            oversize_literal_count: 0,
            first_oversize_preview: None,
            blank_node_triples: 0,
            top_predicates: Vec::new(),
            comment_gloss_predicate_hits: 0,
            notes: vec![
                "Not an RDF serialisation supported by import/verify-graph for Cross-Format Validation."
                    .into(),
            ],
        });
    }

    let lower = path.to_string_lossy().to_ascii_lowercase();
    let fmt = format_from_path(&lower);
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut predicate_counts: HashMap<String, u64> = HashMap::new();
    let mut source_triple_count = 0u64;
    let mut oversize_literal_count = 0u64;
    let mut first_oversize_preview = None;
    let mut blank_node_triples = 0u64;
    let gloss: std::collections::HashSet<u64> =
        crate::query::integrity_omit::COMMENT_GLOSS_PREDICATE_IRIS
            .iter()
            .map(|iri| q_hash(iri))
            .collect();
    let mut comment_gloss_predicate_hits = 0u64;

    let mut on_triple = |raw: RawTriple| {
        source_triple_count += 1;
        *predicate_counts.entry(raw.predicate.clone()).or_default() += 1;
        if gloss.contains(&q_hash(&raw.predicate)) {
            comment_gloss_predicate_hits += 1;
        }
        let blank = raw.subject.starts_with("_:") || raw.object.starts_with("_:");
        if blank {
            blank_node_triples += 1;
        }
        for part in [&raw.subject, &raw.predicate, &raw.object] {
            if part.len() > MAX_LEX_TERM_BYTES {
                oversize_literal_count += 1;
                if first_oversize_preview.is_none() {
                    let preview: String = part.chars().take(120).collect();
                    first_oversize_preview = Some(format!("{}… ({} bytes)", preview, part.len()));
                }
            }
        }
    };

    if let Err(e) = parse_triples_format(fmt, reader, None, &mut on_triple) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, e));
    }

    let mut ranked: Vec<PredicateCount> = predicate_counts
        .into_iter()
        .map(|(iri, count)| PredicateCount { iri, count })
        .collect();
    ranked.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.iri.cmp(&b.iri)));
    ranked.truncate(top_n.max(1));

    let mut notes = Vec::new();
    if oversize_literal_count > 0 {
        notes.push(format!(
            "{oversize_literal_count} term(s) exceed {MAX_LEX_TERM_BYTES} UTF-8 bytes (Complete ingest fails closed / TermTooLong)."
        ));
    }
    if blank_node_triples > 0 {
        notes.push(
            "Source contains blank nodes; ground-graph proof will report BlankNodeCanonicalizationRequired when sets match."
                .into(),
        );
    }

    Ok(SourceAuditReport {
        path: path_s,
        format: format!("{fmt:?}"),
        support,
        source_triple_count,
        skipped_or_filtered: 0,
        oversize_literal_count,
        first_oversize_preview,
        blank_node_triples,
        top_predicates: ranked,
        comment_gloss_predicate_hits,
        notes,
    })
}

/// Walk a local ontology library root (files + datasources/*.zip) into a support matrix.
pub fn audit_source_root(root: &Path) -> io::Result<LibraryAuditMatrix> {
    let mut entries = Vec::new();
    push_if_exists(&mut entries, root.join("chebi.owl"));
    let oewn = root.join("english-wordnet-2025-plus");
    if oewn.is_dir() {
        for ent in std::fs::read_dir(&oewn)? {
            let ent = ent?;
            let p = ent.path();
            if p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("ttl"))
                == Some(true)
            {
                push_file(&mut entries, &p)?;
            }
        }
    }
    let datasources = root.join("datasources");
    if datasources.is_dir() {
        for ent in std::fs::read_dir(&datasources)? {
            let ent = ent?;
            let p = ent.path();
            if p.is_file() {
                push_file(&mut entries, &p)?;
            }
        }
    }
    // Also list loose RDF next to root.
    if root.is_dir() {
        for ent in std::fs::read_dir(root)? {
            let ent = ent?;
            let p = ent.path();
            if !p.is_file() {
                continue;
            }
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.eq_ignore_ascii_case("chebi.owl") {
                continue;
            }
            let lower = name.to_ascii_lowercase();
            if lower.ends_with(".ttl")
                || lower.ends_with(".nt")
                || lower.ends_with(".owl")
                || lower.ends_with(".rdf")
            {
                push_file(&mut entries, &p)?;
            }
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(LibraryAuditMatrix {
        source_root: root.display().to_string(),
        entries,
    })
}

fn push_if_exists(entries: &mut Vec<LibraryEntry>, path: PathBuf) {
    if path.exists() {
        let _ = push_file(entries, &path);
    }
}

fn push_file(entries: &mut Vec<LibraryEntry>, path: &Path) -> io::Result<()> {
    let bytes = std::fs::metadata(path)?.len();
    let (support, format_hint) = support_for_path(path);
    let note = match support {
        SourceSupport::RdfImportVerify => "import + verify-graph --encoder=import".into(),
        SourceSupport::ZipBag => "unpack then audit members; RDF members are Tier-2".into(),
        SourceSupport::UnsupportedTabular => {
            "CSV/XLSX are not RDF graph import targets in this landing".into()
        }
        SourceSupport::UnsupportedOther => "format not supported for verify-graph".into(),
        SourceSupport::Missing => "path missing".into(),
    };
    entries.push(LibraryEntry {
        path: path.display().to_string(),
        bytes,
        support,
        format_hint,
        note,
    });
    Ok(())
}
