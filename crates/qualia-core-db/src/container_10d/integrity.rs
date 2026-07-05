//! Whole-file content hash + canonical-encoding integrity gates (P0.3).
//!
//! The `.10d` container has two integrity layers:
//!
//! 1. **Per-section CRC-32C** (P0.2, in [`super::section`]) — each section
//!    descriptor carries a CRC-32C over its payload; a flipped payload bit is
//!    caught at section read.
//! 2. **Whole-file content hash** (P0.3, here) — a CRC-32C over the entire
//!    file (header + section table + payloads + padding), stored in the
//!    header's `header_crc32c` field. This catches header corruption and
//!    table corruption that the per-section CRC cannot (e.g. a flipped
//!    `byte_offset` in a descriptor that still points somewhere valid).
//!
//! The whole-file hash is computed with `header_crc32c` zeroed during
//! computation (the standard self-referential CRC technique): on encode, the
//! field is written zero, the CRC is computed over the full buffer, then the
//! CRC is written into the field; on verify, the stored value is saved, the
//! field is zeroed in-place, the CRC is recomputed, and the two are compared.
//!
//! **Determinism / canonical bytes.** Because [`super::section::encode_container`]
//! produces byte-identical output for identical input (canonical section
//! order, zeroed padding), the whole-file hash is stable across encodes and
//! changes on any payload-byte change. This is the P0.3 "whole-file hash is
//! stable across encodes and changes on any payload-byte change, zero-alloc
//! over the caller buffer" gate.

use crate::container_10d::crc32c::crc32c;
use crate::container_10d::header::HEADER_BYTE_SIZE;

/// Offset of the `header_crc32c` field within the header (and thus within
/// the file, since the header is at offset 0).
const HEADER_CRC32C_OFFSET: usize = 52;

/// Integrity verification error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityError {
    /// The whole-file CRC-32C does not match the value stored in the header.
    WholeFileCrcMismatch { expected: u32, got: u32 },
    /// The input is shorter than the header.
    TooShort { got: usize },
}

impl std::fmt::Display for IntegrityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WholeFileCrcMismatch { expected, got } => write!(f, "10d whole-file CRC-32C mismatch: expected {expected:#010x}, got {got:#010x}"),
            Self::TooShort { got } => write!(f, "10d input too short for integrity check: got {got}, need {HEADER_BYTE_SIZE}"),
        }
    }
}

impl std::error::Error for IntegrityError {}

/// Compute the whole-file CRC-32C over `data`, treating the `header_crc32c`
/// field (bytes 52..56) as zero. Zero-heap: operates directly over the caller
/// buffer. Does not modify `data`.
///
/// This is the value that should be stored in the header's `header_crc32c`
/// field after encoding.
pub fn compute_whole_file_crc32c(data: &[u8]) -> u32 {
    if data.len() < HEADER_BYTE_SIZE {
        // A sub-header buffer: CRC over whatever is present (with the crc
        // field region treated as zero if it's not even there).
        return crc32c(data);
    }
    // CRC over [0..52] + [56..end], with [52..56] treated as zero.
    // Compute incrementally: first the head, then four zero bytes, then the
    // tail. This avoids allocating a modified copy.
    let mut crc = crate::container_10d::crc32c::crc32c_update(0xFFFF_FFFF, &data[..HEADER_CRC32C_OFFSET]);
    // Four zero bytes for the crc field itself.
    crc = crate::container_10d::crc32c::crc32c_update(crc, &[0u8, 0, 0, 0]);
    crc = crate::container_10d::crc32c::crc32c_update(crc, &data[HEADER_CRC32C_OFFSET + 4..]);
    !crc
}

/// Write the whole-file CRC-32C into the header's `header_crc32c` field
/// in-place within `data`. Called by the encoder after the full file is
/// written. Zero-heap.
pub fn seal_whole_file_crc32c(data: &mut [u8]) {
    if data.len() < HEADER_BYTE_SIZE {
        return;
    }
    // Ensure the crc field is zero before computing (the encoder already
    // writes it zero, but be defensive).
    data[HEADER_CRC32C_OFFSET..HEADER_CRC32C_OFFSET + 4].copy_from_slice(&0u32.to_le_bytes());
    let crc = compute_whole_file_crc32c(data);
    data[HEADER_CRC32C_OFFSET..HEADER_CRC32C_OFFSET + 4].copy_from_slice(&crc.to_le_bytes());
}

/// Verify the whole-file CRC-32C stored in the header against a recomputed
/// value. Returns `Ok(())` if they match, or an error naming both values.
/// Zero-heap: saves the stored CRC, zeroes the field in-place, recomputes,
/// restores the field.
pub fn verify_whole_file_crc32c(data: &mut [u8]) -> Result<(), IntegrityError> {
    if data.len() < HEADER_BYTE_SIZE {
        return Err(IntegrityError::TooShort { got: data.len() });
    }
    let stored = u32::from_le_bytes(
        data[HEADER_CRC32C_OFFSET..HEADER_CRC32C_OFFSET + 4]
            .try_into()
            .unwrap(),
    );
    // Zero the field, recompute, restore.
    data[HEADER_CRC32C_OFFSET..HEADER_CRC32C_OFFSET + 4].copy_from_slice(&0u32.to_le_bytes());
    let actual = compute_whole_file_crc32c(data);
    data[HEADER_CRC32C_OFFSET..HEADER_CRC32C_OFFSET + 4].copy_from_slice(&stored.to_le_bytes());
    if actual != stored {
        return Err(IntegrityError::WholeFileCrcMismatch { expected: stored, got: actual });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::header::Container10dHeader;
    use crate::container_10d::section::{
        encode_container, parse_section_table, AlignmentTier, SectionInput, SectionType,
    };

    #[test]
    fn whole_file_crc_is_stable_across_two_identical_encodes() {
        let h = Container10dHeader::proposed();
        let mesh_payload = [0xAAu8; 100];
        let node_payload = [0xBBu8; 40 * 3];
        let inputs = [
            SectionInput { section_type: SectionType::QuantizedMesh, alignment_tier: AlignmentTier::Word, stride: 0, element_count: 0, payload: &mesh_payload },
            SectionInput { section_type: SectionType::Tensor10DNodes, alignment_tier: AlignmentTier::CacheLine, stride: 40, element_count: 3, payload: &node_payload },
        ];
        let mut out_a = [0u8; 512];
        let mut out_b = [0u8; 512];
        let n_a = encode_container(&h, &inputs, &mut out_a).expect("encode a");
        let n_b = encode_container(&h, &inputs, &mut out_b).expect("encode b");
        seal_whole_file_crc32c(&mut out_a[..n_a]);
        seal_whole_file_crc32c(&mut out_b[..n_b]);
        let crc_a = compute_whole_file_crc32c(&out_a[..n_a]);
        let crc_b = compute_whole_file_crc32c(&out_b[..n_b]);
        assert_eq!(crc_a, crc_b, "whole-file CRC must be stable across identical encodes");
        // And the sealed bytes are identical.
        assert_eq!(&out_a[..n_a], &out_b[..n_b]);
    }

    #[test]
    fn whole_file_crc_changes_on_payload_bit_flip() {
        let h = Container10dHeader::proposed();
        let payload = [0xAAu8; 100];
        let inputs = [SectionInput { section_type: SectionType::QuantizedMesh, alignment_tier: AlignmentTier::Word, stride: 0, element_count: 0, payload: &payload }];
        let mut out = [0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        seal_whole_file_crc32c(&mut out[..n]);
        let crc_clean = compute_whole_file_crc32c(&out[..n]);
        // Flip a payload bit.
        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        let descs = parse_section_table(&out[..n], &parsed_h).expect("table parse");
        let p_off = descs[0].byte_offset as usize;
        out[p_off] ^= 0x01;
        let crc_flipped = compute_whole_file_crc32c(&out[..n]);
        assert_ne!(crc_clean, crc_flipped, "whole-file CRC must change on a payload bit flip");
    }

    #[test]
    fn whole_file_crc_changes_on_header_byte_flip() {
        let h = Container10dHeader::proposed();
        let payload = [0xAAu8; 100];
        let inputs = [SectionInput { section_type: SectionType::QuantizedMesh, alignment_tier: AlignmentTier::Word, stride: 0, element_count: 0, payload: &payload }];
        let mut out = [0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        seal_whole_file_crc32c(&mut out[..n]);
        let crc_clean = compute_whole_file_crc32c(&out[..n]);
        // Flip a header byte (the flags field at offset 6, avoiding the crc field).
        out[6] ^= 0x01;
        let crc_flipped = compute_whole_file_crc32c(&out[..n]);
        assert_ne!(crc_clean, crc_flipped, "whole-file CRC must change on a header byte flip");
    }

    #[test]
    fn verify_passes_on_clean_file() {
        let h = Container10dHeader::proposed();
        let payload = [0xAAu8; 100];
        let inputs = [SectionInput { section_type: SectionType::QuantizedMesh, alignment_tier: AlignmentTier::Word, stride: 0, element_count: 0, payload: &payload }];
        let mut out = [0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        seal_whole_file_crc32c(&mut out[..n]);
        verify_whole_file_crc32c(&mut out[..n]).expect("clean file must verify");
    }

    #[test]
    fn verify_rejects_flipped_payload_bit() {
        let h = Container10dHeader::proposed();
        let payload = [0xAAu8; 100];
        let inputs = [SectionInput { section_type: SectionType::QuantizedMesh, alignment_tier: AlignmentTier::Word, stride: 0, element_count: 0, payload: &payload }];
        let mut out = [0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        seal_whole_file_crc32c(&mut out[..n]);
        // Flip a payload bit.
        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        let descs = parse_section_table(&out[..n], &parsed_h).expect("table parse");
        let p_off = descs[0].byte_offset as usize;
        out[p_off] ^= 0x01;
        let err = verify_whole_file_crc32c(&mut out[..n]).expect_err("flipped bit must fail verify");
        assert!(matches!(err, IntegrityError::WholeFileCrcMismatch { .. }), "{err}");
    }

    #[test]
    fn verify_restores_the_crc_field_after_check() {
        // verify_whole_file_crc32c zeroes the field in-place, recomputes, then
        // restores it. The field must be unchanged after the call (whether
        // pass or fail).
        let h = Container10dHeader::proposed();
        let payload = [0xAAu8; 100];
        let inputs = [SectionInput { section_type: SectionType::QuantizedMesh, alignment_tier: AlignmentTier::Word, stride: 0, element_count: 0, payload: &payload }];
        let mut out = [0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        seal_whole_file_crc32c(&mut out[..n]);
        let stored_before: [u8; 4] = out[HEADER_CRC32C_OFFSET..HEADER_CRC32C_OFFSET + 4].try_into().unwrap();
        let _ = verify_whole_file_crc32c(&mut out[..n]);
        let stored_after: [u8; 4] = out[HEADER_CRC32C_OFFSET..HEADER_CRC32C_OFFSET + 4].try_into().unwrap();
        assert_eq!(stored_before, stored_after, "verify must restore the crc field");
    }

    #[test]
    fn bare_header_seals_and_verifies() {
        let h = Container10dHeader::proposed();
        let mut out = [0u8; 128];
        let n = encode_container(&h, &[], &mut out).expect("bare encode");
        seal_whole_file_crc32c(&mut out[..n]);
        verify_whole_file_crc32c(&mut out[..n]).expect("bare header must verify");
    }
}
