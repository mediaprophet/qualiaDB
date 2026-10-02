//! Zero-Copy Binary Archive Format for DNS & SDN Records.
//!
//! Provides persistent cold-storage serialization and memory-mapped reader
//! for archived DNS `NQuin` statements, supporting long-term compliance,
//! DNSSEC provenance tracking, and forensic network audits.
//!
//! Conforms to QualiaDB Rule 0-A (zero heap in hot paths) and Rule 0-B (< 500 lines).

#![allow(dead_code)]

use crate::NQuin;
use std::io;

/// 4-byte magic identifier for Qualia DNS Archive files: "QDNS".
pub const QDNS_MAGIC: [u8; 4] = [0x51, 0x44, 0x4E, 0x53];

/// Fixed 64-byte Preamble for `.qdns` archive files.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DnsArchiveHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub reserved_u16: u16,
    pub record_count: u32,
    pub clock_start: u32,
    pub clock_end: u32,
    pub reserved: [u8; 44],
}

impl DnsArchiveHeader {
    pub const SIZE: usize = 64;

    pub fn new(record_count: u32, clock_start: u32, clock_end: u32) -> Self {
        Self {
            magic: QDNS_MAGIC,
            version: 1,
            reserved_u16: 0,
            record_count,
            clock_start,
            clock_end,
            reserved: [0u8; 44],
        }
    }

    pub fn validate(&self) -> io::Result<()> {
        if self.magic != QDNS_MAGIC {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid QDNS archive magic"));
        }
        if self.version != 1 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "unsupported QDNS archive version"));
        }
        Ok(())
    }
}

/// Write a batch of archived `NQuin` records to an output stream.
///
/// Writes the 64-byte header followed by contiguous 48-byte records.
/// Returns the total bytes written.
pub fn write_dns_archive<W: io::Write>(
    writer: &mut W,
    quins: &[NQuin],
    clock_start: u32,
    clock_end: u32,
) -> io::Result<usize> {
    let header = DnsArchiveHeader::new(quins.len() as u32, clock_start, clock_end);
    let header_bytes = unsafe {
        std::slice::from_raw_parts(&header as *const _ as *const u8, DnsArchiveHeader::SIZE)
    };
    writer.write_all(header_bytes)?;

    let record_bytes = bytemuck::cast_slice(quins);
    writer.write_all(record_bytes)?;

    Ok(DnsArchiveHeader::SIZE + record_bytes.len())
}

/// Read archived `NQuin` records from an input stream into caller-supplied `out_quins`.
///
/// Returns the header and count of records read. Zero heap allocation.
pub fn read_dns_archive<R: io::Read>(
    reader: &mut R,
    out_quins: &mut [NQuin],
) -> io::Result<(DnsArchiveHeader, usize)> {
    let mut header_buf = [0u8; DnsArchiveHeader::SIZE];
    reader.read_exact(&mut header_buf)?;

    let header: DnsArchiveHeader = unsafe {
        std::ptr::read_unaligned(header_buf.as_ptr() as *const DnsArchiveHeader)
    };
    header.validate()?;

    let to_read = core::cmp::min(header.record_count as usize, out_quins.len());
    let target_bytes = bytemuck::cast_slice_mut(&mut out_quins[..to_read]);
    reader.read_exact(target_bytes)?;

    Ok((header, to_read))
}

#[cfg(not(target_arch = "wasm32"))]
/// Zero-deserialization memory-mapped reader for `.qdns` archive files.
pub struct DnsArchiveMmap {
    mmap: memmap2::Mmap,
}

#[cfg(not(target_arch = "wasm32"))]
impl DnsArchiveMmap {
    /// Memory-maps an archive file, validating the magic preamble immediately.
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };

        if mmap.len() < DnsArchiveHeader::SIZE {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "file too small for QDNS header"));
        }

        let archive = Self { mmap };
        archive.header().validate()?;
        Ok(archive)
    }

    /// Casts the 64-byte preamble directly from mapped memory.
    pub fn header(&self) -> &DnsArchiveHeader {
        unsafe { &*(self.mmap.as_ptr() as *const DnsArchiveHeader) }
    }

    /// Returns a zero-copy slice of all `NQuin` records in the archive.
    pub fn records(&self) -> &[NQuin] {
        let payload = &self.mmap[DnsArchiveHeader::SIZE..];
        let record_size = std::mem::size_of::<NQuin>();
        let count = payload.len() / record_size;
        unsafe {
            std::slice::from_raw_parts(payload.as_ptr() as *const NQuin, count)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::dns::quin_records::*;
    use crate::PermissiveRoutingLane;
    use std::io::Cursor;

    #[test]
    fn header_size_and_alignment() {
        assert_eq!(std::mem::size_of::<DnsArchiveHeader>(), 64);
    }

    #[test]
    fn archive_roundtrip_stream() {
        let q1 = encode_a_record("arch1.lan", [192, 168, 1, 10], 3600, PermissiveRoutingLane::PassthroughStandard);
        let q2 = encode_aaaa_record("arch2.lan", [0x20; 16], 7200, PermissiveRoutingLane::EnforcePermissiveCommons);
        let quins = [q1, q2];

        let mut buf = Vec::new();
        let bytes_written = write_dns_archive(&mut buf, &quins, 100, 200).unwrap();
        assert_eq!(bytes_written, 64 + 48 * 2);

        let mut cursor = Cursor::new(buf);
        let mut out_quins = [NQuin::default(); 4];
        let (header, count) = read_dns_archive(&mut cursor, &mut out_quins).unwrap();

        assert_eq!(header.magic, QDNS_MAGIC);
        assert_eq!(header.version, 1);
        assert_eq!(header.record_count, 2);
        assert_eq!(header.clock_start, 100);
        assert_eq!(header.clock_end, 200);
        assert_eq!(count, 2);

        assert_eq!(out_quins[0].subject, q1.subject);
        assert_eq!(out_quins[1].subject, q2.subject);
        assert!(out_quins[0].verify_ecc_parity());
        assert!(out_quins[1].verify_ecc_parity());
    }

    #[test]
    fn archive_mmap_file_roundtrip() {
        use tempfile::NamedTempFile;

        let q1 = encode_a_record("file.lan", [1, 2, 3, 4], 300, PermissiveRoutingLane::PassthroughStandard);
        let quins = [q1];

        let mut temp_file = NamedTempFile::new().unwrap();
        write_dns_archive(&mut temp_file, &quins, 50, 150).unwrap();

        let mmap_archive = DnsArchiveMmap::open(temp_file.path()).unwrap();
        assert_eq!(mmap_archive.header().record_count, 1);
        assert_eq!(mmap_archive.records().len(), 1);
        assert_eq!(mmap_archive.records()[0].subject, q1.subject);
        assert!(mmap_archive.records()[0].verify_ecc_parity());
    }
}
