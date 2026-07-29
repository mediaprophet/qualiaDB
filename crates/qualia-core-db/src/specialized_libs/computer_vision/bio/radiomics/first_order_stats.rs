//! First-order intensity statistics on a gray-level ROI.
//!
//! Operates on a flat `f32` intensity slice (caller supplies ROI samples).
//! Returns a fixed struct — no heap on the evaluation path.

/// Shared radiomics error for the radiomics submodule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadiomicsError {
    EmptyInput,
    InvalidParameter,
    BufferTooSmall,
    DimensionMismatch,
}

impl core::fmt::Display for RadiomicsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "empty input"),
            Self::InvalidParameter => write!(f, "invalid parameter"),
            Self::BufferTooSmall => write!(f, "output buffer too small"),
            Self::DimensionMismatch => write!(f, "dimension mismatch"),
        }
    }
}

/// First-order radiomics features (IBSI-style subset).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirstOrderStats {
    pub mean: f64,
    pub std: f64,
    pub skewness: f64,
    /// Excess kurtosis (Fisher; normal distribution → 0).
    pub kurtosis: f64,
    /// Sum of squares of intensities.
    pub energy: f64,
    /// Shannon entropy of the intensity histogram (nats).
    pub entropy: f64,
    pub count: usize,
    pub min: f64,
    pub max: f64,
}

/// Default histogram bins for entropy when not specified.
pub const DEFAULT_HIST_BINS: usize = 32;

/// Compute first-order stats over `samples` (gray / intensity ROI values).
///
/// Uses 32 histogram bins for entropy. Empty input → `Err(EmptyInput)`.
pub fn first_order_stats(samples: &[f32]) -> Result<FirstOrderStats, RadiomicsError> {
    first_order_stats_with_bins(samples, DEFAULT_HIST_BINS)
}

/// Same as [`first_order_stats`] with explicit entropy histogram bin count.
pub fn first_order_stats_with_bins(
    samples: &[f32],
    hist_bins: usize,
) -> Result<FirstOrderStats, RadiomicsError> {
    if samples.is_empty() {
        return Err(RadiomicsError::EmptyInput);
    }
    let n = samples.len();
    let nf = n as f64;

    let mut sum = 0.0f64;
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for &v in samples {
        let x = v as f64;
        sum += x;
        if x < min_v {
            min_v = x;
        }
        if x > max_v {
            max_v = x;
        }
    }
    let mean = sum / nf;

    let mut m2 = 0.0f64;
    let mut m3 = 0.0f64;
    let mut m4 = 0.0f64;
    let mut energy = 0.0f64;
    for &v in samples {
        let x = v as f64;
        let d = x - mean;
        let d2 = d * d;
        m2 += d2;
        m3 += d2 * d;
        m4 += d2 * d2;
        energy += x * x;
    }

    let variance = m2 / nf;
    let std = variance.sqrt();

    let (skewness, kurtosis) = if std > 1e-15 {
        let s3 = std * std * std;
        let s4 = s3 * std;
        (m3 / nf / s3, m4 / nf / s4 - 3.0)
    } else {
        (0.0, 0.0)
    };

    let bins = hist_bins.clamp(2, 256);
    let entropy = hist_entropy(samples, min_v, max_v, bins);

    Ok(FirstOrderStats {
        mean,
        std,
        skewness,
        kurtosis,
        energy,
        entropy,
        count: n,
        min: min_v,
        max: max_v,
    })
}

fn hist_entropy(samples: &[f32], min_v: f64, max_v: f64, bins: usize) -> f64 {
    let mut counts = [0u32; 256];
    let span = (max_v - min_v).max(1e-15);
    for &v in samples {
        let t = ((v as f64 - min_v) / span * (bins as f64)).floor() as isize;
        let idx = t.clamp(0, (bins as isize) - 1) as usize;
        counts[idx] += 1;
    }
    let n = samples.len() as f64;
    let mut h = 0.0f64;
    for i in 0..bins {
        let c = counts[i];
        if c > 0 {
            let p = c as f64 / n;
            h -= p * p.ln();
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_errors() {
        assert_eq!(
            first_order_stats(&[]).unwrap_err(),
            RadiomicsError::EmptyInput
        );
    }

    #[test]
    fn constant_roi() {
        let s = [5.0f32; 16];
        let st = first_order_stats(&s).unwrap();
        assert!((st.mean - 5.0).abs() < 1e-12);
        assert!(st.std < 1e-12);
        assert!((st.energy - 16.0 * 25.0).abs() < 1e-9);
        assert!(st.entropy < 1e-12);
        assert_eq!(st.count, 16);
    }

    #[test]
    fn two_value_mean_std() {
        let s = [0.0f32, 2.0, 0.0, 2.0];
        let st = first_order_stats(&s).unwrap();
        assert!((st.mean - 1.0).abs() < 1e-12);
        assert!((st.std - 1.0).abs() < 1e-12);
        assert!(st.entropy > 0.0);
    }

    #[test]
    fn min_max_tracked() {
        let s = [3.0f32, -1.0, 7.5, 0.0];
        let st = first_order_stats(&s).unwrap();
        assert!((st.min - (-1.0)).abs() < 1e-12);
        assert!((st.max - 7.5).abs() < 1e-12);
    }
}
