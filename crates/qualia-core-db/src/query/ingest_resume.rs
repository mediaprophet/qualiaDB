//! Format-aware resume: newline skip for NT/NQ, prefix+offset for Turtle/TriG/N3.
//! Gzip/XML/JSON-LD stay skip-N (cannot seek honestly).

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::query::ingest_job::IngestRdfFormat;

pub const RESUME_FILE: &str = "resume.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResumeCursor {
    pub schema: u32,
    pub format: IngestRdfFormat,
    pub byte_offset: u64,
    pub triples: u64,
    pub prolog: String,
    pub seekable: bool,
}

impl ResumeCursor {
    pub fn load(dir: &Path) -> io::Result<Option<Self>> {
        let path = dir.join(RESUME_FILE);
        if !path.is_file() {
            return Ok(None);
        }
        Ok(Some(
            serde_json::from_slice(&std::fs::read(path)?)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        ))
    }

    pub fn store(&self, dir: &Path) -> io::Result<()> {
        let path = dir.join(RESUME_FILE);
        let tmp = path.with_extension("json.tmp");
        let body =
            serde_json::to_vec(self).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        std::fs::write(&tmp, body)?;
        std::fs::rename(tmp, path)
    }
}

pub fn format_is_line_oriented(fmt: IngestRdfFormat) -> bool {
    matches!(fmt, IngestRdfFormat::NTriples | IngestRdfFormat::NQuads)
}

pub fn format_uses_prolog(fmt: IngestRdfFormat) -> bool {
    matches!(
        fmt,
        IngestRdfFormat::Turtle | IngestRdfFormat::TriG | IngestRdfFormat::N3
    )
}

pub fn format_can_seek(fmt: IngestRdfFormat) -> bool {
    format_is_line_oriented(fmt) || format_uses_prolog(fmt)
}

/// Skip `n` newline-terminated records. Returns (bytes consumed, unread tail
/// after the nth newline so the next parser does not lose triples).
pub fn skip_newlines<R: Read>(
    reader: &mut R,
    n: u64,
    tick: impl Fn(u64, u64),
) -> io::Result<(u64, Vec<u8>)> {
    if n == 0 {
        return Ok((0, Vec::new()));
    }
    let mut buf = [0u8; 64 * 1024];
    let mut lines = 0u64;
    let mut bytes = 0u64;
    let mut carry = Vec::new();
    while lines < n {
        let k = reader.read(&mut buf)?;
        if k == 0 {
            break;
        }
        for (i, &b) in buf[..k].iter().enumerate() {
            bytes += 1;
            if b == b'\n' {
                lines += 1;
                if lines % 2_000_000 == 0 {
                    tick(lines, bytes);
                }
                if lines == n {
                    carry.extend_from_slice(&buf[i + 1..k]);
                    tick(lines, bytes);
                    return Ok((bytes, carry));
                }
            }
        }
    }
    tick(lines, bytes);
    Ok((bytes, carry))
}

/// Counts bytes the inner reader actually delivers (parser-facing).
pub struct ByteCounter<R> {
    inner: R,
    count: Arc<AtomicU64>,
}

impl<R> ByteCounter<R> {
    pub fn new(inner: R, count: Arc<AtomicU64>) -> Self {
        Self { inner, count }
    }

    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }
}

impl<R: Read> Read for ByteCounter<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            self.count.fetch_add(n as u64, Ordering::Relaxed);
        }
        Ok(n)
    }
}

/// Copies `@prefix` / `@base` / `PREFIX` / `BASE` statements into `prolog`.
pub struct PrefixCapture<R> {
    inner: R,
    pub prolog: String,
    line: Vec<u8>,
    finished_header: bool,
}

impl<R> PrefixCapture<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            prolog: String::new(),
            line: Vec::new(),
            finished_header: false,
        }
    }
}

impl<R: Read> Read for PrefixCapture<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n == 0 || self.finished_header {
            return Ok(n);
        }
        for &b in &buf[..n] {
            self.line.push(b);
            if b == b'\n' {
                consider_line(&mut self.prolog, &self.line, &mut self.finished_header);
                self.line.clear();
            }
        }
        Ok(n)
    }
}

fn consider_line(prolog: &mut String, line: &[u8], finished: &mut bool) {
    let s = std::str::from_utf8(line).unwrap_or("");
    let t = s.trim_start();
    if t.is_empty() || t.starts_with('#') {
        if !*finished {
            prolog.push_str(s);
        }
        return;
    }
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("@prefix")
        || lower.starts_with("@base")
        || lower.starts_with("prefix ")
        || lower.starts_with("base ")
    {
        prolog.push_str(s);
        return;
    }
    *finished = true;
}

/// Open a seekable file, optionally resume at `cursor`.
pub fn open_resumed_file(
    path: &Path,
    cursor: Option<&ResumeCursor>,
) -> io::Result<(Box<dyn Read + Send>, u64, String)> {
    let mut file = File::open(path)?;
    let Some(cur) = cursor.filter(|c| c.seekable && c.byte_offset > 0) else {
        return Ok((Box::new(file), 0, String::new()));
    };
    file.seek(SeekFrom::Start(cur.byte_offset))?;
    if cur.prolog.is_empty() {
        return Ok((Box::new(file), cur.byte_offset, String::new()));
    }
    let header = std::io::Cursor::new(cur.prolog.clone().into_bytes());
    Ok((
        Box::new(header.chain(file)),
        cur.byte_offset,
        cur.prolog.clone(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_newlines_stops_after_n() {
        let src = b"a\nb\nc\nd\n";
        let mut cur = std::io::Cursor::new(&src[..]);
        let (bytes, tail) = skip_newlines(&mut cur, 2, |_, _| {}).unwrap();
        assert_eq!(bytes, 4);
        let mut rest = String::from_utf8(tail).unwrap();
        cur.read_to_string(&mut rest).unwrap();
        assert_eq!(rest, "c\nd\n");
    }

    #[test]
    fn prefix_capture_keeps_prefixes_only() {
        let src = b"@prefix ex: <http://ex/> .\n# c\nex:a ex:b ex:c .\n";
        let mut cap = PrefixCapture::new(&src[..]);
        let mut out = Vec::new();
        cap.read_to_end(&mut out).unwrap();
        assert!(cap.prolog.contains("@prefix ex:"));
        assert!(!cap.prolog.contains("ex:a ex:b"));
    }
}
