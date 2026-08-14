//! Transport-neutral, caller-buffered byte-range access.
//!
//! HTTP gateways and IPFS partial-CAR retrieval must obey this same exact-range
//! contract. The local file adapter is both useful itself and the reference
//! implementation for response-length and boundary validation.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

/// A half-open byte interval, represented without an end-offset overflow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42ByteRange {
    pub offset: u64,
    pub length: usize,
}

impl Q42ByteRange {
    pub fn end(self) -> io::Result<u64> {
        self.offset
            .checked_add(self.length as u64)
            .ok_or_else(|| invalid("Q42 byte range overflows u64"))
    }

    pub fn validate_for(self, source_length: u64) -> io::Result<()> {
        if self.end()? > source_length {
            return Err(invalid("Q42 byte range exceeds source length"));
        }
        Ok(())
    }
}

/// Exact, caller-buffered random access. Implementations must fail if the
/// source returns a short response; partial data is never treated as success.
pub trait Q42RangeSource {
    fn length(&self) -> io::Result<u64>;
    fn read_range_into(&self, range: Q42ByteRange, out: &mut [u8]) -> io::Result<()>;
}

/// Native local-file implementation. The mutex serialises seek/read pairs;
/// high-concurrency remote adapters should instead use independent requests.
pub struct LocalFileRangeSource {
    length: u64,
    file: Mutex<File>,
}

impl LocalFileRangeSource {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let length = file.metadata()?.len();
        Ok(Self {
            length,
            file: Mutex::new(file),
        })
    }
}

impl Q42RangeSource for LocalFileRangeSource {
    fn length(&self) -> io::Result<u64> {
        Ok(self.length)
    }

    fn read_range_into(&self, range: Q42ByteRange, out: &mut [u8]) -> io::Result<()> {
        if out.len() != range.length {
            return Err(invalid(
                "Q42 range output buffer length does not match request",
            ));
        }
        range.validate_for(self.length)?;
        let mut file = self
            .file
            .lock()
            .map_err(|_| io::Error::other("Q42 range source lock poisoned"))?;
        file.seek(SeekFrom::Start(range.offset))?;
        file.read_exact(out)
    }
}

/// Validate an HTTP/IPFS adapter response before it is accepted by a reader.
/// `content_range_start` is the start declared by `Content-Range` or the
/// verified CAR block offset; `returned_len` is its actual payload size.
pub fn validate_exact_range_response(
    requested: Q42ByteRange,
    source_length: u64,
    content_range_start: u64,
    returned_len: usize,
) -> io::Result<()> {
    requested.validate_for(source_length)?;
    if content_range_start != requested.offset || returned_len != requested.length {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "range response does not exactly match the requested Q42 bytes",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn local_source_requires_exact_bounded_ranges() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"0123456789").unwrap();
        let source = LocalFileRangeSource::open(file.path()).unwrap();
        let mut out = [0u8; 4];
        source
            .read_range_into(
                Q42ByteRange {
                    offset: 3,
                    length: 4,
                },
                &mut out,
            )
            .unwrap();
        assert_eq!(&out, b"3456");
        assert!(source
            .read_range_into(
                Q42ByteRange {
                    offset: 9,
                    length: 2
                },
                &mut [0; 2]
            )
            .is_err());
        assert!(validate_exact_range_response(
            Q42ByteRange {
                offset: 3,
                length: 4
            },
            10,
            4,
            4
        )
        .is_err());
    }
}
