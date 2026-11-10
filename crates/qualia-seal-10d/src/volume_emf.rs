//! Official `.emf` (`Q42EMF64`) from an OCC1 occupancy lattice.
//!
//! Reduced-order cardiac bioelectric field: myocardium carries an apex–base
//! depolarization E-field, blood is a conductive lumen, epicardial fat is
//! dielectric contrast. Quasi-static (ECG-band) cells; optical wavelength is
//! left at 0. Uses [`EmfContainerWriter`] — this is not a parallel format.

use qualia_core_db::q42::emf_container::{
    emf_brick_voxel_linear_index, EmfCellRecord64, EmfContainerWriter, EMF_BRICK_CELLS,
    EMF_FLAG_SPECTRAL_BANDS, SPEED_OF_LIGHT,
};
use qualia_core_db::q42::m64_matter::encode_hilbert3d_u64;

use super::occupancy::{OccupancyGrid, MAT_EMPTY, MAT_EPICARDIAL_FAT, MAT_LUMEN, MAT_MYOCARDIUM};
use super::SealError;

const BRICK: u32 = 8;
const MAX_BRICKS_AXIS: u32 = 4;

/// Peak myocardial depolarization field (V/m) along the long axis.
const MYO_E_PEAK: f32 = 50.0;
/// Blood-pool Faraday / volume-conductor residual (V/m).
const LUMEN_E: f32 = 8.0;

pub fn encode_occupancy_emf(grid: &OccupancyGrid) -> Result<Vec<u8>, SealError> {
    let n = grid.resolution.max(1);
    let bricks = ((n + BRICK - 1) / BRICK).clamp(1, MAX_BRICKS_AXIS);
    let size = grid.size();
    let origin_m = [
        (grid.min[0] as f64 * 0.01) as f32,
        (grid.min[1] as f64 * 0.01) as f32,
        (grid.min[2] as f64 * 0.01) as f32,
    ];
    let extent_m = [
        (size[0] as f64 * 0.01).max(1.0e-4) as f32,
        (size[1] as f64 * 0.01).max(1.0e-4) as f32,
        (size[2] as f64 * 0.01).max(1.0e-4) as f32,
    ];
    let voxels = bricks * BRICK;
    let voxel_m = (extent_m[0] / voxels as f32)
        .max(extent_m[1] / voxels as f32)
        .max(extent_m[2] / voxels as f32)
        .max(1.0e-5);

    let mut writer = EmfContainerWriter::new(origin_m, voxel_m, [bricks, bricks, bricks], 0.0);
    writer.flags |= EMF_FLAG_SPECTRAL_BANDS;

    let order = 3u32;
    for bz in 0..bricks {
        for by in 0..bricks {
            for bx in 0..bricks {
                let mut cells = [EmfCellRecord64::default(); EMF_BRICK_CELLS];
                for lz in 0..8u8 {
                    for ly in 0..8u8 {
                        for lx in 0..8u8 {
                            let gx = sample_axis(bx * BRICK + lx as u32, voxels, n);
                            let gy = sample_axis(by * BRICK + ly as u32, voxels, n);
                            let gz = sample_axis(bz * BRICK + lz as u32, voxels, n);
                            let mat = grid.material_at(gx, gy, gz);
                            let pos = grid.voxel_center(gx, gy, gz);
                            cells[emf_brick_voxel_linear_index(lx, ly, lz)] =
                                tissue_cell(mat, pos, grid.min, size);
                        }
                    }
                }
                let hilbert = encode_hilbert3d_u64(bx, by, bz, order);
                writer
                    .add_brick_chunk(hilbert, &cells, 0, 1, 0)
                    .map_err(|e| SealError::Emf(format!("add brick: {e}")))?;
            }
        }
    }

    let bytes = writer
        .to_bytes()
        .map_err(|e| SealError::Emf(format!("serialize: {e}")))?;
    if bytes.len() < 8 || &bytes[0..8] != b"Q42EMF64" {
        return Err(SealError::Emf("writer did not emit Q42EMF64 magic".into()));
    }
    Ok(bytes)
}

fn sample_axis(emf_i: u32, emf_n: u32, occ_n: u32) -> u32 {
    if emf_n <= 1 {
        return 0;
    }
    ((emf_i as u64) * (occ_n as u64 - 1) / (emf_n as u64 - 1)) as u32
}

fn tissue_cell(mat: u8, pos: [f32; 3], min: [f32; 3], size: [f32; 3]) -> EmfCellRecord64 {
    let ty = if size[1].abs() > 1.0e-6 {
        ((pos[1] - min[1]) / size[1]).clamp(0.0, 1.0)
    } else {
        0.5
    };
    let phase = ty * std::f32::consts::PI;
    let (e_y, n, k, sigma, alpha) = match mat {
        MAT_MYOCARDIUM => (MYO_E_PEAK * phase.sin(), 50.0, 0.35, 0.12, 0.72),
        MAT_LUMEN => (LUMEN_E, 70.0, 0.18, 0.06, 0.65),
        MAT_EPICARDIAL_FAT => (1.5, 6.0, 0.03, 0.01, 0.55),
        MAT_EMPTY | _ => (0.0, 1.0, 0.0, 0.0, 0.20),
    };
    let e_field = [0.0, e_y, 0.0];
    let b_mag = e_y / SPEED_OF_LIGHT;
    let mut cell = EmfCellRecord64 {
        e_field,
        b_field: [b_mag, 0.0, 0.0],
        poynting_flux: 0.0,
        wavelength_nm: 0.0,
        refractive_n: n,
        extinction_k: k,
        coherence_phase: phase,
        stokes_q: 0.0,
        stokes_u: 0.0,
        stokes_v: 0.0,
        spectral_sigma: sigma,
        confidence_alpha: alpha,
    };
    cell.poynting_flux = cell.poynting_flux_magnitude();
    cell
}
