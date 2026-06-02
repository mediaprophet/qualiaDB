use crate::QualiaQuin;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;
use rio_api::parser::TriplesParser;
use rio_xml::RdfXmlParser;
use rio_turtle::{TurtleParser, NTriplesParser};
use std::sync::Arc;
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

pub fn streaming_import_rdf(in_path: &str, out_path: &str) -> std::io::Result<()> {
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
    for worker_id in 0..target_workers {
        let rx = rx_raw.clone();
        let tx = tx_bin.clone();
        
        let handle = thread::spawn(move || {
            let mut local_count = 0;
            for triple in rx {
                // Cryptographic Hashing Simulation (Ed25519 / Blake3 mapping)
                // In a production system, this hashes the string to a strict 64-bit ID.
                // We'll use a simple deterministic hash for the benchmark.
                let s_hash = deterministic_hash(&triple.subject);
                let p_hash = deterministic_hash(&triple.predicate);
                let o_hash = deterministic_hash(&triple.object);
                
                let quin = QualiaQuin {
                    subject: s_hash,
                    predicate: p_hash,
                    object: o_hash,
                    context: 0,
                    metadata: 0,
                    parity: 0,
                };
                
                // Send back to the writer thread
                if tx.send(quin).is_err() {
                    break;
                }
                local_count += 1;
            }
            // println!("Shard {} completed: processed {} quins.", worker_id, local_count);
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
                out_file.write_all(&(compressed.len() as u32).to_le_bytes()).unwrap();
                out_file.write_all(&(buffer.len() as u32).to_le_bytes()).unwrap();
                // Write payload
                out_file.write_all(&compressed).unwrap();
                
                buffer.clear();
                block_id += 1;
            }
        }
        
        // Flush remaining
        if !buffer.is_empty() {
            let compressed = lz4_flex::compress_prepend_size(&buffer);
            out_file.write_all(&block_id.to_le_bytes()).unwrap();
            out_file.write_all(&(compressed.len() as u32).to_le_bytes()).unwrap();
            out_file.write_all(&(buffer.len() as u32).to_le_bytes()).unwrap();
            out_file.write_all(&compressed).unwrap();
        }
        
        written_count
    });

    // 5. The Streaming Sieve (Main Thread)
    // Uses Rio to read the file sequentially without loading the whole graph into RAM.
    let in_file = File::open(&in_path)?;
    let buf_reader = BufReader::new(in_file);

    let mut triples_read = 0;
    
    // Setup a callback that parses Rio triples and sends them to the worker queue
    let mut on_triple = |t: rio_api::model::Triple| -> Result<(), std::io::Error> {
        let subject = t.subject.to_string();
        let predicate = t.predicate.to_string();
        let object = t.object.to_string();
        
        let raw = RawTriple { subject, predicate, object };
        if tx_raw.send(raw).is_ok() {
            triples_read += 1;
        }
        Ok(())
    };

    let path_lower = in_path.to_lowercase();
    if path_lower.ends_with(".rdf") || path_lower.ends_with(".xml") {
        let mut parser = RdfXmlParser::new(buf_reader, None);
        if let Err(e) = parser.parse_all(&mut on_triple) {
            eprintln!("RDF/XML Parsing Error: {}", e);
        }
    } else if path_lower.ends_with(".ttl") {
        let mut parser = TurtleParser::new(buf_reader, None);
        if let Err(e) = parser.parse_all(&mut on_triple) {
            eprintln!("Turtle Parsing Error: {}", e);
        }
    } else if path_lower.ends_with(".nt") {
        let mut parser = NTriplesParser::new(buf_reader);
        if let Err(e) = parser.parse_all(&mut on_triple) {
            eprintln!("N-Triples Parsing Error: {}", e);
        }
    } else if path_lower.ends_with(".n3") {
        let mut parser = crate::n3_parser::N3Parser::new(buf_reader);
        let mut sentinel = crate::sentinel::SlgArena::new();
        let mut rules_parsed = 0;
        
        let on_n3_event = |event: crate::n3_parser::N3Event| -> Result<(), std::io::Error> {
            match event {
                crate::n3_parser::N3Event::StaticTriple(triple) => {
                    let subject = match triple.subject {
                        crate::n3_parser::Term::Uri(s) | crate::n3_parser::Term::Variable(s) | crate::n3_parser::Term::Literal(s) => s,
                    };
                    let predicate = match triple.predicate {
                        crate::n3_parser::Term::Uri(s) | crate::n3_parser::Term::Variable(s) | crate::n3_parser::Term::Literal(s) => s,
                    };
                    let object = match triple.object {
                        crate::n3_parser::Term::Uri(s) | crate::n3_parser::Term::Variable(s) | crate::n3_parser::Term::Literal(s) => s,
                    };
                    let raw = RawTriple { subject, predicate, object };
                    if tx_raw.send(raw).is_ok() {
                        triples_read += 1;
                    }
                }
                crate::n3_parser::N3Event::LogicRule(rule) => {
                    sentinel.register_rule(rule);
                    rules_parsed += 1;
                }
                crate::n3_parser::N3Event::AspBlock(_) | crate::n3_parser::N3Event::DiffuseBlock(_) => {
                    // Pass these modalities to the Sentinel
                }
            }
            Ok(())
        };

        if let Err(e) = parser.parse_all(on_n3_event) {
            eprintln!("N3 Logic Parsing Error: {}", e);
        }
        println!("Registered {} N3 Logic Rules into the Sentinel VM.", rules_parsed);
    } else {
        eprintln!("Unsupported file extension. Expected .rdf, .xml, .ttl, .nt, or .n3");
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
    println!("Wrote {} Super-Quins to {}.", total_written, out_path);
    println!("Total Time: {:?}", duration);

    Ok(())
}

fn deterministic_hash(input: &str) -> u64 {
    // Simple FNV-1a hash for deterministic benchmark ID generation
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
