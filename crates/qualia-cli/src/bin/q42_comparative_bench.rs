//! Comparative harness helper: load a `.q42` artifact and benchmark point / two-hop / filter.
//!
//! Usage:
//!   q42_comparative_bench --q42 path/to/file.q42 --queries-json '{"point_subject":"..."}'
//!
//! Emits JSON on stdout for `benchmarks/qualia/q42_runner.py`.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use qualia_core_db::{q_hash, QualiaQuin, QUINS_PER_BLOCK};

const BLOCK_SIZE: u64 = 40_960;
const HEADER_SIZE: usize = 160;
const QUIN_SIZE: usize = 48;

/// Read `.q42` files produced by `qualia-cli ingest` (160-byte SuperBlock headers).
fn read_cli_superblock_quins(path: &std::path::Path) -> std::io::Result<Vec<QualiaQuin>> {
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

const WARMUP: usize = 10;
const SAMPLES: usize = 30;

#[derive(serde::Deserialize)]
struct QuerySpec {
    point_subject: String,
    twohop_start: String,
    filter_predicate: String,
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

fn build_index(quins: &[QualiaQuin]) -> HashMap<u64, Vec<(u64, u64)>> {
    let mut map: HashMap<u64, Vec<(u64, u64)>> = HashMap::new();
    for q in quins {
        map.entry(q.subject).or_default().push((q.predicate, q.object));
    }
    map
}

fn main() {
    let mut q42_path: Option<PathBuf> = None;
    let mut queries_json: Option<String> = None;

    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--q42" => {
                i += 1;
                q42_path = Some(PathBuf::from(&args[i]));
            }
            "--queries-json" => {
                i += 1;
                queries_json = Some(args[i].clone());
            }
            _ => {}
        }
        i += 1;
    }

    let q42_path = match q42_path {
        Some(p) => p,
        None => {
            eprintln!("missing --q42");
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

    let file_bytes = fs::metadata(&q42_path).map(|m| m.len()).unwrap_or(0);
    let file_mb = (file_bytes as f64) / (1024.0 * 1024.0);

    let t0 = Instant::now();
    let quins = match read_cli_superblock_quins(&q42_path) {
        Ok(q) => q,
        Err(e) => {
            let out = serde_json::json!({
                "engine": "qualia_q42",
                "error": format!("read_cli_superblock_quins failed: {e}"),
                "dataset_file_bytes": file_bytes,
                "dataset_file_mb": (file_mb * 1000.0).round() / 1000.0,
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

    let out = serde_json::json!({
        "engine": "qualia_q42",
        "n_triples": quins.len(),
        "ingestion_ms": (ingestion_ms * 1000.0).round() / 1000.0,
        "point": point,
        "twohop": twohop,
        "filter": filter,
        "dataset_file_bytes": file_bytes,
        "dataset_file_mb": (file_mb * 1000.0).round() / 1000.0,
        "dataset_format": "q42",
        "native_q42_path": q42_path.to_string_lossy(),
        "measurement_path": "in_process_q42_mmap",
        "note": "Loads qualia-cli SuperBlock .q42 and queries an in-memory subject index.",
    });

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
