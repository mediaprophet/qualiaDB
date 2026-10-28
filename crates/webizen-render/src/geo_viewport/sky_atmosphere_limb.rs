// Copyright (c) 2026 QualiaDB Contributors.
// Adapted from EarthCraft-main v0.2.0, nerthus-main v2.3.1, and RustGIS-main v0.4.0 under MIT License.
// Single-scattering atmospheric limb formulation incorporates Eric Bruneton's sky model (BSD-3-Clause).

//! Spherical atmospheric limb & aerial perspective, Gerstner waves, hypsometric tinting, and GPU picking.

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtmosphereParams {
    pub planet_radius_m: f32,
    pub atmosphere_top_m: f32,
    pub rayleigh_scale_h_m: f32,
    pub mie_scale_h_m: f32,
    pub beta_rayleigh: [f32; 3],
    pub mie_g: f32,
    pub beta_mie: [f32; 3],
    pub sun_intensity: f32,
    pub sun_dir: [f32; 3],
    pub ozone_peak_h_m: f32,
}

#[inline]
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn norm3(a: [f32; 3]) -> f32 {
    dot3(a, a).sqrt()
}

#[inline]
fn normalize3(a: [f32; 3]) -> [f32; 3] {
    let l = norm3(a);
    if l > 1e-9 { [a[0] / l, a[1] / l, a[2] / l] } else { [0.0, 1.0, 0.0] }
}

#[inline]
fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Evaluates single-scattering Rayleigh & Mie sky limb radiance and aerial perspective transmittance.
pub fn evaluate_sky_limb_and_aerial_perspective(
    params: &AtmosphereParams,
    cam_alt_m: f32,
    view_dir: [f32; 3],
    max_dist_m: f32,
    steps: u32,
) -> ([f32; 3], [f32; 3]) {
    let r_p = params.planet_radius_m;
    let r_top = r_p + params.atmosphere_top_m;
    let p0 = [0.0, r_p + cam_alt_m.max(0.0), 0.0];
    let v = normalize3(view_dir);
    let s = normalize3(params.sun_dir);

    let b = dot3(p0, v);
    let c_top = dot3(p0, p0) - r_top * r_top;
    let d_top = b * b - c_top;
    if d_top < 0.0 {
        return ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    }
    let sqrt_d_top = d_top.sqrt();
    let t_top2 = -b + sqrt_d_top;
    if t_top2 <= 0.0 {
        return ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    }
    let t_start = (-b - sqrt_d_top).max(0.0);
    let mut t_end = t_top2;

    let c_p = dot3(p0, p0) - r_p * r_p;
    let d_p = b * b - c_p;
    if d_p >= 0.0 {
        let t_p1 = -b - d_p.sqrt();
        if t_p1 > 0.0 && t_p1 < t_end {
            t_end = t_p1;
        }
    }
    t_end = t_end.min(t_start + max_dist_m.max(0.0));
    if t_end <= t_start + 1e-3 {
        return ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    }

    let n_steps = steps.max(1);
    let ds = (t_end - t_start) / (n_steps as f32);
    let mu = dot3(v, s);
    let p_rayleigh = (3.0 / (16.0 * core::f32::consts::PI)) * (1.0 + mu * mu);
    let g = params.mie_g.clamp(-0.99, 0.99);
    let mie_denom = (1.0 + g * g - 2.0 * g * mu).max(1e-4);
    let p_mie = (3.0 * (1.0 - g * g) / (8.0 * core::f32::consts::PI * (2.0 + g * g)))
        * ((1.0 + mu * mu) / (mie_denom.sqrt() * mie_denom));

    let mut tau_view = [0.0f32; 3];
    let mut in_scatter = [0.0f32; 3];

    for i in 0..n_steps {
        let t = t_start + (i as f32 + 0.5) * ds;
        let p = [p0[0] + t * v[0], p0[1] + t * v[1], p0[2] + t * v[2]];
        let r = norm3(p);
        let h = (r - r_p).max(0.0);
        if h > params.atmosphere_top_m {
            continue;
        }

        let rho_r = (-h / params.rayleigh_scale_h_m.max(1.0)).exp();
        let rho_m = (-h / params.mie_scale_h_m.max(1.0)).exp();

        let p_dot_s = dot3(p, s);
        let in_shadow = p_dot_s < 0.0 && (r * r - p_dot_s * p_dot_s) < (r_p * r_p);
        let mut t_sun = [0.0f32; 3];
        if !in_shadow {
            let c_sun = r * r - r_top * r_top;
            let d_sun = (p_dot_s * p_dot_s - c_sun).max(0.0);
            let t_sun_top = -p_dot_s + d_sun.sqrt();
            let ds_sun = t_sun_top / 3.0;
            let mut opt_r_sun = 0.0f32;
            let mut opt_m_sun = 0.0f32;
            for j in 0..3 {
                let tj = (j as f32 + 0.5) * ds_sun;
                let pj = [p[0] + tj * s[0], p[1] + tj * s[1], p[2] + tj * s[2]];
                let hj = (norm3(pj) - r_p).max(0.0);
                opt_r_sun += (-hj / params.rayleigh_scale_h_m.max(1.0)).exp() * ds_sun;
                opt_m_sun += (-hj / params.mie_scale_h_m.max(1.0)).exp() * ds_sun;
            }
            for c in 0..3 {
                let tau_s = opt_r_sun * params.beta_rayleigh[c] + opt_m_sun * params.beta_mie[c] * 1.11;
                t_sun[c] = (-tau_s).exp();
            }
        }

        for c in 0..3 {
            let ext_c = rho_r * params.beta_rayleigh[c] + rho_m * params.beta_mie[c] * 1.11;
            let scat_c = rho_r * params.beta_rayleigh[c] * p_rayleigh + rho_m * params.beta_mie[c] * p_mie;
            let t_view_c = (-tau_view[c]).exp();
            in_scatter[c] += scat_c * params.sun_intensity * t_sun[c] * t_view_c * ds;
            tau_view[c] += ext_c * ds;
        }
    }

    let transmittance = [(-tau_view[0]).exp(), (-tau_view[1]).exp(), (-tau_view[2]).exp()];
    (in_scatter, transmittance)
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterSurfaceSample {
    pub displacement_xyz: [f32; 3],
    pub normal_xyz: [f32; 3],
    pub shore_foam_weight: f32,
    pub fresnel_reflectance: f32,
}

/// Evaluates 4-octave Gerstner trochoidal water displacement, normal, Schlick Fresnel, and shore foam.
pub fn evaluate_water_wave_and_foam(
    pos_xy: [f32; 2],
    time_sec: f32,
    water_depth_m: f32,
    wind_dir: [f32; 2],
    wave_amp_m: f32,
    view_cos_theta: f32,
) -> WaterSurfaceSample {
    let wind_len = (wind_dir[0] * wind_dir[0] + wind_dir[1] * wind_dir[1]).sqrt();
    let w = if wind_len > 1e-6 { [wind_dir[0] / wind_len, wind_dir[1] / wind_len] } else { [1.0, 0.0] };

    let angles = [0.0f32, 0.32, -0.45, 0.75];
    let wl_scales = [1.0f32, 0.48, 0.24, 0.12];
    let amp_scales = [0.52f32, 0.26, 0.14, 0.08];
    let base_wl = (wave_amp_m * 24.0 + 8.0).max(2.0);

    let mut disp = [0.0f32; 3];
    let mut n_accum = [0.0f32, 0.0f32, 1.0f32];

    for i in 0..4 {
        let (sin_a, cos_a) = angles[i].sin_cos();
        let di = [cos_a * w[0] - sin_a * w[1], sin_a * w[0] + cos_a * w[1]];
        let lambda = base_wl * wl_scales[i];
        let k = 2.0 * core::f32::consts::PI / lambda;
        let amp = wave_amp_m * amp_scales[i];
        let depth_arg = (k * water_depth_m.max(0.05)).clamp(0.001, 10.0);
        let omega = (9.81 * k * depth_arg.tanh()).sqrt();
        let phase = k * (di[0] * pos_xy[0] + di[1] * pos_xy[1]) - omega * time_sec;
        let (sin_p, cos_p) = phase.sin_cos();

        let q = (0.55 / (k * amp * 4.0).max(1e-4)).min(0.25);
        disp[0] += q * amp * di[0] * cos_p;
        disp[1] += q * amp * di[1] * cos_p;
        disp[2] += amp * sin_p;

        let wa = k * amp;
        n_accum[0] -= di[0] * wa * cos_p;
        n_accum[1] -= di[1] * wa * cos_p;
        n_accum[2] -= q * wa * sin_p;
    }

    let n_len = (n_accum[0] * n_accum[0] + n_accum[1] * n_accum[1] + n_accum[2] * n_accum[2]).sqrt();
    let normal_xyz = if n_len > 1e-6 {
        [n_accum[0] / n_len, n_accum[1] / n_len, n_accum[2] / n_len]
    } else {
        [0.0, 0.0, 1.0]
    };

    let r0 = 0.020f32;
    let cos_t = view_cos_theta.clamp(0.0, 1.0);
    let om_cos = 1.0 - cos_t;
    let fresnel_reflectance = r0 + (1.0 - r0) * om_cos.powi(5);

    let shallow_break = (wave_amp_m * 2.5).max(0.2);
    let shallow_weight = ((shallow_break - water_depth_m) / shallow_break).clamp(0.0, 1.0);
    let crest_weight = ((disp[2] + wave_amp_m * 0.2) / wave_amp_m.max(0.1)).clamp(0.0, 1.0);
    let shore_foam_weight = (shallow_weight * 0.75 + crest_weight * shallow_weight * 0.25).clamp(0.0, 1.0);

    WaterSurfaceSample { displacement_xyz: disp, normal_xyz, shore_foam_weight, fresnel_reflectance }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeoAtmosphereError {
    SliceLengthMismatch,
    InvalidElevationRange,
    InvalidContourStep,
    EmptyBuffer,
}

impl core::fmt::Display for GeoAtmosphereError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SliceLengthMismatch => write!(f, "Buffer slice lengths do not match"),
            Self::InvalidElevationRange => write!(f, "Max elevation must exceed min elevation"),
            Self::InvalidContourStep => write!(f, "Contour step must be positive and non-zero"),
            Self::EmptyBuffer => write!(f, "Input elevation buffer is empty"),
        }
    }
}

impl std::error::Error for GeoAtmosphereError {}

fn hypsometric_ramp(t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    if t < 0.15 {
        lerp3([38.0, 82.0, 128.0], [78.0, 142.0, 72.0], t / 0.15)
    } else if t < 0.38 {
        lerp3([78.0, 142.0, 72.0], [156.0, 182.0, 92.0], (t - 0.15) / 0.23)
    } else if t < 0.62 {
        lerp3([156.0, 182.0, 92.0], [204.0, 168.0, 98.0], (t - 0.38) / 0.24)
    } else if t < 0.82 {
        lerp3([204.0, 168.0, 98.0], [152.0, 144.0, 140.0], (t - 0.62) / 0.20)
    } else {
        lerp3([152.0, 144.0, 140.0], [248.0, 250.0, 252.0], (t - 0.82) / 0.18)
    }
}

/// Swiss-style hypsometric elevation tint and screen-space anti-aliased elevation contours.
pub fn shade_hypsometric_and_contours_into(
    dem: &[f32],
    slope_rad: &[f32],
    min_elev_m: f32,
    max_elev_m: f32,
    minor_contour_step_m: f32,
    major_contour_every: u32,
    out_rgba8: &mut [[u8; 4]],
) -> Result<(), GeoAtmosphereError> {
    if dem.is_empty() {
        return Err(GeoAtmosphereError::EmptyBuffer);
    }
    if dem.len() != slope_rad.len() || dem.len() != out_rgba8.len() {
        return Err(GeoAtmosphereError::SliceLengthMismatch);
    }
    if max_elev_m <= min_elev_m {
        return Err(GeoAtmosphereError::InvalidElevationRange);
    }
    if minor_contour_step_m <= 0.0 || major_contour_every == 0 {
        return Err(GeoAtmosphereError::InvalidContourStep);
    }

    let elev_range = max_elev_m - min_elev_m;
    let s_minor = minor_contour_step_m;
    let s_major = s_minor * (major_contour_every as f32);

    for i in 0..dem.len() {
        let z = dem[i];
        let t = ((z - min_elev_m) / elev_range).clamp(0.0, 1.0);
        let base_rgb = hypsometric_ramp(t);

        let slope = slope_rad[i].clamp(0.0, core::f32::consts::FRAC_PI_2);
        let shade = (1.0 - 0.35 * slope.sin()).clamp(0.55, 1.15);

        let mod_min = z.rem_euclid(s_minor);
        let d_min = mod_min.min(s_minor - mod_min);
        let mod_maj = z.rem_euclid(s_major);
        let d_maj = mod_maj.min(s_major - mod_maj);

        let w = (slope.tan() * 2.0 + 0.6).max(0.6);
        let u_maj = d_maj / (w * 1.2);
        let u_min = d_min / (w * 0.6);

        let darken = if u_maj < 1.0 {
            1.0 - 0.50 * (1.0 - u_maj * u_maj * (3.0 - 2.0 * u_maj))
        } else if u_min < 1.0 {
            1.0 - 0.28 * (1.0 - u_min * u_min * (3.0 - 2.0 * u_min))
        } else {
            1.0
        };

        out_rgba8[i] = [
            (base_rgb[0] * shade * darken).clamp(0.0, 255.0) as u8,
            (base_rgb[1] * shade * darken).clamp(0.0, 255.0) as u8,
            (base_rgb[2] * shade * darken).clamp(0.0, 255.0) as u8,
            255,
        ];
    }
    Ok(())
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickingHitRecord {
    pub feature_id: u64,
    pub layer_id: u32,
    pub depth_ndc: f32,
    pub world_xyz: [f64; 3],
}

/// Resolves the nearest non-zero packed entity `(layer_id: u32 << 32 | feature_low: u32)` within aperture.
pub fn resolve_gpu_picking_id_aperture(
    id_buffer: &[u64],
    depth_buffer: &[f32],
    width: usize,
    height: usize,
    click_x: usize,
    click_y: usize,
    radius_px: usize,
) -> Option<(u64, u32, usize, usize, f32)> {
    if width == 0 || height == 0 || click_x >= width || click_y >= height {
        return None;
    }
    let total_px = width.checked_mul(height)?;
    if id_buffer.len() < total_px || depth_buffer.len() < total_px {
        return None;
    }

    let r = radius_px as isize;
    let cx = click_x as isize;
    let cy = click_y as isize;
    let mut best: Option<(u64, u32, usize, usize, f32, f32)> = None;

    for dy in -r..=r {
        let py = cy + dy;
        if py < 0 || py >= height as isize {
            continue;
        }
        for dx in -r..=r {
            let px = cx + dx;
            if px < 0 || px >= width as isize {
                continue;
            }
            let idx = (py as usize) * width + (px as usize);
            let packed = id_buffer[idx];
            if packed == 0 {
                continue;
            }
            let depth = depth_buffer[idx];
            let dist_sq = (dx * dx + dy * dy) as f32;
            let layer_id = (packed >> 32) as u32;
            let feature_id = packed & 0xFFFF_FFFF;

            let replace = match best {
                None => true,
                Some((_, _, _, _, best_d, best_dist)) => {
                    depth < best_d - 1e-5 || ((depth - best_d).abs() <= 1e-5 && dist_sq < best_dist)
                }
            };
            if replace {
                best = Some((feature_id, layer_id, px as usize, py as usize, depth, dist_sq));
            }
        }
    }

    best.map(|(f_id, l_id, x, y, d, _)| (f_id, l_id, x, y, d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atmosphere_params_size() {
        assert_eq!(core::mem::size_of::<AtmosphereParams>(), 64);
    }

    #[test]
    fn test_sky_limb_and_aerial_perspective() {
        let params = AtmosphereParams {
            planet_radius_m: 6_371_000.0,
            atmosphere_top_m: 100_000.0,
            rayleigh_scale_h_m: 8_000.0,
            mie_scale_h_m: 1_200.0,
            beta_rayleigh: [5.8e-6, 13.5e-6, 33.1e-6],
            mie_g: 0.76,
            beta_mie: [21.0e-6, 21.0e-6, 21.0e-6],
            sun_intensity: 20.0,
            sun_dir: [0.0, 1.0, 0.0],
            ozone_peak_h_m: 25_000.0,
        };

        let (zenith_rad, zenith_trans) =
            evaluate_sky_limb_and_aerial_perspective(&params, 10.0, [0.0, 1.0, 0.0], 100_000.0, 16);
        assert!(zenith_rad[2] > zenith_rad[0], "Zenith scattered light must be blue dominant");
        assert!(zenith_trans[0] > 0.0 && zenith_trans[0] <= 1.0);

        let mut sunset_params = params;
        sunset_params.sun_dir = [1.0, 0.0, 0.0];
        let (_, sunset_trans) =
            evaluate_sky_limb_and_aerial_perspective(&sunset_params, 10.0, [1.0, 0.05, 0.0], 80_000.0, 16);
        assert!(sunset_trans[0] > sunset_trans[2], "Sunset horizon transmittance preserves red over blue");

        let (_, near_trans) =
            evaluate_sky_limb_and_aerial_perspective(&params, 10.0, [1.0, 0.0, 0.0], 1_000.0, 8);
        let (_, far_trans) =
            evaluate_sky_limb_and_aerial_perspective(&params, 10.0, [1.0, 0.0, 0.0], 50_000.0, 8);
        assert!(near_trans[0] > far_trans[0], "Aerial perspective transmittance decays over distance");
    }

    #[test]
    fn test_gerstner_waves_and_foam() {
        let sample = evaluate_water_wave_and_foam([15.0, 25.0], 2.0, 50.0, [1.0, 0.0], 1.8, 0.6);
        let n_len = norm3(sample.normal_xyz);
        assert!((n_len - 1.0).abs() < 1e-4, "Wave normal must be normalized");

        let grazing = evaluate_water_wave_and_foam([0.0, 0.0], 0.0, 50.0, [1.0, 0.0], 1.0, 0.0);
        assert!((grazing.fresnel_reflectance - 1.0).abs() < 1e-3, "Grazing Fresnel must approach 1.0");
        let normal_inc = evaluate_water_wave_and_foam([0.0, 0.0], 0.0, 50.0, [1.0, 0.0], 1.0, 1.0);
        assert!((normal_inc.fresnel_reflectance - 0.02).abs() < 1e-3, "Normal incidence must match R0");

        let shallow = evaluate_water_wave_and_foam([0.0, 0.0], 0.0, 0.5, [1.0, 0.0], 2.0, 0.5);
        let deep = evaluate_water_wave_and_foam([0.0, 0.0], 0.0, 100.0, [1.0, 0.0], 2.0, 0.5);
        assert!(shallow.shore_foam_weight > 0.4, "Shallow breaking water generates foam");
        assert!(deep.shore_foam_weight < 0.05, "Deep water has negligible shore foam");
    }

    #[test]
    fn test_hypsometric_contours_and_picking() {
        let dem = [50.0, 400.0, 1000.0, 1000.0];
        let slope = [0.05, 0.05, 0.05, 0.05];
        let mut out = [[0u8; 4]; 4];
        shade_hypsometric_and_contours_into(&dem, &slope, 0.0, 2000.0, 100.0, 5, &mut out).unwrap();
        assert_eq!(out[0][3], 255);
        assert!(out[2][0] > 0);

        let mut id_buf = [0u64; 25];
        let mut depth_buf = [1.0f32; 25];
        id_buf[2 * 5 + 2] = (12u64 << 32) | 101u64;
        depth_buf[2 * 5 + 2] = 0.6;
        id_buf[2 * 5 + 3] = (14u64 << 32) | 202u64;
        depth_buf[2 * 5 + 3] = 0.3;

        let hit = resolve_gpu_picking_id_aperture(&id_buf, &depth_buf, 5, 5, 2, 2, 1);
        assert_eq!(hit, Some((202, 14, 3, 2, 0.3)), "Must pick closest depth within aperture");
        let miss = resolve_gpu_picking_id_aperture(&id_buf, &depth_buf, 5, 5, 0, 0, 0);
        assert_eq!(miss, None);
    }
}
