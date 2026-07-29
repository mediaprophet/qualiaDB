//! Foote novelty curve: a checkerboard kernel correlated along the SSM diagonal.
//!
//! Sliding a `L × L` checkerboard kernel down the main diagonal of a
//! self-similarity matrix scores, at each frame, how much the local structure
//! looks like the seam between two homogeneous blocks. The kernel's sign
//! pattern is
//!
//! ```text
//!   + -
//!   - +
//! ```
//!
//! so within a homogeneous region (all four quadrants equally similar) the
//! contributions cancel to ~0, while at a boundary the two same-block quadrants
//! (`+`) are high and the two cross-block quadrants (`-`) are low, producing a
//! sharp positive peak.

use crate::types::AudioError;

/// Correlate a checkerboard kernel along the diagonal of `ssm`, producing a
/// per-frame novelty curve.
///
/// `ssm` is a row-major `n_frames × n_frames` self-similarity matrix (as
/// produced by [`super::ssm::self_similarity`]). `kernel_size` is the full width
/// `L` of the (`L × L`) checkerboard kernel and **must be even and ≥ 2**; each
/// half-quadrant then spans `M = L/2` frames.
///
/// The novelty at frame `n` is `Σ_{a,b} C(a,b) · S[n+a, n+b]` over kernel
/// offsets `a, b ∈ [-M, M)`, where the checkerboard sign is `+1` when `a` and
/// `b` lie on the same side of the split and `-1` otherwise. The diagonal is
/// centred at `(n, n)`. Kernel taps that fall outside `[0, n_frames)` are
/// treated as zero (zero-padding), so a full-length curve of `n_frames` values
/// is produced.
///
/// # Caller-buffer contract (zero-heap)
/// The `n_frames`-long result is written into the caller-provided `out`;
/// `out.len()` must be at least `n_frames`. No heap allocation is performed.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `n_frames == 0`, `kernel_size == 0`,
///   `kernel_size` is odd, or `ssm.len() < n_frames * n_frames`.
/// - [`AudioError::OutputBufferTooSmall`] if `out.len() < n_frames`.
pub fn novelty_curve(
    ssm: &[f32],
    n_frames: usize,
    kernel_size: usize,
    out: &mut [f32],
) -> Result<(), AudioError> {
    if n_frames == 0 || kernel_size == 0 || kernel_size % 2 != 0 {
        return Err(AudioError::InvalidParameter);
    }
    let ssm_needed = n_frames
        .checked_mul(n_frames)
        .ok_or(AudioError::InvalidParameter)?;
    if ssm.len() < ssm_needed {
        return Err(AudioError::InvalidParameter);
    }
    if out.len() < n_frames {
        return Err(AudioError::OutputBufferTooSmall);
    }

    let m = (kernel_size / 2) as isize;
    let nf = n_frames as isize;

    for n in 0..n_frames {
        let center = n as isize;
        let mut acc = 0.0f64;
        let mut a = -m;
        while a < m {
            let row = center + a;
            if row < 0 || row >= nf {
                a += 1;
                continue;
            }
            let a_neg = a < 0;
            let mut b = -m;
            while b < m {
                let col = center + b;
                if col < 0 || col >= nf {
                    b += 1;
                    continue;
                }
                let b_neg = b < 0;
                // Checkerboard: same side of the split => +, opposite => -.
                let sign = if a_neg == b_neg { 1.0f64 } else { -1.0f64 };
                acc += sign * ssm[(row as usize) * n_frames + (col as usize)] as f64;
                b += 1;
            }
            a += 1;
        }
        out[n] = acc as f32;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::ssm::self_similarity;
    use super::*;

    fn two_section_ssm(n_frames: usize, half: usize) -> Vec<f32> {
        let dims = 2;
        let mut f = vec![0.0f32; n_frames * dims];
        for frame in 0..n_frames {
            if frame < half {
                f[frame * dims] = 1.0;
            } else {
                f[frame * dims + 1] = 1.0;
            }
        }
        let mut ssm = vec![0.0f32; n_frames * n_frames];
        self_similarity(&f, n_frames, dims, &mut ssm).expect("ssm");
        ssm
    }

    /// Golden: two orthogonal sections of 4 frames each, kernel size 4.
    /// The novelty curve must peak at the boundary frame (index 4) with the
    /// analytically derived value 8.0, and be ~0 in the block interior.
    #[test]
    fn golden_peaks_at_boundary() {
        let n = 8;
        let ssm = two_section_ssm(n, 4);
        let mut nov = vec![-1.0f32; n];
        novelty_curve(&ssm, n, 4, &mut nov).expect("novelty");

        // Analytical curve derived by hand: [4, 1, 0, 2, 8, 2, 0, 1].
        // (Edge values differ low vs high side because the zero-padded kernel
        // overhangs asymmetrically: frame 0 sees rows {0,1}, frame 7 sees only
        // {5,6,7}. The boundary peak at frame 4 is the strict interior maximum.)
        let expect = [4.0f32, 1.0, 0.0, 2.0, 8.0, 2.0, 0.0, 1.0];
        for (i, (&got, &want)) in nov.iter().zip(expect.iter()).enumerate() {
            assert!((got - want).abs() < 1e-5, "n[{i}]={got} want {want}");
        }

        // The strict interior maximum is at the true boundary, frame 4.
        let mut best = 0usize;
        for i in 1..n - 1 {
            if nov[i] > nov[best] {
                best = i;
            }
        }
        assert_eq!(best, 4, "peak frame");
        assert!((nov[4] - 8.0).abs() < 1e-5);
        // Homogeneous interior (frame 2) is flat.
        assert!(nov[2].abs() < 1e-6, "interior novelty {}", nov[2]);
    }

    /// A larger split (12 frames, boundary at 6, kernel 4) still peaks exactly
    /// on the boundary frame.
    #[test]
    fn peak_tracks_midpoint() {
        let n = 12;
        let ssm = two_section_ssm(n, 6);
        let mut nov = vec![0.0f32; n];
        novelty_curve(&ssm, n, 4, &mut nov).expect("novelty");
        let mut best = 1usize;
        for i in 1..n - 1 {
            if nov[i] > nov[best] {
                best = i;
            }
        }
        assert_eq!(best, 6, "peak at boundary; curve={nov:?}");
    }

    #[test]
    fn rejects_bad_params() {
        let ssm = vec![0.0f32; 16];
        let mut out = [0.0f32; 4];
        assert_eq!(
            novelty_curve(&ssm, 0, 2, &mut out),
            Err(AudioError::InvalidParameter)
        );
        // odd kernel
        assert_eq!(
            novelty_curve(&ssm, 4, 3, &mut out),
            Err(AudioError::InvalidParameter)
        );
        // zero kernel
        assert_eq!(
            novelty_curve(&ssm, 4, 0, &mut out),
            Err(AudioError::InvalidParameter)
        );
        // ssm too small: need 4*4=16.
        assert_eq!(
            novelty_curve(&[0.0f32; 15], 4, 2, &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_small_output() {
        let ssm = vec![0.0f32; 16];
        let mut out = [0.0f32; 3]; // need 4
        assert_eq!(
            novelty_curve(&ssm, 4, 2, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
