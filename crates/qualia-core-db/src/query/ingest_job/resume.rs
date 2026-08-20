//! Adopt existing quin/lex runs and decide whether a job can continue.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::job::{IngestJob, JOB_RUNS};
use super::source::IngestEncoding;
use super::source::IngestSourceKind;
use crate::query::ingest::IngestMode;

/// A job can continue when it is not complete and we still have the source
/// locator. Turtle/HTTP resume re-reads from the start and skips already
/// accepted triples (correct; prefixes stay valid). Gzip/zstd HTTP costs
/// another download. There is no mid-stream inflater checkpoint in v1.
pub fn resume_is_supported(job: &IngestJob) -> bool {
    !job.is_complete()
}

/// Copy a live/legacy scratch tree into a new job directory so it can be inspected
/// and, if the source file is still there and uncompressed, continued.
pub fn adopt_legacy_scratch(
    scratch: &Path,
    job_dir: PathBuf,
    source_path: &Path,
    output: &Path,
    mode: IngestMode,
    segment_mib: Option<u64>,
) -> io::Result<IngestJob> {
    let run_src = find_legacy_run_root(scratch)?;
    let mut job = IngestJob::create(
        job_dir,
        IngestSourceKind::File {
            path: source_path.to_string_lossy().into_owned(),
        },
        IngestEncoding::Identity,
        super::source::infer_rdf_format(&source_path.to_string_lossy()),
        mode,
        segment_mib,
        output,
    )?;
    let dest = job.runs_dir();
    let mut chunks = 0u64;
    let mut lex = 0u64;
    let mut triples = 0u64;
    for ent in fs::read_dir(&run_src)? {
        let ent = ent?;
        let name = ent.file_name();
        let name_s = name.to_string_lossy();
        if !(name_s.starts_with("chunk_") || name_s.starts_with("lexrun_")) {
            continue;
        }
        if name_s.starts_with("chunk_") {
            let len = ent.metadata()?.len();
            if len % 48 != 0 {
                // Torn last write — skip; resume will re-parse that tail.
                continue;
            }
            chunks += 1;
            triples += len / 48;
        } else {
            lex += 1;
        }
        // Same-volume rename: OSM-scale scratch is 100+ GiB; copy would not fit.
        let dest_path = dest.join(&name);
        if let Err(err) = fs::rename(ent.path(), &dest_path) {
            fs::copy(ent.path(), &dest_path)?;
            let _ = err;
        }
    }
    job.record_progress(triples, 0, chunks, lex)?;
    Ok(job)
}

/// Publish already-hashed quin/lex runs to the job's `.q42` root. Does **not**
/// re-read the Turtle (the OSM continue died on disk during this step).
pub fn publish_job(
    job_dir: &Path,
    report: crate::query::ingest_report::IngestReport,
) -> io::Result<u64> {
    let job = IngestJob::open(job_dir.to_path_buf())?;
    let mut sorter = crate::external_sort::ExternalSorter::adopt_existing(job.runs_dir())?;
    sorter.set_note_sink(report.note_sink());
    let cap = job
        .spec
        .segment_mib
        .unwrap_or(512)
        .saturating_mul(1024 * 1024);
    if cap == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "segment_mib must be greater than zero",
        ));
    }
    report.emit(
        crate::query::ingest_report::IngestPhase::Publishing,
        format!(
            "publishing {} quin run(s) + {} lex run(s) → {}",
            sorter.quin_run_count(),
            sorter.lex_run_count(),
            job.spec.output
        ),
        None,
    );
    let stats = sorter.merge_volume_set(Path::new(&job.spec.output), cap)?;
    Ok(stats.blocks_written)
}

/// Continue is implemented by `streaming_import_rdf_with_job` (adopt + skip-N).
pub fn continue_job(job: &IngestJob) -> io::Result<u64> {
    crate::query::ingest::streaming_import_rdf_with_job(
        &job.dir,
        crate::query::ingest_report::IngestReport::silent(),
    )
}

fn find_legacy_run_root(scratch: &Path) -> io::Result<PathBuf> {
    if scratch.join(JOB_RUNS).is_dir() {
        return Ok(scratch.join(JOB_RUNS));
    }
    let mut best = scratch.to_path_buf();
    let mut best_n = 0u64;
    if let Ok(rd) = fs::read_dir(scratch) {
        for ent in rd.flatten() {
            let p = ent.path();
            if !p.is_dir() {
                continue;
            }
            let n = fs::read_dir(&p)
                .map(|rd| {
                    rd.flatten()
                        .filter(|e| e.file_name().to_string_lossy().starts_with("chunk_"))
                        .count() as u64
                })
                .unwrap_or(0);
            if n > best_n {
                best_n = n;
                best = p;
            }
        }
    }
    if best_n == 0
        && fs::read_dir(scratch)
            .map(|rd| {
                rd.flatten()
                    .any(|e| e.file_name().to_string_lossy().starts_with("chunk_"))
            })
            .unwrap_or(false)
    {
        return Ok(scratch.to_path_buf());
    }
    if best_n == 0 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no chunk_*.tmp under {}", scratch.display()),
        ));
    }
    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NQuin;

    #[test]
    fn adopt_moves_chunks_on_same_volume() {
        let scratch = tempfile::TempDir::new().unwrap();
        let inner = scratch.path().join("run");
        fs::create_dir_all(&inner).unwrap();
        let q = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 0,
            metadata: 0,
            parity: 1 ^ 2 ^ 3,
        };
        let bytes = bytemuck::bytes_of(&q);
        fs::write(inner.join("chunk_0.tmp"), bytes).unwrap();
        fs::write(inner.join("lexrun_0.tmp"), b"x").unwrap();
        let job_dir = scratch.path().join("job");
        let src = scratch.path().join("src.ttl");
        fs::write(&src, b"").unwrap();
        let out = scratch.path().join("out.q42");
        let job = adopt_legacy_scratch(
            scratch.path(),
            job_dir,
            &src,
            &out,
            IngestMode::Complete,
            Some(512),
        )
        .unwrap();
        assert_eq!(job.checkpoint.triples, 1);
        assert_eq!(job.checkpoint.quin_chunks, 1);
        assert!(!inner.join("chunk_0.tmp").exists());
        assert!(job.runs_dir().join("chunk_0.tmp").exists());
    }
}
