//! Header-only structural admission with an independently established full extent.
//! No source authentication or whole-file CRC verification is performed here.
use super::{
    Container10dHeader, HEADER_BYTE_SIZE, HeaderParseError, MAGIC_10D, MAX_SECTION_COUNT,
    SUPPORTED_HEADER_VERSIONS,
};
use crate::container_10d::{
    axis_role::AxisRole,
    metric_check::{MetricDivergence, verify_descriptor_against_reality},
};
/// Preserve structured metric errors on hot paths; no diagnostic string allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrefixError {
    Header(HeaderParseError),
    Metric(MetricDivergence),
}
impl PrefixError {
    pub(super) fn into_legacy(self) -> HeaderParseError {
        match self {
            Self::Header(e) => e,
            Self::Metric(e) => HeaderParseError::MetricDivergence(e.to_string()),
        }
    }
}
/// Validate original header bytes without requiring payload/metadata to be resident.
/// The caller must establish the declared extent independently and check sections.
pub fn parse_header_prefix(data: &[u8], extent: u64) -> Result<Container10dHeader, PrefixError> {
    use PrefixError::Header;
    if data.len() < HEADER_BYTE_SIZE {
        return Err(Header(HeaderParseError::TooShort { got: data.len() }));
    }
    if extent < HEADER_BYTE_SIZE as u64 {
        return Err(Header(HeaderParseError::TooShort {
            got: extent as usize,
        }));
    }
    let header: Container10dHeader = bytemuck::pod_read_unaligned(&data[..HEADER_BYTE_SIZE]);
    if header.magic != MAGIC_10D {
        return Err(Header(HeaderParseError::BadMagic { got: header.magic }));
    }
    if !SUPPORTED_HEADER_VERSIONS.contains(&header.version) {
        return Err(Header(HeaderParseError::UnknownVersion {
            got: header.version,
        }));
    }
    if header.pad0 != [0, 0] {
        return Err(Header(HeaderParseError::NonZeroPadding { field: "pad0" }));
    }
    let (off, count) = (header.section_table_offset, header.section_count);
    let valid = (off == 0 && count == 0)
        || (off as u64 >= HEADER_BYTE_SIZE as u64
            && u64::from(off) <= extent
            && count != 0
            && count <= MAX_SECTION_COUNT);
    if !valid {
        return Err(Header(HeaderParseError::BadSectionTablePointer {
            offset: off,
            count,
        }));
    }
    for (i, &raw) in header.axis_roles.iter().enumerate() {
        if AxisRole::from_u8(raw).is_none() || raw == AxisRole::Undefined as u8 {
            return Err(Header(HeaderParseError::UndefinedAxisRole {
                axis_index: i,
                got: raw,
            }));
        }
    }
    verify_descriptor_against_reality(&header.metric_descriptor).map_err(PrefixError::Metric)?;
    Ok(header)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
    #[test]
    fn late_table_requires_independent_extent_not_fake_resident_bytes() {
        let mut h = Container10dHeader::proposed();
        h.section_table_offset = 4096;
        h.section_count = 1;
        let mut raw = [0; 65];
        raw[1..].copy_from_slice(&h.encode_to_vec64());
        assert!(Container10dHeader::parse(&raw[1..]).is_err());
        let guard = AllocGuard::begin("10d-header-prefix", true);
        assert_eq!(parse_header_prefix(&raw[1..], 8192).unwrap(), h);
        assert!(parse_header_prefix(&raw[1..], 4095).is_err());
        assert!(parse_header_prefix(&raw[1..], 63).is_err());
        guard.check().unwrap();
    }
    #[test]
    fn malformed_metric_rejection_uses_structured_error_without_allocation() {
        let mut h = Container10dHeader::proposed();
        h.metric_descriptor.branches[0].metric_kind = 0;
        let mut raw = [0; 64];
        h.encode(&mut raw);
        let guard = AllocGuard::begin("10d-prefix-metric-refusal", true);
        assert!(matches!(
            parse_header_prefix(&raw, 64),
            Err(PrefixError::Metric(_))
        ));
        guard.check().unwrap();
    }
}
