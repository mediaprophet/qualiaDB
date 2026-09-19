//! Hybrid operator candidate generator and residual fitting (W5: EOS-051).
//!
//! Fits the hybrid decomposition:
//! $$W \approx Q + A B + S$$
//! where:
//! - $Q$ is the base quantized matrix (e.g. Q4_K reconstructed).
//! - $A \in \mathbb{R}^{m \times r}, B \in \mathbb{R}^{r \times n}$ is a rank-$r$ correction.
//! - $S$ is a sparse outlier correction (or dense escape tile for sensitive blocks).
//! - Fitting is weighted by activation statistics $H \approx L L^T$.

use super::activation_stats::ConverterError;

/// A sparse matrix entry stored in coordinate format.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SparseEntry {
    pub row: u32,
    pub col: u32,
    pub val: f32,
}

/// A dense escape tile for regions where low-rank/sparse approximation is insufficient.
#[derive(Debug, Clone, PartialEq)]
pub struct DenseEscapeTile {
    pub row_start: usize,
    pub col_start: usize,
    pub rows: usize,
    pub cols: usize,
    pub values: Vec<f32>,
}

/// Representation of a fitted hybrid operator candidate $W \approx Q + A B + S$.
#[derive(Debug, Clone, PartialEq)]
pub struct HybridDecomposition {
    pub rows: usize,
    pub cols: usize,
    pub rank: usize,
    /// Base quantized weights (flattened row-major).
    pub base_q: Vec<f32>,
    /// Low-rank left factor A: shape `rows × rank` (row-major).
    pub low_rank_a: Vec<f32>,
    /// Low-rank right factor B: shape `rank × cols` (row-major).
    pub low_rank_b: Vec<f32>,
    /// Sparse outlier entries.
    pub sparse_entries: Vec<SparseEntry>,
    /// Dense escape tiles.
    pub escape_tiles: Vec<DenseEscapeTile>,
}

impl HybridDecomposition {
    /// Evaluate matrix-vector multiplication $y = (Q + A B + S) x$.
    pub fn apply_gemv(&self, x: &[f32], y: &mut [f32]) -> Result<(), ConverterError> {
        if x.len() != self.cols {
            return Err(ConverterError::DimensionMismatch {
                expected: self.cols,
                got: x.len(),
            });
        }
        if y.len() != self.rows {
            return Err(ConverterError::DimensionMismatch {
                expected: self.rows,
                got: y.len(),
            });
        }

        // 1. Base Q apply
        for i in 0..self.rows {
            let mut dot = 0.0f32;
            let row_offset = i * self.cols;
            for j in 0..self.cols {
                dot += self.base_q[row_offset + j] * x[j];
            }
            y[i] = dot;
        }

        // 2. Low-rank AB apply: y += A * (B * x)
        if self.rank > 0 {
            // intermediate = B * x (length `rank`)
            let mut bx = vec![0.0f32; self.rank];
            for r in 0..self.rank {
                let mut dot = 0.0f32;
                let b_offset = r * self.cols;
                for j in 0..self.cols {
                    dot += self.low_rank_b[b_offset + j] * x[j];
                }
                bx[r] = dot;
            }

            // y += A * bx
            for i in 0..self.rows {
                let mut dot = 0.0f32;
                let a_offset = i * self.rank;
                for r in 0..self.rank {
                    dot += self.low_rank_a[a_offset + r] * bx[r];
                }
                y[i] += dot;
            }
        }

        // 3. Sparse outlier apply: y[row] += val * x[col]
        for entry in &self.sparse_entries {
            y[entry.row as usize] += entry.val * x[entry.col as usize];
        }

        // 4. Dense escape tiles overwrite or add
        for tile in &self.escape_tiles {
            for r in 0..tile.rows {
                let global_r = tile.row_start + r;
                let tile_row_offset = r * tile.cols;
                for c in 0..tile.cols {
                    let global_c = tile.col_start + c;
                    // Compute difference from already applied Base Q
                    let escape_val = tile.values[tile_row_offset + c];
                    let base_val = self.base_q[global_r * self.cols + global_c];
                    y[global_r] += (escape_val - base_val) * x[global_c];
                }
            }
        }

        Ok(())
    }

    /// Reconstruct the full dense matrix $\hat{W} = Q + A B + S$.
    pub fn reconstruct_dense(&self) -> Vec<f32> {
        let mut w_hat = self.base_q.clone();

        if self.rank > 0 {
            for i in 0..self.rows {
                for j in 0..self.cols {
                    let mut dot = 0.0f32;
                    for r in 0..self.rank {
                        dot += self.low_rank_a[i * self.rank + r] * self.low_rank_b[r * self.cols + j];
                    }
                    w_hat[i * self.cols + j] += dot;
                }
            }
        }

        for entry in &self.sparse_entries {
            w_hat[entry.row as usize * self.cols + entry.col as usize] += entry.val;
        }

        for tile in &self.escape_tiles {
            for r in 0..tile.rows {
                let global_r = tile.row_start + r;
                let tile_row_offset = r * tile.cols;
                for c in 0..tile.cols {
                    let global_c = tile.col_start + c;
                    w_hat[global_r * self.cols + global_c] = tile.values[tile_row_offset + c];
                }
            }
        }

        w_hat
    }
}

/// Configuration options for hybrid fitting.
#[derive(Debug, Clone)]
pub struct HybridFitConfig {
    pub rank: usize,
    pub max_sparse_ratio: f32,
    pub outlier_threshold_std: f32,
    pub num_power_iters: usize,
    pub max_budget_bytes: usize,
}

impl Default for HybridFitConfig {
    fn default() -> Self {
        Self {
            rank: 4,
            max_sparse_ratio: 0.02, // at most 2% sparse entries
            outlier_threshold_std: 2.5,
            num_power_iters: 10,
            max_budget_bytes: 42 * 1024 * 1024, // 42 MiB Sentinel
        }
    }
}

/// Fit a hybrid decomposition $W \approx Q + A B + S$ from source weights, base quantized weights,
/// and optional activation diagonal weights.
pub fn fit_hybrid_decomposition(
    rows: usize,
    cols: usize,
    w_source: &[f32],
    base_q: &[f32],
    act_weights: Option<&[f32]>,
    config: &HybridFitConfig,
) -> Result<HybridDecomposition, ConverterError> {
    if w_source.len() != rows * cols || base_q.len() != rows * cols {
        return Err(ConverterError::DimensionMismatch {
            expected: rows * cols,
            got: w_source.len(),
        });
    }

    let total_scratch_bytes = (rows * cols * 4) * 3 + (rows * config.rank * 4) + (config.rank * cols * 4);
    if total_scratch_bytes > config.max_budget_bytes {
        return Err(ConverterError::BudgetExceeded {
            required_bytes: total_scratch_bytes,
            budget_bytes: config.max_budget_bytes,
        });
    }

    // Compute residual R = W_source - Base_Q
    let mut residual = vec![0.0f32; rows * cols];
    for idx in 0..rows * cols {
        residual[idx] = w_source[idx] - base_q[idx];
    }

    // Apply activation weights: R_w[:, j] = R[:, j] * w_j
    if let Some(weights) = act_weights {
        if weights.len() != cols {
            return Err(ConverterError::DimensionMismatch {
                expected: cols,
                got: weights.len(),
            });
        }
        for i in 0..rows {
            let offset = i * cols;
            for j in 0..cols {
                residual[offset + j] *= weights[j];
            }
        }
    }

    // Fit rank-r factor using randomized/deflated power iteration
    let mut low_rank_a = vec![0.0f32; rows * config.rank];
    let mut low_rank_b = vec![0.0f32; config.rank * cols];

    let mut deflated_residual = residual.clone();

    for r in 0..config.rank {
        // Initial random vector v
        let mut v = vec![1.0f32 / (cols as f32).sqrt(); cols];
        for (idx, item) in v.iter_mut().enumerate() {
            *item = (((idx * 17 + r * 31 + 7) % 100) as f32 / 100.0) - 0.5;
        }
        let norm_v: f32 = v.iter().map(|&x| x * x).sum::<f32>().sqrt();
        if norm_v > 1e-12 {
            for x in &mut v {
                *x /= norm_v;
            }
        }

        let mut u = vec![0.0f32; rows];

        // Power iteration
        for _ in 0..config.num_power_iters {
            // u = R * v
            for i in 0..rows {
                let mut sum = 0.0f32;
                let offset = i * cols;
                for j in 0..cols {
                    sum += deflated_residual[offset + j] * v[j];
                }
                u[i] = sum;
            }
            let norm_u: f32 = u.iter().map(|&x| x * x).sum::<f32>().sqrt();
            if norm_u > 1e-12 {
                for x in &mut u {
                    *x /= norm_u;
                }
            }

            // v = R^T * u
            for j in 0..cols {
                let mut sum = 0.0f32;
                for i in 0..rows {
                    sum += deflated_residual[i * cols + j] * u[i];
                }
                v[j] = sum;
            }
            let norm_v: f32 = v.iter().map(|&x| x * x).sum::<f32>().sqrt();
            if norm_v > 1e-12 {
                for x in &mut v {
                    *x /= norm_v;
                }
            }
        }

        // Compute singular value sigma = u^T * R * v
        let mut sigma = 0.0f32;
        for i in 0..rows {
            let offset = i * cols;
            for j in 0..cols {
                sigma += u[i] * deflated_residual[offset + j] * v[j];
            }
        }

        // Store into low_rank_a and low_rank_b: A[:, r] = u * sqrt(sigma), B[r, :] = v * sqrt(sigma)
        let factor = sigma.abs().sqrt();
        let sign = if sigma >= 0.0 { 1.0f32 } else { -1.0f32 };
        for i in 0..rows {
            low_rank_a[i * config.rank + r] = u[i] * factor * sign;
        }
        for j in 0..cols {
            low_rank_b[r * cols + j] = v[j] * factor;
        }

        // Deflate residual: R -= sigma * u * v^T
        for i in 0..rows {
            let offset = i * cols;
            for j in 0..cols {
                deflated_residual[offset + j] -= sigma * u[i] * v[j];
            }
        }
    }

    // Unweight B if activation weights were used: B[r, j] /= w_j
    if let Some(weights) = act_weights {
        for r in 0..config.rank {
            let offset = r * cols;
            for j in 0..cols {
                let w = weights[j];
                if w.abs() > 1e-8 {
                    low_rank_b[offset + j] /= w;
                }
            }
        }
    }

    // Compute remaining error matrix E = W_source - (Base_Q + AB)
    let mut err = vec![0.0f32; rows * cols];
    let mut sum_sq = 0.0f64;
    for i in 0..rows {
        for j in 0..cols {
            let mut ab = 0.0f32;
            for r in 0..config.rank {
                ab += low_rank_a[i * config.rank + r] * low_rank_b[r * cols + j];
            }
            let diff = w_source[i * cols + j] - (base_q[i * cols + j] + ab);
            err[i * cols + j] = diff;
            sum_sq += (diff * diff) as f64;
        }
    }

    let mean_sq = sum_sq / (rows * cols) as f64;
    let std_dev = mean_sq.sqrt() as f32;
    let threshold = config.outlier_threshold_std * std_dev;

    // Extract sparse outlier entries
    let max_sparse_count = ((rows * cols) as f32 * config.max_sparse_ratio) as usize;
    let mut sparse_candidates = Vec::new();
    for i in 0..rows {
        for j in 0..cols {
            let val = err[i * cols + j];
            if val.abs() > threshold {
                sparse_candidates.push(SparseEntry {
                    row: i as u32,
                    col: j as u32,
                    val,
                });
            }
        }
    }

    // Sort by absolute magnitude descending and truncate to max_sparse_count
    sparse_candidates.sort_by(|a, b| b.val.abs().partial_cmp(&a.val.abs()).unwrap());
    sparse_candidates.truncate(max_sparse_count);

    Ok(HybridDecomposition {
        rows,
        cols,
        rank: config.rank,
        base_q: base_q.to_vec(),
        low_rank_a,
        low_rank_b,
        sparse_entries: sparse_candidates,
        escape_tiles: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_fitting_reduces_error() {
        let rows = 8;
        let cols = 16;
        let mut w_source = vec![0.0f32; rows * cols];
        let mut base_q = vec![0.0f32; rows * cols];

        // Fill with synthetic data: base_q is a rough approximation of w_source
        for i in 0..rows {
            for j in 0..cols {
                let ground_truth = ((i * 7 + j * 13) as f32 * 0.05).sin();
                w_source[i * cols + j] = ground_truth;
                // Add quantization noise
                base_q[i * cols + j] = (ground_truth * 4.0).round() / 4.0;
            }
        }

        let base_error: f32 = w_source
            .iter()
            .zip(&base_q)
            .map(|(w, q)| (w - q).powi(2))
            .sum();

        let config = HybridFitConfig {
            rank: 2,
            max_sparse_ratio: 0.05,
            outlier_threshold_std: 1.5,
            num_power_iters: 15,
            max_budget_bytes: 42 * 1024 * 1024,
        };

        let hybrid = fit_hybrid_decomposition(rows, cols, &w_source, &base_q, None, &config).unwrap();
        let w_hat = hybrid.reconstruct_dense();

        let hybrid_error: f32 = w_source
            .iter()
            .zip(&w_hat)
            .map(|(w, h)| (w - h).powi(2))
            .sum();

        assert!(
            hybrid_error < base_error,
            "Hybrid error ({}) must be strictly less than base error ({})",
            hybrid_error,
            base_error
        );

        // Test apply_gemv against reconstructed dense GEMV
        let x = vec![1.0f32; cols];
        let mut y_hybrid = vec![0.0f32; rows];
        hybrid.apply_gemv(&x, &mut y_hybrid).unwrap();

        for i in 0..rows {
            let mut dot = 0.0f32;
            for j in 0..cols {
                dot += w_hat[i * cols + j] * x[j];
            }
            assert!(
                (y_hybrid[i] - dot).abs() < 1e-4,
                "GEMV row {} mismatch: hybrid={}, dense={}",
                i,
                y_hybrid[i],
                dot
            );
        }
    }
}
