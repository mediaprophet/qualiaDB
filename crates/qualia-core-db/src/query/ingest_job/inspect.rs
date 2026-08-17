//! Review a job directory, a legacy scratch tree, or a volume root.

use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::job::{IngestJob, IngestJobPhase, SourceAttestation};
use crate::q42_volume::Q42Volume;

#[derive(Clone, Debug, Serialize)]
pub struct IngestInspectReport {
    pub kind: String,
    pub path: String,
    pub phase: Option<IngestJobPhase>,
    pub triples: u64,
    pub quin_chunks: u64,
    pub lex_runs: u64,
    pub run_bytes: u64,
    pub last_chunk: Option<String>,
    pub last_write_unix: Option<u64>,
    pub complete: bool,
    pub resumable: bool,
    pub notes: Vec<String>,
    pub attestation: Option<SourceAttestation>,
    pub volume_segments: Option<u64>,
    pub volume_lex_shards: Option<u64>,
}

pub fn inspect_job(dir: &Path) -> io::Result<IngestInspectReport> {
    let job = IngestJob::open(dir.to_path_buf())?;
    let runs = summarize_runs(&job.runs_dir())?;
    let resumable = super::resume::resume_is_supported(&job);
    let mut notes = Vec::new();
    notes.push(format!("source {}", job.spec.source.locator()));
    notes.push(format!(
        "encoding {:?} format {:?} mode {}",
        job.spec.encoding, job.spec.format, job.spec.mode
    ));
    if let Some(att) = job.load_attestation()? {
        notes.push(att.verify_story().to_string());
    }
    if !job.is_complete() {
        notes.push("job is incomplete — use ingest-job continue if resumable".into());
    }
    if let Some(live) = super::status::read_progress_json(&dir.join(super::job::JOB_PROGRESS)) {
        notes.push(format!(
            "live {}  {:.1}%  {:.1} MB/s  {} triples",
            live.phase.as_str(),
            live.pct,
            live.mb_per_sec,
            live.triples
        ));
    }
    Ok(IngestInspectReport {
        kind: "job".into(),
        path: dir.display().to_string(),
        phase: Some(job.checkpoint.phase),
        triples: job.checkpoint.triples.max(runs.triples_estimate),
        quin_chunks: runs.quin_chunks,
        lex_runs: runs.lex_runs,
        run_bytes: runs.bytes,
        last_chunk: runs.last_name,
        last_write_unix: runs.last_write,
        complete: job.is_complete(),
        resumable,
        notes,
        attestation: job.load_attestation()?,
        volume_segments: None,
        volume_lex_shards: None,
    })
}

/// Review a TempDir-style scratch (the live OSM job): chunk_*.tmp + lexrun_*.tmp.
pub fn inspect_legacy_scratch(dir: &Path) -> io::Result<IngestInspectReport> {
    let search = find_run_dir(dir)?;
    let runs = summarize_runs(&search)?;
    let mut notes = vec![
        "legacy scratch (no job.json) — this is what the pre-job OSM ingest writes".into(),
        "cannot resume this layout as a job until `ingest-job adopt-scratch` copies it".into(),
        format!("quin estimate ≈ {} (1M quins per full 45.8 MiB chunk)", runs.triples_estimate),
    ];
    if runs.partial_tail {
        notes.push("last chunk is not a multiple of 48 bytes — treat as torn; do not adopt it".into());
    }
    Ok(IngestInspectReport {
        kind: "legacy_scratch".into(),
        path: search.display().to_string(),
        phase: Some(IngestJobPhase::Parsing),
        triples: runs.triples_estimate,
        quin_chunks: runs.quin_chunks,
        lex_runs: runs.lex_runs,
        run_bytes: runs.bytes,
        last_chunk: runs.last_name,
        last_write_unix: runs.last_write,
        complete: false,
        resumable: false,
        notes,
        attestation: None,
        volume_segments: None,
        volume_lex_shards: None,
    })
}

pub fn inspect_volume_root(path: &Path) -> io::Result<IngestInspectReport> {
    let vol = Q42Volume::open(path)?;
    let mut notes = Vec::new();
    let (segs, lex) = match vol.volume_manifest()? {
        Some(m) => {
            notes.push(format!("generation {}", m.generation));
            (m.segments.len() as u64, m.lexicon_segments.len() as u64)
        }
        None => {
            notes.push("single-file volume (no volume-set manifest)".into());
            (0, 0)
        }
    };
    let att_path = path.with_extension("q42.source.json");
    let attestation = if att_path.is_file() {
        Some(super::job::read_json(&att_path)?)
    } else {
        notes.push("no sidecar source-attestation (*.q42.source.json)".into());
        None
    };
    Ok(IngestInspectReport {
        kind: "volume".into(),
        path: path.display().to_string(),
        phase: Some(IngestJobPhase::Complete),
        triples: 0,
        quin_chunks: 0,
        lex_runs: 0,
        run_bytes: fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        last_chunk: None,
        last_write_unix: None,
        complete: true,
        resumable: false,
        notes,
        attestation,
        volume_segments: Some(segs),
        volume_lex_shards: Some(lex),
    })
}

struct RunSummary {
    quin_chunks: u64,
    lex_runs: u64,
    bytes: u64,
    triples_estimate: u64,
    last_name: Option<String>,
    last_write: Option<u64>,
    partial_tail: bool,
}

fn summarize_runs(dir: &Path) -> io::Result<RunSummary> {
    let mut quin_chunks = 0u64;
    let mut lex_runs = 0u64;
    let mut bytes = 0u64;
    let mut triples_estimate = 0u64;
    let mut last_name = None;
    let mut last_write = None::<u64>;
    let mut last_mtime = 0u64;
    let mut partial_tail = false;
    if !dir.is_dir() {
        return Ok(RunSummary {
            quin_chunks,
            lex_runs,
            bytes,
            triples_estimate,
            last_name,
            last_write,
            partial_tail,
        });
    }
    let mut chunks: Vec<PathBuf> = Vec::new();
    for ent in fs::read_dir(dir)? {
        let ent = ent?;
        let path = ent.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let meta = ent.metadata()?;
        let len = meta.len();
        bytes += len;
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if mtime >= last_mtime {
            last_mtime = mtime;
            last_write = Some(mtime);
            last_name = Some(name.clone());
        }
        if name.starts_with("chunk_") && name.ends_with(".tmp") {
            quin_chunks += 1;
            chunks.push(path);
            if len % 48 != 0 {
                partial_tail = true;
            } else {
                triples_estimate += len / 48;
            }
        } else if name.starts_with("lexrun_") && name.ends_with(".tmp") {
            lex_runs += 1;
        }
    }
    let _ = chunks;
    Ok(RunSummary {
        quin_chunks,
        lex_runs,
        bytes,
        triples_estimate,
        last_name,
        last_write,
        partial_tail,
    })
}

fn find_run_dir(dir: &Path) -> io::Result<PathBuf> {
    if dir.join("job.json").is_file() {
        return Ok(dir.join("runs"));
    }
    // TempDir nest: scratch/.tmpXXXX/chunk_0.tmp
    let mut best = dir.to_path_buf();
    let mut best_n = count_chunks(dir);
    if let Ok(rd) = fs::read_dir(dir) {
        for ent in rd.flatten() {
            if ent.path().is_dir() {
                let n = count_chunks(&ent.path());
                if n > best_n {
                    best_n = n;
                    best = ent.path();
                }
            }
        }
    }
    Ok(best)
}

fn count_chunks(dir: &Path) -> u64 {
    fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("chunk_")
                })
                .count() as u64
        })
        .unwrap_or(0)
}
