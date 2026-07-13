//! Normative `.10d` container header — the barrier surface every later P0
//! task consumes.
//!
//! Layout: 64 bytes, `repr(C)`, all fields naturally aligned, one explicit
//! `pad0[2]` field that must be zero. The header carries:
//!
//! - magic + version (the parse-rejection gates for bad magic / unknown version)
//! - `axis_roles[10]` — the normative axis-role taxonomy (one `AxisRole` per
//!   axis in `AXIS_ORDER`); any `Undefined` entry is rejected
//! - `metric_descriptor` — the [`super::metric_check::MetricCompletenessDescriptor`]
//!   verified against `Tensor10D::full_distance`'s actual v-branch behaviour;
//!   a diverging descriptor is rejected (the "queryability claim == code" gate)
//! - `header_crc32c` — **spec-reserved in P0.1**: the field exists and is
//!   written as zero on encode, but P0.3 wires the shared CRC-32C (delegated
//!   from `q42/p64_weight.rs`) and starts enforcing it. P0.1 does not enforce
//!   the CRC — that is P0.3's acceptance gate, not P0.1's.
//! - `reserved[8]` — zero, future use (governance default-disposition flags,
//!   capability bits, time-base selector — all P0.2+ territory).
//!
//! The header is the foundation every later P0 task writes into, so its byte
//! layout is frozen at v1 and asserted by `header_is_pod_with_exact_size`:
//! `size_of::<Container10dHeader>() == 64` and the named pad/reserved fields
//! are zero.

use bytemuck::{Pod, Zeroable};

use crate::container_10d::axis_role::{AxisRole, PROPOSED_AXIS_ROLES};
use crate::container_10d::metric_check::{
    proposed_metric_descriptor, verify_descriptor_against_reality, MetricCompletenessDescriptor,
};

/// `.10d` container magic — ASCII `"10d\0"`.
pub const MAGIC_10D: [u8; 4] = *b"10d\0";

/// `.10d` header version. Increment only when the POD layout or
/// caller-buffer contract changes (forward-compat is by version, not by
/// flag bits).
pub const HEADER_VERSION: u16 = 1;

/// Header flag bit 0: default disposition = Refuse. A reader that ignores the
/// Governance section still fails closed. Always set in v1 headers produced by
/// [`Container10dHeader::proposed`].
pub const FLAG_DEFAULT_DISPOSITION_REFUSE: u16 = 1 << 0;

/// Exact byte size of the header POD. Asserted by the size-of test so a
/// future field addition cannot silently shift the layout.
pub const HEADER_BYTE_SIZE: usize = 64;

/// Parse error categories — one per acceptance-gate rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderParseError {
    /// Input shorter than `HEADER_BYTE_SIZE`.
    TooShort { got: usize },
    /// Magic bytes do not match `MAGIC_10D`.
    BadMagic { got: [u8; 4] },
    /// Version is not `HEADER_VERSION`.
    UnknownVersion { got: u16 },
    /// A padding/reserved field is non-zero (the "zero padding" gate).
    NonZeroPadding { field: &'static str },
    /// An axis role byte is not a defined `AxisRole` variant.
    UndefinedAxisRole { axis_index: usize, got: u8 },
    /// The metric-completeness descriptor diverges from `full_distance`'s
    /// actual v-branch behaviour. Carries the divergence detail.
    MetricDivergence(String),
    /// The section-table pointer in the header is inconsistent: offset is not
    /// `0` (no table) and not `>= HEADER_BYTE_SIZE` and within the file, or
    /// `section_count` exceeds `MAX_SECTION_COUNT`, or exactly one of
    /// (offset, count) is zero.
    BadSectionTablePointer { offset: u32, count: u32 },
}

impl std::fmt::Display for HeaderParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => write!(
                f,
                "10d header too short: got {got} bytes, need {HEADER_BYTE_SIZE}"
            ),
            Self::BadMagic { got } => write!(f, "10d bad magic: got {got:?}, need {MAGIC_10D:?}"),
            Self::UnknownVersion { got } => {
                write!(f, "10d unknown version: got {got}, need {HEADER_VERSION}")
            }
            Self::NonZeroPadding { field } => write!(f, "10d non-zero padding in field {field:?}"),
            Self::UndefinedAxisRole { axis_index, got } => write!(
                f,
                "10d undefined axis role at index {axis_index}: raw {got}"
            ),
            Self::MetricDivergence(msg) => write!(f, "10d {msg}"),
            Self::BadSectionTablePointer { offset, count } => write!(
                f,
                "10d bad section-table pointer: offset={offset}, count={count}"
            ),
        }
    }
}

impl std::error::Error for HeaderParseError {}

/// Maximum number of section descriptors the parser will accept. Bounds the
/// section-table allocation against a hostile/malformed file. 1024 is far
/// beyond any realistic `.10d` (the design's section types number ~10) while
/// keeping the table trivially small.
pub const MAX_SECTION_COUNT: u32 = 1024;

/// The normative `.10d` v1 header — 64 bytes, `repr(C)`, naturally aligned.
///
/// Field layout (offsets):
/// ```text
/// offset  size  field
/// 0       4     magic
/// 4       2     version
/// 6       2     flags
/// 8       10    axis_roles          (one AxisRole u8 per AXIS_ORDER axis)
/// 18      2     pad0                (must be zero — aligns metric_descriptor to 4)
/// 20      32    metric_descriptor   (4 x MetricBranchDescriptor, 8 bytes each)
/// 52      4     header_crc32c       (spec-reserved in P0.1; P0.3 wires shared CRC-32C)
/// 56      4     section_table_offset (byte offset from file start; 0 = no table)
/// 60      4     section_count        (number of SectionDescriptor rows; 0 = no table)
/// ```
///
/// The `section_table_offset` + `section_count` fields at offsets 56–63 were
/// the `reserved[8]` field in the initial P0.1 landing; P0.2 defines their
/// meaning within v1 (the POD layout is unchanged — only a reserved field's
/// semantics are now specified). See the P0.2 progress-log entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct Container10dHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    pub axis_roles: [u8; 10],
    pub pad0: [u8; 2],
    pub metric_descriptor: MetricCompletenessDescriptor,
    pub header_crc32c: u32,
    /// Byte offset of the section table from the start of the file. `0` means
    /// no section table (a bare header — `section_count` must also be `0`).
    /// Otherwise must be `>= HEADER_BYTE_SIZE` and within the file.
    pub section_table_offset: u32,
    /// Number of `SectionDescriptor` rows in the section table. `0` means no
    /// table (a bare header — `section_table_offset` must also be `0`).
    /// Must be `<= MAX_SECTION_COUNT`.
    pub section_count: u32,
}

impl Default for Container10dHeader {
    fn default() -> Self {
        Self::proposed()
    }
}

impl Container10dHeader {
    /// The proposed (not-yet-frozen) v1 header: Option A axis-role taxonomy +
    /// option (b) metric-completeness descriptor (the documented limitation
    /// matching current `full_distance` reality) + default-disposition-Refuse
    /// flag set. CRC left zero (P0.3 wires the shared CRC-32C).
    pub fn proposed() -> Self {
        let mut axis_roles = [0u8; 10];
        for (i, role) in PROPOSED_AXIS_ROLES.iter().enumerate() {
            axis_roles[i] = *role as u8;
        }
        Self {
            magic: MAGIC_10D,
            version: HEADER_VERSION,
            flags: FLAG_DEFAULT_DISPOSITION_REFUSE,
            axis_roles,
            pad0: [0, 0],
            metric_descriptor: proposed_metric_descriptor(),
            header_crc32c: 0,
            section_table_offset: 0,
            section_count: 0,
        }
    }

    /// Encode the header into a caller-supplied 64-byte buffer (little-endian
    /// where applicable; the POD is already LE-friendly). Zero-alloc.
    pub fn encode(&self, out: &mut [u8; HEADER_BYTE_SIZE]) {
        // SAFETY: Container10dHeader is repr(C) + Pod + size 64. Casting to
        // bytes is sound; copy into the caller buffer.
        let bytes: &[u8; HEADER_BYTE_SIZE] = bytemuck::cast_ref(self);
        *out = *bytes;
    }

    /// Encode into a freshly-owned 64-byte array. Convenience for tests and
    /// writers that are not on a zero-heap hot path.
    pub fn encode_to_vec64(&self) -> [u8; HEADER_BYTE_SIZE] {
        let mut out = [0u8; HEADER_BYTE_SIZE];
        self.encode(&mut out);
        out
    }

    /// Parse and validate a 64-byte header. Runs every P0.1 acceptance gate:
    /// bad magic, unknown version, non-zero structural padding, undefined axis
    /// role, metric-completeness divergence from `full_distance` reality, and
    /// (P0.2) a consistent section-table pointer.
    pub fn parse(data: &[u8]) -> Result<Container10dHeader, HeaderParseError> {
        if data.len() < HEADER_BYTE_SIZE {
            return Err(HeaderParseError::TooShort { got: data.len() });
        }
        let mut buf = [0u8; HEADER_BYTE_SIZE];
        buf.copy_from_slice(&data[..HEADER_BYTE_SIZE]);
        // SAFETY: Container10dHeader is repr(C) + Pod + size 64; the buffer is
        // exactly 64 bytes and initialised.
        let header: Container10dHeader = *bytemuck::from_bytes(&buf);

        if header.magic != MAGIC_10D {
            return Err(HeaderParseError::BadMagic { got: header.magic });
        }
        if header.version != HEADER_VERSION {
            return Err(HeaderParseError::UnknownVersion {
                got: header.version,
            });
        }
        if header.pad0 != [0, 0] {
            return Err(HeaderParseError::NonZeroPadding { field: "pad0" });
        }
        // Section-table pointer consistency (P0.2). Either both zero (no
        // table — a bare header) or both non-zero with offset >= header size,
        // offset within the file, and count <= MAX_SECTION_COUNT. The
        // table-bytes-within-file and per-descriptor validation is done by
        // the section-table reader in `section.rs`; here we only gate the
        // header-level pointer.
        let (off, cnt) = (header.section_table_offset, header.section_count);
        let both_zero = off == 0 && cnt == 0;
        let both_nonzero = off != 0 && cnt != 0;
        let valid_nonzero = both_nonzero
            && off as usize >= HEADER_BYTE_SIZE
            && off as usize <= data.len()
            && cnt <= MAX_SECTION_COUNT;
        if !both_zero && !valid_nonzero {
            return Err(HeaderParseError::BadSectionTablePointer {
                offset: off,
                count: cnt,
            });
        }
        for (i, &raw) in header.axis_roles.iter().enumerate() {
            if AxisRole::from_u8(raw).is_none() || raw == AxisRole::Undefined as u8 {
                return Err(HeaderParseError::UndefinedAxisRole {
                    axis_index: i,
                    got: raw,
                });
            }
        }
        verify_descriptor_against_reality(&header.metric_descriptor)
            .map_err(|d| HeaderParseError::MetricDivergence(d.to_string()))?;
        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::axis_role::{AxisRole, AXIS_ORDER};
    use crate::container_10d::metric_check::{MetricKind, BOUNDARY_CLIQUE_BRANCH_INDEX};

    #[test]
    fn header_is_pod_with_exact_size_and_zero_padding() {
        assert_eq!(
            std::mem::size_of::<Container10dHeader>(),
            HEADER_BYTE_SIZE,
            "header must be exactly {HEADER_BYTE_SIZE} bytes"
        );
        // Offset assertions so doc and code cannot drift.
        assert_eq!(std::mem::offset_of!(Container10dHeader, magic), 0);
        assert_eq!(std::mem::offset_of!(Container10dHeader, version), 4);
        assert_eq!(std::mem::offset_of!(Container10dHeader, flags), 6);
        assert_eq!(std::mem::offset_of!(Container10dHeader, axis_roles), 8);
        assert_eq!(std::mem::offset_of!(Container10dHeader, pad0), 18);
        assert_eq!(
            std::mem::offset_of!(Container10dHeader, metric_descriptor),
            20
        );
        assert_eq!(std::mem::offset_of!(Container10dHeader, header_crc32c), 52);
        assert_eq!(
            std::mem::offset_of!(Container10dHeader, section_table_offset),
            56
        );
        assert_eq!(std::mem::offset_of!(Container10dHeader, section_count), 60);
        // The proposed (bare) header has zero pad and no section table.
        let h = Container10dHeader::proposed();
        assert_eq!(h.pad0, [0, 0]);
        assert_eq!(h.section_table_offset, 0);
        assert_eq!(h.section_count, 0);
    }

    #[test]
    fn encode_is_bit_identical_across_two_runs() {
        let h = Container10dHeader::proposed();
        let mut a = [0u8; HEADER_BYTE_SIZE];
        let mut b = [0u8; HEADER_BYTE_SIZE];
        h.encode(&mut a);
        h.encode(&mut b);
        assert_eq!(
            a, b,
            "two encodes of the same header must be byte-identical"
        );
    }

    #[test]
    fn round_trip_proposed_header() {
        let h = Container10dHeader::proposed();
        let bytes = h.encode_to_vec64();
        let parsed = Container10dHeader::parse(&bytes).expect("proposed header must parse");
        assert_eq!(parsed, h);
    }

    #[test]
    fn parse_rejects_bad_magic() {
        let mut bytes = Container10dHeader::proposed().encode_to_vec64();
        bytes[0] = b'x';
        let err = Container10dHeader::parse(&bytes).expect_err("bad magic must reject");
        assert!(matches!(err, HeaderParseError::BadMagic { .. }), "{err}");
    }

    #[test]
    fn parse_rejects_unknown_version() {
        let mut bytes = Container10dHeader::proposed().encode_to_vec64();
        // version is at offset 4, little-endian u16.
        bytes[4] = 0xff;
        bytes[5] = 0xff;
        let err = Container10dHeader::parse(&bytes).expect_err("unknown version must reject");
        assert!(
            matches!(err, HeaderParseError::UnknownVersion { got: 0xffff }),
            "{err}"
        );
    }

    #[test]
    fn parse_rejects_undefined_axis_role() {
        let mut bytes = Container10dHeader::proposed().encode_to_vec64();
        // axis_roles starts at offset 8; set the first (q) to Undefined (0).
        bytes[8] = AxisRole::Undefined as u8;
        let err = Container10dHeader::parse(&bytes).expect_err("undefined axis role must reject");
        assert!(
            matches!(
                err,
                HeaderParseError::UndefinedAxisRole { axis_index: 0, .. }
            ),
            "{err}"
        );
    }

    #[test]
    fn parse_rejects_unknown_axis_role_byte() {
        let mut bytes = Container10dHeader::proposed().encode_to_vec64();
        // axis_roles[3] (x) — set to an undefined raw value (5).
        bytes[8 + 3] = 5;
        let err =
            Container10dHeader::parse(&bytes).expect_err("unknown axis role byte must reject");
        assert!(
            matches!(
                err,
                HeaderParseError::UndefinedAxisRole {
                    axis_index: 3,
                    got: 5
                }
            ),
            "{err}"
        );
    }

    #[test]
    fn parse_rejects_non_zero_pad0() {
        let mut bytes = Container10dHeader::proposed().encode_to_vec64();
        bytes[18] = 1;
        let err = Container10dHeader::parse(&bytes).expect_err("non-zero pad0 must reject");
        assert!(
            matches!(err, HeaderParseError::NonZeroPadding { field: "pad0" }),
            "{err}"
        );
    }

    #[test]
    fn parse_rejects_section_table_offset_below_header_size() {
        // section_table_offset at offset 56 (u32 LE). Set it to 8 (< 64) with
        // a non-zero count — must reject.
        let mut bytes = Container10dHeader::proposed().encode_to_vec64();
        bytes[56..60].copy_from_slice(&8u32.to_le_bytes());
        bytes[60..64].copy_from_slice(&1u32.to_le_bytes());
        let err = Container10dHeader::parse(&bytes).expect_err("offset < header size must reject");
        assert!(
            matches!(err, HeaderParseError::BadSectionTablePointer { .. }),
            "{err}"
        );
    }

    #[test]
    fn parse_rejects_section_count_without_offset() {
        // offset = 0 but count != 0 — inconsistent, must reject.
        let mut bytes = Container10dHeader::proposed().encode_to_vec64();
        bytes[60..64].copy_from_slice(&1u32.to_le_bytes());
        let err = Container10dHeader::parse(&bytes).expect_err("count without offset must reject");
        assert!(
            matches!(err, HeaderParseError::BadSectionTablePointer { .. }),
            "{err}"
        );
    }

    #[test]
    fn parse_rejects_offset_without_count() {
        // offset != 0 but count = 0 — inconsistent, must reject.
        let mut bytes = Container10dHeader::proposed().encode_to_vec64();
        bytes[56..60].copy_from_slice(&64u32.to_le_bytes());
        let err = Container10dHeader::parse(&bytes).expect_err("offset without count must reject");
        assert!(
            matches!(err, HeaderParseError::BadSectionTablePointer { .. }),
            "{err}"
        );
    }

    #[test]
    fn parse_accepts_bare_header_no_section_table() {
        // The proposed header has offset=0, count=0 — a bare header. Must parse.
        let bytes = Container10dHeader::proposed().encode_to_vec64();
        let parsed = Container10dHeader::parse(&bytes).expect("bare header must parse");
        assert_eq!(parsed.section_table_offset, 0);
        assert_eq!(parsed.section_count, 0);
    }

    #[test]
    fn parse_rejects_metric_completeness_claiming_v1_folds_t() {
        let mut bytes = Container10dHeader::proposed().encode_to_vec64();
        // metric_descriptor starts at offset 20. Each branch is 8 bytes:
        //   v_class@+0, metric_kind@+1, folded_axes@+2 (u16 LE), reserved@+4
        // Branch 1 (v=1 cyclic) starts at offset 20 + 8 = 28.
        // Set the t bit (bit 6) in folded_axes at offset 28 + 2.
        let folded_offset = 28 + 2;
        bytes[folded_offset] |= 1 << 6;
        let err =
            Container10dHeader::parse(&bytes).expect_err("diverging metric descriptor must reject");
        match err {
            HeaderParseError::MetricDivergence(msg) => {
                assert!(msg.contains("v=1"), "message must name v=1: {msg}");
                assert!(msg.contains("t"), "message must name axis t: {msg}");
            }
            other => panic!("expected MetricDivergence, got {other:?}"),
        }
    }

    #[test]
    fn parse_rejects_metric_completeness_claiming_v3_folds_x() {
        let mut bytes = Container10dHeader::proposed().encode_to_vec64();
        // Branch 3 (v>=3 catch-all) starts at offset 20 + 3*8 = 44.
        // folded_axes at offset 44 + 2. Set bit 3 (x).
        let folded_offset = 44 + 2;
        bytes[folded_offset] |= 1 << 3;
        let err =
            Container10dHeader::parse(&bytes).expect_err("diverging metric descriptor must reject");
        assert!(
            matches!(err, HeaderParseError::MetricDivergence(_)),
            "{err:?}"
        );
    }

    #[test]
    fn parse_rejects_too_short_input() {
        let short = [0u8; 32];
        let err = Container10dHeader::parse(&short).expect_err("short input must reject");
        assert!(
            matches!(err, HeaderParseError::TooShort { got: 32 }),
            "{err}"
        );
    }

    #[test]
    fn proposed_header_carries_option_a_taxonomy() {
        let h = Container10dHeader::proposed();
        for (i, &raw) in h.axis_roles.iter().enumerate() {
            let role = AxisRole::from_u8(raw).expect("proposed roles are all defined");
            assert_eq!(
                role as u8, PROPOSED_AXIS_ROLES[i] as u8,
                "axis {}",
                AXIS_ORDER[i]
            );
        }
        // μ (index 8) is the dual-role coordinate+carrier
        assert_eq!(h.axis_roles[8], AxisRole::CoordinateCarrier as u8);
    }

    #[test]
    fn proposed_header_carries_documented_limitation_descriptor() {
        let h = Container10dHeader::proposed();
        let d = &h.metric_descriptor;
        // v=0 euclidean folds all seven
        assert_eq!(d.branches[0].v_class, 0);
        assert_eq!(d.branches[0].metric_kind, MetricKind::Euclidean as u8);
        assert_eq!(
            d.branches[0].folded_axes.count_ones(),
            7,
            "v=0 must fold all seven coordinate axes"
        );
        // v=1 / v=2 fold xyz only (3 bits)
        assert_eq!(d.branches[1].folded_axes.count_ones(), 3);
        assert_eq!(d.branches[2].folded_axes.count_ones(), 3);
        // v>=3 catch-all folds none
        assert_eq!(d.branches[BOUNDARY_CLIQUE_BRANCH_INDEX].folded_axes, 0);
        assert_eq!(
            d.branches[BOUNDARY_CLIQUE_BRANCH_INDEX].metric_kind,
            MetricKind::BoundaryClique as u8
        );
    }

    #[test]
    fn proposed_header_default_disposition_is_refuse() {
        let h = Container10dHeader::proposed();
        assert_ne!(
            h.flags & FLAG_DEFAULT_DISPOSITION_REFUSE,
            0,
            "v1 header must fail closed by default"
        );
    }
}
