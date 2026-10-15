//! P20.9 — Canopy radiative transfer (PlantGL `ZBufferEngine` + `plantgl.light`):
//! orthographic organ-ID/depth rasterization, periodic boundary wrapping,
//! Standard Overcast Sky (SOC) dome and solar-track direction generation, and
//! a per-organ PAR / STAR / LAI integrator.
//!
//! References:
//! - Sinoquet, Rivet & Godin (1997), *Annals of Botany* — 3-D light
//!   interception of tree crowns (STAR = silhouette / total area ratio).
//! - Den Dubbleden (1989) SOC luminance convention used by PlantGL.
//!
//! All public entry points are Tier-1 `HotZeroHeap`: they write only into
//! caller-owned slices and allocate nothing.

use super::primitives::Point3;

/// Sentinel organ ID marking a background (sky) pixel — no geometry hit.
pub const NO_ORGAN_ID: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanopyRadiationError {
    InvalidResolution,
    InvalidDirection,
    IndexOutOfBounds { index: usize },
    OutputTooSmall { required: usize },
}

/// A directional light source: `direction` points **from the sun/sky cell
/// towards the canopy**; `weight` is PAR irradiance (or a normalized dome
/// weight) carried by that direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalSource {
    pub direction: Point3,
    pub weight: f64,
}

/// Summary receipt for [`integrate_canopy_radiation`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanopyRadiationReceipt {
    /// Σ over sources and organs of intercepted PAR flux.
    pub total_intercepted_par: f64,
    /// Irradiance-weighted mean sky gap fraction (1 − silhouette coverage).
    pub mean_gap_fraction: f64,
    /// Irradiance-weighted mean STAR over all canopy area
    /// (Σ_dirs w·silhouette_area / Σ_dirs w·total_leaf_area).
    pub mean_star: f64,
    /// `total_leaf_area / ground_area` when `ground_area > 0`, else NaN.
    pub leaf_area_index: f64,
    /// Gap-fraction inversion estimate −ln(gap)/G with G = 0.5 (spherical).
    pub estimated_effective_lai: f64,
}

#[inline]
fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
#[inline]
fn dot(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}
#[inline]
fn cross(a: Point3, b: Point3) -> Point3 {
    Point3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}
#[inline]
fn normalize(a: Point3) -> Option<Point3> {
    let n = dot(a, a).sqrt();
    (n > 1e-12 && n.is_finite()).then(|| Point3::new(a.x / n, a.y / n, a.z / n))
}
#[inline]
fn tri_area_3d(a: Point3, b: Point3, c: Point3) -> f64 {
    0.5 * dot(cross(sub(b, a), sub(c, a)), cross(sub(b, a), sub(c, a))).sqrt()
}

/// Accumulate true 3-D triangle surface area per organ ID into
/// `out_areas[organ_id]`. Returns the number of distinct organ IDs touched.
///
/// Tier-1 `HotZeroHeap`.
pub fn organ_surface_areas(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    organ_ids: &[u32],
    out_areas: &mut [f64],
) -> Result<usize, CanopyRadiationError> {
    if triangles.len() != organ_ids.len() {
        return Err(CanopyRadiationError::InvalidResolution);
    }
    out_areas.fill(0.0);
    let mut touched = [false; 256];
    let mut n_touched = 0usize;
    for (ti, tri) in triangles.iter().enumerate() {
        let oid = organ_ids[ti];
        let idx = oid as usize;
        if idx >= out_areas.len() || idx >= touched.len() {
            return Err(CanopyRadiationError::OutputTooSmall { required: idx + 1 });
        }
        let [i0, i1, i2] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
        if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
            return Err(CanopyRadiationError::IndexOutOfBounds { index: ti });
        }
        let a = tri_area_3d(vertices[i0], vertices[i1], vertices[i2]);
        if !touched[idx] && a > 0.0 {
            touched[idx] = true;
            n_touched += 1;
        }
        out_areas[idx] += a;
    }
    Ok(n_touched)
}

/// Generate the 16-direction Standard Overcast Sky dome
/// (zenith + ring of 5 at ≈63.4° + ring of 10 at ≈26.6° elevation), each
/// direction weighted by the SOC luminance law L(ε) ∝ (1 + 2 sin ε)/3 · sin ε
/// and normalized so Σ weight = `total_par`.
///
/// Tier-1 `HotZeroHeap`.
pub fn soc_sky_dome_16(
    total_par: f64,
    out_sources: &mut [DirectionalSource],
) -> Result<usize, CanopyRadiationError> {
    if out_sources.len() < 16 || !(total_par >= 0.0 && total_par.is_finite()) {
        return Err(CanopyRadiationError::OutputTooSmall { required: 16 });
    }
    let mut dirs = [Point3::default(); 16];
    let mut lum = [0.0f64; 16];
    dirs[0] = Point3::new(0.0, 0.0, -1.0);
    lum[0] = (1.0 + 2.0) / 3.0;
    for (ring, count, elev_deg) in [(1usize, 5usize, 63.43494882f64), (6, 10, 26.56505118)] {
        let e = elev_deg.to_radians();
        let (ce, se) = (e.cos(), e.sin());
        for i in 0..count {
            let az = core::f64::consts::TAU * (i as f64) / (count as f64);
            dirs[ring + i] = Point3::new(-ce * az.cos(), -ce * az.sin(), -se);
            lum[ring + i] = se * (1.0 + 2.0 * se) / 3.0;
        }
    }
    let w_sum: f64 = lum.iter().sum();
    for i in 0..16 {
        out_sources[i] = DirectionalSource {
            direction: dirs[i],
            weight: total_par * lum[i] / w_sum.max(1e-15),
        };
    }
    Ok(16)
}

/// Generate an equal-duration solar track for `latitude_deg` on
/// `day_of_year` (1–365), sampling `samples_per_day` instants of the 24 h and
/// keeping only above-horizon ones. Weights are normalized so Σ = `total_par`.
///
/// Declination δ = 23.45°·sin(2π(284 + N)/365); elevation from the standard
/// hour-angle formula; direction points sun → canopy.
///
/// Tier-1 `HotZeroHeap`.
pub fn solar_track_sources(
    latitude_deg: f64,
    day_of_year: u32,
    samples_per_day: usize,
    total_par: f64,
    out_sources: &mut [DirectionalSource],
) -> Result<usize, CanopyRadiationError> {
    if samples_per_day == 0
        || out_sources.len() < samples_per_day
        || !(latitude_deg.abs() <= 90.0 && total_par >= 0.0 && total_par.is_finite())
        || !(1..=365).contains(&day_of_year)
    {
        return Err(CanopyRadiationError::OutputTooSmall {
            required: samples_per_day,
        });
    }
    let lat = latitude_deg.to_radians();
    let decl = (23.45f64).to_radians()
        * (core::f64::consts::TAU * (284 + day_of_year as i32) as f64 / 365.0).sin();

    let mut n_out = 0usize;
    for s in 0..samples_per_day {
        let hour = 24.0 * (s as f64 + 0.5) / (samples_per_day as f64);
        let hour_angle = (15.0f64).to_radians() * (hour - 12.0);
        let sin_e = lat.sin() * decl.sin() + lat.cos() * decl.cos() * hour_angle.cos();
        if sin_e <= 0.0 {
            continue;
        }
        let elev = sin_e.clamp(-1.0, 1.0).asin();
        // Azimuth from north, clockwise; atan2 formulation avoids acos domain issues.
        let cos_az = ((decl.sin() - lat.sin() * sin_e) / (lat.cos() * elev.cos()).max(1e-12))
            .clamp(-1.0, 1.0);
        let az = if hour <= 12.0 {
            cos_az.acos()
        } else {
            core::f64::consts::TAU - cos_az.acos()
        };
        // Sun position direction; ray direction is sun → canopy (−position).
        let sun_dir = Point3::new(elev.cos() * az.sin(), elev.cos() * az.cos(), elev.sin());
        out_sources[n_out] = DirectionalSource {
            direction: Point3::new(-sun_dir.x, -sun_dir.y, -sun_dir.z),
            weight: 1.0,
        };
        n_out += 1;
    }
    if n_out == 0 {
        return Err(CanopyRadiationError::InvalidDirection);
    }
    let per = total_par / (n_out as f64);
    for s in &mut out_sources[..n_out] {
        s.weight = per;
    }
    Ok(n_out)
}

/// Rasterize a triangle mesh into an orthographic ID/Z-buffer pair along
/// `ray_direction` (source → scene). Each buffer has `width·height` cells;
/// `out_depth` is the distance coordinate along the ray (smaller wins),
/// `out_ids` holds the triangle's organ ID or [`NO_ORGAN_ID`].
///
/// Returns `(pixel_area, covered_pixel_count)`.
///
/// Tier-1 `HotZeroHeap`.
pub fn rasterize_orthographic_id_buffer(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    organ_ids: &[u32],
    ray_direction: Point3,
    width: usize,
    height: usize,
    out_depth: &mut [f64],
    out_ids: &mut [u32],
) -> Result<(f64, usize), CanopyRadiationError> {
    let n_pix = width.saturating_mul(height);
    if width < 2 || height < 2 || triangles.len() != organ_ids.len() {
        return Err(CanopyRadiationError::InvalidResolution);
    }
    if out_depth.len() < n_pix || out_ids.len() < n_pix {
        return Err(CanopyRadiationError::OutputTooSmall { required: n_pix });
    }
    let dir = normalize(ray_direction).ok_or(CanopyRadiationError::InvalidDirection)?;
    let ref_up = if dir.z.abs() < 0.9 {
        Point3::new(0.0, 0.0, 1.0)
    } else {
        Point3::new(1.0, 0.0, 0.0)
    };
    let u_axis = normalize(cross(dir, ref_up)).ok_or(CanopyRadiationError::InvalidDirection)?;
    let v_axis = cross(u_axis, dir);

    out_depth[..n_pix].fill(f64::INFINITY);
    out_ids[..n_pix].fill(NO_ORGAN_ID);
    if vertices.is_empty() || triangles.is_empty() {
        return Ok((0.0, 0));
    }

    let (mut u_min, mut u_max) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut v_min, mut v_max) = (f64::INFINITY, f64::NEG_INFINITY);
    for &p in vertices {
        let (u, v) = (dot(p, u_axis), dot(p, v_axis));
        u_min = u_min.min(u);
        u_max = u_max.max(u);
        v_min = v_min.min(v);
        v_max = v_max.max(v);
    }
    let du = (u_max - u_min).max(1e-9) / (width as f64);
    let dv = (v_max - v_min).max(1e-9) / (height as f64);
    let pixel_area = du * dv;

    for (ti, tri) in triangles.iter().enumerate() {
        let [i0, i1, i2] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
        if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
            return Err(CanopyRadiationError::IndexOutOfBounds { index: ti });
        }
        let (p0, p1, p2) = (vertices[i0], vertices[i1], vertices[i2]);
        let (u0, v0, z0) = (dot(p0, u_axis), dot(p0, v_axis), dot(p0, dir));
        let (u1, v1, z1) = (dot(p1, u_axis), dot(p1, v_axis), dot(p1, dir));
        let (u2, v2, z2) = (dot(p2, u_axis), dot(p2, v_axis), dot(p2, dir));
        let det = (v1 - v2) * (u0 - u2) + (u2 - u1) * (v0 - v2);
        if det.abs() <= 1e-14 {
            continue;
        }
        let inv = 1.0 / det;
        let x0 = (((u0.min(u1).min(u2) - u_min) / du).floor().max(0.0) as usize).min(width - 1);
        let x1 = (((u0.max(u1).max(u2) - u_min) / du).ceil().max(0.0) as usize).min(width - 1);
        let y0 = (((v0.min(v1).min(v2) - v_min) / dv).floor().max(0.0) as usize).min(height - 1);
        let y1 = (((v0.max(v1).max(v2) - v_min) / dv).ceil().max(0.0) as usize).min(height - 1);
        let oid = organ_ids[ti];
        for py in y0..=y1 {
            let v = v_min + (py as f64 + 0.5) * dv;
            for px in x0..=x1 {
                let u = u_min + (px as f64 + 0.5) * du;
                let w0 = ((v1 - v2) * (u - u2) + (u2 - u1) * (v - v2)) * inv;
                let w1 = ((v2 - v0) * (u - u2) + (u0 - u2) * (v - v2)) * inv;
                let w2 = 1.0 - w0 - w1;
                if w0 >= -1e-9 && w1 >= -1e-9 && w2 >= -1e-9 {
                    let z = w0 * z0 + w1 * z1 + w2 * z2;
                    let idx = py * width + px;
                    if z < out_depth[idx] {
                        out_depth[idx] = z;
                        out_ids[idx] = oid;
                    }
                }
            }
        }
    }
    let covered = out_ids[..n_pix]
        .iter()
        .filter(|&&i| i != NO_ORGAN_ID)
        .count();
    Ok((pixel_area, covered))
}

/// Periodic-wrap the ID/depth buffers by `(dx_px, dy_px)` with a ray-depth
/// offset `dz_depth`, z-compositing the wrapped copy over existing content
/// (PlantGL `periodizeBuffer` tile replication).
///
/// `depth_scratch`/`id_scratch` must each hold `width·height` cells; the
/// original buffers are snapshotted into scratch first so wrap-around writes
/// never alias the source pixels.
///
/// Tier-1 `HotZeroHeap`.
pub fn periodize_id_depth_buffer(
    width: usize,
    height: usize,
    dx_px: isize,
    dy_px: isize,
    dz_depth: f64,
    depth_buf: &mut [f64],
    id_buf: &mut [u32],
    depth_scratch: &mut [f64],
    id_scratch: &mut [u32],
) -> Result<(), CanopyRadiationError> {
    let n_pix = width.saturating_mul(height);
    if width == 0
        || height == 0
        || depth_buf.len() < n_pix
        || id_buf.len() < n_pix
        || depth_scratch.len() < n_pix
        || id_scratch.len() < n_pix
    {
        return Err(CanopyRadiationError::InvalidResolution);
    }
    depth_scratch[..n_pix].copy_from_slice(&depth_buf[..n_pix]);
    id_scratch[..n_pix].copy_from_slice(&id_buf[..n_pix]);
    let (w, h) = (width as isize, height as isize);
    for y in 0..h {
        let sy = (y - dy_px).rem_euclid(h) as usize;
        for x in 0..w {
            let sx = (x - dx_px).rem_euclid(w) as usize;
            let src = sy * width + sx;
            let dst = (y as usize) * width + (x as usize);
            let z = depth_scratch[src] + dz_depth;
            if id_scratch[src] != NO_ORGAN_ID && z < depth_buf[dst] {
                depth_buf[dst] = z;
                id_buf[dst] = id_scratch[src];
            }
        }
    }
    Ok(())
}

/// Integrate canopy light interception over `sources` (SOC dome, solar track,
/// or a mix). For each source direction the mesh is rasterized into
/// `depth_scratch`/`id_scratch`; silhouette area per organ goes through
/// `silhouette_scratch`, PAR flux accumulates into `out_organ_par`, and
/// irradiance-weighted STAR into `out_organ_star` (normalized by Σ weights).
///
/// `organ_areas` are true 3-D areas (see [`organ_surface_areas`]); STAR for
/// organ i is silhouette_area / organ_areas[i]. `ground_area > 0.0` enables
/// the absolute LAI field on the receipt.
///
/// Tier-1 `HotZeroHeap`.
#[allow(clippy::too_many_arguments)]
pub fn integrate_canopy_radiation(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    organ_ids: &[u32],
    sources: &[DirectionalSource],
    width: usize,
    height: usize,
    depth_scratch: &mut [f64],
    id_scratch: &mut [u32],
    silhouette_scratch: &mut [f64],
    organ_areas: &[f64],
    ground_area: f64,
    out_organ_par: &mut [f64],
    out_organ_star: &mut [f64],
) -> Result<CanopyRadiationReceipt, CanopyRadiationError> {
    let n_pix = width.saturating_mul(height);
    if sources.is_empty()
        || depth_scratch.len() < n_pix
        || id_scratch.len() < n_pix
        || silhouette_scratch.len() < organ_areas.len()
        || out_organ_par.len() < organ_areas.len()
        || out_organ_star.len() < organ_areas.len()
    {
        return Err(CanopyRadiationError::InvalidResolution);
    }
    let n_org = organ_areas.len();
    out_organ_par.fill(0.0);
    out_organ_star.fill(0.0);

    let (mut total_par, mut w_gap, mut w_star, mut w_sum) = (0.0, 0.0, 0.0, 0.0);
    let total_leaf_area: f64 = organ_areas.iter().sum();

    for src in sources {
        if src.weight <= 0.0 {
            continue;
        }
        let (pix_area, covered) = rasterize_orthographic_id_buffer(
            vertices,
            triangles,
            organ_ids,
            src.direction,
            width,
            height,
            depth_scratch,
            id_scratch,
        )?;
        w_gap += (1.0 - (covered as f64) / (n_pix as f64)) * src.weight;
        w_sum += src.weight;

        silhouette_scratch[..n_org].fill(0.0);
        for &oid in &id_scratch[..n_pix] {
            if oid != NO_ORGAN_ID {
                let idx = oid as usize;
                if idx >= n_org {
                    return Err(CanopyRadiationError::OutputTooSmall { required: idx + 1 });
                }
                silhouette_scratch[idx] += pix_area;
            }
        }
        for i in 0..n_org {
            let sil = silhouette_scratch[i];
            out_organ_par[i] += sil * src.weight;
            total_par += sil * src.weight;
            if organ_areas[i] > 0.0 {
                out_organ_star[i] += (sil / organ_areas[i]) * src.weight;
                w_star += (sil / total_leaf_area.max(1e-12)) * src.weight;
            }
        }
    }

    let mean_gap = if w_sum > 0.0 {
        (w_gap / w_sum).clamp(0.0, 1.0)
    } else {
        1.0
    };
    for i in 0..n_org {
        if w_sum > 0.0 {
            out_organ_star[i] /= w_sum;
        }
    }

    Ok(CanopyRadiationReceipt {
        total_intercepted_par: total_par,
        mean_gap_fraction: mean_gap,
        mean_star: if w_sum > 0.0 { w_star / w_sum } else { 0.0 },
        leaf_area_index: if ground_area > 0.0 {
            total_leaf_area / ground_area
        } else {
            f64::NAN
        },
        estimated_effective_lai: -mean_gap.max(1e-6).ln() / 0.5,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_leaf_scene() -> ([Point3; 8], [[u32; 3]; 4], [u32; 4]) {
        let verts = [
            Point3::new(-1.0, -1.0, 2.0),
            Point3::new(1.0, -1.0, 2.0),
            Point3::new(1.0, 1.0, 2.0),
            Point3::new(-1.0, 1.0, 2.0),
            Point3::new(-1.0, -1.0, 0.0),
            Point3::new(1.0, -1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(-1.0, 1.0, 0.0),
        ];
        let tris = [[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]];
        (verts, tris, [0, 0, 1, 1])
    }

    #[test]
    fn zenith_occlusion_par_star_and_periodize() {
        let (verts, tris, oids) = two_leaf_scene();
        let mut areas = [0.0f64; 2];
        assert_eq!(
            organ_surface_areas(&verts, &tris, &oids, &mut areas).unwrap(),
            2
        );
        assert!((areas[0] - 4.0).abs() < 1e-9);

        // ID/depth buffer: upper leaf (z=2, organ 0) must win over lower.
        let mut depth = [0.0f64; 64 * 64];
        let mut ids = [0u32; 64 * 64];
        let (pix_area, covered) = rasterize_orthographic_id_buffer(
            &verts,
            &tris,
            &oids,
            Point3::new(0.0, 0.0, -1.0),
            64,
            64,
            &mut depth,
            &mut ids,
        )
        .unwrap();
        assert!(covered > 0 && pix_area > 0.0);
        assert!(
            ids[..64 * 64].iter().all(|&i| i != 1),
            "lower leaf must be fully occluded"
        );

        // Periodic wrap: shifting by half a tile duplicates coverage into gaps.
        let mut ds = [0.0f64; 64 * 64];
        let mut is_ = [0u32; 64 * 64];
        periodize_id_depth_buffer(64, 64, 32, 32, 0.0, &mut depth, &mut ids, &mut ds, &mut is_)
            .unwrap();
        let covered2 = ids.iter().filter(|&&i| i != NO_ORGAN_ID).count();
        assert!(
            covered2 >= covered,
            "periodized buffer must not lose coverage"
        );

        // Solar track produces daylight-only sources.
        let mut track = [DirectionalSource {
            direction: Point3::default(),
            weight: 0.0,
        }; 96];
        let n = solar_track_sources(45.0, 172, 96, 1200.0, &mut track).unwrap();
        assert!(n > 0 && n < 96);
        let sum_w: f64 = track[..n].iter().map(|s| s.weight).sum();
        assert!((sum_w - 1200.0).abs() < 1e-9);
        assert!(track[..n].iter().all(|s| s.direction.z < 0.0));

        // Full integration: upper organ intercepts zenith PAR, lower gets none;
        // STAR for the exposed organ equals projected area / true area = 1.
        let zenith = [DirectionalSource {
            direction: Point3::new(0.0, 0.0, -1.0),
            weight: 100.0,
        }];
        let mut sil = [0.0f64; 2];
        let mut par = [0.0f64; 2];
        let mut star = [0.0f64; 2];
        let rec = integrate_canopy_radiation(
            &verts, &tris, &oids, &zenith, 64, 64, &mut depth, &mut ids, &mut sil, &areas, 4.0,
            &mut par, &mut star,
        )
        .unwrap();
        assert!(par[0] > 0.0 && par[1] == 0.0);
        assert!(star[0] > 0.9 && star[0] <= 1.0, "star[0]={}", star[0]);
        assert!((rec.leaf_area_index - 2.0).abs() < 1e-9);
        assert!(rec.mean_gap_fraction >= 0.0 && rec.mean_gap_fraction < 1.0);

        // SOC dome: weights sum to the requested PAR.
        let mut sky = [DirectionalSource {
            direction: Point3::default(),
            weight: 0.0,
        }; 16];
        assert_eq!(soc_sky_dome_16(500.0, &mut sky).unwrap(), 16);
        let sw: f64 = sky.iter().map(|s| s.weight).sum();
        assert!((sw - 500.0).abs() < 1e-9);
    }
}
