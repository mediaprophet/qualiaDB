//! Self-similarity matrix (SSM) from a per-frame feature matrix.
//!
//! An SSM is the frames×frames Gram-like matrix `S[i,j] = sim(x_i, x_j)` where
//! `x_i` is the feature vector of frame `i`. Homogeneous musical sections form
//! high-similarity square blocks on the diagonal; section boundaries appear as
//! the seams between blocks. This is the substrate the novelty curve then reads
//! (contrast the energy-hysteresis heuristic previously in `music.rs`).

use crate::types::AudioError;

/// Compute the cosine self-similarity matrix of a per-frame feature matrix.
///
/// `features` is a row-major `n_frames × n_dims` matrix: frame `i` occupies
/// `features[i*n_dims .. i*n_dims + n_dims]`. These frames are provided by the
/// caller (e.g. from [`crate::features::mel::mfcc`] or chroma / tonal features)
/// — this function never recomputes features.
///
/// The similarity is the cosine of the angle between two feature vectors,
/// `S[i,j] = (x_i · x_j) / (‖x_i‖ ‖x_j‖)`, in `[-1, 1]` (clamped against
/// floating-point overshoot). When either frame has zero norm its similarity to
/// every other frame is defined as `0.0`. The matrix is symmetric with a unit
/// diagonal for non-zero frames.
///
/// # Caller-buffer contract (zero internal frames² allocation)
/// The full `n_frames × n_frames` result is written, row-major, into the
/// caller-provided `out` buffer (`out[i*n_frames + j] = S[i,j]`). This function
/// performs **no** heap allocation — the inherently O(frames²) storage is the
/// caller's `out`, and the O(frames²·dims) compute uses only scalar locals.
/// `out.len()` must be at least `n_frames * n_frames`.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `n_frames == 0`, `n_dims == 0`, or
///   `features.len() < n_frames * n_dims`.
/// - [`AudioError::OutputBufferTooSmall`] if `out.len() < n_frames * n_frames`.
pub fn self_similarity(
    features: &[f32],
    n_frames: usize,
    n_dims: usize,
    out: &mut [f32],
) -> Result<(), AudioError> {
    if n_frames == 0 || n_dims == 0 {
        return Err(AudioError::InvalidParameter);
    }
    let feat_needed = n_frames
        .checked_mul(n_dims)
        .ok_or(AudioError::InvalidParameter)?;
    if features.len() < feat_needed {
        return Err(AudioError::InvalidParameter);
    }
    let out_needed = n_frames
        .checked_mul(n_frames)
        .ok_or(AudioError::InvalidParameter)?;
    if out.len() < out_needed {
        return Err(AudioError::OutputBufferTooSmall);
    }

    // Symmetric: compute the upper triangle (j >= i) and mirror it. Norms are
    // recomputed per pair rather than cached, keeping the path strictly
    // zero-heap (a per-frame norm cache would be an O(frames) allocation).
    for i in 0..n_frames {
        let row_i = &features[i * n_dims..i * n_dims + n_dims];
        let norm_i = l2_norm(row_i);
        for j in i..n_frames {
            let sim = if norm_i == 0.0 {
                0.0f32
            } else {
                let row_j = &features[j * n_dims..j * n_dims + n_dims];
                let norm_j = l2_norm(row_j);
                if norm_j == 0.0 {
                    0.0
                } else {
                    let mut dot = 0.0f64;
                    for (a, b) in row_i.iter().zip(row_j.iter()) {
                        dot += (*a as f64) * (*b as f64);
                    }
                    (dot / (norm_i * norm_j)).clamp(-1.0, 1.0) as f32
                }
            };
            out[i * n_frames + j] = sim;
            out[j * n_frames + i] = sim;
        }
    }
    Ok(())
}

/// L2 norm of a feature row (f64 accumulation for stability).
#[inline]
fn l2_norm(row: &[f32]) -> f64 {
    let mut acc = 0.0f64;
    for v in row {
        acc += (*v as f64) * (*v as f64);
    }
    acc.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a two-section feature matrix: first `half` frames are vector A,
    /// the remainder are the orthogonal vector B.
    fn two_section(n_frames: usize, half: usize) -> (Vec<f32>, usize) {
        let n_dims = 2;
        let mut f = vec![0.0f32; n_frames * n_dims];
        for frame in 0..n_frames {
            if frame < half {
                f[frame * n_dims] = 1.0; // A = (1, 0)
            } else {
                f[frame * n_dims + 1] = 1.0; // B = (0, 1)
            }
        }
        (f, n_dims)
    }

    /// Golden: two orthogonal sections give unit within-block similarity and
    /// zero cross-block similarity, with a unit diagonal.
    #[test]
    fn golden_two_blocks() {
        let n = 8;
        let half = 4;
        let (feat, dims) = two_section(n, half);
        let mut ssm = vec![-9.0f32; n * n];
        self_similarity(&feat, n, dims, &mut ssm).expect("ssm");

        // Diagonal is 1.
        for i in 0..n {
            assert!(
                (ssm[i * n + i] - 1.0).abs() < 1e-6,
                "diag {i}={}",
                ssm[i * n + i]
            );
        }
        // Within block A (0..4) and within block B (4..8): similarity 1.
        for i in 0..half {
            for j in 0..half {
                assert!((ssm[i * n + j] - 1.0).abs() < 1e-6, "A[{i},{j}]");
            }
        }
        for i in half..n {
            for j in half..n {
                assert!((ssm[i * n + j] - 1.0).abs() < 1e-6, "B[{i},{j}]");
            }
        }
        // Across blocks: similarity 0.
        for i in 0..half {
            for j in half..n {
                assert!(
                    ssm[i * n + j].abs() < 1e-6,
                    "cross[{i},{j}]={}",
                    ssm[i * n + j]
                );
                assert!(ssm[j * n + i].abs() < 1e-6, "cross[{j},{i}]");
            }
        }
    }

    /// Symmetry holds for an arbitrary (non-orthogonal) matrix.
    #[test]
    fn symmetric_and_diagonal_unit() {
        let n = 5;
        let dims = 3;
        // Distinct, non-zero rows.
        let feat: Vec<f32> = (0..n * dims)
            .map(|k| (k as f32 * 0.37).sin() + 1.5)
            .collect();
        let mut ssm = vec![0.0f32; n * n];
        self_similarity(&feat, n, dims, &mut ssm).expect("ssm");
        for i in 0..n {
            assert!((ssm[i * n + i] - 1.0).abs() < 1e-5, "diag {i}");
            for j in 0..n {
                assert!(
                    (ssm[i * n + j] - ssm[j * n + i]).abs() < 1e-6,
                    "asym at {i},{j}"
                );
                assert!(ssm[i * n + j] <= 1.0 + 1e-6 && ssm[i * n + j] >= -1.0 - 1e-6);
            }
        }
    }

    /// A zero-norm frame is similar to nothing (defined as 0).
    #[test]
    fn zero_frame_similarity_is_zero() {
        let n = 3;
        let dims = 2;
        // frame 1 is the zero vector.
        let feat = [1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0];
        let mut ssm = vec![9.0f32; n * n];
        self_similarity(&feat, n, dims, &mut ssm).expect("ssm");
        for j in 0..n {
            assert!(ssm[1 * n + j].abs() < 1e-6, "row1 col{j}");
            assert!(ssm[j * n + 1].abs() < 1e-6, "col1 row{j}");
        }
    }

    #[test]
    fn rejects_bad_params() {
        let mut out = [0.0f32; 4];
        assert_eq!(
            self_similarity(&[1.0, 2.0], 0, 2, &mut out),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            self_similarity(&[1.0, 2.0], 2, 0, &mut out),
            Err(AudioError::InvalidParameter)
        );
        // features too short: need 2*2=4, have 3.
        assert_eq!(
            self_similarity(&[1.0, 2.0, 3.0], 2, 2, &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_small_output() {
        let feat = [1.0f32, 0.0, 0.0, 1.0];
        let mut out = [0.0f32; 3]; // need 2*2 = 4
        assert_eq!(
            self_similarity(&feat, 2, 2, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
