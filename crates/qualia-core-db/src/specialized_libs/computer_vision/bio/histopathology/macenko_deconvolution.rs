//! Macenko stain deconvolution (pure Rust).
//!
//! OD transform → covariance eigen of tissue OD → two stain vectors →
//! concentrations for H and E written to caller buffers.
//!
//! Implements the Macenko plane + angular extreme construction (min/max φ on the
//! projected plane — equivalent to α extremes on small tiles without a heap sort).

use super::HistoError;

/// Result of Macenko: two unit-ish stain vectors in OD space (rows: H, E; cols: R,G,B).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MacenkoResult {
    /// Stain matrix S (2×3): row 0 = hematoxylin-like, row 1 = eosin-like (by blue/red bias).
    pub stains: [[f32; 3]; 2],
    /// Number of tissue OD samples used for covariance.
    pub n_tissue: usize,
}

const LN10: f32 = 2.302_585_092_994_046;

#[inline]
fn od_pixel(r: u8, g: u8, b: u8) -> [f32; 3] {
    [
        -(((r as f32 + 1.0) / 255.0).ln() / LN10),
        -(((g as f32 + 1.0) / 255.0).ln() / LN10),
        -(((b as f32 + 1.0) / 255.0).ln() / LN10),
    ]
}

#[inline]
fn norm3(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

#[inline]
fn normalize3(mut v: [f32; 3]) -> [f32; 3] {
    let n = norm3(v).max(1e-12);
    v[0] /= n;
    v[1] /= n;
    v[2] /= n;
    v
}

#[inline]
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Jacobi eigen-decomposition for 3×3 symmetric matrix.
/// Returns eigenvalues and eigenvectors as columns of `vecs`.
fn jacobi_eigen_3x3(mut a: [[f32; 3]; 3]) -> ([f32; 3], [[f32; 3]; 3]) {
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..32 {
        // Find largest off-diagonal.
        let mut p = 0usize;
        let mut q = 1usize;
        let mut max = a[0][1].abs();
        if a[0][2].abs() > max {
            max = a[0][2].abs();
            p = 0;
            q = 2;
        }
        if a[1][2].abs() > max {
            max = a[1][2].abs();
            p = 1;
            q = 2;
        }
        if max < 1e-12 {
            break;
        }
        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];
        // τ = (aqq−app)/(2 apq); t = 1 / (τ + sign(τ) √(1+τ²))
        let tau = (aqq - app) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;
        // Rotate A.
        a[p][p] = app - t * apq;
        a[q][q] = aqq + t * apq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;
        for r in 0..3 {
            if r == p || r == q {
                continue;
            }
            let arp = a[r][p];
            let arq = a[r][q];
            a[r][p] = c * arp - s * arq;
            a[p][r] = a[r][p];
            a[r][q] = s * arp + c * arq;
            a[q][r] = a[r][q];
        }
        // Accumulate eigenvectors.
        for r in 0..3 {
            let vrp = v[r][p];
            let vrq = v[r][q];
            v[r][p] = c * vrp - s * vrq;
            v[r][q] = s * vrp + c * vrq;
        }
    }
    let evals = [a[0][0], a[1][1], a[2][2]];
    // v columns are eigenvectors: column j = (v[0][j], v[1][j], v[2][j])
    (evals, v)
}

fn sort_eigen_desc(evals: [f32; 3], vecs: [[f32; 3]; 3]) -> ([f32; 3], [[f32; 3]; 3]) {
    let mut idx = [0usize, 1, 2];
    idx.sort_by(|&i, &j| {
        evals[j]
            .partial_cmp(&evals[i])
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    let mut e = [0.0f32; 3];
    let mut v = [[0.0f32; 3]; 3];
    for (k, &i) in idx.iter().enumerate() {
        e[k] = evals[i];
        for r in 0..3 {
            v[r][k] = vecs[r][i];
        }
    }
    (e, v)
}

/// Macenko deconvolution.
///
/// - `rgb`: packed RGB8, length `n*3`
/// - `od_thresh`: minimum OD L2 norm to count as tissue (typical 0.15)
/// - `out_h` / `out_e`: concentration channels, length ≥ `n` (pixels)
/// - returns stain vectors and tissue sample count
pub fn macenko_deconvolution(
    rgb: &[u8],
    od_thresh: f32,
    out_h: &mut [f32],
    out_e: &mut [f32],
) -> Result<MacenkoResult, HistoError> {
    if rgb.is_empty() {
        return Err(HistoError::EmptyInput);
    }
    if rgb.len() % 3 != 0 {
        return Err(HistoError::InvalidParameter);
    }
    let n = rgb.len() / 3;
    if out_h.len() < n || out_e.len() < n {
        return Err(HistoError::BufferTooSmall);
    }
    if od_thresh < 0.0 {
        return Err(HistoError::InvalidParameter);
    }

    // Pass 1: accumulate mean of tissue OD (stack only).
    let mut mean = [0.0f64; 3];
    let mut n_tissue = 0usize;
    for i in 0..n {
        let base = i * 3;
        let od = od_pixel(rgb[base], rgb[base + 1], rgb[base + 2]);
        if norm3(od) >= od_thresh {
            mean[0] += od[0] as f64;
            mean[1] += od[1] as f64;
            mean[2] += od[2] as f64;
            n_tissue += 1;
        }
    }
    if n_tissue < 3 {
        return Err(HistoError::DegenerateData);
    }
    for c in 0..3 {
        mean[c] /= n_tissue as f64;
    }

    // Pass 2: covariance of tissue OD.
    let mut cov = [[0.0f64; 3]; 3];
    for i in 0..n {
        let base = i * 3;
        let od = od_pixel(rgb[base], rgb[base + 1], rgb[base + 2]);
        if norm3(od) < od_thresh {
            continue;
        }
        let d = [
            od[0] as f64 - mean[0],
            od[1] as f64 - mean[1],
            od[2] as f64 - mean[2],
        ];
        for r in 0..3 {
            for c in 0..3 {
                cov[r][c] += d[r] * d[c];
            }
        }
    }
    let inv = 1.0 / (n_tissue as f64);
    let mut cov_f = [[0.0f32; 3]; 3];
    for r in 0..3 {
        for c in 0..3 {
            cov_f[r][c] = (cov[r][c] * inv) as f32;
        }
    }

    let (evals, vecs) = jacobi_eigen_3x3(cov_f);
    let (_evals, vecs) = sort_eigen_desc(evals, vecs);
    // Plane spanned by top-2 eigenvectors (columns 0 and 1).
    let v1 = normalize3([vecs[0][0], vecs[1][0], vecs[2][0]]);
    let v2 = normalize3([vecs[0][1], vecs[1][1], vecs[2][1]]);

    // Project tissue OD onto plane; track min/max angle (Macenko α extremes).
    let mut min_phi = f32::INFINITY;
    let mut max_phi = f32::NEG_INFINITY;
    let mut min_dir = v1;
    let mut max_dir = v1;
    for i in 0..n {
        let base = i * 3;
        let od = od_pixel(rgb[base], rgb[base + 1], rgb[base + 2]);
        if norm3(od) < od_thresh {
            continue;
        }
        let p1 = dot3(od, v1);
        let p2 = dot3(od, v2);
        let phi = p2.atan2(p1);
        if phi < min_phi {
            min_phi = phi;
            min_dir = normalize3([
                phi.cos() * v1[0] + phi.sin() * v2[0],
                phi.cos() * v1[1] + phi.sin() * v2[1],
                phi.cos() * v1[2] + phi.sin() * v2[2],
            ]);
        }
        if phi > max_phi {
            max_phi = phi;
            max_dir = normalize3([
                phi.cos() * v1[0] + phi.sin() * v2[0],
                phi.cos() * v1[1] + phi.sin() * v2[1],
                phi.cos() * v1[2] + phi.sin() * v2[2],
            ]);
        }
    }
    if !min_phi.is_finite() || !max_phi.is_finite() {
        return Err(HistoError::DegenerateData);
    }

    let mut s0 = min_dir;
    let mut s1 = max_dir;
    // Order: hematoxylin tends to higher blue OD component.
    if s0[2] < s1[2] {
        core::mem::swap(&mut s0, &mut s1);
    }
    // Ensure non-negative OD directions (flip if majority negative).
    for s in [&mut s0, &mut s1] {
        let mut pos = 0.0f32;
        for c in 0..3 {
            pos += s[c];
        }
        if pos < 0.0 {
            s[0] = -s[0];
            s[1] = -s[1];
            s[2] = -s[2];
        }
        *s = normalize3(*s);
    }

    // Concentrations via least-squares: C = (S S^T)^{-1} S OD, with S rows = stains (2×3).
    // 2×2 Gram matrix.
    let g00 = dot3(s0, s0);
    let g01 = dot3(s0, s1);
    let g11 = dot3(s1, s1);
    let det = g00 * g11 - g01 * g01;
    if det.abs() < 1e-12 {
        return Err(HistoError::DegenerateData);
    }
    let inv00 = g11 / det;
    let inv01 = -g01 / det;
    let inv11 = g00 / det;

    for i in 0..n {
        let base = i * 3;
        let od = od_pixel(rgb[base], rgb[base + 1], rgb[base + 2]);
        let y0 = dot3(s0, od);
        let y1 = dot3(s1, od);
        let mut c0 = inv00 * y0 + inv01 * y1;
        let mut c1 = inv01 * y0 + inv11 * y1;
        // Non-negative concentrations (physical OD).
        if c0 < 0.0 {
            c0 = 0.0;
        }
        if c1 < 0.0 {
            c1 = 0.0;
        }
        out_h[i] = c0;
        out_e[i] = c1;
    }

    Ok(MacenkoResult {
        stains: [s0, s1],
        n_tissue,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic two-stain tile: mix of purple (H) and pink (E) pixels + white bg.
    fn synthetic_he_tile() -> Vec<u8> {
        let mut v = Vec::with_capacity(32 * 32 * 3);
        for y in 0..32u32 {
            for x in 0..32u32 {
                let (r, g, b) = if x < 4 || y < 4 {
                    (250u8, 250, 250) // background
                } else if (x + y) % 2 == 0 {
                    // Hematoxylin-like: dark purple/blue
                    (60, 40, 120)
                } else {
                    // Eosin-like: pink
                    (220, 100, 140)
                };
                v.push(r);
                v.push(g);
                v.push(b);
            }
        }
        v
    }

    #[test]
    fn macenko_produces_two_channels() {
        let rgb = synthetic_he_tile();
        let n = rgb.len() / 3;
        let mut h = vec![0f32; n];
        let mut e = vec![0f32; n];
        let res = macenko_deconvolution(&rgb, 0.15, &mut h, &mut e).unwrap();
        assert_eq!(res.stains.len(), 2);
        assert!(res.n_tissue > 10);
        // Stain vectors unit-ish.
        for s in &res.stains {
            let nrm = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
            assert!((nrm - 1.0).abs() < 1e-3, "norm={nrm}");
        }
        // Both channels have some positive mass on tissue.
        let sum_h: f32 = h.iter().sum();
        let sum_e: f32 = e.iter().sum();
        assert!(sum_h > 0.0, "H empty");
        assert!(sum_e > 0.0, "E empty");
        // Not identical channels.
        let mut diff = 0.0f32;
        for i in 0..n {
            diff += (h[i] - e[i]).abs();
        }
        assert!(diff > 1.0, "H and E nearly identical");
    }

    #[test]
    fn macenko_rejects_all_white() {
        let rgb = vec![255u8; 12];
        let mut h = [0f32; 4];
        let mut e = [0f32; 4];
        assert_eq!(
            macenko_deconvolution(&rgb, 0.15, &mut h, &mut e),
            Err(HistoError::DegenerateData)
        );
    }
}
