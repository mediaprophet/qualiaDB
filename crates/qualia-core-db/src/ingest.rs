//! Legacy streaming import path for RDF text sources.
//!
//! Important: this function currently writes framed LZ4 blocks directly to the
//! output path. That behavior predates the canonical split between raw `.q42`
//! SuperBlock containers and `.c.q42` transport artifacts, and should now be
//! treated as a migration-era compatibility format rather than the governing
//! raw `.q42` layout.

use crate::{q_hash, QualiaQuin};
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
    let (tx_bin, rx_bin): (Sender<QualiaQuin>, Receiver<QualiaQuin>) = bounded(10_000);

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

                let quin = QualiaQuin {
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
        let mut out_file = File::create(out_path_copy).expect("Failed to create output .q42 file");
        let mut written_count = 0;
        let mut block_id: u64 = 0;
        let mut buffer = Vec::with_capacity(393_216);

        for quin in rx_bin {
            let bytes = bytemuck::bytes_of(&quin);
            buffer.extend_from_slice(bytes);
            written_count += 1;

            if buffer.len() >= 393_216 {
                let compressed = lz4_flex::compress_prepend_size(&buffer);
                // Write Header: block_id (8), compressed_len (4), uncompressed_len (4)
                out_file.write_all(&block_id.to_le_bytes()).unwrap();
                out_file
                    .write_all(&(compressed.len() as u32).to_le_bytes())
                    .unwrap();
                out_file
                    .write_all(&(buffer.len() as u32).to_le_bytes())
                    .unwrap();
                // Write payload
                out_file.write_all(&compressed).unwrap();

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
            out_file.write_all(&block_id.to_le_bytes()).unwrap();
            out_file
                .write_all(&(compressed.len() as u32).to_le_bytes())
                .unwrap();
            out_file
                .write_all(&(buffer.len() as u32).to_le_bytes())
                .unwrap();
            out_file.write_all(&compressed).unwrap();
            log::info!(
                "Ontology Ingest: wrote SuperBlock #{} (final block, {} quins total)",
                block_id + 1,
                written_count
            );
        }

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
