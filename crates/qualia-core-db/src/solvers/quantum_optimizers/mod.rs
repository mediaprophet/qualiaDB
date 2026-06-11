//! Hybrid Quantum Optimizers - Zero-Allocation Implementation
//! 
//! This module provides quantum-aware optimization algorithms that serve as the
//! classical half of hybrid quantum-classical loops, designed specifically for
//! the #![no_std] environment of Qualia-DB.

use crate::solvers::{SolverConfig, SolverState, SolverResult};
use crate::solvers::SolversError as ExecutionError;
use crate::NQuin;
use core::f64::consts;

/// QAOA angle optimizer for quantum approximate optimization
#[repr(C)]
pub struct QAOAAngleOptimizer {
    /// Current angle parameters (β, γ pairs)
    pub angles: QAOAAngles,
    /// Angle updates
    pub angle_updates: QAOAAngles,
    /// Cost function history
    pub cost_history: [f64; 50],
    /// Gradient estimates
    pub gradients: QAOAAngles,
    /// Current depth
    pub depth: u8,
    /// Solver configuration
    pub config: SolverConfig,
    /// Solver state
    pub solver_state: SolverState,
}

/// SPSA optimizer for quantum hardware noise resilience
#[repr(C)]
pub struct SpsaOptimizer {
    /// Current parameters
    pub parameters: [f64; 20], // Up to 20 parameters
    /// Perturbation vector
    pub perturbation: [f64; 20],
    /// Gradient estimates
    pub gradient: [f64; 20],
    /// Cost evaluations
    pub cost_plus: f64,
    pub cost_minus: f64,
    /// Perturbation parameters
    pub ck: f64, // Perturbation magnitude
    pub ak: f64, // Step size
    pub a: f64,  // Initial step size
    pub c: f64,  // Initial perturbation
    pub alpha: f64, // Step size decay
    pub gamma: f64, // Perturbation decay
    /// Number of parameters
    pub num_params: u8,
    /// Solver configuration
    pub config: SolverConfig,
    /// Solver state
    pub solver_state: SolverState,
}

/// QAOA angle parameters
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QAOAAngles {
    /// Beta angles (problem unitary)
    pub beta: [f64; 10],
    /// Gamma angles (mixing unitary)
    pub gamma: [f64; 10],
}

/// SPSA gradient estimate
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SpsaGradient {
    /// Gradient components
    pub components: [f64; 20],
    /// Gradient norm
    pub norm: f64,
    /// Perturbation used
    pub perturbation_norm: f64,
}

/// Quantum optimizer state
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QuantumOptimizerState {
    /// Current iteration
    pub iteration: u32,
    /// Current cost value
    pub cost_value: f64,
    /// Converged flag
    pub converged: bool,
    /// Quantum hardware calls made
    pub quantum_calls: u32,
}

/// Quantum cost function trait
pub trait QuantumCostFunction {
    /// Evaluate cost function on quantum hardware
    fn evaluate_quantum(&self, angles: &QAOAAngles) -> SolverResult<f64>;
    
    /// Evaluate cost function with perturbed parameters
    fn evaluate_perturbed(&self, angles: &QAOAAngles, perturbation: &QAOAAngles) -> SolverResult<(f64, f64)>;
    
    /// Get problem size
    fn problem_size(&self) -> u8;
}

/// SPSA cost function trait
pub trait SpsaCostFunction {
    /// Evaluate cost function with given parameters
    fn evaluate(&self, params: &[f64; 20], num_params: u8) -> SolverResult<f64>;
    
    /// Check if parameters are valid for quantum hardware
    fn valid_parameters(&self, params: &[f64; 20], num_params: u8) -> bool;
}

impl QAOAAngleOptimizer {
    /// Create new QAOA angle optimizer
    pub fn new(depth: u8, config: SolverConfig) -> Self {
        Self {
            angles: QAOAAngles::default(),
            angle_updates: QAOAAngles::default(),
            cost_history: [f64::MAX; 50],
            gradients: QAOAAngles::default(),
            depth,
            config,
            solver_state: SolverState::default(),
        }
    }

    /// Optimize QAOA angles using gradient-based method
    pub fn optimize<F>(&mut self, f: &F, initial_angles: QAOAAngles) -> SolverResult<QuantumOptimizerState>
    where
        F: QuantumCostFunction,
    {
        self.angles = initial_angles;
        self.solver_state.iteration = 0;
        self.solver_state.quantum_calls = 0;
        self.solver_state.converged = false;

        // Initial cost evaluation
        let initial_cost = f.evaluate_quantum(&self.angles)?;
        self.cost_history[0] = initial_cost;
        self.solver_state.cost_value = initial_cost;

        while self.solver_state.iteration < self.config.max_iterations {
            // Compute gradient estimate
            self.compute_gradient(f)?;
            
            // Update angles using gradient descent
            self.update_angles()?;
            
            // Evaluate new cost
            let new_cost = f.evaluate_quantum(&self.angles)?;
            self.solver_state.cost_value = new_cost;
            self.solver_state.quantum_calls += 1;
            
            // Store in history
            let history_idx = (self.solver_state.iteration % 50) as usize;
            self.cost_history[history_idx] = new_cost;
            
            // Check convergence
            if self.check_convergence() {
                self.solver_state.converged = true;
                break;
            }
            
            self.solver_state.iteration += 1;
        }

        Ok(QuantumOptimizerState {
            iteration: self.solver_state.iteration,
            cost_value: self.solver_state.cost_value,
            converged: self.solver_state.converged,
            quantum_calls: self.solver_state.quantum_calls,
        })
    }

    /// Compute gradient using finite differences
    fn compute_gradient<F>(&mut self, f: &F) -> SolverResult<()>
    where
        F: QuantumCostFunction,
    {
        let epsilon = 1e-3; // Small perturbation for gradient
        
        // Compute gradient for beta angles
        for i in 0..self.depth as usize {
            // Perturb beta angle
            let mut perturbed_angles = self.angles;
            perturbed_angles.beta[i] += epsilon;
            
            let cost_plus = f.evaluate_quantum(&perturbed_angles)?;
            self.solver_state.quantum_calls += 1;
            
            // Perturb in opposite direction
            perturbed_angles.beta[i] = self.angles.beta[i] - epsilon;
            
            let cost_minus = f.evaluate_quantum(&perturbed_angles)?;
            self.solver_state.quantum_calls += 1;
            
            // Gradient estimate
            self.gradients.beta[i] = (cost_plus - cost_minus) / (2.0 * epsilon);
        }
        
        // Compute gradient for gamma angles
        for i in 0..self.depth as usize {
            // Perturb gamma angle
            let mut perturbed_angles = self.angles;
            perturbed_angles.gamma[i] += epsilon;
            
            let cost_plus = f.evaluate_quantum(&perturbed_angles)?;
            self.solver_state.quantum_calls += 1;
            
            // Perturb in opposite direction
            perturbed_angles.gamma[i] = self.angles.gamma[i] - epsilon;
            
            let cost_minus = f.evaluate_quantum(&perturbed_angles)?;
            self.solver_state.quantum_calls += 1;
            
            // Gradient estimate
            self.gradients.gamma[i] = (cost_plus - cost_minus) / (2.0 * epsilon);
        }
        
        Ok(())
    }

    /// Update angles using gradient descent
    fn update_angles(&mut self) -> SolverResult<()> {
        let learning_rate = 0.01; // Learning rate
        
        // Update beta angles
        for i in 0..self.depth as usize {
            self.angle_updates.beta[i] = -learning_rate * self.gradients.beta[i];
            self.angles.beta[i] += self.angle_updates.beta[i];
            
            // Keep angles in [0, 2π] range
            self.angles.beta[i] = self.angles.beta[i] % (2.0 * consts::PI);
            if self.angles.beta[i] < 0.0 {
                self.angles.beta[i] += 2.0 * consts::PI;
            }
        }
        
        // Update gamma angles
        for i in 0..self.depth as usize {
            self.angle_updates.gamma[i] = -learning_rate * self.gradients.gamma[i];
            self.angles.gamma[i] += self.angle_updates.gamma[i];
            
            // Keep angles in [0, 2π] range
            self.angles.gamma[i] = self.angles.gamma[i] % (2.0 * consts::PI);
            if self.angles.gamma[i] < 0.0 {
                self.angles.gamma[i] += 2.0 * consts::PI;
            }
        }
        
        Ok(())
    }

    /// Check convergence based on cost history
    fn check_convergence(&self) -> bool {
        if self.solver_state.iteration < 10 {
            return false;
        }
        
        // Check if cost improvement is small
        let recent_window = 5;
        let start_idx = ((self.solver_state.iteration - recent_window) % 50) as usize;
        let end_idx = (self.solver_state.iteration % 50) as usize;
        
        let mut cost_change = 0.0;
        let mut count = 0;
        
        for i in 0..recent_window {
            let idx = (start_idx + i as usize) % 50;
            cost_change += (self.cost_history[idx] - self.cost_history[end_idx]).abs();
            count += 1;
        }
        
        if count > 0 {
            cost_change / (count as f64) < self.config.tolerance
        } else {
            false
        }
    }

    /// Get optimized angles
    pub fn get_angles(&self) -> QAOAAngles {
        self.angles
    }

    /// Get gradient information
    pub fn get_gradients(&self) -> QAOAAngles {
        self.gradients
    }
}

impl SpsaOptimizer {
    /// Create new SPSA optimizer
    pub fn new(num_params: u8, config: SolverConfig) -> Self {
        Self {
            parameters: [0.0; 20],
            perturbation: [0.0; 20],
            gradient: [0.0; 20],
            cost_plus: 0.0,
            cost_minus: 0.0,
            ck: 0.1,
            ak: 0.1,
            a: 0.1,
            c: 0.1,
            alpha: 0.602, // Standard values
            gamma: 0.101,
            num_params,
            config,
            solver_state: SolverState::default(),
        }
    }

    /// Optimize parameters using SPSA algorithm
    pub fn optimize<F>(&mut self, f: &F, initial_params: &[f64; 20]) -> SolverResult<QuantumOptimizerState>
    where
        F: SpsaCostFunction,
    {
        // Initialize parameters
        for i in 0..self.num_params as usize {
            self.parameters[i] = initial_params[i];
        }
        
        self.solver_state.iteration = 0;
        self.solver_state.quantum_calls = 0;
        self.solver_state.converged = false;

        // Initial cost evaluation
        let initial_cost = f.evaluate(&self.parameters, self.num_params)?;
        self.solver_state.cost_value = initial_cost;

        while self.solver_state.iteration < self.config.max_iterations {
            // Update step sizes
            self.update_step_sizes();
            
            // Generate random perturbation
            self.generate_perturbation()?;
            
            // Evaluate cost at perturbed points
            self.evaluate_perturbed_costs(f)?;
            self.solver_state.quantum_calls += 2;
            
            // Estimate gradient
            self.estimate_gradient()?;
            
            // Update parameters
            self.update_parameters()?;
            
            // Evaluate new cost
            let new_cost = f.evaluate(&self.parameters, self.num_params)?;
            self.solver_state.cost_value = new_cost;
            self.solver_state.quantum_calls += 1;
            
            // Check convergence
            if self.check_convergence() {
                self.solver_state.converged = true;
                break;
            }
            
            self.solver_state.iteration += 1;
        }

        Ok(QuantumOptimizerState {
            iteration: self.solver_state.iteration,
            cost_value: self.solver_state.cost_value,
            converged: self.solver_state.converged,
            quantum_calls: self.solver_state.quantum_calls,
        })
    }

    /// Update step sizes according to SPSA schedule
    fn update_step_sizes(&mut self) {
        let k = self.solver_state.iteration as f64 + 1.0;
        
        // ak = a / (k + A + 1)^alpha (simplified)
        self.ak = self.a / (k + 1.0).powf(self.alpha);
        
        // ck = c / (k + 1)^gamma
        self.ck = self.c / (k + 1.0).powf(self.gamma);
    }

    /// Generate random perturbation vector
    fn generate_perturbation(&mut self) -> SolverResult<()> {
        for i in 0..self.num_params as usize {
            // Generate random ±1 perturbation
            let random_bit = self.generate_random_bit();
            self.perturbation[i] = if random_bit { 1.0 } else { -1.0 };
        }
        
        Ok(())
    }

    /// Evaluate costs at perturbed parameter points
    fn evaluate_perturbed_costs<F>(&mut self, f: &F) -> SolverResult<()>
    where
        F: SpsaCostFunction,
    {
        // Create perturbed parameter vectors
        let mut params_plus = self.parameters;
        let mut params_minus = self.parameters;
        
        for i in 0..self.num_params as usize {
            let delta = self.ck * self.perturbation[i];
            params_plus[i] += delta;
            params_minus[i] -= delta;
        }
        
        // Evaluate costs
        self.cost_plus = f.evaluate(&params_plus, self.num_params)?;
        self.cost_minus = f.evaluate(&params_minus, self.num_params)?;
        
        Ok(())
    }

    /// Estimate gradient from perturbed costs
    fn estimate_gradient(&mut self) -> SolverResult<()> {
        for i in 0..self.num_params as usize {
            // Gradient estimate: (cost_plus - cost_minus) / (2 * ck * perturbation[i])
            let denominator = 2.0 * self.ck * self.perturbation[i];
            if denominator.abs() > 1e-10 {
                self.gradient[i] = (self.cost_plus - self.cost_minus) / denominator;
            } else {
                self.gradient[i] = 0.0;
            }
        }
        
        Ok(())
    }

    /// Update parameters using gradient estimate
    fn update_parameters(&mut self) -> SolverResult<()> {
        for i in 0..self.num_params as usize {
            // Update: theta_new = theta_old - ak * gradient
            self.parameters[i] -= self.ak * self.gradient[i];
            
            // Apply parameter bounds if needed
            if !self.valid_parameter_range(self.parameters[i]) {
                // Clip to valid range
                self.parameters[i] = self.clip_parameter(self.parameters[i]);
            }
        }
        
        Ok(())
    }

    /// Check if parameter is in valid range
    fn valid_parameter_range(&self, param: f64) -> bool {
        // Simple bounds: [-π, π] for angles, [-10, 10] for other parameters
        param >= -consts::PI && param <= consts::PI
    }

    /// Clip parameter to valid range
    fn clip_parameter(&self, param: f64) -> f64 {
        param.clamp(-consts::PI, consts::PI)
    }

    /// Check convergence
    fn check_convergence(&self) -> bool {
        if self.solver_state.iteration < 10 {
            return false;
        }
        
        // Check if step size is small
        self.ak < self.config.tolerance
    }

    /// Generate random bit (simplified)
    fn generate_random_bit(&self) -> bool {
        // Simple pseudo-random generator
        static mut SEED: u64 = 12345;
        unsafe {
            SEED = SEED.wrapping_mul(1103515245).wrapping_add(12345);
            (SEED & 1) == 1
        }
    }

    /// Get optimized parameters
    pub fn get_parameters(&self) -> [f64; 20] {
        let mut result = [0.0; 20];
        for i in 0..self.num_params as usize {
            result[i] = self.parameters[i];
        }
        result
    }

    /// Get gradient information
    pub fn get_gradient(&self) -> SpsaGradient {
        let mut gradient = SpsaGradient::default();
        
        for i in 0..self.num_params as usize {
            gradient.components[i] = self.gradient[i];
        }
        
        // Calculate gradient norm
        let mut norm = 0.0;
        for i in 0..self.num_params as usize {
            norm += self.gradient[i] * self.gradient[i];
        }
        gradient.norm = norm.sqrt();
        
        // Perturbation norm
        let mut pert_norm = 0.0;
        for i in 0..self.num_params as usize {
            pert_norm += self.perturbation[i] * self.perturbation[i];
        }
        gradient.perturbation_norm = pert_norm.sqrt();
        
        gradient
    }
}

impl Default for QAOAAngles {
    fn default() -> Self {
        Self {
            beta: [0.0; 10],
            gamma: [0.0; 10],
        }
    }
}

impl Default for SpsaGradient {
    fn default() -> Self {
        Self {
            components: [0.0; 20],
            norm: 0.0,
            perturbation_norm: 0.0,
        }
    }
}

impl Default for QuantumOptimizerState {
    fn default() -> Self {
        Self {
            iteration: 0,
            cost_value: f64::MAX,
            converged: false,
            quantum_calls: 0,
        }
    }
}

impl Default for QAOAAngleOptimizer {
    fn default() -> Self {
        Self::new(1, SolverConfig::default())
    }
}

impl Default for SpsaOptimizer {
    fn default() -> Self {
        Self::new(4, SolverConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock quantum cost function for testing
    struct MockQuantumCost;

    impl QuantumCostFunction for MockQuantumCost {
        fn evaluate_quantum(&self, angles: &QAOAAngles) -> SolverResult<f64> {
            // Simple quadratic cost function for testing
            let mut cost = 0.0;
            for i in 0..10 {
                cost += (angles.beta[i] - 1.0).powi(2);
                cost += (angles.gamma[i] - 0.5).powi(2);
            }
            Ok(cost)
        }
        
        fn evaluate_perturbed(&self, angles: &QAOAAngles, _perturbation: &QAOAAngles) -> SolverResult<(f64, f64)> {
            let cost1 = self.evaluate_quantum(angles)?;
            let cost2 = self.evaluate_quantum(angles)?;
            Ok((cost1, cost2))
        }
        
        fn problem_size(&self) -> u8 {
            10
        }
    }

    #[test]
    fn test_qaoa_angle_optimizer() {
        let mut optimizer = QAOAAngleOptimizer::new(2, SolverConfig::default());
        
        let initial_angles = QAOAAngles {
            beta: [0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            gamma: [0.25, 0.25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        };
        
        let cost_func = MockQuantumCost;
        let result = optimizer.optimize(&cost_func, initial_angles);
        assert!(result.is_ok());
        
        let state = result.unwrap();
        assert!(state.quantum_calls > 0);
        
        let optimized_angles = optimizer.get_angles();
        assert!((optimized_angles.beta[0] - 1.0).abs() < 0.1);
        assert!((optimized_angles.gamma[0] - 0.5).abs() < 0.1);
    }

    // Mock SPSA cost function
    struct MockSpsaCost;

    impl SpsaCostFunction for MockSpsaCost {
        fn evaluate(&self, params: &[f64; 20], num_params: u8) -> SolverResult<f64> {
            // Simple quadratic function: sum((x_i - target_i)^2)
            let mut cost = 0.0;
            for i in 0..num_params as usize {
                let target = (i as f64 + 1.0) * 0.5; // Targets: 0.5, 1.0, 1.5, ...
                cost += (params[i] - target).powi(2);
            }
            Ok(cost)
        }
        
        fn valid_parameters(&self, params: &[f64; 20], num_params: u8) -> bool {
            for i in 0..num_params as usize {
                if params[i] < -consts::PI || params[i] > consts::PI {
                    return false;
                }
            }
            true
        }
    }

    #[test]
    fn test_spsa_optimizer() {
        let mut optimizer = SpsaOptimizer::new(4, SolverConfig::default());
        
        let initial_params = [0.1; 20];
        
        let cost_func = MockSpsaCost;
        let result = optimizer.optimize(&cost_func, &initial_params);
        assert!(result.is_ok());
        
        let state = result.unwrap();
        assert!(state.quantum_calls > 0);
        
        let optimized_params = optimizer.get_parameters();
        assert!((optimized_params[0] - 0.5).abs() < 0.5); // Should move toward target
        assert!((optimized_params[1] - 1.0).abs() < 0.5);
        assert!((optimized_params[2] - 1.5).abs() < 0.5);
        assert!((optimized_params[3] - 2.0).abs() < 0.5);
    }

    #[test]
    fn test_zero_allocation_guarantee() {
        assert_eq!(core::mem::size_of::<QAOAAngleOptimizer>(), 1248);
        assert_eq!(core::mem::size_of::<SpsaOptimizer>(), 656);
        assert_eq!(core::mem::size_of::<QAOAAngles>(), 160);
        assert_eq!(core::mem::size_of::<SpsaGradient>(), 176);
    }
}
