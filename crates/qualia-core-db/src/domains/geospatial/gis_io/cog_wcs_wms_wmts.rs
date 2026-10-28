// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) RustGIS contributors (adapted under MIT License)
//! Cloud-Optimized GeoTIFF (`COG`) IFD header & HTTP Range byte-window planner,
//! OGC WCS 2.0, OGC WMS 1.3.0, OGC WMTS 1.0, and Slippy / TMS Tile Pyramid Math.

/// Errors encountered when parsing raster headers or constructing OGC requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RasterServiceError {
    InvalidTiffMagic,
    TruncatedIfd,
    MissingTag(u16),
    TileOutOfBounds,
    ByteBudgetExceeded { requested: usize, budget: usize },
}

/// Parsed metadata and layout of a Cloud-Optimized GeoTIFF (COG) IFD0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CogIfdLayout {
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub bits_per_sample: u16,
    pub compression: u16,
    pub epsg_code: u32,
    pub pixel_scale: [f64; 3],
    pub tiepoint_model: [f64; 3],
    pub tile_offsets_ptr: u32,
    pub tile_byte_counts_ptr: u32,
    pub tile_count: usize,
}

/// HTTP Range request window for a single COG tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpByteRange {
    pub tile_col: u32,
    pub tile_row: u32,
    pub start_byte: u64,
    pub end_byte_inclusive: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Endianness {
    Little,
    Big,
}

impl Endianness {
    #[inline]
    fn u16(self, b: &[u8], off: usize) -> Option<u16> {
        let s = b.get(off..off + 2)?;
        Some(match self {
            Self::Little => u16::from_le_bytes([s[0], s[1]]),
            Self::Big => u16::from_be_bytes([s[0], s[1]]),
        })
    }
    #[inline]
    fn u32(self, b: &[u8], off: usize) -> Option<u32> {
        let s = b.get(off..off + 4)?;
        Some(match self {
            Self::Little => u32::from_le_bytes([s[0], s[1], s[2], s[3]]),
            Self::Big => u32::from_be_bytes([s[0], s[1], s[2], s[3]]),
        })
    }
    #[inline]
    fn f64(self, b: &[u8], off: usize) -> Option<f64> {
        let s = b.get(off..off + 8)?;
        Some(match self {
            Self::Little => f64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]),
            Self::Big => f64::from_be_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]),
        })
    }
}

/// Parse little-endian or big-endian TIFF header and IFD0 tags from a memory buffer.
pub fn parse_cog_ifd(tiff_header_bytes: &[u8]) -> Result<CogIfdLayout, RasterServiceError> {
    if tiff_header_bytes.len() < 8 {
        return Err(RasterServiceError::TruncatedIfd);
    }
    let endian = match &tiff_header_bytes[0..4] {
        [0x49, 0x49, 0x2A, 0x00] => Endianness::Little,
        [0x4D, 0x4D, 0x00, 0x2A] => Endianness::Big,
        _ => return Err(RasterServiceError::InvalidTiffMagic),
    };
    let ifd_offset = endian.u32(tiff_header_bytes, 4).ok_or(RasterServiceError::TruncatedIfd)? as usize;
    let num_entries = endian.u16(tiff_header_bytes, ifd_offset).ok_or(RasterServiceError::TruncatedIfd)? as usize;
    let entries_end = ifd_offset.checked_add(2 + num_entries * 12).ok_or(RasterServiceError::TruncatedIfd)?;
    if entries_end > tiff_header_bytes.len() {
        return Err(RasterServiceError::TruncatedIfd);
    }

    let (mut width, mut height, mut bits_per_sample, mut compression) = (None, None, 8u16, 1u16);
    let (mut tile_width, mut tile_height, mut to_ptr, mut tc_ptr) = (None, None, None, None);
    let (mut pixel_scale, mut tiepoint_model, mut epsg_code) = ([1.0, 1.0, 0.0], [0.0, 0.0, 0.0], 0u32);

    for i in 0..num_entries {
        let off = ifd_offset + 2 + i * 12;
        let tag = endian.u16(tiff_header_bytes, off).unwrap();
        let typ = endian.u16(tiff_header_bytes, off + 2).unwrap();
        let count = endian.u32(tiff_header_bytes, off + 4).unwrap();
        let val_or_off = endian.u32(tiff_header_bytes, off + 8).unwrap();
        let val = if typ == 3 { endian.u16(tiff_header_bytes, off + 8).unwrap_or(0) as u32 } else { val_or_off };

        match tag {
            256 => width = Some(val),
            257 => height = Some(val),
            258 => bits_per_sample = val as u16,
            259 => compression = val as u16,
            322 => tile_width = Some(val),
            323 => tile_height = Some(val),
            324 => to_ptr = Some(if count == 1 { (off + 8) as u32 } else { val_or_off }),
            325 => tc_ptr = Some(if count == 1 { (off + 8) as u32 } else { val_or_off }),
            33550 if count >= 2 => {
                let p = val_or_off as usize;
                pixel_scale = [
                    endian.f64(tiff_header_bytes, p).unwrap_or(1.0),
                    endian.f64(tiff_header_bytes, p + 8).unwrap_or(1.0),
                    endian.f64(tiff_header_bytes, p + 16).unwrap_or(0.0),
                ];
            }
            33922 if count >= 6 => {
                let p = val_or_off as usize;
                tiepoint_model = [
                    endian.f64(tiff_header_bytes, p + 24).unwrap_or(0.0),
                    endian.f64(tiff_header_bytes, p + 32).unwrap_or(0.0),
                    endian.f64(tiff_header_bytes, p + 40).unwrap_or(0.0),
                ];
            }
            34735 if count >= 4 => {
                let p = val_or_off as usize;
                if let Some(num_keys) = endian.u16(tiff_header_bytes, p + 6) {
                    let (mut proj, mut geo) = (0u32, 0u32);
                    for k in 0..(num_keys as usize) {
                        let kp = p + 8 + k * 8;
                        if let (Some(id), Some(loc), Some(v)) = (
                            endian.u16(tiff_header_bytes, kp),
                            endian.u16(tiff_header_bytes, kp + 2),
                            endian.u16(tiff_header_bytes, kp + 6),
                        ) {
                            if loc == 0 {
                                if id == 3072 { proj = v as u32; }
                                else if id == 2048 { geo = v as u32; }
                            }
                        }
                    }
                    epsg_code = if proj > 0 && proj != 32767 { proj } else { geo };
                }
            }
            _ => {}
        }
    }

    let w = width.ok_or(RasterServiceError::MissingTag(256))?;
    let h = height.ok_or(RasterServiceError::MissingTag(257))?;
    let tw = tile_width.ok_or(RasterServiceError::MissingTag(322))?;
    let th = tile_height.ok_or(RasterServiceError::MissingTag(323))?;
    let tile_offsets_ptr = to_ptr.ok_or(RasterServiceError::MissingTag(324))?;
    let tile_byte_counts_ptr = tc_ptr.ok_or(RasterServiceError::MissingTag(325))?;
    if tw == 0 || th == 0 { return Err(RasterServiceError::TileOutOfBounds); }
    let tile_count = (((w + tw - 1) / tw) as usize) * (((h + th - 1) / th) as usize);

    Ok(CogIfdLayout {
        width: w,
        height: h,
        tile_width: tw,
        tile_height: th,
        bits_per_sample,
        compression,
        epsg_code,
        pixel_scale,
        tiepoint_model,
        tile_offsets_ptr,
        tile_byte_counts_ptr,
        tile_count,
    })
}

/// Compute intersecting tile columns and rows for `query_bbox` and write HTTP byte ranges
/// into caller-provided slice with zero heap allocation.
pub fn plan_cog_http_ranges(
    layout: &CogIfdLayout,
    tile_offsets: &[u64],
    tile_byte_counts: &[u64],
    query_bbox: [f64; 4],
    out_ranges: &mut [HttpByteRange],
) -> Result<usize, RasterServiceError> {
    if layout.tile_width == 0 || layout.tile_height == 0 || layout.width == 0 || layout.height == 0 {
        return Err(RasterServiceError::TileOutOfBounds);
    }
    let (sx, sy) = (layout.pixel_scale[0], layout.pixel_scale[1]);
    if sx <= 0.0 || sy <= 0.0 {
        return Err(RasterServiceError::TileOutOfBounds);
    }
    let (min_x, max_x) = if query_bbox[0] <= query_bbox[2] { (query_bbox[0], query_bbox[2]) } else { (query_bbox[2], query_bbox[0]) };
    let (min_y, max_y) = if query_bbox[1] <= query_bbox[3] { (query_bbox[1], query_bbox[3]) } else { (query_bbox[3], query_bbox[1]) };

    let px_min = (min_x - layout.tiepoint_model[0]) / sx;
    let px_max = (max_x - layout.tiepoint_model[0]) / sx;
    let py_min = (layout.tiepoint_model[1] - max_y) / sy;
    let py_max = (layout.tiepoint_model[1] - min_y) / sy;

    let (img_w, img_h) = (layout.width as f64, layout.height as f64);
    if px_max < 0.0 || px_min >= img_w || py_max < 0.0 || py_min >= img_h {
        return Ok(0);
    }

    let tiles_across = (layout.width + layout.tile_width - 1) / layout.tile_width;
    let tiles_down = (layout.height + layout.tile_height - 1) / layout.tile_height;
    let (max_col, max_row) = (tiles_across.saturating_sub(1), tiles_down.saturating_sub(1));

    let col_0 = ((px_min.max(0.0) as u32) / layout.tile_width).min(max_col);
    let col_1 = ((px_max.min(img_w - 1.0).max(0.0) as u32) / layout.tile_width).min(max_col);
    let row_0 = ((py_min.max(0.0) as u32) / layout.tile_height).min(max_row);
    let row_1 = ((py_max.min(img_h - 1.0).max(0.0) as u32) / layout.tile_height).min(max_row);

    let needed = ((col_1 - col_0 + 1) as usize) * ((row_1 - row_0 + 1) as usize);
    if needed > out_ranges.len() {
        return Err(RasterServiceError::ByteBudgetExceeded { requested: needed, budget: out_ranges.len() });
    }

    let mut written = 0;
    for r in row_0..=row_1 {
        for c in col_0..=col_1 {
            let idx = (r * tiles_across + c) as usize;
            if idx >= tile_offsets.len() || idx >= tile_byte_counts.len() {
                return Err(RasterServiceError::TileOutOfBounds);
            }
            let start = tile_offsets[idx];
            let count = tile_byte_counts[idx];
            let end_inclusive = if count > 0 { start + count - 1 } else { start };
            out_ranges[written] = HttpByteRange { tile_col: c, tile_row: r, start_byte: start, end_byte_inclusive: end_inclusive };
            written += 1;
        }
    }
    Ok(written)
}

#[inline]
fn endpoint_sep(ep: &str) -> &'static str {
    if ep.contains('?') { if ep.ends_with('?') || ep.ends_with('&') { "" } else { "&" } } else { "?" }
}

#[inline]
fn check_budget(url: String, budget: usize) -> Result<String, RasterServiceError> {
    if url.len() > budget { Err(RasterServiceError::ByteBudgetExceeded { requested: url.len(), budget }) } else { Ok(url) }
}

/// Construct an OGC WCS 2.0.1 GetCoverage request URL with caller byte-budget enforcement.
pub fn build_wcs20_get_coverage_url(
    endpoint: &str,
    coverage_id: &str,
    bbox: [f64; 4],
    crs_epsg: u32,
    format: &str,
    byte_budget: usize,
) -> Result<String, RasterServiceError> {
    let sep = endpoint_sep(endpoint);
    let min_needed = endpoint.len() + sep.len() + coverage_id.len() + format.len() + 110;
    if min_needed > byte_budget {
        return Err(RasterServiceError::ByteBudgetExceeded { requested: min_needed, budget: byte_budget });
    }
    let url = format!(
        "{endpoint}{sep}SERVICE=WCS&VERSION=2.0.1&REQUEST=GetCoverage&COVERAGEID={coverage_id}&SUBSET=X({},{})&SUBSET=Y({},{})&SUBSETTINGCRS=http://www.opengis.net/def/crs/EPSG/0/{crs_epsg}&FORMAT={format}",
        bbox[0], bbox[2], bbox[1], bbox[3]
    );
    check_budget(url, byte_budget)
}

fn is_wms130_lat_lon_axis_order(crs: &str) -> bool {
    let s = crs.trim();
    s.eq_ignore_ascii_case("EPSG:4326")
        || s.eq_ignore_ascii_case("EPSG:4283")
        || s.eq_ignore_ascii_case("EPSG:7844")
        || s.eq_ignore_ascii_case("EPSG:4269")
        || s.eq_ignore_ascii_case("EPSG:4258")
        || s.eq_ignore_ascii_case("urn:ogc:def:crs:EPSG::4326")
}

/// Construct an OGC WMS 1.3.0 GetMap request URL, correctly swapping axes for geographic CRSs.
pub fn build_wms130_get_map_url(
    endpoint: &str,
    layers: &str,
    styles: &str,
    crs: &str,
    bbox_minx_miny_maxx_maxy: [f64; 4],
    width: u32,
    height: u32,
    byte_budget: usize,
) -> Result<String, RasterServiceError> {
    let sep = endpoint_sep(endpoint);
    let min_needed = endpoint.len() + sep.len() + layers.len() + styles.len() + crs.len() + 110;
    if min_needed > byte_budget {
        return Err(RasterServiceError::ByteBudgetExceeded { requested: min_needed, budget: byte_budget });
    }
    let b = bbox_minx_miny_maxx_maxy;
    let bbox_str = if is_wms130_lat_lon_axis_order(crs) {
        format!("{},{},{},{}", b[1], b[0], b[3], b[2])
    } else {
        format!("{},{},{},{}", b[0], b[1], b[2], b[3])
    };
    let url = format!(
        "{endpoint}{sep}SERVICE=WMS&VERSION=1.3.0&REQUEST=GetMap&LAYERS={layers}&STYLES={styles}&CRS={crs}&BBOX={bbox_str}&WIDTH={width}&HEIGHT={height}&FORMAT=image/png"
    );
    check_budget(url, byte_budget)
}

/// Construct an OGC WMTS 1.0.0 GetTile request URL with caller byte-budget enforcement.
pub fn build_wmts10_get_tile_url(
    endpoint: &str,
    layer: &str,
    tile_matrix_set: &str,
    z: u8,
    tile_col: u32,
    tile_row: u32,
    format: &str,
    byte_budget: usize,
) -> Result<String, RasterServiceError> {
    let sep = endpoint_sep(endpoint);
    let min_needed = endpoint.len() + sep.len() + layer.len() + tile_matrix_set.len() + format.len() + 110;
    if min_needed > byte_budget {
        return Err(RasterServiceError::ByteBudgetExceeded { requested: min_needed, budget: byte_budget });
    }
    let url = format!(
        "{endpoint}{sep}SERVICE=WMTS&VERSION=1.0.0&REQUEST=GetTile&LAYER={layer}&STYLE=default&FORMAT={format}&TILEMATRIXSET={tile_matrix_set}&TILEMATRIX={z}&TILEROW={tile_row}&TILECOL={tile_col}"
    );
    check_budget(url, byte_budget)
}

pub const WEB_MERCATOR_MAX_LAT: f64 = 85.0511287798066;

/// Project WGS 84 (longitude, latitude) in degrees to Slippy XYZ tile indices at given zoom.
pub fn lonlat_to_xyz_tile(lon_deg: f64, lat_deg: f64, zoom: u8) -> (u32, u32) {
    if zoom == 0 { return (0, 0); }
    let n = (1u64 << zoom.min(31)) as f64;
    let clamped_lon = lon_deg.clamp(-180.0, 180.0);
    let mut x = ((clamped_lon + 180.0) / 360.0 * n).floor();
    if x >= n { x = n - 1.0; }
    let clamped_lat = lat_deg.clamp(-WEB_MERCATOR_MAX_LAT, WEB_MERCATOR_MAX_LAT);
    let lat_rad = clamped_lat.to_radians();
    let sec_plus_tan = lat_rad.tan() + (1.0 / lat_rad.cos());
    let mut y = ((1.0 - sec_plus_tan.ln() / core::f64::consts::PI) * 0.5 * n).floor();
    if y >= n { y = n - 1.0; }
    (x.max(0.0) as u32, y.max(0.0) as u32)
}

/// Compute WGS 84 `[min_lon, min_lat, max_lon, max_lat]` bounding box for a Slippy XYZ tile.
pub fn xyz_tile_to_lonlat_bbox(z: u8, x: u32, y: u32) -> [f64; 4] {
    let n = (1u64 << z.min(31)) as f64;
    let min_lon = (x as f64) / n * 360.0 - 180.0;
    let max_lon = ((x + 1) as f64) / n * 360.0 - 180.0;
    let pi = core::f64::consts::PI;
    let lat_rad_north = (pi * (1.0 - 2.0 * (y as f64) / n)).sinh().atan();
    let lat_rad_south = (pi * (1.0 - 2.0 * ((y + 1) as f64) / n)).sinh().atan();
    [min_lon, lat_rad_south.to_degrees(), max_lon, lat_rad_north.to_degrees()]
}

/// Convert Slippy XYZ tile row (origin north) to TMS tile row (origin south).
#[inline]
pub fn xyz_y_to_tms_y(z: u8, y_xyz: u32) -> u32 {
    ((1u32 << z) - 1).saturating_sub(y_xyz)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_synthetic_cog_buffer() -> [u8; 320] {
        let mut buf = [0u8; 320];
        buf[0..2].copy_from_slice(b"II");
        buf[2..4].copy_from_slice(&42u16.to_le_bytes());
        buf[4..8].copy_from_slice(&8u32.to_le_bytes());
        buf[8..10].copy_from_slice(&11u16.to_le_bytes());
        let mut off = 10;
        let mut write_entry = |buf: &mut [u8], tag: u16, typ: u16, count: u32, val_or_off: u32| {
            buf[off..off + 2].copy_from_slice(&tag.to_le_bytes());
            buf[off + 2..off + 4].copy_from_slice(&typ.to_le_bytes());
            buf[off + 4..off + 8].copy_from_slice(&count.to_le_bytes());
            buf[off + 8..off + 12].copy_from_slice(&val_or_off.to_le_bytes());
            off += 12;
        };

        write_entry(&mut buf, 256, 4, 1, 512);
        write_entry(&mut buf, 257, 4, 1, 512);
        write_entry(&mut buf, 258, 3, 1, 8);
        write_entry(&mut buf, 259, 3, 1, 1);
        write_entry(&mut buf, 322, 4, 1, 256);
        write_entry(&mut buf, 323, 4, 1, 256);
        write_entry(&mut buf, 324, 4, 4, 234);
        write_entry(&mut buf, 325, 4, 4, 250);
        write_entry(&mut buf, 33550, 12, 3, 146);
        write_entry(&mut buf, 33922, 12, 6, 170);
        write_entry(&mut buf, 34735, 3, 8, 218);

        for (i, v) in [10.0f64, 10.0, 0.0].iter().enumerate() {
            buf[146 + i * 8..154 + i * 8].copy_from_slice(&v.to_le_bytes());
        }
        for (i, v) in [0.0f64, 0.0, 0.0, 300000.0, 6000000.0, 0.0].iter().enumerate() {
            buf[170 + i * 8..178 + i * 8].copy_from_slice(&v.to_le_bytes());
        }
        for (i, v) in [1u16, 1, 0, 1, 3072, 0, 1, 7855].iter().enumerate() {
            buf[218 + i * 2..220 + i * 2].copy_from_slice(&v.to_le_bytes());
        }
        for i in 0..4 {
            let o = 1000u32 + i * 500;
            buf[234 + i as usize * 4..238 + i as usize * 4].copy_from_slice(&o.to_le_bytes());
            buf[250 + i as usize * 4..254 + i as usize * 4].copy_from_slice(&500u32.to_le_bytes());
        }
        buf
    }

    #[test]
    fn test_parse_cog_ifd_and_plan_ranges() {
        let buf = build_synthetic_cog_buffer();
        let layout = parse_cog_ifd(&buf).expect("valid cog ifd");
        assert_eq!(layout.width, 512);
        assert_eq!(layout.height, 512);
        assert_eq!(layout.tile_width, 256);
        assert_eq!(layout.tile_height, 256);
        assert_eq!(layout.tile_count, 4);
        assert_eq!(layout.epsg_code, 7855);
        assert_eq!(layout.pixel_scale, [10.0, 10.0, 0.0]);
        assert_eq!(layout.tiepoint_model, [300000.0, 6000000.0, 0.0]);

        let tile_offsets = [1000u64, 1500, 2000, 2500];
        let tile_byte_counts = [500u64, 500, 500, 500];
        let mut ranges = [HttpByteRange { tile_col: 0, tile_row: 0, start_byte: 0, end_byte_inclusive: 0 }; 4];

        let query_bbox = [300000.0, 5998000.0, 302000.0, 6000000.0];
        let count = plan_cog_http_ranges(&layout, &tile_offsets, &tile_byte_counts, query_bbox, &mut ranges)
            .expect("plan ranges");
        assert_eq!(count, 1);
        assert_eq!(ranges[0].tile_col, 0);
        assert_eq!(ranges[0].tile_row, 0);
        assert_eq!(ranges[0].start_byte, 1000);
        assert_eq!(ranges[0].end_byte_inclusive, 1499);
    }

    #[test]
    fn test_wms130_axis_ordering() {
        let bbox = [144.0, -38.0, 145.0, -37.0];
        let url_4326 = build_wms130_get_map_url("https://wms.test", "l1", "", "EPSG:4326", bbox, 256, 256, 512).unwrap();
        assert!(url_4326.contains("BBOX=-38,144,-37,145"));

        let url_7855 = build_wms130_get_map_url("https://wms.test", "l1", "", "EPSG:7855", bbox, 256, 256, 512).unwrap();
        assert!(url_7855.contains("BBOX=144,-38,145,-37"));

        let url_crs84 = build_wms130_get_map_url("https://wms.test", "l1", "", "CRS:84", bbox, 256, 256, 512).unwrap();
        assert!(url_crs84.contains("BBOX=144,-38,145,-37"));
    }

    #[test]
    fn test_wcs_wmts_and_tile_math() {
        let bbox = [144.0, -38.0, 145.0, -37.0];
        let wcs_url = build_wcs20_get_coverage_url("https://wcs.test", "dem", bbox, 4326, "image/tiff", 512).unwrap();
        assert!(wcs_url.contains("SUBSET=X(144,145)&SUBSET=Y(-38,-37)"));

        let wmts_url = build_wmts10_get_tile_url("https://wmts.test", "topo", "EPSG:3857", 12, 3810, 2530, "image/png", 512).unwrap();
        assert!(wmts_url.contains("TILEMATRIX=12&TILEROW=2530&TILECOL=3810"));

        let (x, y) = lonlat_to_xyz_tile(0.0, 0.0, 1);
        assert_eq!((x, y), (1, 1));
        assert_eq!(xyz_y_to_tms_y(1, y), 0);

        let z0_bbox = xyz_tile_to_lonlat_bbox(0, 0, 0);
        assert!((z0_bbox[0] - (-180.0)).abs() < 1e-6);
        assert!((z0_bbox[2] - 180.0).abs() < 1e-6);

        let err = build_wcs20_get_coverage_url("https://wcs.test", "dem", bbox, 4326, "image/tiff", 20);
        assert!(matches!(err, Err(RasterServiceError::ByteBudgetExceeded { .. })));
    }
}
