//! Comparative harness helper: load Qualia dataset artifacts and benchmark point / two-hop / filter.
//!
//! Supports the three Schema.org profile inputs:
//!   - N-Triples (same RDF text as Oxigraph / Comunica / WASM-Prolog)
//!   - `.q42` SuperBlock artifact (qualia-cli ingest output)
//!   - `.c.q42` LZ4 distribution artifact (browser / WebTorrent deploy)
//!
//! Usage:
//!   q42_comparative_bench --input path.nt --format ntriples --engine qualia_nt --queries-json '{...}'
//!   q42_comparative_bench --input path.q42 --format superblock --engine qualia_q42 --queries-json '{...}'
//!   q42_comparative_bench --input path.c.q42 --format cq42 --engine qualia_cq42 --queries-json '{...}'
//!
//! `--q42` is a legacy alias for `--input` with `--format superblock`.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Instant;

use qualia_core_db::mini_parser::hash_token;
use qualia_core_db::q42_reader::read_c_q42_quins;
use qualia_core_db::{q_hash, QualiaQuin, QUINS_PER_BLOCK};
use sysinfo::System;

const BLOCK_SIZE: u64 = 40_960;
const HEADER_SIZE: usize = 160;
const QUIN_SIZE: usize = 48;
const OBJECT_HASH_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputFormat {
    Superblock,
    Cq42,
    Ntriples,
}

const WARMUP: usize = 10;
const SAMPLES: usize = 30;

#[derive(serde::Deserialize)]
struct QuerySpec {
    point_subject: String,
    twohop_start: String,
    filter_predicate: String,
}

/// Read `.q42` files produced by `qualia-cli ingest` (160-byte SuperBlock headers).
fn read_cli_superblock_quins(path: &Path) -> std::io::Result<Vec<QualiaQuin>> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len();
    let mut quins = Vec::new();
    let mut offset = 0u64;

    while offset + BLOCK_SIZE <= file_len {
        let mut header = [0u8; HEADER_SIZE];
        file.read_exact(&mut header)?;
        let active = u64::from_le_bytes(header[16..24].try_into().unwrap()) as usize;

        let mut ledger = vec![0u8; QUINS_PER_BLOCK * QUIN_SIZE];
        file.read_exact(&mut ledger)?;

        let count = active.min(QUINS_PER_BLOCK);
        for i in 0..count {
            let off = i * QUIN_SIZE;
            let quin: QualiaQuin = unsafe {
                std::ptr::read_unaligned(ledger[off..off + QUIN_SIZE].as_ptr() as *const QualiaQuin)
            };
            quins.push(quin);
        }
        offset += BLOCK_SIZE;
    }

    Ok(quins)
}

fn read_ntriples_quins(path: &Path) -> std::io::Result<Vec<QualiaQuin>> {
    let reader = BufReader::new(fs::File::open(path)?);
    let mut quins = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut tokens = line.split_ascii_whitespace();
        let Some(s) = tokens.next() else { continue };
        let Some(p) = tokens.next() else { continue };
        let Some(o) = tokens.next() else { continue };

        let sh = hash_token(s);
        let ph = hash_token(p);
        let oh = hash_token(o) & OBJECT_HASH_MASK;
        quins.push(QualiaQuin {
            subject: sh,
            predicate: ph,
            object: oh,
            context: 0,
            metadata: 0,
            parity: 0,
        });
    }

    Ok(quins)
}

fn load_quins(path: &Path, format: InputFormat) -> std::io::Result<Vec<QualiaQuin>> {
    match format {
        InputFormat::Superblock => read_cli_superblock_quins(path),
        InputFormat::Cq42 => read_c_q42_quins(path),
        InputFormat::Ntriples => read_ntriples_quins(path),
    }
}

fn latency_stats_ms(times_ms: &mut [f64]) -> serde_json::Value {
    times_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = times_ms.len();
    let mean = times_ms.iter().sum::<f64>() / n as f64;
    serde_json::json!({
        "min": times_ms[0],
        "max": times_ms[n - 1],
        "mean": mean,
        "p50": times_ms[n / 2],
        "p95": times_ms[(n as f64 * 0.95) as usize],
        "p99": times_ms[(n as f64 * 0.99) as usize],
        "samples": n,
        "warmup_samples": WARMUP,
        "unit": "milliseconds"
    })
}

fn bench<F: FnMut()>(mut f: F) -> serde_json::Value {
    for _ in 0..WARMUP {
        f();
    }
    let mut times = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let t0 = Instant::now();
        f();
        times.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    latency_stats_ms(&mut times)
}

fn peak_rss_mb() -> f64 {
    let mut sys = System::new_all();
    sys.refresh_all();
    let pid = sysinfo::get_current_pid().expect("current pid");
    sys.process(pid)
        .map(|p| p.memory() as f64 / 1_048_576.0)
        .unwrap_or(0.0)
}

fn build_index(quins: &[QualiaQuin]) -> HashMap<u64, Vec<(u64, u64)>> {
    let mut map: HashMap<u64, Vec<(u64, u64)>> = HashMap::new();
    for q in quins {
        map.entry(q.subject)
            .or_default()
            .push((q.predicate, q.object));
    }
    map
}

fn parse_format(raw: &str) -> Option<InputFormat> {
    match raw {
        "superblock" | "q42" => Some(InputFormat::Superblock),
        "cq42" | "c.q42" => Some(InputFormat::Cq42),
        "ntriples" | "nt" => Some(InputFormat::Ntriples),
        _ => None,
    }
}

fn infer_format(path: &Path, engine: &str) -> InputFormat {
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        if ext == "nt" {
            return InputFormat::Ntriples;
        }
    }
    if path
        .file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|n| n.ends_with(".c.q42"))
    {
        return InputFormat::Cq42;
    }
    match engine {
        "qualia_nt" => InputFormat::Ntriples,
        "qualia_cq42" => InputFormat::Cq42,
        _ => InputFormat::Superblock,
    }
}

fn dataset_format_label(format: InputFormat) -> &'static str {
    match format {
        InputFormat::Superblock => "q42",
        InputFormat::Cq42 => "c.q42",
        InputFormat::Ntriples => "ntriples",
    }
}

fn measurement_path(format: InputFormat) -> &'static str {
    match format {
        InputFormat::Superblock => "in_process_q42_superblock",
        InputFormat::Cq42 => "in_process_cq42_decompress",
        InputFormat::Ntriples => "in_process_ntriples_parse",
    }
}

fn default_note(format: InputFormat) -> &'static str {
    match format {
        InputFormat::Superblock => {
            "Loads qualia-cli SuperBlock .q42 and queries an in-memory subject index."
        }
        InputFormat::Cq42 => {
            "Decompresses .c.q42 LZ4 distribution blocks and queries an in-memory subject index."
        }
        InputFormat::Ntriples => {
            "Parses N-Triples text (same RDF source as Oxigraph/Comunica) into quins and queries an in-memory subject index."
        }
    }
}

fn main() {
    let mut input_path: Option<PathBuf> = None;
    let mut queries_json: Option<String> = None;
    let mut format: Option<InputFormat> = None;
    let mut engine = String::from("qualia_q42");

    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" | "--q42" => {
                i += 1;
                input_path = Some(PathBuf::from(&args[i]));
            }
            "--format" => {
                i += 1;
                format = parse_format(&args[i]);
                if format.is_none() {
                    eprintln!("unknown --format: {}", args[i]);
                    std::process::exit(2);
                }
            }
            "--engine" => {
                i += 1;
                engine = args[i].clone();
            }
            "--queries-json" => {
                i += 1;
                queries_json = Some(args[i].clone());
            }
            _ => {}
        }
        i += 1;
    }

    let input_path = match input_path {
        Some(p) => p,
        None => {
            eprintln!("missing --input (or legacy --q42)");
            std::process::exit(2);
        }
    };

    let queries: QuerySpec = match queries_json {
        Some(raw) => serde_json::from_str(&raw).unwrap_or_else(|e| {
            eprintln!("invalid --queries-json: {e}");
            std::process::exit(2);
        }),
        None => {
            eprintln!("missing --queries-json");
            std::process::exit(2);
        }
    };

    let format = format.unwrap_or_else(|| infer_format(&input_path, &engine));

    let file_bytes = fs::metadata(&input_path).map(|m| m.len()).unwrap_or(0);
    let file_mb = (file_bytes as f64) / (1024.0 * 1024.0);

    let t0 = Instant::now();
    let quins = match load_quins(&input_path, format) {
        Ok(q) => q,
        Err(e) => {
            let out = serde_json::json!({
                "engine": engine,
                "error": format!("load {:?} failed: {e}", format),
                "dataset_file_bytes": file_bytes,
                "dataset_file_mb": (file_mb * 1000.0).round() / 1000.0,
                "dataset_format": dataset_format_label(format),
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
            std::process::exit(1);
        }
    };
    let index = build_index(&quins);
    let ingestion_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let point_subject = q_hash(&queries.point_subject);
    let twohop_start = q_hash(&queries.twohop_start);
    let filter_predicate = q_hash(&queries.filter_predicate);

    let point = bench(|| {
        let _ = index.get(&point_subject);
    });

    let twohop = bench(|| {
        let mut count = 0usize;
        if let Some(edges) = index.get(&twohop_start) {
            for &(_, obj) in edges {
                if let Some(h2) = index.get(&obj) {
                    count += h2.len();
                }
            }
        }
        std::hint::black_box(count);
    });

    let filter = bench(|| {
        let mut count = 0usize;
        for edges in index.values() {
            for &(p, _) in edges {
                if p == filter_predicate {
                    count += 1;
                }
            }
        }
        std::hint::black_box(count);
    });

    let peak_rss = peak_rss_mb();

    let out = serde_json::json!({
        "engine": engine,
        "n_triples": quins.len(),
        "ingestion_ms": (ingestion_ms * 1000.0).round() / 1000.0,
        "point": point,
        "twohop": twohop,
        "filter": filter,
        "peak_rss_mb": (peak_rss * 100.0).round() / 100.0,
        "dataset_file_bytes": file_bytes,
        "dataset_file_mb": (file_mb * 1000.0).round() / 1000.0,
        "dataset_format": dataset_format_label(format),
        "dataset_file_path": input_path.to_string_lossy(),
        "measurement_path": measurement_path(format),
        "note": default_note(format),
    });

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
