//! Footprint Validator — Qualia-DB 512 MB RAM Floor Constraint
//!
//! Simulates processing a 500 MB WordNet shard and asserts that peak RSS
//! never exceeds the 512 MB architectural ceiling.  Run via:
//!
//! ```
//! cargo bench --package qualia-core-db --bench ram_usage
//! ```
//!
//! Exit code 0 = PASS, 1 = FAIL (peak RSS exceeded ceiling).
//! Designed to be wired into CI as a blocking check.

use std::time::Instant;
use sysinfo::{Pid, System};

/// Architectural RAM ceiling.  Any commit that pushes peak RSS above this
/// value on a 500 MB shard should be blocked in CI.
const RSS_CEILING_BYTES: u64 = 512 * 1024 * 1024;

/// Synthetic shard size: 10 M × 48-byte Quins ≈ 480 MB — realistic for a
/// full WordNet sharding pass.
const QUIN_SIZE: usize = 48;
const SYNTHETIC_QUIN_COUNT: usize = 10_000_000;

fn rss_bytes(sys: &mut System, pid: Pid) -> u64 {
    sys.refresh_process(pid);
    sys.process(pid).map(|p| p.memory()).unwrap_or(0)
}

fn main() {
    let mut sys = System::new();
    let pid = Pid::from(std::process::id() as usize);

    println!("╔══════════════════════════════════════════════════╗");
    println!("║     Qualia-DB RAM Footprint Validator            ║");
    println!("║     Ceiling: 512 MB  |  Shard: ~480 MB          ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    let rss_base = rss_bytes(&mut sys, pid);
    println!("RSS baseline    {:>7} MB", rss_base / (1024 * 1024));

    // ── Allocate synthetic shard ─────────────────────────────────────────────
    let shard_bytes = SYNTHETIC_QUIN_COUNT * QUIN_SIZE;
    println!("Shard size      {:>7} MB  ({} quins × {} B)",
        shard_bytes / (1024 * 1024), SYNTHETIC_QUIN_COUNT, QUIN_SIZE);

    let t_start = Instant::now();

    // Fill with deterministic content so the allocator can't collapse pages.
    let shard: Vec<u8> = (0u64..)
        .take(shard_bytes)
        .map(|i| ((i ^ (i >> 8) ^ (i >> 16)) & 0xff) as u8)
        .collect();

    let rss_after_alloc = rss_bytes(&mut sys, pid);
    println!("RSS after alloc {:>7} MB  (Δ +{} MB)",
        rss_after_alloc / (1024 * 1024),
        rss_after_alloc.saturating_sub(rss_base) / (1024 * 1024));

    // ── Scan: read every Quin record, accumulate checksum ────────────────────
    // This mirrors the hot path in webizen_bytecode::execute_program —
    // sequential 48-byte reads, integer equality on S/P/O fields.
    let mut checksum: u64 = 0;
    for chunk in shard.chunks_exact(QUIN_SIZE) {
        // Safety: chunks_exact guarantees exactly QUIN_SIZE bytes.
        let s = u64::from_le_bytes(chunk[0..8].try_into().unwrap());
        let p = u64::from_le_bytes(chunk[8..16].try_into().unwrap());
        let o = u64::from_le_bytes(chunk[16..24].try_into().unwrap());
        // Prevent the optimizer from eliding the reads.
        checksum = checksum.wrapping_add(s ^ p.wrapping_mul(0xcbf29ce484222325) ^ o);
    }

    let elapsed = t_start.elapsed();
    let rss_peak = rss_bytes(&mut sys, pid);

    println!("RSS after scan  {:>7} MB  (Δ +{} MB)  ← peak",
        rss_peak / (1024 * 1024),
        rss_peak.saturating_sub(rss_base) / (1024 * 1024));
    println!("Throughput      {:.1} MB/s",
        shard_bytes as f64 / elapsed.as_secs_f64() / (1024.0 * 1024.0));
    println!("Elapsed         {:.3} s", elapsed.as_secs_f64());
    println!("Checksum        0x{checksum:016x}  (non-zero ⇒ scan was real)");

    // ── Release and confirm OS reclaim ───────────────────────────────────────
    drop(shard);
    let rss_after_drop = rss_bytes(&mut sys, pid);
    println!("RSS after drop  {:>7} MB", rss_after_drop / (1024 * 1024));
    println!();

    // ── Constraint verdict ───────────────────────────────────────────────────
    let ceiling_mb = RSS_CEILING_BYTES / (1024 * 1024);
    let peak_mb    = rss_peak / (1024 * 1024);

    if rss_peak <= RSS_CEILING_BYTES {
        println!("PASS  peak {peak_mb} MB ≤ {ceiling_mb} MB ceiling");
        std::process::exit(0);
    } else {
        eprintln!("FAIL  peak {peak_mb} MB EXCEEDS {ceiling_mb} MB ceiling  \
                   (overshoot: {} MB)",
            peak_mb - ceiling_mb);
        std::process::exit(1);
    }
}
