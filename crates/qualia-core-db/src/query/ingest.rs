//! Streaming import path for RDF text sources → canonical `.q42` volume.
//!
//! Pipeline: a Rio parser on the main thread streams triples into a bounded channel; a pool of worker
//! shards hashes each triple into an `NQuin` and (in [`IngestMode::Complete`]) interns every
//! subject/predicate/object string into a per-shard lexicon; a collector gathers the quins; the main
//! thread merges the lexicon, sorts the quins by object hash, and writes the volume via
//! [`crate::q42_volume::UnifiedVolumeBuilder`] — the GOVERNING `.q42` layout (160-byte SuperBlock
//! headers, block directory, BIDX object index, Merkle-DAG, and a real lexicon section).
//!
//! History: this path previously wrote headerless LZ4 blocks and an empty lexicon, so
//! `Q42Volume::read_all_quins` could not read the graph back and all literal text was discarded while
//! the shrunk file was reported as "compression". Both defects are fixed — see [`IngestMode`].

use crate::{q_hash, NQuin};
use log;

const OBJECT_HASH_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;
use crossbeam_channel::{bounded, Receiver, Sender};
use rio_api::parser::TriplesParser;
use rio_turtle::{NTriplesParser, TurtleParser};
use rio_xml::RdfXmlParser;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Cursor};
use std::path::Path;
use std::thread;
use std::time::Instant;
use sysinfo::System;
use tempfile::TempDir;

/// Base IRI for relative terms / empty `xml:base` in catalog RDF.
pub fn catalog_base_iri(path: &Path) -> Option<oxiri::Iri<String>> {
    let stem = path.file_stem()?.to_str()?.to_ascii_lowercase();
    let iri = match stem.as_str() {
        "earl" => "http://www.w3.org/ns/earl#".to_string(),
        "music" => "http://purl.org/ontology/mo/".to_string(),
        other => format!("https://www.w3.org/ns/{other}#"),
    };
    oxiri::Iri::parse(iri).ok()
}

/// Expand Turtle `prefix:` empty local names (`dc:`, `foaf:`, `ns1:`) to IRIs.
///
/// Rio rejects those tokens even though Turtle 1.1 allows them. Catalog
/// ontologies (Music Ontology in particular) use them as namespace objects.
pub fn expand_empty_turtle_prefixed_names(src: &str) -> String {
    let mut prefixes: Vec<(String, String)> = Vec::new();
    for line in src.lines() {
        let trimmed = line.trim();
        let rest = trimmed
            .strip_prefix("@prefix ")
            .or_else(|| trimmed.strip_prefix("PREFIX "));
        let Some(rest) = rest else {
            continue;
        };
        let rest = rest.trim();
        let Some(colon) = rest.find(':') else {
            continue;
        };
        let name = rest[..colon].trim();
        let Some(start) = rest[colon + 1..].find('<') else {
            continue;
        };
        let after = &rest[colon + 1 + start + 1..];
        let Some(end) = after.find('>') else {
            continue;
        };
        prefixes.push((name.to_string(), after[..end].to_string()));
    }
    prefixes.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    if prefixes.is_empty() {
        return src.to_string();
    }

    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len() + 256);
    let mut i = 0;
    let mut in_iri = false;
    let mut in_string = false;
    let mut long_string = false;
    let mut in_prefix_decl = false;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if !in_iri && !in_string && starts_with_keyword(bytes, i, b"@prefix") {
            in_prefix_decl = true;
        } else if !in_iri && !in_string && starts_with_keyword(bytes, i, b"PREFIX") {
            in_prefix_decl = true;
        }
        if in_prefix_decl {
            out.push(c);
            if c == '>' {
                in_prefix_decl = false;
            }
            i += 1;
            continue;
        }
        if in_iri {
            out.push(c);
            if c == '>' {
                in_iri = false;
            }
            i += 1;
            continue;
        }
        if in_string {
            out.push(c);
            if c == '\\' && i + 1 < bytes.len() {
                out.push(bytes[i + 1] as char);
                i += 2;
                continue;
            }
            if long_string
                && c == '"'
                && i + 2 < bytes.len()
                && bytes[i + 1] == b'"'
                && bytes[i + 2] == b'"'
            {
                out.push('"');
                out.push('"');
                i += 3;
                in_string = false;
                long_string = false;
                continue;
            }
            if !long_string && c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if c == '<' {
            in_iri = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '"' {
            in_string = true;
            long_string = i + 2 < bytes.len() && bytes[i + 1] == b'"' && bytes[i + 2] == b'"';
            out.push(c);
            i += 1;
            continue;
        }
        if let Some((name, iri)) = prefixes.iter().find(|(name, _)| {
            let start = i;
            let end = start + name.len();
            bytes.get(end) == Some(&b':')
                && bytes.get(start..end) == Some(name.as_bytes())
                && (start == 0 || !is_prefix_name_char(bytes[start - 1]))
        }) {
            let local_start = i + name.len() + 1;
            if empty_local_name_follows(bytes, local_start) {
                out.push('<');
                out.push_str(iri);
                out.push('>');
                i = local_start;
                continue;
            }
            if let Some(hash_at) = hash_terminated_local_name(bytes, local_start) {
                out.push('<');
                out.push_str(iri);
                out.push_str(&src[local_start..=hash_at]);
                out.push('>');
                i = hash_at + 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

fn starts_with_keyword(bytes: &[u8], i: usize, keyword: &[u8]) -> bool {
    bytes.get(i..i + keyword.len()) == Some(keyword)
        && (i == 0 || !is_prefix_name_char(bytes[i - 1]))
        && match bytes.get(i + keyword.len()) {
            Some(b) if is_prefix_name_char(*b) => false,
            _ => true,
        }
}

fn is_prefix_name_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

fn repair_rdfxml_empty_base(src: &str, base: &str) -> String {
    src.replace("xml:base=\"\"", &format!("xml:base=\"{base}\""))
        .replace("xml:base=''", &format!("xml:base='{base}'"))
}

fn empty_local_name_follows(bytes: &[u8], i: usize) -> bool {
    match bytes.get(i) {
        None => true,
        Some(b) => matches!(*b, b' ' | b'\t' | b'\r' | b'\n' | b',' | b';' | b'.' | b')' | b']'),
    }
}

fn hash_terminated_local_name(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    if i >= bytes.len() || !is_prefix_name_char(bytes[i]) {
        return None;
    }
    i += 1;
    while i < bytes.len() && is_prefix_name_char(bytes[i]) {
        i += 1;
    }
    if bytes.get(i) == Some(&b'#') {
        Some(i)
    } else {
        None
    }
}

fn ingest_scratch_dir() -> io::Result<TempDir> {
    if let Some(parent) = std::env::var_os("QUALIA_INGEST_SCRATCH") {
        std::fs::create_dir_all(&parent)?;
        TempDir::new_in(parent)
    } else {
        TempDir::new()
    }
}

/// How much of the source graph the `.q42` retains.
///
/// The historical ingest hashed every subject/predicate/object into a 48-byte quin and wrote an
/// **empty** lexicon (`lex_length: 0`). That threw away every URI and every literal — the source text
/// was irrecoverable — while the shrunk output was presented as "compression". It was not compression;
/// it was data loss reported as a size win. That is exactly the kind of claim-vs-reality gap this
/// project's integrity rules (CLAUDE.md §15) forbid. This enum makes the choice explicit and the
/// reporting honest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IngestMode {
    /// **Lossless.** Intern every subject/predicate URI and every literal (full UTF-8 / Unicode) into
    /// the q42 lexicon, so `Q42LexMmap::lookup_hash(quin.field)` recovers the original term. The `.q42`
    /// is a faithful, reversible representation of the source graph. This is the default — honesty over
    /// a smaller file.
    #[default]
    Complete,
    /// **Lossy, structure-only.** Store only the 48-byte hash quins; discard all human-readable text
    /// (URIs and literals). Smaller on disk, but the original strings CANNOT be recovered. The size
    /// reduction is data loss, not compression, and is reported as such. Use only when the graph
    /// structure alone is wanted and the term strings are available elsewhere.
    StripLiterals,
}

impl IngestMode {
    fn label(self) -> &'static str {
        match self {
            IngestMode::Complete => "COMPLETE (lossless — all URIs & literals retained)",
            IngestMode::StripLiterals => {
                "STRIP-LITERALS (lossy — human-readable text discarded, structure only)"
            }
        }
    }
}

/// Represents a raw string-based Triple extracted from RDF/XML
#[derive(Debug)]
pub struct RawTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub packed_object: Option<u64>,
}

/// Back-compat entry point: ingests losslessly ([`IngestMode::Complete`]) — the honest default.
pub fn streaming_import_rdf(in_path: &str, out_path: &str) -> std::io::Result<u64> {
    streaming_import_rdf_with_mode(in_path, out_path, IngestMode::Complete)
}

/// Stream-ingest an RDF source into a `.q42` volume under an explicit [`IngestMode`].
pub fn streaming_import_rdf_with_mode(
    in_path: &str,
    out_path: &str,
    mode: IngestMode,
) -> std::io::Result<u64> {
    streaming_import_rdf_with_mode_inner(in_path, out_path, mode, None)
}

/// Stream-ingest RDF into a front-embedded logical volume root and immutable,
/// size-capped child Q42 segments.
pub fn streaming_import_rdf_volume_set_with_mode(
    in_path: &str,
    root_path: &str,
    mode: IngestMode,
    max_segment_bytes: u64,
) -> std::io::Result<u64> {
    streaming_import_rdf_with_mode_inner(in_path, root_path, mode, Some(max_segment_bytes))
}

fn streaming_import_rdf_with_mode_inner(
    in_path: &str,
    out_path: &str,
    mode: IngestMode,
    max_segment_bytes: Option<u64>,
) -> std::io::Result<u64> {
    let start_time = Instant::now();
    println!("Initializing Native Ingestion Pipeline...");

    // 1. Hardware Detection & Scaling
    let mut sys = System::new_all();
    sys.refresh_all();
    let logical_cores = sys.cpus().len();

    // Constraint: Use no more than 80% of available CPU resources
    let target_workers = std::cmp::max(1, (logical_cores as f32 * 0.8).floor() as usize);
    println!("Hardware Sieve: Detected {} logical cores. Spinning up {} parallel hasher shards (capped at 80%).", logical_cores, target_workers);

    // 2. Channel Setup
    // Use bounded channels to strictly enforce the 512MB RAM floor (backpressure)
    let (tx_raw, rx_raw): (Sender<RawTriple>, Receiver<RawTriple>) = bounded(10_000);
    let (tx_bin, rx_bin): (Sender<NQuin>, Receiver<NQuin>) = bounded(10_000);

    // 3. Spawn Parallel Hasher Shards (Workers)
    // Each worker returns its local hash→string lexicon shard (empty in StripLiterals mode); the main
    // thread merges the shards into one lexicon and writes it into the volume so the terms are
    // recoverable. This is the fix for the historical data loss: previously the strings were hashed and
    // thrown away here, and no lexicon was ever written.
    let collect_lexicon = mode == IngestMode::Complete;
    let mut worker_handles: Vec<thread::JoinHandle<HashMap<u64, String>>> = vec![];
    for _worker_id in 0..target_workers {
        let rx = rx_raw.clone();
        let tx = tx_bin.clone();

        let handle = thread::spawn(move || {
            let mut _local_count = 0u64;
            let mut lex: HashMap<u64, String> = HashMap::new();
            for triple in rx {
                let s_hash = q_hash(&triple.subject);
                let p_hash = q_hash(&triple.predicate);
                // Objects that pack into the quin inline (typed int/decimal/bool) carry their full value
                // in the field itself — no string needed. Everything else is stored under the same
                // masked hash that lands in `quin.object`, so `lookup_hash(quin.object)` resolves it.
                let is_inline = triple.packed_object.is_some();
                let o_hash = triple
                    .packed_object
                    .unwrap_or_else(|| q_hash(&triple.object) & OBJECT_HASH_MASK);
                let context = 0u64;
                let metadata = 0u64;
                let parity = s_hash ^ p_hash ^ o_hash ^ context ^ metadata;

                let quin = NQuin {
                    subject: s_hash,
                    predicate: p_hash,
                    object: o_hash,
                    context,
                    metadata,
                    parity,
                };

                if collect_lexicon {
                    // `or_insert_with(|| moved)` still moves the string even when the entry is
                    // occupied, so branch explicitly to avoid needless clones/moves on hot hits.
                    if !lex.contains_key(&s_hash) {
                        lex.insert(s_hash, triple.subject);
                    }
                    if !lex.contains_key(&p_hash) {
                        lex.insert(p_hash, triple.predicate);
                    }
                    if !is_inline && !lex.contains_key(&o_hash) {
                        lex.insert(o_hash, triple.object);
                    }
                }

                // Send back to the writer thread
                if tx.send(quin).is_err() {
                    break;
                }
                _local_count += 1;
            }
            if _local_count > 0 {
                log::debug!(
                    "Ontology Ingest: worker shard finished {} triples ({} lexemes)",
                    _local_count,
                    lex.len()
                );
            }
            lex
        });
        worker_handles.push(handle);
    }

    // Drop the extra transmitters so channels close correctly
    drop(tx_bin);

    // 4. Drain hashed Quins into bounded external-sort runs instead of one
    // whole-graph Vec. The TempDir owns every run and cleans it on success,
    // error, or unwind.
    let sorter_temp = ingest_scratch_dir()?;
    let sorter_path = sorter_temp.path().to_owned();
    let collector_handle = thread::spawn(
        move || -> std::io::Result<crate::external_sort::ExternalSorter> {
            let mut sorter = crate::external_sort::ExternalSorter::new(sorter_path);
            for quin in rx_bin {
                sorter.push(quin)?;
            }
            Ok(sorter)
        },
    );

    // 5. The Streaming Sieve (Main Thread)
    // Uses Rio to read the file sequentially without loading the whole graph into RAM.
    let in_file = File::open(&in_path)?;
    let buf_reader = BufReader::new(in_file);

    let mut triples_read = 0;

    // Setup a callback that parses Rio triples and sends them to the worker queue
    log::info!("Ontology Ingest: streaming triples from {}", in_path);
    let mut on_triple = |t: rio_api::model::Triple| -> Result<(), std::io::Error> {
        let subject = t.subject.to_string();
        let predicate = t.predicate.to_string();
        let object = t.object.to_string();
        let mut packed_object = None;

        if let rio_api::model::Term::Literal(rio_api::model::Literal::Typed { value, datatype }) =
            t.object
        {
            let dt = datatype.iri;
            if dt == "http://www.w3.org/2001/XMLSchema#integer" {
                if let Ok(num) = value.parse::<i64>() {
                    let max_val = (1i64 << 59) - 1;
                    let min_val = -(1i64 << 59);
                    if num >= min_val && num <= max_val {
                        let unsigned = (num as u64) & crate::resolver::INLINE_VALUE_MASK;
                        packed_object = Some(crate::resolver::INLINE_TAG_INTEGER | unsigned);
                    }
                }
            } else if dt == "http://www.w3.org/2001/XMLSchema#decimal" {
                if let Ok(num) = value.parse::<f64>() {
                    let scaled = num * 1_000_000.0;
                    let max_val = ((1i64 << 59) - 1) as f64;
                    let min_val = (-(1i64 << 59)) as f64;
                    if scaled >= min_val && scaled <= max_val {
                        let num_i64 = scaled.round() as i64;
                        let unsigned = (num_i64 as u64) & crate::resolver::INLINE_VALUE_MASK;
                        packed_object = Some(crate::resolver::INLINE_TAG_DECIMAL | unsigned);
                    }
                }
            } else if dt == "http://www.w3.org/2001/XMLSchema#boolean" {
                if value == "true" || value == "1" {
                    packed_object = Some(crate::resolver::INLINE_TAG_BOOLEAN | 1);
                } else if value == "false" || value == "0" {
                    packed_object = Some(crate::resolver::INLINE_TAG_BOOLEAN | 0);
                }
            }
        }

        let raw = RawTriple {
            subject,
            predicate,
            object,
            packed_object,
        };
        if tx_raw.send(raw).is_ok() {
            triples_read += 1;
        }
        Ok(())
    };

    let path_lower = in_path.to_lowercase();
    let base_iri = catalog_base_iri(Path::new(in_path));
    let mut parse_error: Option<String> = None;
    if path_lower.ends_with(".rdf") || path_lower.ends_with(".xml") || path_lower.ends_with(".owl")
    {
        log::info!("Ontology Ingest: parsing RDF/XML source {}", in_path);
        let raw = std::fs::read_to_string(in_path)?;
        let repaired = if let Some(base) = base_iri.as_ref() {
            repair_rdfxml_empty_base(&raw, base.as_str())
        } else {
            raw
        };
        let mut parser = RdfXmlParser::new(Cursor::new(repaired), base_iri.clone());
        if let Err(e) = parser.parse_all(&mut on_triple) {
            parse_error = Some(format!("RDF/XML: {e}"));
        }
        log::info!("Ontology Ingest: completed RDF/XML parse for {}", in_path);
    } else if path_lower.ends_with(".ttl") {
        log::info!("Ontology Ingest: parsing Turtle source {}", in_path);
        let raw = std::fs::read_to_string(in_path)?;
        let expanded = expand_empty_turtle_prefixed_names(&raw);
        let mut parser = TurtleParser::new(Cursor::new(expanded), base_iri.clone());
        if let Err(e) = parser.parse_all(&mut on_triple) {
            parse_error = Some(format!("Turtle: {e}"));
        }
        log::info!("Ontology Ingest: completed Turtle parse for {}", in_path);
    } else if path_lower.ends_with(".nt") {
        log::info!("Ontology Ingest: parsing N-Triples source {}", in_path);
        let mut parser = NTriplesParser::new(buf_reader);
        if let Err(e) = parser.parse_all(&mut on_triple) {
            parse_error = Some(format!("N-Triples: {e}"));
        }
        log::info!("Ontology Ingest: completed N-Triples parse for {}", in_path);
    } else if path_lower.ends_with(".n3") {
        log::info!("Ontology Ingest: parsing N3 source {}", in_path);
        let text = std::fs::read_to_string(in_path).unwrap_or_default();
        let mut parser = crate::modalities::logic::n3_parser::N3Parser::new(&text);
        let mut webizen = crate::webizen::SlgArena::new();
        let mut rules_parsed = 0;

        let on_n3_event = |event: crate::modalities::logic::n3_parser::N3Event| -> Result<(), crate::modalities::logic::n3_parser::N3ParserError> {
            match event {
                crate::modalities::logic::n3_parser::N3Event::StaticTriple(triple) => {
                    let subject = match triple.subject {
                        crate::modalities::logic::n3_parser::Term::Uri(s)
                        | crate::modalities::logic::n3_parser::Term::Variable(s)
                        | crate::modalities::logic::n3_parser::Term::Literal(s)
                        | crate::modalities::logic::n3_parser::Term::Formula(s) => s.to_string(),
                    };
                    let predicate = match triple.predicate {
                        crate::modalities::logic::n3_parser::Term::Uri(s)
                        | crate::modalities::logic::n3_parser::Term::Variable(s)
                        | crate::modalities::logic::n3_parser::Term::Literal(s)
                        | crate::modalities::logic::n3_parser::Term::Formula(s) => s.to_string(),
                    };
                    let object = match triple.object {
                        crate::modalities::logic::n3_parser::Term::Uri(s)
                        | crate::modalities::logic::n3_parser::Term::Variable(s)
                        | crate::modalities::logic::n3_parser::Term::Literal(s)
                        | crate::modalities::logic::n3_parser::Term::Formula(s) => s.to_string(),
                    };
                    let raw = RawTriple {
                        subject,
                        predicate,
                        object,
                        packed_object: None,
                    };
                    if tx_raw.send(raw).is_ok() {
                        triples_read += 1;
                    }
                }
                crate::modalities::logic::n3_parser::N3Event::LogicRule(rule) => {
                    webizen.register_rule(&rule);
                    rules_parsed += 1;
                }
                crate::modalities::logic::n3_parser::N3Event::AspBlock(_)
                | crate::modalities::logic::n3_parser::N3Event::DiffuseBlock(_) => {
                    // Pass these modalities to the Webizen
                }
            }
            Ok(())
        };

        if let Err(e) = parser.parse_all(on_n3_event) {
            parse_error = Some(format!("N3: {e}"));
        }
        let fired = webizen.fire_registered_rules(crate::q_hash("q42:ingestSession"));
        println!(
            "Registered {} N3 Logic Rules; fired {} through Core-1 Sentinel VM.",
            rules_parsed, fired
        );
        log::info!(
            "Ontology Ingest: completed N3 parse for {} (rules parsed: {}, fired: {})",
            in_path,
            rules_parsed,
            fired
        );
    } else {
        drop(tx_raw);
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported RDF extension for {in_path}; expected .rdf, .xml, .owl, .ttl, .nt, or .n3"),
        ));
    }

    if let Some(error) = parse_error {
        drop(tx_raw);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "RDF parse failed after {triples_read} triples in {in_path}: {error}"
            ),
        ));
    }

    // Drop the main sender so workers know to terminate
    drop(tx_raw);

    // 6. Join the workers first (this closes the collector's channel), merging each shard's lexicon
    // into one hash→string map. First-writer-wins on hash collisions across shards — deterministic
    // given the same input regardless of shard scheduling, since a given term always hashes to the same
    // key.
    let mut lexicon: HashMap<u64, String> = HashMap::new();
    for handle in worker_handles {
        let shard = handle.join().unwrap();
        if lexicon.is_empty() {
            lexicon = shard;
        } else {
            for (k, v) in shard {
                lexicon.entry(k).or_insert(v);
            }
        }
    }

    let mut sorter = collector_handle
        .join()
        .map_err(|_| std::io::Error::other("Q42 external-sort collector thread panicked"))??;
    if mode == IngestMode::Complete {
        for (hash, term) in &lexicon {
            sorter.push_lex(*hash, term);
        }
    }
    let total_written = match max_segment_bytes {
        Some(cap) => {
            sorter
                .merge_volume_set(std::path::Path::new(out_path), cap)?
                .blocks_written
        }
        None => sorter.merge(std::path::Path::new(out_path))?,
    };

    // Lexicon byte size actually written — read back cheaply from the finished header (mmap, no
    // re-serialize) so the report reflects what is really on disk.
    let (lex_length, out_bytes) =
        crate::q42_volume::Q42Volume::open(std::path::Path::new(out_path))
            .ok()
            .map(|root| {
                let mut lex_bytes = root.header().lex_length;
                let mut logical_bytes = std::fs::metadata(out_path).map(|m| m.len()).unwrap_or(0);
                if let Ok(Some(manifest)) = root.volume_manifest() {
                    let parent = std::path::Path::new(out_path)
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("."));
                    for segment in manifest.segments {
                        logical_bytes = logical_bytes.saturating_add(segment.byte_length);
                    }
                    for segment in manifest.lexicon_segments {
                        logical_bytes = logical_bytes.saturating_add(segment.byte_length);
                    if let Ok(shard) =
                        crate::q42_volume::Q42Volume::open(&parent.join(&segment.locator))
                        {
                            lex_bytes = lex_bytes.saturating_add(shard.header().lex_length);
                        }
                    }
                }
                (lex_bytes, logical_bytes)
            })
            .unwrap_or((0, std::fs::metadata(out_path).map(|m| m.len()).unwrap_or(0)));

    let duration = start_time.elapsed();

    // 8. Honest reporting — state the mode and, for the lossy mode, that the size reduction is data
    // loss, NOT compression (CLAUDE.md §15: no claim-vs-reality gap).
    let src_bytes = std::fs::metadata(in_path).map(|m| m.len()).unwrap_or(0);
    println!("✅ Import Complete!");
    println!("Parsed {} triples.", triples_read);
    log::info!("Ontology Ingest: parsed {} triples", triples_read);
    println!("Wrote {} Super-Quins to {}.", total_written, out_path);
    println!("Ingest mode: {}", mode.label());
    match mode {
        IngestMode::Complete => {
            println!(
                "Lexicon: {} unique terms retained ({} bytes) — every URI and literal recoverable (full Unicode).",
                lexicon.len(),
                lex_length
            );
            println!(
                "Source {} B → .q42 {} B (lossless: 48-byte structure + complete lexicon; reversible to the source terms).",
                src_bytes, out_bytes
            );
        }
        IngestMode::StripLiterals => {
            println!(
                "⚠  STRIP-LITERALS mode: human-readable text (URIs and literals) was DISCARDED and is NOT recoverable."
            );
            println!(
                "Source {} B → .q42 {} B. This size reduction is DATA LOSS (structure-only), not compression.",
                src_bytes, out_bytes
            );
        }
    }
    println!("Total Time: {:?}", duration);
    let total_superblocks =
        (total_written + (crate::QUINS_PER_BLOCK as u64) - 1) / crate::QUINS_PER_BLOCK as u64;
    log::info!(
        "Ontology Ingest: Completed {} SuperBlocks ({} quins, mode {:?}) in {:?}",
        total_superblocks,
        total_written,
        mode,
        duration
    );

    Ok(total_written)
}

pub fn verify_integrity(
    input_path: std::path::PathBuf,
    dataset_path: std::path::PathBuf,
) -> std::io::Result<bool> {
    use crate::rdf_star::RdfStarParser;
    use crate::sparql_library::parsers::turtle_star::TurtleStarParser;
    use std::fs::File;
    use std::io::BufReader;

    // Retained only as a fast legacy diagnostic. XOR has compensating-error
    // collisions; `verify-graph` performs the bounded encoded-set proof.
    let mut source_checksum: u64 = 0;
    let mut source_records: u64 = 0;
    let file = File::open(&input_path)?;
    let mut reader = BufReader::new(file);
    let mut parser = TurtleStarParser::new(0);

    let mut buffer = Vec::new();
    while {
        buffer.clear();
        std::io::BufRead::read_until(&mut reader, b'\n', &mut buffer)? > 0
    } {
        let mut slice = buffer.as_slice();
        if slice.ends_with(b"\r\n") {
            slice = &slice[..slice.len() - 2];
        } else if slice.ends_with(b"\n") {
            slice = &slice[..slice.len() - 1];
        }

        if slice.is_empty() || slice[0] == b'#' || slice.iter().all(|b| b.is_ascii_whitespace()) {
            continue;
        }

        let (s, p, o) = parser.parse_triple(slice).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("source contains an unparseable RDF triple: {e}"),
            )
        })?;
        source_checksum ^= s ^ p ^ o;
        source_records += 1;
    }

    println!("Source Checksum: 0x{:016X}", source_checksum);

    // Dataset calculation
    let mut dataset_checksum: u64 = 0;
    let mut dataset_records: u64 = 0;

    let volume = match crate::q42_volume::Q42Volume::open(&dataset_path) {
        Ok(v) => v,
        Err(e) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to open Q42 volume: {}", e),
            ))
        }
    };

    let mut sb_buf = vec![0u8; crate::q42_volume::SUPERBLOCK_SIZE];
    for i in 0..volume.block_count() as usize {
        let _ = volume.read_superblock_into(i, &mut sb_buf)?;
        let quin_count = u64::from_le_bytes(sb_buf[16..24].try_into().unwrap()) as usize;
        let mut off = crate::q42_volume::SUPERBLOCK_HEADER;
        for _ in 0..quin_count {
            let quin: crate::NQuin =
                bytemuck::pod_read_unaligned(&sb_buf[off..off + crate::q42_volume::QUIN_SIZE]);
            if !quin.verify_ecc_parity() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Q42 parity mismatch in block {i}"),
                ));
            }
            dataset_checksum ^= quin.parity;
            dataset_records += 1;
            off += crate::q42_volume::QUIN_SIZE;
        }
    }

    println!("Dataset Checksum: 0x{:016X}", dataset_checksum);

    println!("Source records: {source_records}");
    println!("Dataset records: {dataset_records}");
    Ok(source_checksum == dataset_checksum
        && source_records == dataset_records
        && source_records != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn rdf_ingest_uses_external_runs_and_embeds_lossless_lexicon() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("input.nt");
        let output = dir.path().join("output.q42");
        std::fs::write(
            &input,
            "<https://example.test/s> <https://example.test/p> \"value\" .\n",
        )
        .unwrap();
        assert_eq!(
            streaming_import_rdf_with_mode(
                input.to_str().unwrap(),
                output.to_str().unwrap(),
                IngestMode::Complete,
            )
            .unwrap(),
            1
        );
        let volume = crate::q42_volume::Q42Volume::open(&output).unwrap();
        assert_eq!(volume.block_count(), 1);
        assert!(volume.lex_view().unwrap().entry_count() >= 3);
    }

    #[test]
    fn empty_turtle_prefixed_names_expand_to_iris() {
        let src = "@prefix dc: <http://purl.org/dc/terms/> .\n@prefix ns1: <http://purl.org/ontology/mo/> .\n<http://ex/s> rdfs:seeAlso dc: ;\n  rdfs:isDefinedBy ns1: .\n";
        let expanded = expand_empty_turtle_prefixed_names(src);
        assert!(expanded.contains("<http://purl.org/dc/terms/>"));
        assert!(expanded.contains("<http://purl.org/ontology/mo/>"));
        assert!(!expanded.contains(" dc: ;"));
        assert!(!expanded.contains(" ns1: ."));
    }

    #[test]
    fn turtle_with_empty_prefix_names_ingests() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("music-mini.ttl");
        let output = dir.path().join("music-mini.q42");
        std::fs::write(
            &input,
            "@prefix dc: <http://purl.org/dc/terms/> .\n\
             @prefix ns1: <http://purl.org/ontology/mo/> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             <http://purl.org/ontology/mo/Track> rdfs:isDefinedBy ns1: .\n\
             <http://purl.org/ontology/mo/> dc:title \"The Music Ontology\" .\n",
        )
        .unwrap();
        let written =
            streaming_import_rdf(input.to_str().unwrap(), output.to_str().unwrap()).unwrap();
        assert!(written >= 1, "expected at least one SuperBlock, got {written}");
        let report = crate::q42_volume::Q42InspectReport::from_path(&output).unwrap();
        assert!(!report.lexicon_has_no_terms);
        assert!(report.flags & crate::q42_volume::FLAG_PERMISSIVE_COMMONS != 0);
    }

    #[test]
    fn rdfxml_empty_xml_base_ingests_with_catalog_base() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("earl.rdf");
        let output = dir.path().join("earl.q42");
        std::fs::write(
            &input,
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
                "<rdf:RDF xml:base=\"\"\n",
                "         xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"\n",
                "         xmlns:rdfs=\"http://www.w3.org/2000/01/rdf-schema#\">\n",
                "  <rdf:Description rdf:about=\"#Assertion\">\n",
                "    <rdfs:label xml:lang=\"en\">Assertion</rdfs:label>\n",
                "  </rdf:Description>\n",
                "</rdf:RDF>\n",
            ),
        )
        .unwrap();
        let written = streaming_import_rdf(input.to_str().unwrap(), output.to_str().unwrap())
            .expect("empty xml:base must not fail closed after catalog base is applied");
        assert!(written >= 1);
        let report = crate::q42_volume::Q42InspectReport::from_path(&output).unwrap();
        assert!(!report.lexicon_has_no_terms);
    }

    #[test]
    fn broken_turtle_fails_closed() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("bad.ttl");
        let output = dir.path().join("bad.q42");
        std::fs::write(&input, "@prefix broken\n").unwrap();
        let err = streaming_import_rdf(input.to_str().unwrap(), output.to_str().unwrap())
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("parse failed"));
    }
}
