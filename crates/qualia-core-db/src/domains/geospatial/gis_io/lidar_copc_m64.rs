//! ASPRS LAS 1.2–1.4 & COPC Octree Ingestion, Cloth Simulation Filter (CSF),
//! 64-Byte `.m64` Mode 0x00 LiDAR Point Container, and DSM/DTM/CHM Rasterizer.

/// ASPRS Standard LiDAR Point Classes (ASPRS LAS 1.4 Specification).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsprsClass {
    CreatedNeverClassified = 0, Unclassified = 1, Ground = 2, LowVegetation = 3,
    MediumVegetation = 4, HighVegetation = 5, Building = 6, LowPointNoise = 7,
    Water = 9, Rail = 10, RoadSurface = 11, WireGuard = 13, WireConductor = 14,
    TransmissionTower = 15, BridgeDeck = 17, HighNoise = 18,
}

/// Summary of an ASPRS LAS 1.2–1.4 binary header.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LasHeaderSummary {
    pub version_major: u8, pub version_minor: u8, pub point_data_format: u8,
    pub point_data_record_length: u16, pub offset_to_point_data: u32, pub point_count: u64,
    pub scale: [f64; 3], pub offset: [f64; 3], pub min_bounds: [f64; 3], pub max_bounds: [f64; 3],
}

/// Cloud-Optimized Point Cloud (COPC) Octree Voxel Hierarchy Entry (32 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopcOctreeVoxelKey {
    pub depth: i32, pub x: i32, pub y: i32, pub z: i32, pub point_offset: u64, pub point_count: u32,
}

/// Errors occurring during LiDAR ingestion, validation, or rasterization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LidarError {
    InvalidMagic, TruncatedData, UnsupportedFormat(u8),
    CrcMismatch, BufferTooSmall, ByteBudgetExceeded { requested: usize, budget: usize },
}

/// Submagic for Mode 0x00 point tensors in `.m64` containers.
pub const M64_LIDAR_SUBMAGIC: u8 = 0x00;

/// 64-Byte Aligned LiDAR Point Container (`HotZeroHeap`).
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct M64LidarPoint {
    pub x: f64, pub y: f64, pub z: f64, pub intensity: u16,
    pub return_num: u8, pub num_returns: u8, pub classification: u8, pub scan_angle_rank: i8,
    pub red: u16, pub green: u16, pub blue: u16, pub nir: u16, pub hag_m: f32,
    pub gps_time_s: f64, pub submagic: u8, pub flags: u8, pub reserved: u16,
    pub crc32c: u32, pub _pad: [u8; 0],
}

const _: () = {
    assert!(core::mem::size_of::<M64LidarPoint>() == 64);
    assert!(core::mem::align_of::<M64LidarPoint>() == 64);
};

/// Castagnoli CRC-32C calculation (RFC 3720 / qualiaDB standard).
#[inline]
fn crc32c_fast(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if (crc & 1) != 0 { (crc >> 1) ^ 0x82F6_3B78 } else { crc >> 1 };
        }
    }
    !crc
}

impl Default for M64LidarPoint {
    fn default() -> Self {
        let mut pt = Self {
            x: 0.0, y: 0.0, z: 0.0, intensity: 0, return_num: 1, num_returns: 1,
            classification: AsprsClass::CreatedNeverClassified as u8, scan_angle_rank: 0,
            red: 0, green: 0, blue: 0, nir: 0, hag_m: 0.0, gps_time_s: 0.0,
            submagic: M64_LIDAR_SUBMAGIC, flags: 0, reserved: 0, crc32c: 0, _pad: [],
        };
        pt.seal_crc();
        pt
    }
}

impl M64LidarPoint {
    #[inline] pub fn seal_crc(&mut self) { self.crc32c = crc32c_fast(&self.to_bytes()[..60]); }
    #[inline] pub fn verify_crc(&self) -> bool { self.crc32c == crc32c_fast(&self.to_bytes()[..60]) }
    #[inline] pub fn to_bytes(&self) -> [u8; 64] { unsafe { core::mem::transmute_copy(self) } }
    #[inline]
    pub fn from_bytes_checked(b: &[u8; 64]) -> Result<Self, LidarError> {
        let pt: Self = unsafe { core::mem::transmute_copy(b) };
        if pt.submagic != M64_LIDAR_SUBMAGIC { return Err(LidarError::InvalidMagic); }
        if !pt.verify_crc() { return Err(LidarError::CrcMismatch); }
        Ok(pt)
    }
}

#[inline] fn r_u16(b: &[u8]) -> u16 { u16::from_le_bytes([b[0], b[1]]) }
#[inline] fn r_u32(b: &[u8]) -> u32 { u32::from_le_bytes([b[0], b[1], b[2], b[3]]) }
#[inline] fn r_i32(b: &[u8]) -> i32 { i32::from_le_bytes([b[0], b[1], b[2], b[3]]) }
#[inline] fn r_u64(b: &[u8]) -> u64 { u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]) }
#[inline] fn r_f64(b: &[u8]) -> f64 { f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]) }

/// Parses an ASPRS LAS 1.2–1.4 binary header.
pub fn parse_las_header(bytes: &[u8]) -> Result<LasHeaderSummary, LidarError> {
    if bytes.len() < 227 { return Err(LidarError::TruncatedData); }
    if &bytes[0..4] != b"LASF" { return Err(LidarError::InvalidMagic); }
    let (version_major, version_minor) = (bytes[24], bytes[25]);
    if version_major != 1 || version_minor < 2 || version_minor > 4 {
        return Err(LidarError::UnsupportedFormat(version_minor));
    }
    let offset_to_point_data = r_u32(&bytes[96..100]);
    let point_data_format = bytes[104];
    let point_data_record_length = r_u16(&bytes[105..107]);
    let legacy_point_count = r_u32(&bytes[107..111]) as u64;

    let scale = [r_f64(&bytes[131..139]), r_f64(&bytes[139..147]), r_f64(&bytes[147..155])];
    let offset = [r_f64(&bytes[155..163]), r_f64(&bytes[163..171]), r_f64(&bytes[171..179])];
    let (max_x, min_x) = (r_f64(&bytes[179..187]), r_f64(&bytes[187..195]));
    let (max_y, min_y) = (r_f64(&bytes[195..203]), r_f64(&bytes[203..211]));
    let (max_z, min_z) = (r_f64(&bytes[211..219]), r_f64(&bytes[219..227]));

    let point_count = if version_minor >= 4 && bytes.len() >= 275 {
        let count_64 = r_u64(&bytes[267..275]);
        if count_64 > 0 { count_64 } else { legacy_point_count }
    } else {
        legacy_point_count
    };

    Ok(LasHeaderSummary {
        version_major, version_minor, point_data_format, point_data_record_length,
        offset_to_point_data, point_count, scale, offset,
        min_bounds: [min_x, min_y, min_z], max_bounds: [max_x, max_y, max_z],
    })
}

/// Decodes a 20-byte ASPRS LAS format 0 point record into `M64LidarPoint` (`HotZeroHeap`).
pub fn decode_las_point_format0(record: &[u8], hdr: &LasHeaderSummary) -> Result<M64LidarPoint, LidarError> {
    if record.len() < 20 { return Err(LidarError::TruncatedData); }
    let (raw_x, raw_y, raw_z) = (r_i32(&record[0..4]), r_i32(&record[4..8]), r_i32(&record[8..12]));
    let (intensity, return_byte, classification, scan_angle_rank) =
        (r_u16(&record[12..14]), record[14], record[15], record[16] as i8);

    let x = (raw_x as f64) * hdr.scale[0] + hdr.offset[0];
    let y = (raw_y as f64) * hdr.scale[1] + hdr.offset[1];
    let z = (raw_z as f64) * hdr.scale[2] + hdr.offset[2];

    let mut pt = M64LidarPoint {
        x, y, z, intensity, return_num: return_byte & 0x07, num_returns: (return_byte >> 3) & 0x07,
        classification, scan_angle_rank, red: 0, green: 0, blue: 0, nir: 0, hag_m: 0.0,
        gps_time_s: 0.0, submagic: M64_LIDAR_SUBMAGIC, flags: 0, reserved: 0, crc32c: 0, _pad: [],
    };
    pt.seal_crc();
    Ok(pt)
}

/// Parses a 32-byte COPC VLR octree hierarchy entry (`HotZeroHeap`).
#[inline]
pub fn parse_copc_hierarchy_entry(bytes: &[u8; 32]) -> CopcOctreeVoxelKey {
    CopcOctreeVoxelKey {
        depth: r_i32(&bytes[0..4]), x: r_i32(&bytes[4..8]), y: r_i32(&bytes[8..12]),
        z: r_i32(&bytes[12..16]), point_offset: r_u64(&bytes[16..24]), point_count: r_u32(&bytes[24..28]),
    }
}

/// Batch ingestion of LAS point records enforcing an explicit caller byte budget (`ColdBounded`).
pub fn ingest_las_point_batch(
    bytes: &[u8],
    hdr: &LasHeaderSummary,
    byte_budget: usize,
    out_points: &mut [M64LidarPoint],
) -> Result<usize, LidarError> {
    let point_offset = hdr.offset_to_point_data as usize;
    let rec_len = hdr.point_data_record_length as usize;
    if rec_len == 0 { return Err(LidarError::UnsupportedFormat(hdr.point_data_format)); }
    if bytes.len() < point_offset { return Err(LidarError::TruncatedData); }
    let available_records = (bytes.len() - point_offset) / rec_len;
    let count = (hdr.point_count as usize).min(available_records).min(out_points.len());
    let requested = count * core::mem::size_of::<M64LidarPoint>();
    if requested > byte_budget {
        return Err(LidarError::ByteBudgetExceeded { requested, budget: byte_budget });
    }

    match hdr.point_data_format {
        0 => {
            for i in 0..count {
                let start = point_offset + i * rec_len;
                out_points[i] = decode_las_point_format0(&bytes[start..start + rec_len], hdr)?;
            }
            Ok(count)
        }
        fmt => Err(LidarError::UnsupportedFormat(fmt)),
    }
}

/// Cloth Simulation Filter (CSF, Zhang et al. 2016) Ground & Canopy Classifier (`HotZeroHeap`).
pub fn classify_ground_and_canopy_csf(
    points: &mut [M64LidarPoint],
    cloth_z: &mut [f32],
    min_z_grid: &mut [f32],
    grid_w: usize,
    grid_h: usize,
    min_x: f64,
    min_y: f64,
    cell_size: f64,
    rigidness_iters: usize,
    ground_threshold_m: f32,
) -> Result<usize, LidarError> {
    let total_cells = grid_w.checked_mul(grid_h).ok_or(LidarError::BufferTooSmall)?;
    if cloth_z.len() < total_cells || min_z_grid.len() < total_cells || cell_size <= 0.0 {
        return Err(LidarError::BufferTooSmall);
    }
    if points.is_empty() { return Ok(0); }

    for val in min_z_grid[..total_cells].iter_mut() { *val = f32::INFINITY; }
    let mut global_min_z = f32::INFINITY;
    for p in points.iter() {
        let pz = p.z as f32;
        if pz < global_min_z { global_min_z = pz; }
        if p.x >= min_x && p.y >= min_y {
            let gx = ((p.x - min_x) / cell_size) as usize;
            let gy = ((p.y - min_y) / cell_size) as usize;
            if gx < grid_w && gy < grid_h {
                let idx = gy * grid_w + gx;
                if pz < min_z_grid[idx] { min_z_grid[idx] = pz; }
            }
        }
    }

    for val in min_z_grid[..total_cells].iter_mut() {
        if val.is_infinite() { *val = global_min_z; }
    }
    cloth_z[..total_cells].copy_from_slice(&min_z_grid[..total_cells]);

    for _ in 0..rigidness_iters {
        for y in 0..grid_h {
            for x in 0..grid_w {
                let idx = y * grid_w + x;
                let mut sum = 0.0f32;
                let mut count = 0usize;
                if x > 0 { sum += cloth_z[idx - 1]; count += 1; }
                if x + 1 < grid_w { sum += cloth_z[idx + 1]; count += 1; }
                if y > 0 { sum += cloth_z[idx - grid_w]; count += 1; }
                if y + 1 < grid_h { sum += cloth_z[idx + grid_w]; count += 1; }
                if count > 0 {
                    let avg = sum / (count as f32);
                    let relaxed = cloth_z[idx] * 0.3 + avg * 0.7;
                    cloth_z[idx] = relaxed.min(min_z_grid[idx]);
                }
            }
        }
    }

    let mut ground_count = 0usize;
    for p in points.iter_mut() {
        let cloth_elev = if p.x >= min_x && p.y >= min_y {
            let gx = ((p.x - min_x) / cell_size) as usize;
            let gy = ((p.y - min_y) / cell_size) as usize;
            if gx < grid_w && gy < grid_h { cloth_z[gy * grid_w + gx] } else { global_min_z }
        } else {
            global_min_z
        };

        let hag = (p.z as f32) - cloth_elev;
        p.hag_m = hag;
        if hag <= ground_threshold_m {
            p.classification = AsprsClass::Ground as u8;
            ground_count += 1;
        } else if hag <= 0.5 {
            p.classification = AsprsClass::LowVegetation as u8;
        } else if hag <= 2.0 {
            p.classification = AsprsClass::MediumVegetation as u8;
        } else {
            p.classification = AsprsClass::HighVegetation as u8;
        }
        p.seal_crc();
    }
    Ok(ground_count)
}

/// Rasterizes LiDAR points into DTM (bare-earth), DSM (canopy-top), and CHM (`HotZeroHeap`).
pub fn rasterize_dtm_dsm_chm(
    points: &[M64LidarPoint],
    width: usize,
    height: usize,
    min_x: f64,
    min_y: f64,
    cell_size: f64,
    out_dtm: &mut [f32],
    out_dsm: &mut [f32],
    out_chm: &mut [f32],
) -> Result<(), LidarError> {
    let total_cells = width.checked_mul(height).ok_or(LidarError::BufferTooSmall)?;
    if out_dtm.len() < total_cells || out_dsm.len() < total_cells || out_chm.len() < total_cells || cell_size <= 0.0 {
        return Err(LidarError::BufferTooSmall);
    }

    for i in 0..total_cells { out_dsm[i] = f32::NEG_INFINITY; out_dtm[i] = f32::INFINITY; }

    for p in points {
        if p.x < min_x || p.y < min_y { continue; }
        let gx = ((p.x - min_x) / cell_size) as usize;
        let gy = ((p.y - min_y) / cell_size) as usize;
        if gx < width && gy < height {
            let idx = gy * width + gx;
            let z = p.z as f32;
            if z > out_dsm[idx] { out_dsm[idx] = z; }
            if p.classification == AsprsClass::Ground as u8 || p.classification == 2 {
                if z < out_dtm[idx] { out_dtm[idx] = z; }
            }
        }
    }

    for i in 0..total_cells {
        if out_dsm[i].is_infinite() { out_dsm[i] = 0.0; }
        if out_dtm[i].is_infinite() { out_dtm[i] = out_dsm[i]; }
        out_chm[i] = (out_dsm[i] - out_dtm[i]).max(0.0);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_m64_lidar_point_size_align_and_crc_roundtrip() {
        assert_eq!(core::mem::size_of::<M64LidarPoint>(), 64);
        assert_eq!(core::mem::align_of::<M64LidarPoint>(), 64);

        let mut pt = M64LidarPoint {
            x: 612345.67, y: 5965432.10, z: 2228.0, intensity: 1200, return_num: 1, num_returns: 2,
            classification: AsprsClass::Ground as u8, scan_angle_rank: -3, red: 180, green: 200,
            blue: 150, nir: 3500, hag_m: 0.0, gps_time_s: 123456789.0,
            submagic: M64_LIDAR_SUBMAGIC, flags: 1, reserved: 0, crc32c: 0, _pad: [],
        };
        pt.seal_crc();
        assert!(pt.verify_crc());

        let bytes = pt.to_bytes();
        assert_eq!(bytes.len(), 64);

        let decoded = M64LidarPoint::from_bytes_checked(&bytes).expect("valid point");
        assert_eq!(decoded.x, pt.x);
        assert_eq!(decoded.y, pt.y);
        assert_eq!(decoded.z, pt.z);
        assert_eq!(decoded.intensity, 1200);
        assert_eq!(decoded.classification, AsprsClass::Ground as u8);

        let mut corrupted = bytes;
        corrupted[10] ^= 0xFF;
        assert_eq!(M64LidarPoint::from_bytes_checked(&corrupted), Err(LidarError::CrcMismatch));

        let mut bad_magic = bytes;
        bad_magic[56] = 0xAA;
        assert_eq!(M64LidarPoint::from_bytes_checked(&bad_magic), Err(LidarError::InvalidMagic));
    }

    #[test]
    fn test_parse_las_header_and_format0_decoding() {
        let mut buf = [0u8; 395];
        buf[0..4].copy_from_slice(b"LASF");
        buf[24] = 1; buf[25] = 4;
        buf[96..100].copy_from_slice(&375u32.to_le_bytes());
        buf[104] = 0; buf[105..107].copy_from_slice(&20u16.to_le_bytes());
        buf[107..111].copy_from_slice(&1u32.to_le_bytes());
        buf[131..139].copy_from_slice(&0.01f64.to_le_bytes()); buf[139..147].copy_from_slice(&0.01f64.to_le_bytes()); buf[147..155].copy_from_slice(&0.01f64.to_le_bytes());
        buf[155..163].copy_from_slice(&600000.0f64.to_le_bytes()); buf[163..171].copy_from_slice(&5900000.0f64.to_le_bytes()); buf[171..179].copy_from_slice(&2000.0f64.to_le_bytes());
        buf[179..187].copy_from_slice(&601000.0f64.to_le_bytes()); buf[187..195].copy_from_slice(&600000.0f64.to_le_bytes());
        buf[195..203].copy_from_slice(&5901000.0f64.to_le_bytes()); buf[203..211].copy_from_slice(&5900000.0f64.to_le_bytes());
        buf[211..219].copy_from_slice(&2228.0f64.to_le_bytes()); buf[219..227].copy_from_slice(&2000.0f64.to_le_bytes());
        buf[267..275].copy_from_slice(&1u64.to_le_bytes());

        buf[375..379].copy_from_slice(&10000i32.to_le_bytes());
        buf[379..383].copy_from_slice(&20000i32.to_le_bytes());
        buf[383..387].copy_from_slice(&22800i32.to_le_bytes());
        buf[387..389].copy_from_slice(&1500u16.to_le_bytes());
        buf[389] = 1 | (2 << 3); buf[390] = AsprsClass::Ground as u8; buf[391] = (-5i8) as u8;

        let hdr = parse_las_header(&buf).expect("header parses");
        assert_eq!(hdr.version_major, 1); assert_eq!(hdr.version_minor, 4); assert_eq!(hdr.point_count, 1);
        assert_eq!(hdr.scale, [0.01, 0.01, 0.01]);
        assert_eq!(hdr.min_bounds, [600000.0, 5900000.0, 2000.0]);
        assert_eq!(hdr.max_bounds, [601000.0, 5901000.0, 2228.0]);

        let pt = decode_las_point_format0(&buf[375..395], &hdr).expect("point decodes");
        assert!((pt.x - 600100.0).abs() < 1e-4);
        assert!((pt.y - 5900200.0).abs() < 1e-4);
        assert!((pt.z - 2228.0).abs() < 1e-4);
        assert_eq!(pt.intensity, 1500); assert_eq!(pt.return_num, 1); assert_eq!(pt.num_returns, 2);
        assert_eq!(pt.classification, AsprsClass::Ground as u8);
        assert_eq!(pt.scan_angle_rank, -5);
        assert!(pt.verify_crc());

        let mut out = [M64LidarPoint::default(); 2];
        assert_eq!(
            ingest_las_point_batch(&buf, &hdr, 32, &mut out),
            Err(LidarError::ByteBudgetExceeded { requested: 64, budget: 32 })
        );
        assert_eq!(ingest_las_point_batch(&buf, &hdr, 1024, &mut out).expect("ingests"), 1);

        let mut copc_bytes = [0u8; 32];
        copc_bytes[0..4].copy_from_slice(&3i32.to_le_bytes()); copc_bytes[4..8].copy_from_slice(&2i32.to_le_bytes());
        copc_bytes[8..12].copy_from_slice(&4i32.to_le_bytes()); copc_bytes[12..16].copy_from_slice(&1i32.to_le_bytes());
        copc_bytes[16..24].copy_from_slice(&65536u64.to_le_bytes()); copc_bytes[24..28].copy_from_slice(&128u32.to_le_bytes());
        let key = parse_copc_hierarchy_entry(&copc_bytes);
        assert_eq!(key.depth, 3); assert_eq!(key.x, 2); assert_eq!(key.y, 4); assert_eq!(key.z, 1);
        assert_eq!(key.point_offset, 65536); assert_eq!(key.point_count, 128);
    }

    #[test]
    fn test_csf_ground_canopy_and_rasterization() {
        let mut points = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                let mut pt = M64LidarPoint::default();
                pt.x = x as f64 + 0.5; pt.y = y as f64 + 0.5; pt.z = 1800.0;
                pt.classification = AsprsClass::Unclassified as u8;
                points.push(pt);
            }
        }
        for &(cx, cy) in &[(1, 1), (1, 2), (2, 1), (2, 2)] {
            let mut pt = M64LidarPoint::default();
            pt.x = cx as f64 + 0.5; pt.y = cy as f64 + 0.5; pt.z = 1808.0;
            pt.classification = AsprsClass::Unclassified as u8;
            points.push(pt);
        }
        for &(sx, sy) in &[(0, 1), (3, 2)] {
            let mut pt = M64LidarPoint::default();
            pt.x = sx as f64 + 0.5; pt.y = sy as f64 + 0.5; pt.z = 1800.4;
            pt.classification = AsprsClass::Unclassified as u8;
            points.push(pt);
        }

        let mut cloth_z = [0.0f32; 16];
        let mut min_z_grid = [0.0f32; 16];

        let ground_count = classify_ground_and_canopy_csf(
            &mut points, &mut cloth_z, &mut min_z_grid, 4, 4, 0.0, 0.0, 1.0, 10, 0.25,
        ).expect("CSF succeeds");

        assert_eq!(ground_count, 16);

        for p in &points[16..20] {
            assert_eq!(p.classification, AsprsClass::HighVegetation as u8);
            assert!((p.hag_m - 8.0).abs() < 1e-3);
        }
        for p in &points[20..22] {
            assert_eq!(p.classification, AsprsClass::LowVegetation as u8);
            assert!((p.hag_m - 0.4).abs() < 1e-3);
        }

        let mut dtm = [0.0f32; 16];
        let mut dsm = [0.0f32; 16];
        let mut chm = [0.0f32; 16];

        rasterize_dtm_dsm_chm(&points, 4, 4, 0.0, 0.0, 1.0, &mut dtm, &mut dsm, &mut chm)
            .expect("rasterization succeeds");

        assert!((dsm[5] - 1808.0).abs() < 1e-3);
        assert!((dtm[5] - 1800.0).abs() < 1e-3);
        assert!((chm[5] - 8.0).abs() < 1e-3);
        assert!((chm[0] - 0.0).abs() < 1e-3);
    }
}
