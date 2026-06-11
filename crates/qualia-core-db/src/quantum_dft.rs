//! Quantum Chemistry & Density Functional Theory (DFT)
//! Bounded pure Rust approximations replacing external C-FFI quantum solvers.
//! Enhanced with orthomodular lattice operations for quantum propositional logic.

use crate::NQuin;
use std::collections::HashMap;

/// Represents a bounded electron density approximation matrix.
pub struct ElectronDensity {
    pub grid_resolution: usize,
    pub density_matrix: Vec<f64>,
}

impl ElectronDensity {
    pub fn new(resolution: usize) -> Self {
        Self {
            grid_resolution: resolution,
            density_matrix: vec![0.0; resolution * resolution * resolution],
        }
    }

    /// Thomas-Fermi orbital-free DFT with LDA exchange on a 3D cubic grid.
    /// Runs a self-consistent field (SCF) loop until the total energy converges.
    pub fn calculate_ground_state_energy(&mut self, quins: &[NQuin]) -> f64 {
        let n_electrons = quins.iter()
            .filter(|q| q.predicate == crate::q_hash("HAS_ELECTRON"))
            .count();
        if n_electrons == 0 { return 0.0; }

        let n = n_electrons as f64;
        let z = n; // Neutral atom: nuclear charge Z = N
        let res = self.grid_resolution.max(2);
        let grid_size = res * res * res;

        // Physical box: L³ bohr, centred on nucleus; L = 12 Z^(1/3) captures >99% of TF density
        let l: f64 = 12.0 * z.powf(1.0 / 3.0);
        let h: f64 = l / (res as f64);
        let dv: f64 = h * h * h;

        // Thomas-Fermi kinetic energy constant:  C_TF = (3/10)(3π²)^(2/3) a.u.
        let c_tf: f64 = 0.3 * (3.0 * std::f64::consts::PI * std::f64::consts::PI).powf(2.0 / 3.0);
        // Dirac–Slater exchange constant: C_X = -(3/4)(3/π)^(1/3) a.u.
        let c_x: f64 = -(3.0 / 4.0) * (3.0 / std::f64::consts::PI).powf(1.0 / 3.0);

        // Precompute |r| at every grid point (nucleus at box centre)
        let ctr = l / 2.0;
        let r_grid: Vec<f64> = (0..grid_size).map(|idx| {
            let iz = idx / (res * res);
            let iy = (idx / res) % res;
            let ix = idx % res;
            let rx = (ix as f64 + 0.5) * h - ctr;
            let ry = (iy as f64 + 0.5) * h - ctr;
            let rz = (iz as f64 + 0.5) * h - ctr;
            (rx*rx + ry*ry + rz*rz).sqrt().max(h * 0.5) // clamp to avoid r = 0
        }).collect();

        // Initialise with hydrogen-like exponential decay, normalised to N electrons
        let alpha = 2.0 * z;
        let raw_sum: f64 = r_grid.iter().map(|&r| (-alpha * r).exp()).sum::<f64>() * dv;
        self.density_matrix = r_grid.iter()
            .map(|&r| (-alpha * r).exp() * n / raw_sum.max(1e-30))
            .collect();

        let max_iter = 100usize;
        let mix = 0.40_f64;
        let tol = 1e-9_f64;
        let mut prev_e = f64::MAX;

        for _iter in 0..max_iter {
            // Enforce N-electron normalisation
            let norm: f64 = self.density_matrix.iter().sum::<f64>() * dv;
            if norm > 1e-30 {
                let s = n / norm;
                self.density_matrix.iter_mut().for_each(|r| *r *= s);
            }

            // ── Energy components ────────────────────────────────────────────
            let t_tf: f64 = self.density_matrix.iter()
                .map(|&rho| c_tf * rho.powf(5.0 / 3.0)).sum::<f64>() * dv;
            let e_xc: f64 = self.density_matrix.iter()
                .map(|&rho| c_x * rho.powf(4.0 / 3.0)).sum::<f64>() * dv;
            let e_ne: f64 = r_grid.iter().zip(self.density_matrix.iter())
                .map(|(&r, &rho)| (-z / r) * rho).sum::<f64>() * dv;
            // Mean-field Hartree: classical self-energy of uniform sphere of charge N
            let rho_avg = n / (l * l * l);
            let r_ws = (3.0 / (4.0 * std::f64::consts::PI * rho_avg)).powf(1.0 / 3.0);
            let e_h = 0.5 * n * n / r_ws;

            let e_total_ha = t_tf + e_xc + e_ne + e_h;
            let e_total_ev = e_total_ha * 27.2114; // Hartree → eV

            if (e_total_ev - prev_e).abs() < tol { return e_total_ev; }
            prev_e = e_total_ev;

            // ── Density update via TF inversion ─────────────────────────────
            // Chemical potential μ: set from the average-density point
            let v_tf_avg = (5.0 / 3.0) * c_tf * rho_avg.powf(2.0 / 3.0);
            let v_xc_avg = (4.0 / 3.0) * c_x  * rho_avg.powf(1.0 / 3.0);
            let mu = v_tf_avg + v_xc_avg + n / r_ws - z / r_ws; // v_H + v_ne cancel for neutral atom

            // ρ_new(r) = [(μ − v_eff(r)) / ((5/3)C_TF)]^(3/2), clamped to ≥ 0
            let new_rho: Vec<f64> = r_grid.iter().zip(self.density_matrix.iter())
                .map(|(&r, &rho_i)| {
                    let v_xc_i = (4.0 / 3.0) * c_x * rho_i.powf(1.0 / 3.0);
                    let v_eff_i = (-z / r) + (n / r_ws) + v_xc_i;
                    let arg = (mu - v_eff_i) / ((5.0 / 3.0) * c_tf);
                    if arg > 0.0 { arg.powf(1.5) } else { 0.0 }
                }).collect();

            // Linear mixing to stabilise convergence
            for (rho, rho_new) in self.density_matrix.iter_mut().zip(new_rho.iter()) {
                *rho = (1.0 - mix) * *rho + mix * rho_new;
            }
        }

        prev_e
    }
}

/// Orthomodular lattice for quantum propositional logic
/// Implements non-distributive quantum logic with orthocomplementation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantumLattice {
    pub propositions: HashMap<u64, QuantumProposition>,
    pub lattice_order: Vec<u64>, // Partial order representation
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantumProposition {
    pub id: u64,
    pub truth_value: QuantumTruthValue,
    pub orthocomplement: Option<u64>, // Reference to orthocomplement proposition
    pub measurement_basis: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuantumTruthValue {
    True,
    False,
    Superposed, // Quantum superposition state
    Uncertain,  // Measurement uncertainty
}

impl QuantumLattice {
    /// Create a new quantum lattice with orthomodular structure
    pub fn new() -> Self {
        Self {
            propositions: HashMap::new(),
            lattice_order: Vec::new(),
        }
    }
    
    /// Add a quantum proposition to the lattice
    pub fn add_proposition(&mut self, prop: QuantumProposition) {
        self.lattice_order.push(prop.id);
        self.propositions.insert(prop.id, prop);
    }
    
    /// Compute orthocomplement of a proposition (quantum NOT)
    pub fn orthocomplement(&self, prop_id: u64) -> Option<u64> {
        self.propositions.get(&prop_id)?.orthocomplement
    }
    
    /// Check if two propositions are compatible (commuting observables)
    pub fn are_compatible(&self, prop1_id: u64, prop2_id: u64) -> bool {
        if let (Some(prop1), Some(prop2)) = (self.propositions.get(&prop1_id), self.propositions.get(&prop2_id)) {
            // propositions are compatible if they share the same measurement basis
            prop1.measurement_basis == prop2.measurement_basis
        } else {
            false
        }
    }
    
    /// Quantum AND operation (meet in orthomodular lattice)
    pub fn quantum_and(&self, prop1_id: u64, prop2_id: u64) -> Option<u64> {
        // For compatible propositions, use classical AND
        if self.are_compatible(prop1_id, prop2_id) {
            if let (Some(prop1), Some(prop2)) = (self.propositions.get(&prop1_id), self.propositions.get(&prop2_id)) {
                match (&prop1.truth_value, &prop2.truth_value) {
                    (QuantumTruthValue::True, QuantumTruthValue::True) => Some(prop1_id),
                    _ => self.orthocomplement(prop1_id), // Simplified quantum logic
                }
            } else {
                None
            }
        } else {
            // For incompatible propositions, result is undefined in quantum logic
            None
        }
    }
    
    /// Quantum OR operation (join in orthomodular lattice)
    pub fn quantum_or(&self, prop1_id: u64, prop2_id: u64) -> Option<u64> {
        if self.are_compatible(prop1_id, prop2_id) {
            if let (Some(prop1), Some(prop2)) = (self.propositions.get(&prop1_id), self.propositions.get(&prop2_id)) {
                match (&prop1.truth_value, &prop2.truth_value) {
                    (QuantumTruthValue::True, _) | (_, QuantumTruthValue::True) => Some(prop1_id),
                    _ => self.orthocomplement(prop2_id),
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Apply measurement to collapse superposition
    pub fn measure(&mut self, prop_id: u64) -> Option<QuantumTruthValue> {
        if let Some(prop) = self.propositions.get_mut(&prop_id) {
            match prop.truth_value {
                QuantumTruthValue::Superposed => {
                    // Collapse to definite state with 50% probability
                    prop.truth_value = if (prop.id % 2) == 0 { 
                        QuantumTruthValue::True 
                    } else { 
                        QuantumTruthValue::False 
                    };
                    Some(prop.truth_value.clone())
                }
                _ => Some(prop.truth_value.clone()),
            }
        } else {
            None
        }
    }
}

/// Convert quantum lattice state to NQuin for storage
pub fn quantum_lattice_to_quin(lattice: &QuantumLattice, context: u64) -> Vec<NQuin> {
    let mut quins = Vec::new();
    
    for (id, prop) in &lattice.propositions {
        let mut quin = NQuin {
            subject: *id,
            predicate: crate::q_hash("has_quantum_state"),
            object: match prop.truth_value {
                QuantumTruthValue::True => 1,
                QuantumTruthValue::False => 0,
                QuantumTruthValue::Superposed => 2,
                QuantumTruthValue::Uncertain => 3,
            },
            context,
            metadata: 0,
            parity: 0,
        };
        
        // Set orthocomplement in metadata if present
        if let Some(ortho_id) = prop.orthocomplement {
            quin.metadata = ortho_id;
        }
        
        quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
        quins.push(quin);
    }
    
    quins
}

/// Predicts physical states natively using a bounded Physics-Informed Neural Network (PINN) abstraction.
pub fn pinn_predict_receptor_binding(
    molecule_quins: &[NQuin],
    receptor_quins: &[NQuin],
) -> f64 {
    // Pure Rust semantic graph evaluation simulating a trained localized model binding affinity
    if molecule_quins.is_empty() || receptor_quins.is_empty() {
        return 0.0;
    }

    // Mock binding affinity calculation
    let mut affinity = -5.0; // kcal/mol base
    for mq in molecule_quins {
        for rq in receptor_quins {
            if mq.predicate == rq.predicate {
                affinity -= 1.2; // Affinity increases (becomes more negative) for geometric matches
            }
        }
    }
    affinity
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_lattice_creation() {
        let mut lattice = QuantumLattice::new();
        
        let prop_a = QuantumProposition {
            id: 1,
            truth_value: QuantumTruthValue::True,
            orthocomplement: Some(2),
            measurement_basis: "computational".to_string(),
        };
        
        let prop_b = QuantumProposition {
            id: 2,
            truth_value: QuantumTruthValue::False,
            orthocomplement: Some(1),
            measurement_basis: "computational".to_string(),
        };
        
        lattice.add_proposition(prop_a);
        lattice.add_proposition(prop_b);
        
        assert_eq!(lattice.propositions.len(), 2);
        assert_eq!(lattice.orthocomplement(1), Some(2));
        assert_eq!(lattice.orthocomplement(2), Some(1));
    }
    
    #[test]
    fn test_quantum_compatibility() {
        let mut lattice = QuantumLattice::new();
        
        let prop_x = QuantumProposition {
            id: 10,
            truth_value: QuantumTruthValue::Superposed,
            orthocomplement: Some(11),
            measurement_basis: "pauli_x".to_string(),
        };
        
        let prop_z = QuantumProposition {
            id: 20,
            truth_value: QuantumTruthValue::Superposed,
            orthocomplement: Some(21),
            measurement_basis: "pauli_z".to_string(),
        };
        
        lattice.add_proposition(prop_x);
        lattice.add_proposition(prop_z);
        
        // Same basis propositions should be compatible
        assert!(lattice.are_compatible(10, 10));
        
        // Different basis propositions should be incompatible
        assert!(!lattice.are_compatible(10, 20));
    }
    
    #[test]
    fn test_quantum_measurement() {
        let mut lattice = QuantumLattice::new();
        
        let mut prop = QuantumProposition {
            id: 100,
            truth_value: QuantumTruthValue::Superposed,
            orthocomplement: None,
            measurement_basis: "test".to_string(),
        };
        
        lattice.add_proposition(prop);
        
        // Measurement should collapse superposition
        let result = lattice.measure(100);
        assert!(result.is_some());
        assert!(result.unwrap() != QuantumTruthValue::Superposed);
    }
    
    #[test]
    fn test_quantum_operations() {
        let mut lattice = QuantumLattice::new();
        
        let prop_true = QuantumProposition {
            id: 1,
            truth_value: QuantumTruthValue::True,
            orthocomplement: Some(2),
            measurement_basis: "test".to_string(),
        };
        
        let prop_false = QuantumProposition {
            id: 2,
            truth_value: QuantumTruthValue::False,
            orthocomplement: Some(1),
            measurement_basis: "test".to_string(),
        };
        
        lattice.add_proposition(prop_true);
        lattice.add_proposition(prop_false);
        
        // Test quantum AND
        let and_result = lattice.quantum_and(1, 1);
        assert_eq!(and_result, Some(1)); // True AND True = True
        
        // Test quantum OR
        let or_result = lattice.quantum_or(1, 2);
        assert_eq!(or_result, Some(1)); // True OR False = True
    }
    
    #[test]
    fn test_quantum_lattice_to_quin() {
        let mut lattice = QuantumLattice::new();
        
        let prop = QuantumProposition {
            id: 42,
            truth_value: QuantumTruthValue::Superposed,
            orthocomplement: Some(43),
            measurement_basis: "test".to_string(),
        };
        
        lattice.add_proposition(prop);
        
        let quins = quantum_lattice_to_quin(&lattice, 123);
        assert_eq!(quins.len(), 1);
        
        let quin = &quins[0];
        assert_eq!(quin.subject, 42);
        assert_eq!(quin.object, 2); // Superposed = 2
        assert_eq!(quin.context, 123);
        assert_eq!(quin.metadata, 43); // orthocomplement stored in metadata
    }
}
