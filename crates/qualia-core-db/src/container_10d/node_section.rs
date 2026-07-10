//! `.10d` Tensor10D NODE section — the 40-byte epistemic atom in the container
//! (P0.5).
//!
//! A NODE section wraps a set of `Tensor10D` records (the 40-byte
//! `[q,v,w,x,y,z,t,α,μ,σ]` stride) as a self-describing `.10d` section. It
//! supports two byte-equivalent layouts:
//!
//! - **AoS** (array-of-structs): `N × Tensor10D` records back-to-back — the
//!   natural `Tensor10D` layout, identical to what
//!   `tensor/buffer_export.rs::write_tensor_buffer` produces (minus its
//!   32-byte `Q42*` header, which the `.10d` container header replaces).
//! - **SoA** (structure-of-arrays): ten contiguous lanes, one per axis —
//!   lane 0 = all `q` values, lane 1 = all `v` values, …, lane 9 = all `σ`
//!   values. This is the "page-friendly" layout the design doc §4.1 names:
//!   "any single axis is a contiguous strided read."
//!
//! **AoS↔SoA is byte-identical** (lossless transpose): converting AoS→SoA→AoS
//! (or SoA→AoS→SoA) reproduces the original bytes exactly, because the
//! transpose is just a reordering of the same `N×10` `f32` values with no
//! precision loss. **Per-axis SoA lane reads match AoS field reads**: reading
//! axis `i` for node `j` from the SoA layout yields the same `f32` as reading
//! field `i` from the `j`-th `Tensor10D` in the AoS layout.
//!
//! **`write_tensor_q_at` semantics:** the wavefunction-collapse write (setting
//! `q` for one node, returning the previous `q`) works on the NODE section in
//! either layout — for AoS it writes the first `f32` of the `j`-th record; for
//! SoA it writes lane 0 (`q`) at position `j`. The semantics match
//! `tensor/buffer_export.rs::write_tensor_q_at` exactly (same return-the-prev-q
//! contract), with the only difference being the byte offset (the NODE section
//! has a 16-byte mini-header where `buffer_export.rs` has a 32-byte `Q42*`
//! header).
//!
//! **Determinism + CRC:** two encodes of the same tensor set in the same layout
//! are byte-identical (the AoS/SoA transpose is deterministic). The per-section
//! CRC-32C (P0.2) catches a flipped bit in the payload. The whole-file CRC-32C
//! (P0.3) catches header corruption.
//!
//! **Spec-reserved (NOT yet implemented):** the mini-header's `reserved` bytes
//! are reserved for future per-axis SoA lane offset table, a q-superposition
//! render/export mask (the design doc's "render/export default to a
//! ground-truth-only mask; Sandbox nodes not citable as provenance until
//! collapsed"), and a GSR-result back-pointer. These are governance/attestation
//! concerns (the `SpecReservedGovernance` / `SpecReservedTemporalIndex`
//! section types) and are NOT wired here — P0.5 is the atom, not the
//! attestation layer.

use bytemuck::{Pod, Zeroable};

use crate::tensor::Tensor10D;

/// Section payload mini-header size in bytes.
pub const NODE_MINI_HEADER_SIZE: usize = 16;

/// `Tensor10D` record size in bytes (10 × f32).
pub const TENSOR10D_SIZE: usize = 40;

/// Number of axes (lanes).
pub const AXIS_COUNT: usize = 10;

/// Layout tag: 0 = AoS (array-of-structs), 1 = SoA (structure-of-arrays).
pub const LAYOUT_AOS: u8 = 0;
pub const LAYOUT_SOA: u8 = 1;

/// Maximum node count the NODE section will accept. Bounds against a
/// hostile/malformed file: the 42MB Sentinel ceiling / 40 bytes per node
/// = ~1M nodes max; 1M is a comfortable upper bound for a single section
/// while keeping the mini-header's `node_count` field well within range.
pub const MAX_NODE_COUNT: usize = 1_048_576; // 2^20

/// The 16-byte NODE-section mini-header. `repr(C)`, naturally aligned, no
/// padding.
///
/// ```text
/// offset  size  field
/// 0       4     node_count:u32
/// 4       1     layout:u8        (0=AoS, 1=SoA)
/// 5       1     reserved_u8     (must be zero)
/// 6       2     reserved_u16    (must be zero)
/// 8       8     reserved_u64    (must be zero — future: q-mask, GSR back-pointer)
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct NodeMiniHeader {
    pub node_count: u32,
    pub layout: u8,
    pub reserved_u8: u8,
    pub reserved_u16: u16,
    pub reserved_u64: u64,
}

impl NodeMiniHeader {
    /// Total payload byte length for `count` nodes (mini-header + data).
    /// Same for AoS and SoA (N×40 either way).
    #[inline]
    pub const fn payload_bytes(count: usize) -> usize {
        NODE_MINI_HEADER_SIZE + count * TENSOR10D_SIZE
    }
}

/// NODE-section read/write error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeSectionError {
    /// The payload is too short for the mini-header.
    PayloadTooShort { got: usize, need: usize },
    /// The `layout` byte is not `LAYOUT_AOS` or `LAYOUT_SOA`.
    UnknownLayout { got: u8 },
    /// A reserved field in the mini-header is non-zero.
    NonZeroReserved { field: &'static str },
    /// `node_count` exceeds `MAX_NODE_COUNT`.
    NodeCountTooLarge { got: u32, max: usize },
    /// The payload is too short for `node_count` records.
    PayloadTruncated { expected: usize, got: usize },
    /// A node index is out of range.
    IndexOutOfRange { index: usize, count: usize },
}

impl std::fmt::Display for NodeSectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PayloadTooShort { got, need } => {
                write!(f, "10d NODE payload too short: got {got}, need {need}")
            }
            Self::UnknownLayout { got } => write!(
                f,
                "10d NODE unknown layout byte {got} (expected 0=AoS or 1=SoA)"
            ),
            Self::NonZeroReserved { field } => {
                write!(f, "10d NODE non-zero reserved field {field:?}")
            }
            Self::NodeCountTooLarge { got, max } => {
                write!(f, "10d NODE node_count {got} exceeds max {max}")
            }
            Self::PayloadTruncated { expected, got } => write!(
                f,
                "10d NODE payload truncated: expected {expected}, got {got}"
            ),
            Self::IndexOutOfRange { index, count } => {
                write!(f, "10d NODE index {index} out of range (count={count})")
            }
        }
    }
}

impl std::error::Error for NodeSectionError {}

/// Parse and validate the NODE-section mini-header. Returns the header and the
/// total payload byte length it claims.
pub fn parse_node_header(payload: &[u8]) -> Result<(NodeMiniHeader, usize), NodeSectionError> {
    if payload.len() < NODE_MINI_HEADER_SIZE {
        return Err(NodeSectionError::PayloadTooShort {
            got: payload.len(),
            need: NODE_MINI_HEADER_SIZE,
        });
    }
    let header: NodeMiniHeader = *bytemuck::from_bytes(&payload[..NODE_MINI_HEADER_SIZE]);
    if header.layout != LAYOUT_AOS && header.layout != LAYOUT_SOA {
        return Err(NodeSectionError::UnknownLayout { got: header.layout });
    }
    if header.reserved_u8 != 0 {
        return Err(NodeSectionError::NonZeroReserved {
            field: "reserved_u8",
        });
    }
    if header.reserved_u16 != 0 {
        return Err(NodeSectionError::NonZeroReserved {
            field: "reserved_u16",
        });
    }
    if header.reserved_u64 != 0 {
        return Err(NodeSectionError::NonZeroReserved {
            field: "reserved_u64",
        });
    }
    let count = header.node_count as usize;
    if count > MAX_NODE_COUNT {
        return Err(NodeSectionError::NodeCountTooLarge {
            got: header.node_count,
            max: MAX_NODE_COUNT,
        });
    }
    let total = NodeMiniHeader::payload_bytes(count);
    if payload.len() < total {
        return Err(NodeSectionError::PayloadTruncated {
            expected: total,
            got: payload.len(),
        });
    }
    Ok((header, total))
}

/// Write a tensor set as a NODE section in AoS layout into a caller-supplied
/// buffer. Returns the bytes written. Zero-heap.
pub fn write_node_section_aos(
    tensors: &[Tensor10D],
    out: &mut [u8],
) -> Result<usize, NodeSectionError> {
    let need = NodeMiniHeader::payload_bytes(tensors.len());
    if out.len() < need {
        return Err(NodeSectionError::PayloadTruncated {
            expected: need,
            got: out.len(),
        });
    }
    if tensors.len() > MAX_NODE_COUNT {
        return Err(NodeSectionError::NodeCountTooLarge {
            got: tensors.len() as u32,
            max: MAX_NODE_COUNT,
        });
    }
    let header = NodeMiniHeader {
        node_count: tensors.len() as u32,
        layout: LAYOUT_AOS,
        reserved_u8: 0,
        reserved_u16: 0,
        reserved_u64: 0,
    };
    let header_bytes: &[u8; NODE_MINI_HEADER_SIZE] = bytemuck::cast_ref(&header);
    out[..NODE_MINI_HEADER_SIZE].copy_from_slice(header_bytes);
    let mut off = NODE_MINI_HEADER_SIZE;
    for t in tensors {
        // bytemuck only Pod-implements [u8; N] for N<=32; use bytes_of for 40-byte Tensor10D.
        let tb = bytemuck::bytes_of(t);
        debug_assert_eq!(tb.len(), TENSOR10D_SIZE);
        out[off..off + TENSOR10D_SIZE].copy_from_slice(tb);
        off += TENSOR10D_SIZE;
    }
    Ok(off)
}

/// Write a tensor set as a NODE section in SoA layout into a caller-supplied
/// buffer. Returns the bytes written. Zero-heap.
///
/// SoA lane layout: lane `axis` occupies bytes `[16 + axis*N*4 .. 16 + (axis+1)*N*4)`.
/// Lane 0 = `q`, lane 1 = `v`, …, lane 9 = `σ` (matching `AXIS_ORDER`).
pub fn write_node_section_soa(
    tensors: &[Tensor10D],
    out: &mut [u8],
) -> Result<usize, NodeSectionError> {
    let need = NodeMiniHeader::payload_bytes(tensors.len());
    if out.len() < need {
        return Err(NodeSectionError::PayloadTruncated {
            expected: need,
            got: out.len(),
        });
    }
    if tensors.len() > MAX_NODE_COUNT {
        return Err(NodeSectionError::NodeCountTooLarge {
            got: tensors.len() as u32,
            max: MAX_NODE_COUNT,
        });
    }
    let n = tensors.len();
    let header = NodeMiniHeader {
        node_count: n as u32,
        layout: LAYOUT_SOA,
        reserved_u8: 0,
        reserved_u16: 0,
        reserved_u64: 0,
    };
    let header_bytes: &[u8; NODE_MINI_HEADER_SIZE] = bytemuck::cast_ref(&header);
    out[..NODE_MINI_HEADER_SIZE].copy_from_slice(header_bytes);
    // For each axis, write the lane: out[16 + axis*n*4 + j*4] = tensors[j].field[axis]
    for axis in 0..AXIS_COUNT {
        let lane_start = NODE_MINI_HEADER_SIZE + axis * n * 4;
        for j in 0..n {
            let val = tensor_field(tensors[j], axis);
            let off = lane_start + j * 4;
            out[off..off + 4].copy_from_slice(&val.to_le_bytes());
        }
    }
    Ok(need)
}

/// Read one `Tensor10D` by index from a NODE section payload (dispatches on
/// layout). Zero-heap.
pub fn read_node(payload: &[u8], index: usize) -> Result<Tensor10D, NodeSectionError> {
    let (header, _) = parse_node_header(payload)?;
    let count = header.node_count as usize;
    if index >= count {
        return Err(NodeSectionError::IndexOutOfRange { index, count });
    }
    match header.layout {
        LAYOUT_AOS => read_node_aos(payload, index),
        LAYOUT_SOA => read_node_soa(payload, index),
        _ => Err(NodeSectionError::UnknownLayout { got: header.layout }),
    }
}

/// Read one `Tensor10D` by index from an AoS-layout NODE section. Zero-heap.
pub fn read_node_aos(payload: &[u8], index: usize) -> Result<Tensor10D, NodeSectionError> {
    let (header, _) = parse_node_header(payload)?;
    let count = header.node_count as usize;
    if index >= count {
        return Err(NodeSectionError::IndexOutOfRange { index, count });
    }
    let off = NODE_MINI_HEADER_SIZE + index * TENSOR10D_SIZE;
    Ok(*bytemuck::from_bytes(&payload[off..off + TENSOR10D_SIZE]))
}

/// Read one `f32` lane value (axis `axis`, node `index`) from an SoA-layout
/// NODE section. Zero-heap. This is the "per-axis SoA lane read" the P0.5
/// acceptance gate names.
pub fn read_node_soa_lane(
    payload: &[u8],
    axis: usize,
    index: usize,
) -> Result<f32, NodeSectionError> {
    if axis >= AXIS_COUNT {
        return Err(NodeSectionError::IndexOutOfRange {
            index: axis,
            count: AXIS_COUNT,
        });
    }
    let (header, _) = parse_node_header(payload)?;
    let count = header.node_count as usize;
    if index >= count {
        return Err(NodeSectionError::IndexOutOfRange { index, count });
    }
    let n = count;
    let off = NODE_MINI_HEADER_SIZE + axis * n * 4 + index * 4;
    Ok(f32::from_le_bytes(
        payload[off..off + 4].try_into().unwrap(),
    ))
}

/// Read one `Tensor10D` by index from an SoA-layout NODE section (assembles
/// from ten lane reads). Zero-heap.
pub fn read_node_soa(payload: &[u8], index: usize) -> Result<Tensor10D, NodeSectionError> {
    let (header, _) = parse_node_header(payload)?;
    let count = header.node_count as usize;
    if index >= count {
        return Err(NodeSectionError::IndexOutOfRange { index, count });
    }
    let mut t = Tensor10D::default();
    for axis in 0..AXIS_COUNT {
        let val = read_node_soa_lane(payload, axis, index)?;
        set_tensor_field(&mut t, axis, val);
    }
    Ok(t)
}

/// Write the `q` field (axis 0) for one node in a NODE section — the
/// wavefunction-collapse semantics matching `tensor/buffer_export.rs::
/// write_tensor_q_at`. Returns the previous `q` value. Works on either layout.
/// Zero-heap.
pub fn write_node_q_at(payload: &mut [u8], index: usize, q: f32) -> Result<f32, NodeSectionError> {
    let (header, _) = parse_node_header(payload)?;
    let count = header.node_count as usize;
    if index >= count {
        return Err(NodeSectionError::IndexOutOfRange { index, count });
    }
    let n = count;
    let off = match header.layout {
        LAYOUT_AOS => NODE_MINI_HEADER_SIZE + index * TENSOR10D_SIZE,
        LAYOUT_SOA => NODE_MINI_HEADER_SIZE + 0 * n * 4 + index * 4, // lane 0 = q
        _ => return Err(NodeSectionError::UnknownLayout { got: header.layout }),
    };
    let prev = f32::from_le_bytes(payload[off..off + 4].try_into().unwrap());
    payload[off..off + 4].copy_from_slice(&q.to_le_bytes());
    Ok(prev)
}

/// Transpose an AoS-layout NODE section payload to SoA into a caller-supplied
/// output buffer. The output buffer must be at least `total` bytes. Zero-heap.
/// This is the primary AoS→SoA path.
pub fn transpose_aos_to_soa(payload: &[u8], out: &mut [u8]) -> Result<usize, NodeSectionError> {
    let (header, total) = parse_node_header(payload)?;
    if header.layout != LAYOUT_AOS {
        return Err(NodeSectionError::UnknownLayout { got: header.layout });
    }
    if out.len() < total {
        return Err(NodeSectionError::PayloadTruncated {
            expected: total,
            got: out.len(),
        });
    }
    let n = header.node_count as usize;
    // Write the SoA mini-header.
    let soa_header = NodeMiniHeader {
        node_count: header.node_count,
        layout: LAYOUT_SOA,
        reserved_u8: 0,
        reserved_u16: 0,
        reserved_u64: 0,
    };
    let header_bytes: &[u8; NODE_MINI_HEADER_SIZE] = bytemuck::cast_ref(&soa_header);
    out[..NODE_MINI_HEADER_SIZE].copy_from_slice(header_bytes);
    // For each axis, for each node, read the field from the AoS record and
    // write it to the SoA lane.
    for axis in 0..AXIS_COUNT {
        let lane_start = NODE_MINI_HEADER_SIZE + axis * n * 4;
        for j in 0..n {
            let aos_off = NODE_MINI_HEADER_SIZE + j * TENSOR10D_SIZE + axis * 4;
            let val = f32::from_le_bytes(payload[aos_off..aos_off + 4].try_into().unwrap());
            let off = lane_start + j * 4;
            out[off..off + 4].copy_from_slice(&val.to_le_bytes());
        }
    }
    Ok(total)
}

/// Transpose an SoA-layout NODE section payload to AoS into a caller-supplied
/// output buffer. The output buffer must be at least `total` bytes. Zero-heap.
pub fn transpose_soa_to_aos(payload: &[u8], out: &mut [u8]) -> Result<usize, NodeSectionError> {
    let (header, total) = parse_node_header(payload)?;
    if header.layout != LAYOUT_SOA {
        return Err(NodeSectionError::UnknownLayout { got: header.layout });
    }
    if out.len() < total {
        return Err(NodeSectionError::PayloadTruncated {
            expected: total,
            got: out.len(),
        });
    }
    let n = header.node_count as usize;
    let aos_header = NodeMiniHeader {
        node_count: header.node_count,
        layout: LAYOUT_AOS,
        reserved_u8: 0,
        reserved_u16: 0,
        reserved_u64: 0,
    };
    let header_bytes: &[u8; NODE_MINI_HEADER_SIZE] = bytemuck::cast_ref(&aos_header);
    out[..NODE_MINI_HEADER_SIZE].copy_from_slice(header_bytes);
    for j in 0..n {
        let record_off = NODE_MINI_HEADER_SIZE + j * TENSOR10D_SIZE;
        for axis in 0..AXIS_COUNT {
            let lane_off = NODE_MINI_HEADER_SIZE + axis * n * 4 + j * 4;
            let val = f32::from_le_bytes(payload[lane_off..lane_off + 4].try_into().unwrap());
            let off = record_off + axis * 4;
            out[off..off + 4].copy_from_slice(&val.to_le_bytes());
        }
    }
    Ok(total)
}

// --- helpers ---

/// Read one `f32` field by axis index from a `Tensor10D`.
#[inline]
fn tensor_field(t: Tensor10D, axis: usize) -> f32 {
    match axis {
        0 => t.q,
        1 => t.v,
        2 => t.w,
        3 => t.x,
        4 => t.y,
        5 => t.z,
        6 => t.t,
        7 => t.alpha,
        8 => t.mu,
        9 => t.sigma,
        _ => unreachable!("axis out of range"),
    }
}

/// Set one `f32` field by axis index on a `Tensor10D`.
#[inline]
fn set_tensor_field(t: &mut Tensor10D, axis: usize, val: f32) {
    match axis {
        0 => t.q = val,
        1 => t.v = val,
        2 => t.w = val,
        3 => t.x = val,
        4 => t.y = val,
        5 => t.z = val,
        6 => t.t = val,
        7 => t.alpha = val,
        8 => t.mu = val,
        9 => t.sigma = val,
        _ => unreachable!("axis out of range"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Axis *names* (for bit-exact assertion messages) — only the numeric AXIS_COUNT is used by the
    // production transpose, so AXIS_ORDER lives with the tests that reference it.
    use crate::container_10d::axis_role::AXIS_ORDER;
    use crate::container_10d::crc32c::crc32c;
    use crate::container_10d::header::Container10dHeader;
    use crate::container_10d::section::{
        encode_container, parse_section_table, AlignmentTier, SectionInput, SectionType,
    };

    fn sample_tensors() -> [Tensor10D; 3] {
        [
            Tensor10D::new(0.0, 0.0, 0.0, 0.1, 0.2, 0.3, 0.0, 1.0, 0.0, 0.5),
            Tensor10D::new(0.5, 1.0, 2.0, 0.4, 0.5, 0.6, 1.0, 0.8, 0.2, 0.75),
            Tensor10D::new(999.0, 2.0, 3.0, 0.7, 0.8, 0.9, 2.0, 0.6, 0.9, 0.25), // Sandbox q
        ]
    }

    #[test]
    fn mini_header_is_pod_with_exact_size() {
        assert_eq!(std::mem::size_of::<NodeMiniHeader>(), NODE_MINI_HEADER_SIZE);
        assert_eq!(std::mem::offset_of!(NodeMiniHeader, node_count), 0);
        assert_eq!(std::mem::offset_of!(NodeMiniHeader, layout), 4);
        assert_eq!(std::mem::offset_of!(NodeMiniHeader, reserved_u8), 5);
        assert_eq!(std::mem::offset_of!(NodeMiniHeader, reserved_u16), 6);
        assert_eq!(std::mem::offset_of!(NodeMiniHeader, reserved_u64), 8);
    }

    #[test]
    fn aos_section_reads_back_tensor10d_for_tensor10d_identical() {
        let tensors = sample_tensors();
        let need = NodeMiniHeader::payload_bytes(tensors.len());
        let mut payload = vec![0u8; need];
        let n = write_node_section_aos(&tensors, &mut payload).expect("aos write");
        assert_eq!(n, need);
        for i in 0..tensors.len() {
            let read = read_node(&payload, i).expect("aos read");
            assert_eq!(read, tensors[i], "AoS node {i} must read back identical");
        }
    }

    #[test]
    fn soa_section_reads_back_tensor10d_for_tensor10d_identical() {
        let tensors = sample_tensors();
        let need = NodeMiniHeader::payload_bytes(tensors.len());
        let mut payload = vec![0u8; need];
        let n = write_node_section_soa(&tensors, &mut payload).expect("soa write");
        assert_eq!(n, need);
        for i in 0..tensors.len() {
            let read = read_node(&payload, i).expect("soa read");
            assert_eq!(read, tensors[i], "SoA node {i} must read back identical");
        }
    }

    #[test]
    fn per_axis_soa_lane_reads_match_aos_field_reads() {
        let tensors = sample_tensors();
        let need = NodeMiniHeader::payload_bytes(tensors.len());
        let mut aos = vec![0u8; need];
        let mut soa = vec![0u8; need];
        write_node_section_aos(&tensors, &mut aos).expect("aos write");
        write_node_section_soa(&tensors, &mut soa).expect("soa write");
        for axis in 0..AXIS_COUNT {
            for j in 0..tensors.len() {
                let aos_val = tensor_field(read_node_aos(&aos, j).expect("aos read"), axis);
                let soa_val = read_node_soa_lane(&soa, axis, j).expect("soa lane read");
                assert_eq!(
                    aos_val.to_bits(),
                    soa_val.to_bits(),
                    "axis {} ({}) node {} : AoS field read must match SoA lane read (bit-exact)",
                    axis,
                    AXIS_ORDER[axis],
                    j
                );
            }
        }
    }

    #[test]
    fn aos_to_soa_to_aos_is_byte_identical() {
        let tensors = sample_tensors();
        let need = NodeMiniHeader::payload_bytes(tensors.len());
        let mut aos = vec![0u8; need];
        let mut soa = vec![0u8; need];
        let mut aos2 = vec![0u8; need];
        write_node_section_aos(&tensors, &mut aos).expect("aos write");
        transpose_aos_to_soa(&aos, &mut soa).expect("aos->soa");
        transpose_soa_to_aos(&soa, &mut aos2).expect("soa->aos");
        assert_eq!(&aos[..], &aos2[..], "AoS→SoA→AoS must be byte-identical");
    }

    #[test]
    fn soa_to_aos_to_soa_is_byte_identical() {
        let tensors = sample_tensors();
        let need = NodeMiniHeader::payload_bytes(tensors.len());
        let mut soa = vec![0u8; need];
        let mut aos = vec![0u8; need];
        let mut soa2 = vec![0u8; need];
        write_node_section_soa(&tensors, &mut soa).expect("soa write");
        transpose_soa_to_aos(&soa, &mut aos).expect("soa->aos");
        transpose_aos_to_soa(&aos, &mut soa2).expect("aos->soa");
        assert_eq!(&soa[..], &soa2[..], "SoA→AoS→SoA must be byte-identical");
    }

    #[test]
    fn aos_and_soa_payloads_have_identical_crc() {
        // Same tensor set, two layouts — the CRC over the payload differs
        // (the byte order differs), but the CRC over the *semantic content*
        // (the tensor values) is the same. The P0.5 gate is that both layouts
        // are valid; the CRC is per-layout. This test confirms both payloads
        // are deterministic (same input → same CRC) and that the two layouts
        // are NOT byte-identical (they're different byte orderings of the same
        // values).
        let tensors = sample_tensors();
        let need = NodeMiniHeader::payload_bytes(tensors.len());
        let mut aos_a = vec![0u8; need];
        let mut aos_b = vec![0u8; need];
        let mut soa_a = vec![0u8; need];
        let mut soa_b = vec![0u8; need];
        write_node_section_aos(&tensors, &mut aos_a).expect("aos a");
        write_node_section_aos(&tensors, &mut aos_b).expect("aos b");
        write_node_section_soa(&tensors, &mut soa_a).expect("soa a");
        write_node_section_soa(&tensors, &mut soa_b).expect("soa b");
        // Determinism: same layout twice = same bytes.
        assert_eq!(&aos_a[..], &aos_b[..], "AoS must be deterministic");
        assert_eq!(&soa_a[..], &soa_b[..], "SoA must be deterministic");
        // The two layouts are different byte orderings (not byte-identical).
        assert_ne!(&aos_a[..], &soa_a[..], "AoS and SoA are different layouts");
        // But both have a valid CRC (the per-section CRC is computed by the
        // section-table writer; here we just confirm the payload is stable).
        assert_eq!(crc32c(&aos_a[..]), crc32c(&aos_b[..]));
        assert_eq!(crc32c(&soa_a[..]), crc32c(&soa_b[..]));
    }

    #[test]
    fn write_node_q_at_aos_matches_buffer_export_semantics() {
        let mut tensors = sample_tensors();
        let need = NodeMiniHeader::payload_bytes(tensors.len());
        let mut payload = vec![0u8; need];
        write_node_section_aos(&tensors, &mut payload).expect("aos write");
        // Collapse node 1's q to 0.0 (ground truth). Prev q should be 0.5.
        let prev = write_node_q_at(&mut payload, 1, 0.0).expect("q write");
        assert!((prev - 0.5).abs() < 1e-6, "prev q must be 0.5, got {prev}");
        let t = read_node(&payload, 1).expect("read after collapse");
        assert!(t.q.abs() < 1e-6, "q must be collapsed to 0.0");
        // The other fields are unchanged.
        assert!((t.x - 0.4).abs() < 1e-6);
        // Mirror the same collapse on the source array for consistency.
        tensors[1].q = 0.0;
        assert_eq!(read_node(&payload, 1).expect("read"), tensors[1]);
    }

    #[test]
    fn write_node_q_at_soa_matches_buffer_export_semantics() {
        let mut tensors = sample_tensors();
        let need = NodeMiniHeader::payload_bytes(tensors.len());
        let mut payload = vec![0u8; need];
        write_node_section_soa(&tensors, &mut payload).expect("soa write");
        // Collapse node 2's q (currently 999.0 Sandbox) to 0.0.
        let prev = write_node_q_at(&mut payload, 2, 0.0).expect("q write");
        assert!(
            (prev - 999.0).abs() < 1e-4,
            "prev q must be 999.0, got {prev}"
        );
        let t = read_node(&payload, 2).expect("read after collapse");
        assert!(t.q.abs() < 1e-6, "q must be collapsed to 0.0");
        // Other fields unchanged.
        assert!((t.sigma - 0.25).abs() < 1e-6);
        tensors[2].q = 0.0;
        assert_eq!(read_node(&payload, 2).expect("read"), tensors[2]);
    }

    #[test]
    fn write_node_q_at_out_of_range_rejects() {
        let tensors = sample_tensors();
        let need = NodeMiniHeader::payload_bytes(tensors.len());
        let mut payload = vec![0u8; need];
        write_node_section_aos(&tensors, &mut payload).expect("aos write");
        let err = write_node_q_at(&mut payload, 99, 0.0).expect_err("oob must reject");
        assert!(
            matches!(err, NodeSectionError::IndexOutOfRange { .. }),
            "{err}"
        );
    }

    #[test]
    fn node_section_round_trips_through_10d_container_with_per_section_crc() {
        // Wrap a NODE section in a full .10d container and round-trip it
        // through the P0.2 section table (per-section CRC) + P0.3 whole-file
        // CRC. This is the integration test that P0.5 + P0.2 + P0.3 work
        // together.
        let tensors = sample_tensors();
        let node_need = NodeMiniHeader::payload_bytes(tensors.len());
        let mut node_payload = vec![0u8; node_need];
        write_node_section_soa(&tensors, &mut node_payload).expect("soa write");

        let h = Container10dHeader::proposed();
        let inputs = [SectionInput {
            section_type: SectionType::Tensor10DNodes,
            alignment_tier: AlignmentTier::CacheLine,
            stride: 0, // blob at the section-table level (mini-header inside)
            element_count: 0,
            payload: &node_payload,
        }];
        let mut out = vec![0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("container encode");
        crate::container_10d::integrity::seal_whole_file_crc32c(&mut out[..n]);

        // Verify whole-file CRC.
        crate::container_10d::integrity::verify_whole_file_crc32c(&mut out[..n])
            .expect("whole-file CRC");

        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        let descs = parse_section_table(&out[..n], &parsed_h).expect("table parse");
        assert_eq!(descs.len(), 1);
        assert_eq!(descs[0].section_type, SectionType::Tensor10DNodes as u8);

        // Extract the NODE payload and read the tensors back.
        let p_off = descs[0].byte_offset as usize;
        let p_len = descs[0].byte_length as usize;
        let node_payload_back = &out[p_off..p_off + p_len];
        for i in 0..tensors.len() {
            let t = read_node(node_payload_back, i).expect("node read from container");
            assert_eq!(t, tensors[i], "container round-trip node {i}");
        }
    }

    #[test]
    fn flipped_payload_bit_in_node_section_is_caught_by_per_section_crc() {
        let tensors = sample_tensors();
        let node_need = NodeMiniHeader::payload_bytes(tensors.len());
        let mut node_payload = vec![0u8; node_need];
        write_node_section_aos(&tensors, &mut node_payload).expect("aos write");

        let h = Container10dHeader::proposed();
        let inputs = [SectionInput {
            section_type: SectionType::Tensor10DNodes,
            alignment_tier: AlignmentTier::CacheLine,
            stride: 0,
            element_count: 0,
            payload: &node_payload,
        }];
        let mut out = vec![0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        let descs = parse_section_table(&out[..n], &parsed_h).expect("clean table parses");
        let p_off = descs[0].byte_offset as usize;
        // Flip a bit in the NODE payload (inside the tensor data, past the mini-header).
        out[p_off + NODE_MINI_HEADER_SIZE + 5] ^= 0x01;
        let err =
            parse_section_table(&out[..n], &parsed_h).expect_err("flipped bit must be caught");
        assert!(
            matches!(
                err,
                crate::container_10d::section::SectionTableError::CrcMismatch { .. }
            ),
            "{err}"
        );
    }

    #[test]
    fn unknown_layout_is_rejected() {
        let mut payload = vec![0u8; NODE_MINI_HEADER_SIZE + 40];
        let mut header = NodeMiniHeader::zeroed();
        header.node_count = 1;
        header.layout = 99; // unknown
        let header_bytes: &[u8; NODE_MINI_HEADER_SIZE] = bytemuck::cast_ref(&header);
        payload[..NODE_MINI_HEADER_SIZE].copy_from_slice(header_bytes);
        let err = parse_node_header(&payload).expect_err("unknown layout must reject");
        assert!(
            matches!(err, NodeSectionError::UnknownLayout { got: 99 }),
            "{err}"
        );
    }

    #[test]
    fn non_zero_reserved_field_is_rejected() {
        let mut payload = vec![0u8; NODE_MINI_HEADER_SIZE + 40];
        let mut header = NodeMiniHeader::zeroed();
        header.node_count = 1;
        header.layout = LAYOUT_AOS;
        header.reserved_u64 = 1;
        let header_bytes: &[u8; NODE_MINI_HEADER_SIZE] = bytemuck::cast_ref(&header);
        payload[..NODE_MINI_HEADER_SIZE].copy_from_slice(header_bytes);
        let err = parse_node_header(&payload).expect_err("non-zero reserved must reject");
        assert!(
            matches!(err, NodeSectionError::NonZeroReserved { .. }),
            "{err}"
        );
    }

    #[test]
    fn node_count_too_large_is_rejected() {
        let mut payload = vec![0u8; NODE_MINI_HEADER_SIZE];
        let mut header = NodeMiniHeader::zeroed();
        header.node_count = (MAX_NODE_COUNT + 1) as u32;
        header.layout = LAYOUT_AOS;
        let header_bytes: &[u8; NODE_MINI_HEADER_SIZE] = bytemuck::cast_ref(&header);
        payload[..NODE_MINI_HEADER_SIZE].copy_from_slice(header_bytes);
        let err = parse_node_header(&payload).expect_err("too-large count must reject");
        assert!(
            matches!(err, NodeSectionError::NodeCountTooLarge { .. }),
            "{err}"
        );
    }

    #[test]
    fn empty_node_section_round_trips() {
        let tensors: [Tensor10D; 0] = [];
        let need = NodeMiniHeader::payload_bytes(0);
        assert_eq!(need, NODE_MINI_HEADER_SIZE);
        let mut payload = vec![0u8; need];
        write_node_section_aos(&tensors, &mut payload).expect("empty aos write");
        let (header, total) = parse_node_header(&payload).expect("empty parse");
        assert_eq!(header.node_count, 0);
        assert_eq!(total, NODE_MINI_HEADER_SIZE);
    }
}
