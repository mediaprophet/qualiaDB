//! Explicit developmental v3 raw georaster profile. V1/v2 defaults are unchanged.
//! Metadata retains declarations, not CRS authority or scientific validity.
mod validation;
mod wire;
pub(crate) use validation::validate_container;
pub const M64_GEORASTER_VERSION: u32 = 3;
pub const M64_GEORASTER_HEADER_BYTES: usize = 192;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M64RasterRows {
    North = 0,
    South = 1,
}
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M64RasterEdges {
    RejectPartial = 0,
    RepeatNearest = 1,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct M64GeoRasterMetadata {
    pub width: u64,
    pub height: u64,
    /// First-cell-center geographic anchor in degrees, retained at f64 precision.
    pub origin_lat_deg: f64,
    pub origin_lon_deg: f64,
    pub origin_elevation_mm: i32,
    pub cell_size_mm: u32,
    pub epoch_year: f32,
    pub srid: u32,
    pub vertical_datum: u16,
    /// Local tangent grid: columns increase east, rows follow this declaration.
    pub rows: M64RasterRows,
    pub edges: M64RasterEdges,
    pub nodata_sentinel_mm: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct M64GeoRasterShape {
    pub width: usize,
    pub height: usize,
    pub chunks_x: usize,
    pub chunks_y: usize,
    pub chunks: usize,
    pub records: usize,
    pub file_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M64GeoRasterError {
    Truncated,
    Magic,
    Schema,
    Checksum,
    Reserved,
    Policy,
    Dimensions,
    Frame,
    CellSize,
    PartialChunk,
    HilbertExtent,
    Overflow,
    Counts,
    HeaderSummary,
    Directory,
    Padding,
    NonRasterFields,
}

impl M64GeoRasterMetadata {
    pub fn shape(&self) -> Result<M64GeoRasterShape, M64GeoRasterError> {
        use M64GeoRasterError as E;
        if !self.origin_lat_deg.is_finite()
            || !self.origin_lon_deg.is_finite()
            || !(-90.0..=90.0).contains(&self.origin_lat_deg)
            || !(-180.0..=180.0).contains(&self.origin_lon_deg)
            || !self.epoch_year.is_finite()
        {
            return Err(E::Frame);
        }
        if self.cell_size_mm == 0 {
            return Err(E::CellSize);
        }
        if self.width == 0 || self.height == 0 {
            return Err(E::Dimensions);
        }
        let width = usize::try_from(self.width).map_err(|_| E::Overflow)?;
        let height = usize::try_from(self.height).map_err(|_| E::Overflow)?;
        if self.edges == M64RasterEdges::RejectPartial && (width % 64 != 0 || height % 64 != 0) {
            return Err(E::PartialChunk);
        }
        let chunks_x = width / 64 + usize::from(width % 64 != 0);
        let chunks_y = height / 64 + usize::from(height % 64 != 0);
        if chunks_x > 65_536 || chunks_y > 65_536 {
            return Err(E::HilbertExtent);
        }
        let chunks = chunks_x.checked_mul(chunks_y).ok_or(E::Overflow)?;
        let records = chunks.checked_mul(4096).ok_or(E::Overflow)?;
        let file_bytes = chunks
            .checked_mul(64 + 4096 * 64)
            .and_then(|n| n.checked_add(M64_GEORASTER_HEADER_BYTES))
            .ok_or(E::Overflow)?;
        Ok(M64GeoRasterShape {
            width,
            height,
            chunks_x,
            chunks_y,
            chunks,
            records,
            file_bytes,
        })
    }
}

/// Canonical 16-bit-per-axis Hilbert key for the profile's row-major chunk index.
pub fn georaster_chunk_hilbert(mut x: u32, mut y: u32) -> u64 {
    let mut key = 0;
    let mut s = 1u32 << 15;
    while s > 0 {
        let rx = u32::from(x & s != 0);
        let ry = u32::from(y & s != 0);
        key += u64::from(s) * u64::from(s) * u64::from((3 * rx) ^ ry);
        if ry == 0 {
            if rx == 1 {
                x = (s - 1).wrapping_sub(x);
                y = (s - 1).wrapping_sub(y);
            }
            std::mem::swap(&mut x, &mut y);
        }
        s >>= 1;
    }
    key
}

/// Source sampling distinguishes padding/out-of-extent from declared nodata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M64RasterSample {
    ElevationMm(i32),
    Nodata(i32),
}
