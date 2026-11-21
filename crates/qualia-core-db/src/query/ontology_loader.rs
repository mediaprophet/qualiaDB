#![cfg(not(target_arch = "wasm32"))]
//! Startup ontology loader — parses bundled Turtle/N-Triples ontology files into NQuins
//! and seeds the daemon graph at startup.
//!
//! Ontology files are resolved in priority order:
//!   1. `$QUALIA_ONTOLOGY_PATH/` environment variable if set
//!   2. `ontologies/` relative to the current working directory (dev / workspace layout)
//!   3. Alongside the binary: `<exe-dir>/ontologies/`
//!
//! Any file that cannot be read or parsed emits a log warning and is skipped — the daemon
//! starts successfully even if ontology files are absent.

use crate::{q_hash, NQuin};

// Canonical named graphs for each ontology.
const RIGHTS_GRAPH: u64 = q_hash("urn:qualia:ontology:rights");
const COGAI_GRAPH: u64 = q_hash("urn:qualia:ontology:cogai");
const EPISTEMIC_GRAPH: u64 = q_hash("urn:qualia:ontology:epistemic");
const UDHR_GRAPH: u64 = q_hash("urn:qualia:ontology:udhr");
const CRC_GRAPH: u64 = q_hash("urn:qualia:ontology:crc");
const PLATFORMS_GRAPH: u64 = q_hash("urn:qualia:ontology:platforms");
const EMOJI_GRAPH: u64 = q_hash("urn:qualia:ontology:emoji");
const CRC_ANNOTATED_GRAPH: u64 = q_hash("urn:qualia:ontology:crc_annotated");
const CRPD_ANNOTATED_GRAPH: u64 = q_hash("urn:qualia:ontology:crpd_annotated");
const ICCPR_ANNOTATED_GRAPH: u64 = q_hash("urn:qualia:ontology:iccpr_annotated");
const ICESCR_ANNOTATED_GRAPH: u64 = q_hash("urn:qualia:ontology:icescr_annotated");
const UDHR_ANNOTATED_GRAPH: u64 = q_hash("urn:qualia:ontology:udhr_annotated");
const CAT_ANNOTATED_GRAPH: u64 = q_hash("urn:qualia:ontology:cat_annotated");
const LAWYERS_PRINCIPLES_ANNOTATED_GRAPH: u64 =
    q_hash("urn:qualia:ontology:lawyers_principles_annotated");
const REMEDY_REPARATION_ANNOTATED_GRAPH: u64 =
    q_hash("urn:qualia:ontology:remedy_reparation_annotated");
const RIGHT_DEVELOPMENT_ANNOTATED_GRAPH: u64 =
    q_hash("urn:qualia:ontology:right_development_annotated");
const TORTURE_DECLARATION_ANNOTATED_GRAPH: u64 =
    q_hash("urn:qualia:ontology:torture_declaration_annotated");
const HR_DEFENDERS_ANNOTATED_GRAPH: u64 = q_hash("urn:qualia:ontology:hr_defenders_annotated");
const PEOPLES_PEACE_ANNOTATED_GRAPH: u64 = q_hash("urn:qualia:ontology:peoples_peace_annotated");
const MENTAL_ILLNESS_PRINCIPLES_ANNOTATED_GRAPH: u64 =
    q_hash("urn:qualia:ontology:mental_illness_principles_annotated");
const ISTANBUL_PROTOCOL_ANNOTATED_GRAPH: u64 =
    q_hash("urn:qualia:ontology:istanbul_protocol_annotated");
const PALERMO_PROTOCOL_ANNOTATED_GRAPH: u64 =
    q_hash("urn:qualia:ontology:palermo_protocol_annotated");
const COMMONWEALTH_CHARTER_ANNOTATED_GRAPH: u64 =
    q_hash("urn:qualia:ontology:commonwealth_charter_annotated");
const DNS_RECORD_TYPES_GRAPH: u64 = q_hash("urn:qualia:ontology:dns");
const TERN_GRAPH: u64 = q_hash("urn:qualia:ontology:tern");
const DWC_GRAPH: u64 = q_hash("urn:qualia:ontology:dwc");
const AUNSL_GRAPH: u64 = q_hash("urn:qualia:ontology:aunsl");
const MA_ONT_GRAPH: u64 = q_hash("urn:qualia:ontology:ma_ont");
const SPARQL_MM_GRAPH: u64 = q_hash("urn:qualia:ontology:sparql_mm");
const QUDT_GRAPH: u64 = q_hash("urn:qualia:ontology:qudt");

/// Files to load at startup, as `(filename, named_graph_context)` pairs.
const STARTUP_ONTOLOGIES: &[(&str, u64)] = &[
    ("dns_record_types.ttl", DNS_RECORD_TYPES_GRAPH),
    ("rights_ontology.ttl", RIGHTS_GRAPH),
    ("cogai_shapes.ttl", COGAI_GRAPH),
    ("epistemic_shapes.ttl", EPISTEMIC_GRAPH),
    ("udhr.ttl", UDHR_GRAPH),
    ("crc.ttl", CRC_GRAPH),
    ("platforms.ttl", PLATFORMS_GRAPH),
    ("emoji.n3", EMOJI_GRAPH),
    ("udhr_annotated.ttl", UDHR_ANNOTATED_GRAPH),
    ("crc_annotated.ttl", CRC_ANNOTATED_GRAPH),
    ("crpd_annotated.ttl", CRPD_ANNOTATED_GRAPH),
    ("iccpr_annotated.ttl", ICCPR_ANNOTATED_GRAPH),
    ("icescr_annotated.ttl", ICESCR_ANNOTATED_GRAPH),
    ("cat_annotated.ttl", CAT_ANNOTATED_GRAPH),
    (
        "lawyers_principles_annotated.ttl",
        LAWYERS_PRINCIPLES_ANNOTATED_GRAPH,
    ),
    (
        "remedy_reparation_annotated.ttl",
        REMEDY_REPARATION_ANNOTATED_GRAPH,
    ),
    (
        "right_development_annotated.ttl",
        RIGHT_DEVELOPMENT_ANNOTATED_GRAPH,
    ),
    (
        "torture_declaration_annotated.ttl",
        TORTURE_DECLARATION_ANNOTATED_GRAPH,
    ),
    ("hr_defenders_annotated.ttl", HR_DEFENDERS_ANNOTATED_GRAPH),
    ("peoples_peace_annotated.ttl", PEOPLES_PEACE_ANNOTATED_GRAPH),
    (
        "mental_illness_principles_annotated.ttl",
        MENTAL_ILLNESS_PRINCIPLES_ANNOTATED_GRAPH,
    ),
    (
        "istanbul_protocol_annotated.ttl",
        ISTANBUL_PROTOCOL_ANNOTATED_GRAPH,
    ),
    (
        "palermo_protocol_annotated.ttl",
        PALERMO_PROTOCOL_ANNOTATED_GRAPH,
    ),
    (
        "commonwealth_charter_annotated.ttl",
        COMMONWEALTH_CHARTER_ANNOTATED_GRAPH,
    ),
    ("tern.ttl", TERN_GRAPH),
    ("darwin-core.rdf", DWC_GRAPH),
    ("aunsl.ttl", AUNSL_GRAPH),
    ("ma-ont.ttl", MA_ONT_GRAPH),
    ("sparql-mm.ttl", SPARQL_MM_GRAPH),
    ("qudt-schema.ttl", QUDT_GRAPH),
];

/// Startup ontology catalog: `(filename, named_graph_context_hash)`.
pub fn startup_ontology_catalog() -> &'static [(&'static str, u64)] {
    STARTUP_ONTOLOGIES
}

/// Resolved ontologies directory, if present on disk.
pub fn ontology_dir_path() -> Option<std::path::PathBuf> {
    find_ontology_dir()
}

/// Discover the ontologies directory.
fn find_ontology_dir() -> Option<std::path::PathBuf> {
    // 1. Environment variable override.
    if let Ok(p) = std::env::var("QUALIA_ONTOLOGY_PATH") {
        let pb = std::path::PathBuf::from(p);
        if pb.is_dir() {
            return Some(pb);
        }
    }

    // 2. `./ontologies/` (workspace root when running via `cargo run`).
    let cwd = std::path::PathBuf::from("ontologies");
    if cwd.is_dir() {
        return Some(cwd);
    }

    // 3. Next to the binary.
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.parent().map(|p| p.join("ontologies"));
        if let Some(ref s) = sibling {
            if s.is_dir() {
                return Some(s.clone());
            }
        }
    }

    None
}

/// Parse RDF bytes of an explicit [`crate::query::ingest_job::IngestRdfFormat`]
/// into NQuins plus the `hash -> term` lexicon used by SPARQL lookup.
///
/// Uses [`crate::query::ingest_formats::parse_triples_format`] so IRIs are hashed
/// as bare URI strings (`http://...`, without `<...>`) and literals as bare lexical
/// values (or inline-packed typed literals), matching `sparql_parser` and
/// `ingest::streaming_import_rdf`.
pub fn parse_rdf_bytes_to_quins_and_lex(
    bytes: &[u8],
    format: crate::query::ingest_job::IngestRdfFormat,
    graph_context: u64,
) -> Result<(Vec<NQuin>, std::collections::HashMap<u64, String>), String> {
    let mut quins = Vec::new();
    let mut lex = std::collections::HashMap::new();
    crate::query::ingest_formats::parse_triples_format(
        format,
        std::io::BufReader::new(std::io::Cursor::new(bytes)),
        None,
        &mut |raw| {
            let s = q_hash(&raw.subject);
            let p = q_hash(&raw.predicate);
            let is_inline = raw.packed_object.is_some();
            let o = raw
                .packed_object
                .unwrap_or_else(|| q_hash(&raw.object) & crate::query::ingest_formats::OBJECT_IRI_MASK);
            let ctx = if raw.context != 0 {
                raw.context
            } else {
                graph_context
            };
            lex.entry(s).or_insert(raw.subject);
            lex.entry(p).or_insert(raw.predicate);
            if !is_inline {
                lex.entry(o).or_insert(raw.object);
            }
            quins.push(NQuin {
                subject: s,
                predicate: p,
                object: o,
                context: ctx,
                metadata: 0,
                parity: s ^ p ^ o ^ ctx,
            });
        },
    )?;
    Ok((quins, lex))
}

/// Parse Turtle/N3 bytes into NQuins plus the hash→term lexicon used by SPARQL lookup.
///
/// Each triple becomes:
///   `NQuin { subject = q_hash(subject_iri), predicate = q_hash(pred_iri),
///            object = q_hash(object_str), context = graph_context, ... }`
pub fn parse_ttl_bytes_to_quins_and_lex(
    bytes: &[u8],
    graph_context: u64,
) -> Result<(Vec<NQuin>, std::collections::HashMap<u64, String>), String> {
    parse_rdf_bytes_to_quins_and_lex(
        bytes,
        crate::query::ingest_job::IngestRdfFormat::Turtle,
        graph_context,
    )
}

/// Parse an RDF/OWL/Turtle/N3/NT/JSON-LD file into NQuins + lexicon, placed in `graph_context`.
pub fn parse_rdf_file_to_quins_and_lex(
    path: &std::path::Path,
    graph_context: u64,
) -> Result<(Vec<NQuin>, std::collections::HashMap<u64, String>), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot open {:?}: {e}", path))?;
    let path_str = path.to_string_lossy();
    let fmt = crate::query::ingest_job::infer_rdf_format(&path_str);
    parse_rdf_bytes_to_quins_and_lex(&bytes, fmt, graph_context)
}

/// Parse a single RDF/Turtle/OWL file into NQuins, all placed in `graph_context`.
pub fn parse_ttl_to_quins(path: &std::path::Path, graph_context: u64) -> Vec<NQuin> {
    match parse_rdf_file_to_quins_and_lex(path, graph_context) {
        Ok((quins, lex)) => {
            crate::daemon_graph::extend_graph_lexicon(lex);
            log::info!(
                "[ontology_loader] loaded {} quins from {:?}",
                quins.len(),
                path
            );
            quins
        }
        Err(e) => {
            log::warn!("[ontology_loader] parse error in {:?}: {e}", path);
            Vec::new()
        }
    }
}

/// Load a unified `.q42` volume ontology file.
pub fn load_q42_file(path: &std::path::Path) -> Vec<NQuin> {
    let all_quins = match crate::q42_reader::read_q42_quins(path) {
        Ok(quins) => quins,
        Err(e) => {
            log::warn!("[ontology_loader] cannot open q42 volume {:?}: {e}", path);
            return Vec::new();
        }
    };
    if let Ok(lex) = crate::q42_lex::Q42Lexicon::load_for_q42(path) {
        crate::daemon_graph::extend_graph_lexicon(lex.entries);
    }
    log::info!(
        "[ontology_loader] loaded {} quins from unified volume {:?}",
        all_quins.len(),
        path
    );
    all_quins
}

/// Load all startup ontologies into the daemon graph.
///
/// Call this once, immediately after `daemon_graph::init_daemon_graph()`.
pub fn load_startup_ontologies() {
    let dir = match find_ontology_dir() {
        Some(d) => d,
        None => {
            log::info!("[ontology_loader] no ontologies directory found — skipping");
            return;
        }
    };

    log::info!("[ontology_loader] loading ontologies from {:?}", dir);

    let mut all_quins: Vec<NQuin> = Vec::new();
    for (filename, context) in STARTUP_ONTOLOGIES {
        let path = dir.join(filename);
        if !path.exists() {
            log::warn!("[ontology_loader] {:?} not found — skipping", path);
            continue;
        }
        let quins = parse_ttl_to_quins(&path, *context);
        all_quins.extend(quins);
    }

    // Load any binary .q42 files present in the ontologies directory
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("q42") {
                let quins = load_q42_file(&path);
                all_quins.extend(quins);
            }
        }
    }

    crate::daemon_graph::extend_with_ontology_quins(all_quins);
    log::info!(
        "[ontology_loader] daemon graph now has {} quins after ontology seed",
        crate::daemon_graph::graph_quin_count(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_ttl_minimal() {
        let ttl = b"@prefix ex: <http://example.org/> .\nex:Alice a ex:Person .\n";
        let tmp = tempfile::NamedTempFile::new().expect("tmp");
        tmp.as_file().write_all(ttl).unwrap();
        let quins = parse_ttl_to_quins(tmp.path(), 0xCAFE);
        assert!(!quins.is_empty());
        assert!(quins.iter().all(|q| q.context == 0xCAFE));
        let (from_bytes, lex) = parse_ttl_bytes_to_quins_and_lex(ttl, 0xCAFE).unwrap();
        assert_eq!(from_bytes.len(), quins.len());
        assert!(lex.values().any(|t| t.contains("Alice")));
    }

    #[test]
    fn parse_ttl_missing_file_returns_empty() {
        let quins = parse_ttl_to_quins(std::path::Path::new("/nonexistent/file.ttl"), 0);
        assert!(quins.is_empty());
    }
}
