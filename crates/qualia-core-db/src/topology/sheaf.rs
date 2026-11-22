//! Optional cellular-sheaf footer in a GTOP geometry segment. The original GTOP header,
//! graph directory, graph payloads, offsets, and CRC-32C remain byte-identical.
//!
//! A new segment appends `[restriction CSR?][zero padding to 64 B][64 B footer]`.
//! The footer is little-endian. Offsets are relative to the raw GTOP segment:
//! `0 magic[8]`, `8 version:u16`, `10 flags:u16`, `12 base_length:u32`,
//! `16 stalk_schema_q_hash:u64`, `24 csr_offset:u32`, `28 csr_length:u32`,
//! `32 geometry_id:u64`, `40 base_crc32c:u32`, `44 csr_crc32c:u32`,
//! `48 reserved[12]`, `60 footer_crc32c:u32`. Flag bit 0 signals CSR presence.
//! Absent CSR uses zero offset, length and checksum. The footer CRC covers bytes 0–59.
//!
//! CSR format: `SHR1`, `rows:u32`, `columns:u32`, `nnz:u32`, then `rows+1`
//! u32 offsets, `nnz` u32 column indices, zero padding to 8 B, and `nnz`
//! finite f64 coefficients, all little-endian. Column indices increase within each
//! row. The RDF stalk schema defines the source and target basis of the matrix.

use core::{fmt, ops::Range};

use crate::container_10d::{crc32c::{crc32c, crc32c_update}, GRAPHICS_ASSET_RING_BYTES};

use super::header::{GeometryComplexKind, GeometryMetricKind, GraphEmbedRole, GraphMorphology};

pub const GTOP_SHEAF_FOOTER_MAGIC: [u8; 8] = *b"GTOPSHEF";
pub const GTOP_SHEAF_FOOTER_VERSION: u16 = 1;
pub const GTOP_SHEAF_FOOTER_BYTES: usize = 64;
pub const GTOP_RESTRICTION_CSR_BYTES_MAX: usize = 42 * 1024 * 1024;
pub const GTOP_RESTRICTION_AXIS_MAX: u32 = 1_048_576;

const CSR_MAGIC: [u8; 4] = *b"SHR1";
const PRESENT: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GtopSheafError {
    InvalidFooter,
    UnsupportedVersion(u16),
    InvalidBase,
    InvalidSchemaHash,
    InvalidCsr,
    SizeLimit,
    Checksum,
    OutputTooSmall,
}

impl fmt::Display for GtopSheafError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GTOP sheaf metadata: {self:?}")
    }
}

impl std::error::Error for GtopSheafError {}

#[derive(Clone, Copy, Debug)]
pub struct GtopSheafSpec<'a> {
    pub stalk_schema_q_hash: u64,
    pub restrictions: Option<GtopRestrictionCsrSpec<'a>>,
}

#[derive(Clone, Copy, Debug)]
pub struct GtopRestrictionCsrSpec<'a> {
    pub rows: u32,
    pub columns: u32,
    pub row_offsets: &'a [u32],
    pub column_indices: &'a [u32],
    pub coefficients: &'a [f64],
}

/// Verified, zero-copy metadata, suitable for both aligned mmap and unaligned envelopes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GtopSheafView<'a> {
    pub geometry_id: u64,
    pub stalk_schema_q_hash: u64,
    pub base_segment_bytes: usize,
    pub restriction_csr_offset: Option<u32>,
    pub restrictions: Option<GtopRestrictionCsrView<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GtopRestrictionCsrView<'a> {
    pub rows: u32,
    pub columns: u32,
    row_offsets: &'a [u8],
    column_indices: &'a [u8],
    coefficients: &'a [u8],
}

impl<'a> GtopRestrictionCsrView<'a> {
    pub fn nonzero_count(&self) -> usize {
        self.coefficients.len() / 8
    }

    pub fn row(&self, row: u32) -> Option<impl ExactSizeIterator<Item = (u32, f64)> + 'a> {
        if row >= self.rows {
            return None;
        }
        let begin = le_u32(self.row_offsets, row as usize * 4) as usize;
        let end = le_u32(self.row_offsets, (row as usize + 1) * 4) as usize;
        let indices: &'a [u8] = &self.column_indices[begin * 4..end * 4];
        let coefficients: &'a [u8] = &self.coefficients[begin * 8..end * 8];
        Some(indices.chunks_exact(4).zip(coefficients.chunks_exact(8))
            .map(|(index, coefficient)| (le_u32(index, 0), f64::from_bits(le_u64(coefficient, 0)))))
    }
}

struct CsrLayout {
    row_offsets: Range<usize>,
    column_indices: Range<usize>,
    coefficients: Range<usize>,
}

fn align(n: usize, alignment: usize) -> Option<usize> {
    n.checked_add(alignment - 1).map(|n| n & !(alignment - 1))
}

fn csr_layout(rows: u32, columns: u32, nnz: usize) -> Result<CsrLayout, GtopSheafError> {
    if rows == 0 || columns == 0 || rows > GTOP_RESTRICTION_AXIS_MAX || columns > GTOP_RESTRICTION_AXIS_MAX {
        return Err(GtopSheafError::InvalidCsr);
    }
    if nnz > GTOP_RESTRICTION_CSR_BYTES_MAX / 12 {
        return Err(GtopSheafError::SizeLimit);
    }
    let indices_start = 16 + (rows as usize + 1) * 4;
    let indices_end = indices_start + nnz * 4;
    let coefficients_start = align(indices_end, 8).ok_or(GtopSheafError::SizeLimit)?;
    let end = coefficients_start + nnz * 8;
    if end > GTOP_RESTRICTION_CSR_BYTES_MAX {
        return Err(GtopSheafError::SizeLimit);
    }
    Ok(CsrLayout { row_offsets: 16..indices_start,
        column_indices: indices_start..indices_end, coefficients: coefficients_start..end })
}

/// Required size for the base and optional extension. The caller's ring budget is checked
/// independently against allocation and graph compilation work.
pub fn required_gtop_sheaf_segment_bytes(base_len: usize, spec: GtopSheafSpec<'_>) -> Result<usize, GtopSheafError> {
    if spec.stalk_schema_q_hash == 0 {
        return Err(GtopSheafError::InvalidSchemaHash);
    }
    if base_len < 64 || base_len % 64 != 0 || base_len > GRAPHICS_ASSET_RING_BYTES {
        return Err(GtopSheafError::InvalidBase);
    }
    let csr_length = if let Some(csr) = spec.restrictions {
        let layout = csr_layout(csr.rows, csr.columns, csr.coefficients.len())?;
        if csr.row_offsets.len() != csr.rows as usize + 1 || csr.column_indices.len() != csr.coefficients.len() {
            return Err(GtopSheafError::InvalidCsr);
        }
        layout.coefficients.end
    } else {
        0
    };
    let end = align(base_len + csr_length, 64).ok_or(GtopSheafError::SizeLimit)? + GTOP_SHEAF_FOOTER_BYTES;
    if end > GRAPHICS_ASSET_RING_BYTES {
        return Err(GtopSheafError::SizeLimit);
    }
    Ok(end)
}

/// Append verified sheaf metadata to a compiled raw GTOP prefix in caller storage.
pub fn append_gtop_sheaf_metadata(storage: &mut [u8], base_len: usize, spec: GtopSheafSpec<'_>)
    -> Result<usize, GtopSheafError>
{
    let end = required_gtop_sheaf_segment_bytes(base_len, spec)?;
    let output = storage.get_mut(..end).ok_or(GtopSheafError::OutputTooSmall)?;
    let (base, suffix) = output.split_at_mut(base_len);
    let (geometry_id, base_crc) = verify_base(base)?;
    let footer_start = suffix.len() - GTOP_SHEAF_FOOTER_BYTES;
    let (body, footer) = suffix.split_at_mut(footer_start);
    body.fill(0);
    let (csr_offset, csr_length, csr_crc) = if let Some(csr) = spec.restrictions {
        let layout = csr_layout(csr.rows, csr.columns, csr.coefficients.len())?;
        let block = &mut body[..layout.coefficients.end];
        block[..4].copy_from_slice(&CSR_MAGIC);
        block[4..8].copy_from_slice(&csr.rows.to_le_bytes());
        block[8..12].copy_from_slice(&csr.columns.to_le_bytes());
        block[12..16].copy_from_slice(&(csr.coefficients.len() as u32).to_le_bytes());
        for (bytes, value) in block[layout.row_offsets].chunks_exact_mut(4).zip(csr.row_offsets) {
            bytes.copy_from_slice(&value.to_le_bytes());
        }
        for (bytes, value) in block[layout.column_indices].chunks_exact_mut(4).zip(csr.column_indices) {
            bytes.copy_from_slice(&value.to_le_bytes());
        }
        for (bytes, value) in block[layout.coefficients].chunks_exact_mut(8).zip(csr.coefficients) {
            bytes.copy_from_slice(&value.to_bits().to_le_bytes());
        }
        read_csr(block)?;
        (base_len as u32, block.len() as u32, crc32c(block))
    } else {
        (0, 0, 0)
    };
    footer.fill(0);
    footer[..8].copy_from_slice(&GTOP_SHEAF_FOOTER_MAGIC);
    footer[8..10].copy_from_slice(&GTOP_SHEAF_FOOTER_VERSION.to_le_bytes());
    footer[10..12].copy_from_slice(&(if spec.restrictions.is_some() { PRESENT } else { 0 }).to_le_bytes());
    footer[12..16].copy_from_slice(&(base_len as u32).to_le_bytes());
    footer[16..24].copy_from_slice(&spec.stalk_schema_q_hash.to_le_bytes());
    footer[24..28].copy_from_slice(&csr_offset.to_le_bytes());
    footer[28..32].copy_from_slice(&csr_length.to_le_bytes());
    footer[32..40].copy_from_slice(&geometry_id.to_le_bytes());
    footer[40..44].copy_from_slice(&base_crc.to_le_bytes());
    footer[44..48].copy_from_slice(&csr_crc.to_le_bytes());
    let footer_crc = crc32c(&footer[..60]);
    footer[60..64].copy_from_slice(&footer_crc.to_le_bytes());
    Ok(end)
}

/// Read an optional trailer. A tagged malformed or unknown version fails closed.
/// Use the v2 GTOP envelope to require trailer presence on untrusted retained segments.
pub fn read_gtop_sheaf_metadata(segment: &[u8]) -> Result<Option<GtopSheafView<'_>>, GtopSheafError> {
    if !has_footer_marker(segment) {
        return Ok(None);
    }
    // A legacy segment may happen to contain the marker in its last graph payload.
    // Its valid whole-base CRC takes precedence over this content coincidence.
    if verify_base(segment).is_ok() {
        return Ok(None);
    }
    if segment.len() % 64 != 0 || segment.len() > GRAPHICS_ASSET_RING_BYTES {
        return Err(GtopSheafError::SizeLimit);
    }
    let footer_start = segment.len() - GTOP_SHEAF_FOOTER_BYTES;
    let footer = &segment[footer_start..];
    let version = u16::from_le_bytes([footer[8], footer[9]]);
    if version != GTOP_SHEAF_FOOTER_VERSION {
        return Err(GtopSheafError::UnsupportedVersion(version));
    }
    if crc32c(&footer[..60]) != le_u32(footer, 60) {
        return Err(GtopSheafError::Checksum);
    }
    let flags = u16::from_le_bytes([footer[10], footer[11]]);
    if flags & !PRESENT != 0 || footer[48..60].iter().any(|&b| b != 0) {
        return Err(GtopSheafError::InvalidFooter);
    }
    let base_len = le_u32(footer, 12) as usize;
    let schema_hash = le_u64(footer, 16);
    if schema_hash == 0 {
        return Err(GtopSheafError::InvalidSchemaHash);
    }
    if base_len > footer_start || base_len < 64 || base_len % 64 != 0 {
        return Err(GtopSheafError::InvalidBase);
    }
    let base = &segment[..base_len];
    let (geometry_id, base_crc) = verify_base(base)?;
    if geometry_id != le_u64(footer, 32) || base_crc != le_u32(footer, 40) {
        return Err(GtopSheafError::InvalidBase);
    }
    let offset = le_u32(footer, 24);
    let length = le_u32(footer, 28) as usize;
    let expected_crc = le_u32(footer, 44);
    let restrictions = if flags & PRESENT != 0 {
        let block_end = base_len.checked_add(length).ok_or(GtopSheafError::SizeLimit)?;
        if offset as usize != base_len || length > GTOP_RESTRICTION_CSR_BYTES_MAX
            || align(block_end, 64) != Some(footer_start) || block_end > footer_start
            || segment[block_end..footer_start].iter().any(|&b| b != 0)
        {
            return Err(GtopSheafError::InvalidFooter);
        }
        let block = &segment[base_len..block_end];
        if crc32c(block) != expected_crc {
            return Err(GtopSheafError::Checksum);
        }
        Some(read_csr(block)?)
    } else {
        if offset != 0 || length != 0 || expected_crc != 0 || base_len != footer_start {
            return Err(GtopSheafError::InvalidFooter);
        }
        None
    };
    Ok(Some(GtopSheafView { geometry_id, stalk_schema_q_hash: schema_hash, base_segment_bytes: base_len,
        restriction_csr_offset: restrictions.map(|_| offset), restrictions }))
}

/// Structurally admit a raw compiler-produced GTOP segment even when no footer is present.
/// This LE parser accepts unaligned slices such as a MID2 field inside a mapped `.10d`.
pub fn admit_gtop_segment_sheaf(segment: &[u8]) -> Result<Option<GtopSheafView<'_>>, GtopSheafError> {
    if segment.len() < 64 || segment.len() % 64 != 0 || segment.len() > GRAPHICS_ASSET_RING_BYTES {
        return Err(GtopSheafError::InvalidBase);
    }
    let sheaf = read_gtop_sheaf_metadata(segment)?;
    let base = &segment[..sheaf.map_or(segment.len(), |metadata| metadata.base_segment_bytes)];
    let geometry_id = match sheaf {
        Some(metadata) => metadata.geometry_id,
        None => verify_base(base)?.0,
    };
    let graph_count = u16::from_le_bytes([base[48], base[49]]) as usize;
    let active_graph = u16::from_le_bytes([base[50], base[51]]) as usize;
    if GeometryComplexKind::from_u8(base[56]).is_none() || GeometryMetricKind::from_u8(base[57]).is_none()
        || (graph_count != 0 && active_graph >= graph_count)
    {
        return Err(GtopSheafError::InvalidBase);
    }
    let directory = le_u32(base, 44) as usize;
    let mut cursor = directory + graph_count * 64;
    for slot in 0..graph_count {
        let start = directory + slot * 64;
        let graph = &base[start..start + 64];
        if le_u64(graph, 8) != geometry_id || GraphEmbedRole::from_u8(graph[56]).is_none()
            || GraphMorphology::from_u8(graph[57]).is_none()
        {
            return Err(GtopSheafError::InvalidBase);
        }
        let vertices = le_u32(graph, 44) as usize;
        let edges = le_u32(graph, 48) as usize;
        let anchor_bytes = vertices.checked_mul(16).ok_or(GtopSheafError::InvalidBase)?;
        let pointer_bytes = vertices.checked_add(1).and_then(|count| count.checked_mul(4))
            .ok_or(GtopSheafError::InvalidBase)?;
        let edge_bytes = edges.checked_mul(4).ok_or(GtopSheafError::InvalidBase)?;
        let mut crc = crc32c_update(u32::MAX, &graph[..60]);
        for (field, length) in [(16, anchor_bytes), (20, pointer_bytes), (24, edge_bytes),
            (28, edge_bytes), (32, pointer_bytes), (36, edge_bytes), (40, edge_bytes)]
        {
            let offset = le_u32(graph, field) as usize;
            if field == 40 && offset == 0 {
                continue;
            }
            let end = offset.checked_add(length).filter(|&end| end <= base.len())
                .ok_or(GtopSheafError::InvalidBase)?;
            if offset != cursor {
                return Err(GtopSheafError::InvalidBase);
            }
            cursor = align(end, 64).filter(|&end| end <= base.len()).ok_or(GtopSheafError::InvalidBase)?;
            let data = &base[offset..end];
            crc = crc32c_update(crc, data);
            match field {
                20 | 32 => verify_graph_rows(data, edges)?,
                24 | 36 if data.chunks_exact(4).any(|word| le_u32(word, 0) as usize >= vertices) => {
                    return Err(GtopSheafError::InvalidBase);
                }
                _ => {}
            }
        }
        if !crc != le_u32(graph, 60) {
            return Err(GtopSheafError::Checksum);
        }
    }
    // The existing compiler emits no separate geometry or DDG region. If either is
    // declared by a legacy producer, its position must still belong to the base.
    let geometry_offset = le_u32(base, 36) as usize;
    let ddg_offset = le_u32(base, 40) as usize;
    let auxiliary = [geometry_offset, ddg_offset];
    if auxiliary.iter().all(|&offset| offset == 0) {
        if cursor != base.len() {
            return Err(GtopSheafError::InvalidBase);
        }
    } else if auxiliary.iter().any(|&offset| offset != 0 && !(cursor..base.len()).contains(&offset)) {
        return Err(GtopSheafError::InvalidBase);
    }
    Ok(sheaf)
}

fn verify_graph_rows(bytes: &[u8], edges: usize) -> Result<(), GtopSheafError> {
    let mut previous = 0;
    if le_u32(bytes, 0) != 0 {
        return Err(GtopSheafError::InvalidBase);
    }
    for word in bytes.chunks_exact(4) {
        let next = le_u32(word, 0) as usize;
        if next < previous || next > edges {
            return Err(GtopSheafError::InvalidBase);
        }
        previous = next;
    }
    if previous != edges {
        return Err(GtopSheafError::InvalidBase);
    }
    Ok(())
}

fn read_csr(block: &[u8]) -> Result<GtopRestrictionCsrView<'_>, GtopSheafError> {
    if block.len() < 16 || block[..4] != CSR_MAGIC {
        return Err(GtopSheafError::InvalidCsr);
    }
    let rows = le_u32(block, 4);
    let columns = le_u32(block, 8);
    let nnz = le_u32(block, 12);
    let layout = csr_layout(rows, columns, nnz as usize)?;
    if layout.coefficients.end != block.len() || block[layout.column_indices.end..layout.coefficients.start].iter().any(|&b| b != 0) {
        return Err(GtopSheafError::InvalidCsr);
    }
    let view = GtopRestrictionCsrView { rows, columns,
        row_offsets: &block[layout.row_offsets], column_indices: &block[layout.column_indices],
        coefficients: &block[layout.coefficients] };
    let mut previous = le_u32(view.row_offsets, 0);
    if previous != 0 || le_u32(view.row_offsets, rows as usize * 4) != nnz {
        return Err(GtopSheafError::InvalidCsr);
    }
    for row_offset in view.row_offsets[4..].chunks_exact(4) {
        let next = le_u32(row_offset, 0);
        if next < previous || next > nnz {
            return Err(GtopSheafError::InvalidCsr);
        }
        let mut prior_column = None;
        for i in previous as usize..next as usize {
            let column = le_u32(view.column_indices, i * 4);
            let coefficient = f64::from_bits(le_u64(view.coefficients, i * 8));
            if column >= columns || prior_column.is_some_and(|prior| column <= prior) || !coefficient.is_finite() {
                return Err(GtopSheafError::InvalidCsr);
            }
            prior_column = Some(column);
        }
        previous = next;
    }
    Ok(view)
}

fn has_footer_marker(segment: &[u8]) -> bool {
    segment.len() >= 128 && segment[segment.len() - GTOP_SHEAF_FOOTER_BYTES..][..8] == GTOP_SHEAF_FOOTER_MAGIC
}

fn verify_base(base: &[u8]) -> Result<(u64, u32), GtopSheafError> {
    let directory = le_u32(base, 44) as usize;
    let graphs = u16::from_le_bytes([base[48], base[49]]) as usize;
    if directory < 64 || directory % 64 != 0 || directory.checked_add(graphs * 64).is_none_or(|end| end > base.len()) {
        return Err(GtopSheafError::InvalidBase);
    }
    let expected = le_u32(base, 60);
    let state = crc32c_update(u32::MAX, &base[..60]);
    if !crc32c_update(state, &base[64..]) != expected {
        return Err(GtopSheafError::Checksum);
    }
    Ok((le_u64(base, 0), expected))
}

fn le_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("validated GTOP field length"))
}

fn le_u64(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().expect("validated GTOP field length"))
}
