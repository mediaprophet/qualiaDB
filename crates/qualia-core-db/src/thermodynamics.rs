//! Thermodynamics & Statistical Ensembles
//! Implements pure Rust Markov Chain Monte Carlo (MCMC) sampling for macroscopic properties.


/// State of a thermodynamic ensemble
#[derive(Clone)]
pub struct EnsembleState {
    pub temperature: f64,
    pub particles: usize,
    pub total_energy: f64,
}

/// Computes thermodynamic macroscopic properties from discrete structures via MCMC.
pub struct ThermodynamicSampler {
    pub current_state: EnsembleState,
}

impl ThermodynamicSampler {
    pub fn new(initial_temp: f64, particles: usize) -> Self {
        Self {
            current_state: EnsembleState {
                temperature: initial_temp,
                particles,
                total_energy: 0.0,
            },
        }
    }

    /// Performs a Metropolis-Hastings MCMC step
    pub fn metropolis_step(&mut self, proposed_energy: f64, random_uniform: f64) -> bool {
        let delta_e = proposed_energy - self.current_state.total_energy;
        
        // Accept if energy decreases or strictly probabilistically according to Boltzmann distribution
        let k_b = 8.617333262145e-5; // Boltzmann constant in eV/K
        let beta = 1.0 / (k_b * self.current_state.temperature);
        
        let acceptance_probability = if delta_e < 0.0 {
            1.0
        } else {
            (-beta * delta_e).exp()
        };

        if random_uniform < acceptance_probability {
            self.current_state.total_energy = proposed_energy;
            true // Accepted
        } else {
            false // Rejected
        }
    }

    /// Calculates macroscopic Gibbs Free Energy approximation
    pub fn calculate_gibbs_free_energy(&self, enthalpy: f64, entropy: f64) -> f64 {
        // G = H - TS
        enthalpy - (self.current_state.temperature * entropy)
    }
}
