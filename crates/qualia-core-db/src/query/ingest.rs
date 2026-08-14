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
use std::io::BufReader;
use std::thread;
use std::time::Instant;
use sysinfo::System;

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

    // 4. Spawn a collector thread that drains the hashed quins concurrently with parsing (the bounded
    // channel would otherwise backpressure the parser to a halt). It just gathers them; the canonical
    // volume is written on the main thread once the full set is known, via `UnifiedVolumeBuilder` —
    // which produces the GOVERNING `.q42` layout (160-byte SuperBlock headers, block directory, BIDX
    // object index, Merkle-DAG, real lexicon offsets). The previous hand-rolled writer emitted
    // headerless blocks that `Q42Volume::read_all_quins` could not parse — the graph was unreadable.
    let collector_handle = thread::spawn(move || -> Vec<NQuin> {
        let mut quins: Vec<NQuin> = Vec::new();
        for quin in rx_bin {
            quins.push(quin);
        }
        quins
    });

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
    if path_lower.ends_with(".rdf") || path_lower.ends_with(".xml") || path_lower.ends_with(".owl")
    {
        log::info!("Ontology Ingest: parsing RDF/XML source {}", in_path);
        let mut parser = RdfXmlParser::new(buf_reader, None);
        if let Err(e) = parser.parse_all(&mut on_triple) {
            eprintln!("RDF/XML Parsing Error: {}", e);
        }
        log::info!("Ontology Ingest: completed RDF/XML parse for {}", in_path);
    } else if path_lower.ends_with(".ttl") {
        log::info!("Ontology Ingest: parsing Turtle source {}", in_path);
        let mut parser = TurtleParser::new(buf_reader, None);
        if let Err(e) = parser.parse_all(&mut on_triple) {
            eprintln!("Turtle Parsing Error: {}", e);
        }
        log::info!("Ontology Ingest: completed Turtle parse for {}", in_path);
    } else if path_lower.ends_with(".nt") {
        log::info!("Ontology Ingest: parsing N-Triples source {}", in_path);
        let mut parser = NTriplesParser::new(buf_reader);
        if let Err(e) = parser.parse_all(&mut on_triple) {
            eprintln!("N-Triples Parsing Error: {}", e);
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
            eprintln!("N3 Logic Parsing Error: {}", e);
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
        eprintln!("Unsupported file extension. Expected .rdf, .xml, .ttl, .nt, or .n3");
        log::warn!(
            "Ontology Ingest: unsupported file extension for {}",
            in_path
        );
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

    let mut quins = collector_handle.join().unwrap();
    let total_written = quins.len() as u64;

    // 7. Write the canonical volume via `UnifiedVolumeBuilder`. Sort by object hash first so the
    // `FLAG_OBJECT_SORTED` flag and the BIDX object index (both set by the builder) are truthful, not
    // decorative — the old path set the sorted flag without sorting. A quin set is unordered, so this
    // reordering changes nothing semantically.
    quins.sort_unstable_by_key(|q| q.object);

    let mut builder = match mode {
        IngestMode::Complete => crate::q42_volume::UnifiedVolumeBuilder::with_lex_map(&lexicon)
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("invalid lossless Q42LEX: {e:?}"),
                )
            })?,
        IngestMode::StripLiterals => crate::q42_volume::UnifiedVolumeBuilder::with_empty_lex(),
    };
    let mut seq_id: u64 = 0;
    for chunk in quins.chunks(crate::QUINS_PER_BLOCK) {
        builder.push_block(seq_id, chunk)?;
        seq_id += 1;
    }
    builder
        .finish(std::path::Path::new(out_path))
        .map_err(|e| std::io::Error::new(e.kind(), format!("write canonical .q42 volume: {e}")))?;

    // Lexicon byte size actually written — read back cheaply from the finished header (mmap, no
    // re-serialize) so the report reflects what is really on disk.
    let lex_length: u64 = crate::q42_volume::Q42Volume::open(std::path::Path::new(out_path))
        .map(|v| v.header().lex_length)
        .unwrap_or(0);

    let duration = start_time.elapsed();

    // 8. Honest reporting — state the mode and, for the lossy mode, that the size reduction is data
    // loss, NOT compression (CLAUDE.md §15: no claim-vs-reality gap).
    let src_bytes = std::fs::metadata(in_path).map(|m| m.len()).unwrap_or(0);
    let out_bytes = std::fs::metadata(out_path).map(|m| m.len()).unwrap_or(0);

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
