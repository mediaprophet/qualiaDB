//! Compact per-block S/P/C membership for range pruning.
//!
//! Field min/max ranges are a weak first reject: any hash inside a wide interval
//! still forces a SuperBlock decode. These postings are exact unique hashes,
//! delta-varint encoded, so a bound subject/predicate/context either is present
//! or the block is skipped. No false positives, no false negatives.
//!
//! A 256-byte Bloom is used only when unique hashes would be *larger* than that
//! Bloom (measured per field). Blooms have no false negatives; measured FPR is
//! reported by tests.

use std::io;

use crate::NQuin;

pub const FIELD_POSTINGS_MAGIC: [u8; 4] = *b"PIDX";
pub const FIELD_POSTINGS_HEADER_BYTES: usize = 16;
pub const BLOOM_BYTES: usize = 256;
pub const BLOOM_BITS: usize = BLOOM_BYTES * 8;
pub const BLOOM_HASHES: usize = 4;

const KIND_POSTINGS: u8 = 0;
const KIND_BLOOM: u8 = 1;

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// One SuperBlock's compact S/P/C membership.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockFieldPostings {
    pub subjects: Vec<u64>,
    pub predicates: Vec<u64>,
    pub contexts: Vec<u64>,
}

impl BlockFieldPostings {
    pub fn from_quins(quins: &[NQuin]) -> Self {
        let mut subjects = Vec::with_capacity(quins.len());
        let mut predicates = Vec::with_capacity(quins.len());
        let mut contexts = Vec::with_capacity(quins.len());
        for quin in quins {
            subjects.push(quin.subject);
            predicates.push(quin.predicate);
            contexts.push(quin.context);
        }
        sort_unique(&mut subjects);
        sort_unique(&mut predicates);
        sort_unique(&mut contexts);
        Self {
            subjects,
            predicates,
            contexts,
        }
    }

    pub fn contains_subject(&self, value: u64) -> bool {
        self.subjects.binary_search(&value).is_ok()
    }
    pub fn contains_predicate(&self, value: u64) -> bool {
        self.predicates.binary_search(&value).is_ok()
    }
    pub fn contains_context(&self, value: u64) -> bool {
        self.contexts.binary_search(&value).is_ok()
    }

    /// Encoded size if this field used exact postings.
    pub fn posting_bytes(values: &[u64]) -> usize {
        2 + encoded_delta_len(values)
    }
}

/// Encode one SuperBlock's three fields. Chooses Bloom when it is strictly
/// smaller than the exact posting list.
pub fn encode_block_postings(postings: &BlockFieldPostings) -> Vec<u8> {
    let mut out = Vec::new();
    encode_field(&mut out, &postings.subjects);
    encode_field(&mut out, &postings.predicates);
    encode_field(&mut out, &postings.contexts);
    out
}

fn encode_field(out: &mut Vec<u8>, values: &[u64]) {
    let posting_len = BlockFieldPostings::posting_bytes(values);
    if values.len() > 8 && posting_len > BLOOM_BYTES + 1 {
        out.push(KIND_BLOOM);
        let mut bloom = [0u8; BLOOM_BYTES];
        for value in values {
            bloom_insert(&mut bloom, *value);
        }
        out.extend_from_slice(&bloom);
        return;
    }
    out.push(KIND_POSTINGS);
    out.extend_from_slice(&(values.len() as u16).to_le_bytes());
    encode_deltas(out, values);
}

/// Decode one SuperBlock payload into caller-owned hash scratch, then test
/// membership without retaining the payload.
pub fn field_may_contain(encoded: &[u8], field: usize, value: u64) -> io::Result<bool> {
    let mut offset = 0usize;
    for current in 0..3 {
        let (next, present) = decode_field_contains(encoded, offset, value)?;
        if current == field {
            return Ok(present);
        }
        offset = next;
    }
    Err(invalid("field postings have fewer than three fields"))
}

fn decode_field_contains(encoded: &[u8], offset: usize, value: u64) -> io::Result<(usize, bool)> {
    let kind = *encoded
        .get(offset)
        .ok_or_else(|| invalid("truncated field postings"))?;
    match kind {
        KIND_POSTINGS => {
            if offset + 3 > encoded.len() {
                return Err(invalid("truncated posting count"));
            }
            let count = u16::from_le_bytes([encoded[offset + 1], encoded[offset + 2]]) as usize;
            let mut cursor = offset + 3;
            let mut previous = 0u64;
            let mut found = false;
            for _ in 0..count {
                let (delta, used) = decode_varint(&encoded[cursor..])?;
                previous = previous
                    .checked_add(delta)
                    .ok_or_else(|| invalid("posting delta overflow"))?;
                cursor = cursor
                    .checked_add(used)
                    .ok_or_else(|| invalid("posting cursor overflow"))?;
                if previous == value {
                    found = true;
                }
            }
            Ok((cursor, found))
        }
        KIND_BLOOM => {
            let end = offset
                .checked_add(1 + BLOOM_BYTES)
                .ok_or_else(|| invalid("bloom overflow"))?;
            if end > encoded.len() {
                return Err(invalid("truncated bloom filter"));
            }
            let bloom = &encoded[offset + 1..end];
            Ok((end, bloom_may_contain(bloom, value)))
        }
        _ => Err(invalid("unknown field-postings kind")),
    }
}

pub fn encode_postings_section(blocks: &[BlockFieldPostings]) -> Vec<u8> {
    let mut payloads = Vec::with_capacity(blocks.len());
    let mut offsets = Vec::with_capacity(blocks.len() + 1);
    let mut cursor = 0u32;
    offsets.push(0);
    for block in blocks {
        let payload = encode_block_postings(block);
        cursor = cursor
            .checked_add(payload.len() as u32)
            .expect("field postings payload exceeds u32");
        offsets.push(cursor);
        payloads.push(payload);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&FIELD_POSTINGS_MAGIC);
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&(blocks.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    for offset in &offsets {
        out.extend_from_slice(&offset.to_le_bytes());
    }
    for payload in payloads {
        out.extend_from_slice(&payload);
    }
    out
}

pub fn validate_postings_section(bytes: &[u8], expected_blocks: usize) -> io::Result<()> {
    if bytes.len() < FIELD_POSTINGS_HEADER_BYTES {
        return Err(invalid("field postings shorter than header"));
    }
    if bytes[0..4] != FIELD_POSTINGS_MAGIC {
        return Err(invalid("field postings have bad magic"));
    }
    if u32::from_le_bytes(bytes[4..8].try_into().unwrap()) != 1 {
        return Err(invalid("unsupported field postings version"));
    }
    let block_count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    if block_count != expected_blocks {
        return Err(invalid("field postings block count mismatch"));
    }
    let table_bytes = (block_count + 1)
        .checked_mul(4)
        .ok_or_else(|| invalid("field postings table overflow"))?;
    let header_end = FIELD_POSTINGS_HEADER_BYTES + table_bytes;
    if bytes.len() < header_end {
        return Err(invalid("field postings offset table truncated"));
    }
    let last = u32::from_le_bytes(bytes[header_end - 4..header_end].try_into().unwrap()) as usize;
    if bytes.len() != header_end + last {
        return Err(invalid("field postings payload length mismatch"));
    }
    for block_index in 0..block_count {
        let _ = block_payload(bytes, block_index)?;
    }
    Ok(())
}

/// Absolute byte interval of one SuperBlock's encoded PIDX payload, relative to
/// the start of a complete PIDX section (header + offset table + payloads).
pub(crate) fn block_payload_interval(
    block_count: usize,
    block_index: usize,
    table_start: u32,
    table_end: u32,
) -> io::Result<(usize, usize)> {
    if block_index >= block_count {
        return Err(invalid("field postings block index out of range"));
    }
    if table_end < table_start {
        return Err(invalid("field postings interval inverted"));
    }
    let table_bytes = (block_count + 1)
        .checked_mul(4)
        .ok_or_else(|| invalid("field postings table overflow"))?;
    let payload_base = FIELD_POSTINGS_HEADER_BYTES
        .checked_add(table_bytes)
        .ok_or_else(|| invalid("field postings payload base overflow"))?;
    let from = payload_base
        .checked_add(table_start as usize)
        .ok_or_else(|| invalid("posting payload start overflow"))?;
    let to = payload_base
        .checked_add(table_end as usize)
        .ok_or_else(|| invalid("posting payload end overflow"))?;
    Ok((from, to))
}

/// Slice one SuperBlock's encoded S/P/C membership out of an in-memory PIDX section.
pub fn block_payload<'a>(bytes: &'a [u8], block_index: usize) -> io::Result<&'a [u8]> {
    if bytes.len() < FIELD_POSTINGS_HEADER_BYTES {
        return Err(invalid("field postings shorter than header"));
    }
    let block_count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let table = FIELD_POSTINGS_HEADER_BYTES
        .checked_add(
            block_index
                .checked_mul(4)
                .ok_or_else(|| invalid("field postings table overflow"))?,
        )
        .ok_or_else(|| invalid("field postings table overflow"))?;
    if bytes.len() < table + 8 {
        return Err(invalid("field postings offset table truncated"));
    }
    let start = u32::from_le_bytes(bytes[table..table + 4].try_into().unwrap());
    let end = u32::from_le_bytes(bytes[table + 4..table + 8].try_into().unwrap());
    let (from, to) = block_payload_interval(block_count, block_index, start, end)?;
    bytes
        .get(from..to)
        .ok_or_else(|| invalid("field postings payload slice out of range"))
}

fn sort_unique(values: &mut Vec<u64>) {
    values.sort_unstable();
    values.dedup();
}

fn encoded_delta_len(values: &[u64]) -> usize {
    let mut previous = 0u64;
    let mut len = 0usize;
    for value in values {
        len += varint_len(value - previous);
        previous = *value;
    }
    len
}

fn encode_deltas(out: &mut Vec<u8>, values: &[u64]) {
    let mut previous = 0u64;
    for value in values {
        encode_varint(out, value - previous);
        previous = *value;
    }
}

fn encode_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            return;
        }
    }
}

fn varint_len(mut value: u64) -> usize {
    let mut len = 1;
    while value > 0x7f {
        value >>= 7;
        len += 1;
    }
    len
}

fn decode_varint(bytes: &[u8]) -> io::Result<(u64, usize)> {
    let mut value = 0u64;
    let mut shift = 0u32;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if shift >= 64 {
            return Err(invalid("varint shift overflow"));
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((value, index + 1));
        }
        shift += 7;
    }
    Err(invalid("truncated varint"))
}

fn bloom_insert(bloom: &mut [u8], value: u64) {
    for hash in bloom_hashes(value) {
        let bit = (hash as usize) % BLOOM_BITS;
        bloom[bit / 8] |= 1 << (bit % 8);
    }
}

fn bloom_may_contain(bloom: &[u8], value: u64) -> bool {
    bloom_hashes(value).into_iter().all(|hash| {
        let bit = (hash as usize) % BLOOM_BITS;
        bloom[bit / 8] & (1 << (bit % 8)) != 0
    })
}

fn bloom_hashes(value: u64) -> [u64; BLOOM_HASHES] {
    let mixed = value.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let alt = (!value).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    [
        mixed,
        alt,
        mixed.wrapping_add(alt),
        mixed ^ alt.rotate_left(17),
    ]
}

/// Measured false-positive rate of the Bloom encoding on a synthetic field.
pub fn measure_bloom_false_positives(values: &[u64], probes: &[u64]) -> (usize, usize) {
    let mut bloom = [0u8; BLOOM_BYTES];
    for value in values {
        bloom_insert(&mut bloom, *value);
    }
    let mut false_positives = 0usize;
    let mut eligible = 0usize;
    for probe in probes {
        if values.binary_search(probe).is_ok() {
            continue;
        }
        eligible += 1;
        if bloom_may_contain(&bloom, *probe) {
            false_positives += 1;
        }
    }
    (false_positives, eligible)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_postings_have_no_false_positives() {
        let quins = [quin(10, 1, 100), quin(20, 1, 101), quin(10, 2, 102)];
        let postings = BlockFieldPostings::from_quins(&quins);
        let encoded = encode_block_postings(&postings);
        assert!(field_may_contain(&encoded, 0, 10).unwrap());
        assert!(field_may_contain(&encoded, 0, 20).unwrap());
        assert!(!field_may_contain(&encoded, 0, 99).unwrap());
        assert!(field_may_contain(&encoded, 1, 2).unwrap());
        assert!(!field_may_contain(&encoded, 1, 9).unwrap());
    }

    #[test]
    fn bloom_is_chosen_when_it_is_smaller_and_has_no_false_negatives() {
        let values: Vec<u64> = (0..400).map(|i| i * 1_000_003 + 17).collect();
        let mut encoded = Vec::new();
        encode_field(&mut encoded, &values);
        assert_eq!(encoded[0], KIND_BLOOM);
        for value in &values {
            assert!(field_may_contain(&encoded, 0, *value).unwrap());
        }
        let probes: Vec<u64> = (0..2_000).map(|i| i * 97 + 3).collect();
        let (fp, eligible) = measure_bloom_false_positives(&values, &probes);
        assert!(eligible > 0);
        // 256-byte / 4-hash Bloom on 400 items is compact; FPR must stay usable.
        assert!(
            (fp as f64) / (eligible as f64) < 0.35,
            "bloom FPR too high: {fp}/{eligible}"
        );
    }

    #[test]
    fn section_round_trip_and_validation() {
        let blocks = vec![
            BlockFieldPostings::from_quins(&[quin(1, 2, 3)]),
            BlockFieldPostings::from_quins(&[quin(4, 5, 6), quin(7, 5, 8)]),
        ];
        let section = encode_postings_section(&blocks);
        validate_postings_section(&section, 2).unwrap();
        let payload = block_payload(&section, 1).unwrap();
        assert!(field_may_contain(payload, 0, 7).unwrap());
        assert!(!field_may_contain(payload, 0, 1).unwrap());
        assert!(block_payload(&section, 2).is_err());
    }

    #[test]
    fn inverted_table_interval_is_rejected() {
        let mut section = encode_postings_section(&[
            BlockFieldPostings::from_quins(&[quin(1, 2, 3)]),
            BlockFieldPostings::from_quins(&[quin(4, 5, 6)]),
        ]);
        let table = FIELD_POSTINGS_HEADER_BYTES;
        section[table..table + 4].copy_from_slice(&8u32.to_le_bytes());
        section[table + 4..table + 8].copy_from_slice(&0u32.to_le_bytes());
        assert!(validate_postings_section(&section, 2).is_err());
        assert!(block_payload(&section, 0).is_err());
    }

    fn quin(subject: u64, predicate: u64, context: u64) -> NQuin {
        NQuin {
            subject,
            predicate,
            object: 0,
            context,
            metadata: 0,
            parity: 0,
        }
    }
}
