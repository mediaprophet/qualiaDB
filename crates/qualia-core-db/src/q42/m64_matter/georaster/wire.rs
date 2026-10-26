use super::*;
use crate::container_10d::crc32c::crc32c;

impl M64GeoRasterMetadata {
    /// Encode one checked GRM3 block. Refusal leaves output unchanged.
    pub fn encode(&self, out: &mut [u8]) -> Result<(), M64GeoRasterError> {
        let shape = self.shape()?;
        if out.len() < 128 {
            return Err(M64GeoRasterError::Truncated);
        }
        let mut raw = [0u8; 128];
        raw[..4].copy_from_slice(b"GRM3");
        raw[4..6].copy_from_slice(&1u16.to_le_bytes());
        raw[6..8].copy_from_slice(&128u16.to_le_bytes());
        raw[8..16].copy_from_slice(&self.width.to_le_bytes());
        raw[16..24].copy_from_slice(&self.height.to_le_bytes());
        raw[24..32].copy_from_slice(&self.origin_lat_deg.to_le_bytes());
        raw[32..40].copy_from_slice(&self.origin_lon_deg.to_le_bytes());
        raw[40..44].copy_from_slice(&self.origin_elevation_mm.to_le_bytes());
        raw[44..48].copy_from_slice(&self.cell_size_mm.to_le_bytes());
        raw[48..52].copy_from_slice(&self.epoch_year.to_le_bytes());
        raw[52..56].copy_from_slice(&self.srid.to_le_bytes());
        raw[56..58].copy_from_slice(&self.vertical_datum.to_le_bytes());
        raw[58] = self.rows as u8;
        // anchor 0 = first-cell-center; basis 0 = local east/north, columns east.
        raw[61] = self.edges as u8;
        raw[62] = u8::from(self.nodata_sentinel_mm.is_some());
        raw[64..68].copy_from_slice(&self.nodata_sentinel_mm.unwrap_or(0).to_le_bytes());
        raw[68..72].copy_from_slice(&64u32.to_le_bytes());
        raw[72..80].copy_from_slice(&(shape.records as u64).to_le_bytes());
        raw[80..88].copy_from_slice(&(shape.chunks as u64).to_le_bytes());
        raw[88..96].copy_from_slice(&((shape.records as u64) * 64).to_le_bytes());
        let checksum = crc32c(&raw[..124]);
        raw[124..128].copy_from_slice(&checksum.to_le_bytes());
        out[..128].copy_from_slice(&raw);
        Ok(())
    }

    pub fn decode(raw: &[u8]) -> Result<Self, M64GeoRasterError> {
        use M64GeoRasterError as E;
        let raw = raw.get(..128).ok_or(E::Truncated)?;
        if &raw[..4] != b"GRM3" {
            return Err(E::Magic);
        }
        if raw[4..8] != [1, 0, 128, 0] {
            return Err(E::Schema);
        }
        if u32_at(raw, 124) != crc32c(&raw[..124]) {
            return Err(E::Checksum);
        }
        if raw[63] != 0 || raw[96..124].iter().any(|&v| v != 0) {
            return Err(E::Reserved);
        }
        if raw[59] != 0 || raw[60] != 0 || raw[62] > 1 {
            return Err(E::Policy);
        }
        let rows = match raw[58] {
            0 => M64RasterRows::North,
            1 => M64RasterRows::South,
            _ => return Err(E::Policy),
        };
        let edges = match raw[61] {
            0 => M64RasterEdges::RejectPartial,
            1 => M64RasterEdges::RepeatNearest,
            _ => return Err(E::Policy),
        };
        let sentinel = i32::from_le_bytes(raw[64..68].try_into().unwrap());
        if raw[62] == 0 && sentinel != 0 {
            return Err(E::Reserved);
        }
        let meta = Self {
            width: u64_at(raw, 8),
            height: u64_at(raw, 16),
            origin_lat_deg: f64::from_bits(u64_at(raw, 24)),
            origin_lon_deg: f64::from_bits(u64_at(raw, 32)),
            origin_elevation_mm: i32::from_le_bytes(raw[40..44].try_into().unwrap()),
            cell_size_mm: u32_at(raw, 44),
            epoch_year: f32::from_bits(u32_at(raw, 48)),
            srid: u32_at(raw, 52),
            vertical_datum: u16::from_le_bytes(raw[56..58].try_into().unwrap()),
            rows,
            edges,
            nodata_sentinel_mm: if raw[62] == 1 { Some(sentinel) } else { None },
        };
        let shape = meta.shape()?;
        if u32_at(raw, 68) != 64
            || u64_at(raw, 72) != shape.records as u64
            || u64_at(raw, 80) != shape.chunks as u64
            || u64_at(raw, 88) != shape.records as u64 * 64
        {
            return Err(E::Counts);
        }
        Ok(meta)
    }
}

fn u32_at(raw: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(raw[offset..offset + 4].try_into().unwrap())
}
fn u64_at(raw: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(raw[offset..offset + 8].try_into().unwrap())
}
