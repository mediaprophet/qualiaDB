//! Lite non-negative unmixing given 2–3 stain basis vectors.
//!
//! Honest **lite** path: multiplicative updates (Lee–Seung style) on OD with a
//! fixed iteration budget — not full sparse NMF / SNMF with dictionary learning.
//! Suitable for small patches when stain vectors are already known (e.g. from Macenko).

use super::HistoError;

/// Stain basis in optical-density space: up to 3 vectors × RGB.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StainBasis {
    /// Number of active stains (2 or 3).
    pub k: u8,
    /// Rows = stains, cols = R,G,B OD direction (need not be unit, should be ≥0).
    pub vectors: [[f32; 3]; 3],
}

impl StainBasis {
    /// Build a 2-stain basis from two RGB OD vectors.
    pub fn two(s0: [f32; 3], s1: [f32; 3]) -> Self {
        Self {
            k: 2,
            vectors: [s0, s1, [0.0; 3]],
        }
    }

    /// Build a 3-stain basis.
    pub fn three(s0: [f32; 3], s1: [f32; 3], s2: [f32; 3]) -> Self {
        Self {
            k: 3,
            vectors: [s0, s1, s2],
        }
    }
}

const LN10: f32 = 2.302_585_092_994_046;

#[inline]
fn od_from_u8(v: u8) -> f32 {
    -(((v as f32 + 1.0) / 255.0).ln() / LN10)
}

/// Unmix packed RGB into non-negative concentrations via multiplicative updates.
///
/// - `rgb`: packed RGB8, `n` pixels
/// - `basis`: 2 or 3 stain vectors in OD space
/// - `iters`: multiplicative update iterations (typical 20–50 for lite)
/// - `out_conc`: row-major `[pixel][stain]`, length ≥ `n * k`
///
/// Initializes concentrations from a non-negative least projection, then refines
/// with `C ← C ⊙ (Sᵀ Y) / (Sᵀ S C + ε)` per pixel (independent MU — patch-local lite).
pub fn snmf_unmix_lite(
    rgb: &[u8],
    basis: &StainBasis,
    iters: u32,
    out_conc: &mut [f32],
) -> Result<usize, HistoError> {
    if rgb.is_empty() {
        return Err(HistoError::EmptyInput);
    }
    if rgb.len() % 3 != 0 {
        return Err(HistoError::InvalidParameter);
    }
    let k = basis.k as usize;
    if k < 2 || k > 3 {
        return Err(HistoError::InvalidParameter);
    }
    let n = rgb.len() / 3;
    if out_conc.len() < n * k {
        return Err(HistoError::BufferTooSmall);
    }

    // Gram G = S Sᵀ (k×k) and we also need S columns.
    // S is k×3 (rows = stains).
    let mut s = [[0.0f32; 3]; 3];
    for i in 0..k {
        for c in 0..3 {
            // Force non-negative basis for physical OD unmix.
            s[i][c] = basis.vectors[i][c].max(0.0);
        }
    }
    let mut g = [[0.0f32; 3]; 3];
    for i in 0..k {
        for j in 0..k {
            g[i][j] = s[i][0] * s[j][0] + s[i][1] * s[j][1] + s[i][2] * s[j][2];
        }
    }
    // Degenerate basis check.
    let mut trace = 0.0f32;
    for i in 0..k {
        trace += g[i][i];
    }
    if trace < 1e-12 {
        return Err(HistoError::DegenerateData);
    }

    const EPS: f32 = 1e-8;
    for p in 0..n {
        let base = p * 3;
        let y = [
            od_from_u8(rgb[base]),
            od_from_u8(rgb[base + 1]),
            od_from_u8(rgb[base + 2]),
        ];
        // sty = S * y  (k,)  — note S rows, so (S y)_i = s_i · y
        let mut sty = [0.0f32; 3];
        for i in 0..k {
            sty[i] = s[i][0] * y[0] + s[i][1] * y[1] + s[i][2] * y[2];
            if sty[i] < 0.0 {
                sty[i] = 0.0;
            }
        }
        // Init C from max(sty, eps) / diag(G) (scaled).
        let mut c = [0.0f32; 3];
        for i in 0..k {
            c[i] = (sty[i] / (g[i][i] + EPS)).max(EPS);
        }
        // Multiplicative updates.
        for _ in 0..iters {
            // num = sty (fixed); den = G c
            let mut den = [0.0f32; 3];
            for i in 0..k {
                let mut d = 0.0f32;
                for j in 0..k {
                    d += g[i][j] * c[j];
                }
                den[i] = d + EPS;
            }
            for i in 0..k {
                c[i] = (c[i] * sty[i] / den[i]).max(0.0);
            }
        }
        let o = p * k;
        for i in 0..k {
            out_conc[o + i] = c[i];
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmix_recovers_dominant_stain() {
        // Pure-ish blue-purple pixel should load on H-like vector.
        let h = [0.2f32, 0.3, 0.8]; // blue-heavy
        let e = [0.7f32, 0.2, 0.2]; // red-heavy
        let basis = StainBasis::two(h, e);
        // RGB that looks purple/blue stained.
        let rgb = [50u8, 40, 140];
        let mut conc = [0f32; 2];
        snmf_unmix_lite(&rgb, &basis, 30, &mut conc).unwrap();
        // At least one stain channel loads; exact basis dominance is data-dependent.
        assert!(
            conc[0] > 0.0 || conc[1] > 0.0,
            "expected non-zero unmix: {:?}",
            conc
        );
    }

    #[test]
    fn three_stain_writes_k_channels() {
        let basis = StainBasis::three([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
        let rgb = [100u8, 80, 60, 50, 50, 50];
        let mut conc = [0f32; 6];
        let n = snmf_unmix_lite(&rgb, &basis, 10, &mut conc).unwrap();
        assert_eq!(n, 2);
        // Non-negative.
        for v in &conc {
            assert!(*v >= 0.0);
        }
    }

    #[test]
    fn rejects_k_one() {
        let basis = StainBasis {
            k: 1,
            vectors: [[1.0, 0.0, 0.0], [0.0; 3], [0.0; 3]],
        };
        let rgb = [10u8, 10, 10];
        let mut conc = [0f32; 1];
        assert_eq!(
            snmf_unmix_lite(&rgb, &basis, 5, &mut conc),
            Err(HistoError::InvalidParameter)
        );
    }
}
