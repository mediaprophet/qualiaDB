use super::{
    range::{extent, nonoverlap},
    AuthenticatedRange, FormatError as E, FormatLimits, PayloadInterval,
};
use crate::container_10d::{
    crc32c::crc32c,
    header::{prefix::parse_header_prefix, Container10dHeader},
    section::{
        metadata::validate_descriptor_metadata, SectionDescriptor, SectionType,
        SECTION_DESCRIPTOR_SIZE,
    },
};
/// Global descriptor geometry plus selected payload consistency. Unfetched
/// payloads, padding, cross-section references and whole-file CRC remain unchecked.
pub struct TenDDirectoryView<'a, 's> {
    header: Container10dHeader,
    table: AuthenticatedRange<'a>,
    rows: &'s [SectionDescriptor],
    limits: FormatLimits,
}
pub struct TenDSectionView<'a> {
    descriptor: SectionDescriptor,
    payload: &'a [u8],
}
impl<'a> TenDSectionView<'a> {
    pub const fn descriptor(&self) -> &SectionDescriptor {
        &self.descriptor
    }
    pub fn bytes(&self) -> &'a [u8] {
        self.payload
    }
    /// Runs the existing bounded mini-header decoder only. This does not validate
    /// triangle indices or dependent topology/material/spatial sections.
    pub fn mesh_header(&self) -> Result<crate::container_10d::mesh_section::MeshMiniHeader, E> {
        if self.descriptor.typ() != Some(SectionType::QuantizedMesh) {
            return Err(E::Unsupported);
        }
        let (header, length) = crate::container_10d::mesh_section::parse_mesh_header(self.payload)
            .map_err(|_| E::Integrity)?;
        if length != self.payload.len() {
            return Err(E::Bounds);
        }
        Ok(header)
    }
}
impl<'a, 's> TenDDirectoryView<'a, 's> {
    pub fn admit(
        header: AuthenticatedRange<'a>,
        table: AuthenticatedRange<'a>,
        limits: FormatLimits,
        rows: &'s mut [SectionDescriptor],
        scratch: &mut [PayloadInterval],
    ) -> Result<Self, E> {
        if !cfg!(target_endian = "little") {
            return Err(E::Unsupported);
        }
        if !header.same_entry(&table) {
            return Err(E::Identity);
        }
        if header.offset() != 0 || header.bytes().len() != 64 {
            return Err(E::Bounds);
        }
        let total = header.selected().logical_length();
        if total > limits.max_entry_bytes {
            return Err(E::Limit);
        }
        let h = parse_header_prefix(header.bytes(), total).map_err(|_| E::Integrity)?;
        let count = usize::try_from(h.section_count).map_err(|_| E::Limit)?;
        if count > limits.max_rows {
            return Err(E::Limit);
        }
        let length = count
            .checked_mul(SECTION_DESCRIPTOR_SIZE)
            .ok_or(E::Bounds)?;
        if length.checked_add(64).ok_or(E::Bounds)? > limits.max_metadata_bytes {
            return Err(E::Limit);
        }
        if rows.len() < count || scratch.len() < count {
            return Err(E::Capacity);
        }
        let start = u64::from(h.section_table_offset);
        let end = extent(start, length as u64, total)?;
        if table.offset() != start || table.bytes().len() != length {
            return Err(E::Bounds);
        }
        let mut seen = [false; 256];
        let mut intervals = 0;
        for (i, raw) in table
            .bytes()
            .chunks_exact(SECTION_DESCRIPTOR_SIZE)
            .enumerate()
        {
            let d: SectionDescriptor = bytemuck::pod_read_unaligned(raw);
            if seen[d.section_type as usize] {
                return Err(E::Geometry);
            }
            seen[d.section_type as usize] = true;
            validate_descriptor_metadata(&d, i, h.version, total, start, end)
                .map_err(|_| E::Geometry)?;
            if d.byte_length != 0 {
                scratch[intervals] = PayloadInterval {
                    start: u64::from(d.byte_offset),
                    end: u64::from(d.byte_offset) + u64::from(d.byte_length),
                };
                intervals += 1;
            }
            rows[i] = d;
        }
        nonoverlap(&mut scratch[..intervals])?;
        Ok(Self {
            header: h,
            table,
            rows: &rows[..count],
            limits,
        })
    }
    pub const fn header(&self) -> &Container10dHeader {
        &self.header
    }
    pub fn descriptors(&self) -> &[SectionDescriptor] {
        self.rows
    }
    pub fn required_section(&self, kind: SectionType) -> Result<SectionDescriptor, E> {
        self.rows
            .iter()
            .find(|r| r.typ() == Some(kind))
            .copied()
            .ok_or(E::Bounds)
    }
    pub fn selected_section(
        &self,
        kind: SectionType,
        payload: AuthenticatedRange<'a>,
    ) -> Result<TenDSectionView<'a>, E> {
        if !self.table.same_entry(&payload) {
            return Err(E::Identity);
        }
        let row = self.required_section(kind)?;
        if payload.bytes().len() > self.limits.max_chunk_bytes {
            return Err(E::Limit);
        }
        if payload.offset() != u64::from(row.byte_offset)
            || payload.bytes().len() != row.byte_length as usize
        {
            return Err(E::Bounds);
        }
        if crc32c(payload.bytes()) != row.crc32c {
            return Err(E::Integrity);
        }
        Ok(TenDSectionView {
            descriptor: row,
            payload: payload.bytes(),
        })
    }
}
