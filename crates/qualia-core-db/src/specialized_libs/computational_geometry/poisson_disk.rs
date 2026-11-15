//! P21.1 — Poisson-disk (blue-noise) sampling in 2-D and 3-D (Bridson 2007).
//!
//! Deterministic, seeded, caller-buffered sampling for ecological scatter
//! (seed dispersal, crown spacing, undergrowth density) with a guaranteed
//! minimum pairwise distance. Both kernels are Tier-1 `HotZeroHeap`: the
//! caller supplies the background acceleration grid and active-list scratch.

use super::primitives::{Point2, Point3};

const EMPTY: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoissonDiskError {
    InvalidDomain,
    InvalidRadius,
    ScratchTooSmall { required: usize },
    OutputTooSmall { required: usize },
}

/// SplitMix64 — fixed-seed stream, bit-exact across platforms.
#[inline]
fn next_u64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[inline]
fn next_f64(state: &mut u64) -> f64 {
    ((next_u64(state) >> 11) as f64) / ((1u64 << 53) as f64)
}

/// Bridson (2007) dart-throwing over a background grid.
///
/// Samples in `[min, max]^2` with minimum pairwise distance `radius`
/// (background grid cell = `radius/√2`). `grid` must hold
/// `⌈w/cell⌉·⌈h/cell⌉` cells; `active` and `out` cap the active list and the
/// output. Returns the number of emitted samples.
///
/// Tier-1 `HotZeroHeap`.
#[allow(clippy::too_many_arguments)]
pub fn poisson_disk_2d(
    min: Point2,
    max: Point2,
    radius: f64,
    seed: u64,
    k_attempts: usize,
    grid: &mut [u32],
    active: &mut [u32],
    out: &mut [Point2],
) -> Result<usize, PoissonDiskError> {
    let (w, h) = (max.x - min.x, max.y - min.y);
    if !(w > 0.0 && h > 0.0 && w.is_finite() && h.is_finite()) {
        return Err(PoissonDiskError::InvalidDomain);
    }
    if !(radius > 0.0 && radius.is_finite()) || k_attempts == 0 {
        return Err(PoissonDiskError::InvalidRadius);
    }
    let cell = radius / std::f64::consts::SQRT_2;
    let (gx, gy) = ((w / cell).ceil() as usize, (h / cell).ceil() as usize);
    if grid.len() < gx.saturating_mul(gy) || active.is_empty() {
        return Err(PoissonDiskError::ScratchTooSmall { required: gx * gy });
    }
    grid[..gx * gy].fill(EMPTY);
    let mut rng = seed;
    let mut n_out = 0usize;
    let mut n_active = 0usize;

    let emit = |p: Point2,
                grid: &mut [u32],
                active: &mut [u32],
                out: &mut [Point2],
                n_out: &mut usize,
                n_active: &mut usize| {
        if *n_out >= out.len() || *n_active >= active.len() {
            return;
        }
        let cx = ((p.x - min.x) / cell) as usize;
        let cy = ((p.y - min.y) / cell) as usize;
        grid[cy * gx + cx] = *n_out as u32;
        out[*n_out] = p;
        active[*n_active] = *n_out as u32;
        *n_out += 1;
        *n_active += 1;
    };

    emit(
        Point2::new(
            min.x + w * next_f64(&mut rng),
            min.y + h * next_f64(&mut rng),
        ),
        grid,
        active,
        out,
        &mut n_out,
        &mut n_active,
    );

    let r2 = radius * radius;
    while n_active > 0 && n_out < out.len() {
        let ai = (next_u64(&mut rng) as usize) % n_active;
        let base = out[active[ai] as usize];
        let mut accepted = false;
        for _ in 0..k_attempts {
            let ang = std::f64::consts::TAU * next_f64(&mut rng);
            let rad = radius * (1.0 + next_f64(&mut rng));
            let cand = Point2::new(base.x + rad * ang.cos(), base.y + rad * ang.sin());
            if cand.x < min.x || cand.x >= max.x || cand.y < min.y || cand.y >= max.y {
                continue;
            }
            let cx = ((cand.x - min.x) / cell) as usize;
            let cy = ((cand.y - min.y) / cell) as usize;
            let mut ok = true;
            'ngb: for dy in -2i64..=2 {
                for dx in -2i64..=2 {
                    let nx = cx as i64 + dx;
                    let ny = cy as i64 + dy;
                    if nx < 0 || ny < 0 || nx >= gx as i64 || ny >= gy as i64 {
                        continue;
                    }
                    let gi = grid[ny as usize * gx + nx as usize];
                    if gi != EMPTY {
                        let q = out[gi as usize];
                        let d2 = (q.x - cand.x).powi(2) + (q.y - cand.y).powi(2);
                        if d2 < r2 {
                            ok = false;
                            break 'ngb;
                        }
                    }
                }
            }
            if ok {
                emit(cand, grid, active, out, &mut n_out, &mut n_active);
                accepted = true;
                break;
            }
        }
        if !accepted {
            // Swap-remove the exhausted active index (deterministic order).
            n_active -= 1;
            active[ai] = active[n_active];
        }
    }
    Ok(n_out)
}

/// Bridson 3-D variant (cell = `radius/√3`).
///
/// Tier-1 `HotZeroHeap`.
#[allow(clippy::too_many_arguments)]
pub fn poisson_disk_3d(
    min: Point3,
    max: Point3,
    radius: f64,
    seed: u64,
    k_attempts: usize,
    grid: &mut [u32],
    active: &mut [u32],
    out: &mut [Point3],
) -> Result<usize, PoissonDiskError> {
    let (w, h, d) = (max.x - min.x, max.y - min.y, max.z - min.z);
    if !(w > 0.0 && h > 0.0 && d > 0.0) || !(w + h + d).is_finite() {
        return Err(PoissonDiskError::InvalidDomain);
    }
    if !(radius > 0.0 && radius.is_finite()) || k_attempts == 0 {
        return Err(PoissonDiskError::InvalidRadius);
    }
    let cell = radius / 3.0f64.sqrt();
    let (gx, gy, gz) = (
        (w / cell).ceil() as usize,
        (h / cell).ceil() as usize,
        (d / cell).ceil() as usize,
    );
    if grid.len() < gx.saturating_mul(gy).saturating_mul(gz) || active.is_empty() {
        return Err(PoissonDiskError::ScratchTooSmall {
            required: gx * gy * gz,
        });
    }
    grid[..gx * gy * gz].fill(EMPTY);
    let mut rng = seed;
    let (mut n_out, mut n_active) = (0usize, 0usize);

    let cell_index = |p: Point3| -> usize {
        let cx = ((p.x - min.x) / cell) as usize;
        let cy = ((p.y - min.y) / cell) as usize;
        let cz = ((p.z - min.z) / cell) as usize;
        (cz * gy + cy) * gx + cx
    };

    macro_rules! emit {
        ($p:expr) => {{
            let _p = $p;
            if n_out < out.len() && n_active < active.len() {
                grid[cell_index(_p)] = n_out as u32;
                out[n_out] = _p;
                active[n_active] = n_out as u32;
                n_out += 1;
                n_active += 1;
            }
        }};
    }

    emit!(Point3::new(
        min.x + w * next_f64(&mut rng),
        min.y + h * next_f64(&mut rng),
        min.z + d * next_f64(&mut rng),
    ));

    let r2 = radius * radius;
    while n_active > 0 && n_out < out.len() {
        let ai = (next_u64(&mut rng) as usize) % n_active;
        let base = out[active[ai] as usize];
        let mut accepted = false;
        for _ in 0..k_attempts {
            // Uniform direction + radius in [r, 2r].
            let z = 2.0 * next_f64(&mut rng) - 1.0;
            let ang = std::f64::consts::TAU * next_f64(&mut rng);
            let s = (1.0 - z * z).max(0.0).sqrt();
            let rad = radius * (1.0 + next_f64(&mut rng));
            let cand = Point3::new(
                base.x + rad * s * ang.cos(),
                base.y + rad * s * ang.sin(),
                base.z + rad * z,
            );
            if cand.x < min.x
                || cand.x >= max.x
                || cand.y < min.y
                || cand.y >= max.y
                || cand.z < min.z
                || cand.z >= max.z
            {
                continue;
            }
            let cx = ((cand.x - min.x) / cell) as i64;
            let cy = ((cand.y - min.y) / cell) as i64;
            let cz = ((cand.z - min.z) / cell) as i64;
            let mut ok = true;
            'ngb: for dz in -2i64..=2 {
                for dy in -2i64..=2 {
                    for dx in -2i64..=2 {
                        let (nx, ny, nz) = (cx + dx, cy + dy, cz + dz);
                        if nx < 0
                            || ny < 0
                            || nz < 0
                            || nx >= gx as i64
                            || ny >= gy as i64
                            || nz >= gz as i64
                        {
                            continue;
                        }
                        let gi = grid[(nz as usize * gy + ny as usize) * gx + nx as usize];
                        if gi != EMPTY {
                            let q = out[gi as usize];
                            let d2 = (q.x - cand.x).powi(2)
                                + (q.y - cand.y).powi(2)
                                + (q.z - cand.z).powi(2);
                            if d2 < r2 {
                                ok = false;
                                break 'ngb;
                            }
                        }
                    }
                }
            }
            if ok {
                emit!(cand);
                accepted = true;
                break;
            }
        }
        if !accepted {
            n_active -= 1;
            active[ai] = active[n_active];
        }
    }
    Ok(n_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poisson_2d_respects_min_distance_and_seed_determinism() {
        let mut grid = [EMPTY; 64 * 64];
        let mut active = [0u32; 512];
        let mut out = [Point2::default(); 512];
        let n = poisson_disk_2d(
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 4.0),
            0.4,
            42,
            30,
            &mut grid,
            &mut active,
            &mut out,
        )
        .unwrap();
        assert!(n > 30, "expected dense blue-noise set, got {n}");
        for i in 0..n {
            for j in i + 1..n {
                let d2 = (out[i].x - out[j].x).powi(2) + (out[i].y - out[j].y).powi(2);
                assert!(
                    d2 >= 0.4 * 0.4 - 1e-9,
                    "samples {i},{j} violate min distance"
                );
            }
        }
        // Same seed reproduces identical stream.
        let mut out2 = [Point2::default(); 512];
        let n2 = poisson_disk_2d(
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 4.0),
            0.4,
            42,
            30,
            &mut grid,
            &mut active,
            &mut out2,
        )
        .unwrap();
        assert_eq!(n, n2);
        assert_eq!(&out[..n], &out2[..n2]);
    }

    #[test]
    fn poisson_3d_respects_min_distance() {
        let mut grid = [EMPTY; 32 * 32 * 32];
        let mut active = [0u32; 256];
        let mut out = [Point3::default(); 256];
        let n = poisson_disk_3d(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(3.0, 3.0, 3.0),
            0.6,
            7,
            30,
            &mut grid,
            &mut active,
            &mut out,
        )
        .unwrap();
        assert!(n > 10);
        for i in 0..n {
            for j in i + 1..n {
                let d2 = (out[i].x - out[j].x).powi(2)
                    + (out[i].y - out[j].y).powi(2)
                    + (out[i].z - out[j].z).powi(2);
                assert!(d2 >= 0.36 - 1e-9);
            }
        }
    }
}
