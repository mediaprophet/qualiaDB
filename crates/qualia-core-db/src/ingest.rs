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
use std::fs::File;
use std::io::{BufReader, Write};
use std::thread;
use std::time::Instant;
use sysinfo::System;

/// Represents a raw string-based Triple extracted from RDF/XML
#[derive(Debug)]
pub struct RawTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

pub fn streaming_import_rdf(in_path: &str, out_path: &str) -> std::io::Result<u64> {
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
    let mut worker_handles = vec![];
    for _worker_id in 0..target_workers {
        let rx = rx_raw.clone();
        let tx = tx_bin.clone();

        let handle = thread::spawn(move || {
            let mut _local_count = 0u64;
            for triple in rx {
                let s_hash = q_hash(&triple.subject);
                let p_hash = q_hash(&triple.predicate);
                let o_hash = q_hash(&triple.object) & OBJECT_HASH_MASK;
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

                // Send back to the writer thread
                if tx.send(quin).is_err() {
                    break;
                }
                _local_count += 1;
            }
            if _local_count > 0 {
                log::debug!(
                    "Ontology Ingest: worker shard finished {} triples",
                    _local_count
                );
            }
        });
        worker_handles.push(handle);
    }

    // Drop the extra transmitters so channels close correctly
    drop(tx_bin);

    // 4. Spawn Writer Thread
    let out_path_copy = out_path.to_string();
    let writer_handle = thread::spawn(move || {
        use std::io::{Seek, SeekFrom};
        use crate::q42_volume::{Q42VolumeHeader, BlockDirectoryEntry, HEADER_SIZE, header_to_bytes};
        use crate::git_bridge::DagStore;

        let mut out_file = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(out_path_copy).expect("Failed to create output .q42 file");
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

        let empty_section_offset = current_offset;
        out_file.flush().unwrap();

        let merkle_root = if last_dag_hash == [0u8; 32] {
            [0u8; 32]
        } else {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(last_dag_hash);
            h.finalize().into()
        };

        let assertion_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let header = Q42VolumeHeader {
            magic: crate::q42_volume::Q42_MAGIC,
            version: crate::q42_volume::Q42_VERSION_V3,
            flags: crate::q42_volume::FLAG_BLOCKS_LZ4 | crate::q42_volume::FLAG_OBJECT_SORTED,
            lex_offset: empty_section_offset,
            lex_length: 0,
            bidx_offset: empty_section_offset,
            bidx_length: 0,
            block_dir_offset,
            block_dir_length,
            data_offset,
            data_length: block_dir_offset - data_offset,
            block_count: block_id,
            block_size: crate::q42_volume::SUPERBLOCK_SIZE as u32,
            quins_per_block: crate::QUINS_PER_BLOCK as u32,
            temporal_index_offset: 0,
            temporal_index_length: 0,
            merkle_root,
            assertion_timestamp,
            dag_root_offset,
            dag_root_length,
            _reserved: [0u8; 96],
        };

        out_file.seek(SeekFrom::Start(0)).unwrap();
        out_file.write_all(&header_to_bytes(&header)).unwrap();
        out_file.flush().unwrap();

        log::debug!(
            "Ontology Ingest: writer processed {} quins across {} SuperBlocks",
            written_count,
            block_id
        );
        written_count
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

        let raw = RawTriple {
            subject,
            predicate,
            object,
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
        let mut parser = crate::modalities::logic::n3_parser::N3Parser::new(buf_reader);
        let mut webizen = crate::webizen::SlgArena::new();
        let mut rules_parsed = 0;

        let on_n3_event = |event: crate::modalities::logic::n3_parser::N3Event| -> Result<(), std::io::Error> {
            match event {
                crate::modalities::logic::n3_parser::N3Event::StaticTriple(triple) => {
                    let subject = match triple.subject {
                        crate::modalities::logic::n3_parser::Term::Uri(s)
                        | crate::modalities::logic::n3_parser::Term::Variable(s)
                        | crate::modalities::logic::n3_parser::Term::Literal(s) => s,
                    };
                    let predicate = match triple.predicate {
                        crate::modalities::logic::n3_parser::Term::Uri(s)
                        | crate::modalities::logic::n3_parser::Term::Variable(s)
                        | crate::modalities::logic::n3_parser::Term::Literal(s) => s,
                    };
                    let object = match triple.object {
                        crate::modalities::logic::n3_parser::Term::Uri(s)
                        | crate::modalities::logic::n3_parser::Term::Variable(s)
                        | crate::modalities::logic::n3_parser::Term::Literal(s) => s,
                    };
                    let raw = RawTriple {
                        subject,
                        predicate,
                        object,
                    };
                    if tx_raw.send(raw).is_ok() {
                        triples_read += 1;
                    }
                }
                crate::modalities::logic::n3_parser::N3Event::LogicRule(rule) => {
                    webizen.register_rule(rule);
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

    // 6. Wait for all threads to finish
    for handle in worker_handles {
        handle.join().unwrap();
    }

    let total_written = writer_handle.join().unwrap();
    let duration = start_time.elapsed();

    println!("✅ Import Complete!");
    println!("Parsed {} triples.", triples_read);
    log::info!("Ontology Ingest: parsed {} triples", triples_read);
    println!("Wrote {} Super-Quins to {}.", total_written, out_path);
    println!("Total Time: {:?}", duration);
    let total_superblocks =
        (total_written + (crate::QUINS_PER_BLOCK as u64) - 1) / crate::QUINS_PER_BLOCK as u64;
    log::info!(
        "Ontology Ingest: Completed {} SuperBlocks ({} quins) in {:?}",
        total_superblocks,
        total_written,
        duration
    );

    Ok(total_written)
}

pub fn verify_integrity(input_path: std::path::PathBuf, dataset_path: std::path::PathBuf) -> std::io::Result<bool> {
    use std::io::Read;
    use std::fs::File;
    use std::io::BufReader;
    use crate::sparql_library::parsers::turtle_star::TurtleStarParser;
    use crate::rdf_star::RdfStarParser;
    
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
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Failed to open Q42 volume: {}", e))),
    };
    
    let mut sb_buf = vec![0u8; crate::q42_volume::SUPERBLOCK_SIZE];
    for i in 0..volume.block_count() as usize {
        let _ = volume.read_superblock_into(i, &mut sb_buf)?;
        let quin_count = u64::from_le_bytes(sb_buf[16..24].try_into().unwrap()) as usize;
        let mut off = crate::q42_volume::SUPERBLOCK_HEADER;
        for _ in 0..quin_count {
            let parity = u64::from_le_bytes(sb_buf[off+40..off+48].try_into().unwrap());
            if parity != 0 {
                dataset_checksum ^= parity;
            }
            off += crate::q42_volume::QUIN_SIZE;
        }
    }
    
    println!("Dataset Checksum: 0x{:016X}", dataset_checksum);
    
    Ok(source_checksum == dataset_checksum && source_checksum != 0)
}
