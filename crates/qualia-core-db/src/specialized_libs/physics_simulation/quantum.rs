use super::*;

impl PhysicsSimulationLibrary {
    /// QuantumMechanics — 1D time-independent Schrödinger equation
    /// `[-ħ²/(2m)·d²/dx² + V(x)]·ψ = E·ψ` discretised by second-order finite differences
    /// (Dirichlet walls). The resulting symmetric tridiagonal Hamiltonian is diagonalised
    /// by the tested `symmetric_eigen`; the lowest `num_levels` energies are returned.
    pub fn run_quantum_stationary_states_1d(
        &self,
        potential: Vec<f64>,
        dx: f64,
        mass: f64,
        hbar: f64,
        num_levels: usize,
    ) -> Result<QuantumSpectrumResult, PhysicsError> {
        let n = potential.len();
        if n < 2 || !(dx > 0.0 && mass > 0.0 && hbar > 0.0) {
            return Err(PhysicsError::InvalidConfiguration(
                "require potential length >= 2, dx > 0, mass > 0, hbar > 0".to_string(),
            ));
        }
        // Kinetic coupling t = ħ²/(2m·dx²). Diagonal 2t + V_i; off-diagonal -t.
        let t = hbar * hbar / (2.0 * mass * dx * dx);
        let mut a = vec![0.0f64; n * n];
        for i in 0..n {
            a[i * n + i] = 2.0 * t + potential[i];
            if i + 1 < n {
                a[i * n + (i + 1)] = -t;
                a[(i + 1) * n + i] = -t;
            }
        }
        let mut eigvecs = vec![0.0f64; n * n];
        symmetric_eigen(n, &mut a, &mut eigvecs)
            .map_err(|e| PhysicsError::SolverError(format!("symmetric_eigen: {:?}", e)))?;
        let mut eigenvalues: Vec<f64> = (0..n).map(|i| a[i * n + i]).collect();
        eigenvalues.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        eigenvalues.truncate(num_levels.min(n).max(1));
        Ok(QuantumSpectrumResult {
            eigenvalues,
            num_grid_points: n,
            dx,
        })
    }
}
