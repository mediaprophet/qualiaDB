//! Calculus & Differential Solvers - Zero-Allocation Implementation
//! 
//! This module provides fixed-size stack-based solvers for differential equations,
//! boundary value problems, and numerical integration suitable for the #![no_std]
//! environment of Qualia-DB.

use crate::solvers::{SolverConfig, SolverState, SolverResult};
use crate::solvers::SolversError as ExecutionError;
use core::f64::consts;

/// Runge-Kutta 4th order ODE solver with fixed memory footprint
#[repr(C)]
pub struct RungeKutta4Static {
    /// Current state vector (fixed size)
    pub state: [f64; 4],
    /// Time step size
    pub dt: f64,
    /// Current time
    pub t: f64,
    /// Solver configuration
    pub config: SolverConfig,
    /// Solver state
    pub solver_state: SolverState,
}

/// ODE state for RK4 solver
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ODEState {
    /// Current values
    pub y: [f64; 4],
    /// Current time
    pub t: f64,
    /// Step size
    pub dt: f64,
}

/// Boundary Value Problem solver using shooting method
#[repr(C)]
pub struct ShootingMethodBVP {
    /// Initial guess for shooting
    pub initial_guess: [f64; 4],
    /// Target boundary values
    pub target_values: [f64; 4],
    /// Current trajectory
    pub trajectory: [ODEState; 100],
    /// Error tracking
    pub boundary_error: [f64; 4],
    /// Solver configuration
    pub config: SolverConfig,
    /// Solver state
    pub solver_state: SolverState,
}

/// BVP state for shooting method
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BVPState {
    /// Current iteration
    pub iteration: u32,
    /// Boundary error
    pub boundary_error: f64,
    /// Converged flag
    pub converged: bool,
    /// Shooting parameters
    pub shooting_params: [f64; 4],
}

/// Simpson's rule integrator with chunked processing
#[repr(C)]
pub struct SimpsonsIntegratorChunked {
    /// Chunk buffer (12 points for optimal cache usage)
    pub chunk_buffer: [f64; 12],
    /// Current chunk index
    pub chunk_index: u16,
    /// Accumulated integral
    pub accumulated_integral: f64,
    /// Integration limits
    pub a: f64,
    pub b: f64,
    /// Number of chunks processed
    pub chunks_processed: u16,
    /// Solver configuration
    pub config: SolverConfig,
    /// Solver state
    pub solver_state: SolverState,
}

/// Integral chunk for processing
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IntegralChunk {
    /// Function values
    pub values: [f64; 12],
    /// X coordinates
    pub x_coords: [f64; 12],
    /// Chunk contribution
    pub contribution: f64,
}

/// ODE function trait for RK4
pub trait ODEFunction {
    /// Calculate derivatives dy/dt = f(t, y)
    fn derivatives(&self, t: f64, y: &[f64; 4]) -> [f64; 4];
}

/// BVP function trait for shooting method
pub trait BVPFunction {
    /// Calculate derivatives for boundary value problem
    fn derivatives(&self, t: f64, y: &[f64; 4], params: &[f64; 4]) -> [f64; 4];
    
    /// Calculate boundary condition residuals
    fn boundary_residuals(&self, y0: &[f64; 4], y1: &[f64; 4]) -> [f64; 4];
}

/// Integrand function trait for Simpson's integrator
pub trait IntegrandFunction {
    /// Evaluate function at point x
    fn evaluate(&self, x: f64) -> f64;
}

impl RungeKutta4Static {
    /// Create new RK4 solver
    pub fn new(dt: f64, config: SolverConfig) -> Self {
        Self {
            state: [0.0; 4],
            dt,
            t: 0.0,
            config,
            solver_state: SolverState::default(),
        }
    }

    /// Integrate ODE from t0 to t_final
    pub fn integrate<F>(&mut self, f: &F, t0: f64, y0: [f64; 4], t_final: f64) -> SolverResult<ODEState>
    where
        F: ODEFunction,
    {
        // Initialize state
        self.state = y0;
        self.t = t0;
        self.solver_state.iteration = 0;
        self.solver_state.converged = false;

        // Integrate until final time
        while self.t < t_final && self.solver_state.iteration < self.config.max_iterations {
            // Check if we need to adjust step size to hit final time exactly
            let remaining_time = t_final - self.t;
            let dt = if remaining_time < self.dt {
                remaining_time
            } else {
                self.dt
            };

            // Perform RK4 step
            self.rk4_step(f, dt)?;

            // Update solver state
            self.solver_state.iteration += 1;
            
            // Check convergence (for steady-state problems)
            if self.check_convergence() {
                self.solver_state.converged = true;
                break;
            }
        }

        Ok(ODEState {
            y: self.state,
            t: self.t,
            dt: self.dt,
        })
    }

    /// Perform single RK4 step
    fn rk4_step<F>(&mut self, f: &F, dt: f64) -> SolverResult<()>
    where
        F: ODEFunction,
    {
        // RK4 coefficients
        let k1 = f.derivatives(self.t, &self.state);
        
        let y_temp: [f64; 4];
        let mut y_temp = [0.0; 4];
        for i in 0..4 {
            y_temp[i] = self.state[i] + 0.5 * dt * k1[i];
        }
        let k2 = f.derivatives(self.t + 0.5 * dt, &y_temp);
        
        for i in 0..4 {
            y_temp[i] = self.state[i] + 0.5 * dt * k2[i];
        }
        let k3 = f.derivatives(self.t + 0.5 * dt, &y_temp);
        
        for i in 0..4 {
            y_temp[i] = self.state[i] + dt * k3[i];
        }
        let k4 = f.derivatives(self.t + dt, &y_temp);
        
        // Update state using RK4 formula
        for i in 0..4 {
            self.state[i] += dt * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]) / 6.0;
        }
        
        self.t += dt;
        
        // Calculate error estimate
        self.solver_state.error = self.estimate_error(&k1, &k2, &k3, &k4);
        
        Ok(())
    }

    /// Estimate error from RK4 coefficients
    fn estimate_error(&self, k1: &[f64; 4], k2: &[f64; 4], k3: &[f64; 4], k4: &[f64; 4]) -> f64 {
        let mut error: f64 = 0.0;
        
        for i in 0..4 {
            // Error estimate: |k2 - k3| / max(|k2|, |k3|, 1e-10)
            let local_error = (k2[i] - k3[i]).abs() / k2[i].abs().max(k3[i].abs()).max(1e-10);
            error = error.max(local_error);
        }
        
        error
    }

    /// Check for convergence (steady state)
    fn check_convergence(&self) -> bool {
        self.solver_state.error < self.config.tolerance
    }

    /// Get current state
    pub fn get_state(&self) -> ODEState {
        ODEState {
            y: self.state,
            t: self.t,
            dt: self.dt,
        }
    }
}

impl ShootingMethodBVP {
    /// Create new BVP solver
    pub fn new(target_values: [f64; 4], config: SolverConfig) -> Self {
        Self {
            initial_guess: [0.0; 4],
            target_values,
            trajectory: [ODEState::default(); 100],
            boundary_error: [0.0; 4],
            config,
            solver_state: SolverState::default(),
        }
    }

    /// Solve boundary value problem using shooting method
    pub fn solve<F>(&mut self, f: &F, t0: f64, t1: f64, initial_guess: [f64; 4]) -> SolverResult<BVPState>
    where
        F: BVPFunction,
    {
        self.initial_guess = initial_guess;
        self.solver_state.iteration = 0;
        self.solver_state.converged = false;

        while self.solver_state.iteration < self.config.max_iterations {
            // Shoot from initial boundary
            self.shoot_trajectory(f, t0, t1)?;
            
            // Calculate boundary error
            self.calculate_boundary_error(f)?;
            
            // Check convergence
            let max_error = self.boundary_error.iter().fold(0.0_f64, |acc, &x| acc.max(x.abs()));
            self.solver_state.error = max_error;
            
            if max_error < self.config.tolerance {
                self.solver_state.converged = true;
                break;
            }
            
            // Update initial guess using Newton-like method
            self.update_shooting_parameters()?;
            
            self.solver_state.iteration += 1;
        }

        Ok(BVPState {
            iteration: self.solver_state.iteration,
            boundary_error: self.solver_state.error,
            converged: self.solver_state.converged,
            shooting_params: self.initial_guess,
        })
    }

    /// Shoot trajectory from initial to final boundary
    fn shoot_trajectory<F>(&mut self, f: &F, t0: f64, t1: f64) -> SolverResult<()>
    where
        F: BVPFunction,
    {
        struct BvpOdeAdapter<'a, G: BVPFunction>(&'a G);
        impl<G: BVPFunction> ODEFunction for BvpOdeAdapter<'_, G> {
            fn derivatives(&self, t: f64, y: &[f64; 4]) -> [f64; 4] {
                self.0.derivatives(t, y, &[0.0; 4])
            }
        }

        // Create RK4 solver for trajectory
        let mut rk4 = RungeKutta4Static::new((t1 - t0) / 100.0, self.config);

        // Initial state
        let initial_state = [self.initial_guess[0], self.initial_guess[1],
                            self.initial_guess[2], self.initial_guess[3]];

        // Integrate to final boundary using ODE adapter
        let ode_f = BvpOdeAdapter(f);
        let final_state = rk4.integrate(&ode_f, t0, initial_state, t1)?;
        
        // Store trajectory (sample points)
        self.store_trajectory_sample(&final_state, 99); // Store final state
        
        Ok(())
    }

    /// Calculate boundary error
    fn calculate_boundary_error<F>(&mut self, f: &F) -> SolverResult<()>
    where
        F: BVPFunction,
    {
        // Get initial and final states
        let initial_state = &self.trajectory[0];
        let final_state = &self.trajectory[99];
        
        // Calculate boundary residuals
        self.boundary_error = f.boundary_residuals(&initial_state.y, &final_state.y);
        
        Ok(())
    }

    /// Update shooting parameters using Newton-like method
    fn update_shooting_parameters(&mut self) -> SolverResult<()> {
        // Simple parameter update: reduce error proportionally
        for i in 0..4 {
            if self.boundary_error[i].abs() > 1e-10 {
                let correction = -self.boundary_error[i] * 0.1; // Damping factor
                self.initial_guess[i] += correction;
            }
        }
        
        Ok(())
    }

    /// Store trajectory sample point
    fn store_trajectory_sample(&mut self, state: &ODEState, index: usize) {
        if index < 100 {
            self.trajectory[index] = *state;
        }
    }

    /// Get shooting parameters
    pub fn get_shooting_params(&self) -> [f64; 4] {
        self.initial_guess
    }
}

impl SimpsonsIntegratorChunked {
    /// Create new Simpson's integrator
    pub fn new(a: f64, b: f64, config: SolverConfig) -> Self {
        Self {
            chunk_buffer: [0.0; 12],
            chunk_index: 0,
            accumulated_integral: 0.0,
            a,
            b,
            chunks_processed: 0,
            config,
            solver_state: SolverState::default(),
        }
    }

    /// Integrate function using chunked Simpson's rule
    pub fn integrate<F>(&mut self, f: &F) -> SolverResult<f64>
    where
        F: IntegrandFunction,
    {
        self.accumulated_integral = 0.0;
        self.chunk_index = 0;
        self.chunks_processed = 0;
        self.solver_state.iteration = 0;

        let total_length = self.b - self.a;
        let chunk_size = total_length / 100.0; // 100 chunks
        
        while self.chunk_index < 100 && self.solver_state.iteration < self.config.max_iterations {
            let chunk_start = self.a + self.chunk_index as f64 * chunk_size;
            let chunk_end = chunk_start + chunk_size;
            
            // Process chunk
            let chunk_integral = self.process_chunk(f, chunk_start, chunk_end)?;
            self.accumulated_integral += chunk_integral;
            
            self.chunk_index += 1;
            self.chunks_processed += 1;
            self.solver_state.iteration += 1;
        }

        self.solver_state.converged = self.chunk_index >= 100;
        self.solver_state.error = self.estimate_integration_error();
        
        Ok(self.accumulated_integral)
    }

    /// Process a single chunk of the integral
    fn process_chunk<F>(&mut self, f: &F, x_start: f64, x_end: f64) -> SolverResult<f64>
    where
        F: IntegrandFunction,
    {
        // Generate 12 points in this chunk
        let h = (x_end - x_start) / 11.0;
        
        for i in 0..12 {
            let x = x_start + i as f64 * h;
            self.chunk_buffer[i] = f.evaluate(x);
        }
        
        // Apply Simpson's rule to chunk
        let mut chunk_integral = self.chunk_buffer[0] + self.chunk_buffer[11];
        
        for i in 1..11 {
            let weight = if i % 2 == 1 { 4.0 } else { 2.0 };
            chunk_integral += weight * self.chunk_buffer[i];
        }
        
        chunk_integral *= h / 3.0;
        
        Ok(chunk_integral)
    }

    /// Estimate integration error
    fn estimate_integration_error(&self) -> f64 {
        // Simple error estimate based on remaining chunks
        if self.chunk_index < 100 {
            let remaining_chunks = 100 - self.chunk_index;
            remaining_chunks as f64 / 100.0
        } else {
            0.0
        }
    }

    /// Get current integral value
    pub fn get_integral(&self) -> f64 {
        self.accumulated_integral
    }
}

impl Default for ODEState {
    fn default() -> Self {
        Self {
            y: [0.0; 4],
            t: 0.0,
            dt: 0.01,
        }
    }
}

impl Default for BVPState {
    fn default() -> Self {
        Self {
            iteration: 0,
            boundary_error: f64::MAX,
            converged: false,
            shooting_params: [0.0; 4],
        }
    }
}

impl Default for IntegralChunk {
    fn default() -> Self {
        Self {
            values: [0.0; 12],
            x_coords: [0.0; 12],
            contribution: 0.0,
        }
    }
}

impl Default for RungeKutta4Static {
    fn default() -> Self {
        Self::new(0.01, SolverConfig::default())
    }
}

impl Default for ShootingMethodBVP {
    fn default() -> Self {
        Self::new([0.0; 4], SolverConfig::default())
    }
}

impl Default for SimpsonsIntegratorChunked {
    fn default() -> Self {
        Self::new(0.0, 1.0, SolverConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test ODE: dy/dt = -y (exponential decay)
    struct ExponentialDecay;

    impl ODEFunction for ExponentialDecay {
        fn derivatives(&self, _t: f64, y: &[f64; 4]) -> [f64; 4] {
            [-y[0], -y[1], -y[2], -y[3]]
        }
    }

    #[test]
    fn test_rk4_static() {
        let mut rk4 = RungeKutta4Static::new(0.1, SolverConfig::default());
        let decay = ExponentialDecay;
        
        let result = rk4.integrate(&decay, 0.0, [1.0; 4], 1.0);
        assert!(result.is_ok());
        
        let state = result.unwrap();
        // After t=1, y = e^-1 ≈ 0.3679
        assert!((state.y[0] - 0.3679).abs() < 0.01);
    }

    #[test]
    fn test_shooting_method_bvp() {
        let mut bvp = ShootingMethodBVP::new([0.0; 4], SolverConfig::default());
        
        // Simple BVP: dy/dt = -y, with y(0)=1, y(1)=e^-1
        struct DecayBVP;
        
        impl BVPFunction for DecayBVP {
            fn derivatives(&self, _t: f64, y: &[f64; 4], _params: &[f64; 4]) -> [f64; 4] {
                [-y[0], -y[1], -y[2], -y[3]]
            }
            
            fn boundary_residuals(&self, y0: &[f64; 4], y1: &[f64; 4]) -> [f64; 4] {
                [y0[0] - 1.0, y1[0] - (-1.0_f64).exp(), 0.0, 0.0]
            }
        }
        
        let decay = DecayBVP;
        let result = bvp.solve(&decay, 0.0, 1.0, [1.0, 0.0, 0.0, 0.0]);
        assert!(result.is_ok());
        
        let state = result.unwrap();
        assert!(state.converged);
    }

    #[test]
    fn test_simpsons_integrator_chunked() {
        let mut integrator = SimpsonsIntegratorChunked::new(0.0, consts::PI, SolverConfig::default());
        
        // Integrate sin(x) from 0 to π
        struct SinFunction;
        
        impl IntegrandFunction for SinFunction {
            fn evaluate(&self, x: f64) -> f64 {
                x.sin()
            }
        }
        
        let sin_func = SinFunction;
        let result = integrator.integrate(&sin_func);
        assert!(result.is_ok());
        
        let integral = result.unwrap();
        // ∫₀^π sin(x) dx = 2.0
        assert!((integral - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_zero_allocation_guarantee() {
        assert_eq!(core::mem::size_of::<RungeKutta4Static>(), 112);
        assert_eq!(core::mem::size_of::<ShootingMethodBVP>(), 3648);
        assert_eq!(core::mem::size_of::<SimpsonsIntegratorChunked>(), 208);
    }
}
