//! Explicit developmental M64 georaster v3 producer and lossless source recovery.
//! The legacy geotiff writer and M64_VERSION=2 remain unchanged.
mod writer;
use crate::{ElevationRaster, GeodeticFrame};
use qualia_core_db::q42::m64_matter::{
    M64GeoRasterError, M64GeoRasterMetadata, M64RasterEdges, M64RasterRows, M64RasterSample,
    M64Reader,
};
use std::sync::atomic::{AtomicBool, Ordering};
pub use writer::compile_raster_to_m64_v3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RasterM64Limits {
    pub max_output_bytes: usize,
    pub max_chunks: usize,
    pub max_records: usize,
}

/// The two placement/padding decisions have no implicit defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RasterM64Policy {
    pub rows: M64RasterRows,
    pub edges: M64RasterEdges,
    /// None explicitly declares all samples valid; Some retains that sentinel.
    pub nodata_sentinel_mm: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RasterM64Error {
    Metadata(M64GeoRasterError),
    DimensionMismatch,
    OutputByteLimit,
    ChunkLimit,
    RecordLimit,
    BufferTooSmall,
    Cancelled,
    UnsupportedEndian,
    NotGeoRaster,
}

impl RasterM64Policy {
    pub fn metadata(
        &self,
        raster: &ElevationRaster<'_>,
        frame: &GeodeticFrame,
    ) -> M64GeoRasterMetadata {
        M64GeoRasterMetadata {
            width: raster.width as u64,
            height: raster.height as u64,
            origin_lat_deg: frame.origin_lat_deg,
            origin_lon_deg: frame.origin_lon_deg,
            origin_elevation_mm: frame.origin_elevation_mm,
            cell_size_mm: raster.cell_size_mm,
            epoch_year: frame.epoch_year,
            srid: frame.srid,
            vertical_datum: frame.vertical_datum,
            rows: self.rows,
            edges: self.edges,
            nodata_sentinel_mm: self.nodata_sentinel_mm,
        }
    }
}

/// Losslessly recover the original integer samples and exact frame declarations.
/// Output capacity/cancellation are checked before mutation. Final bounded copy
/// is non-interruptible; an arriving cancellation may complete successfully.
pub fn decode_raster_from_m64_v3(
    reader: &M64Reader<'_>,
    cancelled: &AtomicBool,
    out: &mut [i32],
) -> Result<(usize, GeodeticFrame), RasterM64Error> {
    let meta = reader.georaster().ok_or(RasterM64Error::NotGeoRaster)?;
    let shape = meta.shape().map_err(RasterM64Error::Metadata)?;
    let count = shape
        .width
        .checked_mul(shape.height)
        .ok_or(RasterM64Error::DimensionMismatch)?;
    if out.len() < count {
        return Err(RasterM64Error::BufferTooSmall);
    }
    if cancelled.load(Ordering::Relaxed) {
        return Err(RasterM64Error::Cancelled);
    }
    for (index, value) in out[..count].iter_mut().enumerate() {
        // The immutable checked reader guarantees every source coordinate exists.
        *value = match reader
            .raster_sample(index % shape.width, index / shape.width)
            .expect("checked v3 source extent")
        {
            M64RasterSample::ElevationMm(v) | M64RasterSample::Nodata(v) => v,
        };
    }
    Ok((
        count,
        GeodeticFrame {
            srid: meta.srid,
            vertical_datum: meta.vertical_datum,
            epoch_year: meta.epoch_year,
            origin_lat_deg: meta.origin_lat_deg,
            origin_lon_deg: meta.origin_lon_deg,
            origin_elevation_mm: meta.origin_elevation_mm,
        },
    ))
}
