//! Progress and process snapshots for RDF → Q42 ingest.
//!
//! Library callers stay silent unless they attach a sink. The CLI and desktop
//! UIs attach text, JSON-lines, or a `ProgressPayload` mapper. Cold path only.

use serde::Serialize;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const TICK_EVERY_TRIPLES: u64 = 2_000_000;
const TICK_EVERY_MS: u128 = 2_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestPhase {
    Starting,
    /// Re-reading already-accepted triples (skip-N). Not new work.
    Skipping,
    Parsing,
    Sorting,
    Compacting,
    Publishing,
    Lexicon,
    Complete,
    Failed,
}

impl IngestPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Skipping => "skip",
            Self::Parsing => "parse",
            Self::Sorting => "sort",
            Self::Compacting => "compact",
            Self::Publishing => "publish",
            Self::Lexicon => "lexicon",
            Self::Complete => "complete",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct IngestSnapshot {
    pub phase: IngestPhase,
    pub triples: u64,
    pub source_bytes: u64,
    pub bytes_read: u64,
    pub interned_terms: u64,
    pub lex_runs: u64,
    pub quin_chunks: u64,
    pub workers: u32,
    pub rss_bytes: Option<u64>,
    pub elapsed_ms: u64,
    pub triples_per_sec: f64,
    pub mb_per_sec: f64,
    pub pct: f64,
    pub eta_secs: Option<f64>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Wall ms spent in parse (set at parse-finished).
    pub parse_ms: u64,
    /// Wall ms spent in GPU/CPU chunk sorts (sum).
    pub sort_ms: u64,
    /// Wall ms spent merging/publishing the volume.
    pub publish_ms: u64,
    /// Last chunk sort path (`gpu` / `cpu`), if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sort_path: Option<String>,
    /// Accepted triples already on disk (skip-N target). 0 if this is a fresh ingest.
    pub skip_target: u64,
}

impl IngestSnapshot {
    pub fn format_text(&self, verbose: u8) -> String {
        let rss = self
            .rss_bytes
            .map(format_bytes)
            .unwrap_or_else(|| "n/a".into());
        let eta = self.eta_secs.map(format_secs).unwrap_or_else(|| "—".into());
        let triples_col = if self.phase == IngestPhase::Skipping && self.skip_target > 0 {
            format!(
                "{} / {} replayed",
                format_count(self.triples),
                format_count(self.skip_target)
            )
        } else {
            format!("{} triples", format_count(self.triples))
        };
        let mut line = format!(
            "ingest {:>5.1}%  {:<8}  {}  {:.0}/s  {:.1} MB/s  rss {}  eta {}",
            self.pct,
            self.phase.as_str(),
            triples_col,
            self.triples_per_sec,
            self.mb_per_sec,
            rss,
            eta
        );
        if verbose >= 1 {
            line.push_str(&format!(
                "  | workers {}  interned {}  lex_runs {}  quin_chunks {}  read {}  parse {}ms sort {}ms publish {}ms",
                self.workers,
                format_count(self.interned_terms),
                self.lex_runs,
                self.quin_chunks,
                format_bytes(self.bytes_read),
                self.parse_ms,
                self.sort_ms,
                self.publish_ms
            ));
            if let Some(p) = &self.last_sort_path {
                line.push_str("  accel ");
                line.push_str(p);
            }
        }
        if verbose >= 2 {
            if let Some(detail) = &self.detail {
                line.push_str("  | ");
                line.push_str(detail);
            } else if !self.message.is_empty() {
                line.push_str("  | ");
                line.push_str(&self.message);
            }
        }
        line
    }

    pub fn format_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{\"phase\":\"failed\"}".into())
    }
}

#[derive(Default)]
struct Counters {
    triples: AtomicU64,
    bytes_read: AtomicU64,
    source_bytes: AtomicU64,
    interned: AtomicU64,
    lex_runs: AtomicU64,
    quin_chunks: AtomicU64,
    parse_ms: AtomicU64,
    sort_ms: AtomicU64,
    publish_ms: AtomicU64,
    last_sort_path: Mutex<Option<String>>,
    skip_target: AtomicU64,
}

/// Cloneable handle shared by the parser, hasher collector, and merge.
#[derive(Clone)]
pub struct IngestReport {
    sink: Arc<dyn Fn(&IngestSnapshot) + Send + Sync>,
    verbose: u8,
    workers: u32,
    started: Instant,
    last_emit: Arc<Mutex<Instant>>,
    last_triples: Arc<AtomicU64>,
    counters: Arc<Counters>,
    progress_path: Arc<Mutex<Option<std::path::PathBuf>>>,
}

impl IngestReport {
    pub fn silent() -> Self {
        Self::new(0, |_| {})
    }

    pub fn new(verbose: u8, sink: impl Fn(&IngestSnapshot) + Send + Sync + 'static) -> Self {
        let now = Instant::now();
        Self {
            sink: Arc::new(sink),
            verbose,
            workers: 0,
            started: now,
            last_emit: Arc::new(Mutex::new(now)),
            last_triples: Arc::new(AtomicU64::new(0)),
            counters: Arc::new(Counters::default()),
            progress_path: Arc::new(Mutex::new(None)),
        }
    }

    pub fn text_stderr(verbose: u8) -> Self {
        Self::new(verbose, move |snap| {
            let _ = writeln!(io::stderr(), "{}", snap.format_text(verbose));
        })
    }

    pub fn json_stdout(verbose: u8) -> Self {
        Self::new(verbose, move |snap| {
            let _ = writeln!(io::stdout(), "{}", snap.format_json());
        })
    }

    pub fn set_workers(&mut self, workers: u32) {
        self.workers = workers;
    }

    pub fn set_source_bytes(&self, n: u64) {
        self.counters.source_bytes.store(n, Ordering::Relaxed);
    }

    pub fn add_bytes_read(&self, n: u64) {
        self.counters.bytes_read.fetch_add(n, Ordering::Relaxed);
    }

    pub fn set_triples(&self, n: u64) {
        self.counters.triples.store(n, Ordering::Relaxed);
    }

    pub fn set_interned(&self, n: u64) {
        self.counters.interned.store(n, Ordering::Relaxed);
    }

    pub fn add_lex_run(&self) {
        self.counters.lex_runs.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_quin_chunk(&self) {
        self.counters.quin_chunks.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_sort_ms(&self, ms: u64) {
        self.counters.sort_ms.fetch_add(ms, Ordering::Relaxed);
    }

    pub fn set_parse_ms(&self, ms: u64) {
        self.counters.parse_ms.store(ms, Ordering::Relaxed);
    }

    pub fn set_publish_ms(&self, ms: u64) {
        self.counters.publish_ms.store(ms, Ordering::Relaxed);
    }

    pub fn set_skip_target(&self, n: u64) {
        self.counters.skip_target.store(n, Ordering::Relaxed);
    }

    pub fn set_quin_chunks(&self, n: u64) {
        self.counters.quin_chunks.store(n, Ordering::Relaxed);
    }

    pub fn set_lex_runs(&self, n: u64) {
        self.counters.lex_runs.store(n, Ordering::Relaxed);
    }

    pub fn quin_chunks(&self) -> u64 {
        self.counters.quin_chunks.load(Ordering::Relaxed)
    }

    pub fn lex_runs(&self) -> u64 {
        self.counters.lex_runs.load(Ordering::Relaxed)
    }

    /// Durable snapshot for a job directory (`progress.json`). Does not change
    /// `checkpoint.triples` — that is accepted work only.
    pub fn attach_progress_file(&self, path: std::path::PathBuf) {
        if let Ok(mut g) = self.progress_path.lock() {
            *g = Some(path);
        }
    }

    pub fn verbose(&self) -> u8 {
        self.verbose
    }

    pub fn note_sink(&self) -> Arc<dyn Fn(&str) + Send + Sync> {
        let me = self.clone();
        Arc::new(move |msg: &str| {
            if msg.contains("Lexicon spill: wrote") {
                me.add_lex_run();
            }
            if msg.contains("quin chunk") {
                me.add_quin_chunk();
                if let Some(ms) = parse_trailing_ms(msg) {
                    me.add_sort_ms(ms);
                }
                if let Some(p) = parse_sort_path(msg) {
                    if let Ok(mut g) = me.counters.last_sort_path.lock() {
                        *g = Some(p);
                    }
                }
            }
            let phase = if msg.contains("compact") {
                IngestPhase::Compacting
            } else if msg.to_ascii_lowercase().contains("lexicon") {
                IngestPhase::Lexicon
            } else {
                IngestPhase::Publishing
            };
            me.emit(phase, msg, Some(msg.to_string()));
        })
    }

    pub fn emit(&self, phase: IngestPhase, message: impl Into<String>, detail: Option<String>) {
        let snap = self.snapshot(phase, message.into(), detail);
        (self.sink)(&snap);
        self.write_progress_file(&snap);
        if let Ok(mut last) = self.last_emit.lock() {
            *last = Instant::now();
        }
        self.last_triples.store(snap.triples, Ordering::Relaxed);
    }

    fn write_progress_file(&self, snap: &IngestSnapshot) {
        let path = match self.progress_path.lock() {
            Ok(g) => g.clone(),
            Err(_) => return,
        };
        let Some(path) = path else {
            return;
        };
        let Ok(body) = serde_json::to_vec(snap) else {
            return;
        };
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, body).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }

    /// Throttled parse tick: every 2M triples or 2 seconds.
    pub fn maybe_tick(&self, phase: IngestPhase, triples: u64) {
        self.set_triples(triples);
        let last_n = self.last_triples.load(Ordering::Relaxed);
        let due_count = triples.saturating_sub(last_n) >= TICK_EVERY_TRIPLES;
        let due_time = self
            .last_emit
            .lock()
            .map(|t| t.elapsed().as_millis() >= TICK_EVERY_MS)
            .unwrap_or(true);
        if due_count || due_time {
            self.emit(phase, format!("{} triples", format_count(triples)), None);
        }
    }

    /// Skip-N replay tick. `replayed` is how many already-accepted triples this
    /// pass has walked; `target` is `checkpoint.triples`. Does not mean new work.
    pub fn maybe_tick_skip(&self, replayed: u64, target: u64) {
        self.set_skip_target(target);
        self.set_triples(replayed);
        let last_n = self.last_triples.load(Ordering::Relaxed);
        let due_count = replayed.saturating_sub(last_n) >= TICK_EVERY_TRIPLES;
        let due_time = self
            .last_emit
            .lock()
            .map(|t| t.elapsed().as_millis() >= TICK_EVERY_MS)
            .unwrap_or(true);
        if due_count || due_time {
            self.emit(
                IngestPhase::Skipping,
                format!("skip {} / {}", format_count(replayed), format_count(target)),
                None,
            );
        }
    }

    pub fn snapshot(
        &self,
        phase: IngestPhase,
        message: String,
        detail: Option<String>,
    ) -> IngestSnapshot {
        let triples = self.counters.triples.load(Ordering::Relaxed);
        let source_bytes = self.counters.source_bytes.load(Ordering::Relaxed);
        let bytes_read = self.counters.bytes_read.load(Ordering::Relaxed);
        let elapsed = self.started.elapsed();
        let secs = elapsed.as_secs_f64().max(0.001);
        let triples_per_sec = triples as f64 / secs;
        let mb_per_sec = (bytes_read as f64 / secs) / 1_048_576.0;
        let pct = if source_bytes > 0 {
            ((bytes_read as f64 / source_bytes as f64) * 100.0).min(100.0)
        } else {
            0.0
        };
        let eta_secs = if bytes_read > 0 && source_bytes > bytes_read && mb_per_sec > 0.0 {
            let remain = (source_bytes - bytes_read) as f64 / 1_048_576.0;
            Some(remain / mb_per_sec)
        } else {
            None
        };
        IngestSnapshot {
            phase,
            triples,
            source_bytes,
            bytes_read,
            interned_terms: self.counters.interned.load(Ordering::Relaxed),
            lex_runs: self.counters.lex_runs.load(Ordering::Relaxed),
            quin_chunks: self.counters.quin_chunks.load(Ordering::Relaxed),
            parse_ms: self.counters.parse_ms.load(Ordering::Relaxed),
            sort_ms: self.counters.sort_ms.load(Ordering::Relaxed),
            publish_ms: self.counters.publish_ms.load(Ordering::Relaxed),
            last_sort_path: self
                .counters
                .last_sort_path
                .lock()
                .ok()
                .and_then(|g| g.clone()),
            workers: self.workers,
            rss_bytes: sample_rss_bytes(),
            elapsed_ms: elapsed.as_millis() as u64,
            triples_per_sec,
            mb_per_sec,
            pct,
            eta_secs,
            message,
            detail,
            skip_target: self.counters.skip_target.load(Ordering::Relaxed),
        }
    }
}

/// Counts bytes as Rio reads the source, so `%` / ETA are honest.
pub struct CountingReader<R: Read> {
    inner: R,
    report: IngestReport,
}

impl<R: Read> CountingReader<R> {
    pub fn new(inner: R, report: IngestReport) -> Self {
        Self { inner, report }
    }
}

impl<R: Read> Read for CountingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            self.report.add_bytes_read(n as u64);
        }
        Ok(n)
    }
}

pub fn format_bytes(n: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    let x = n as f64;
    if x >= GIB {
        format!("{:.2} GiB", x / GIB)
    } else if x >= MIB {
        format!("{:.1} MiB", x / MIB)
    } else if x >= KIB {
        format!("{:.0} KiB", x / KIB)
    } else {
        format!("{n} B")
    }
}

pub fn format_count(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.2}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn parse_trailing_ms(msg: &str) -> Option<u64> {
    let ms = msg.rsplit_once(' ')?;
    let num = ms.1.strip_suffix("ms")?;
    num.parse().ok()
}

fn parse_sort_path(msg: &str) -> Option<String> {
    let rest = msg.split("path=").nth(1)?;
    let token = rest.split_whitespace().next()?;
    Some(token.to_string())
}

fn format_secs(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "—".into();
    }
    let s = secs.round() as u64;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    if h > 0 {
        format!("{h}h{m:02}m")
    } else if m > 0 {
        format!("{m}m{sec:02}s")
    } else {
        format!("{sec}s")
    }
}

fn sample_rss_bytes() -> Option<u64> {
    use sysinfo::{Pid, ProcessesToUpdate, System};
    let mut sys = System::new();
    let pid = Pid::from_u32(std::process::id());
    sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    sys.process(pid).map(|p| p.memory())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_line_includes_phase_and_percent() {
        let snap = IngestSnapshot {
            phase: IngestPhase::Parsing,
            triples: 2_000_000,
            source_bytes: 100,
            bytes_read: 25,
            interned_terms: 10,
            lex_runs: 1,
            quin_chunks: 2,
            workers: 6,
            rss_bytes: Some(64 * 1024 * 1024),
            elapsed_ms: 1000,
            triples_per_sec: 2_000_000.0,
            mb_per_sec: 10.0,
            pct: 25.0,
            eta_secs: Some(90.0),
            message: "2.00M triples".into(),
            detail: Some("lexrun_0.tmp".into()),
            parse_ms: 10,
            sort_ms: 20,
            publish_ms: 30,
            last_sort_path: Some("cpu".into()),
            skip_target: 0,
        };
        let line = snap.format_text(0);
        assert!(line.contains("25.0%"), "{line}");
        assert!(line.contains("parse"), "{line}");
        assert!(line.contains("2.00M"), "{line}");
        let verbose = snap.format_text(2);
        assert!(verbose.contains("lexrun_0.tmp"), "{verbose}");
        assert!(verbose.contains("workers 6"), "{verbose}");
    }

    #[test]
    fn skip_tick_does_not_look_like_zero_parse() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let slot = seen.clone();
        let report = IngestReport::new(1, move |s| slot.lock().unwrap().push(s.clone()));
        report.set_source_bytes(100);
        report.add_bytes_read(40);
        report.maybe_tick_skip(2_000_000, 1_808_000_000);
        let snaps = seen.lock().unwrap();
        assert_eq!(snaps.len(), 1);
        assert_eq!(snaps[0].phase, IngestPhase::Skipping);
        assert_eq!(snaps[0].skip_target, 1_808_000_000);
        assert_eq!(snaps[0].triples, 2_000_000);
        let line = snaps[0].format_text(0);
        assert!(line.contains("skip"), "{line}");
        assert!(line.contains("replayed"), "{line}");
    }

    #[test]
    fn progress_file_is_written_on_emit() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("progress.json");
        let report = IngestReport::silent();
        report.attach_progress_file(path.clone());
        report.set_source_bytes(10);
        report.emit(IngestPhase::Skipping, "skip 1 / 2", None);
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("\"skipping\""), "{body}");
    }

    #[test]
    fn report_ticks_and_records_triples() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let slot = seen.clone();
        let report = IngestReport::new(1, move |s| slot.lock().unwrap().push(s.clone()));
        report.set_source_bytes(1000);
        report.add_bytes_read(250);
        report.maybe_tick(IngestPhase::Parsing, 2_000_000);
        let snaps = seen.lock().unwrap();
        assert_eq!(snaps.len(), 1);
        assert_eq!(snaps[0].triples, 2_000_000);
        assert_eq!(snaps[0].phase, IngestPhase::Parsing);
    }
}
