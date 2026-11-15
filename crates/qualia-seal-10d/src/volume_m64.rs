//! Official `.m64` matter volume from an OCC1 occupancy lattice (LRM3 anatomical frame).

use qualia_core_db::q42::m64_matter::{
    brick_voxel_linear_index, encode_hilbert3d_u64, M64BioTissueRecord, M64CellMode,
    M64ChunkDirectoryEntry, M64Header, M64HoloRecord, M64LocalMetricHeader, M64PayloadInterval,
    M64ReadLimits, M64Reader, COORD_FRAME_CARTESIAN_MICROMETER, M64_BRICK3D_RECORDS,
    M64_FLAG_3D_BRICK_16X16X16, M64_FLAG_LOCAL_METRIC_LRM3, M64_MODE_HOLO,
    M64_SRID_LOCAL_MICROMETER, TISSUE_CLASS_ADIPOSE, TISSUE_CLASS_CORTICAL_BONE,
    TISSUE_CLASS_EMPTY, TISSUE_CLASS_EPICARDIAL_FAT, TISSUE_CLASS_LUMEN, TISSUE_CLASS_MYOCARDIUM,
    TISSUE_CLASS_TRABECULAR_BONE,
};

use super::occupancy::{OccupancyGrid, MAT_EMPTY, MAT_EPICARDIAL_FAT, MAT_LUMEN, MAT_MYOCARDIUM};
use super::SealError;

const BRICK: u32 = 16;

pub fn encode_occupancy_m64(grid: &OccupancyGrid) -> Result<Vec<u8>, SealError> {
    let n = grid.resolution;
    let bricks = (n + BRICK - 1) / BRICK;
    let mut brick_xyz: Vec<(u32, u32, u32)> = Vec::new();
    for bz in 0..bricks {
        for by in 0..bricks {
            for bx in 0..bricks {
                if brick_occupied(grid, bx, by, bz) {
                    brick_xyz.push((bx, by, bz));
                }
            }
        }
    }
    if brick_xyz.is_empty() {
        return Err(SealError::Volume("occupancy has no occupied bricks".into()));
    }

    let chunk_count = 1 + brick_xyz.len();
    let dir_off = 192usize;
    let dir_bytes = chunk_count * 64;
    let organ_off = dir_off + dir_bytes;
    let organ_len = 64usize;
    let brick_bytes = M64_BRICK3D_RECORDS * 64;
    let total = organ_off + organ_len + brick_xyz.len() * brick_bytes;
    let mut buffer = vec![0u8; total];

    let mut lrm3 = anatomical_lrm3(grid);
    lrm3.update_crc();
    buffer[64..192].copy_from_slice(bytemuck::bytes_of(&lrm3));

    let organ = organ_holo();
    buffer[organ_off..organ_off + 64].copy_from_slice(bytemuck::bytes_of(&organ));

    let mut entries: Vec<M64ChunkDirectoryEntry> = Vec::with_capacity(chunk_count);
    entries.push(directory_entry(
        0,
        encode_hilbert3d_u64(0, 0, 0, 4),
        organ_off as u64,
        organ_len as u32,
        1,
    ));

    for (i, &(bx, by, bz)) in brick_xyz.iter().enumerate() {
        let start = organ_off + organ_len + i * brick_bytes;
        fill_holo_brick(grid, bx, by, bz, &mut buffer[start..start + brick_bytes]);
        entries.push(directory_entry(
            (i + 1) as u8,
            encode_hilbert3d_u64(bx, by, bz, 8),
            start as u64,
            brick_bytes as u32,
            M64_BRICK3D_RECORDS as u32,
        ));
    }

    for (i, entry) in entries.iter().enumerate() {
        let o = dir_off + i * 64;
        buffer[o..o + 64].copy_from_slice(bytemuck::bytes_of(entry));
    }

    let mut header = M64Header::default();
    header.total_file_size = total as u64;
    header.chunk_count = chunk_count as u64;
    header.chunk_table_offset = dir_off as u64;
    header.spatial_origin = [0.0, 0.0];
    header.elevation_datum = [0.0, 1.0];
    header.container_flags =
        (M64_FLAG_3D_BRICK_16X16X16 as u32) | (M64_FLAG_LOCAL_METRIC_LRM3 as u32);
    header.update_crc();
    buffer[..64].copy_from_slice(bytemuck::bytes_of(&header));

    let limits = M64ReadLimits {
        max_file_bytes: total + 1024,
        max_chunks: chunk_count + 4,
        max_records: (1 + brick_xyz.len() * M64_BRICK3D_RECORDS) as u64 + 16,
    };
    let mut scratch = vec![M64PayloadInterval::default(); chunk_count + 4];
    M64Reader::parse(&buffer, limits, &mut scratch)
        .map_err(|e| SealError::Volume(format!("M64 self-verification failed: {e}")))?;
    Ok(buffer)
}

/// Seal a human heart occupancy lattice using `.m64` Mode `0x05` (`M64BioTissueRecord`).
pub fn seal_human_heart_m64(grid: &OccupancyGrid) -> Result<Vec<u8>, SealError> {
    let mut organ = M64BioTissueRecord::default();
    organ.tissue_class_id = TISSUE_CLASS_MYOCARDIUM;
    organ.apparent_density_kg_m3 = 1060;
    organ.set_elastic_moduli_mpa(0.08, 0.04, 0.04, 0.02);
    organ.set_fiber_dir([0.707, 0.707, 0.0]);
    organ.muscle_hu_q8 = 45;
    organ.set_imat_fraction(0.03);
    organ.set_perfusion_metabolic(0.92);
    organ.update_crc();

    encode_bio_tissue_volume(grid, organ, |gx, gy, gz, mat| {
        let mut rec = M64BioTissueRecord::default();
        rec.header = ((gx as u64) & 0xFFFF)
            | (((gy as u64) & 0xFFFF) << 16)
            | (((gz as u64) & 0xFFFF) << 32);
        match mat {
            MAT_MYOCARDIUM => {
                rec.tissue_class_id = TISSUE_CLASS_MYOCARDIUM;
                rec.apparent_density_kg_m3 = 1060;
                rec.set_elastic_moduli_mpa(0.08, 0.04, 0.04, 0.02);
                rec.set_fiber_dir([0.707, 0.707, 0.0]);
                rec.muscle_hu_q8 = 45;
                rec.set_imat_fraction(0.03);
                rec.set_perfusion_metabolic(0.92);
            }
            MAT_LUMEN => {
                rec.tissue_class_id = TISSUE_CLASS_LUMEN;
                rec.apparent_density_kg_m3 = 1050;
                rec.set_perfusion_metabolic(1.0);
            }
            MAT_EPICARDIAL_FAT => {
                rec.tissue_class_id = TISSUE_CLASS_EPICARDIAL_FAT;
                rec.apparent_density_kg_m3 = 910;
                rec.set_elastic_moduli_mpa(0.01, 0.01, 0.01, 0.004);
                rec.muscle_hu_q8 = -90;
                rec.set_imat_fraction(0.95);
                rec.set_perfusion_metabolic(0.25);
            }
            _ => {
                rec.tissue_class_id = TISSUE_CLASS_EMPTY;
            }
        }
        rec.update_crc();
        rec
    })
}

/// Seal a vertebrate bone occupancy lattice using `.m64` Mode `0x05` (`M64BioTissueRecord`).
pub fn seal_vertebrate_bone_m64(
    grid: &OccupancyGrid,
    vbmd_mg_cm3: u16,
    bv_tv: f32,
) -> Result<Vec<u8>, SealError> {
    let clamped_bv_tv = bv_tv.clamp(0.0, 1.0);
    let rho_app = (1800.0 * clamped_bv_tv).round().clamp(50.0, 2200.0) as u16;
    let e1_mpa = (18_000.0 * clamped_bv_tv.powf(2.0)).clamp(10.0, 25_000.0);
    let e2_mpa = e1_mpa * 0.6;
    let g12_mpa = e1_mpa * 0.22;

    let mut organ = M64BioTissueRecord::default();
    organ.tissue_class_id = if clamped_bv_tv >= 0.7 {
        TISSUE_CLASS_CORTICAL_BONE
    } else {
        TISSUE_CLASS_TRABECULAR_BONE
    };
    organ.apparent_density_kg_m3 = rho_app;
    organ.mineral_vbmd_mg_cm3 = vbmd_mg_cm3;
    organ.set_porosity_bv_tv(clamped_bv_tv);
    organ.set_mineralization_ratio(0.68);
    organ.set_elastic_moduli_mpa(e1_mpa, e2_mpa, e2_mpa, g12_mpa);
    organ.set_fiber_dir([0.0, 1.0, 0.0]);
    organ.set_perfusion_metabolic(0.30);
    organ.update_crc();

    encode_bio_tissue_volume(grid, organ, |gx, gy, gz, mat| {
        let mut rec = M64BioTissueRecord::default();
        rec.header = ((gx as u64) & 0xFFFF)
            | (((gy as u64) & 0xFFFF) << 16)
            | (((gz as u64) & 0xFFFF) << 32);
        match mat {
            MAT_MYOCARDIUM => {
                rec.tissue_class_id = TISSUE_CLASS_CORTICAL_BONE;
                rec.apparent_density_kg_m3 = rho_app;
                rec.mineral_vbmd_mg_cm3 = vbmd_mg_cm3;
                rec.set_porosity_bv_tv(clamped_bv_tv);
                rec.set_mineralization_ratio(0.68);
                rec.set_elastic_moduli_mpa(e1_mpa, e2_mpa, e2_mpa, g12_mpa);
                rec.set_fiber_dir([0.0, 1.0, 0.0]);
                rec.set_perfusion_metabolic(0.20);
            }
            MAT_LUMEN => {
                rec.tissue_class_id = TISSUE_CLASS_TRABECULAR_BONE;
                rec.apparent_density_kg_m3 = (rho_app / 3).max(150);
                rec.mineral_vbmd_mg_cm3 = vbmd_mg_cm3 / 3;
                rec.set_porosity_bv_tv((clamped_bv_tv * 0.35).clamp(0.05, 0.6));
                rec.set_mineralization_ratio(0.60);
                rec.set_elastic_moduli_mpa(e1_mpa * 0.15, e2_mpa * 0.15, e2_mpa * 0.15, g12_mpa * 0.15);
                rec.set_fiber_dir([0.0, 1.0, 0.0]);
                rec.set_perfusion_metabolic(0.45);
            }
            MAT_EPICARDIAL_FAT => {
                rec.tissue_class_id = TISSUE_CLASS_ADIPOSE;
                rec.apparent_density_kg_m3 = 910;
                rec.muscle_hu_q8 = -90;
                rec.set_imat_fraction(0.95);
            }
            _ => {
                rec.tissue_class_id = TISSUE_CLASS_EMPTY;
            }
        }
        rec.update_crc();
        rec
    })
}

fn encode_bio_tissue_volume<F>(
    grid: &OccupancyGrid,
    organ: M64BioTissueRecord,
    mut make_voxel: F,
) -> Result<Vec<u8>, SealError>
where
    F: FnMut(u32, u32, u32, u8) -> M64BioTissueRecord,
{
    let n = grid.resolution;
    let bricks = (n + BRICK - 1) / BRICK;
    let mut brick_xyz: Vec<(u32, u32, u32)> = Vec::new();
    for bz in 0..bricks {
        for by in 0..bricks {
            for bx in 0..bricks {
                if brick_occupied(grid, bx, by, bz) {
                    brick_xyz.push((bx, by, bz));
                }
            }
        }
    }
    if brick_xyz.is_empty() {
        return Err(SealError::Volume("occupancy has no occupied bricks".into()));
    }

    let chunk_count = 1 + brick_xyz.len();
    let dir_off = 192usize;
    let dir_bytes = chunk_count * 64;
    let organ_off = dir_off + dir_bytes;
    let organ_len = 64usize;
    let brick_bytes = M64_BRICK3D_RECORDS * 64;
    let total = organ_off + organ_len + brick_xyz.len() * brick_bytes;
    let mut buffer = vec![0u8; total];

    let mut lrm3 = anatomical_lrm3(grid);
    lrm3.update_crc();
    buffer[64..192].copy_from_slice(bytemuck::bytes_of(&lrm3));
    buffer[organ_off..organ_off + 64].copy_from_slice(bytemuck::bytes_of(&organ));

    let mode = M64CellMode::BioTissue.as_u8();
    let mut entries: Vec<M64ChunkDirectoryEntry> = Vec::with_capacity(chunk_count);
    entries.push(directory_entry_with_mode(
        0,
        encode_hilbert3d_u64(0, 0, 0, 4),
        organ_off as u64,
        organ_len as u32,
        1,
        mode,
    ));

    for (i, &(bx, by, bz)) in brick_xyz.iter().enumerate() {
        let start = organ_off + organ_len + i * brick_bytes;
        let out = &mut buffer[start..start + brick_bytes];
        for lz in 0..16u8 {
            for ly in 0..16u8 {
                for lx in 0..16u8 {
                    let gx = bx * BRICK + lx as u32;
                    let gy = by * BRICK + ly as u32;
                    let gz = bz * BRICK + lz as u32;
                    let mat = if gx < grid.resolution && gy < grid.resolution && gz < grid.resolution {
                        grid.material_at(gx, gy, gz)
                    } else {
                        MAT_EMPTY
                    };
                    let rec = make_voxel(gx, gy, gz, mat);
                    let idx = brick_voxel_linear_index(lx, ly, lz);
                    let off = idx * 64;
                    out[off..off + 64].copy_from_slice(bytemuck::bytes_of(&rec));
                }
            }
        }
        entries.push(directory_entry_with_mode(
            (i + 1) as u8,
            encode_hilbert3d_u64(bx, by, bz, 8),
            start as u64,
            brick_bytes as u32,
            M64_BRICK3D_RECORDS as u32,
            mode,
        ));
    }

    for (i, entry) in entries.iter().enumerate() {
        let o = dir_off + i * 64;
        buffer[o..o + 64].copy_from_slice(bytemuck::bytes_of(entry));
    }

    let mut header = M64Header::default();
    header.total_file_size = total as u64;
    header.chunk_count = chunk_count as u64;
    header.chunk_table_offset = dir_off as u64;
    header.spatial_origin = [0.0, 0.0];
    header.elevation_datum = [0.0, 1.0];
    header.container_flags =
        (M64_FLAG_3D_BRICK_16X16X16 as u32) | (M64_FLAG_LOCAL_METRIC_LRM3 as u32);
    header.update_crc();
    buffer[..64].copy_from_slice(bytemuck::bytes_of(&header));

    let limits = M64ReadLimits {
        max_file_bytes: total + 1024,
        max_chunks: chunk_count + 4,
        max_records: (1 + brick_xyz.len() * M64_BRICK3D_RECORDS) as u64 + 16,
    };
    let mut scratch = vec![M64PayloadInterval::default(); chunk_count + 4];
    M64Reader::parse(&buffer, limits, &mut scratch)
        .map_err(|e| SealError::Volume(format!("M64 self-verification failed: {e}")))?;
    Ok(buffer)
}

fn anatomical_lrm3(grid: &OccupancyGrid) -> M64LocalMetricHeader {
    let size = grid.size();
    let n = grid.resolution.max(1) as f32;
    let voxel_cm = [size[0] / n, size[1] / n, size[2] / n];
    M64LocalMetricHeader {
        magic: *b"LRM3",
        version: 1,
        scale_tier: 2,
        coord_frame_kind: COORD_FRAME_CARTESIAN_MICROMETER,
        anchor_did_hash: 0,
        origin_offset_m: [
            (grid.min[0] as f64) * 0.01,
            (grid.min[1] as f64) * 0.01,
            (grid.min[2] as f64) * 0.01,
        ],
        orientation_quat: [0.0, 0.0, 0.0, 1.0],
        anisotropic_scale_um: [
            voxel_cm[0] * 10_000.0,
            voxel_cm[1] * 10_000.0,
            voxel_cm[2] * 10_000.0,
        ],
        bbox_min_um: [
            (grid.min[0] * 10_000.0) as i32,
            (grid.min[1] * 10_000.0) as i32,
            (grid.min[2] * 10_000.0) as i32,
        ],
        bbox_max_um: [
            (grid.max[0] * 10_000.0) as i32,
            (grid.max[1] * 10_000.0) as i32,
            (grid.max[2] * 10_000.0) as i32,
        ],
        reserved: [0u8; 32],
        header_crc32c: 0,
    }
}

fn organ_holo() -> M64HoloRecord {
    let mut rec = M64HoloRecord::default();
    rec.metric_variety = 0;
    rec.semantic_node_idx = MAT_MYOCARDIUM;
    rec.natural_freq_omega = [f32_to_f16(7.33), f32_to_f16(7.33), f32_to_f16(7.33)];
    rec.damping_decay = f32_to_f16(0.35);
    rec.mass_amplitude = f32_to_f16(1.0);
    rec.topological_spin = (0.26 * 32767.0) as i16;
    rec.phase_attractor_mask = 0x0003;
    rec.spectral_spd_base = [180, 40, 40, 90, 70, 50];
    rec.subsurface_roughness = [90, 60];
    rec
}

fn fill_holo_brick(grid: &OccupancyGrid, bx: u32, by: u32, bz: u32, out: &mut [u8]) {
    for lz in 0..16u8 {
        for ly in 0..16u8 {
            for lx in 0..16u8 {
                let gx = bx * BRICK + lx as u32;
                let gy = by * BRICK + ly as u32;
                let gz = bz * BRICK + lz as u32;
                let mat = if gx < grid.resolution && gy < grid.resolution && gz < grid.resolution {
                    grid.material_at(gx, gy, gz)
                } else {
                    MAT_EMPTY
                };
                let mut rec = M64HoloRecord::default();
                rec.local_origin = [
                    f32_to_f16(gx as f32),
                    f32_to_f16(gy as f32),
                    f32_to_f16(gz as f32),
                ];
                rec.metric_variety = 0;
                rec.semantic_node_idx = mat;
                rec.natural_freq_omega = match mat {
                    MAT_MYOCARDIUM => [f32_to_f16(7.33), f32_to_f16(7.33), f32_to_f16(7.33)],
                    MAT_LUMEN => [f32_to_f16(7.33), f32_to_f16(4.0), f32_to_f16(4.0)],
                    MAT_EPICARDIAL_FAT => [f32_to_f16(2.0), f32_to_f16(2.0), f32_to_f16(2.0)],
                    _ => [0, 0, 0],
                };
                rec.mass_amplitude = match mat {
                    MAT_MYOCARDIUM => f32_to_f16(1.2),
                    MAT_LUMEN => f32_to_f16(0.4),
                    MAT_EPICARDIAL_FAT => f32_to_f16(0.7),
                    _ => 0,
                };
                rec.spectral_spd_base = match mat {
                    MAT_MYOCARDIUM => [180, 40, 40, 90, 70, 50],
                    MAT_LUMEN => [140, 20, 20, 40, 20, 10],
                    MAT_EPICARDIAL_FAT => [210, 180, 90, 80, 50, 30],
                    _ => [0; 6],
                };
                let idx = brick_voxel_linear_index(lx, ly, lz);
                let off = idx * 64;
                out[off..off + 64].copy_from_slice(bytemuck::bytes_of(&rec));
            }
        }
    }
}

fn brick_occupied(grid: &OccupancyGrid, bx: u32, by: u32, bz: u32) -> bool {
    let x0 = bx * BRICK;
    let y0 = by * BRICK;
    let z0 = bz * BRICK;
    for z in z0..(z0 + BRICK).min(grid.resolution) {
        for y in y0..(y0 + BRICK).min(grid.resolution) {
            for x in x0..(x0 + BRICK).min(grid.resolution) {
                if grid.material_at(x, y, z) != MAT_EMPTY {
                    return true;
                }
            }
        }
    }
    false
}

fn directory_entry(
    uuid_tag: u8,
    hilbert: u64,
    offset: u64,
    length: u32,
    records: u32,
) -> M64ChunkDirectoryEntry {
    directory_entry_with_mode(uuid_tag, hilbert, offset, length, records, M64_MODE_HOLO)
}

fn directory_entry_with_mode(
    uuid_tag: u8,
    hilbert: u64,
    offset: u64,
    length: u32,
    records: u32,
    mode: u8,
) -> M64ChunkDirectoryEntry {
    let mut uuid = [0u8; 16];
    uuid[0] = uuid_tag;
    uuid[1] = 0x6d;
    uuid[2] = 0x36;
    uuid[3] = 0x34;
    let mut entry = M64ChunkDirectoryEntry {
        chunk_uuid: uuid,
        spatial_hilbert_index: hilbert,
        byte_offset: offset,
        byte_length: length,
        record_count: records,
        srid_geodetic_code: M64_SRID_LOCAL_MICROMETER,
        lut_bank_epoch_id: 1,
        chunk_record_mode: mode,
        compression_flags: 0,
        bao_root_hash_prefix: [0u8; 12],
        entry_crc32c: 0,
    };
    entry.update_crc();
    entry
}

fn f32_to_f16(f: f32) -> u16 {
    let f = if f.is_finite() { f } else { 0.0 };
    let bits = f.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exp = ((bits >> 23) & 0xff) as i32;
    let mant = bits & 0x7fffff;
    if exp == 0 {
        return sign;
    }
    if exp == 255 {
        return sign | 0x7c00;
    }
    let new_exp = exp - 127 + 15;
    if new_exp <= 0 {
        return sign;
    }
    if new_exp >= 31 {
        return sign | 0x7c00;
    }
    sign | ((new_exp as u16) << 10) | ((mant >> 13) as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::occupancy::synthetic_ball;
    use qualia_core_db::q42::m64_matter::{M64DecodedRecord, M64_MODE_BIO_TISSUE};

    #[test]
    fn seals_human_heart_and_vertebrate_bone_as_mode_0x05_bio_tissue() {
        let grid = synthetic_ball(16, 0.8);
        let heart = seal_human_heart_m64(&grid).expect("heart m64");
        let bone = seal_vertebrate_bone_m64(&grid, 1120, 0.85).expect("bone m64");

        for (bytes, expected_class) in [
            (&heart, TISSUE_CLASS_MYOCARDIUM),
            (&bone, TISSUE_CLASS_CORTICAL_BONE),
        ] {
            let limits = M64ReadLimits {
                max_file_bytes: bytes.len() + 1024,
                max_chunks: 16,
                max_records: 8192,
            };
            let mut scratch = [M64PayloadInterval::default(); 16];
            let reader = M64Reader::parse(bytes, limits, &mut scratch).expect("parse bio m64");
            let chunk = reader.chunk(0).expect("organ chunk");
            assert_eq!(chunk.directory_entry().chunk_record_mode, M64_MODE_BIO_TISSUE);
            match chunk.decode_record(0).expect("decode").expect("record 0") {
                M64DecodedRecord::BioTissue(rec) => {
                    assert_eq!(rec.tissue_class_id, expected_class);
                    assert!(rec.validate().is_ok());
                }
                other => panic!("expected BioTissue, got {other:?}"),
            }
        }
    }
}
