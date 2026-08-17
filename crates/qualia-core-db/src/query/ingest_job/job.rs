//! On-disk job specification, checkpoint, and source attestation.

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::source::{IngestEncoding, IngestRdfFormat, IngestSourceKind, WINDOW_BYTES};
use crate::query::ingest::IngestMode;

pub const JOB_SPEC: &str = "job.json";
pub const JOB_CHECKPOINT: &str = "checkpoint.json";
pub const JOB_ATTESTATION: &str = "source-attestation.json";
pub const JOB_WINDOWS: &str = "windows.sha256";
pub const JOB_PROGRESS: &str = "progress.json";
pub const JOB_RUNS: &str = "runs";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestJobPhase {
    Starting,
    Parsing,
    Sorting,
    Publishing,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IngestJobSpec {
    pub schema: u32,
    pub source: IngestSourceKind,
    pub encoding: IngestEncoding,
    pub format: IngestRdfFormat,
    pub mode: String,
    pub segment_mib: Option<u64>,
    pub output: String,
    pub created_unix: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IngestCheckpoint {
    pub schema: u32,
    pub phase: IngestJobPhase,
    pub triples: u64,
    pub uncompressed_bytes: u64,
    pub wire_bytes: u64,
    pub quin_chunks: u64,
    pub lex_runs: u64,
    pub last_error: Option<String>,
    pub updated_unix: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceAttestation {
    pub locator: String,
    pub encoding: IngestEncoding,
    pub format: IngestRdfFormat,
    pub uncompressed_bytes: u64,
    pub wire_bytes: Option<u64>,
    pub triples: u64,
    /// SHA-256 of the full decompressed source. Present only after a complete
    /// single-pass hash (no crash mid-stream without resume).
    pub uncompressed_sha256_hex: Option<String>,
    /// SHA-256 of the on-the-wire bytes (compressed if the server sent gzip/zstd).
    pub wire_sha256_hex: Option<String>,
    /// SHA-256 of the concatenation of 16 MiB window hashes. Always available
    /// after at least one full window; used to verify a prefix of an incomplete job.
    pub window_commitment_hex: Option<String>,
    pub window_bytes: u64,
    pub window_count: u64,
    pub etag: Option<String>,
    pub content_length: Option<u64>,
    pub retrieved_unix: u64,
}

impl SourceAttestation {
    pub fn verify_story(&self) -> &'static str {
        if self.uncompressed_sha256_hex.is_some() {
            "re-stream the locator (file or URL), decompress the same way, SHA-256 the bytes Rio saw, compare uncompressed_sha256_hex. The source file does not need to stay on disk."
        } else if self.window_commitment_hex.is_some() {
            "job did not finish a single-pass digest. Re-stream and compare 16 MiB window hashes to windows.sha256; full file SHA-256 is unknown until a complete pass."
        } else {
            "no digest recorded yet — nothing to verify against."
        }
    }
}

#[derive(Clone, Debug)]
pub struct IngestJob {
    pub dir: PathBuf,
    pub spec: IngestJobSpec,
    pub checkpoint: IngestCheckpoint,
}

impl IngestJob {
    pub fn create(
        dir: PathBuf,
        source: IngestSourceKind,
        encoding: IngestEncoding,
        format: IngestRdfFormat,
        mode: IngestMode,
        segment_mib: Option<u64>,
        output: &Path,
    ) -> io::Result<Self> {
        fs::create_dir_all(dir.join(JOB_RUNS))?;
        let spec = IngestJobSpec {
            schema: 1,
            source,
            encoding,
            format,
            mode: match mode {
                IngestMode::Complete => "complete".into(),
                IngestMode::StripLiterals => "strip_literals".into(),
            },
            segment_mib,
            output: output.to_string_lossy().into_owned(),
            created_unix: unix_now(),
        };
        let checkpoint = IngestCheckpoint {
            schema: 1,
            phase: IngestJobPhase::Starting,
            triples: 0,
            uncompressed_bytes: 0,
            wire_bytes: 0,
            quin_chunks: 0,
            lex_runs: 0,
            last_error: None,
            updated_unix: unix_now(),
        };
        let job = Self {
            dir,
            spec,
            checkpoint,
        };
        job.write_spec()?;
        job.write_checkpoint()?;
        Ok(job)
    }

    pub fn open(dir: PathBuf) -> io::Result<Self> {
        let spec = read_json(&dir.join(JOB_SPEC))?;
        let checkpoint = read_json(&dir.join(JOB_CHECKPOINT))?;
        Ok(Self {
            dir,
            spec,
            checkpoint,
        })
    }

    pub fn runs_dir(&self) -> PathBuf {
        self.dir.join(JOB_RUNS)
    }

    pub fn is_complete(&self) -> bool {
        self.checkpoint.phase == IngestJobPhase::Complete
    }

    pub fn write_spec(&self) -> io::Result<()> {
        write_json_atomic(&self.dir.join(JOB_SPEC), &self.spec)
    }

    pub fn write_checkpoint(&self) -> io::Result<()> {
        write_json_atomic(&self.dir.join(JOB_CHECKPOINT), &self.checkpoint)
    }

    pub fn set_phase(&mut self, phase: IngestJobPhase) -> io::Result<()> {
        self.checkpoint.phase = phase;
        self.checkpoint.updated_unix = unix_now();
        self.write_checkpoint()
    }

    pub fn record_progress(
        &mut self,
        triples: u64,
        uncompressed_bytes: u64,
        quin_chunks: u64,
        lex_runs: u64,
    ) -> io::Result<()> {
        self.checkpoint.phase = IngestJobPhase::Parsing;
        self.checkpoint.triples = triples;
        self.checkpoint.uncompressed_bytes = uncompressed_bytes;
        self.checkpoint.quin_chunks = quin_chunks;
        self.checkpoint.lex_runs = lex_runs;
        self.checkpoint.updated_unix = unix_now();
        self.write_checkpoint()
    }

    pub fn mark_failed(&mut self, err: impl ToString) -> io::Result<()> {
        self.checkpoint.phase = IngestJobPhase::Failed;
        self.checkpoint.last_error = Some(err.to_string());
        self.checkpoint.updated_unix = unix_now();
        self.write_checkpoint()
    }

    pub fn write_attestation(&self, att: &SourceAttestation) -> io::Result<()> {
        write_json_atomic(&self.dir.join(JOB_ATTESTATION), att)
    }

    pub fn attestation_path(&self) -> PathBuf {
        self.dir.join(JOB_ATTESTATION)
    }

    pub fn windows_path(&self) -> PathBuf {
        self.dir.join(JOB_WINDOWS)
    }

    pub fn load_attestation(&self) -> io::Result<Option<SourceAttestation>> {
        let path = self.attestation_path();
        if !path.is_file() {
            return Ok(None);
        }
        Ok(Some(read_json(&path)?))
    }
}

pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn write_json_atomic(path: &Path, value: &impl Serialize) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let mut f = File::create(&tmp)?;
    serde_json::to_writer_pretty(&mut f, value)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    f.write_all(b"\n")?;
    f.flush()?;
    fs::rename(tmp, path)
}

pub fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> io::Result<T> {
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn append_window_hash(path: &Path, hash: &[u8; 32]) -> io::Result<()> {
    use std::io::Write as _;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{}", hex_encode(hash))
}

pub fn read_window_hashes(path: &Path) -> io::Result<Vec<[u8; 32]>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        out.push(hex_decode32(line)?);
    }
    Ok(out)
}

pub fn window_commitment(windows: &[[u8; 32]]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update((WINDOW_BYTES as u64).to_le_bytes());
    h.update((windows.len() as u64).to_le_bytes());
    for w in windows {
        h.update(w);
    }
    h.finalize().into()
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

pub fn hex_decode32(s: &str) -> io::Result<[u8; 32]> {
    let s = s.trim();
    if s.len() != 64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected 64 hex chars",
        ));
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        let hi = hex_nibble(s.as_bytes()[i * 2])?;
        let lo = hex_nibble(s.as_bytes()[i * 2 + 1])?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::ingest::IngestMode;
    use crate::query::ingest_job::source::{IngestEncoding, IngestRdfFormat, IngestSourceKind};

    #[test]
    fn job_round_trip_and_window_commitment() {
        let dir = tempfile::TempDir::new().unwrap();
        let job = IngestJob::create(
            dir.path().to_path_buf(),
            IngestSourceKind::File {
                path: "x.nt".into(),
            },
            IngestEncoding::Identity,
            IngestRdfFormat::NTriples,
            IngestMode::Complete,
            Some(512),
            Path::new("out.q42"),
        )
        .unwrap();
        assert!(job.runs_dir().is_dir());
        append_window_hash(&job.windows_path(), &[1u8; 32]).unwrap();
        append_window_hash(&job.windows_path(), &[2u8; 32]).unwrap();
        let wins = read_window_hashes(&job.windows_path()).unwrap();
        assert_eq!(wins.len(), 2);
        let c = window_commitment(&wins);
        assert_ne!(c, [0u8; 32]);
        let opened = IngestJob::open(dir.path().to_path_buf()).unwrap();
        assert_eq!(opened.spec.output, "out.q42");
    }
}

fn hex_nibble(b: u8) -> io::Result<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(io::Error::new(io::ErrorKind::InvalidData, "invalid hex")),
    }
}
