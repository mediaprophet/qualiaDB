use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::computation::*;
use super::core_types::*;
use super::privacy::*;
use super::storage::*;

/// Optimization engine for matrix operations
pub struct OptimizationEngine {
    pub optimizer: MatrixOptimizer,
    pub analyzer: MatrixAnalyzer,
    pub transformer: MatrixTransformer,
}

/// Matrix optimizer
pub struct MatrixOptimizer {
    pub optimization_strategies: Vec<OptimizationStrategy>,
    pub optimization_history: Vec<OptimizationRecord>,
}

/// Optimization strategies
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationStrategy {
    CacheOptimization,
    MemoryLayoutOptimization,
    AlgorithmSelection,
    Parallelization,
    Vectorization,
    Fusion,
}

/// Optimization record
#[derive(Debug, Clone)]
pub struct OptimizationRecord {
    pub timestamp: u64,
    pub matrix_id: String,
    pub strategy: OptimizationStrategy,
    pub performance_improvement: f64,
    pub memory_reduction: f64,
}

/// Matrix analyzer
pub struct MatrixAnalyzer {
    pub analysis_algorithms: Vec<AnalysisAlgorithm>,
    pub pattern_recognition: PatternRecognition,
}

/// Analysis algorithms
#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisAlgorithm {
    SparsityAnalysis,
    StructureAnalysis,
    AccessPatternAnalysis,
    PerformanceAnalysis,
}

/// Pattern recognition
pub struct PatternRecognition {
    pub recognized_patterns: Vec<MatrixPattern>,
    pub pattern_library: PatternLibrary,
}

/// Matrix patterns
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MatrixPattern {
    Diagonal,
    Triangular,
    Banded,
    Symmetric,
    PositiveDefinite,
    Orthogonal,
    Sparse,
    Dense,
    BlockDiagonal,
    Toeplitz,
    Hankel,
    Circulant,
}

/// Pattern library
pub struct PatternLibrary {
    pub patterns: HashMap<String, MatrixPattern>,
    pub optimization_hints: HashMap<MatrixPattern, OptimizationHint>,
}

/// Optimization hints
#[derive(Debug, Clone)]
pub struct OptimizationHint {
    pub preferred_algorithm: String,
    pub memory_layout: StorageFormat,
    pub parallelization_strategy: String,
    pub vectorization_hints: Vec<String>,
    /// Estimated speedup factor (e.g. 2.0 = 2x faster than naive)
    pub estimated_speedup: f64,
}

/// Matrix transformer
pub struct MatrixTransformer {
    pub transformation_rules: Vec<TransformationRule>,
    pub transformation_history: Vec<TransformationRecord>,
}

/// Transformation rules
#[derive(Debug, Clone, PartialEq)]
pub enum TransformationRule {
    RowColumnSwap,
    BlockReordering,
    DataTypeConversion,
    CompressionDecompression,
    LayoutConversion,
}

/// Transformation record
#[derive(Debug, Clone)]
pub struct TransformationRecord {
    pub timestamp: u64,
    pub matrix_id: String,
    pub transformation: TransformationRule,
    pub performance_impact: f64,
}

/// Target memory layout for a `MatrixTransformer` layout conversion.
///
/// The `Matrix.data` buffer is always a flat `Vec<f64>`; the layout describes
/// how logical element `(i, j)` is addressed within that buffer. Converting
/// between layouts reorganises the buffer in place and updates
/// `Matrix.storage_format` accordingly.
#[derive(Debug, Clone, PartialEq)]
pub enum MatrixLayout {
    /// Row-major storage: element `(i, j)` lives at `data[i * cols + j]`.
    /// This is the canonical layout used throughout the linear-algebra
    /// library and the default for sequential row-wise access.
    RowMajor,
    /// Column-major storage: element `(i, j)` lives at `data[j * rows + i]`.
    /// Preferred for column-heavy (strided) access patterns.
    ColMajor,
    /// Cache-friendly blocked (tiled) storage. The matrix is partitioned into
    /// `block_size` x `block_size` sub-blocks. Blocks themselves are laid out
    /// in row-major block order (block row, then block column); within each
    /// block the element order follows `inner` (typically `RowMajor`).
    ///
    /// Edge blocks that fall outside the matrix dimensions contain only the
    /// valid elements, so the total buffer length remains `rows * cols`.
    Blocked(Box<MatrixLayout>, usize),
    /// SIMD-packed storage: each row is zero-padded to a multiple of
    /// [`SIMD_WIDTH`] so that a full SIMD vector load never crosses a row
    /// boundary. Element `(i, j)` lives at `data[i * stride + j]` where
    /// `stride = ceil(cols / SIMD_WIDTH) * SIMD_WIDTH`; padding slots are `0.0`.
    Packed,
}

/// SIMD vector width (in `f64` elements) used by the [`MatrixLayout::Packed`]
/// layout. The value `4` corresponds to a 256-bit AVX2 register holding four
/// double-precision lanes.
const SIMD_WIDTH: usize = 4;

impl OptimizationEngine {
    pub fn new() -> Self {
        Self {
            optimizer: MatrixOptimizer::new(),
            analyzer: MatrixAnalyzer::new(),
            transformer: MatrixTransformer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        self.optimizer.initialize()?;
        self.analyzer.initialize()?;
        self.transformer.initialize()?;
        Ok(())
    }

    pub fn optimize_multiplication(
        &mut self,
        left: &Matrix,
        right: &Matrix,
    ) -> Result<OptimizedMultiplication, LinearAlgebraError> {
        // Analyze matrices
        let _left_analysis = self.analyzer.analyze_matrix(left)?;
        let _right_analysis = self.analyzer.analyze_matrix(right)?;

        // Create optimized operation
        let optimized = OptimizedMultiplication {
            left: left.clone(),
            right: right.clone(),
            optimization_strategy: OptimizationStrategy::Vectorization,
            expected_performance_gain: 2.0,
        };

        Ok(optimized)
    }
}

impl MatrixOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_strategies: vec![
                OptimizationStrategy::Vectorization,
                OptimizationStrategy::CacheOptimization,
                OptimizationStrategy::Parallelization,
            ],
            optimization_history: Vec::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        Ok(())
    }
}

impl MatrixAnalyzer {
    pub fn new() -> Self {
        Self {
            analysis_algorithms: vec![
                AnalysisAlgorithm::StructureAnalysis,
                AnalysisAlgorithm::SparsityAnalysis,
            ],
            pattern_recognition: PatternRecognition::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        self.pattern_recognition.initialize()?;
        Ok(())
    }

    /// Analyze a matrix: detect all structural patterns, compute sparsity,
    /// and determine recommended algorithms from optimization hints.
    pub fn analyze_matrix(
        &mut self,
        matrix: &Matrix,
    ) -> Result<MatrixAnalysis, LinearAlgebraError> {
        let detected = self.detect_structure(matrix);
        let sparsity = self.calculate_sparsity(matrix);

        // Determine recommended algorithms from optimization hints
        let mut recommended_algorithms = Vec::new();
        let mut hint_strings = Vec::new();
        for pattern in &detected {
            if let Some(hint) = self
                .pattern_recognition
                .pattern_library
                .get_optimization_hint(pattern)
            {
                recommended_algorithms.push(hint.preferred_algorithm.clone());
                hint_strings.push(format!(
                    "{:?}: {} (speedup ~{:.1}x)",
                    pattern, hint.preferred_algorithm, hint.estimated_speedup
                ));
            }
        }

        // Pick the primary structure: prefer the most specific pattern
        let structure = detected
            .first()
            .cloned()
            .unwrap_or(MatrixPattern::Dense);

        Ok(MatrixAnalysis {
            matrix_id: matrix.matrix_id.clone(),
            sparsity,
            structure,
            detected_patterns: detected,
            access_pattern: AccessPattern::Sequential,
            optimization_hints: hint_strings,
            recommended_algorithms,
        })
    }

    fn calculate_sparsity(&self, matrix: &Matrix) -> f64 {
        let non_zero = matrix.data.iter().filter(|&&x| x.abs() > 1e-10).count();
        1.0 - (non_zero as f64 / matrix.data.len().max(1) as f64)
    }

    /// Detect ALL applicable structural patterns in the matrix.
    /// Uses a tolerance of 1e-10 for floating-point comparisons.
    pub fn detect_structure(&self, matrix: &Matrix) -> Vec<MatrixPattern> {
        const TOL: f64 = 1e-10;
        let rows = matrix.rows;
        let cols = matrix.cols;
        let data = &matrix.data;
        let mut patterns = Vec::new();

        // Helper: get element (i, j)
        let at = |i: usize, j: usize| -> f64 {
            data[i * cols + j]
        };

        // --- Sparse: fraction of non-zero elements < 0.3 ---
        let non_zero_count = data.iter().filter(|&&x| x.abs() > TOL).count();
        let total = data.len();
        let non_zero_frac = if total > 0 {
            non_zero_count as f64 / total as f64
        } else {
            0.0
        };
        let is_sparse = non_zero_frac < 0.3;
        if is_sparse {
            patterns.push(MatrixPattern::Sparse);
        }

        // Only square matrices can be diagonal, triangular, symmetric, etc.
        let is_square = rows == cols && rows > 0;

        // --- Diagonal: all off-diagonal elements are ~0 ---
        if is_square {
            let mut is_diagonal = true;
            for i in 0..rows {
                for j in 0..cols {
                    if i != j && at(i, j).abs() > TOL {
                        is_diagonal = false;
                        break;
                    }
                }
                if !is_diagonal {
                    break;
                }
            }
            if is_diagonal {
                patterns.push(MatrixPattern::Diagonal);
            }
        }

        // --- Triangular (Upper/Lower) ---
        if is_square {
            // Upper triangular: all elements below diagonal are ~0
            let mut is_upper = true;
            for i in 1..rows {
                for j in 0..i {
                    if at(i, j).abs() > TOL {
                        is_upper = false;
                        break;
                    }
                }
                if !is_upper {
                    break;
                }
            }
            // Lower triangular: all elements above diagonal are ~0
            let mut is_lower = true;
            for i in 0..rows {
                for j in (i + 1)..cols {
                    if at(i, j).abs() > TOL {
                        is_lower = false;
                        break;
                    }
                }
                if !is_lower {
                    break;
                }
            }
            if is_upper || is_lower {
                patterns.push(MatrixPattern::Triangular);
            }
        }

        // --- Symmetric: matrix == transpose (within tolerance) ---
        if is_square {
            let mut is_symmetric = true;
            for i in 0..rows {
                for j in (i + 1)..cols {
                    if (at(i, j) - at(j, i)).abs() > TOL {
                        is_symmetric = false;
                        break;
                    }
                }
                if !is_symmetric {
                    break;
                }
            }
            if is_symmetric {
                patterns.push(MatrixPattern::Symmetric);

                // --- PositiveDefinite: symmetric + all eigenvalues > 0 ---
                // Use Sylvester's criterion: all leading principal minors > 0
                if Self::is_positive_definite(rows, data, TOL) {
                    patterns.push(MatrixPattern::PositiveDefinite);
                }
            }
        }

        // --- Banded: non-zero only within a band around diagonal ---
        if rows > 0 && cols > 0 {
            let max_dim = rows.max(cols);
            // Determine the bandwidth
            let mut bandwidth = 0usize;
            for i in 0..rows {
                for j in 0..cols {
                    if at(i, j).abs() > TOL {
                        let dist = if i >= j { i - j } else { j - i };
                        if dist > bandwidth {
                            bandwidth = dist;
                        }
                    }
                }
            }
            // Banded if bandwidth is small relative to matrix dimension
            // and bandwidth > 0 (not purely diagonal)
            if bandwidth > 0 && (bandwidth as f64) < (max_dim as f64) / 3.0 {
                patterns.push(MatrixPattern::Banded);
            }
        }

        // --- BlockDiagonal: non-zero blocks along diagonal ---
        if is_square && rows >= 4 {
            if Self::is_block_diagonal(rows, data, TOL) {
                patterns.push(MatrixPattern::BlockDiagonal);
            }
        }

        // --- Toeplitz: each diagonal has constant value ---
        if rows > 1 && cols > 1 {
            let mut is_toeplitz = true;
            // Check each diagonal
            for d in -(rows as isize - 1)..(cols as isize) {
                // Get the first value on this diagonal
                let first = if d >= 0 {
                    at(0, d as usize)
                } else {
                    at((-d) as usize, 0)
                };
                // Check all elements on this diagonal
                let start_i = if d >= 0 { 0 } else { (-d) as usize };
                let start_j = if d >= 0 { d as usize } else { 0 };
                let mut i = start_i;
                let mut j = start_j;
                while i < rows && j < cols {
                    if (at(i, j) - first).abs() > TOL {
                        is_toeplitz = false;
                        break;
                    }
                    i += 1;
                    j += 1;
                }
                if !is_toeplitz {
                    break;
                }
            }
            if is_toeplitz {
                patterns.push(MatrixPattern::Toeplitz);
            }
        }

        // --- Orthogonal: A * A^T ≈ I ---
        if is_square && rows > 0 {
            if Self::is_orthogonal(rows, data, TOL) {
                patterns.push(MatrixPattern::Orthogonal);
            }
        }

        // --- Circulant: each row is a cyclic shift of the previous ---
        if is_square && rows > 1 {
            let mut is_circulant = true;
            for i in 1..rows {
                // Row i should be row (i-1) shifted right by 1 (cyclically)
                for j in 0..cols {
                    let expected = at(i - 1, if j == 0 { cols - 1 } else { j - 1 });
                    if (at(i, j) - expected).abs() > TOL {
                        is_circulant = false;
                        break;
                    }
                }
                if !is_circulant {
                    break;
                }
            }
            if is_circulant {
                patterns.push(MatrixPattern::Circulant);
            }
        }

        // --- Hankel: constant along anti-diagonals ---
        if rows > 1 && cols > 1 {
            let mut is_hankel = true;
            // Anti-diagonal d ranges from 0 to (rows-1)+(cols-1)
            // Element (i, j) is on anti-diagonal i + j
            for d in 0..(rows + cols - 1) {
                // Get first element on this anti-diagonal
                let mut first_val: Option<f64> = None;
                for i in 0..rows {
                    let j = d as isize - i as isize;
                    if j >= 0 && (j as usize) < cols {
                        let v = at(i, j as usize);
                        if first_val.is_none() {
                            first_val = Some(v);
                        } else if (v - first_val.unwrap()).abs() > TOL {
                            is_hankel = false;
                            break;
                        }
                    }
                }
                if !is_hankel {
                    break;
                }
            }
            if is_hankel {
                patterns.push(MatrixPattern::Hankel);
            }
        }

        // Always include Dense if no other pattern was detected
        if patterns.is_empty() {
            patterns.push(MatrixPattern::Dense);
        }

        patterns
    }

    /// Check if a square matrix is positive definite using Sylvester's criterion:
    /// all leading principal minors must be positive.
    fn is_positive_definite(n: usize, data: &[f64], tol: f64) -> bool {
        for k in 1..=n {
            // Compute the determinant of the k×k leading principal submatrix
            let mut sub = vec![0.0; k * k];
            for i in 0..k {
                for j in 0..k {
                    sub[i * k + j] = data[i * n + j];
                }
            }
            // Compute determinant via LU-like recursive expansion for small k
            let det = Self::determinant(k, &sub);
            if det <= tol {
                return false;
            }
        }
        true
    }

    /// Compute the determinant of an n×n matrix via LU decomposition
    fn determinant(n: usize, data: &[f64]) -> f64 {
        if n == 0 {
            return 1.0;
        }
        if n == 1 {
            return data[0];
        }
        // LU decomposition with partial pivoting
        let mut a = data.to_vec();
        let mut sign = 1.0_f64;
        for i in 0..n {
            // Find pivot
            let mut max_row = i;
            let mut max_val = a[i * n + i].abs();
            for k in (i + 1)..n {
                if a[k * n + i].abs() > max_val {
                    max_val = a[k * n + i].abs();
                    max_row = k;
                }
            }
            if max_val < 1e-15 {
                return 0.0; // singular
            }
            if max_row != i {
                for j in 0..n {
                    let tmp = a[i * n + j];
                    a[i * n + j] = a[max_row * n + j];
                    a[max_row * n + j] = tmp;
                }
                sign = -sign;
            }
            // Eliminate
            for k in (i + 1)..n {
                let factor = a[k * n + i] / a[i * n + i];
                for j in i..n {
                    a[k * n + j] -= factor * a[i * n + j];
                }
            }
        }
        let mut det = sign;
        for i in 0..n {
            det *= a[i * n + i];
        }
        det
    }

    /// Check if a square matrix is block diagonal (non-zero blocks along the diagonal)
    fn is_block_diagonal(n: usize, data: &[f64], tol: f64) -> bool {
        // Heuristic: check if there's a consistent block size where off-block
        // elements are zero. Try block sizes 2, 4, etc.
        let at = |i: usize, j: usize| -> f64 { data[i * n + j] };
        for block_size in [2, 4, 8].iter() {
            if *block_size >= n {
                continue;
            }
            if n % block_size != 0 {
                continue;
            }
            let mut is_block_diag = true;
            'outer: for bi in 0..(n / block_size) {
                for bj in 0..(n / block_size) {
                    if bi == bj {
                        continue; // diagonal block
                    }
                    // Check off-diagonal block is all zeros
                    for i in 0..*block_size {
                        for j in 0..*block_size {
                            if at(bi * block_size + i, bj * block_size + j).abs() > tol {
                                is_block_diag = false;
                                break 'outer;
                            }
                        }
                    }
                }
            }
            if is_block_diag {
                return true;
            }
        }
        false
    }

    /// Check if a square matrix is orthogonal: A * A^T ≈ I
    fn is_orthogonal(n: usize, data: &[f64], tol: f64) -> bool {
        let at = |i: usize, j: usize| -> f64 { data[i * n + j] };
        // Compute A * A^T and check if it's approximately I
        for i in 0..n {
            for j in 0..n {
                let mut dot = 0.0;
                for k in 0..n {
                    dot += at(i, k) * at(j, k);
                }
                let expected = if i == j { 1.0 } else { 0.0 };
                if (dot - expected).abs() > tol * 10.0 {
                    return false;
                }
            }
        }
        true
    }
}

impl PatternRecognition {
    pub fn new() -> Self {
        Self {
            recognized_patterns: Vec::new(),
            pattern_library: PatternLibrary::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        self.pattern_library.initialize()?;
        Ok(())
    }
}

impl PatternLibrary {
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            optimization_hints: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        // Populate the patterns HashMap with names for each MatrixPattern variant
        self.patterns.insert("diagonal".to_string(), MatrixPattern::Diagonal);
        self.patterns.insert("triangular".to_string(), MatrixPattern::Triangular);
        self.patterns.insert("banded".to_string(), MatrixPattern::Banded);
        self.patterns.insert("symmetric".to_string(), MatrixPattern::Symmetric);
        self.patterns
            .insert("positive_definite".to_string(), MatrixPattern::PositiveDefinite);
        self.patterns.insert("orthogonal".to_string(), MatrixPattern::Orthogonal);
        self.patterns.insert("sparse".to_string(), MatrixPattern::Sparse);
        self.patterns.insert("dense".to_string(), MatrixPattern::Dense);
        self.patterns
            .insert("block_diagonal".to_string(), MatrixPattern::BlockDiagonal);
        self.patterns.insert("toeplitz".to_string(), MatrixPattern::Toeplitz);
        self.patterns.insert("hankel".to_string(), MatrixPattern::Hankel);
        self.patterns.insert("circulant".to_string(), MatrixPattern::Circulant);

        // Populate optimization_hints with recommended algorithms for each pattern
        self.optimization_hints.insert(
            MatrixPattern::Diagonal,
            OptimizationHint {
                preferred_algorithm: "diagonal_scale".to_string(),
                memory_layout: StorageFormat::CompressedSparseRow,
                parallelization_strategy: "element_wise".to_string(),
                vectorization_hints: vec!["scalar_multiply".to_string()],
                estimated_speedup: 10.0,
            },
        );
        self.optimization_hints.insert(
            MatrixPattern::Triangular,
            OptimizationHint {
                preferred_algorithm: "triangular_solve".to_string(),
                memory_layout: StorageFormat::RowMajor,
                parallelization_strategy: "row_parallel".to_string(),
                vectorization_hints: vec!["forward_substitution".to_string()],
                estimated_speedup: 3.0,
            },
        );
        self.optimization_hints.insert(
            MatrixPattern::Banded,
            OptimizationHint {
                preferred_algorithm: "banded_gemm".to_string(),
                memory_layout: StorageFormat::Blocked,
                parallelization_strategy: "band_parallel".to_string(),
                vectorization_hints: vec!["band_vectorization".to_string()],
                estimated_speedup: 5.0,
            },
        );
        self.optimization_hints.insert(
            MatrixPattern::Symmetric,
            OptimizationHint {
                preferred_algorithm: "symmetric_gemm".to_string(),
                memory_layout: StorageFormat::RowMajor,
                parallelization_strategy: "block_parallel".to_string(),
                vectorization_hints: vec!["symmetric_pack".to_string()],
                estimated_speedup: 2.0,
            },
        );
        self.optimization_hints.insert(
            MatrixPattern::PositiveDefinite,
            OptimizationHint {
                preferred_algorithm: "cholesky_decomposition".to_string(),
                memory_layout: StorageFormat::RowMajor,
                parallelization_strategy: "block_parallel".to_string(),
                vectorization_hints: vec!["cholesky_vectorized".to_string()],
                estimated_speedup: 4.0,
            },
        );
        self.optimization_hints.insert(
            MatrixPattern::Orthogonal,
            OptimizationHint {
                preferred_algorithm: "orthogonal_transform".to_string(),
                memory_layout: StorageFormat::RowMajor,
                parallelization_strategy: "column_parallel".to_string(),
                vectorization_hints: vec!["transpose_free".to_string()],
                estimated_speedup: 3.0,
            },
        );
        self.optimization_hints.insert(
            MatrixPattern::Sparse,
            OptimizationHint {
                preferred_algorithm: "sparse_gemm".to_string(),
                memory_layout: StorageFormat::CompressedSparseRow,
                parallelization_strategy: "row_parallel".to_string(),
                vectorization_hints: vec!["sparse_vectorization".to_string()],
                estimated_speedup: 8.0,
            },
        );
        self.optimization_hints.insert(
            MatrixPattern::Dense,
            OptimizationHint {
                preferred_algorithm: "blocked_gemm".to_string(),
                memory_layout: StorageFormat::Blocked,
                parallelization_strategy: "block_parallel".to_string(),
                vectorization_hints: vec!["avx2_vectorization".to_string()],
                estimated_speedup: 1.0,
            },
        );
        self.optimization_hints.insert(
            MatrixPattern::BlockDiagonal,
            OptimizationHint {
                preferred_algorithm: "block_diagonal_gemm".to_string(),
                memory_layout: StorageFormat::Blocked,
                parallelization_strategy: "block_parallel".to_string(),
                vectorization_hints: vec!["block_vectorization".to_string()],
                estimated_speedup: 6.0,
            },
        );
        self.optimization_hints.insert(
            MatrixPattern::Toeplitz,
            OptimizationHint {
                preferred_algorithm: "toeplitz_fft".to_string(),
                memory_layout: StorageFormat::RowMajor,
                parallelization_strategy: "diagonal_parallel".to_string(),
                vectorization_hints: vec!["fft_convolution".to_string()],
                estimated_speedup: 7.0,
            },
        );
        self.optimization_hints.insert(
            MatrixPattern::Hankel,
            OptimizationHint {
                preferred_algorithm: "hankel_fft".to_string(),
                memory_layout: StorageFormat::RowMajor,
                parallelization_strategy: "anti_diagonal_parallel".to_string(),
                vectorization_hints: vec!["fft_convolution".to_string()],
                estimated_speedup: 7.0,
            },
        );
        self.optimization_hints.insert(
            MatrixPattern::Circulant,
            OptimizationHint {
                preferred_algorithm: "circulant_fft".to_string(),
                memory_layout: StorageFormat::RowMajor,
                parallelization_strategy: "row_parallel".to_string(),
                vectorization_hints: vec!["fft_convolution".to_string()],
                estimated_speedup: 8.0,
            },
        );

        Ok(())
    }

    /// Return the optimization hint for a given pattern
    pub fn get_optimization_hint(&self, pattern: &MatrixPattern) -> Option<&OptimizationHint> {
        self.optimization_hints.get(pattern)
    }
}

impl MatrixTransformer {
    pub fn new() -> Self {
        Self {
            transformation_rules: vec![
                TransformationRule::LayoutConversion,
                TransformationRule::RowColumnSwap,
                TransformationRule::BlockReordering,
                TransformationRule::DataTypeConversion,
            ],
            transformation_history: Vec::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        Ok(())
    }

    /// Transform a matrix's in-memory storage layout to `target_layout`.
    ///
    /// The source layout is inferred from `matrix.storage_format`. The
    /// returned matrix has its `data` buffer reorganised and its
    /// `storage_format` (and `metadata.storage_format`) updated to reflect
    /// the new layout. Row/column dimensions and element values are preserved.
    ///
    /// Supported conversions:
    /// - `RowMajor` <-> `ColMajor` (transpose storage layout)
    /// - `Blocked(inner, block_size)` (reorganise into cache-friendly tiles)
    /// - `Packed` (pad each row to a multiple of the SIMD vector width)
    ///
    /// Reading from a `Blocked` source is not supported because the block size
    /// is not carried in `Matrix` metadata; reading from sparse formats
    /// (`CompressedSparseRow` / `CompressedSparseColumn`) is likewise
    /// unsupported. Both return an `OptimizationError`.
    pub fn transform_matrix(
        &self,
        matrix: &Matrix,
        target_layout: MatrixLayout,
    ) -> Result<Matrix, LinearAlgebraError> {
        let rows = matrix.rows;
        let cols = matrix.cols;

        // Validate the source buffer is consistent with its declared layout.
        validate_source_buffer(matrix)?;

        // Read a logical element (i, j) from the source according to its
        // current storage_format.
        let src = |i: usize, j: usize| -> Result<f64, LinearAlgebraError> {
            read_element(matrix, i, j)
        };

        let (data, storage_format) = match target_layout {
            MatrixLayout::RowMajor => {
                let mut data = Vec::with_capacity(rows * cols);
                for i in 0..rows {
                    for j in 0..cols {
                        data.push(src(i, j)?);
                    }
                }
                (data, StorageFormat::RowMajor)
            }
            MatrixLayout::ColMajor => {
                let mut data = Vec::with_capacity(rows * cols);
                // Column-major: contiguous down each column.
                for j in 0..cols {
                    for i in 0..rows {
                        data.push(src(i, j)?);
                    }
                }
                (data, StorageFormat::ColumnMajor)
            }
            MatrixLayout::Blocked(inner, block_size) => {
                if block_size == 0 {
                    return Err(LinearAlgebraError::OptimizationError(
                        "Blocked layout requires block_size > 0".to_string(),
                    ));
                }
                let mut data = Vec::with_capacity(rows * cols);
                let block_rows = (rows + block_size - 1) / block_size;
                let block_cols = (cols + block_size - 1) / block_size;
                // Within-block element order follows the inner layout; default
                // to row-major for any non-column-major inner layout.
                let col_major_inner = *inner == MatrixLayout::ColMajor;
                for bi in 0..block_rows {
                    for bj in 0..block_cols {
                        let i_start = bi * block_size;
                        let i_end = rows.min((bi + 1) * block_size);
                        let j_start = bj * block_size;
                        let j_end = cols.min((bj + 1) * block_size);
                        if col_major_inner {
                            for j in j_start..j_end {
                                for i in i_start..i_end {
                                    data.push(src(i, j)?);
                                }
                            }
                        } else {
                            for i in i_start..i_end {
                                for j in j_start..j_end {
                                    data.push(src(i, j)?);
                                }
                            }
                        }
                    }
                }
                (data, StorageFormat::Blocked)
            }
            MatrixLayout::Packed => {
                let stride = padded_stride(cols);
                let mut data = vec![0.0; rows * stride];
                for i in 0..rows {
                    for j in 0..cols {
                        data[i * stride + j] = src(i, j)?;
                    }
                }
                (data, StorageFormat::Packed)
            }
        };

        let mut result = matrix.clone();
        result.data = data;
        result.storage_format = storage_format.clone();
        result.metadata.storage_format = storage_format;
        Ok(result)
    }

    /// Analyse an access pattern and automatically pick the best layout for
    /// the given matrix, then transform it.
    ///
    /// Layout selection heuristic:
    /// - [`AccessPattern::Sequential`] (row-wise traversal) -> `RowMajor`
    /// - [`AccessPattern::Strided`] (column-wise traversal) -> `ColMajor`
    /// - [`AccessPattern::Blocked`] -> `Blocked(RowMajor, block_size)` where
    ///   `block_size` is a cache-friendly tile (capped at 16)
    /// - [`AccessPattern::Random`] -> `RowMajor` (no clear winner; keep the
    ///   canonical layout)
    /// - [`AccessPattern::Adaptive`] -> `ColMajor` for tall matrices
    ///   (`rows > cols`), `RowMajor` otherwise
    pub fn optimize_layout(
        &self,
        matrix: &Matrix,
        access_pattern: &AccessPattern,
    ) -> Result<Matrix, LinearAlgebraError> {
        let target = match access_pattern {
            AccessPattern::Sequential => MatrixLayout::RowMajor,
            AccessPattern::Strided => MatrixLayout::ColMajor,
            AccessPattern::Blocked => {
                // Pick a cache-friendly tile size bounded by the matrix's
                // smaller dimension and a 16-element cap.
                let block_size = matrix.rows.min(matrix.cols).max(1).min(16);
                MatrixLayout::Blocked(Box::new(MatrixLayout::RowMajor), block_size)
            }
            AccessPattern::Random => MatrixLayout::RowMajor,
            AccessPattern::Adaptive => {
                if matrix.rows > matrix.cols {
                    MatrixLayout::ColMajor
                } else {
                    MatrixLayout::RowMajor
                }
            }
        };
        self.transform_matrix(matrix, target)
    }
}

/// Padded row stride for the [`MatrixLayout::Packed`] layout: the column count
/// rounded up to the next multiple of [`SIMD_WIDTH`].
fn padded_stride(cols: usize) -> usize {
    ((cols + SIMD_WIDTH - 1) / SIMD_WIDTH) * SIMD_WIDTH
}

/// Read logical element `(i, j)` from `matrix` according to its
/// `storage_format`. Returns an error for unsupported source layouts
/// (`Blocked`, sparse formats) or out-of-bounds indices.
fn read_element(matrix: &Matrix, i: usize, j: usize) -> Result<f64, LinearAlgebraError> {
    let rows = matrix.rows;
    let cols = matrix.cols;
    if i >= rows || j >= cols {
        return Err(LinearAlgebraError::OptimizationError(format!(
            "element index ({}, {}) out of bounds for matrix of shape {}x{}",
            i, j, rows, cols
        )));
    }
    let idx = match matrix.storage_format {
        StorageFormat::RowMajor => i * cols + j,
        StorageFormat::ColumnMajor => j * rows + i,
        StorageFormat::Packed => {
            let stride = padded_stride(cols);
            i * stride + j
        }
        StorageFormat::Blocked => {
            return Err(LinearAlgebraError::OptimizationError(
                "cannot read from Blocked source: block size is not stored in Matrix metadata"
                    .to_string(),
            ));
        }
        StorageFormat::CompressedSparseRow | StorageFormat::CompressedSparseColumn => {
            return Err(LinearAlgebraError::OptimizationError(
                "sparse source layouts are not supported by the layout transformer".to_string(),
            ));
        }
    };
    if idx >= matrix.data.len() {
        return Err(LinearAlgebraError::OptimizationError(format!(
            "linear index {} out of bounds for data buffer of length {} (layout {:?})",
            idx,
            matrix.data.len(),
            matrix.storage_format
        )));
    }
    Ok(matrix.data[idx])
}

/// Validate that the source matrix's `data` buffer length is consistent with
/// its declared `storage_format` and dimensions.
fn validate_source_buffer(matrix: &Matrix) -> Result<(), LinearAlgebraError> {
    let expected = match matrix.storage_format {
        StorageFormat::RowMajor | StorageFormat::ColumnMajor | StorageFormat::Blocked => {
            matrix.rows * matrix.cols
        }
        StorageFormat::Packed => matrix.rows * padded_stride(matrix.cols),
        StorageFormat::CompressedSparseRow | StorageFormat::CompressedSparseColumn => {
            // Sparse formats carry their own structure; skip strict length
            // validation rather than guess the expected buffer size.
            return Ok(());
        }
    };
    if matrix.data.len() != expected {
        return Err(LinearAlgebraError::OptimizationError(format!(
            "source data length {} does not match expected {} for {:?} layout ({}x{})",
            matrix.data.len(),
            expected,
            matrix.storage_format,
            matrix.rows,
            matrix.cols
        )));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct MatrixAnalysis {
    pub matrix_id: String,
    pub sparsity: f64,
    pub structure: MatrixPattern,
    /// All detected structural patterns
    pub detected_patterns: Vec<MatrixPattern>,
    pub access_pattern: AccessPattern,
    pub optimization_hints: Vec<String>,
    /// Recommended algorithms derived from optimization hints
    pub recommended_algorithms: Vec<String>,
}

/// Result of a full matrix analysis (alias for MatrixAnalysis for API clarity)
pub type MatrixAnalysisResult = MatrixAnalysis;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_matrix(id: &str, rows: usize, cols: usize, data: Vec<f64>) -> Matrix {
        let metadata = MatrixMetadata {
            matrix_id: id.to_string(),
            rows,
            cols,
            data_type: DataType::Float64,
            storage_format: StorageFormat::RowMajor,
            compression: CompressionType::None,
            created_at: 0,
            last_accessed: 0,
            access_count: 0,
        };
        Matrix {
            matrix_id: id.to_string(),
            rows,
            cols,
            data_type: DataType::Float64,
            data,
            storage_format: StorageFormat::RowMajor,
            metadata,
        }
    }

    fn make_analyzer() -> MatrixAnalyzer {
        let mut analyzer = MatrixAnalyzer::new();
        analyzer.initialize().unwrap();
        analyzer
    }

    #[test]
    fn test_detect_diagonal() {
        let analyzer = make_analyzer();
        // Diagonal matrix
        let m = make_matrix("d", 3, 3, vec![1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0]);
        let patterns = analyzer.detect_structure(&m);
        assert!(patterns.contains(&MatrixPattern::Diagonal));
    }

    #[test]
    fn test_detect_upper_triangular() {
        let analyzer = make_analyzer();
        let m = make_matrix("u", 3, 3, vec![1.0, 2.0, 3.0, 0.0, 4.0, 5.0, 0.0, 0.0, 6.0]);
        let patterns = analyzer.detect_structure(&m);
        assert!(patterns.contains(&MatrixPattern::Triangular));
    }

    #[test]
    fn test_detect_lower_triangular() {
        let analyzer = make_analyzer();
        let m = make_matrix("l", 3, 3, vec![1.0, 0.0, 0.0, 2.0, 3.0, 0.0, 4.0, 5.0, 6.0]);
        let patterns = analyzer.detect_structure(&m);
        assert!(patterns.contains(&MatrixPattern::Triangular));
    }

    #[test]
    fn test_detect_symmetric() {
        let analyzer = make_analyzer();
        let m = make_matrix("s", 3, 3, vec![1.0, 2.0, 3.0, 2.0, 4.0, 5.0, 3.0, 5.0, 6.0]);
        let patterns = analyzer.detect_structure(&m);
        assert!(patterns.contains(&MatrixPattern::Symmetric));
    }

    #[test]
    fn test_detect_not_symmetric() {
        let analyzer = make_analyzer();
        let m = make_matrix("ns", 2, 2, vec![1.0, 2.0, 3.0, 4.0]);
        let patterns = analyzer.detect_structure(&m);
        assert!(!patterns.contains(&MatrixPattern::Symmetric));
    }

    #[test]
    fn test_detect_positive_definite() {
        let analyzer = make_analyzer();
        // [[2,1],[1,2]] is symmetric positive definite (eigenvalues 1, 3)
        let m = make_matrix("pd", 2, 2, vec![2.0, 1.0, 1.0, 2.0]);
        let patterns = analyzer.detect_structure(&m);
        assert!(patterns.contains(&MatrixPattern::PositiveDefinite));
    }

    #[test]
    fn test_detect_not_positive_definite() {
        let analyzer = make_analyzer();
        // [[1,2],[2,1]] is symmetric but not positive definite (eigenvalues -1, 3)
        let m = make_matrix("npd", 2, 2, vec![1.0, 2.0, 2.0, 1.0]);
        let patterns = analyzer.detect_structure(&m);
        assert!(!patterns.contains(&MatrixPattern::PositiveDefinite));
    }

    #[test]
    fn test_detect_sparse() {
        let analyzer = make_analyzer();
        // 4x4 matrix with mostly zeros
        let m = make_matrix(
            "sp",
            4,
            4,
            vec![
                1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        );
        let patterns = analyzer.detect_structure(&m);
        assert!(patterns.contains(&MatrixPattern::Sparse));
    }

    #[test]
    fn test_detect_dense() {
        let analyzer = make_analyzer();
        let m = make_matrix("d", 2, 2, vec![1.0, 2.0, 3.0, 4.0]);
        let patterns = analyzer.detect_structure(&m);
        assert!(patterns.contains(&MatrixPattern::Dense));
    }

    #[test]
    fn test_detect_banded() {
        let analyzer = make_analyzer();
        // Tridiagonal 5x5 (bandwidth 1, which is < 5/3 = 1.66)
        let m = make_matrix(
            "b",
            5,
            5,
            vec![
                1.0, 2.0, 0.0, 0.0, 0.0, 3.0, 4.0, 5.0, 0.0, 0.0, 0.0, 6.0, 7.0, 8.0, 0.0, 0.0,
                0.0, 9.0, 10.0, 11.0, 0.0, 0.0, 0.0, 12.0, 13.0,
            ],
        );
        let patterns = analyzer.detect_structure(&m);
        assert!(patterns.contains(&MatrixPattern::Banded));
    }

    #[test]
    fn test_detect_toeplitz() {
        let analyzer = make_analyzer();
        // Toeplitz: each diagonal constant
        // [[1,2,3],[4,1,2],[5,4,1]]
        let m = make_matrix("t", 3, 3, vec![1.0, 2.0, 3.0, 4.0, 1.0, 2.0, 5.0, 4.0, 1.0]);
        let patterns = analyzer.detect_structure(&m);
        assert!(patterns.contains(&MatrixPattern::Toeplitz));
    }

    #[test]
    fn test_detect_orthogonal() {
        let analyzer = make_analyzer();
        // 2x2 rotation matrix (orthogonal)
        // [[0, -1], [1, 0]] — A*A^T = I
        let m = make_matrix("o", 2, 2, vec![0.0, -1.0, 1.0, 0.0]);
        let patterns = analyzer.detect_structure(&m);
        assert!(patterns.contains(&MatrixPattern::Orthogonal));
    }

    #[test]
    fn test_detect_circulant() {
        let analyzer = make_analyzer();
        // Circulant 3x3: each row is cyclic shift of previous
        // [[1,2,3],[3,1,2],[2,3,1]]
        let m = make_matrix("c", 3, 3, vec![1.0, 2.0, 3.0, 3.0, 1.0, 2.0, 2.0, 3.0, 1.0]);
        let patterns = analyzer.detect_structure(&m);
        assert!(patterns.contains(&MatrixPattern::Circulant));
    }

    #[test]
    fn test_detect_hankel() {
        let analyzer = make_analyzer();
        // Hankel: constant along anti-diagonals
        // [[1,2,3],[2,3,4],[3,4,5]]
        let m = make_matrix(
            "h",
            3,
            3,
            vec![1.0, 2.0, 3.0, 2.0, 3.0, 4.0, 3.0, 4.0, 5.0],
        );
        let patterns = analyzer.detect_structure(&m);
        assert!(patterns.contains(&MatrixPattern::Hankel));
    }

    #[test]
    fn test_detect_block_diagonal() {
        let analyzer = make_analyzer();
        // 4x4 block diagonal with 2x2 blocks
        let m = make_matrix(
            "bd",
            4,
            4,
            vec![
                1.0, 2.0, 0.0, 0.0, 3.0, 4.0, 0.0, 0.0, 0.0, 0.0, 5.0, 6.0, 0.0, 0.0, 7.0, 8.0,
            ],
        );
        let patterns = analyzer.detect_structure(&m);
        assert!(patterns.contains(&MatrixPattern::BlockDiagonal));
    }

    #[test]
    fn test_pattern_library_initialize() {
        let mut lib = PatternLibrary::new();
        lib.initialize().unwrap();

        // All 12 patterns should be registered
        assert_eq!(lib.patterns.len(), 12);
        assert_eq!(lib.optimization_hints.len(), 12);
    }

    #[test]
    fn test_get_optimization_hint() {
        let mut lib = PatternLibrary::new();
        lib.initialize().unwrap();

        let hint = lib.get_optimization_hint(&MatrixPattern::Diagonal);
        assert!(hint.is_some());
        let h = hint.unwrap();
        assert_eq!(h.preferred_algorithm, "diagonal_scale");
        assert!(h.estimated_speedup > 0.0);

        let hint2 = lib.get_optimization_hint(&MatrixPattern::Sparse);
        assert!(hint2.is_some());
        assert_eq!(hint2.unwrap().preferred_algorithm, "sparse_gemm");
    }

    #[test]
    fn test_analyze_matrix() {
        let mut analyzer = make_analyzer();
        let m = make_matrix("s", 2, 2, vec![2.0, 1.0, 1.0, 2.0]);
        let result = analyzer.analyze_matrix(&m).unwrap();

        assert_eq!(result.matrix_id, "s");
        assert!(result.detected_patterns.contains(&MatrixPattern::Symmetric));
        assert!(result.detected_patterns.contains(&MatrixPattern::PositiveDefinite));
        assert!(!result.recommended_algorithms.is_empty());
    }

    #[test]
    fn test_analyze_matrix_sparsity() {
        let mut analyzer = make_analyzer();
        // 3x3 with 5 zeros out of 9 → sparsity = 5/9
        let m = make_matrix(
            "sp",
            3,
            3,
            vec![1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0],
        );
        let result = analyzer.analyze_matrix(&m).unwrap();
        assert!((result.sparsity - 6.0 / 9.0).abs() < 1e-10); // 6 zeros, 3 non-zeros
    }

    // ---- MatrixTransformer layout-conversion tests ----

    fn make_transformer() -> MatrixTransformer {
        let mut t = MatrixTransformer::new();
        t.initialize().unwrap();
        t
    }

    /// Build a column-major matrix from a row-major `data` description.
    /// `data` is given in row-major order (row by row); the returned matrix
    /// stores it in column-major order with `storage_format = ColumnMajor`.
    fn make_col_matrix(id: &str, rows: usize, cols: usize, row_major_data: Vec<f64>) -> Matrix {
        // Reorganise row-major input into column-major storage.
        let mut col_major = Vec::with_capacity(rows * cols);
        for j in 0..cols {
            for i in 0..rows {
                col_major.push(row_major_data[i * cols + j]);
            }
        }
        let metadata = MatrixMetadata {
            matrix_id: id.to_string(),
            rows,
            cols,
            data_type: DataType::Float64,
            storage_format: StorageFormat::ColumnMajor,
            compression: CompressionType::None,
            created_at: 0,
            last_accessed: 0,
            access_count: 0,
        };
        Matrix {
            matrix_id: id.to_string(),
            rows,
            cols,
            data_type: DataType::Float64,
            data: col_major,
            storage_format: StorageFormat::ColumnMajor,
            metadata,
        }
    }

    /// Read element (i, j) from a blocked-layout matrix with a known block size
    /// (row-major within blocks, row-major block order). Used to verify the
    /// blocked transform reorganised the data correctly.
    fn read_blocked(m: &Matrix, block_size: usize, i: usize, j: usize) -> f64 {
        let rows = m.rows;
        let cols = m.cols;
        let block_rows = (rows + block_size - 1) / block_size;
        let block_cols = (cols + block_size - 1) / block_size;
        let bi = i / block_size;
        let bj = j / block_size;
        let li = i % block_size; // local row within block
        let lj = j % block_size; // local col within block
        // Count elements in blocks preceding (bi, bj) in row-major block order.
        let mut offset = 0usize;
        'outer: for bbi in 0..block_rows {
            for bbj in 0..block_cols {
                if bbi == bi && bbj == bj {
                    break 'outer;
                }
                let i_end = rows.min((bbi + 1) * block_size);
                let j_end = cols.min((bbj + 1) * block_size);
                offset += (i_end - bbi * block_size) * (j_end - bbj * block_size);
            }
        }
        // Local block dimensions for block (bi, bj).
        let i_start = bi * block_size;
        let j_start = bj * block_size;
        let _local_rows = rows.min((bi + 1) * block_size) - i_start;
        let local_cols = cols.min((bj + 1) * block_size) - j_start;
        // Row-major within block.
        offset += li * local_cols + lj;
        m.data[offset]
    }

    #[test]
    fn test_row_to_col_major() {
        let transformer = make_transformer();
        // 2x3 matrix:
        //   [[1, 2, 3],
        //    [4, 5, 6]]
        let m = make_matrix("rm", 2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(m.storage_format, StorageFormat::RowMajor);

        let result = transformer
            .transform_matrix(&m, MatrixLayout::ColMajor)
            .unwrap();

        assert_eq!(result.storage_format, StorageFormat::ColumnMajor);
        assert_eq!(result.metadata.storage_format, StorageFormat::ColumnMajor);
        assert_eq!(result.rows, 2);
        assert_eq!(result.cols, 3);
        // Column-major: column 0 = [1, 4], column 1 = [2, 5], column 2 = [3, 6]
        assert_eq!(result.data, vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
        // Verify every logical element is preserved under col-major addressing.
        for i in 0..2 {
            for j in 0..3 {
                let expected = m.data[i * 3 + j];
                let got = result.data[j * 2 + i];
                assert_eq!(got, expected, "element ({},{}) mismatch", i, j);
            }
        }
    }

    #[test]
    fn test_col_to_row_major() {
        let transformer = make_transformer();
        // Build a column-major matrix representing [[1,2,3],[4,5,6]].
        let m = make_col_matrix("cm", 2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(m.storage_format, StorageFormat::ColumnMajor);
        assert_eq!(m.data, vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);

        let result = transformer
            .transform_matrix(&m, MatrixLayout::RowMajor)
            .unwrap();

        assert_eq!(result.storage_format, StorageFormat::RowMajor);
        assert_eq!(result.rows, 2);
        assert_eq!(result.cols, 3);
        // Row-major: [[1,2,3],[4,5,6]] flattened row by row.
        assert_eq!(result.data, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        // Verify every logical element is preserved under row-major addressing.
        for i in 0..2 {
            for j in 0..3 {
                let expected = m.data[j * 2 + i]; // col-major source read
                let got = result.data[i * 3 + j]; // row-major result read
                assert_eq!(got, expected, "element ({},{}) mismatch", i, j);
            }
        }
    }

    #[test]
    fn test_blocked_layout() {
        let transformer = make_transformer();
        // 4x4 matrix 1..16 (row-major):
        //   [[ 1,  2,  3,  4],
        //    [ 5,  6,  7,  8],
        //    [ 9, 10, 11, 12],
        //    [13, 14, 15, 16]]
        let m = make_matrix(
            "blk",
            4,
            4,
            vec![
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
                16.0,
            ],
        );

        let result = transformer
            .transform_matrix(&m, MatrixLayout::Blocked(Box::new(MatrixLayout::RowMajor), 2))
            .unwrap();

        assert_eq!(result.storage_format, StorageFormat::Blocked);
        assert_eq!(result.rows, 4);
        assert_eq!(result.cols, 4);
        assert_eq!(result.data.len(), 16);
        // Expected block order (2x2 blocks, row-major within each block):
        //   block(0,0) = [1, 2, 5, 6]
        //   block(0,1) = [3, 4, 7, 8]
        //   block(1,0) = [9, 10, 13, 14]
        //   block(1,1) = [11, 12, 15, 16]
        assert_eq!(
            result.data,
            vec![
                1.0, 2.0, 5.0, 6.0, 3.0, 4.0, 7.0, 8.0, 9.0, 10.0, 13.0, 14.0, 11.0, 12.0, 15.0,
                16.0,
            ]
        );
        // Verify every logical element is recoverable from the blocked buffer.
        for i in 0..4 {
            for j in 0..4 {
                let expected = m.data[i * 4 + j];
                let got = read_blocked(&result, 2, i, j);
                assert_eq!(got, expected, "blocked element ({},{}) mismatch", i, j);
            }
        }
    }

    #[test]
    fn test_optimize_layout_row_access() {
        let transformer = make_transformer();
        let m = make_matrix("r", 2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        // Sequential access = row-heavy traversal -> RowMajor.
        let result = transformer
            .optimize_layout(&m, &AccessPattern::Sequential)
            .unwrap();
        assert_eq!(result.storage_format, StorageFormat::RowMajor);
        // Row-major data is unchanged from the row-major source.
        assert_eq!(result.data, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_optimize_layout_col_access() {
        let transformer = make_transformer();
        let m = make_matrix("c", 2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        // Strided access = column-heavy traversal -> ColMajor.
        let result = transformer
            .optimize_layout(&m, &AccessPattern::Strided)
            .unwrap();
        assert_eq!(result.storage_format, StorageFormat::ColumnMajor);
        // Column-major reorganisation: [1, 4, 2, 5, 3, 6].
        assert_eq!(result.data, vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
    }
}
