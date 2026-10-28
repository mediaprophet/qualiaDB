// C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/terrain/noise_simd_lut.rs
// Deterministic, seed-reproducible procedural noise & multi-fractal cascades.
// Allocation class: Tier-1 HotZeroHeap (zero heap allocations in hot paths).
// Pure Rust implementation: no external noise dependencies.

/// Deterministic 256-entry Ken Perlin permutation table.
pub const RANPERM: [u8; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225,
    140, 36, 103, 30, 69, 142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148,
    247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219, 203, 117, 35, 11, 32,
    57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122,
    60, 211, 133, 230, 220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54,
    65, 25, 63, 161, 1, 216, 80, 73, 209, 76, 132, 187, 208, 89, 18, 169,
    200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173, 186, 3, 64,
    52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212,
    207, 206, 59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213,
    119, 248, 152, 2, 44, 154, 163, 70, 221, 153, 101, 155, 167, 43, 172, 9,
    129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232, 178, 185, 112, 104,
    218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162, 241,
    81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157,
    184, 84, 204, 176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93,
    222, 114, 67, 29, 24, 72, 243, 141, 128, 195, 78, 66, 215, 61, 156, 180,
];

/// 16-direction 3D unit gradient lookup table.
pub const GRAD3: [[f32; 3]; 16] = [
    [1.0, 1.0, 0.0], [-1.0, 1.0, 0.0], [1.0, -1.0, 0.0], [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0], [-1.0, 0.0, 1.0], [1.0, 0.0, -1.0], [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0], [0.0, -1.0, 1.0], [0.0, 1.0, -1.0], [0.0, -1.0, -1.0],
    [1.0, 1.0, 0.0], [-1.0, 1.0, 0.0], [0.0, -1.0, 1.0], [0.0, -1.0, -1.0],
];

/// Hash coordinate triple with seed into an 8-bit index via permutation table.
#[inline]
pub fn perm_hash(seed: u64, x: i32, y: i32, z: i32) -> u8 {
    let xi = (x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let yi = (y as i64 as u64).wrapping_mul(0xC6A4_A793_5BD1_E995);
    let zi = (z as i64 as u64).wrapping_mul(0x85EB_CA6B_4723_0937);
    let mut h = seed ^ xi ^ yi.rotate_left(21) ^ zi.rotate_left(42);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    RANPERM[(h as usize) & 0xFF]
}

#[inline(always)]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline(always)]
fn fade_deriv(t: f32) -> f32 {
    30.0 * t * t * (t * (t - 2.0) + 1.0)
}

/// 2D Perlin gradient noise evaluated with quintic fade curve, range [-1.0, 1.0].
pub fn perlin_2d(x: f32, y: f32, seed: u64) -> f32 {
    perlin_2d_deriv(x, y, seed).0
}

/// 2D Perlin gradient noise returning value and analytical partial derivatives `(val, dx, dy)`.
pub fn perlin_2d_deriv(x: f32, y: f32, seed: u64) -> (f32, f32, f32) {
    let (x0, y0) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let (u, v, du, dv) = (fade(fx), fade(fy), fade_deriv(fx), fade_deriv(fy));

    let g00 = GRAD3[(perm_hash(seed, x0, y0, 0) & 15) as usize];
    let g10 = GRAD3[(perm_hash(seed, x0 + 1, y0, 0) & 15) as usize];
    let g01 = GRAD3[(perm_hash(seed, x0, y0 + 1, 0) & 15) as usize];
    let g11 = GRAD3[(perm_hash(seed, x0 + 1, y0 + 1, 0) & 15) as usize];

    let (d00, d10) = (g00[0] * fx + g00[1] * fy, g10[0] * (fx - 1.0) + g10[1] * fy);
    let (d01, d11) = (g01[0] * fx + g01[1] * (fy - 1.0), g11[0] * (fx - 1.0) + g11[1] * (fy - 1.0));

    let (a, b, c, d) = (d00, d10 - d00, d01 - d00, d11 - d10 - d01 + d00);
    let val = a + u * b + v * c + u * v * d;

    let (da_dx, db_dx, dc_dx, dd_dx) = (g00[0], g10[0] - g00[0], g01[0] - g00[0], g11[0] - g10[0] - g01[0] + g00[0]);
    let (da_dy, db_dy, dc_dy, dd_dy) = (g00[1], g10[1] - g00[1], g01[1] - g00[1], g11[1] - g10[1] - g01[1] + g00[1]);

    let dx = du * (b + v * d) + (da_dx + u * db_dx + v * dc_dx + u * v * dd_dx);
    let dy = dv * (c + u * d) + (da_dy + u * db_dy + v * dc_dy + u * v * dd_dy);

    const SCALE: f32 = 1.41421356;
    ((val * SCALE).clamp(-1.0, 1.0), dx * SCALE, dy * SCALE)
}

/// 3D Perlin gradient noise with trilinear quintic interpolation, range [-1.0, 1.0].
pub fn perlin_3d(x: f32, y: f32, z: f32, seed: u64) -> f32 {
    let (x0, y0, z0) = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
    let (fx, fy, fz) = (x - x.floor(), y - y.floor(), z - z.floor());
    let (u, v, w) = (fade(fx), fade(fy), fade(fz));

    let mut dots = [0.0f32; 8];
    for i in 0..2 {
        for j in 0..2 {
            for k in 0..2 {
                let g = GRAD3[(perm_hash(seed, x0 + i, y0 + j, z0 + k) & 15) as usize];
                dots[((i << 2) | (j << 1) | k) as usize] = g[0] * (fx - i as f32) + g[1] * (fy - j as f32) + g[2] * (fz - k as f32);
            }
        }
    }

    let x00 = dots[0] + u * (dots[4] - dots[0]);
    let x01 = dots[1] + u * (dots[5] - dots[1]);
    let x10 = dots[2] + u * (dots[6] - dots[2]);
    let x11 = dots[3] + u * (dots[7] - dots[3]);
    let y0_val = x00 + v * (x10 - x00);
    let y1_val = x01 + v * (x11 - x01);
    let val = y0_val + w * (y1_val - y0_val);
    (val * 1.1547).clamp(-1.0, 1.0)
}

/// OpenSimplex2S smooth 2D simplex gradient noise without axis-aligned artifacts.
pub fn opensimplex2s_2d(x: f32, y: f32, seed: u64) -> f32 {
    const SKEW_IN: f32 = 0.3660254037844386;
    const SKEW_OUT: f32 = 0.21132486540518713;

    let s = (x + y) * SKEW_IN;
    let (i, j) = ((x + s).floor() as i32, (y + s).floor() as i32);
    let t = (i + j) as f32 * SKEW_OUT;
    let (x0, y0) = (x - (i as f32 - t), y - (j as f32 - t));

    let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
    let (x1, y1) = (x0 - i1 as f32 + SKEW_OUT, y0 - j1 as f32 + SKEW_OUT);
    let (x2, y2) = (x0 - 1.0 + 2.0 * SKEW_OUT, y0 - 1.0 + 2.0 * SKEW_OUT);

    let mut n = 0.0f32;
    let t0 = 0.5 - x0 * x0 - y0 * y0;
    if t0 > 0.0 {
        let g0 = GRAD3[(perm_hash(seed, i, j, 0) & 15) as usize];
        let t0_2 = t0 * t0;
        n += t0_2 * t0_2 * (g0[0] * x0 + g0[1] * y0);
    }
    let t1 = 0.5 - x1 * x1 - y1 * y1;
    if t1 > 0.0 {
        let g1 = GRAD3[(perm_hash(seed, i + i1, j + j1, 0) & 15) as usize];
        let t1_2 = t1 * t1;
        n += t1_2 * t1_2 * (g1[0] * x1 + g1[1] * y1);
    }
    let t2 = 0.5 - x2 * x2 - y2 * y2;
    if t2 > 0.0 {
        let g2 = GRAD3[(perm_hash(seed, i + 1, j + 1, 0) & 15) as usize];
        let t2_2 = t2 * t2;
        n += t2_2 * t2_2 * (g2[0] * x2 + g2[1] * y2);
    }

    (n * 70.1480577).clamp(-1.0, 1.0)
}

/// 2D Value noise with quintic interpolation, range [-1.0, 1.0].
pub fn value_noise_2d(x: f32, y: f32, seed: u64) -> f32 {
    let (x0, y0) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let (u, v) = (fade(fx), fade(fy));

    let v00 = ((perm_hash(seed, x0, y0, 3) as f32) / 127.5) - 1.0;
    let v10 = ((perm_hash(seed, x0 + 1, y0, 3) as f32) / 127.5) - 1.0;
    let v01 = ((perm_hash(seed, x0, y0 + 1, 3) as f32) / 127.5) - 1.0;
    let v11 = ((perm_hash(seed, x0 + 1, y0 + 1, 3) as f32) / 127.5) - 1.0;

    let nx0 = v00 + u * (v10 - v00);
    let nx1 = v01 + u * (v11 - v01);
    (nx0 + v * (nx1 - nx0)).clamp(-1.0, 1.0)
}

/// Sample payload returned by Voronoi cellular evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoronoiSample {
    pub f1: f32,
    pub f2: f32,
    pub crackle: f32,
    pub cell_id: u64,
}

/// 2D Voronoi / Worley cellular noise evaluation.
pub fn voronoi_2d(x: f32, y: f32, jitter: f32, seed: u64) -> VoronoiSample {
    let (xi, yj) = (x.floor() as i32, y.floor() as i32);
    let j_clamped = jitter.clamp(0.0, 1.0);
    let (mut f1, mut f2, mut cell_id) = (f32::MAX, f32::MAX, 0u64);

    for di in -1..=1 {
        for dj in -1..=1 {
            let (cx, cy) = (xi + di, yj + dj);
            let px = (cx as f32) + (perm_hash(seed, cx, cy, 7) as f32 / 255.0) * j_clamped;
            let py = (cy as f32) + (perm_hash(seed, cx, cy, 9) as f32 / 255.0) * j_clamped;
            let (dx, dy) = (px - x, py - y);
            let dist = (dx * dx + dy * dy).sqrt();

            let cid = seed
                ^ (cx as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ (cy as i64 as u64).wrapping_mul(0xC6A4_A793_5BD1_E995);

            if dist < f1 {
                f2 = f1;
                f1 = dist;
                cell_id = cid;
            } else if dist < f2 {
                f2 = dist;
            }
        }
    }
    VoronoiSample { f1, f2, crackle: (f2 - f1).clamp(0.0, 1.0), cell_id }
}

/// 2D Anisotropic Gabor noise for geological strike / foliation synthesis.
pub fn gabor_anisotropic_2d(
    x: f32,
    y: f32,
    direction_rad: f32,
    freq: f32,
    bandwidth: f32,
    seed: u64,
) -> f32 {
    let (cos_dir, sin_dir) = (direction_rad.cos(), direction_rad.sin());
    let bw2 = (bandwidth.max(0.01)) * (bandwidth.max(0.01));
    let (xi, yj) = (x.floor() as i32, y.floor() as i32);
    let (mut sum, mut weight_sum) = (0.0f32, 0.0f32);

    for di in -1..=1 {
        for dj in -1..=1 {
            let (cx, cy) = ((xi + di) as f32 + 0.5, (yj + dj) as f32 + 0.5);
            let (dx, dy) = (x - cx, y - cy);
            let xr = dx * cos_dir + dy * sin_dir;
            let yr = -dx * sin_dir + dy * cos_dir;
            let dist_sq = xr * xr + (yr * 0.25) * (yr * 0.25);
            let env = (-core::f32::consts::PI * bw2 * dist_sq).exp();
            if env > 1e-5 {
                let phase = (perm_hash(seed, xi + di, yj + dj, 17) as f32 / 255.0) * core::f32::consts::TAU;
                sum += env * (core::f32::consts::TAU * freq * xr + phase).cos();
                weight_sum += env;
            }
        }
    }
    if weight_sum > 1e-4 { (sum / weight_sum).clamp(-1.0, 1.0) } else { 0.0 }
}

/// 2D Band-limited orthogonal wavelet tile evaluation via cubic B-spline reconstruction.
pub fn wavelet_bandlimited_2d(x: f32, y: f32, seed: u64) -> f32 {
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (tx, ty) = (x - x.floor(), y - y.floor());

    let wx = [
        (1.0 - tx).powi(3) / 6.0,
        (2.0 / 3.0) - tx * tx * (1.0 - 0.5 * tx),
        (2.0 / 3.0) - (1.0 - tx) * (1.0 - tx) * (1.0 - 0.5 * (1.0 - tx)),
        tx.powi(3) / 6.0,
    ];
    let wy = [
        (1.0 - ty).powi(3) / 6.0,
        (2.0 / 3.0) - ty * ty * (1.0 - 0.5 * ty),
        (2.0 / 3.0) - (1.0 - ty) * (1.0 - ty) * (1.0 - 0.5 * (1.0 - ty)),
        ty.powi(3) / 6.0,
    ];

    let mut sum = 0.0f32;
    for (k, &wk) in wx.iter().enumerate() {
        let gx = ix - 1 + k as i32;
        for (m, &wm) in wy.iter().enumerate() {
            let gy = iy - 1 + m as i32;
            let coeff = ((perm_hash(seed, gx, gy, 42) as f32) / 127.5) - 1.0;
            sum += wk * wm * coeff;
        }
    }
    (sum * 1.5).clamp(-1.0, 1.0)
}

/// Procedural fractal evaluation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractalMode {
    Fbm,
    Ridged,
    Billow,
    HybridMulti,
    SwissTurbulence,
}

/// Multi-fractal cascade configuration parameters.
#[derive(Debug, Clone, Copy)]
pub struct FractalConfig {
    pub mode: FractalMode,
    pub octaves: u8,
    pub frequency: f32,
    pub lacunarity: f32,
    pub persistence: f32,
    pub ridge_offset: f32,
    pub warp_slope: f32,
    pub seed: u64,
}

impl Default for FractalConfig {
    fn default() -> Self {
        Self {
            mode: FractalMode::Fbm,
            octaves: 6,
            frequency: 1.0,
            lacunarity: 2.0,
            persistence: 0.5,
            ridge_offset: 1.0,
            warp_slope: 0.15,
            seed: 1337,
        }
    }
}

/// Evaluates multi-fractal cascades with analytical derivative warp for SwissTurbulence.
pub fn evaluate_fractal_2d(x: f32, y: f32, cfg: &FractalConfig) -> f32 {
    match cfg.mode {
        FractalMode::Fbm => {
            let (mut sum, mut amp, mut freq, mut max_amp) = (0.0f32, 1.0f32, cfg.frequency, 0.0f32);
            for oct in 0..cfg.octaves {
                sum += perlin_2d(x * freq, y * freq, cfg.seed.wrapping_add(oct as u64)) * amp;
                max_amp += amp;
                amp *= cfg.persistence;
                freq *= cfg.lacunarity;
            }
            if max_amp > 0.0 { sum / max_amp } else { 0.0 }
        }
        FractalMode::Ridged => {
            let (mut sum, mut amp, mut freq, mut weight, mut max_amp) = (0.0f32, 1.0f32, cfg.frequency, 1.0f32, 0.0f32);
            for oct in 0..cfg.octaves {
                let n = perlin_2d(x * freq, y * freq, cfg.seed.wrapping_add(oct as u64));
                let mut sig = cfg.ridge_offset - n.abs();
                sig = sig * sig * weight;
                weight = (sig * 2.0).clamp(0.0, 1.0);
                sum += sig * amp;
                max_amp += amp * (cfg.ridge_offset * cfg.ridge_offset);
                amp *= cfg.persistence;
                freq *= cfg.lacunarity;
            }
            if max_amp > 0.0 { ((sum / max_amp) * 2.0 - 1.0).clamp(-1.0, 1.0) } else { 0.0 }
        }
        FractalMode::Billow => {
            let (mut sum, mut amp, mut freq, mut max_amp) = (0.0f32, 1.0f32, cfg.frequency, 0.0f32);
            for oct in 0..cfg.octaves {
                let n = perlin_2d(x * freq, y * freq, cfg.seed.wrapping_add(oct as u64));
                sum += (n.abs() * 2.0 - 1.0) * amp;
                max_amp += amp;
                amp *= cfg.persistence;
                freq *= cfg.lacunarity;
            }
            if max_amp > 0.0 { sum / max_amp } else { 0.0 }
        }
        FractalMode::HybridMulti => {
            let (mut freq, mut amp) = (cfg.frequency, 1.0f32);
            let mut sum = perlin_2d(x * freq, y * freq, cfg.seed) + cfg.ridge_offset;
            let mut weight = sum;
            let mut max_amp = 1.0 + cfg.ridge_offset;
            for oct in 1..cfg.octaves {
                freq *= cfg.lacunarity;
                amp *= cfg.persistence;
                let sig = (perlin_2d(x * freq, y * freq, cfg.seed.wrapping_add(oct as u64)) + cfg.ridge_offset) * amp;
                sum += weight.clamp(0.0, 1.0) * sig;
                weight = sig;
                max_amp += amp * (1.0 + cfg.ridge_offset);
            }
            if max_amp > 0.0 { ((sum / max_amp) * 2.0 - 1.0).clamp(-1.0, 1.0) } else { 0.0 }
        }
        FractalMode::SwissTurbulence => {
            let (mut sum, mut amp, mut max_amp) = (0.0f32, 1.0f32, 0.0f32);
            let (mut wx, mut wy, mut freq) = (0.0f32, 0.0f32, cfg.frequency);
            for oct in 0..cfg.octaves {
                let (val, dx, dy) = perlin_2d_deriv(x * freq + wx, y * freq + wy, cfg.seed.wrapping_add(oct as u64));
                let sign = if val >= 0.0 { -1.0 } else { 1.0 };
                sum += (cfg.ridge_offset - val.abs()) * amp;
                max_amp += amp * cfg.ridge_offset;
                wx += sign * dx * amp * cfg.warp_slope;
                wy += sign * dy * amp * cfg.warp_slope;
                amp *= cfg.persistence;
                freq *= cfg.lacunarity;
            }
            if max_amp > 0.0 { ((sum / max_amp) * 2.0 - 1.0).clamp(-1.0, 1.0) } else { 0.0 }
        }
    }
}

/// 4-wide unrolled evaluation loop writing directly into a caller-supplied grid buffer.
pub fn eval_fractal_grid_into(
    width: usize,
    height: usize,
    origin_x: f32,
    origin_y: f32,
    step: f32,
    cfg: &FractalConfig,
    out: &mut [f32],
) -> Result<(), &'static str> {
    let total = width.checked_mul(height).ok_or("Grid dimensions overflow")?;
    if out.len() < total {
        return Err("Output buffer too small for grid");
    }
    let (chunks, remainder) = (width / 4, width % 4);

    for y in 0..height {
        let py = origin_y + y as f32 * step;
        let row_start = y * width;
        for c in 0..chunks {
            let x0 = c * 4;
            let px0 = origin_x + x0 as f32 * step;
            let (v0, v1) = (evaluate_fractal_2d(px0, py, cfg), evaluate_fractal_2d(px0 + step, py, cfg));
            let (v2, v3) = (evaluate_fractal_2d(px0 + step * 2.0, py, cfg), evaluate_fractal_2d(px0 + step * 3.0, py, cfg));
            out[row_start + x0] = v0;
            out[row_start + x0 + 1] = v1;
            out[row_start + x0 + 2] = v2;
            out[row_start + x0 + 3] = v3;
        }
        for r in 0..remainder {
            let x = chunks * 4 + r;
            out[row_start + x] = evaluate_fractal_2d(origin_x + x as f32 * step, py, cfg);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_determinism_and_ranges() {
        let (s1, s2) = (42u64, 1337u64);

        let p1 = perlin_2d(12.34, 56.78, s1);
        let (p1_again, p2) = (perlin_2d(12.34, 56.78, s1), perlin_2d(12.34, 56.78, s2));
        assert_eq!(p1, p1_again); assert_ne!(p1, p2); assert!(p1 >= -1.0 && p1 <= 1.0);

        let p3d_1 = perlin_3d(1.2, 3.4, 5.6, s1);
        let (p3d_again, p3d_2) = (perlin_3d(1.2, 3.4, 5.6, s1), perlin_3d(1.2, 3.4, 5.6, s2));
        assert_eq!(p3d_1, p3d_again); assert_ne!(p3d_1, p3d_2); assert!(p3d_1 >= -1.0 && p3d_1 <= 1.0);

        let os1 = opensimplex2s_2d(7.8, 9.1, s1);
        let (os1_again, os2) = (opensimplex2s_2d(7.8, 9.1, s1), opensimplex2s_2d(7.8, 9.1, s2));
        assert_eq!(os1, os1_again); assert_ne!(os1, os2); assert!(os1 >= -1.0 && os1 <= 1.0);

        let v1 = voronoi_2d(4.5, 6.7, 0.8, s1);
        let (v1_again, v2) = (voronoi_2d(4.5, 6.7, 0.8, s1), voronoi_2d(4.5, 6.7, 0.8, s2));
        assert_eq!(v1, v1_again); assert_ne!(v1.cell_id, v2.cell_id);
        assert!(v1.f1 <= v1.f2); assert!(v1.crackle >= 0.0 && v1.crackle <= 1.0);

        let g1 = gabor_anisotropic_2d(2.0, 3.0, 0.785, 1.5, 0.5, s1);
        let (g1_again, g2) = (gabor_anisotropic_2d(2.0, 3.0, 0.785, 1.5, 0.5, s1), gabor_anisotropic_2d(2.0, 3.0, 0.785, 1.5, 0.5, s2));
        assert_eq!(g1, g1_again); assert_ne!(g1, g2); assert!(g1 >= -1.0 && g1 <= 1.0);

        let w1 = wavelet_bandlimited_2d(15.3, 22.7, s1);
        let (w1_again, w2) = (wavelet_bandlimited_2d(15.3, 22.7, s1), wavelet_bandlimited_2d(15.3, 22.7, s2));
        assert_eq!(w1, w1_again); assert_ne!(w1, w2); assert!(w1 >= -1.0 && w1 <= 1.0);
    }

    #[test]
    fn test_fractal_modes_and_grid_eval() {
        let modes = [
            FractalMode::Fbm, FractalMode::Ridged, FractalMode::Billow,
            FractalMode::HybridMulti, FractalMode::SwissTurbulence,
        ];
        for &mode in &modes {
            let cfg = FractalConfig { mode, ..Default::default() };
            let val = evaluate_fractal_2d(10.5, 20.25, &cfg);
            assert!(val >= -1.0 && val <= 1.0, "Mode {:?} out of bounds: {}", mode, val);
        }

        let mut grid_buffer = [0.0f32; 32 * 32];
        let cfg = FractalConfig { mode: FractalMode::SwissTurbulence, ..Default::default() };
        assert!(eval_fractal_grid_into(32, 32, 0.0, 0.0, 0.1, &cfg, &mut grid_buffer).is_ok());
        for &val in &grid_buffer {
            assert!(val >= -1.0 && val <= 1.0);
        }

        let mut small_buffer = [0.0f32; 10];
        assert!(eval_fractal_grid_into(32, 32, 0.0, 0.0, 0.1, &cfg, &mut small_buffer).is_err());
    }
}
