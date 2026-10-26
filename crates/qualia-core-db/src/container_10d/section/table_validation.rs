//! Zero-copy table preconditions and padding traversal for the canonical reader.
use super::{HEADER_BYTE_SIZE, MAX_SECTION_COUNT, SectionDescriptor, SectionTableError};

pub(super) fn borrow_table(bytes: &[u8]) -> Result<&[SectionDescriptor], SectionTableError> {
    bytemuck::try_cast_slice(bytes).map_err(|_| SectionTableError::UnalignedSectionTable)
}

pub(super) fn metadata_range(
    descriptor: &SectionDescriptor,
    index: usize,
    table_start: usize,
    table_end: usize,
) -> Result<(), SectionTableError> {
    let start = descriptor.byte_offset as usize;
    let len = descriptor.byte_length as usize;
    // Empty sections occupy no bytes; legacy encoders may share their offset.
    if len != 0 && (start < HEADER_BYTE_SIZE || (start < table_end && start + len > table_start)) {
        return Err(SectionTableError::SectionOverlapsMetadata { index });
    }
    Ok(())
}

fn gap(data: &[u8], start: usize, end: usize) -> Result<(), SectionTableError> {
    let bytes = data
        .get(start..end)
        .ok_or(SectionTableError::InputTooShort {
            got: data.len(),
            need: end,
        })?;
    if bytes.iter().any(|b| *b != 0) {
        return Err(SectionTableError::NonZeroPadding { at: start });
    }
    Ok(())
}

/// The table need not immediately follow the header. Treat it as occupied
/// metadata while traversing sorted payloads; do not slice backwards across it.
pub(super) fn padding(
    data: &[u8],
    descriptors: &[SectionDescriptor],
    table_start: usize,
    table_end: usize,
) -> Result<(), SectionTableError> {
    let mut order = [0usize; MAX_SECTION_COUNT as usize];
    for (index, slot) in order[..descriptors.len()].iter_mut().enumerate() {
        *slot = index;
    }
    for index in 1..descriptors.len() {
        let mut j = index;
        while j > 0 && descriptors[order[j - 1]].byte_offset > descriptors[order[j]].byte_offset {
            order.swap(j - 1, j);
            j -= 1;
        }
    }
    let mut cursor = HEADER_BYTE_SIZE;
    let mut table_seen = false;
    for &index in &order[..descriptors.len()] {
        let section = descriptors[index];
        if section.byte_length == 0 {
            continue;
        }
        let offset = section.byte_offset as usize;
        if !table_seen && table_start <= offset {
            gap(data, cursor, table_start)?;
            cursor = table_end;
            table_seen = true;
        }
        gap(data, cursor, offset)?;
        cursor = offset + section.byte_length as usize;
    }
    if !table_seen {
        gap(data, cursor, table_start)?;
    }
    // Preserve the existing no-tail-padding-validation contract.
    Ok(())
}
