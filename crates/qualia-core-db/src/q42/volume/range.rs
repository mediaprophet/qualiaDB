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

/// Native HTTP byte-range source for trusted gateways. It never accepts a
/// `200 OK` full-body fallback: large Q42 access must stay range-bounded.
#[cfg(not(target_arch = "wasm32"))]
pub struct HttpRangeSource {
    client: reqwest::blocking::Client,
    url: reqwest::Url,
    length: u64,
}

#[cfg(not(target_arch = "wasm32"))]
impl HttpRangeSource {
    pub fn new(url: &str, length: u64) -> io::Result<Self> {
        if length == 0 {
            return Err(invalid("Q42 HTTP source length must be non-zero"));
        }
        let url = reqwest::Url::parse(url)
            .map_err(|error| invalid(format!("invalid Q42 HTTP URL: {error}")))?;
        if url.scheme() != "https" && url.scheme() != "http" {
            return Err(invalid("Q42 HTTP source must use http or https"));
        }
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|error| io::Error::other(format!("build Q42 HTTP client: {error}")))?;
        Ok(Self {
            client,
            url,
            length,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Q42RangeSource for HttpRangeSource {
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
        let end = range
            .end()?
            .checked_sub(1)
            .ok_or_else(|| invalid("Q42 HTTP range may not be empty"))?;
        let response = self
            .client
            .get(self.url.clone())
            .header(
                reqwest::header::RANGE,
                format!("bytes={}-{}", range.offset, end),
            )
            .send()
            .map_err(|error| io::Error::other(format!("fetch Q42 range: {error}")))?;
        if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Q42 gateway did not return HTTP 206 Partial Content",
            ));
        }
        let header = response
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Q42 range response has no valid Content-Range",
                )
            })?;
        let (start, returned_end, total) = parse_content_range(header)?;
        if total != self.length
            || returned_end
                .checked_add(1)
                .and_then(|value| value.checked_sub(start))
                .and_then(|value| usize::try_from(value).ok())
                != Some(range.length)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Q42 Content-Range does not match catalog length",
            ));
        }
        let mut response = response;
        response
            .read_exact(out)
            .map_err(|error| io::Error::other(format!("read Q42 range body: {error}")))?;
        let mut extra = [0u8; 1];
        if response
            .read(&mut extra)
            .map_err(|error| io::Error::other(format!("read Q42 range tail: {error}")))?
            != 0
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Q42 range response contains extra bytes",
            ));
        }
        validate_exact_range_response(range, self.length, start, out.len())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ipfs_gateway_range_source(
    gateway: &str,
    cid: &str,
    length: u64,
) -> io::Result<HttpRangeSource> {
    if cid.is_empty() || !cid.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return Err(invalid("IPFS CID must be a non-empty base32/base58 token"));
    }
    let gateway = gateway.trim_end_matches('/');
    HttpRangeSource::new(&format!("{gateway}/ipfs/{cid}"), length)
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_content_range(value: &str) -> io::Result<(u64, u64, u64)> {
    let body = value
        .strip_prefix("bytes ")
        .ok_or_else(|| invalid("unsupported Content-Range unit"))?;
    let (range, total) = body
        .split_once('/')
        .ok_or_else(|| invalid("malformed Content-Range"))?;
    let (start, end) = range
        .split_once('-')
        .ok_or_else(|| invalid("malformed Content-Range interval"))?;
    let start = start
        .parse()
        .map_err(|_| invalid("invalid Content-Range start"))?;
    let end = end
        .parse()
        .map_err(|_| invalid("invalid Content-Range end"))?;
    let total = total
        .parse()
        .map_err(|_| invalid("invalid Content-Range total"))?;
    if start > end || end >= total {
        return Err(invalid("Content-Range lies outside source"));
    }
    Ok((start, end, total))
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

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn content_range_parser_rejects_mismatched_or_invalid_bounds() {
        assert_eq!(parse_content_range("bytes 5-8/10").unwrap(), (5, 8, 10));
        assert!(parse_content_range("bytes 8-5/10").is_err());
        assert!(parse_content_range("bytes 5-10/10").is_err());
        assert!(parse_content_range("items 5-8/10").is_err());
    }
}
