// Copyright (c) EarthCraft contributors
// Adapted under MIT License for QualiaDB.
//! Geological landform primitives for procedural and geodetic terrain generation.
//!
//! Provides 14 morphological landform primitives surpassing traditional landscape
//! generators with tectonic, karst, and glacial structures.
//!
//! Strictly adheres to Tier-1 `HotZeroHeap` (zero allocations in hot sampling loops).

use std::f32::consts::PI;

/// The 14 distinct geological landform primitive kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandformKind {
    Mountain,
    Volcano,
    Caldera,
    Canyon,
    DuneSea,
    FaultBlock,
    Escarpment,
    ImpactCrater,
    Archipelago,
    RidgeLine,
    Plateau,
    TiltedStrata,
    KarstTower,
    GlacialCirque,
}

/// Parameters controlling landform placement, scale, orientation, and morphological details.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LandformParams {
    pub kind: LandformKind,
    pub center_x: f32,
    pub center_y: f32,
    pub radius: f32,
    pub height: f32,
    pub azimuth_rad: f32,
    pub sharpness: f32,
    pub roughness: f32,
    pub secondary_ratio: f32,
    pub seed: u64,
}

impl Default for LandformParams {
    fn default() -> Self {
        Self {
            kind: LandformKind::Mountain,
            center_x: 0.0, center_y: 0.0, radius: 1000.0, height: 500.0,
            azimuth_rad: 0.0, sharpness: 1.0, roughness: 0.2, secondary_ratio: 0.5, seed: 42,
        }
    }
}

#[inline]
fn hash2(ix: i32, iy: i32, seed: u64) -> f32 {
    let mut h = seed ^ ((ix as u64).wrapping_mul(0x517c_c1b7_2722_0a95));
    h ^= (iy as u64).wrapping_mul(0x6c62_272e_07bb_0142);
    h ^= h >> 32;
    h = h.wrapping_mul(0x2545_f491_4f6c_dd1d);
    h ^= h >> 32;
    ((h & 0x00ff_ffff) as f32) / 16777215.0
}

#[inline]
fn val_noise(x: f32, y: f32, seed: u64) -> f32 {
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let wx = fx * fx * (3.0 - 2.0 * fx);
    let wy = fy * fy * (3.0 - 2.0 * fy);
    let (n00, n10) = (hash2(ix, iy, seed), hash2(ix + 1, iy, seed));
    let (n01, n11) = (hash2(ix, iy + 1, seed), hash2(ix + 1, iy + 1, seed));
    let nx0 = n00 + wx * (n10 - n00);
    let nx1 = n01 + wx * (n11 - n01);
    nx0 + wy * (nx1 - nx0)
}

#[inline]
fn fbm2(x: f32, y: f32, seed: u64, octaves: u32) -> f32 {
    let (mut amp, mut freq, mut total, mut max_amp) = (0.5f32, 1.0f32, 0.0f32, 0.0f32);
    for o in 0..octaves {
        total += amp * (val_noise(x * freq, y * freq, seed.wrapping_add(o as u64 * 31)) * 2.0 - 1.0);
        max_amp += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    if max_amp > 0.0 { total / max_amp } else { 0.0 }
}

#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Evaluates the elevation contribution of a geological landform at given world coordinates.
///
/// Fully adheres to Tier-1 `HotZeroHeap` (zero allocations, constant stack footprint).
pub fn sample_landform(x: f32, y: f32, params: &LandformParams) -> f32 {
    let (dx, dy) = (x - params.center_x, y - params.center_y);
    let dist = (dx * dx + dy * dy).sqrt();
    let rad = params.radius.max(1e-4);
    let r = dist / rad;
    if r >= 1.0 {
        return 0.0;
    }

    let (sin_a, cos_a) = (params.azimuth_rad.sin(), params.azimuth_rad.cos());
    let (u, v) = (dx * cos_a + dy * sin_a, -dx * sin_a + dy * cos_a);

    match params.kind {
        LandformKind::Mountain => {
            let phi = dy.atan2(dx);
            let cone = (1.0 - r).powf(1.0 + params.sharpness.clamp(0.1, 4.0));
            let rib_angle = 5.0 * phi + fbm2(x * 0.005, y * 0.005, params.seed, 2) * 1.5;
            let ribs = rib_angle.cos() * 0.5 + 0.5;
            let rib_factor = 1.0 + 0.35 * ribs * (r * (1.0 - r) * 4.0);
            let rough = params.roughness * fbm2(x / rad * 6.0, y / rad * 6.0, params.seed, 3) * cone;
            params.height * (cone * rib_factor + rough).max(0.0)
        }
        LandformKind::Volcano => {
            let flank = (1.0 - r).powf(2.0 + params.sharpness.clamp(0.1, 4.0));
            let vent_r = 0.12;
            let vent_factor = if r < vent_r { 0.65 + 0.35 * (r / vent_r).powi(2) } else { 1.0 };
            let rough = params.roughness * 0.1 * fbm2(x / rad * 8.0, y / rad * 8.0, params.seed, 2) * flank;
            params.height * (flank * vent_factor + rough).max(0.0)
        }
        LandformKind::Caldera => {
            let rim_r = 0.45;
            if r >= rim_r {
                let flank_t = (1.0 - r) / (1.0 - rim_r);
                let flank = flank_t.clamp(0.0, 1.0).powf(1.5 + params.sharpness * 0.5);
                let rough = params.roughness * 0.1 * fbm2(x / rad * 6.0, y / rad * 6.0, params.seed, 2);
                params.height * (flank + rough * flank).max(0.0)
            } else {
                let inner_t = r / rim_r;
                let wall = smoothstep(0.7, 1.0, inner_t);
                let floor = 0.3 + 0.7 * wall;
                let dome_r = inner_t / 0.4;
                let dome = if dome_r < 1.0 { params.secondary_ratio.clamp(0.0, 1.0) * 0.3 * (1.0 - dome_r * dome_r) } else { 0.0 };
                params.height * (floor + dome)
            }
        }
        LandformKind::Canyon => {
            let meander = (u / (rad * 0.35)).sin() * (rad * 0.12);
            let v_dist = (v - meander).abs();
            let gorge_w = rad * 0.35;
            let t = (v_dist / gorge_w).clamp(0.0, 1.0);
            let step = (t * 4.0).floor();
            let frac = (t * 4.0) - step;
            let stepped = (step + smoothstep(0.35, 0.85, frac)) / 4.0;
            let profile = if t < 0.2 { 0.0 } else { stepped };
            let env = (1.0 - r * r).powi(2);
            params.height * profile * env
        }
        LandformKind::DuneSea => {
            let barchan = 0.15 * (v / (rad * 0.2)).sin() * rad;
            let wavelength = rad * 0.2;
            let phase = (((u - barchan) / wavelength).rem_euclid(1.0)).clamp(0.0, 1.0);
            let dune_h = if phase < 0.75 {
                (phase / 0.75).powf(1.3)
            } else {
                ((1.0 - phase) / 0.25).powf(3.0)
            };
            let skew = 0.1 * (phase * 2.0 * PI).sin();
            let env = (1.0 - r * r).powi(2);
            params.height * (dune_h + skew).clamp(0.0, 1.0) * env
        }
        LandformKind::FaultBlock => {
            let env = (1.0 - r * r).powi(2);
            let scarp_w = rad * 0.08;
            let v_norm = v / rad;
            let elev = if v >= 0.0 {
                let tilt = (1.0 - v_norm * 0.5).clamp(0.4, 1.0);
                0.85 * tilt
            } else {
                let roll = (-v_norm * 0.4).clamp(0.0, 0.3);
                0.15 + roll
            };
            let scarp_factor = smoothstep(-scarp_w / rad, scarp_w / rad, v_norm);
            let blended = 0.15 + (elev - 0.15) * scarp_factor;
            params.height * blended * env
        }
        LandformKind::Escarpment => {
            let env = (1.0 - r * r).powi(2);
            let v_norm = v / rad;
            let profile = if v_norm < 0.0 {
                (1.0 + v_norm).clamp(0.0, 1.0).powf(1.0 + params.sharpness * 0.5)
            } else {
                let cliff_drop = (-v_norm / 0.15 * (2.0 + params.sharpness)).exp();
                let talus = 0.35 * (1.0 - v_norm).clamp(0.0, 1.0).powi(2);
                cliff_drop.max(talus)
            };
            params.height * profile * env
        }
        LandformKind::ImpactCrater => {
            let rim_r = 0.45;
            if r < rim_r {
                let t = r / rim_r;
                let floor = 0.15 + 0.75 * t * t;
                let peak_r = t / 0.3;
                let peak = if peak_r < 1.0 { params.secondary_ratio.clamp(0.0, 1.0) * 0.35 * (1.0 - peak_r * peak_r) } else { 0.0 };
                params.height * (floor + peak)
            } else {
                let ejecta_t = (1.0 - r) / (1.0 - rim_r);
                let ejecta = 0.90 * ejecta_t.clamp(0.0, 1.0).powf(2.0 + params.sharpness * 0.5);
                params.height * ejecta
            }
        }
        LandformKind::Archipelago => {
            let arc_v = v - 0.2 * (u / rad).powi(2) * rad;
            let mut max_island = 0.0f32;
            for k in 0..5 {
                let frac = (k as f32 - 2.0) / 2.5;
                let (iu, iv) = (frac * rad * 0.7, 0.0f32);
                let (du, dv) = (u - iu, arc_v - iv);
                let idist = (du * du + dv * dv).sqrt();
                let irad = rad * (0.12 + 0.03 * ((k * 17) % 5) as f32);
                if idist < irad {
                    let it = idist / irad;
                    let cone = (1.0 - it).powf(1.5 + params.sharpness * 0.5);
                    let shelf = smoothstep(0.0, 0.2, 1.0 - it);
                    let island = cone * shelf;
                    if island > max_island {
                        max_island = island;
                    }
                }
            }
            let env = (1.0 - r * r).powi(2);
            params.height * max_island * env
        }
        LandformKind::RidgeLine => {
            let meander = (u / (rad * 0.3)).sin() * (rad * 0.12);
            let v_crest = (v - meander).abs();
            let flank = (-v_crest / (rad * 0.2) * (1.5 + params.sharpness)).exp();
            let cols = 0.8 + 0.2 * (u / (rad * 0.15)).cos();
            let env = (1.0 - r * r).powi(2);
            params.height * flank * cols * env
        }
        LandformKind::Plateau => {
            let phi = dy.atan2(dx);
            let canyons = 0.08 * (phi * 7.0 + params.seed as f32).cos();
            let rim_r = (0.55 + canyons).clamp(0.3, 0.8);
            let elev = if r < rim_r {
                1.0
            } else {
                let cliff_t = (r - rim_r) / (1.0 - rim_r);
                (1.0 - cliff_t).clamp(0.0, 1.0).powf(4.0 + params.sharpness * 2.0)
            };
            params.height * elev
        }
        LandformKind::TiltedStrata => {
            let bed_spacing = rad * 0.2;
            let bed_phase = ((v / bed_spacing).rem_euclid(1.0)).clamp(0.0, 1.0);
            let cuesta = if bed_phase < 0.7 {
                (bed_phase / 0.7).powf(1.0 + params.sharpness * 0.5)
            } else {
                ((1.0 - bed_phase) / 0.3).powf(2.0 + params.sharpness)
            };
            let diff_erosion = 0.85 + 0.15 * fbm2(u / rad * 4.0, 0.0, params.seed, 2);
            let env = (1.0 - r * r).powi(2);
            params.height * cuesta * diff_erosion * env
        }
        LandformKind::KarstTower => {
            let mut max_tower = 0.0f32;
            let offsets: [(f32, f32, f32); 7] = [
                (0.0, 0.0, 1.0), (0.35, 0.25, 0.85), (-0.30, 0.32, 0.90), (-0.25, -0.35, 0.80),
                (0.40, -0.22, 0.75), (0.15, 0.50, 0.70), (-0.45, -0.10, 0.65),
            ];
            for (ox, oy, h_scale) in offsets {
                let (tx, ty) = (ox * rad * 0.8, oy * rad * 0.8);
                let tdist = ((dx - tx).powi(2) + (dy - ty).powi(2)).sqrt();
                let tower_r = rad * 0.09;
                let tower = h_scale / (1.0 + (tdist / tower_r).powi(8));
                if tower > max_tower {
                    max_tower = tower;
                }
            }
            let env = (1.0 - r * r).powi(2);
            params.height * max_tower * env
        }
        LandformKind::GlacialCirque => {
            let env = (1.0 - r * r).powi(2);
            let headwall_r = ((u.min(0.0) * 1.4).powi(2) + v.powi(2)).sqrt() / rad;
            let headwall = smoothstep(0.2, 0.65, headwall_r).powf(1.5 + params.sharpness * 0.5);
            let tarn_dist = ((u + rad * 0.05).powi(2) + v.powi(2)).sqrt() / (rad * 0.3);
            let tarn = (-tarn_dist * tarn_dist * 2.0).exp();
            let lip_u = (u - rad * 0.35).abs() / (rad * 0.15);
            let lip_v = v.abs() / (rad * 0.4);
            let moraine_lip = if lip_u < 1.0 && lip_v < 1.0 { 0.35 * (1.0 - lip_u * lip_u) * (1.0 - lip_v * lip_v) } else { 0.0 };
            let valley_open = if u > rad * 0.4 { ((u - rad * 0.4) / (rad * 0.6)).clamp(0.0, 1.0) * 0.2 } else { 0.0 };
            let elev = (0.2 + 0.8 * headwall - 0.25 * tarn + moraine_lip - valley_open).clamp(0.0, 1.2);
            params.height * elev * env
        }
    }
}

/// Evaluates a geological landform over a 2D regular grid into a caller-supplied slice.
///
/// Fully adheres to Tier-1 `HotZeroHeap` (zero allocations, caller provides buffer).
pub fn generate_landform_grid_into(
    width: usize,
    height: usize,
    domain_min: [f32; 2],
    domain_max: [f32; 2],
    params: &LandformParams,
    out: &mut [f32],
) -> Result<(), &'static str> {
    if width == 0 || height == 0 {
        return Err("Grid dimensions must be non-zero");
    }
    let total_cells = width.checked_mul(height).ok_or("Grid dimensions overflow")?;
    if out.len() < total_cells {
        return Err("Output buffer too small for grid dimensions");
    }
    let dx = if width > 1 { (domain_max[0] - domain_min[0]) / (width as f32 - 1.0) } else { 0.0 };
    let dy = if height > 1 { (domain_max[1] - domain_min[1]) / (height as f32 - 1.0) } else { 0.0 };
    for j in 0..height {
        let y = domain_min[1] + j as f32 * dy;
        let row_offset = j * width;
        for i in 0..width {
            let x = domain_min[0] + i as f32 * dx;
            out[row_offset + i] = sample_landform(x, y, params);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_KINDS: [LandformKind; 14] = [
        LandformKind::Mountain, LandformKind::Volcano, LandformKind::Caldera, LandformKind::Canyon,
        LandformKind::DuneSea, LandformKind::FaultBlock, LandformKind::Escarpment, LandformKind::ImpactCrater,
        LandformKind::Archipelago, LandformKind::RidgeLine, LandformKind::Plateau, LandformKind::TiltedStrata,
        LandformKind::KarstTower, LandformKind::GlacialCirque,
    ];

    #[test]
    fn test_all_14_landforms_finite_and_far_field() {
        for kind in ALL_KINDS {
            let mut params = LandformParams::default();
            params.kind = kind;
            params.center_x = 100.0;
            params.center_y = -50.0;
            params.radius = 500.0;
            params.height = 300.0;

            let c = sample_landform(params.center_x, params.center_y, &params);
            assert!(c.is_finite(), "{:?} center was not finite", kind);

            let rim = sample_landform(params.center_x + params.radius * 0.45, params.center_y, &params);
            assert!(rim.is_finite(), "{:?} rim was not finite", kind);

            let far = sample_landform(params.center_x + params.radius * 2.0, params.center_y, &params);
            assert!(far.is_finite(), "{:?} far-field was not finite", kind);
            assert_eq!(far, 0.0, "{:?} far-field was not zero", kind);
        }
    }

    #[test]
    fn test_caldera_morphology_profile() {
        let mut params = LandformParams::default();
        params.kind = LandformKind::Caldera;
        params.radius = 1000.0;
        params.height = 800.0;
        params.secondary_ratio = 0.4;

        let center = sample_landform(0.0, 0.0, &params);
        let rim = sample_landform(params.radius * 0.45, 0.0, &params);
        assert!(rim > center, "Caldera rim ({}) should be higher than center depression ({})", rim, center);
    }

    #[test]
    fn test_impact_crater_morphology_profile() {
        let mut params = LandformParams::default();
        params.kind = LandformKind::ImpactCrater;
        params.radius = 1000.0;
        params.height = 500.0;
        params.secondary_ratio = 0.3;

        let floor = sample_landform(params.radius * 0.2, 0.0, &params);
        let rim = sample_landform(params.radius * 0.45, 0.0, &params);
        assert!(rim > floor, "Impact crater rim ({}) should be higher than cavity floor ({})", rim, floor);
    }

    #[test]
    fn test_fault_block_morphology_profile() {
        let mut params = LandformParams::default();
        params.kind = LandformKind::FaultBlock;
        params.radius = 1000.0;
        params.height = 400.0;
        params.azimuth_rad = 0.0;

        let footwall = sample_landform(0.0, params.radius * 0.2, &params);
        let hanging_wall = sample_landform(0.0, -params.radius * 0.2, &params);
        assert!(
            footwall > hanging_wall,
            "Fault block footwall ({}) should be higher than hanging wall ({})",
            footwall, hanging_wall
        );
    }

    #[test]
    fn test_grid_generation_16x16_stack_buffer() {
        let params = LandformParams::default();
        let mut grid = [0.0f32; 256];
        let res = generate_landform_grid_into(16, 16, [-500.0, -500.0], [500.0, 500.0], &params, &mut grid);
        assert!(res.is_ok());

        let mut has_positive = false;
        for &val in grid.iter() {
            assert!(val.is_finite());
            if val > 0.0 {
                has_positive = true;
            }
        }
        assert!(has_positive, "Grid should contain positive landform heights");
    }

    #[test]
    fn test_grid_generation_buffer_too_small() {
        let params = LandformParams::default();
        let mut small_buf = [0.0f32; 10];
        let res = generate_landform_grid_into(4, 4, [-10.0, -10.0], [10.0, 10.0], &params, &mut small_buf);
        assert!(res.is_err());
    }
}
