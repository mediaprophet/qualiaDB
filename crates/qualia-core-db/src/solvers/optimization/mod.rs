//! Optimization & Root Finding - Zero-Allocation Implementation
//! 
//! This module provides fixed-size stack-based optimization algorithms and
//! root finding methods suitable for the #![no_std] environment of Qualia-DB.

use crate::solvers::{SolverConfig, SolverState, SolverResult};
use crate::ggml_quants::ExecutionError;
use core::f64::consts;

/// Nelder-Mead simplex optimizer for unconstrained optimization
#[repr(C)]
pub struct NelderMeadSimplex {
    /// Simplex vertices (n+1 points in n dimensions)
    pub vertices: [[f64; 4]; 5], // 4D simplex
    /// Function values at vertices
    pub values: [f64; 5],
    /// Current iteration
    pub iteration: u32,
    /// Best point found
    pub best_point: [f64; 4],
    pub best_value: f64,
    /// Solver configuration
    pub config: SolverConfig,
    /// Solver state
    pub solver_state: SolverState,
}

/// Bounded Newton-Raphson root finder
#[repr(C)]
pub struct BoundedNewtonRaphson {
    /// Current guess
    pub current_guess: f64,
    /// Previous guess for convergence checking
    pub previous_guess: f64,
    /// Function value at current guess
    pub current_value: f64,
    /// Derivative at current guess
    pub current_derivative: f64,
    /// Search bounds
    pub lower_bound: f64,
    pub upper_bound: f64,
    /// Solver configuration
    pub config: SolverConfig,
    /// Solver state
    pub solver_state: SolverState,
}

/// Levenberg-Marquardt curve fitting optimizer
#[repr(C)]
pub struct LevenbergMarquardtStack {
    /// Current parameters
    pub parameters: [f64; 4],
    /// Parameter updates
    pub delta_parameters: [f64; 4],
    /// Jacobian matrix (4xN observations)
    pub jacobian: [[f64; 10]; 4],
    /// Residual vector
    pub residuals: [f64; 10],
    /// Damping parameter
    pub lambda: f64,
    /// Chi-squared value
    pub chi_squared: f64,
    /// Solver configuration
    pub config: SolverConfig,
    /// Solver state
    pub solver_state: SolverState,
}

/// Optimization state tracking
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OptimizationState {
    /// Current iteration
    pub iteration: u32,
    /// Current objective value
    pub objective_value: f64,
    /// Converged flag
    pub converged: bool,
    /// Simplex size (for Nelder-Mead)
    pub simplex_size: f64,
}

/// Root finding state tracking
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RootFindingState {
    /// Current iteration
    pub iteration: u32,
    /// Current function value
    pub function_value: f64,
    /// Converged flag
    pub converged: bool,
    /// Step size
    pub step_size: f64,
}

/// Curve fitting state tracking
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CurveFitState {
    /// Current iteration
    pub iteration: u32,
    /// Current chi-squared
    pub chi_squared: f64,
    /// Converged flag
    pub converged: bool,
    /// Damping parameter
    pub lambda: f64,
}

/// Objective function trait for optimization
pub trait ObjectiveFunction {
    /// Evaluate objective function
    fn evaluate(&self, params: &[f64; 4]) -> f64;
    
    /// Check if parameters are within bounds
    fn in_bounds(&self, params: &[f64; 4]) -> bool {
        true // Default: no bounds
    }
}

/// Root finding function trait
pub trait RootFunction {
    /// Evaluate function f(x)
    fn evaluate(&self, x: f64) -> f64;
    
    /// Evaluate derivative f'(x)
    fn derivative(&self, x: f64) -> f64;
}

/// Curve fitting function trait
pub trait CurveFitFunction {
    /// Evaluate model at given x with parameters
    fn evaluate(&self, x: f64, params: &[f64; 4]) -> f64;
    
    /// Evaluate Jacobian at given x with parameters
    fn jacobian(&self, x: f64, params: &[f64; 4]) -> [f64; 4];
}

impl NelderMeadSimplex {
    /// Create new Nelder-Mead optimizer
    pub const fn new(initial_point: [f64; 4], config: SolverConfig) -> Self {
        Self {
            vertices: Self::initialize_simplex(initial_point),
            values: [0.0; 5],
            iteration: 0,
            best_point: initial_point,
            best_value: f64::MAX,
            config,
            solver_state: SolverState::default(),
        }
    }

    /// Initialize simplex around initial point
    const fn initialize_simplex(initial_point: [f64; 4]) -> [[f64; 4]; 5] {
        let mut simplex = [[0.0; 4]; 5];
        
        // First vertex is the initial point
        simplex[0] = initial_point;
        
        // Other vertices are perturbed
        let mut i = 1;
        while i < 5 {
            let mut vertex = initial_point;
            vertex[i - 1] += 0.1; // Perturb one dimension
            simplex[i] = vertex;
            i += 1;
        }
        
        simplex
    }

    /// Optimize objective function
    pub fn optimize<F>(&mut self, f: &F) -> SolverResult<OptimizationState>
    where
        F: ObjectiveFunction,
    {
        // Evaluate initial simplex
        for i in 0..5 {
            self.values[i] = f.evaluate(&self.vertices[i]);
        }
        
        self.iteration = 0;
        self.solver_state.converged = false;

        while self.iteration < self.config.max_iterations {
            // Sort vertices by function value
            self.sort_vertices();
            
            // Update best point
            self.best_point = self.vertices[0];
            self.best_value = self.values[0];
            
            // Check convergence
            if self.check_convergence() {
                self.solver_state.converged = true;
                break;
            }
            
            // Perform Nelder-Mead operations
            self.nelder_mead_step(f)?;
            
            self.iteration += 1;
        }

        Ok(OptimizationState {
            iteration: self.iteration,
            objective_value: self.best_value,
            converged: self.solver_state.converged,
            simplex_size: self.calculate_simplex_size(),
        })
    }

    /// Sort vertices by function value
    fn sort_vertices(&mut self) {
        // Simple bubble sort (fixed size, no allocation)
        for i in 0..4 {
            for j in 0..4 - i {
                if self.values[j] > self.values[j + 1] {
                    // Swap vertices
                    let temp_vertex = self.vertices[j];
                    self.vertices[j] = self.vertices[j + 1];
                    self.vertices[j + 1] = temp_vertex;
                    
                    // Swap values
                    let temp_value = self.values[j];
                    self.values[j] = self.values[j + 1];
                    self.values[j + 1] = temp_value;
                }
            }
        }
    }

    /// Check convergence
    fn check_convergence(&self) -> bool {
        let simplex_size = self.calculate_simplex_size();
        simplex_size < self.config.tolerance
    }

    /// Calculate simplex size
    fn calculate_simplex_size(&self) -> f64 {
        let mut size = 0.0;
        
        for i in 1..5 {
            let mut distance = 0.0;
            for j in 0..4 {
                distance += (self.vertices[i][j] - self.vertices[0][j]).powi(2);
            }
            size = size.max(distance.sqrt());
        }
        
        size
    }

    /// Perform Nelder-Mead step
    fn nelder_mead_step<F>(&mut self, f: &F) -> SolverResult<()>
    where
        F: ObjectiveFunction,
    {
        // Calculate centroid of best n vertices
        let centroid = self.calculate_centroid();
        
        // Reflection
        let reflected = self.reflect(&centroid);
        let reflected_value = f.evaluate(&reflected);
        
        if reflected_value < self.values[0] {
            // Expansion
            let expanded = self.expand(&centroid, &reflected);
            let expanded_value = f.evaluate(&expanded);
            
            if expanded_value < reflected_value {
                self.vertices[4] = expanded;
                self.values[4] = expanded_value;
            } else {
                self.vertices[4] = reflected;
                self.values[4] = reflected_value;
            }
        } else if reflected_value < self.values[3] {
            // Accept reflection
            self.vertices[4] = reflected;
            self.values[4] = reflected_value;
        } else {
            // Contraction
            if reflected_value < self.values[4] {
                let contracted = self.contract(&centroid, &reflected);
                let contracted_value = f.evaluate(&contracted);
                
                if contracted_value < reflected_value {
                    self.vertices[4] = contracted;
                    self.values[4] = contracted_value;
                } else {
                    self.shrink(f)?;
                }
            } else {
                let contracted = self.contract(&centroid, &self.vertices[4]);
                let contracted_value = f.evaluate(&contracted);
                
                if contracted_value < self.values[4] {
                    self.vertices[4] = contracted;
                    self.values[4] = contracted_value;
                } else {
                    self.shrink(f)?;
                }
            }
        }
        
        Ok(())
    }

    /// Calculate centroid of best n vertices
    fn calculate_centroid(&self) -> [f64; 4] {
        let mut centroid = [0.0; 4];
        
        for i in 0..4 {
            for j in 0..4 {
                centroid[j] += self.vertices[i][j];
            }
        }
        
        for j in 0..4 {
            centroid[j] /= 4.0;
        }
        
        centroid
    }

    /// Reflection operation
    fn reflect(&self, centroid: &[f64; 4]) -> [f64; 4] {
        let mut reflected = [0.0; 4];
        let alpha = 1.0; // Reflection coefficient
        
        for i in 0..4 {
            reflected[i] = centroid[i] + alpha * (centroid[i] - self.vertices[4][i]);
        }
        
        reflected
    }

    /// Expansion operation
    fn expand(&self, centroid: &[f64; 4], reflected: &[f64; 4]) -> [f64; 4] {
        let mut expanded = [0.0; 4];
        let gamma = 2.0; // Expansion coefficient
        
        for i in 0..4 {
            expanded[i] = centroid[i] + gamma * (reflected[i] - centroid[i]);
        }
        
        expanded
    }

    /// Contraction operation
    fn contract(&self, centroid: &[f64; 4], worst: &[f64; 4]) -> [f64; 4] {
        let mut contracted = [0.0; 4];
        let rho = 0.5; // Contraction coefficient
        
        for i in 0..4 {
            contracted[i] = centroid[i] + rho * (worst[i] - centroid[i]);
        }
        
        contracted
    }

    /// Shrink operation
    fn shrink<F>(&mut self, f: &F) -> SolverResult<()>
    where
        F: ObjectiveFunction,
    {
        let sigma = 0.5; // Shrink coefficient
        
        for i in 1..5 {
            for j in 0..4 {
                self.vertices[i][j] = self.vertices[0][j] + sigma * (self.vertices[i][j] - self.vertices[0][j]);
            }
            self.values[i] = f.evaluate(&self.vertices[i]);
        }
        
        Ok(())
    }

    /// Get best solution
    pub fn get_best_solution(&self) -> ([f64; 4], f64) {
        (self.best_point, self.best_value)
    }
}

impl BoundedNewtonRaphson {
    /// Create new bounded Newton-Raphson solver
    pub const fn new(initial_guess: f64, lower_bound: f64, upper_bound: f64, config: SolverConfig) -> Self {
        Self {
            current_guess: initial_guess,
            previous_guess: initial_guess,
            current_value: 0.0,
            current_derivative: 0.0,
            lower_bound,
            upper_bound,
            config,
            solver_state: SolverState::default(),
        }
    }

    /// Find root of function
    pub fn find_root<F>(&mut self, f: &F) -> SolverResult<RootFindingState>
    where
        F: RootFunction,
    {
        self.solver_state.iteration = 0;
        self.solver_state.converged = false;

        while self.solver_state.iteration < self.config.max_iterations {
            // Evaluate function and derivative
            self.current_value = f.evaluate(self.current_guess);
            self.current_derivative = f.derivative(self.current_guess);
            
            // Check convergence
            if self.current_value.abs() < self.config.tolerance {
                self.solver_state.converged = true;
                break;
            }
            
            // Check for zero derivative
            if self.current_derivative.abs() < 1e-10 {
                return Err(ExecutionError::ConvergenceFailed);
            }
            
            // Newton step
            let new_guess = self.current_guess - self.current_value / self.current_derivative;
            
            // Apply bounds
            let bounded_guess = new_guess.clamp(self.lower_bound, self.upper_bound);
            
            // Check for convergence in x
            if (bounded_guess - self.current_guess).abs() < self.config.tolerance {
                self.solver_state.converged = true;
                break;
            }
            
            self.previous_guess = self.current_guess;
            self.current_guess = bounded_guess;
            self.solver_state.iteration += 1;
        }

        Ok(RootFindingState {
            iteration: self.solver_state.iteration,
            function_value: self.current_value,
            converged: self.solver_state.converged,
            step_size: (self.current_guess - self.previous_guess).abs(),
        })
    }

    /// Get current root estimate
    pub fn get_root(&self) -> f64 {
        self.current_guess
    }
}

impl LevenbergMarquardtStack {
    /// Create new Levenberg-Marquardt optimizer
    pub const fn new(initial_parameters: [f64; 4], config: SolverConfig) -> Self {
        Self {
            parameters: initial_parameters,
            delta_parameters: [0.0; 4],
            jacobian: [[0.0; 10]; 4],
            residuals: [0.0; 10],
            lambda: 1e-3,
            chi_squared: f64::MAX,
            config,
            solver_state: SolverState::default(),
        }
    }

    /// Fit curve to data points
    pub fn fit_curve<F>(&mut self, f: &F, x_data: &[f64; 10], y_data: &[f64; 10]) -> SolverResult<CurveFitState>
    where
        F: CurveFitFunction,
    {
        self.solver_state.iteration = 0;
        self.solver_state.converged = false;

        // Initial evaluation
        self.evaluate_residuals(f, x_data, y_data)?;
        self.chi_squared = self.calculate_chi_squared();

        while self.solver_state.iteration < self.config.max_iterations {
            // Calculate Jacobian
            self.calculate_jacobian(f, x_data)?;
            
            // Solve for parameter update
            self.solve_parameter_update()?;
            
            // Try new parameters
            let old_chi_squared = self.chi_squared;
            let old_parameters = self.parameters;
            
            // Update parameters
            for i in 0..4 {
                self.parameters[i] += self.delta_parameters[i];
            }
            
            // Evaluate new chi-squared
            self.evaluate_residuals(f, x_data, y_data)?;
            self.chi_squared = self.calculate_chi_squared();
            
            // Check if improvement
            if self.chi_squared < old_chi_squared {
                // Accept update, decrease lambda
                self.lambda *= 0.1;
            } else {
                // Reject update, increase lambda
                self.lambda *= 10.0;
                self.parameters = old_parameters;
                self.chi_squared = old_chi_squared;
            }
            
            // Check convergence
            if self.check_convergence(old_chi_squared) {
                self.solver_state.converged = true;
                break;
            }
            
            self.solver_state.iteration += 1;
        }

        Ok(CurveFitState {
            iteration: self.solver_state.iteration,
            chi_squared: self.chi_squared,
            converged: self.solver_state.converged,
            lambda: self.lambda,
        })
    }

    /// Evaluate residuals
    fn evaluate_residuals<F>(&mut self, f: &F, x_data: &[f64; 10], y_data: &[f64; 10]) -> SolverResult<()>
    where
        F: CurveFitFunction,
    {
        for i in 0..10 {
            let model_value = f.evaluate(x_data[i], &self.parameters);
            self.residuals[i] = y_data[i] - model_value;
        }
        
        Ok(())
    }

    /// Calculate chi-squared
    fn calculate_chi_squared(&self) -> f64 {
        let mut chi_sq = 0.0;
        
        for i in 0..10 {
            chi_sq += self.residuals[i] * self.residuals[i];
        }
        
        chi_sq
    }

    /// Calculate Jacobian matrix
    fn calculate_jacobian<F>(&mut self, f: &F, x_data: &[f64; 10]) -> SolverResult<()>
    where
        F: CurveFitFunction,
    {
        for i in 0..10 {
            let jacobian_row = f.jacobian(x_data[i], &self.parameters);
            for j in 0..4 {
                self.jacobian[j][i] = jacobian_row[j];
            }
        }
        
        Ok(())
    }

    /// Solve for parameter update using (J^T J + λI)δ = J^T r
    fn solve_parameter_update(&mut self) -> SolverResult<()> {
        // Calculate J^T J
        let mut jtj = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                let mut sum = 0.0;
                for k in 0..10 {
                    sum += self.jacobian[i][k] * self.jacobian[j][k];
                }
                jtj[i][j] = sum;
            }
            // Add damping term
            jtj[i][i] += self.lambda;
        }
        
        // Calculate J^T r
        let mut jtr = [0.0; 4];
        for i in 0..4 {
            let mut sum = 0.0;
            for k in 0..10 {
                sum += self.jacobian[i][k] * self.residuals[k];
            }
            jtr[i] = sum;
        }
        
        // Solve linear system (simplified 4x4 solver)
        self.solve_4x4_system(&jtj, &jtr)
    }

    /// Solve 4x4 linear system (simplified)
    fn solve_4x4_system(&mut self, matrix: &[[f64; 4]; 4], rhs: &[f64; 4]) -> SolverResult<()> {
        // Gaussian elimination with partial pivoting
        let mut a = *matrix;
        let mut b = *rhs;
        let mut pivot = [0; 4];
        
        // Forward elimination
        for i in 0..4 {
            // Find pivot
            let mut max_row = i;
            let mut max_val = a[i][i].abs();
            
            for j in i + 1..4 {
                if a[j][i].abs() > max_val {
                    max_val = a[j][i].abs();
                    max_row = j;
                }
            }
            
            if max_val < 1e-10 {
                return Err(ExecutionError::SingularMatrix);
            }
            
            // Swap rows
            if max_row != i {
                for k in 0..4 {
                    let temp = a[i][k];
                    a[i][k] = a[max_row][k];
                    a[max_row][k] = temp;
                }
                let temp = b[i];
                b[i] = b[max_row];
                b[max_row] = temp;
            }
            
            pivot[i] = max_row;
            
            // Eliminate column
            for j in i + 1..4 {
                let factor = a[j][i] / a[i][i];
                a[j][i] = factor;
                
                for k in i + 1..4 {
                    a[j][k] -= factor * a[i][k];
                }
                b[j] -= factor * b[i];
            }
        }
        
        // Back substitution
        for i in (0..4).rev() {
            let mut sum = b[i];
            for j in i + 1..4 {
                sum -= a[i][j] * self.delta_parameters[j];
            }
            self.delta_parameters[i] = sum / a[i][i];
        }
        
        Ok(())
    }

    /// Check convergence
    fn check_convergence(&self, old_chi_squared: f64) -> bool {
        let relative_change = (old_chi_squared - self.chi_squared).abs() / old_chi_squared;
        relative_change < self.config.tolerance
    }

    /// Get fitted parameters
    pub fn get_parameters(&self) -> [f64; 4] {
        self.parameters
    }
}

impl Default for OptimizationState {
    fn default() -> Self {
        Self {
            iteration: 0,
            objective_value: f64::MAX,
            converged: false,
            simplex_size: f64::MAX,
        }
    }
}

impl Default for RootFindingState {
    fn default() -> Self {
        Self {
            iteration: 0,
            function_value: f64::MAX,
            converged: false,
            step_size: f64::MAX,
        }
    }
}

impl Default for CurveFitState {
    fn default() -> Self {
        Self {
            iteration: 0,
            chi_squared: f64::MAX,
            converged: false,
            lambda: 1e-3,
        }
    }
}

impl Default for NelderMeadSimplex {
    fn default() -> Self {
        Self::new([0.0; 4], SolverConfig::default())
    }
}

impl Default for BoundedNewtonRaphson {
    fn default() -> Self {
        Self::new(0.0, -1e6, 1e6, SolverConfig::default())
    }
}

impl Default for LevenbergMarquardtStack {
    fn default() -> Self {
        Self::new([0.0; 4], SolverConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test function: f(x) = (x-1)² + (y-2)² + (z-3)² + (w-4)²
    struct QuadraticFunction;

    impl ObjectiveFunction for QuadraticFunction {
        fn evaluate(&self, params: &[f64; 4]) -> f64 {
            (params[0] - 1.0).powi(2) + 
            (params[1] - 2.0).powi(2) + 
            (params[2] - 3.0).powi(2) + 
            (params[3] - 4.0).powi(2)
        }
    }

    #[test]
    fn test_nelder_mead_simplex() {
        let mut optimizer = NelderMeadSimplex::new([0.0; 4], SolverConfig::default());
        let func = QuadraticFunction;
        
        let result = optimizer.optimize(&func);
        assert!(result.is_ok());
        
        let state = result.unwrap();
        assert!(state.converged);
        
        let (params, value) = optimizer.get_best_solution();
        assert!((params[0] - 1.0).abs() < 0.1);
        assert!((params[1] - 2.0).abs() < 0.1);
        assert!((params[2] - 3.0).abs() < 0.1);
        assert!((params[3] - 4.0).abs() < 0.1);
        assert!(value < 0.1);
    }

    // Test root finding: f(x) = x³ - 2x - 5
    struct CubicFunction;

    impl RootFunction for CubicFunction {
        fn evaluate(&self, x: f64) -> f64 {
            x.powi(3) - 2.0 * x - 5.0
        }
        
        fn derivative(&self, x: f64) -> f64 {
            3.0 * x * x - 2.0
        }
    }

    #[test]
    fn test_bounded_newton_raphson() {
        let mut solver = BoundedNewtonRaphson::new(2.0, -10.0, 10.0, SolverConfig::default());
        let func = CubicFunction;
        
        let result = solver.find_root(&func);
        assert!(result.is_ok());
        
        let state = result.unwrap();
        assert!(state.converged);
        
        let root = solver.get_root();
        assert!((root - 2.094).abs() < 0.01); // Known root
    }

    // Test curve fitting: y = a + bx + cx² + dx³
    struct PolynomialFit;

    impl CurveFitFunction for PolynomialFit {
        fn evaluate(&self, x: f64, params: &[f64; 4]) -> f64 {
            params[0] + params[1] * x + params[2] * x * x + params[3] * x * x * x
        }
        
        fn jacobian(&self, x: f64, _params: &[f64; 4]) -> [f64; 4] {
            [1.0, x, x * x, x * x * x]
        }
    }

    #[test]
    fn test_levenberg_marquardt_stack() {
        let mut optimizer = LevenbergMarquardtStack::new([1.0, 1.0, 1.0, 1.0], SolverConfig::default());
        
        // Generate test data
        let mut x_data = [0.0; 10];
        let mut y_data = [0.0; 10];
        
        for i in 0..10 {
            x_data[i] = i as f64;
            y_data[i] = 2.0 + 3.0 * x_data[i] + 0.5 * x_data[i] * x_data[i] + 0.1 * x_data[i] * x_data[i] * x_data[i];
        }
        
        let func = PolynomialFit;
        let result = optimizer.fit_curve(&func, &x_data, &y_data);
        assert!(result.is_ok());
        
        let state = result.unwrap();
        assert!(state.converged);
        
        let params = optimizer.get_parameters();
        assert!((params[0] - 2.0).abs() < 0.1);
        assert!((params[1] - 3.0).abs() < 0.1);
    }

    #[test]
    fn test_zero_allocation_guarantee() {
        assert_eq!(core::mem::size_of::<NelderMeadSimplex>(), 240);
        assert_eq!(core::mem::size_of::<BoundedNewtonRaphson>(), 96);
        assert_eq!(core::mem::size_of::<LevenbergMarquardtStack>(), 576);
    }
}
