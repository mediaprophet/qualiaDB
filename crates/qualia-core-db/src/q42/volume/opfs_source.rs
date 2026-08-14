//! WASM/OPFS caller-buffered Q42 range source.
//!
//! JS or native OPFS fills a caller-owned buffer; Rust never maps the whole
//! file on the hot path. A byte slice is accepted when the host already holds
//! the bytes (tests, verified CAR entity after cold construction). The
//! callback adapter is the browser path: OPFS/`File` reads into `out` and
//! returns the filled count.
//!
//! Local CARv1 verification is a separate cold construction. It calls
//! [`decode_and_verify_car`] and then exposes concatenated raw (0x55) UnixFS
//! entity bytes through the same [`Q42RangeSource`] contract. Q42 offsets
//! address that entity, never the CAR envelope. This module does not contact
//! a public IPFS gateway.

use std::io;
use std::path::Path;

use super::car::{decode_and_verify_car, VerifiedCarBlock};
use super::cid::CidSha256;
use super::range::{validate_exact_range_response, Q42ByteRange, Q42RangeSource};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn short_fill() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "OPFS range fill did not return exactly the requested Q42 bytes",
    )
}

/// Caller-buffered view of host-supplied bytes (slice or owned entity).
pub struct OpfsSliceRangeSource<B> {
    bytes: B,
}

impl<B> OpfsSliceRangeSource<B> {
    pub fn new(bytes: B) -> Self {
        Self { bytes }
    }
}

impl<B: AsRef<[u8]>> OpfsSliceRangeSource<B> {
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_ref()
    }
}

impl<B: AsRef<[u8]>> Q42RangeSource for OpfsSliceRangeSource<B> {
    fn length(&self) -> io::Result<u64> {
        Ok(self.bytes.as_ref().len() as u64)
    }

    fn read_range_into(&self, range: Q42ByteRange, out: &mut [u8]) -> io::Result<()> {
        copy_exact_slice(self.bytes.as_ref(), range, out)
    }
}

/// Host fill callback: `(offset, length, buf) -> bytes_written`.
///
/// The callback must write exactly `length` bytes into `buf`. A short fill is
/// never treated as success. Rust does not open or map the backing file.
pub struct OpfsCallbackRangeSource<F> {
    length: u64,
    fill: F,
}

impl<F> OpfsCallbackRangeSource<F>
where
    F: Fn(u64, usize, &mut [u8]) -> io::Result<usize>,
{
    pub fn new(length: u64, fill: F) -> Self {
        Self { length, fill }
    }
}

impl<F> Q42RangeSource for OpfsCallbackRangeSource<F>
where
    F: Fn(u64, usize, &mut [u8]) -> io::Result<usize>,
{
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
        let filled = (self.fill)(range.offset, range.length, out)?;
        if filled != range.length {
            return Err(short_fill());
        }
        validate_exact_range_response(range, self.length, range.offset, filled)
    }
}

/// Verified UnixFS entity reconstructed from a local CARv1 file.
pub type VerifiedCarRangeSource = OpfsSliceRangeSource<Vec<u8>>;

/// Decode and CID-verify a CARv1 buffer, then expose raw-leaf entity bytes.
pub fn verify_car_bytes_as_q42_source(car: &[u8]) -> io::Result<VerifiedCarRangeSource> {
    let blocks = decode_and_verify_car(car)?;
    Ok(OpfsSliceRangeSource::new(concatenate_raw_leaves(&blocks)))
}

/// Read a local CARv1 path, CID-verify every block, expose entity bytes.
///
/// Cold construction: the CAR is loaded to run [`decode_and_verify_car`]. The
/// returned source then serves exact ranges of the UnixFS entity. This is not
/// a live public-gateway check.
pub fn verify_local_car_as_q42_source(path: &Path) -> io::Result<VerifiedCarRangeSource> {
    let car = std::fs::read(path)?;
    verify_car_bytes_as_q42_source(&car)
}

fn concatenate_raw_leaves(blocks: &[VerifiedCarBlock]) -> Vec<u8> {
    let mut total = 0usize;
    for block in blocks {
        if block.cid.codec == CidSha256::RAW {
            total = total.saturating_add(block.data.len());
        }
    }
    let mut entity = Vec::with_capacity(total);
    for block in blocks {
        if block.cid.codec == CidSha256::RAW {
            entity.extend_from_slice(&block.data);
        }
    }
    entity
}

fn copy_exact_slice(bytes: &[u8], range: Q42ByteRange, out: &mut [u8]) -> io::Result<()> {
    if out.len() != range.length {
        return Err(invalid(
            "Q42 range output buffer length does not match request",
        ));
    }
    let source_length = bytes.len() as u64;
    range.validate_for(source_length)?;
    let start = usize::try_from(range.offset)
        .map_err(|_| invalid("Q42 range offset exceeds platform"))?;
    let end = start
        .checked_add(range.length)
        .ok_or_else(|| invalid("Q42 range overflows platform usize"))?;
    out.copy_from_slice(&bytes[start..end]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42_volume::write_unified_volume;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
    use crate::NQuin;
    use std::collections::HashMap;
    use std::io::Write;

    use super::super::car::encode_raw_car;
    use super::super::range::LocalFileRangeSource;
    use tempfile::NamedTempFile;

    fn tiny_unified_volume() -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        write_unified_volume(
            file.path(),
            &HashMap::new(),
            &[(3, 3)],
            &[vec![NQuin {
                subject: 1,
                predicate: 2,
                object: 3,
                context: 0,
                metadata: 0,
                parity: 0,
            }]],
        )
        .unwrap();
        file
    }

    fn read_all<S: Q42RangeSource>(source: &S) -> Vec<u8> {
        let length = usize::try_from(source.length().unwrap()).unwrap();
        let mut out = vec![0u8; length];
        source
            .read_range_into(
                Q42ByteRange {
                    offset: 0,
                    length,
                },
                &mut out,
            )
            .unwrap();
        out
    }

    fn assert_same_ranges<A: Q42RangeSource, B: Q42RangeSource>(left: &A, right: &B) {
        let left_len = left.length().unwrap();
        let right_len = right.length().unwrap();
        assert_eq!(left_len, right_len);
        assert_eq!(read_all(left), read_all(right));

        let mid_len = 16.min(left_len as usize);
        if left_len >= 8 && mid_len > 0 {
            let mut from_left = vec![0u8; mid_len];
            let mut from_right = vec![0u8; mid_len];
            let range = Q42ByteRange {
                offset: 4,
                length: mid_len,
            };
            left.read_range_into(range, &mut from_left).unwrap();
            right.read_range_into(range, &mut from_right).unwrap();
            assert_eq!(from_left, from_right);
        }

        let overflow = Q42ByteRange {
            offset: left_len,
            length: 1,
        };
        assert!(left.read_range_into(overflow, &mut [0u8; 1]).is_err());
        assert!(right.read_range_into(overflow, &mut [0u8; 1]).is_err());
    }

    #[test]
    fn slice_source_matches_local_file_on_tiny_unified_volume() {
        let file = tiny_unified_volume();
        let disk = LocalFileRangeSource::open(file.path()).unwrap();
        let bytes = std::fs::read(file.path()).unwrap();
        let slice = OpfsSliceRangeSource::new(bytes.as_slice());
        assert_same_ranges(&disk, &slice);
        assert_eq!(slice.as_bytes(), bytes.as_slice());
    }

    #[test]
    fn callback_source_matches_local_file_on_tiny_unified_volume() {
        let file = tiny_unified_volume();
        let disk = LocalFileRangeSource::open(file.path()).unwrap();
        let bytes = std::fs::read(file.path()).unwrap();
        let callback = OpfsCallbackRangeSource::new(bytes.len() as u64, |offset, len, buf| {
            copy_exact_slice(
                &bytes,
                Q42ByteRange {
                    offset,
                    length: len,
                },
                buf,
            )?;
            Ok(len)
        });
        assert_same_ranges(&disk, &callback);
    }

    #[test]
    fn verified_local_car_exposes_the_same_entity_as_the_q42_file() {
        let file = tiny_unified_volume();
        let entity = std::fs::read(file.path()).unwrap();
        let car = encode_raw_car(&[entity.as_slice()]);
        let mut car_file = NamedTempFile::new().unwrap();
        car_file.write_all(&car).unwrap();
        car_file.flush().unwrap();

        let source = verify_local_car_as_q42_source(car_file.path()).unwrap();
        assert_eq!(source.as_bytes(), entity.as_slice());

        let disk = LocalFileRangeSource::open(file.path()).unwrap();
        assert_same_ranges(&disk, &source);
    }

    #[test]
    fn tampered_local_car_is_rejected() {
        let mut car = encode_raw_car(&[b"intact-q42-entity".as_slice()]);
        *car.last_mut().unwrap() ^= 0xff;
        let mut car_file = NamedTempFile::new().unwrap();
        car_file.write_all(&car).unwrap();
        car_file.flush().unwrap();
        assert!(verify_local_car_as_q42_source(car_file.path()).is_err());
        assert!(verify_car_bytes_as_q42_source(&car).is_err());
    }

    #[test]
    fn short_callback_fill_is_rejected() {
        let source = OpfsCallbackRangeSource::new(8, |_offset, _len, buf| {
            buf[..2].copy_from_slice(b"ab");
            Ok(2)
        });
        assert!(source
            .read_range_into(
                Q42ByteRange {
                    offset: 0,
                    length: 4
                },
                &mut [0u8; 4]
            )
            .is_err());
    }

    #[test]
    fn slice_and_callback_hot_reads_are_zero_heap() {
        let file = tiny_unified_volume();
        let bytes = std::fs::read(file.path()).unwrap();
        let slice = OpfsSliceRangeSource::new(bytes.as_slice());
        let callback = OpfsCallbackRangeSource::new(bytes.len() as u64, |offset, len, buf| {
            copy_exact_slice(
                &bytes,
                Q42ByteRange {
                    offset,
                    length: len,
                },
                buf,
            )?;
            Ok(len)
        });
        let mut slice_buf = [0u8; 8];
        let mut callback_buf = [0u8; 8];
        let range = Q42ByteRange {
            offset: 0,
            length: 8,
        };
        assert_zero_alloc("q42_opfs_slice_range_read", || {
            slice.read_range_into(range, &mut slice_buf).unwrap();
        });
        assert_zero_alloc("q42_opfs_callback_range_read", || {
            callback
                .read_range_into(range, &mut callback_buf)
                .unwrap();
        });
        assert_eq!(slice_buf, callback_buf);
    }
}
