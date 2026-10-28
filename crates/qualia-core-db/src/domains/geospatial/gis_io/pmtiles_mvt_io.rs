//! Protomaps PMTiles v3 Archive Header & Hilbert Directory Index +
//! Mapbox Vector Tile (MVT v2.1) Protobuf Wire-Format & ZigZag Command Stream Codec.
//!
//! Two-Tier Zero-Heap Architecture:
//! - Tier-1 (`HotZeroHeap`): 127-byte header codec, Hilbert `(z, x, y)` bijection,
//!   ZigZag command encoding/decoding, and binary directory lookup allocate zero heap.
//! - Tier-2 (`ColdBounded`): Vector tile protobuf encoding/decoding enforce strict
//!   caller-supplied `byte_budget` limits and fail closed on budget exhaustion.
//!
//! Preserves MIT / BSD-3-Clause attribution for Protomaps PMTiles and Mapbox Vector Tile specifications.

pub const PMTILES_V3_MAGIC: [u8; 7] = *b"PMTiles";
pub const MVT_DEFAULT_EXTENT: u32 = 4096;

/// Errors occurring during PMTiles or MVT vector tile encoding/decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileIoError {
    InvalidMagic,
    UnsupportedVersion(u8),
    TruncatedData,
    InvalidCommand(u32),
    InvalidZoom(u8),
    ByteBudgetExceeded { requested: usize, budget: usize },
}

/// PMTiles v3 Fixed 127-Byte Archive Header.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PmTilesHeaderV3 {
    pub root_dir_offset: u64, pub root_dir_bytes: u64,
    pub json_metadata_offset: u64, pub json_metadata_bytes: u64,
    pub leaf_dirs_offset: u64, pub leaf_dirs_bytes: u64,
    pub tile_data_offset: u64, pub tile_data_bytes: u64,
    pub addressed_tiles_count: u64, pub tile_entries_count: u64, pub tile_contents_count: u64,
    pub clustered: bool, pub internal_compression: u8, pub tile_compression: u8, pub tile_type: u8,
    pub min_zoom: u8, pub max_zoom: u8,
    pub min_lon_e7: i32, pub min_lat_e7: i32, pub max_lon_e7: i32, pub max_lat_e7: i32,
}

/// Encodes a PMTiles v3 header into a fixed 127-byte array (HotZeroHeap).
pub fn encode_pmtiles_header_v3(hdr: &PmTilesHeaderV3) -> [u8; 127] {
    let mut buf = [0u8; 127];
    buf[0..7].copy_from_slice(&PMTILES_V3_MAGIC);
    buf[7] = 3;
    let u64s = [
        hdr.root_dir_offset, hdr.root_dir_bytes, hdr.json_metadata_offset, hdr.json_metadata_bytes,
        hdr.leaf_dirs_offset, hdr.leaf_dirs_bytes, hdr.tile_data_offset, hdr.tile_data_bytes,
        hdr.addressed_tiles_count, hdr.tile_entries_count, hdr.tile_contents_count,
    ];
    for (i, &v) in u64s.iter().enumerate() {
        buf[8 + i * 8..16 + i * 8].copy_from_slice(&v.to_le_bytes());
    }
    buf[96..102].copy_from_slice(&[
        u8::from(hdr.clustered), hdr.internal_compression, hdr.tile_compression,
        hdr.tile_type, hdr.min_zoom, hdr.max_zoom,
    ]);
    let i32s = [hdr.min_lon_e7, hdr.min_lat_e7, hdr.max_lon_e7, hdr.max_lat_e7];
    for (i, &v) in i32s.iter().enumerate() {
        buf[102 + i * 4..106 + i * 4].copy_from_slice(&v.to_le_bytes());
    }
    buf
}

/// Decodes a PMTiles v3 header from a fixed 127-byte array (HotZeroHeap).
pub fn decode_pmtiles_header_v3(bytes: &[u8; 127]) -> Result<PmTilesHeaderV3, TileIoError> {
    if bytes[0..7] != PMTILES_V3_MAGIC { return Err(TileIoError::InvalidMagic); }
    if bytes[7] != 3 { return Err(TileIoError::UnsupportedVersion(bytes[7])); }
    let u64_at = |off: usize| -> u64 { u64::from_le_bytes(bytes[off..off + 8].try_into().unwrap()) };
    let i32_at = |off: usize| -> i32 { i32::from_le_bytes(bytes[off..off + 4].try_into().unwrap()) };
    Ok(PmTilesHeaderV3 {
        root_dir_offset: u64_at(8), root_dir_bytes: u64_at(16),
        json_metadata_offset: u64_at(24), json_metadata_bytes: u64_at(32),
        leaf_dirs_offset: u64_at(40), leaf_dirs_bytes: u64_at(48),
        tile_data_offset: u64_at(56), tile_data_bytes: u64_at(64),
        addressed_tiles_count: u64_at(72), tile_entries_count: u64_at(80), tile_contents_count: u64_at(88),
        clustered: bytes[96] != 0, internal_compression: bytes[97], tile_compression: bytes[98], tile_type: bytes[99],
        min_zoom: bytes[100], max_zoom: bytes[101],
        min_lon_e7: i32_at(102), min_lat_e7: i32_at(106), max_lon_e7: i32_at(110), max_lat_e7: i32_at(114),
    })
}

/// Converts slippy map coordinates (z, x, y) to a PMTiles v3 Hilbert Tile ID (HotZeroHeap).
pub fn zxy_to_pmtiles_hilbert_id(z: u8, x: u32, y: u32) -> Result<u64, TileIoError> {
    if z > 26 { return Err(TileIoError::InvalidZoom(z)); }
    let max = 1u64 << z;
    if (x as u64) >= max || (y as u64) >= max { return Err(TileIoError::InvalidCommand(x)); }
    let base = if z == 0 { 0 } else { ((1u64 << (2 * z as u32)) - 1) / 3 };
    let (mut tx, mut ty, mut d, mut s) = (x as u64, y as u64, 0u64, (1u64 << z) / 2);
    while s > 0 {
        let (rx, ry) = (u64::from((tx & s) > 0), u64::from((ty & s) > 0));
        d += s * s * ((3 * rx) ^ ry);
        tx &= s - 1; ty &= s - 1;
        if ry == 0 {
            if rx == 1 { tx = s - 1 - tx; ty = s - 1 - ty; }
            std::mem::swap(&mut tx, &mut ty);
        }
        s /= 2;
    }
    Ok(base + d)
}

/// Converts a PMTiles v3 Hilbert Tile ID back to slippy map coordinates (z, x, y) (HotZeroHeap).
pub fn pmtiles_hilbert_id_to_zxy(tile_id: u64) -> Result<(u8, u32, u32), TileIoError> {
    if tile_id == 0 { return Ok((0, 0, 0)); }
    let c = 3u128 * (tile_id as u128) + 1;
    let z = ((127 - c.leading_zeros()) / 2) as u8;
    if z > 26 { return Err(TileIoError::InvalidZoom(z)); }
    let base = ((1u64 << (2 * z as u32)) - 1) / 3;
    let (mut t, mut x, mut y, n) = (tile_id - base, 0u32, 0u32, 1u32 << z);
    let mut s = 1u32;
    while s < n {
        let rx = (1 & (t / 2)) as u32;
        let ry = (1 & (t ^ (rx as u64))) as u32;
        if ry == 0 {
            if rx != 0 { x = s - 1 - x; y = s - 1 - y; }
            std::mem::swap(&mut x, &mut y);
        }
        t /= 4; x += s * rx; y += s * ry; s <<= 1;
    }
    Ok((z, x, y))
}

/// PMTiles v3 Directory Entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PmTilesDirEntry { pub tile_id: u64, pub offset: u64, pub length: u32, pub run_length: u32 }

/// Performs a binary search lookup on PMTiles directory entries supporting run-length encoding.
pub fn lookup_pmtiles_entry(entries: &[PmTilesDirEntry], target_tile_id: u64) -> Option<(u64, u32)> {
    if entries.is_empty() { return None; }
    let idx = entries.partition_point(|e| e.tile_id <= target_tile_id);
    let entry = entries.get(idx.checked_sub(1)?)?;
    if entry.run_length == 0 || target_tile_id < entry.tile_id.saturating_add(entry.run_length as u64) {
        Some((entry.offset, entry.length))
    } else {
        None
    }
}

/// 32-bit ZigZag integer encoding (HotZeroHeap).
#[inline] pub fn zigzag_encode(val: i32) -> u32 { ((val << 1) ^ (val >> 31)) as u32 }

/// 32-bit ZigZag integer decoding (HotZeroHeap).
#[inline] pub fn zigzag_decode(zz: u32) -> i32 { ((zz >> 1) as i32) ^ -((zz & 1) as i32) }

/// Encodes geometry rings into MVT v2.1 ZigZag commands with zero heap allocations (HotZeroHeap).
pub fn encode_mvt_geometry_commands(
    rings: &[&[[i32; 2]]],
    is_polygon: bool,
    out_cmds: &mut [u32],
) -> Result<usize, TileIoError> {
    let (mut cx, mut cy, mut w) = (0i32, 0i32, 0usize);
    for ring in rings {
        let pts = match ring.split_last() {
            Some((last, rest)) if is_polygon && ring.first() == Some(last) && ring.len() >= 4 => rest,
            _ => *ring,
        };
        if pts.is_empty() { continue; }
        let need = if pts.len() == 1 { 3 } else { 4 + (pts.len() - 1) * 2 } + usize::from(is_polygon);
        if w + need > out_cmds.len() {
            return Err(TileIoError::ByteBudgetExceeded { requested: w + need, budget: out_cmds.len() });
        }
        out_cmds[w] = (1 << 3) | 1; w += 1;
        out_cmds[w] = zigzag_encode(pts[0][0].wrapping_sub(cx)); w += 1;
        out_cmds[w] = zigzag_encode(pts[0][1].wrapping_sub(cy)); w += 1;
        cx = pts[0][0]; cy = pts[0][1];
        if pts.len() > 1 {
            out_cmds[w] = ((pts.len() as u32 - 1) << 3) | 2; w += 1;
            for p in &pts[1..] {
                out_cmds[w] = zigzag_encode(p[0].wrapping_sub(cx)); w += 1;
                out_cmds[w] = zigzag_encode(p[1].wrapping_sub(cy)); w += 1;
                cx = p[0]; cy = p[1];
            }
        }
        if is_polygon { out_cmds[w] = (1 << 3) | 7; w += 1; }
    }
    Ok(w)
}

/// Decodes MVT v2.1 geometry command stream into rings under an explicit byte budget (ColdBounded).
pub fn decode_mvt_geometry_commands(cmds: &[u32], byte_budget: usize) -> Result<Vec<Vec<[i32; 2]>>, TileIoError> {
    let (mut out, mut cur, mut cx, mut cy, mut alloc, mut i) = (Vec::new(), Vec::new(), 0i32, 0i32, 0usize, 0usize);
    while i < cmds.len() {
        let cmd = cmds[i]; i += 1;
        let (id, count) = (cmd & 7, (cmd >> 3) as usize);
        match id {
            1 => {
                if count == 0 || i + count * 2 > cmds.len() {
                    return Err(if count == 0 { TileIoError::InvalidCommand(cmd) } else { TileIoError::TruncatedData });
                }
                if count > 1 {
                    for _ in 0..count {
                        cx = cx.wrapping_add(zigzag_decode(cmds[i])); cy = cy.wrapping_add(zigzag_decode(cmds[i + 1])); i += 2;
                        alloc += 32; if alloc > byte_budget { return Err(TileIoError::ByteBudgetExceeded { requested: alloc, budget: byte_budget }); }
                        out.push(vec![[cx, cy]]);
                    }
                } else {
                    if !cur.is_empty() {
                        alloc += 24; if alloc > byte_budget { return Err(TileIoError::ByteBudgetExceeded { requested: alloc, budget: byte_budget }); }
                        out.push(std::mem::take(&mut cur));
                    }
                    cx = cx.wrapping_add(zigzag_decode(cmds[i])); cy = cy.wrapping_add(zigzag_decode(cmds[i + 1])); i += 2;
                    alloc += 8; if alloc > byte_budget { return Err(TileIoError::ByteBudgetExceeded { requested: alloc, budget: byte_budget }); }
                    cur.push([cx, cy]);
                }
            }
            2 => {
                if count == 0 || cur.is_empty() || i + count * 2 > cmds.len() {
                    return Err(if cur.is_empty() || count == 0 { TileIoError::InvalidCommand(cmd) } else { TileIoError::TruncatedData });
                }
                for _ in 0..count {
                    cx = cx.wrapping_add(zigzag_decode(cmds[i])); cy = cy.wrapping_add(zigzag_decode(cmds[i + 1])); i += 2;
                    alloc += 8; if alloc > byte_budget { return Err(TileIoError::ByteBudgetExceeded { requested: alloc, budget: byte_budget }); }
                    cur.push([cx, cy]);
                }
            }
            7 => {
                if count != 1 { return Err(TileIoError::InvalidCommand(cmd)); }
                if cur.len() >= 3 {
                    let first = cur[0];
                    alloc += 32; if alloc > byte_budget { return Err(TileIoError::ByteBudgetExceeded { requested: alloc, budget: byte_budget }); }
                    cur.push(first);
                    out.push(std::mem::take(&mut cur));
                } else { cur.clear(); }
            }
            _ => return Err(TileIoError::InvalidCommand(cmd)),
        }
    }
    if !cur.is_empty() {
        alloc += 24; if alloc > byte_budget { return Err(TileIoError::ByteBudgetExceeded { requested: alloc, budget: byte_budget }); }
        out.push(cur);
    }
    Ok(out)
}

/// Mapbox Vector Tile (MVT v2.1) Layer.
#[derive(Debug, Clone, PartialEq)]
pub struct MvtLayer { pub name: String, pub extent: u32, pub features_geom_cmds: Vec<(u8, Vec<u32>)> }

#[inline] fn encode_varint(mut v: u64, out: &mut Vec<u8>) {
    while v >= 0x80 { out.push(((v & 0x7F) as u8) | 0x80); v >>= 7; }
    out.push(v as u8);
}

#[inline] fn decode_varint(b: &[u8], pos: &mut usize) -> Result<u64, TileIoError> {
    let (mut res, mut shift) = (0u64, 0u32);
    while *pos < b.len() {
        let byte = b[*pos]; *pos += 1;
        res |= ((byte & 0x7F) as u64) << shift;
        if (byte & 0x80) == 0 { return Ok(res); }
        shift += 7;
        if shift >= 64 { return Err(TileIoError::TruncatedData); }
    }
    Err(TileIoError::TruncatedData)
}

#[inline] fn encode_field_uint(out: &mut Vec<u8>, f: u32, v: u64) { encode_varint((f as u64) << 3, out); encode_varint(v, out); }
#[inline] fn encode_field_bytes(out: &mut Vec<u8>, f: u32, b: &[u8]) { encode_varint(((f as u64) << 3) | 2, out); encode_varint(b.len() as u64, out); out.extend_from_slice(b); }

fn skip_field(slice: &[u8], pos: &mut usize, wire: u64) -> Result<(), TileIoError> {
    match wire {
        0 => { decode_varint(slice, pos)?; Ok(()) }
        1 => { if *pos + 8 > slice.len() { return Err(TileIoError::TruncatedData); } *pos += 8; Ok(()) }
        2 => { let len = decode_varint(slice, pos)? as usize; if *pos + len > slice.len() { return Err(TileIoError::TruncatedData); } *pos += len; Ok(()) }
        5 => { if *pos + 4 > slice.len() { return Err(TileIoError::TruncatedData); } *pos += 4; Ok(()) }
        _ => Err(TileIoError::InvalidCommand(wire as u32)),
    }
}

/// Encodes an MVT layer into a protobuf Tile wire payload with byte budget (ColdBounded).
pub fn encode_mvt_layer_pbf(layer: &MvtLayer, byte_budget: usize) -> Result<Vec<u8>, TileIoError> {
    let mut layer_buf = Vec::new();
    encode_field_uint(&mut layer_buf, 15, 2);
    encode_field_bytes(&mut layer_buf, 1, layer.name.as_bytes());
    for (kind, cmds) in &layer.features_geom_cmds {
        let mut feat_buf = Vec::new();
        encode_field_uint(&mut feat_buf, 3, *kind as u64);
        let mut geom_buf = Vec::new();
        for &c in cmds { encode_varint(c as u64, &mut geom_buf); }
        encode_field_bytes(&mut feat_buf, 4, &geom_buf);
        encode_field_bytes(&mut layer_buf, 2, &feat_buf);
        if layer_buf.len() > byte_budget { return Err(TileIoError::ByteBudgetExceeded { requested: layer_buf.len(), budget: byte_budget }); }
    }
    encode_field_uint(&mut layer_buf, 5, layer.extent as u64);
    let mut tile_buf = Vec::new();
    encode_field_bytes(&mut tile_buf, 3, &layer_buf);
    if tile_buf.len() > byte_budget { return Err(TileIoError::ByteBudgetExceeded { requested: tile_buf.len(), budget: byte_budget }); }
    Ok(tile_buf)
}

/// Decodes an MVT layer from a protobuf Tile wire payload with byte budget (ColdBounded).
pub fn decode_mvt_layer_pbf(pbf: &[u8], byte_budget: usize) -> Result<MvtLayer, TileIoError> {
    if pbf.len() > byte_budget { return Err(TileIoError::ByteBudgetExceeded { requested: pbf.len(), budget: byte_budget }); }
    let mut slice = pbf;
    if !pbf.is_empty() {
        let mut test_pos = 0;
        if let Ok(tag) = decode_varint(pbf, &mut test_pos) {
            if tag == ((3 << 3) | 2) {
                let len = decode_varint(pbf, &mut test_pos)? as usize;
                if test_pos + len <= pbf.len() { slice = &pbf[test_pos..test_pos + len]; }
            }
        }
    }
    let (mut name, mut extent, mut features, mut alloc, mut pos) = (String::new(), MVT_DEFAULT_EXTENT, Vec::new(), 0usize, 0usize);
    while pos < slice.len() {
        let tag = decode_varint(slice, &mut pos)?;
        let (field, wire) = ((tag >> 3) as u32, tag & 7);
        match field {
            1 => {
                let len = decode_varint(slice, &mut pos)? as usize;
                if pos + len > slice.len() { return Err(TileIoError::TruncatedData); }
                alloc += len + 24; if alloc > byte_budget { return Err(TileIoError::ByteBudgetExceeded { requested: alloc, budget: byte_budget }); }
                name = String::from_utf8_lossy(&slice[pos..pos + len]).into_owned();
                pos += len;
            }
            2 => {
                let len = decode_varint(slice, &mut pos)? as usize;
                if pos + len > slice.len() { return Err(TileIoError::TruncatedData); }
                let feat = &slice[pos..pos + len]; pos += len;
                let (mut f_pos, mut geom_type, mut cmds) = (0usize, 0u8, Vec::new());
                while f_pos < feat.len() {
                    let f_tag = decode_varint(feat, &mut f_pos)?;
                    let (f_num, f_wire) = ((f_tag >> 3) as u32, f_tag & 7);
                    match f_num {
                        3 => geom_type = decode_varint(feat, &mut f_pos)? as u8,
                        4 => {
                            let g_len = decode_varint(feat, &mut f_pos)? as usize;
                            if f_pos + g_len > feat.len() { return Err(TileIoError::TruncatedData); }
                            let end = f_pos + g_len;
                            while f_pos < end {
                                cmds.push(decode_varint(feat, &mut f_pos)? as u32);
                                alloc += 4; if alloc > byte_budget { return Err(TileIoError::ByteBudgetExceeded { requested: alloc, budget: byte_budget }); }
                            }
                        }
                        _ => skip_field(feat, &mut f_pos, f_wire)?,
                    }
                }
                alloc += 32; if alloc > byte_budget { return Err(TileIoError::ByteBudgetExceeded { requested: alloc, budget: byte_budget }); }
                features.push((geom_type, cmds));
            }
            5 => extent = decode_varint(slice, &mut pos)? as u32,
            15 => { let _ = decode_varint(slice, &mut pos)?; }
            _ => skip_field(slice, &mut pos, wire)?,
        }
    }
    Ok(MvtLayer { name, extent, features_geom_cmds: features })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pmtiles_header_v3_roundtrip() {
        let hdr = PmTilesHeaderV3 {
            root_dir_offset: 127, root_dir_bytes: 4096,
            json_metadata_offset: 4223, json_metadata_bytes: 2048,
            leaf_dirs_offset: 6271, leaf_dirs_bytes: 1024,
            tile_data_offset: 7295, tile_data_bytes: 1048576,
            addressed_tiles_count: 500, tile_entries_count: 450, tile_contents_count: 400,
            clustered: true, internal_compression: 1, tile_compression: 2, tile_type: 1,
            min_zoom: 2, max_zoom: 14,
            min_lon_e7: -122_419_400, min_lat_e7: 37_774_900,
            max_lon_e7: -122_000_000, max_lat_e7: 38_000_000,
        };
        let encoded = encode_pmtiles_header_v3(&hdr);
        assert_eq!(encoded.len(), 127);
        let decoded = decode_pmtiles_header_v3(&encoded).expect("decode header failed");
        assert_eq!(decoded, hdr);

        let mut invalid_magic = encoded;
        invalid_magic[0] = b'X';
        assert_eq!(decode_pmtiles_header_v3(&invalid_magic), Err(TileIoError::InvalidMagic));

        let mut invalid_version = encoded;
        invalid_version[7] = 2;
        assert_eq!(decode_pmtiles_header_v3(&invalid_version), Err(TileIoError::UnsupportedVersion(2)));
    }

    #[test]
    fn test_zxy_hilbert_id_bijection() {
        for (z, x, y, expected) in [(0, 0, 0, 0), (1, 0, 0, 1), (1, 0, 1, 2), (1, 1, 1, 3), (1, 1, 0, 4), (2, 0, 0, 5)] {
            assert_eq!(zxy_to_pmtiles_hilbert_id(z, x, y).unwrap(), expected);
            assert_eq!(pmtiles_hilbert_id_to_zxy(expected).unwrap(), (z, x, y));
        }

        for z in 0..=5 {
            let max = 1u32 << (z as u32);
            for x in 0..max {
                for y in 0..max {
                    let id = zxy_to_pmtiles_hilbert_id(z, x, y).unwrap();
                    let (rz, rx, ry) = pmtiles_hilbert_id_to_zxy(id).unwrap();
                    assert_eq!((rz, rx, ry), (z, x, y));
                }
            }
        }

        for z in 6..=16 {
            let max = (1u32 << (z as u32)) - 1;
            let samples = [(0, 0), (max, 0), (0, max), (max, max), (max / 2, max / 2)];
            for (x, y) in samples {
                let id = zxy_to_pmtiles_hilbert_id(z, x, y).unwrap();
                let (rz, rx, ry) = pmtiles_hilbert_id_to_zxy(id).unwrap();
                assert_eq!((rz, rx, ry), (z, x, y));
            }
        }
    }

    #[test]
    fn test_lookup_pmtiles_entry() {
        let entries = [
            PmTilesDirEntry { tile_id: 1, offset: 100, length: 50, run_length: 1 },
            PmTilesDirEntry { tile_id: 5, offset: 200, length: 75, run_length: 3 },
            PmTilesDirEntry { tile_id: 10, offset: 500, length: 120, run_length: 0 },
        ];
        for (target, expected) in [
            (0, None), (1, Some((100, 50))), (2, None), (5, Some((200, 75))),
            (6, Some((200, 75))), (7, Some((200, 75))), (8, None), (10, Some((500, 120))),
        ] {
            assert_eq!(lookup_pmtiles_entry(&entries, target), expected);
        }
    }

    #[test]
    fn test_zigzag_and_geometry_codec() {
        for v in [-1000, -1, 0, 1, 1000] {
            assert_eq!(zigzag_decode(zigzag_encode(v)), v);
        }

        let ring = [[10, 10], [20, 10], [20, 20], [10, 20], [10, 10]];
        let mut cmds = [0u32; 32];
        let written = encode_mvt_geometry_commands(&[&ring], true, &mut cmds).unwrap();
        assert!(written > 0);

        let decoded = decode_mvt_geometry_commands(&cmds[..written], 1024).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], ring);

        let line = [[0, 0], [10, 15], [25, 30]];
        let written_line = encode_mvt_geometry_commands(&[&line], false, &mut cmds).unwrap();
        let decoded_line = decode_mvt_geometry_commands(&cmds[..written_line], 1024).unwrap();
        assert_eq!(decoded_line.len(), 1);
        assert_eq!(decoded_line[0], line);
    }

    #[test]
    fn test_mvt_layer_pbf_roundtrip_and_budget() {
        let mut cmds = [0u32; 16];
        let poly = [[0, 0], [10, 0], [10, 10], [0, 0]];
        let n = encode_mvt_geometry_commands(&[&poly], true, &mut cmds).unwrap();

        let layer = MvtLayer {
            name: "waterways".to_string(),
            extent: 4096,
            features_geom_cmds: vec![(3, cmds[..n].to_vec())],
        };

        let pbf = encode_mvt_layer_pbf(&layer, 4096).unwrap();
        let decoded = decode_mvt_layer_pbf(&pbf, 4096).unwrap();
        assert_eq!(decoded, layer);

        let res = encode_mvt_layer_pbf(&layer, 10);
        assert!(matches!(res, Err(TileIoError::ByteBudgetExceeded { .. })));

        let res_decode = decode_mvt_layer_pbf(&pbf, 10);
        assert!(matches!(res_decode, Err(TileIoError::ByteBudgetExceeded { .. })));
    }
}
