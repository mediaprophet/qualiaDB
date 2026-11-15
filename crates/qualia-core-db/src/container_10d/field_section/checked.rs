//! Checked, borrowed FIEL v0 declaration/grid inspection. No allocation or casts.
use super::{
    FieldRepresentationKind, FieldSupportKind, FieldTypeKind, FIELD_DESCRIPTOR_SIZE,
    FIELD_SECTION_HEADER_SIZE, FIELD_SECTION_MAGIC, FIELD_SECTION_VERSION,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldInspectionLimits {
    pub max_payload_bytes: usize,
    pub max_fields: usize,
    pub max_samples: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldInspection {
    pub fields: usize,
    pub grid_fields: usize,
    pub samples: usize,
    /// Bit n denotes an observed FieldTypeKind discriminant n.
    pub type_mask: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldInspectionError {
    Budget,
    Header,
    Descriptor { index: usize },
    UnsupportedSamples { index: usize },
    Grid { index: usize },
    NonFinite { field: usize, sample: usize },
    UnclaimedBytes,
}
impl std::fmt::Display for FieldInspectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FIEL inspection: {self:?}")
    }
}
impl std::error::Error for FieldInspectionError {}

fn u16_at(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(data[at..at + 2].try_into().expect("checked field bounds"))
}
fn u32_at(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().expect("checked field bounds"))
}
fn u64_at(data: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(data[at..at + 8].try_into().expect("checked field bounds"))
}

/// Supports the existing declaration-only schema and contiguous scalar/region/
/// grid samples emitted by `encode_field_section_with_rasters`. Declared units
/// are retained, not authenticated. Unknown sample schemas, gaps, aliases, tails,
/// reserved metadata, invalid enums/shape and nonfinite samples fail closed.
/// Unaligned input follows the same LE path. No output is mutated on error.
pub fn inspect_field_section(
    bytes: &[u8],
    limits: FieldInspectionLimits,
) -> Result<FieldInspection, FieldInspectionError> {
    if bytes.len() > limits.max_payload_bytes {
        return Err(FieldInspectionError::Budget);
    }
    if bytes.len() < FIELD_SECTION_HEADER_SIZE
        || u32_at(bytes, 0) != FIELD_SECTION_MAGIC
        || u16_at(bytes, 4) != FIELD_SECTION_VERSION
        || u32_at(bytes, 8) != 0
    {
        return Err(FieldInspectionError::Header);
    }
    let fields = usize::from(u16_at(bytes, 6));
    if fields > limits.max_fields {
        return Err(FieldInspectionError::Budget);
    }
    let table_end = FIELD_SECTION_HEADER_SIZE + fields * FIELD_DESCRIPTOR_SIZE;
    if table_end > bytes.len() {
        return Err(FieldInspectionError::Header);
    }
    let mut occupied_end = table_end;
    let mut samples = 0usize;
    let mut grid_fields = 0;
    let mut type_mask = 0;
    for index in 0..fields {
        let at = FIELD_SECTION_HEADER_SIZE + index * FIELD_DESCRIPTOR_SIZE;
        let d = &bytes[at..at + FIELD_DESCRIPTOR_SIZE];
        if FieldTypeKind::from_u8(d[16]).is_none()
            || FieldSupportKind::from_u8(d[17]).is_none()
            || FieldRepresentationKind::from_u8(d[18]).is_none()
            || d[19] != 0
            || u32_at(d, 20) != 0
        {
            return Err(FieldInspectionError::Descriptor { index });
        }
        type_mask |= 1 << d[16];
        let raw_offset = u64_at(d, 24);
        if raw_offset == 0 {
            continue;
        }
        if d[16] != FieldTypeKind::Scalar as u8
            || d[17] != FieldSupportKind::Region as u8
            || d[18] != FieldRepresentationKind::Grid as u8
        {
            return Err(FieldInspectionError::UnsupportedSamples { index });
        }
        let offset =
            usize::try_from(raw_offset).map_err(|_| FieldInspectionError::Grid { index })?;
        // Sequential samples exclude overlap, reinterpretation of descriptors,
        // unclaimed bytes, and aliases between different fields.
        if offset != occupied_end {
            return Err(FieldInspectionError::Grid { index });
        }
        let meta_end = offset
            .checked_add(16)
            .filter(|end| *end <= bytes.len())
            .ok_or(FieldInspectionError::Grid { index })?;
        let width = u32_at(bytes, offset);
        let height = u32_at(bytes, offset + 4);
        let count = u32_at(bytes, offset + 8) as usize;
        if width == 0
            || height == 0
            || width.checked_mul(height).map(|n| n as usize) != Some(count)
            || u32_at(bytes, offset + 12) != 0
        {
            return Err(FieldInspectionError::Grid { index });
        }
        samples = samples
            .checked_add(count)
            .ok_or(FieldInspectionError::Budget)?;
        if samples > limits.max_samples {
            return Err(FieldInspectionError::Budget);
        }
        let end = count
            .checked_mul(4)
            .and_then(|n| meta_end.checked_add(n))
            .filter(|end| *end <= bytes.len())
            .ok_or(FieldInspectionError::Grid { index })?;
        for (sample, raw) in bytes[meta_end..end].chunks_exact(4).enumerate() {
            if !f32::from_le_bytes(raw.try_into().expect("four byte chunk")).is_finite() {
                return Err(FieldInspectionError::NonFinite {
                    field: index,
                    sample,
                });
            }
        }
        occupied_end = end;
        grid_fields += 1;
    }
    if occupied_end != bytes.len() {
        return Err(FieldInspectionError::UnclaimedBytes);
    }
    Ok(FieldInspection {
        fields,
        grid_fields,
        samples,
        type_mask,
    })
}

#[cfg(test)]
#[path = "checked/tests.rs"]
mod tests;
