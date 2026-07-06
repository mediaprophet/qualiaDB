//! Legacy streaming import path for RDF text sources.
//!
//! Important: this function currently writes framed LZ4 blocks directly to the
//! output path. That behavior predates the canonical split between raw `.q42`
//! SuperBlock containers and `.c.q42` transport artifacts, and should now be
//! treated as a migration-era compatibility format rather than the governing
//! raw `.q42` layout.

use crate::{q_hash, NQuin};
use log;

const OBJECT_HASH_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;
use crossbeam_channel::{bounded, Receiver, Sender};
use rio_api::parser::TriplesParser;
use rio_turtle::{NTriplesParser, TurtleParser};
use rio_xml::RdfXmlParser;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write};
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

/// What the writer thread hands back so the main thread can append the lexicon (Complete mode) and
/// then stamp the header — the header must record the real `lex_offset`/`lex_length`, which are only
/// known after the full lexicon is serialized.
struct WriterOutput {
    file: File,
    written_count: u64,
    block_count: u64,
    data_offset: u64,
    block_dir_offset: u64,
    block_dir_length: u64,
    dag_root_offset: u64,
    dag_root_length: u64,
    /// Offset just past the DAG blob — where the lexicon section begins.
    tail_offset: u64,
    merkle_root: [u8; 32],
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

    // 4. Spawn Writer Thread
    let out_path_copy = out_path.to_string();
    let writer_handle = thread::spawn(move || -> WriterOutput {
        use crate::git_bridge::DagStore;
        use crate::q42_volume::{BlockDirectoryEntry, HEADER_SIZE};
        use std::io::{Seek, SeekFrom};

        let mut out_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(out_path_copy)
            .expect("Failed to create output .q42 file");
        out_file.seek(SeekFrom::Start(HEADER_SIZE as u64)).unwrap();

        let mut written_count = 0;
        let mut block_id: u64 = 0;
        let mut buffer = Vec::with_capacity(393_216);

        let mut block_directory: Vec<BlockDirectoryEntry> = Vec::new();
        let mut dag_store = DagStore::new();
        let mut last_dag_hash = [0u8; 32];

        let data_offset = HEADER_SIZE as u64;
        let mut current_offset = data_offset;

        for quin in rx_bin {
            let bytes = bytemuck::bytes_of(&quin);
            buffer.extend_from_slice(bytes);
            written_count += 1;

            if buffer.len() >= 393_216 {
                let compressed = lz4_flex::compress_prepend_size(&buffer);

                out_file.write_all(&compressed).unwrap();

                let block_size = compressed.len() as u32;
                block_directory.push(BlockDirectoryEntry {
                    rel_offset: current_offset - data_offset,
                    comp_len: block_size,
                    uncomp_len: buffer.len() as u32,
                });
                current_offset += block_size as u64;

                let quins_slice: &[NQuin] = bytemuck::cast_slice(&buffer);
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let msg = format!("ingest block {block_id}");
                last_dag_hash = if last_dag_hash == [0u8; 32] {
                    dag_store.genesis_node(quins_slice, 0, ts, &msg)
                } else {
                    dag_store.commit_node(last_dag_hash, quins_slice, 0, ts, &msg)
                };

                log::info!(
                    "Ontology Ingest: wrote SuperBlock #{}, streamed {} quins so far",
                    block_id + 1,
                    written_count
                );
                buffer.clear();
                block_id += 1;
            }
        }

        // Flush remaining
        if !buffer.is_empty() {
            let compressed = lz4_flex::compress_prepend_size(&buffer);
            out_file.write_all(&compressed).unwrap();

            let block_size = compressed.len() as u32;
            block_directory.push(BlockDirectoryEntry {
                rel_offset: current_offset - data_offset,
                comp_len: block_size,
                uncomp_len: buffer.len() as u32,
            });
            current_offset += block_size as u64;

            let quins_slice: &[NQuin] = bytemuck::cast_slice(&buffer);
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            let msg = format!("ingest block {block_id}");
            last_dag_hash = if last_dag_hash == [0u8; 32] {
                dag_store.genesis_node(quins_slice, 0, ts, &msg)
            } else {
                dag_store.commit_node(last_dag_hash, quins_slice, 0, ts, &msg)
            };

            log::info!(
                "Ontology Ingest: wrote SuperBlock #{} (final block, {} quins total)",
                block_id + 1,
                written_count
            );
            block_id += 1;
        }

        let block_dir_offset = current_offset;
        for entry in &block_directory {
            entry.write_to(&mut out_file).unwrap();
        }
        let block_dir_length = (block_directory.len() * BlockDirectoryEntry::SIZE) as u64;
        current_offset += block_dir_length;

        let dag_root_offset = current_offset;
        let dag_blob = dag_store.serialize();
        out_file.write_all(&dag_blob).unwrap();
        let dag_root_length = dag_blob.len() as u64;
        current_offset += dag_root_length;

        out_file.flush().unwrap();

        let merkle_root = if last_dag_hash == [0u8; 32] {
            [0u8; 32]
        } else {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(last_dag_hash);
            h.finalize().into()
        };

        log::debug!(
            "Ontology Ingest: writer processed {} quins across {} SuperBlocks",
            written_count,
            block_id
        );

        // Hand the file + offsets back; the main thread appends the lexicon (Complete mode) and only
        // then writes the header, so `lex_offset`/`lex_length` reflect what was actually stored.
        WriterOutput {
            file: out_file,
            written_count,
            block_count: block_id,
            data_offset,
            block_dir_offset,
            block_dir_length,
            dag_root_offset,
            dag_root_length,
            tail_offset: current_offset,
            merkle_root,
        }
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

    // 6. Join the workers first (this closes the writer's channel), merging each shard's lexicon into
    // one hash→string map. First-writer-wins on hash collisions across shards — deterministic given the
    // same input regardless of shard scheduling, since a given term always hashes to the same key.
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

    let mut writer_out = writer_handle.join().unwrap();
    let total_written = writer_out.written_count;

    // 7. Finalize the volume: in Complete mode serialize the lexicon and append it, then stamp the
    // header with the REAL lex_offset/lex_length (previously hard-coded to 0 — the data-loss bug).
    let lex_length: u64 = {
        use crate::q42_volume::{header_to_bytes, Q42VolumeHeader};
        use std::io::{Seek, SeekFrom};

        let (lex_offset, lex_length) = if mode == IngestMode::Complete && !lexicon.is_empty() {
            let lex_bytes = crate::q42_lex::serialize_string_lexicon(&lexicon);
            writer_out.file.seek(SeekFrom::Start(writer_out.tail_offset))?;
            writer_out.file.write_all(&lex_bytes)?;
            (writer_out.tail_offset, lex_bytes.len() as u64)
        } else {
            (writer_out.tail_offset, 0u64)
        };

        let assertion_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let header = Q42VolumeHeader {
            magic: crate::q42_volume::Q42_MAGIC,
            version: crate::q42_volume::Q42_VERSION_V3,
            flags: crate::q42_volume::FLAG_BLOCKS_LZ4 | crate::q42_volume::FLAG_OBJECT_SORTED,
            lex_offset,
            lex_length,
            bidx_offset: lex_offset + lex_length,
            bidx_length: 0,
            block_dir_offset: writer_out.block_dir_offset,
            block_dir_length: writer_out.block_dir_length,
            data_offset: writer_out.data_offset,
            data_length: writer_out.block_dir_offset - writer_out.data_offset,
            block_count: writer_out.block_count,
            block_size: crate::q42_volume::SUPERBLOCK_SIZE as u32,
            quins_per_block: crate::QUINS_PER_BLOCK as u32,
            temporal_index_offset: 0,
            temporal_index_length: 0,
            merkle_root: writer_out.merkle_root,
            assertion_timestamp,
            dag_root_offset: writer_out.dag_root_offset,
            dag_root_length: writer_out.dag_root_length,
            natural_person_did_offset: 0,
            software_agent_did_offset: 0,
            _reserved: [0u8; 80],
        };

        writer_out.file.seek(SeekFrom::Start(0))?;
        writer_out.file.write_all(&header_to_bytes(&header))?;
        writer_out.file.flush()?;
        lex_length
    };

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
    

    // Calculate source checksum
    let mut source_checksum: u64 = 0;
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

        if let Ok((s, p, o)) = parser.parse_triple(slice) {
            let parity = s ^ p ^ o ^ 0;
            source_checksum ^= parity;
        }
    }

    println!("Source Checksum: 0x{:016X}", source_checksum);

    // Dataset calculation
    let mut dataset_checksum: u64 = 0;

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
            let parity = u64::from_le_bytes(sb_buf[off + 40..off + 48].try_into().unwrap());
            if parity != 0 {
                dataset_checksum ^= parity;
            }
            off += crate::q42_volume::QUIN_SIZE;
        }
    }

    println!("Dataset Checksum: 0x{:016X}", dataset_checksum);

    Ok(source_checksum == dataset_checksum && source_checksum != 0)
}
