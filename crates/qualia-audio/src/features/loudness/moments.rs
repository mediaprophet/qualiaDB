//! Statistical moments of a signal: mean, variance, skewness, kurtosis.
//!
//! Population (biased, `/n`) moments over a mono block. Skewness and kurtosis
//! are the standardised 3rd and 4th moments; kurtosis is **Pearson** (a normal
//! distribution → 3.0, subtract 3 for excess kurtosis). Zero-heap (two scalar
//! passes, no allocation).

/// The four leading moments of a block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Moments {
    /// Arithmetic mean.
    pub mean: f32,
    /// Population variance (`E[(x-μ)^2]`).
    pub variance: f32,
    /// Standardised skewness (`E[(x-μ)^3] / σ^3`); `0` for symmetric data.
    pub skewness: f32,
    /// Pearson kurtosis (`E[(x-μ)^4] / σ^4`); `3.0` for a normal distribution.
    pub kurtosis: f32,
}

impl Moments {
    /// Zero moments (used for degenerate/empty input).
    pub const ZERO: Moments = Moments {
        mean: 0.0,
        variance: 0.0,
        skewness: 0.0,
        kurtosis: 0.0,
    };
}

/// Compute the mean, variance, skewness and Pearson kurtosis of `samples`.
///
/// Returns [`Moments::ZERO`] for an empty block. For a constant block
/// (variance `0`) skewness and kurtosis are reported as `0` (undefined ratio).
pub fn moments(samples: &[f32]) -> Moments {
    let n = samples.len();
    if n == 0 {
        return Moments::ZERO;
    }
    let nf = n as f64;

    let mut sum = 0.0f64;
    for &x in samples {
        sum += x as f64;
    }
    let mean = sum / nf;

    let (mut m2, mut m3, mut m4) = (0.0f64, 0.0f64, 0.0f64);
    for &x in samples {
        let d = x as f64 - mean;
        let d2 = d * d;
        m2 += d2;
        m3 += d2 * d;
        m4 += d2 * d2;
    }
    m2 /= nf;
    m3 /= nf;
    m4 /= nf;

    let (skewness, kurtosis) = if m2 > 0.0 {
        let sigma = m2.sqrt();
        ((m3 / (sigma * sigma * sigma)) as f32, (m4 / (m2 * m2)) as f32)
    } else {
        (0.0, 0.0)
    };

    Moments {
        mean: mean as f32,
        variance: m2 as f32,
        skewness,
        kurtosis,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramp_matches_closed_form() {
        // [1,2,3,4,5]: mean 3, population var 2, skew 0 (symmetric),
        // kurtosis = (Σ(x-3)^4/5)/var^2 = 6.8/4 = 1.7.
        let m = moments(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!((m.mean - 3.0).abs() < 1e-5, "mean {}", m.mean);
        assert!((m.variance - 2.0).abs() < 1e-5, "var {}", m.variance);
        assert!(m.skewness.abs() < 1e-5, "skew {}", m.skewness);
        assert!((m.kurtosis - 1.7).abs() < 1e-5, "kurt {}", m.kurtosis);
    }

    #[test]
    fn two_point_symmetric() {
        // ±1: mean 0, var 1, skew 0, Pearson kurtosis 1.0 (excess −2).
        let m = moments(&[-1.0, 1.0, -1.0, 1.0]);
        assert!(m.mean.abs() < 1e-6);
        assert!((m.variance - 1.0).abs() < 1e-6);
        assert!(m.skewness.abs() < 1e-6);
        assert!((m.kurtosis - 1.0).abs() < 1e-6, "kurt {}", m.kurtosis);
    }

    #[test]
    fn positively_skewed_data_has_positive_skew() {
        // Long right tail.
        let m = moments(&[0.0, 0.0, 0.0, 0.0, 10.0]);
        assert!(m.skewness > 1.0, "skew {}", m.skewness);
    }

    #[test]
    fn constant_and_empty_are_well_defined() {
        let c = moments(&[4.0; 16]);
        assert!((c.mean - 4.0).abs() < 1e-6);
        assert_eq!(c.variance, 0.0);
        assert_eq!(c.skewness, 0.0);
        assert_eq!(c.kurtosis, 0.0);
        assert_eq!(moments(&[]), Moments::ZERO);
    }
}
