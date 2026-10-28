// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) RustGIS contributors (adapted under MIT License)
//! Zarr v2/v3 N-D Chunked Array Reader, NetCDF-Classic (`CDF\x01`/`CDF\x02`) Binary Reader,
//! WMO GRIB2 Meteorological Unpacker, and `.10d` 10-Dimensional Environmental Tensor Cube Compiler.

/// NetCDF Classic Format Magic (`CDF\x01`).
pub const NETCDF_CLASSIC_MAGIC: [u8; 4] = *b"CDF\x01";
/// NetCDF 64-bit Offset Format Magic (`CDF\x02`).
pub const NETCDF_64BIT_OFFSET_MAGIC: [u8; 4] = *b"CDF\x02";
/// NetCDF Dimension Table Tag (`NC_DIMENSION`).
pub const NC_DIMENSION: u32 = 0x0000_000A;
/// NetCDF Variable Table Tag (`NC_VARIABLE`).
pub const NC_VARIABLE: u32 = 0x0000_000B;
/// NetCDF Attribute Table Tag (`NC_ATTRIBUTE`).
pub const NC_ATTRIBUTE: u32 = 0x0000_000C;

/// WMO GRIB2 Section 0 Indicator Magic.
pub const GRIB2_MAGIC: [u8; 4] = *b"GRIB";
/// WMO GRIB2 Section 8 End Marker.
pub const GRIB2_END_MARKER: [u8; 4] = *b"7777";

/// Error conditions occurring during datacube ingestion, chunk decoding, or cube compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatacubeError {
    InvalidMagic, TruncatedData, UnsupportedDtype, DimensionMismatch,
    ByteBudgetExceeded { requested: usize, budget: usize },
}

/// Zarr v2 or v3 array metadata descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct ZarrArrayMeta {
    pub zarr_format: u8, pub shape: [usize; 4], pub chunks: [usize; 4],
    pub fill_value: f32, pub little_endian: bool,
}

fn extract_json_usize_array(json: &str, key: &str) -> Option<[usize; 4]> {
    let after = &json[json.find(key)? + key.len()..];
    let open = after.find('[')?;
    let close = after[open..].find(']')? + open;
    let mut out = [0usize; 4];
    let mut idx = 0;
    for part in after[open + 1..close].split(',') {
        if idx < 4 {
            if let Ok(n) = part.trim().parse::<usize>() { out[idx] = n; idx += 1; }
        }
    }
    Some(out)
}

fn extract_json_u8(json: &str, key: &str) -> Option<u8> {
    let after = &json[json.find(key)? + key.len()..];
    let slice = after[after.find(':')? + 1..].trim_start();
    let end = slice.find(|c: char| !c.is_ascii_digit()).unwrap_or(slice.len());
    slice[..end].parse::<u8>().ok()
}

fn extract_json_f32(json: &str, key: &str) -> Option<f32> {
    let after = &json[json.find(key)? + key.len()..];
    let slice = after[after.find(':')? + 1..].trim_start();
    if slice.starts_with("null") { return Some(0.0); }
    let end = slice.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E')).unwrap_or(slice.len());
    slice[..end].parse::<f32>().ok()
}

/// Parses Zarr v2 `.zarray` or Zarr v3 `zarr.json` metadata (`ColdBounded`).
pub fn parse_zarr_array_meta(json: &str, byte_budget: usize) -> Result<ZarrArrayMeta, DatacubeError> {
    if json.len() > byte_budget {
        return Err(DatacubeError::ByteBudgetExceeded { requested: json.len(), budget: byte_budget });
    }
    let zarr_format = extract_json_u8(json, "\"zarr_format\"").unwrap_or(2);
    let shape = extract_json_usize_array(json, "\"shape\"").ok_or(DatacubeError::DimensionMismatch)?;
    let chunks = extract_json_usize_array(json, "\"chunks\"")
        .or_else(|| extract_json_usize_array(json, "\"chunk_shape\""))
        .ok_or(DatacubeError::DimensionMismatch)?;
    let fill_value = extract_json_f32(json, "\"fill_value\"").unwrap_or(0.0);
    let little_endian = !(json.contains("\">f4\"") || json.contains("\">f8\"") || json.contains("\">i4\""));
    Ok(ZarrArrayMeta { zarr_format, shape, chunks, fill_value, little_endian })
}

fn count_digits(mut n: usize) -> usize {
    if n == 0 { return 1; }
    let mut d = 0;
    while n > 0 { d += 1; n /= 10; }
    d
}

/// Formats a Zarr v2/v3 chunk storage key according to dimension coordinates (`ColdBounded`).
pub fn format_zarr_chunk_key(
    meta: &ZarrArrayMeta, chunk_coords: [usize; 4], separator: char, byte_budget: usize,
) -> Result<String, DatacubeError> {
    let active = if meta.shape[3] > 0 { 4 } else if meta.shape[2] > 0 { 3 } else if meta.shape[1] > 0 { 2 } else { 1 };
    let mut needed = if meta.zarr_format == 3 { 2 } else { 0 };
    for i in 0..active {
        if i > 0 { needed += 1; }
        needed += count_digits(chunk_coords[i]);
    }
    if needed > byte_budget {
        return Err(DatacubeError::ByteBudgetExceeded { requested: needed, budget: byte_budget });
    }
    let mut out = String::with_capacity(needed);
    if meta.zarr_format == 3 { out.push('c'); out.push(separator); }
    for i in 0..active {
        if i > 0 { out.push(separator); }
        use core::fmt::Write;
        let _ = write!(out, "{}", chunk_coords[i]);
    }
    Ok(out)
}

/// Decodes an uncompressed binary f32 chunk into a caller-supplied slice (`HotZeroHeap`).
pub fn decode_zarr_f32_chunk_into(raw_bytes: &[u8], little_endian: bool, out_values: &mut [f32]) -> Result<usize, DatacubeError> {
    if raw_bytes.len() % 4 != 0 { return Err(DatacubeError::TruncatedData); }
    let count = usize::min(raw_bytes.len() / 4, out_values.len());
    for i in 0..count {
        let b = [raw_bytes[i * 4], raw_bytes[i * 4 + 1], raw_bytes[i * 4 + 2], raw_bytes[i * 4 + 3]];
        out_values[i] = if little_endian { f32::from_le_bytes(b) } else { f32::from_be_bytes(b) };
    }
    Ok(count)
}

/// NetCDF Classic (`CDF\x01`/`CDF\x02`) Header Summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetCdfHeaderSummary {
    pub version: u8, pub num_recs: u32, pub dim_count: u32, pub var_count: u32,
    pub first_var_offset: u64, pub first_var_vsize: u32,
}

struct ByteCursor<'a> { data: &'a [u8], pos: usize }

impl<'a> ByteCursor<'a> {
    #[inline] fn new(data: &'a [u8]) -> Self { Self { data, pos: 0 } }
    #[inline] fn rem(&self) -> usize { self.data.len().saturating_sub(self.pos) }
    fn read_u32_be(&mut self) -> Result<u32, DatacubeError> {
        if self.rem() < 4 { return Err(DatacubeError::TruncatedData); }
        let val = u32::from_be_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        Ok(val)
    }
    fn read_u64_be(&mut self) -> Result<u64, DatacubeError> {
        if self.rem() < 8 { return Err(DatacubeError::TruncatedData); }
        let val = u64::from_be_bytes(self.data[self.pos..self.pos + 8].try_into().unwrap());
        self.pos += 8;
        Ok(val)
    }
    fn skip(&mut self, n: usize) -> Result<(), DatacubeError> {
        if self.rem() < n { return Err(DatacubeError::TruncatedData); }
        self.pos += n;
        Ok(())
    }
    fn skip_att_list(&mut self) -> Result<(), DatacubeError> {
        let tag = self.read_u32_be()?;
        if tag == 0 { self.skip(4)?; }
        else if tag == NC_ATTRIBUTE {
            let count = self.read_u32_be()?;
            for _ in 0..count {
                let nlen = self.read_u32_be()? as usize;
                self.skip((nlen + 3) & !3)?;
                let esize = match self.read_u32_be()? {
                    1 | 2 => 1, 3 => 2, 4 | 5 => 4, 6 => 8, _ => return Err(DatacubeError::UnsupportedDtype),
                };
                let nelems = self.read_u32_be()? as usize;
                let tot = nelems.checked_mul(esize).ok_or(DatacubeError::DimensionMismatch)?;
                self.skip((tot + 3) & !3)?;
            }
        } else { return Err(DatacubeError::InvalidMagic); }
        Ok(())
    }
}

/// Parses a NetCDF-Classic (`CDF\x01` or `CDF\x02`) binary header (`HotZeroHeap`).
pub fn parse_netcdf_classic_header(bytes: &[u8]) -> Result<NetCdfHeaderSummary, DatacubeError> {
    if bytes.len() < 8 { return Err(DatacubeError::TruncatedData); }
    if &bytes[0..3] != b"CDF" { return Err(DatacubeError::InvalidMagic); }
    let version = bytes[3];
    if version != 1 && version != 2 { return Err(DatacubeError::InvalidMagic); }
    let mut cursor = ByteCursor::new(bytes);
    cursor.skip(4)?;
    let num_recs = cursor.read_u32_be()?;

    let dim_tag = cursor.read_u32_be()?;
    let dim_count = if dim_tag == 0 {
        cursor.skip(4)?; 0
    } else if dim_tag == NC_DIMENSION {
        let count = cursor.read_u32_be()?;
        for _ in 0..count {
            let nlen = cursor.read_u32_be()? as usize;
            cursor.skip((nlen + 3) & !3)?;
            cursor.skip(4)?;
        }
        count
    } else { return Err(DatacubeError::InvalidMagic); };

    cursor.skip_att_list()?;

    let var_tag = cursor.read_u32_be()?;
    let mut var_count = 0u32;
    let mut first_var_offset = 0u64;
    let mut first_var_vsize = 0u32;

    if var_tag == 0 {
        cursor.skip(4)?;
    } else if var_tag == NC_VARIABLE {
        var_count = cursor.read_u32_be()?;
        for i in 0..var_count {
            let nlen = cursor.read_u32_be()? as usize;
            cursor.skip((nlen + 3) & !3)?;
            let dimids = cursor.read_u32_be()? as usize;
            cursor.skip(dimids * 4)?;
            cursor.skip_att_list()?;
            let _nctype = cursor.read_u32_be()?;
            let vsize = cursor.read_u32_be()?;
            let begin = if version == 1 { cursor.read_u32_be()? as u64 } else { cursor.read_u64_be()? };
            if i == 0 { first_var_offset = begin; first_var_vsize = vsize; }
        }
    } else { return Err(DatacubeError::InvalidMagic); }

    Ok(NetCdfHeaderSummary { version, num_recs, dim_count, var_count, first_var_offset, first_var_vsize })
}

/// Reads a big-endian IEEE-754 `f32` slab directly into a caller slice (`HotZeroHeap`).
pub fn read_netcdf_f32_slab_into(file_bytes: &[u8], var_offset: usize, count: usize, out_f32: &mut [f32]) -> Result<(), DatacubeError> {
    if out_f32.len() < count { return Err(DatacubeError::DimensionMismatch); }
    let needed = count.checked_mul(4).ok_or(DatacubeError::DimensionMismatch)?;
    let end = var_offset.checked_add(needed).ok_or(DatacubeError::DimensionMismatch)?;
    if file_bytes.len() < end { return Err(DatacubeError::TruncatedData); }
    for i in 0..count {
        let b = [file_bytes[var_offset + i * 4], file_bytes[var_offset + i * 4 + 1], file_bytes[var_offset + i * 4 + 2], file_bytes[var_offset + i * 4 + 3]];
        out_f32[i] = f32::from_be_bytes(b);
    }
    Ok(())
}

/// Summary of a WMO GRIB2 meteorological field record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grib2RecordSummary {
    pub discipline: u8, pub total_length: u64, pub year: u16, pub month: u8, pub day: u8, pub hour: u8,
    pub nx: u32, pub ny: u32, pub ref_value_r: f32, pub binary_scale_e: i16, pub decimal_scale_d: i16,
    pub bits_per_value: u8, pub data_section7_offset: usize,
}

/// Parses WMO GRIB2 sections 0 through 8 metadata (`HotZeroHeap`).
pub fn parse_grib2_sections(bytes: &[u8]) -> Result<Grib2RecordSummary, DatacubeError> {
    if bytes.len() < 16 { return Err(DatacubeError::TruncatedData); }
    if &bytes[0..4] != &GRIB2_MAGIC { return Err(DatacubeError::InvalidMagic); }
    let discipline = bytes[6];
    if bytes[7] != 2 { return Err(DatacubeError::InvalidMagic); }
    let total_length = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
    if (bytes.len() as u64) < total_length { return Err(DatacubeError::TruncatedData); }

    let mut pos = 16usize;
    let (mut year, mut month, mut day, mut hour) = (0u16, 0u8, 0u8, 0u8);
    let (mut nx, mut ny) = (0u32, 0u32);
    let (mut ref_value_r, mut binary_scale_e, mut decimal_scale_d) = (0.0f32, 0i16, 0i16);
    let (mut bits_per_value, mut data_section7_offset) = (0u8, 0usize);

    while pos + 4 <= bytes.len() {
        if &bytes[pos..pos + 4] == &GRIB2_END_MARKER { break; }
        if pos + 5 > bytes.len() { return Err(DatacubeError::TruncatedData); }
        let sec_len = u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        if sec_len < 5 || pos + sec_len > bytes.len() { return Err(DatacubeError::TruncatedData); }
        match bytes[pos + 4] {
            1 if sec_len >= 18 => {
                year = u16::from_be_bytes([bytes[pos + 12], bytes[pos + 13]]);
                month = bytes[pos + 14]; day = bytes[pos + 15]; hour = bytes[pos + 16];
            }
            3 if sec_len >= 38 => {
                nx = u32::from_be_bytes(bytes[pos + 30..pos + 34].try_into().unwrap());
                ny = u32::from_be_bytes(bytes[pos + 34..pos + 38].try_into().unwrap());
            }
            5 if sec_len >= 20 => {
                ref_value_r = f32::from_be_bytes(bytes[pos + 11..pos + 15].try_into().unwrap());
                binary_scale_e = i16::from_be_bytes(bytes[pos + 15..pos + 17].try_into().unwrap());
                decimal_scale_d = i16::from_be_bytes(bytes[pos + 17..pos + 19].try_into().unwrap());
                bits_per_value = bytes[pos + 19];
            }
            7 => { data_section7_offset = pos + 5; }
            _ => {}
        }
        pos += sec_len;
    }

    if pos > bytes.len() || &bytes[bytes.len() - 4..] != &GRIB2_END_MARKER {
        return Err(DatacubeError::InvalidMagic);
    }
    Ok(Grib2RecordSummary {
        discipline, total_length, year, month, day, hour, nx, ny,
        ref_value_r, binary_scale_e, decimal_scale_d, bits_per_value, data_section7_offset,
    })
}

#[inline]
fn read_bits(data: &[u8], bit_offset: usize, nbits: u8) -> Option<u32> {
    if nbits == 0 { return Some(0); }
    if nbits > 32 { return None; }
    let byte_start = bit_offset / 8;
    let bit_in_byte = bit_offset % 8;
    let byte_end = (bit_offset + nbits as usize + 7) / 8;
    if byte_end > data.len() { return None; }
    let mut val = 0u64;
    for i in byte_start..byte_end { val = (val << 8) | (data[i] as u64); }
    let total_bits = (byte_end - byte_start) * 8;
    let shift = total_bits - (bit_in_byte + nbits as usize);
    let mask = (1u64 << nbits) - 1;
    Some(((val >> shift) & mask) as u32)
}

/// Unpacks WMO GRIB2 Template 5.0 Simple Packing into caller slice (`HotZeroHeap`).
pub fn unpack_grib2_simple_packing_into(
    bytes: &[u8], summary: &Grib2RecordSummary, out_field: &mut [f32],
) -> Result<usize, DatacubeError> {
    if summary.data_section7_offset >= bytes.len() { return Err(DatacubeError::TruncatedData); }
    let data_slice = &bytes[summary.data_section7_offset..];
    let total_points = if summary.nx > 0 && summary.ny > 0 {
        (summary.nx as usize).saturating_mul(summary.ny as usize)
    } else { out_field.len() };
    let count = usize::min(total_points, out_field.len());
    let two_e = (2.0f32).powi(summary.binary_scale_e as i32);
    let ten_minus_d = (10.0f32).powi(-(summary.decimal_scale_d as i32));
    let r = summary.ref_value_r;
    let nbits = summary.bits_per_value;

    if nbits == 0 {
        let val = r * ten_minus_d;
        for i in 0..count { out_field[i] = val; }
        return Ok(count);
    }
    if nbits > 32 { return Err(DatacubeError::UnsupportedDtype); }

    let mut bit_offset = 0usize;
    for i in 0..count {
        let x = read_bits(data_slice, bit_offset, nbits).ok_or(DatacubeError::TruncatedData)?;
        bit_offset += nbits as usize;
        out_field[i] = (r + (x as f32) * two_e) * ten_minus_d;
    }
    Ok(count)
}

/// Compiles heterogeneous environmental raster/datacube slices into a `.10d` tensor (`HotZeroHeap`).
pub fn compile_datacube_slices_to_tensor10d(
    x_m: &[f32], y_m: &[f32], elev_m: &[f32], unix_time_s: f32, temp_k: &[f32],
    precip_mm_hr: &[f32], wind_u_ms: &[f32], wind_v_ms: &[f32], soil_moisture_01: &[f32],
    solar_rad_wm2: &[f32], out_tensor10d: &mut [[f32; 10]],
) -> Result<usize, DatacubeError> {
    let n = x_m.len();
    if y_m.len() != n || elev_m.len() != n || temp_k.len() != n || precip_mm_hr.len() != n
        || wind_u_ms.len() != n || wind_v_ms.len() != n || soil_moisture_01.len() != n
        || solar_rad_wm2.len() != n || out_tensor10d.len() < n {
        return Err(DatacubeError::DimensionMismatch);
    }
    for i in 0..n {
        out_tensor10d[i] = [
            x_m[i], y_m[i], elev_m[i], unix_time_s, temp_k[i],
            precip_mm_hr[i], wind_u_ms[i], wind_v_ms[i], soil_moisture_01[i], solar_rad_wm2[i],
        ];
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zarr_meta_and_chunk_key_and_decode() {
        let zarr_v2_json = r#"{"chunks":[1,2,16,16],"dtype":"<f4","fill_value":-999.0,"order":"C","shape":[1,2,32,32],"zarr_format":2}"#;
        let meta_v2 = parse_zarr_array_meta(zarr_v2_json, 1024).expect("parse v2 meta");
        assert_eq!((meta_v2.zarr_format, meta_v2.shape, meta_v2.chunks), (2, [1, 2, 32, 32], [1, 2, 16, 16]));
        assert_eq!(meta_v2.fill_value, -999.0);
        assert!(meta_v2.little_endian);
        let key_v2 = format_zarr_chunk_key(&meta_v2, [0, 1, 2, 3], '.', 64).expect("v2 key");
        assert_eq!(key_v2, "0.1.2.3");

        let zarr_v3_json = r#"{"zarr_format":3,"node_type":"array","shape":[1,2,32,32],"data_type":"float32","chunk_grid":{"name":"regular","configuration":{"chunk_shape":[1,2,16,16]}},"fill_value":0.0}"#;
        let meta_v3 = parse_zarr_array_meta(zarr_v3_json, 1024).expect("parse v3 meta");
        assert_eq!(meta_v3.zarr_format, 3);
        let key_v3 = format_zarr_chunk_key(&meta_v3, [0, 2, 14, 29], '/', 64).expect("v3 key");
        assert_eq!(key_v3, "c/0/2/14/29");

        let mut raw_bytes = [0u8; 16];
        let fvals = [1.5f32, -2.25f32, 100.0f32, 0.125f32];
        for (i, &v) in fvals.iter().enumerate() { raw_bytes[i * 4..(i + 1) * 4].copy_from_slice(&v.to_le_bytes()); }
        let mut decoded = [0.0f32; 4];
        let n = decode_zarr_f32_chunk_into(&raw_bytes, true, &mut decoded).expect("decode chunk");
        assert_eq!(n, 4);
        assert_eq!(decoded, fvals);
    }

    #[test]
    fn test_netcdf_classic_header_and_slab_reader() {
        let mut buf = [0u8; 112];
        buf[0..16].copy_from_slice(&[b'C', b'D', b'F', 1, 0, 0, 0, 1, 0, 0, 0, 10, 0, 0, 0, 2]);
        buf[16..40].copy_from_slice(&[0, 0, 0, 3, b'l', b'a', b't', 0, 0, 0, 0, 10, 0, 0, 0, 3, b'l', b'o', b'n', 0, 0, 0, 0, 20]);
        buf[40..56].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 1]);
        buf[56..96].copy_from_slice(&[
            0, 0, 0, 4, b't', b'e', b'm', b'p', 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 1,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 3, 32, 0, 0, 0, 96,
        ]);
        let floats = [293.15f32, 294.5f32, 295.0f32, 296.25f32];
        for (i, &f) in floats.iter().enumerate() { buf[96 + i * 4..96 + (i + 1) * 4].copy_from_slice(&f.to_be_bytes()); }

        let summary = parse_netcdf_classic_header(&buf).expect("parse netcdf header");
        assert_eq!((summary.version, summary.num_recs, summary.dim_count, summary.var_count), (1, 1, 2, 1));
        assert_eq!((summary.first_var_offset, summary.first_var_vsize), (96, 800));

        let mut out = [0.0f32; 4];
        read_netcdf_f32_slab_into(&buf, 96, 4, &mut out).expect("read f32 slab");
        assert_eq!(out, floats);
    }

    #[test]
    fn test_grib2_sections_and_unpacking() {
        let mut msg = [0u8; 149];
        msg[0..16].copy_from_slice(&[b'G', b'R', b'I', b'B', 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 149]);
        msg[16..37].copy_from_slice(&[0, 0, 0, 21, 1, 0, 0, 0, 0, 0, 0, 0, 0x07, 0xEA, 10, 28, 12, 0, 0, 0, 1]);
        msg[37..42].copy_from_slice(&[0, 0, 0, 72, 3]);
        msg[67..75].copy_from_slice(&[0, 0, 0, 2, 0, 0, 0, 2]);
        msg[109..114].copy_from_slice(&[0, 0, 0, 21, 5]);
        msg[120..124].copy_from_slice(&273.15f32.to_be_bytes());
        msg[124..129].copy_from_slice(&[0, 1, 0, 1, 8]);
        msg[130..136].copy_from_slice(&[0, 0, 0, 6, 6, 255]);
        msg[136..145].copy_from_slice(&[0, 0, 0, 9, 7, 0, 5, 10, 20]);
        msg[145..149].copy_from_slice(&GRIB2_END_MARKER);

        let summary = parse_grib2_sections(&msg).expect("parse grib2 sections");
        assert_eq!((summary.discipline, summary.year, summary.month, summary.day, summary.hour), (0, 2026, 10, 28, 12));
        assert_eq!((summary.nx, summary.ny, summary.binary_scale_e, summary.decimal_scale_d, summary.bits_per_value), (2, 2, 1, 1, 8));
        assert_eq!(summary.data_section7_offset, 141);
        assert!((summary.ref_value_r - 273.15).abs() < 1e-4);

        let mut field = [0.0f32; 4];
        let count = unpack_grib2_simple_packing_into(&msg, &summary, &mut field).expect("unpack grib2");
        assert_eq!(count, 4);
        for (val, expected) in field.iter().zip([27.315, 28.315, 29.315, 31.315]) {
            assert!((val - expected).abs() < 1e-3);
        }
    }

    #[test]
    fn test_tensor10d_compiler_and_budget_exceeded() {
        let (x, y, elev) = ([100.0, 101.0], [200.0, 201.0], [50.0, 52.0]);
        let t_sec = 1700000000.0;
        let (temp, precip, u, v) = ([298.15, 299.0], [0.0, 1.2], [3.5, 4.0], [-1.2, -0.8]);
        let (sm, solar) = ([0.25, 0.30], [800.0, 750.0]);

        let mut out = [[0.0f32; 10]; 2];
        let n = compile_datacube_slices_to_tensor10d(
            &x, &y, &elev, t_sec, &temp, &precip, &u, &v, &sm, &solar, &mut out,
        ).expect("compile tensor10d");
        assert_eq!(n, 2);
        assert_eq!(out[0], [100.0, 200.0, 50.0, 1700000000.0, 298.15, 0.0, 3.5, -1.2, 0.25, 800.0]);
        assert_eq!(out[1], [101.0, 201.0, 52.0, 1700000000.0, 299.0, 1.2, 4.0, -0.8, 0.30, 750.0]);

        // Mismatched length should fail closed
        let short_y = [200.0];
        assert_eq!(
            compile_datacube_slices_to_tensor10d(&x, &short_y, &elev, t_sec, &temp, &precip, &u, &v, &sm, &solar, &mut out),
            Err(DatacubeError::DimensionMismatch)
        );

        // ByteBudgetExceeded fail-closed test
        let json = r#"{"shape":[1,2,3,4],"chunks":[1,2,3,4]}"#;
        assert_eq!(
            parse_zarr_array_meta(json, 5),
            Err(DatacubeError::ByteBudgetExceeded { requested: json.len(), budget: 5 })
        );
    }
}
