//! Public SDK boundary tests. Fixtures are bytes, not new production assets.
use qualia_core_db::q42::m64_matter::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Counting;
thread_local! { static ALLOCS: Cell<Option<usize>> = const { Cell::new(None) }; }
fn count_allocation() {
    let _ = ALLOCS.try_with(|count| {
        if let Some(value) = count.get() { count.set(Some(value + 1)); }
    });
}
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count_allocation();
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn limits() -> M64ReadLimits {
    M64ReadLimits { max_file_bytes: 1_000_000, max_chunks: 512, max_records: 10_000 }
}

fn fixture(modes: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 64 + modes.len() * 128];
    let mut header = M64Header {
        total_file_size: bytes.len() as u64, chunk_count: modes.len() as u64,
        ..M64Header::default()
    };
    header.update_crc();
    bytes[..64].copy_from_slice(bytemuck::bytes_of(&header));
    for (index, &mode) in modes.iter().enumerate() {
        let start = 64 + modes.len() * 64 + index * 64;
        let mut entry: M64ChunkDirectoryEntry = bytemuck::Zeroable::zeroed();
        entry.byte_offset = start as u64;
        entry.byte_length = 64;
        entry.record_count = 1;
        entry.chunk_record_mode = mode;
        entry.update_crc();
        bytes[64 + index * 64..128 + index * 64].copy_from_slice(bytemuck::bytes_of(&entry));
        match mode {
            M64_MODE_POINT => bytes[start..start + 64].copy_from_slice(bytemuck::bytes_of(&M64PointRecord::default())),
            M64_MODE_HOLO => bytes[start..start + 64].copy_from_slice(bytemuck::bytes_of(&M64HoloRecord::default())),
            M64_MODE_STRATA => {
                let mut record = M64StrataColumnRecord::default();
                record.update_crc();
                bytes[start..start + 64].copy_from_slice(bytemuck::bytes_of(&record));
            }
            M64_MODE_PHASE_JUNCTURE => {
                let mut record = M64PhaseJunctureRecord::default();
                record.update_crc();
                bytes[start..start + 64].copy_from_slice(bytemuck::bytes_of(&record));
            }
            _ => unreachable!(),
        }
    }
    bytes
}

fn header_edit(bytes: &mut [u8], edit: impl FnOnce(&mut M64Header)) {
    let mut value = bytemuck::pod_read_unaligned::<M64Header>(&bytes[..64]);
    edit(&mut value);
    value.update_crc();
    bytes[..64].copy_from_slice(bytemuck::bytes_of(&value));
}

fn entry_edit(bytes: &mut [u8], index: usize, edit: impl FnOnce(&mut M64ChunkDirectoryEntry)) {
    let start = 64 + index * 64;
    let mut value = bytemuck::pod_read_unaligned::<M64ChunkDirectoryEntry>(&bytes[start..start + 64]);
    edit(&mut value);
    value.update_crc();
    bytes[start..start + 64].copy_from_slice(bytemuck::bytes_of(&value));
}

fn error(bytes: &[u8]) -> M64ReadError {
    M64Reader::parse(bytes, limits(), &mut [M64PayloadInterval::default(); 512]).unwrap_err()
}

#[test]
fn public_reader_preserves_versions_aliases_and_unaligned_borrowed_payloads() {
    for version in [1, 2] {
        for magic in [M64_MAGIC, E64_MAGIC] {
            let mut bytes = fixture(&[M64_MODE_POINT, M64_MODE_HOLO, M64_MODE_STRATA, M64_MODE_PHASE_JUNCTURE]);
            header_edit(&mut bytes, |h| { h.version = version; h.magic = magic; });
            let mut unaligned = vec![0x77];
            unaligned.extend_from_slice(&bytes);
            let source = &unaligned[1..];
            let reader = M64Reader::parse(source, limits(), &mut [M64PayloadInterval::default(); 4]).unwrap();
            assert_eq!(reader.header().version, version);
            assert_eq!(reader.header().magic, magic);
            assert_eq!(reader.verification(), M64Verification {
                chunks: 4, total_records: 4, record_crc_checked: 2, records_without_checksum: 2,
            });
            let chunk = reader.chunk(2).unwrap();
            assert_eq!(chunk.payload().as_ptr(), source[448..].as_ptr());
            assert_eq!(chunk.record_bytes(0).unwrap().len(), 64);
            assert!(chunk.record_bytes(1).is_none());
            assert!(chunk.record_bytes(usize::MAX).is_none());
            assert!(reader.chunk(4).is_none());
        }
    }
}

#[test]
fn public_reader_checks_budgets_and_scratch_before_record_work() {
    let bytes = fixture(&[M64_MODE_STRATA]);
    let mut scratch = [M64PayloadInterval::default(); 1];
    let mut policy = limits();
    policy.max_file_bytes = bytes.len() - 1;
    assert_eq!(M64Reader::parse(&bytes, policy, &mut scratch).unwrap_err(), M64ReadError::FileByteLimit);
    policy = limits(); policy.max_chunks = 0;
    assert_eq!(M64Reader::parse(&bytes, policy, &mut scratch).unwrap_err(), M64ReadError::ChunkLimit);
    policy = limits(); policy.max_records = 0;
    assert_eq!(M64Reader::parse(&bytes, policy, &mut scratch).unwrap_err(), M64ReadError::RecordLimit);
    assert_eq!(M64Reader::parse(&bytes, limits(), &mut []).unwrap_err(),
        M64ReadError::ScratchTooSmall { required: 1, provided: 0 });
    assert_eq!(error(&bytes[..63]), M64ReadError::TruncatedHeader);
}

#[test]
fn public_reader_rejects_header_and_directory_structural_failures() {
    let base = fixture(&[M64_MODE_POINT]);
    let mut bytes = base.clone(); bytes[0] = 0;
    assert!(matches!(error(&bytes), M64ReadError::Header(M64Error::InvalidMagic)));
    bytes = base.clone(); header_edit(&mut bytes, |h| h.version = 0x1234_0001);
    assert_eq!(error(&bytes), M64ReadError::UnsupportedVersion(0x1234_0001));
    bytes = base.clone(); bytes[24] ^= 1;
    assert!(matches!(error(&bytes), M64ReadError::Header(M64Error::ChecksumMismatch { .. })));
    bytes = base.clone(); header_edit(&mut bytes, |h| h.header_size = 128);
    assert_eq!(error(&bytes), M64ReadError::Header(M64Error::InvalidHeaderSize));
    bytes = base.clone(); header_edit(&mut bytes, |h| h.container_flags = 1);
    assert_eq!(error(&bytes), M64ReadError::UnsupportedContainerFlags(1));
    bytes = base.clone(); header_edit(&mut bytes, |h| h.total_file_size += 64);
    assert_eq!(error(&bytes), M64ReadError::FileSizeMismatch);
    bytes = base.clone(); header_edit(&mut bytes, |h| h.chunk_table_offset = 65);
    assert_eq!(error(&bytes), M64ReadError::DirectoryAlignment);
    for offset in [0, 192] {
        bytes = base.clone(); header_edit(&mut bytes, |h| h.chunk_table_offset = offset);
        assert_eq!(error(&bytes), M64ReadError::DirectoryBounds);
    }
    bytes = base.clone(); header_edit(&mut bytes, |h| h.chunk_table_offset = u64::MAX - 63);
    assert_eq!(error(&bytes), M64ReadError::ArithmeticOverflow);
    bytes = base.clone(); bytes[64] ^= 1;
    assert!(matches!(error(&bytes), M64ReadError::Directory { error: M64Error::ChecksumMismatch { .. }, .. }));
    bytes = base.clone(); bytes.truncate(100);
    header_edit(&mut bytes, |h| h.total_file_size = 100);
    assert_eq!(error(&bytes), M64ReadError::DirectoryBounds);
}

#[test]
fn public_reader_rejects_payload_policy_bounds_and_overlap() {
    let base = fixture(&[M64_MODE_POINT]);
    let mut bytes = base.clone(); entry_edit(&mut bytes, 0, |e| e.chunk_record_mode = 4);
    assert_eq!(error(&bytes), M64ReadError::UnsupportedMode { chunk: 0, mode: 4 });
    for codec in [1, 2, 3, 4, 5, 6, 255] {
        bytes = base.clone(); entry_edit(&mut bytes, 0, |e| e.compression_flags = codec);
        assert_eq!(error(&bytes), M64ReadError::UnsupportedCodec { chunk: 0, codec });
    }
    for count in [0, 2, u32::MAX] {
        bytes = base.clone(); entry_edit(&mut bytes, 0, |e| e.record_count = count);
        assert_eq!(error(&bytes), M64ReadError::RecordLength { chunk: 0 });
    }
    bytes = base.clone(); entry_edit(&mut bytes, 0, |e| e.byte_offset = 129);
    assert_eq!(error(&bytes), M64ReadError::PayloadAlignment { chunk: 0 });
    bytes = base.clone(); entry_edit(&mut bytes, 0, |e| e.byte_offset = 192);
    assert_eq!(error(&bytes), M64ReadError::PayloadBounds { chunk: 0 });
    bytes = base.clone(); entry_edit(&mut bytes, 0, |e| e.byte_offset = u64::MAX - 63);
    assert_eq!(error(&bytes), M64ReadError::ArithmeticOverflow);
    for offset in [0, 64] {
        bytes = base.clone(); entry_edit(&mut bytes, 0, |e| e.byte_offset = offset);
        assert_eq!(error(&bytes), M64ReadError::MetadataOverlap { chunk: 0 });
    }
    bytes = base.clone(); bytes.pop();
    let length = bytes.len() as u64;
    header_edit(&mut bytes, |h| h.total_file_size = length);
    assert_eq!(error(&bytes), M64ReadError::PayloadBounds { chunk: 0 });
    bytes = fixture(&[M64_MODE_POINT; 3]);
    entry_edit(&mut bytes, 2, |e| e.byte_offset = 256);
    assert_eq!(error(&bytes), M64ReadError::PayloadOverlap);
}

#[test]
fn public_reader_checks_actual_record_crcs_and_discloses_unchecksummed_modes() {
    for mode in [M64_MODE_STRATA, M64_MODE_PHASE_JUNCTURE] {
        let mut bytes = fixture(&[mode]); bytes[128] ^= 1;
        assert!(matches!(error(&bytes), M64ReadError::Record {
            chunk: 0, record: 0, error: M64Error::ChecksumMismatch { .. },
        }));
    }
    for mode in [M64_MODE_POINT, M64_MODE_HOLO] {
        let mut bytes = fixture(&[mode]); bytes[128] ^= 1;
        let reader = M64Reader::parse(&bytes, limits(), &mut [M64PayloadInterval::default(); 1]).unwrap();
        assert_eq!(reader.verification().records_without_checksum, 1);
        assert_eq!(reader.verification().record_crc_checked, 0);
    }
}

#[test]
fn public_reader_accepts_permuted_nonoverlapping_directories_and_empty_files() {
    for count in 0..130 {
        let mut bytes = fixture(&vec![M64_MODE_POINT; count]);
        // Reverse physical payload order in the directory; no ordering policy is imposed.
        for index in 0..count {
            entry_edit(&mut bytes, index, |e| e.byte_offset = (64 + count * 64 + (count - index - 1) * 64) as u64);
        }
        let reader = M64Reader::parse(&bytes, limits(), &mut [M64PayloadInterval::default(); 130]).unwrap();
        assert_eq!(reader.verification().chunks, count);
        if count > 1 {
            let first = (64 + count * 64) as u64;
            entry_edit(&mut bytes, 0, |e| e.byte_offset = first);
            assert_eq!(error(&bytes), M64ReadError::PayloadOverlap);
        }
    }
}

#[test]
fn public_reader_allocates_zero_bytes_in_validation_and_borrowed_access() {
    let bytes = fixture(&[M64_MODE_POINT, M64_MODE_HOLO, M64_MODE_STRATA, M64_MODE_PHASE_JUNCTURE]);
    let mut scratch = [M64PayloadInterval::default(); 4];
    ALLOCS.with(|count| count.set(Some(0)));
    let result = M64Reader::parse(&bytes, limits(), &mut scratch);
    let borrowed = result.as_ref().ok().and_then(|reader| reader.chunk(2)).map(|chunk| chunk.payload());
    let allocations = ALLOCS.with(|count| count.replace(None)).unwrap();
    assert!(result.is_ok());
    assert_eq!(borrowed.unwrap().len(), 64);
    assert_eq!(allocations, 0);
}
