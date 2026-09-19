//! Bounded activation statistics for hybrid operator fitting (W5: EOS-050).
//!
//! Gathers streamed calibration activation statistics:
//! - Supports diagonal and block-diagonal covariance sketching $H = E[x x^T]$.
//! - Enforces an explicit declared memory budget (structured to obey the 42 MiB Sentinel).
//! - Fails closed on budget overflow or dimension mismatch.
//! - Computes regularized scaling weights for activation-weighted residual fitting.

use std::fmt;

/// Error returned by activation statistics capture and fitting.
#[derive(Debug, PartialEq)]
pub enum ConverterError {
    DimensionMismatch { expected: usize, got: usize },
    BudgetExceeded { required_bytes: usize, budget_bytes: usize },
    InvalidConfig(&'static str),
    NumericalError(&'static str),
}

impl fmt::Display for ConverterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch { expected, got } => {
                write!(f, "Dimension mismatch: expected {expected}, got {got}")
            }
            Self::BudgetExceeded { required_bytes, budget_bytes } => {
                write!(f, "Memory budget exceeded: required {required_bytes} bytes, limit is {budget_bytes} bytes")
            }
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {msg}"),
            Self::NumericalError(msg) => write!(f, "Numerical error during regularized solve or decomposition: {msg}"),
        }
    }
}

impl std::error::Error for ConverterError {}

/// Covariance sketch structure for activation statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SketchKind {
    /// Diagonal variance $E[x_i^2]$ ($O(N)$ memory).
    Diagonal,
    /// Block-diagonal covariance of tile size $B \times B$ ($O(N \cdot B)$ memory).
    BlockDiagonal { block_size: usize },
}

/// Bounded collector for activation statistics.
#[derive(Debug, Clone)]
pub struct ActivationStats {
    dim: usize,
    kind: SketchKind,
    sample_count: usize,
    /// Accumulated sums of elements $E[x_i]$.
    sum_x: Vec<f64>,
    /// Accumulated squared diagonal elements $E[x_i^2]$.
    sum_xx_diag: Vec<f64>,
    /// Accumulated block covariances if `BlockDiagonal`.
    /// Stored as contiguous row-major $B \times B$ matrices per block.
    block_covs: Vec<f64>,
    /// Declared scratch byte budget.
    budget_bytes: usize,
}

impl ActivationStats {
    /// Create a new diagonal activation statistic collector.
    pub fn new_diagonal(dim: usize, budget_bytes: usize) -> Result<Self, ConverterError> {
        if dim == 0 {
            return Err(ConverterError::InvalidConfig("Dimension must be > 0"));
        }
        // Memory required: sum_x (8 * dim) + sum_xx_diag (8 * dim)
        let required_bytes = dim.saturating_mul(16);
        if required_bytes > budget_bytes {
            return Err(ConverterError::BudgetExceeded {
                required_bytes,
                budget_bytes,
            });
        }

        Ok(Self {
            dim,
            kind: SketchKind::Diagonal,
            sample_count: 0,
            sum_x: vec![0.0; dim],
            sum_xx_diag: vec![0.0; dim],
            block_covs: Vec::new(),
            budget_bytes,
        })
    }

    /// Create a new block-diagonal activation statistic collector.
    pub fn new_block_diagonal(
        dim: usize,
        block_size: usize,
        budget_bytes: usize,
    ) -> Result<Self, ConverterError> {
        if dim == 0 || block_size == 0 {
            return Err(ConverterError::InvalidConfig("Dim and block_size must be > 0"));
        }
        if dim % block_size != 0 {
            return Err(ConverterError::InvalidConfig(
                "dim must be an exact multiple of block_size",
            ));
        }

        let num_blocks = dim / block_size;
        let block_elems = block_size.saturating_mul(block_size);
        let total_cov_elems = num_blocks.saturating_mul(block_elems);

        let required_bytes = dim
            .saturating_mul(16)
            .saturating_add(total_cov_elems.saturating_mul(8));

        if required_bytes > budget_bytes {
            return Err(ConverterError::BudgetExceeded {
                required_bytes,
                budget_bytes,
            });
        }

        Ok(Self {
            dim,
            kind: SketchKind::BlockDiagonal { block_size },
            sample_count: 0,
            sum_x: vec![0.0; dim],
            sum_xx_diag: vec![0.0; dim],
            block_covs: vec![0.0; total_cov_elems],
            budget_bytes,
        })
    }

    /// Number of samples accumulated.
    pub fn sample_count(&self) -> usize {
        self.sample_count
    }

    /// Feature dimension.
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// Declared byte budget limit.
    pub fn budget_bytes(&self) -> usize {
        self.budget_bytes
    }

    /// Memory footprint in bytes.
    pub fn allocated_bytes(&self) -> usize {
        (self.sum_x.len() + self.sum_xx_diag.len() + self.block_covs.len()) * 8
    }

    /// Stream a single activation vector into the statistics.
    pub fn accumulate_sample(&mut self, x: &[f32]) -> Result<(), ConverterError> {
        if x.len() != self.dim {
            return Err(ConverterError::DimensionMismatch {
                expected: self.dim,
                got: x.len(),
            });
        }

        self.sample_count += 1;

        for i in 0..self.dim {
            let val = x[i] as f64;
            self.sum_x[i] += val;
            self.sum_xx_diag[i] += val * val;
        }

        if let SketchKind::BlockDiagonal { block_size } = self.kind {
            let num_blocks = self.dim / block_size;
            let block_stride = block_size * block_size;

            for b in 0..num_blocks {
                let offset = b * block_stride;
                let x_blk = &x[b * block_size..(b + 1) * block_size];

                for r in 0..block_size {
                    let xr = x_blk[r] as f64;
                    let row_offset = offset + r * block_size;
                    for c in 0..block_size {
                        let xc = x_blk[c] as f64;
                        self.block_covs[row_offset + c] += xr * xc;
                    }
                }
            }
        }

        Ok(())
    }

    /// Compute the diagonal weights $\sqrt{E[x_i^2] + \lambda}$ for activation weighting.
    pub fn compute_diagonal_weights(&self, damping: f32) -> Result<Vec<f32>, ConverterError> {
        if self.sample_count == 0 {
            return Err(ConverterError::InvalidConfig("No samples accumulated"));
        }
        let inv_n = 1.0 / (self.sample_count as f64);
        let damp = damping as f64;

        let mut weights = Vec::with_capacity(self.dim);
        for i in 0..self.dim {
            let mean_sq = self.sum_xx_diag[i] * inv_n;
            let w = (mean_sq + damp).sqrt();
            weights.push(w as f32);
        }

        Ok(weights)
    }

    /// Compute the block-diagonal square-root factor $L$ where $H \approx L L^T$.
    /// For diagonal mode, returns a diagonal vector.
    /// For block-diagonal mode, returns the Cholesky factor $L$ per block (row-major $B \times B$).
    pub fn compute_block_cholesky(
        &self,
        damping: f32,
    ) -> Result<Vec<f32>, ConverterError> {
        if self.sample_count == 0 {
            return Err(ConverterError::InvalidConfig("No samples accumulated"));
        }

        match self.kind {
            SketchKind::Diagonal => self.compute_diagonal_weights(damping),
            SketchKind::BlockDiagonal { block_size } => {
                let inv_n = 1.0 / (self.sample_count as f64);
                let num_blocks = self.dim / block_size;
                let block_stride = block_size * block_size;
                let mut l_matrices = vec![0.0f32; num_blocks * block_stride];

                for b in 0..num_blocks {
                    let in_offset = b * block_stride;
                    let out_offset = b * block_stride;

                    // Copy block and add diagonal damping
                    let mut a = vec![0.0f64; block_stride];
                    for r in 0..block_size {
                        for c in 0..block_size {
                            let mut val = self.block_covs[in_offset + r * block_size + c] * inv_n;
                            if r == c {
                                val += damping as f64;
                            }
                            a[r * block_size + c] = val;
                        }
                    }

                    // Lower Cholesky decomposition L * L^T = A
                    let mut l = vec![0.0f64; block_stride];
                    for i in 0..block_size {
                        for j in 0..=i {
                            let mut sum = 0.0f64;
                            for k in 0..j {
                                sum += l[i * block_size + k] * l[j * block_size + k];
                            }
                            if i == j {
                                let diag_val = a[i * block_size + i] - sum;
                                if diag_val <= 0.0 {
                                    return Err(ConverterError::NumericalError(
                                        "Covariance matrix not positive definite after damping",
                                    ));
                                }
                                l[i * block_size + j] = diag_val.sqrt();
                            } else {
                                let l_jj = l[j * block_size + j];
                                if l_jj <= 1e-12 {
                                    return Err(ConverterError::NumericalError(
                                        "Zero or near-zero pivot in Cholesky decomposition",
                                    ));
                                }
                                l[i * block_size + j] = (a[i * block_size + j] - sum) / l_jj;
                            }
                        }
                    }

                    for idx in 0..block_stride {
                        l_matrices[out_offset + idx] = l[idx] as f32;
                    }
                }

                Ok(l_matrices)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagonal_activation_stats_basic() {
        let dim = 4;
        let budget = 42 * 1024 * 1024; // 42 MiB Sentinel
        let mut stats = ActivationStats::new_diagonal(dim, budget).unwrap();

        // Feed two samples: [1.0, 2.0, 3.0, 4.0] and [3.0, 2.0, 1.0, 0.0]
        stats.accumulate_sample(&[1.0, 2.0, 3.0, 4.0]).unwrap();
        stats.accumulate_sample(&[3.0, 2.0, 1.0, 0.0]).unwrap();

        assert_eq!(stats.sample_count(), 2);
        // E[x^2]: [ (1+9)/2 = 5, (4+4)/2 = 4, (9+1)/2 = 5, (16+0)/2 = 8 ]
        let weights = stats.compute_diagonal_weights(0.0).unwrap();
        assert!((weights[0] - 5.0f32.sqrt()).abs() < 1e-5);
        assert!((weights[1] - 4.0f32.sqrt()).abs() < 1e-5);
        assert!((weights[2] - 5.0f32.sqrt()).abs() < 1e-5);
        assert!((weights[3] - 8.0f32.sqrt()).abs() < 1e-5);
    }

    #[test]
    fn test_budget_exceeded_fails_closed() {
        let dim = 1024 * 1024; // 1M floats
        let tiny_budget = 1024; // 1 KB
        let res = ActivationStats::new_diagonal(dim, tiny_budget);
        assert!(matches!(res, Err(ConverterError::BudgetExceeded { .. })));
    }

    #[test]
    fn test_block_diagonal_cholesky() {
        let dim = 4;
        let block_size = 2;
        let budget = 42 * 1024 * 1024;
        let mut stats = ActivationStats::new_block_diagonal(dim, block_size, budget).unwrap();

        // Feed samples with positive correlation in block 0
        stats.accumulate_sample(&[2.0, 1.0, 1.0, 1.0]).unwrap();
        stats.accumulate_sample(&[1.0, 2.0, 1.0, 1.0]).unwrap();

        let l_factors = stats.compute_block_cholesky(0.01).unwrap();
        assert_eq!(l_factors.len(), 8); // 2 blocks * 4 elems

        // Verify L is lower triangular (upper off-diagonal is 0.0)
        assert_eq!(l_factors[1], 0.0); // block 0, row 0, col 1
        assert_eq!(l_factors[5], 0.0); // block 1, row 0, col 1
        assert!(l_factors[0] > 0.0);
        assert!(l_factors[3] > 0.0);
    }
}
