//! Information theory — Shannon entropy, KL divergence, cross-entropy and mutual
//! information over discrete distributions / samples (all in **bits**, `log₂`).
//!
//! Mission note: mutual information is a principled, assumption-free relevance
//! signal — `I(X;Y)` measures how much knowing `X` reduces uncertainty about `Y`
//! with no linearity assumption — which is exactly what the 10D→5D NQuin relevance
//! router needs to choose its projection.

/// Shannon entropy `H(p) = −Σ pᵢ·log₂ pᵢ` (bits) of a probability vector. Zero
/// probabilities contribute 0. `None` if empty or the masses don't form a positive
/// distribution.
pub fn entropy(p: &[f64]) -> Option<f64> {
    if p.is_empty() {
        return None;
    }
    let total: f64 = p.iter().sum();
    if !(total > 0.0) {
        return None;
    }
    let mut h = 0.0;
    for &pi in p {
        let q = pi / total; // tolerate unnormalized input
        if q > 0.0 {
            h -= q * q.log2();
        }
    }
    Some(h)
}

/// Entropy from integer counts (normalized internally).
pub fn entropy_from_counts(counts: &[usize]) -> Option<f64> {
    if counts.is_empty() {
        return None;
    }
    let p: Vec<f64> = counts.iter().map(|&c| c as f64).collect();
    entropy(&p)
}

/// Kullback–Leibler divergence `D(p‖q) = Σ pᵢ·log₂(pᵢ/qᵢ)` (bits). Both inputs are
/// normalized internally. `None` on a length mismatch, empty input, or if `qᵢ = 0`
/// where `pᵢ > 0` (the divergence is then infinite — refuse rather than fabricate).
pub fn kl_divergence(p: &[f64], q: &[f64]) -> Option<f64> {
    if p.is_empty() || p.len() != q.len() {
        return None;
    }
    let (sp, sq): (f64, f64) = (p.iter().sum(), q.iter().sum());
    if !(sp > 0.0) || !(sq > 0.0) {
        return None;
    }
    let mut d = 0.0;
    for (&pi, &qi) in p.iter().zip(q) {
        let pn = pi / sp;
        let qn = qi / sq;
        if pn > 0.0 {
            if qn <= 0.0 {
                return None; // support of p not covered by q
            }
            d += pn * (pn / qn).log2();
        }
    }
    Some(d)
}

/// Cross-entropy `H(p, q) = −Σ pᵢ·log₂ qᵢ` (bits). `None` like [`kl_divergence`].
pub fn cross_entropy(p: &[f64], q: &[f64]) -> Option<f64> {
    Some(entropy(p)? + kl_divergence(p, q)?)
}

/// Mutual information `I(X;Y) = H(X) + H(Y) − H(X,Y)` (bits), estimated from paired
/// discrete samples (small non-negative integer labels). `None` on length mismatch
/// or empty input. `I ≥ 0`, and `I = 0` iff `X ⟂ Y` in the sample.
pub fn mutual_information_discrete(x: &[usize], y: &[usize]) -> Option<f64> {
    let n = x.len();
    if n == 0 || n != y.len() {
        return None;
    }
    let nx = x.iter().max().copied().unwrap_or(0) + 1;
    let ny = y.iter().max().copied().unwrap_or(0) + 1;
    let mut joint = vec![0.0f64; nx * ny];
    let mut px = vec![0.0f64; nx];
    let mut py = vec![0.0f64; ny];
    for (&xi, &yi) in x.iter().zip(y) {
        joint[xi * ny + yi] += 1.0;
        px[xi] += 1.0;
        py[yi] += 1.0;
    }
    let nf = n as f64;
    let mut mi = 0.0;
    for xi in 0..nx {
        for yi in 0..ny {
            let pxy = joint[xi * ny + yi] / nf;
            if pxy > 0.0 {
                let pxi = px[xi] / nf;
                let pyi = py[yi] / nf;
                mi += pxy * (pxy / (pxi * pyi)).log2();
            }
        }
    }
    Some(mi.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-9;

    #[test]
    fn entropy_known_values() {
        // Fair coin → 1 bit; fair 4-way → 2 bits; certain outcome → 0.
        assert!((entropy(&[0.5, 0.5]).unwrap() - 1.0).abs() < EPS);
        assert!((entropy(&[0.25; 4]).unwrap() - 2.0).abs() < EPS);
        assert!(entropy(&[1.0, 0.0, 0.0]).unwrap().abs() < EPS);
        // Unnormalized counts work too.
        assert!((entropy_from_counts(&[1, 1, 1, 1]).unwrap() - 2.0).abs() < EPS);
    }

    #[test]
    fn kl_is_zero_for_equal_and_positive_otherwise() {
        assert!(kl_divergence(&[0.5, 0.5], &[0.5, 0.5]).unwrap().abs() < EPS);
        assert!(kl_divergence(&[0.9, 0.1], &[0.5, 0.5]).unwrap() > 0.0);
        // Infinite divergence (q has no support where p does) → refuse.
        assert!(kl_divergence(&[0.5, 0.5], &[1.0, 0.0]).is_none());
    }

    #[test]
    fn mutual_information_detects_dependence() {
        // y = x → I(X;Y) = H(X) = 1 bit for a balanced binary x.
        let x = [0usize, 0, 1, 1, 0, 1, 0, 1];
        let y = x;
        assert!((mutual_information_discrete(&x, &y).unwrap() - 1.0).abs() < EPS);
        // Independent x,y → MI ≈ 0.
        let xi = [0usize, 0, 1, 1, 0, 0, 1, 1];
        let yi = [0usize, 1, 0, 1, 0, 1, 0, 1];
        assert!(mutual_information_discrete(&xi, &yi).unwrap() < 1e-9);
    }

    #[test]
    fn cross_entropy_decomposes() {
        // H(p,q) = H(p) + D(p||q).
        let p = [0.7, 0.3];
        let q = [0.5, 0.5];
        let ce = cross_entropy(&p, &q).unwrap();
        let h = entropy(&p).unwrap();
        let d = kl_divergence(&p, &q).unwrap();
        assert!((ce - (h + d)).abs() < EPS);
    }

    #[test]
    fn guards() {
        assert_eq!(entropy(&[]), None);
        assert_eq!(kl_divergence(&[0.5, 0.5], &[1.0]), None);
        assert_eq!(mutual_information_discrete(&[0, 1], &[0]), None);
    }
}
