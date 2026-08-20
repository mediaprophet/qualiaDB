//! Human/agent status for a durable ingest job: checkpoint + live progress.json.

use serde::Serialize;
use std::fs;
use std::io;
use std::path::Path;

use super::job::{IngestJob, IngestJobPhase, JOB_PROGRESS};
use super::resume::resume_is_supported;
use crate::query::ingest_report::{format_bytes, format_count, IngestSnapshot};

#[derive(Clone, Debug, Serialize)]
pub struct IngestJobStatus {
    pub dir: String,
    pub phase: IngestJobPhase,
    pub accepted_triples: u64,
    pub quin_chunks: u64,
    pub lex_runs: u64,
    pub run_bytes: u64,
    pub source: String,
    pub output: String,
    pub output_exists: bool,
    pub resumable: bool,
    pub complete: bool,
    /// Live tick from `progress.json` (this process or a still-running continue).
    pub live: Option<IngestSnapshot>,
    pub summary: String,
}

pub fn job_status(dir: &Path) -> io::Result<IngestJobStatus> {
    let job = IngestJob::open(dir.to_path_buf())?;
    let live = read_progress_json(&dir.join(JOB_PROGRESS));
    let run_bytes = dir_bytes(&job.runs_dir());
    let output_exists = Path::new(&job.spec.output).is_file();
    let summary = format_status_line(&job, live.as_ref(), run_bytes, output_exists);
    Ok(IngestJobStatus {
        dir: dir.display().to_string(),
        phase: job.checkpoint.phase,
        accepted_triples: job.checkpoint.triples,
        quin_chunks: job.checkpoint.quin_chunks,
        lex_runs: job.checkpoint.lex_runs,
        run_bytes,
        source: job.spec.source.locator().to_string(),
        output: job.spec.output.clone(),
        output_exists,
        resumable: resume_is_supported(&job),
        complete: job.is_complete(),
        live,
        summary,
    })
}

pub fn read_progress_json(path: &Path) -> Option<IngestSnapshot> {
    let raw = fs::read(path).ok()?;
    serde_json::from_slice(&raw).ok()
}

fn dir_bytes(dir: &Path) -> u64 {
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return 0,
    };
    rd.flatten()
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

fn format_status_line(
    job: &IngestJob,
    live: Option<&IngestSnapshot>,
    run_bytes: u64,
    output_exists: bool,
) -> String {
    if let Some(snap) = live {
        let extra = if snap.skip_target > 0 && snap.phase.as_str() == "skip" {
            format!(
                "  skip {} / {}",
                format_count(snap.triples),
                format_count(snap.skip_target)
            )
        } else {
            String::new()
        };
        return format!(
            "{}  {}  {:.1}%  {:.1} MB/s  eta {}  runs {}{}",
            snap.phase.as_str(),
            format_count(snap.triples),
            snap.pct,
            snap.mb_per_sec,
            snap.eta_secs
                .map(|s| format!("{:.0}s", s))
                .unwrap_or_else(|| "—".into()),
            format_bytes(run_bytes),
            extra
        );
    }
    format!(
        "checkpoint {:?}  accepted {}  chunks {}  lex {}  runs {}  output {}",
        job.checkpoint.phase,
        format_count(job.checkpoint.triples),
        job.checkpoint.quin_chunks,
        job.checkpoint.lex_runs,
        format_bytes(run_bytes),
        if output_exists { "present" } else { "none" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::ingest::IngestMode;
    use crate::query::ingest_job::{IngestEncoding, IngestRdfFormat, IngestSourceKind};
    use crate::query::ingest_report::{IngestPhase, IngestReport};

    #[test]
    fn status_reads_live_progress_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let job_dir = dir.path().join("job");
        let src = dir.path().join("a.nt");
        let out = dir.path().join("a.q42");
        fs::write(&src, b"").unwrap();
        IngestJob::create(
            job_dir.clone(),
            IngestSourceKind::File {
                path: src.to_string_lossy().into_owned(),
            },
            IngestEncoding::Identity,
            IngestRdfFormat::NTriples,
            IngestMode::Complete,
            None,
            &out,
        )
        .unwrap();
        let report = IngestReport::silent();
        report.attach_progress_file(job_dir.join(JOB_PROGRESS));
        report.set_skip_target(1_808_000_000);
        report.set_source_bytes(100);
        report.add_bytes_read(46);
        report.maybe_tick_skip(2_000_000, 1_808_000_000);
        let st = job_status(&job_dir).unwrap();
        assert!(st.resumable);
        assert!(!st.complete);
        let live = st.live.expect("progress.json");
        assert_eq!(live.phase, IngestPhase::Skipping);
        assert!(st.summary.contains("skip"), "{}", st.summary);
    }
}
