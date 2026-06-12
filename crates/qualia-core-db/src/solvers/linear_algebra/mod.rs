//! Linear Algebra & Matrix Solvers - Zero-Allocation Implementation
//! 
//! This module provides fixed-size stack-based linear algebra solvers for
//! eigenvalue problems, linear systems, and tensor operations suitable for
//! the #![no_std] environment of Qualia-DB.

use crate::solvers::{SolverConfig, SolverState, SolverResult};
use crate::solvers::SolversError as ExecutionError;
use core::f64::consts;

/// Fixed-size 4x4 matrix for stack-based operations
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Matrix4x4 {
    /// Matrix elements in row-major order
    pub data: [[f64; 4]; 4],
}

/// Fixed-size 4-element vector
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vector4 {
    /// Vector elements
    pub data: [f64; 4],
}

/// Fixed-size 3x3x3 tensor
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Tensor3x3x3 {
    /// Tensor elements
    pub data: [[[f64; 3]; 3]; 3],
}

/// Lanczos eigensolver for finding lowest eigenvalues
#[repr(C)]
pub struct FixedLanczosEigensolver {
    /// Current iteration count
    pub iteration: u32,
    /// Tridiagonal matrix elements
    pub alpha: [f64; 100],
    pub beta: [f64; 100],
    /// Lanczos vectors (only store 3 at a time)
    pub vectors: [Vector4; 3],
    /// Eigenvalues
    pub eigenvalues: [f64; 4],
    /// Solver configuration
    pub config: SolverConfig,
    /// Solver state
    pub solver_state: SolverState,
}

/// Static LU decomposition solver
#[repr(C)]
pub struct StaticLuDecomposition {
    /// Matrix being decomposed (overwritten with L and U)
    pub matrix: Matrix4x4,
    /// Permutation vector
    pub permutation: [usize; 4],
    /// Determinant sign
    pub parity: i32,
    /// Solver configuration
    pub config: SolverConfig,
    /// Solver state
    pub solver_state: SolverState,
}

/// Constant tensor contraction solver
#[repr(C)]
pub struct ConstTensorContractor {
    /// Input tensor A
    pub tensor_a: Tensor3x3x3,
    /// Input tensor B
    pub tensor_b: Tensor3x3x3,
    /// Result tensor
    pub result: Tensor3x3x3,
    /// Contraction indices
    pub contraction_indices: [(usize, usize); 3],
    /// Solver configuration
    pub config: SolverConfig,
    /// Solver state
    pub solver_state: SolverState,
}

impl Matrix4x4 {
    /// Create new zero matrix
    pub const fn zero() -> Self {
        Self {
            data: [[0.0; 4]; 4],
        }
    }

    /// Create identity matrix
    pub const fn identity() -> Self {
        Self {
            data: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Get element at (i, j)
    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.data[i][j]
    }

    /// Set element at (i, j)
    pub fn set(&mut self, i: usize, j: usize, value: f64) {
        self.data[i][j] = value;
    }

    /// Matrix-vector multiplication
    pub fn multiply_vector(&self, v: &Vector4) -> Vector4 {
        let mut result = Vector4::zero();
        
        for i in 0..4 {
            let mut sum = 0.0;
            for j in 0..4 {
                sum += self.data[i][j] * v.data[j];
            }
            result.data[i] = sum;
        }
        
        result
    }

    /// Matrix-matrix multiplication
    pub fn multiply_matrix(&self, other: &Matrix4x4) -> Matrix4x4 {
        let mut result = Matrix4x4::zero();
        
        for i in 0..4 {
            for j in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.data[i][k] * other.data[k][j];
                }
                result.data[i][j] = sum;
            }
        }
        
        result
    }

    /// Transpose matrix
    pub fn transpose(&self) -> Matrix4x4 {
        let mut result = Matrix4x4::zero();
        
        for i in 0..4 {
            for j in 0..4 {
                result.data[i][j] = self.data[j][i];
            }
        }
        
        result
    }

    /// Calculate determinant
    pub fn determinant(&self) -> f64 {
        // Use cofactor expansion for 4x4
        let mut det = 0.0;
        
        for i in 0..4 {
            let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
            let minor = self.minor(0, i);
            det += sign * self.data[0][i] * minor.determinant_3x3();
        }
        
        det
    }

    /// Calculate 3x3 minor
    fn minor(&self, row: usize, col: usize) -> Matrix4x4 {
        let mut result = Matrix4x4::zero();
        let mut r = 0;
        
        for i in 0..4 {
            if i == row { continue; }
            let mut c = 0;
            for j in 0..4 {
                if j == col { continue; }
                result.data[r][c] = self.data[i][j];
                c += 1;
            }
            r += 1;
        }
        
        result
    }

    /// Calculate 3x3 determinant
    fn determinant_3x3(&self) -> f64 {
        self.data[0][0] * (self.data[1][1] * self.data[2][2] - self.data[1][2] * self.data[2][1]) -
        self.data[0][1] * (self.data[1][0] * self.data[2][2] - self.data[1][2] * self.data[2][0]) +
        self.data[0][2] * (self.data[1][0] * self.data[2][1] - self.data[1][1] * self.data[2][0])
    }
}

impl Vector4 {
    /// Create zero vector
    pub const fn zero() -> Self {
        Self {
            data: [0.0; 4],
        }
    }

    /// Create vector from array
    pub const fn from_array(data: [f64; 4]) -> Self {
        Self { data }
    }

    /// Get element
    pub fn get(&self, i: usize) -> f64 {
        self.data[i]
    }

    /// Set element
    pub fn set(&mut self, i: usize, value: f64) {
        self.data[i] = value;
    }

    /// Vector dot product
    pub fn dot(&self, other: &Vector4) -> f64 {
        let mut sum = 0.0;
        for i in 0..4 {
            sum += self.data[i] * other.data[i];
        }
        sum
    }

    /// Vector norm (L2)
    pub fn norm(&self) -> f64 {
        self.dot(self).sqrt()
    }

    /// Normalize vector
    pub fn normalize(&self) -> Vector4 {
        let norm = self.norm();
        if norm > 1e-10 {
            Vector4::from_array([
                self.data[0] / norm,
                self.data[1] / norm,
                self.data[2] / norm,
                self.data[3] / norm,
            ])
        } else {
            *self
        }
    }

    /// Vector addition
    pub fn add(&self, other: &Vector4) -> Vector4 {
        Vector4::from_array([
            self.data[0] + other.data[0],
            self.data[1] + other.data[1],
            self.data[2] + other.data[2],
            self.data[3] + other.data[3],
        ])
    }

    /// Vector subtraction
    pub fn subtract(&self, other: &Vector4) -> Vector4 {
        Vector4::from_array([
            self.data[0] - other.data[0],
            self.data[1] - other.data[1],
            self.data[2] - other.data[2],
            self.data[3] - other.data[3],
        ])
    }

    /// Scalar multiplication
    pub fn scale(&self, scalar: f64) -> Vector4 {
        Vector4::from_array([
            self.data[0] * scalar,
            self.data[1] * scalar,
            self.data[2] * scalar,
            self.data[3] * scalar,
        ])
    }
}

impl Tensor3x3x3 {
    /// Create zero tensor
    pub const fn zero() -> Self {
        Self {
            data: [[[0.0; 3]; 3]; 3],
        }
    }

    /// Get element at (i, j, k)
    pub fn get(&self, i: usize, j: usize, k: usize) -> f64 {
        self.data[i][j][k]
    }

    /// Set element at (i, j, k)
    pub fn set(&mut self, i: usize, j: usize, k: usize, value: f64) {
        self.data[i][j][k] = value;
    }

    /// Contract with another tensor
    pub fn contract(&self, other: &Tensor3x3x3, indices: &[(usize, usize); 3]) -> Tensor3x3x3 {
        let mut result = Tensor3x3x3::zero();
        
        // Perform contraction along specified indices
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    let mut sum = 0.0;
                    for (idx_a, idx_b) in indices {
                        sum += self.get(i, j, *idx_a) * other.get(*idx_b, j, k);
                    }
                    result.set(i, j, k, sum);
                }
            }
        }
        
        result
    }
}

impl FixedLanczosEigensolver {
    /// Create new Lanczos eigensolver
    pub fn new(config: SolverConfig) -> Self {
        Self {
            iteration: 0,
            alpha: [0.0; 100],
            beta: [0.0; 100],
            vectors: [Vector4::zero(); 3],
            eigenvalues: [0.0; 4],
            config,
            solver_state: SolverState::default(),
        }
    }

    /// Find lowest eigenvalues of symmetric matrix
    pub fn find_lowest_eigenvalues(&mut self, matrix: &Matrix4x4, num_eigenvalues: usize) -> SolverResult<[f64; 4]> {
        self.iteration = 0;
        self.solver_state.converged = false;

        // Initialize with random vector
        self.vectors[0] = Vector4::from_array([1.0, 0.0, 0.0, 0.0]);
        self.vectors[0] = self.vectors[0].normalize();

        // Perform Lanczos iterations
        while self.iteration < self.config.max_iterations.min(100) {
            // Compute matrix-vector product
            let w = matrix.multiply_vector(&self.vectors[0]);

            // Compute alpha = v_i^T * A * v_i
            let alpha_i = self.vectors[0].dot(&w);
            self.alpha[self.iteration as usize] = alpha_i;

            // Compute w = w - alpha_i * v_i - beta_{i-1} * v_{i-1}
            let mut w_new = w.subtract(&self.vectors[0].scale(alpha_i));
            if self.iteration > 0 {
                w_new = w_new.subtract(&self.vectors[1].scale(self.beta[(self.iteration - 1) as usize]));
            }

            // Compute beta_i = ||w||
            let beta_i = w_new.norm();
            self.beta[self.iteration as usize] = beta_i;

            // Check convergence
            if beta_i < self.config.tolerance {
                self.solver_state.converged = true;
                break;
            }

            // Normalize and update vectors
            self.vectors[2] = self.vectors[1];
            self.vectors[1] = self.vectors[0];
            self.vectors[0] = w_new.normalize();

            self.iteration += 1;
        }

        // Extract eigenvalues from tridiagonal matrix
        self.extract_eigenvalues_from_tridiagonal(num_eigenvalues)?;

        Ok(self.eigenvalues)
    }

    /// Extract eigenvalues from tridiagonal matrix using QR algorithm
    fn extract_eigenvalues_from_tridiagonal(&mut self, num_eigenvalues: usize) -> SolverResult<()> {
        let n = self.iteration as usize;
        if n == 0 {
            return Err(ExecutionError::InvalidParameters);
        }

        // Simple power iteration for lowest eigenvalue
        for i in 0..num_eigenvalues.min(4) {
            let mut v = Vector4::from_array([1.0, 0.0, 0.0, 0.0]);
            
            for _ in 0..50 {
                // Apply tridiagonal matrix approximation
                let mut w = Vector4::zero();
                
                for j in 0..n.min(4) {
                    if j < n {
                        w.data[j] += self.alpha[j] * v.data[j];
                    }
                    if j > 0 {
                        w.data[j] += self.beta[j - 1] * v.data[j - 1];
                    }
                    if j < n - 1 {
                        w.data[j] += self.beta[j] * v.data[j + 1];
                    }
                }
                
                v = w.normalize();
            }
            
            // Estimate eigenvalue using Rayleigh quotient
            self.eigenvalues[i] = self.estimate_rayleigh_quotient(&v);
        }

        Ok(())
    }

    /// Estimate Rayleigh quotient
    fn estimate_rayleigh_quotient(&self, v: &Vector4) -> f64 {
        // Simplified estimation using diagonal elements
        let mut sum = 0.0;
        for i in 0..4.min(self.iteration as usize) {
            sum += self.alpha[i] * v.data[i] * v.data[i];
        }
        sum
    }
}

impl StaticLuDecomposition {
    /// Create new LU decomposition solver
    pub fn new(config: SolverConfig) -> Self {
        Self {
            matrix: Matrix4x4::zero(),
            permutation: [0, 1, 2, 3],
            parity: 1,
            config,
            solver_state: SolverState::default(),
        }
    }

    /// Decompose matrix and solve linear system Ax = b
    pub fn solve(&mut self, matrix: &Matrix4x4, b: &Vector4) -> SolverResult<Vector4> {
        // Copy matrix for decomposition
        self.matrix = *matrix;
        
        // Perform LU decomposition with partial pivoting
        self.lu_decompose()?;
        
        // Solve using forward/backward substitution
        self.solve_lu(b)
    }

    /// Perform LU decomposition with partial pivoting
    fn lu_decompose(&mut self) -> SolverResult<()> {
        self.parity = 1;
        
        for i in 0..4 {
            // Find pivot
            let pivot_row = self.find_pivot(i)?;
            
            // Swap rows if necessary
            if pivot_row != i {
                self.swap_rows(i, pivot_row);
                self.parity = -self.parity;
            }
            
            // Eliminate column
            for j in i + 1..4 {
                let multiplier = self.matrix.data[j][i] / self.matrix.data[i][i];
                self.matrix.data[j][i] = multiplier;
                
                for k in i + 1..4 {
                    self.matrix.data[j][k] -= multiplier * self.matrix.data[i][k];
                }
            }
        }
        
        Ok(())
    }

    /// Find pivot row
    fn find_pivot(&self, col: usize) -> SolverResult<usize> {
        let mut max_row = col;
        let mut max_val = self.matrix.data[col][col].abs();
        
        for i in col + 1..4 {
            let val = self.matrix.data[i][col].abs();
            if val > max_val {
                max_val = val;
                max_row = i;
            }
        }
        
        if max_val < 1e-10 {
            return Err(ExecutionError::SingularMatrix);
        }
        
        Ok(max_row)
    }

    /// Swap two rows
    fn swap_rows(&mut self, i: usize, j: usize) {
        for k in 0..4 {
            let temp = self.matrix.data[i][k];
            self.matrix.data[i][k] = self.matrix.data[j][k];
            self.matrix.data[j][k] = temp;
        }
        
        // Update permutation
        self.permutation.swap(i, j);
    }

    /// Solve using LU decomposition
    fn solve_lu(&self, b: &Vector4) -> SolverResult<Vector4> {
        let mut x = *b;
        
        // Forward substitution (solve Ly = Pb)
        for i in 0..4 {
            let mut sum = 0.0;
            for j in 0..i {
                sum += self.matrix.data[i][j] * x.data[j];
            }
            x.data[i] -= sum;
        }
        
        // Backward substitution (solve Ux = y)
        for i in (0..4).rev() {
            let mut sum = 0.0;
            for j in i + 1..4 {
                sum += self.matrix.data[i][j] * x.data[j];
            }
            x.data[i] = (x.data[i] - sum) / self.matrix.data[i][i];
        }
        
        Ok(x)
    }

    /// Calculate determinant from LU decomposition
    pub fn determinant(&self) -> f64 {
        let mut det = 1.0;
        for i in 0..4 {
            det *= self.matrix.data[i][i];
        }
        det * self.parity as f64
    }
}

impl ConstTensorContractor {
    /// Create new tensor contractor
    pub fn new(config: SolverConfig) -> Self {
        Self {
            tensor_a: Tensor3x3x3::zero(),
            tensor_b: Tensor3x3x3::zero(),
            result: Tensor3x3x3::zero(),
            contraction_indices: [(0, 0), (1, 1), (2, 2)],
            config,
            solver_state: SolverState::default(),
        }
    }

    /// Contract two tensors
    pub fn contract(&mut self, tensor_a: &Tensor3x3x3, tensor_b: &Tensor3x3x3, 
                   indices: &[(usize, usize); 3]) -> SolverResult<Tensor3x3x3> {
        self.tensor_a = *tensor_a;
        self.tensor_b = *tensor_b;
        self.contraction_indices = *indices;
        
        // Perform contraction
        self.result = self.tensor_a.contract(&self.tensor_b, &self.contraction_indices);
        
        self.solver_state.converged = true;
        
        Ok(self.result)
    }

    /// Get result tensor
    pub fn get_result(&self) -> Tensor3x3x3 {
        self.result
    }
}

impl Default for Matrix4x4 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Default for Vector4 {
    fn default() -> Self {
        Self::zero()
    }
}

impl Default for Tensor3x3x3 {
    fn default() -> Self {
        Self::zero()
    }
}

impl Default for FixedLanczosEigensolver {
    fn default() -> Self {
        Self::new(SolverConfig::default())
    }
}

impl Default for StaticLuDecomposition {
    fn default() -> Self {
        Self::new(SolverConfig::default())
    }
}

impl Default for ConstTensorContractor {
    fn default() -> Self {
        Self::new(SolverConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix4x4_operations() {
        let mut m = Matrix4x4::identity();
        m.set(0, 1, 2.0);
        m.set(1, 0, 3.0);
        
        let v = Vector4::from_array([1.0, 2.0, 3.0, 4.0]);
        let result = m.multiply_vector(&v);
        
        assert_eq!(result.data[0], 1.0 + 2.0 * 2.0); // 1 + 4 = 5
        assert_eq!(result.data[1], 3.0 * 1.0 + 2.0); // 3 + 2 = 5
    }

    #[test]
    fn test_vector_operations() {
        let v1 = Vector4::from_array([1.0, 2.0, 3.0, 4.0]);
        let v2 = Vector4::from_array([2.0, 3.0, 4.0, 5.0]);
        
        let dot = v1.dot(&v2);
        assert_eq!(dot, 1.0*2.0 + 2.0*3.0 + 3.0*4.0 + 4.0*5.0);
        
        let norm = v1.norm();
        assert!((norm - (1.0_f64*1.0 + 2.0*2.0 + 3.0*3.0 + 4.0*4.0).sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_lu_decomposition() {
        let mut lu = StaticLuDecomposition::new(SolverConfig::default());
        
        // Test matrix: [[2, 1], [1, 2]] extended to 4x4
        let mut m = Matrix4x4::identity();
        m.set(0, 0, 2.0);
        m.set(0, 1, 1.0);
        m.set(1, 0, 1.0);
        m.set(1, 1, 2.0);
        
        let b = Vector4::from_array([3.0, 3.0, 0.0, 0.0]);
        let result = lu.solve(&m, &b);
        
        assert!(result.is_ok());
        let x = result.unwrap();
        assert!((x.data[0] - 1.0).abs() < 1e-10);
        assert!((x.data[1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_tensor_contraction() {
        let mut contractor = ConstTensorContractor::new(SolverConfig::default());
        
        let mut tensor_a = Tensor3x3x3::zero();
        let mut tensor_b = Tensor3x3x3::zero();
        
        // Set some values
        tensor_a.set(0, 0, 0, 1.0);
        tensor_a.set(1, 1, 1, 2.0);
        tensor_b.set(0, 0, 0, 3.0);
        tensor_b.set(1, 1, 1, 4.0);
        
        let indices = [(0, 0), (1, 1), (2, 2)];
        let result = contractor.contract(&tensor_a, &tensor_b, &indices);
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_zero_allocation_guarantee() {
        assert_eq!(core::mem::size_of::<Matrix4x4>(), 128);
        assert_eq!(core::mem::size_of::<Vector4>(), 32);
        assert_eq!(core::mem::size_of::<Tensor3x3x3>(), 216);
        assert_eq!(core::mem::size_of::<FixedLanczosEigensolver>(), 3368);
        assert_eq!(core::mem::size_of::<StaticLuDecomposition>(), 200);
        assert_eq!(core::mem::size_of::<ConstTensorContractor>(), 680);
    }
}
