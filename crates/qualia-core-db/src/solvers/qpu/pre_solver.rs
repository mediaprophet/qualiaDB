//! QPU Pre-Solver — converts high-level problem descriptions into provider-ready
//! QUBO matrices or gate-model quantum circuits.
//!
//! Ported from `qpu/src/pre_solver.rs`.

use super::{JobParameters, ProblemType, QpuError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Problem description ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemDescription {
    pub problem_type: ProblemType,
    pub variables: Vec<Variable>,
    pub constraints: Vec<Constraint>,
    pub objective: Objective,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub domain: VariableDomain,
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableDomain {
    Binary,
    Spin,
    Integer { min: i32, max: i32 },
    Continuous { min: f64, max: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub constraint_type: ConstraintType,
    pub variables: Vec<String>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintType {
    Linear,
    Quadratic,
    Equality,
    Inequality,
    Logical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Objective {
    pub objective_type: ObjectiveType,
    pub expression: String,
    pub minimize: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectiveType {
    Linear,
    Quadratic,
    Polynomial,
    Custom,
}

// ── QUBO formulation ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuboFormulation {
    pub num_variables: u32,
    /// (variable_index, coefficient)
    pub linear_terms: Vec<(u32, f64)>,
    /// (var_a, var_b, coefficient)
    pub quadratic_terms: Vec<(u32, u32, f64)>,
    pub offset: f64,
}

impl QuboFormulation {
    /// Creates an empty QUBO formulation with `num_variables`.
    pub fn new(num_variables: u32) -> Self {
        Self {
            num_variables,
            linear_terms: Vec::new(),
            quadratic_terms: Vec::new(),
            offset: 0.0,
        }
    }

    /// Adds a linear coefficient for `variable`.
    pub fn add_linear_term(&mut self, variable: u32, coefficient: f64) {
        self.linear_terms.push((variable, coefficient));
    }

    /// Adds a quadratic coupler `(var_a, var_b)`.
    pub fn add_quadratic_term(&mut self, var_a: u32, var_b: u32, coefficient: f64) {
        self.quadratic_terms.push((var_a, var_b, coefficient));
    }

    /// Evaluate the QUBO energy `E(x)` at a binary assignment `bits` (`0` or `1` per variable).
    pub fn evaluate_binary(&self, bits: &[u8]) -> f64 {
        let mut energy = self.offset;
        for &(v, c) in &self.linear_terms {
            let x = if bits.get(v as usize).copied().unwrap_or(0) != 0 {
                1.0
            } else {
                0.0
            };
            energy += c * x;
        }
        for &(a, b, c) in &self.quadratic_terms {
            let xa = if bits.get(a as usize).copied().unwrap_or(0) != 0 {
                1.0
            } else {
                0.0
            };
            let xb = if bits.get(b as usize).copied().unwrap_or(0) != 0 {
                1.0
            } else {
                0.0
            };
            energy += c * xa * xb;
        }
        energy
    }

    /// Serializes the formulation into QPU `JobParameters`.
    pub fn to_job_parameters(&self) -> JobParameters {
        JobParameters {
            num_qubits: self.num_variables,
            hamiltonian: serde_json::to_string(self).ok(),
            circuit: None,
            circuit_depth: 1,
            shots: 1000,
            extra: serde_json::json!({"formulation": "qubo"}),
        }
    }
}

// ── Circuit formulation ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitFormulation {
    pub num_qubits: u32,
    pub gates: Vec<Gate>,
    pub measurements: Vec<Measurement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate {
    pub gate_type: GateType,
    pub qubits: Vec<u32>,
    pub parameters: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateType {
    H,
    X,
    Y,
    Z,
    Rx,
    Ry,
    Rz,
    CNOT,
    CZ,
    SWAP,
    ZZ,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurement {
    pub qubit: u32,
    pub basis: MeasurementBasis,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MeasurementBasis {
    Computational,
    X,
    Y,
    Z,
}

impl CircuitFormulation {
    /// Creates an empty gate-model formulation with `num_qubits`.
    pub fn new(num_qubits: u32) -> Self {
        Self {
            num_qubits,
            gates: Vec::new(),
            measurements: Vec::new(),
        }
    }

    /// Appends a gate to the circuit.
    pub fn add_gate(&mut self, gate: Gate) {
        self.gates.push(gate);
    }

    /// Appends a measurement.
    pub fn add_measurement(&mut self, m: Measurement) {
        self.measurements.push(m);
    }

    /// Serializes the circuit into QPU `JobParameters`.
    pub fn to_job_parameters(&self) -> JobParameters {
        JobParameters {
            num_qubits: self.num_qubits,
            hamiltonian: None,
            circuit: serde_json::to_string(self).ok(),
            circuit_depth: self.gates.len() as u32,
            shots: 1000,
            extra: serde_json::json!({"formulation": "circuit"}),
        }
    }
}

// ── QUBO Penalty Sufficiency Theorem (Q42-08 / §6.2.3) ────────────────────────

/// Bounds for enforcing the QUBO Penalty Sufficiency Theorem (`P * delta > U - L`)
/// on a penalized Hamiltonian `E(x) = f(x) + P * g(x)`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QuboPenaltyBounds {
    /// Global lower bound `L <= min_x f(x)` over all binary states (including infeasible).
    pub objective_lower_bound: f64,
    /// Upper bound `U >= min_{x feasible} f(x)` on at least one known feasible state.
    pub feasible_upper_bound: f64,
    /// Minimum positive constraint violation gap `delta <= min_{x infeasible} g(x)`.
    pub min_violation_delta: f64,
    /// Penalty multiplier `P` applied to `g(x)`.
    pub penalty_weight: f64,
}

/// Enforce the **QUBO Penalty Sufficiency Theorem**:
/// For `E(x) = f(x) + P * g(x)` with `g(x) = 0` on feasible states and `g(x) >= delta > 0`
/// on infeasible states, `P * delta > U - L` guarantees every global ground state of `E(x)`
/// is feasible.
pub fn verify_qubo_penalty_sufficiency(bounds: &QuboPenaltyBounds) -> Result<(), QpuError> {
    if !bounds.objective_lower_bound.is_finite()
        || !bounds.feasible_upper_bound.is_finite()
        || !bounds.min_violation_delta.is_finite()
        || !bounds.penalty_weight.is_finite()
    {
        return Err(QpuError::Api(
            "QUBO penalty bounds must be finite real numbers".to_string(),
        ));
    }
    if bounds.min_violation_delta <= 0.0 {
        return Err(QpuError::Api(format!(
            "QUBO minimum violation gap delta must be > 0, got {}",
            bounds.min_violation_delta
        )));
    }
    if bounds.feasible_upper_bound < bounds.objective_lower_bound {
        return Err(QpuError::Api(format!(
            "QUBO feasible upper bound U ({}) < objective lower bound L ({})",
            bounds.feasible_upper_bound, bounds.objective_lower_bound
        )));
    }
    let span = bounds.feasible_upper_bound - bounds.objective_lower_bound;
    let penalty_gap = bounds.penalty_weight * bounds.min_violation_delta;
    if !(penalty_gap > span) {
        return Err(QpuError::Api(format!(
            "QUBO Penalty Sufficiency Theorem violated: P * delta ({} * {} = {}) must be strictly > U - L ({})",
            bounds.penalty_weight, bounds.min_violation_delta, penalty_gap, span
        )));
    }
    Ok(())
}

/// Compute a sufficient penalty weight `P = (U - L) / delta + safety_margin` (`safety_margin > 0`).
pub fn minimum_sufficient_penalty(
    objective_lower_bound: f64,
    feasible_upper_bound: f64,
    min_violation_delta: f64,
    safety_margin: f64,
) -> Result<f64, QpuError> {
    if !safety_margin.is_finite() || safety_margin <= 0.0 {
        return Err(QpuError::Api(
            "QUBO penalty safety_margin must be finite and > 0".to_string(),
        ));
    }
    if !min_violation_delta.is_finite() || min_violation_delta <= 0.0 {
        return Err(QpuError::Api(
            "QUBO min_violation_delta must be finite and > 0".to_string(),
        ));
    }
    if !objective_lower_bound.is_finite()
        || !feasible_upper_bound.is_finite()
        || feasible_upper_bound < objective_lower_bound
    {
        return Err(QpuError::Api(
            "QUBO objective bounds must be finite with U >= L".to_string(),
        ));
    }
    Ok((feasible_upper_bound - objective_lower_bound) / min_violation_delta + safety_margin)
}

// ── Pre-solver ────────────────────────────────────────────────────────────────

pub struct PreSolver {
    variable_map: HashMap<String, u32>,
}

impl PreSolver {
    /// Creates a pre-solver with the default formulation strategy.
    pub fn new() -> Self {
        Self {
            variable_map: HashMap::new(),
        }
    }

    /// Formulates `problem` into executable `JobParameters`; `QpuError` on unsupported problem kinds.
    pub fn formulate(&mut self, problem: &ProblemDescription) -> Result<JobParameters, QpuError> {
        match problem.problem_type {
            ProblemType::Annealing => self.formulate_qubo(problem),
            ProblemType::GateModel => self.formulate_circuit(problem),
            ProblemType::Vqe | ProblemType::Qaoa => self.formulate_circuit(problem),
        }
    }

    /// Formulate a constrained QUBO only after verifying the QUBO Penalty Sufficiency Theorem
    /// (`P * delta > U - L`).
    pub fn formulate_penalized_qubo(
        &mut self,
        problem: &ProblemDescription,
        bounds: &QuboPenaltyBounds,
    ) -> Result<JobParameters, QpuError> {
        verify_qubo_penalty_sufficiency(bounds)?;
        self.formulate_qubo(problem)
    }

    fn formulate_qubo(&mut self, problem: &ProblemDescription) -> Result<JobParameters, QpuError> {
        let mut qubo = QuboFormulation::new(problem.variables.len() as u32);
        self.variable_map.clear();
        for var in &problem.variables {
            self.variable_map.insert(var.name.clone(), var.index);
        }
        for c in &problem.constraints {
            self.apply_constraint_qubo(&mut qubo, c)?;
        }
        self.apply_objective_qubo(&mut qubo, &problem.objective)?;
        Ok(qubo.to_job_parameters())
    }

    fn formulate_circuit(
        &mut self,
        problem: &ProblemDescription,
    ) -> Result<JobParameters, QpuError> {
        let mut circuit = CircuitFormulation::new(problem.variables.len() as u32);
        self.variable_map.clear();
        for var in &problem.variables {
            self.variable_map.insert(var.name.clone(), var.index);
            circuit.add_gate(Gate {
                gate_type: GateType::H,
                qubits: vec![var.index],
                parameters: vec![],
            });
        }
        for c in &problem.constraints {
            self.apply_constraint_circuit(&mut circuit, c)?;
        }
        for var in &problem.variables {
            circuit.add_measurement(Measurement {
                qubit: var.index,
                basis: MeasurementBasis::Computational,
            });
        }
        Ok(circuit.to_job_parameters())
    }

    fn apply_constraint_qubo(
        &self,
        qubo: &mut QuboFormulation,
        c: &Constraint,
    ) -> Result<(), QpuError> {
        match c.constraint_type {
            ConstraintType::Linear => {
                if let Some(coeffs) = c.parameters.as_array() {
                    for (i, coeff) in coeffs.iter().enumerate() {
                        if let (Some(val), Some(name)) = (coeff.as_f64(), c.variables.get(i)) {
                            if let Some(&idx) = self.variable_map.get(name.as_str()) {
                                qubo.add_linear_term(idx, val);
                            }
                        }
                    }
                }
                Ok(())
            }
            ConstraintType::Quadratic => {
                if let Some(params) = c.parameters.as_object() {
                    for (key, value) in params {
                        if let Some(coeff) = value.as_f64() {
                            let parts: Vec<&str> = key.splitn(2, ',').collect();
                            if parts.len() == 2 {
                                if let (Some(&a), Some(&b)) = (
                                    self.variable_map.get(parts[0]),
                                    self.variable_map.get(parts[1]),
                                ) {
                                    qubo.add_quadratic_term(a, b, coeff);
                                }
                            }
                        }
                    }
                }
                Ok(())
            }
            ref t => Err(QpuError::Api(format!(
                "Constraint type {:?} not supported for QUBO",
                t
            ))),
        }
    }

    fn apply_constraint_circuit(
        &self,
        circuit: &mut CircuitFormulation,
        c: &Constraint,
    ) -> Result<(), QpuError> {
        if c.constraint_type == ConstraintType::Logical && c.variables.len() == 2 {
            let a = self
                .variable_map
                .get(c.variables[0].as_str())
                .copied()
                .unwrap_or(0);
            let b = self
                .variable_map
                .get(c.variables[1].as_str())
                .copied()
                .unwrap_or(0);
            circuit.add_gate(Gate {
                gate_type: GateType::CNOT,
                qubits: vec![a, b],
                parameters: vec![],
            });
            Ok(())
        } else {
            Err(QpuError::Api(format!(
                "Constraint type {:?} not supported for circuit",
                c.constraint_type
            )))
        }
    }

    fn apply_objective_qubo(
        &self,
        qubo: &mut QuboFormulation,
        obj: &Objective,
    ) -> Result<(), QpuError> {
        let sign = if obj.minimize { 1.0 } else { -1.0 };
        match obj.objective_type {
            ObjectiveType::Linear => {
                for &idx in self.variable_map.values() {
                    qubo.add_linear_term(idx, sign);
                }
                Ok(())
            }
            ObjectiveType::Quadratic => {
                let vars: Vec<u32> = self.variable_map.values().cloned().collect();
                for i in 0..vars.len() {
                    for j in i..vars.len() {
                        qubo.add_quadratic_term(vars[i], vars[j], sign);
                    }
                }
                Ok(())
            }
            ref t => Err(QpuError::Api(format!(
                "Objective type {:?} not supported",
                t
            ))),
        }
    }
}

impl Default for PreSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_var_problem(pt: ProblemType) -> ProblemDescription {
        ProblemDescription {
            problem_type: pt,
            variables: vec![
                Variable {
                    name: "x0".into(),
                    domain: VariableDomain::Binary,
                    index: 0,
                },
                Variable {
                    name: "x1".into(),
                    domain: VariableDomain::Binary,
                    index: 1,
                },
            ],
            constraints: vec![],
            objective: Objective {
                objective_type: ObjectiveType::Linear,
                expression: "x0 + x1".into(),
                minimize: true,
            },
        }
    }

    #[test]
    fn qubo_formulation_roundtrip() {
        let mut solver = PreSolver::new();
        let params = solver
            .formulate(&two_var_problem(ProblemType::Annealing))
            .unwrap();
        assert_eq!(params.num_qubits, 2);
        assert!(params.hamiltonian.is_some());
    }

    #[test]
    fn circuit_formulation_roundtrip() {
        let mut solver = PreSolver::new();
        let params = solver
            .formulate(&two_var_problem(ProblemType::GateModel))
            .unwrap();
        assert_eq!(params.num_qubits, 2);
        assert!(params.circuit.is_some());
    }

    #[test]
    fn qubo_add_terms() {
        let mut q = QuboFormulation::new(2);
        q.add_linear_term(0, 1.0);
        q.add_linear_term(1, -1.0);
        q.add_quadratic_term(0, 1, 0.5);
        assert_eq!(q.linear_terms.len(), 2);
        assert_eq!(q.quadratic_terms.len(), 1);
    }

    #[test]
    fn qubo_penalty_sufficiency_theorem_enforces_feasible_ground_state() {
        // Minimize f(x0, x1) = -3*x0 - 3*x1 subject to x0 + x1 <= 1 (g(x0, x1) = x0 * x1, delta = 1).
        // Feasible states: (0,0)->0, (1,0)->-3, (0,1)->-3 => U = -3.
        // Infeasible state: (1,1) has f(1,1) = -6 => L = -6, so U - L = 3.
        let insufficient = QuboPenaltyBounds {
            objective_lower_bound: -6.0,
            feasible_upper_bound: -3.0,
            min_violation_delta: 1.0,
            penalty_weight: 2.0, // P * delta = 2 <= U - L = 3 (under-penalized!)
        };
        assert!(verify_qubo_penalty_sufficiency(&insufficient).is_err());

        let mut solver = PreSolver::new();
        assert!(solver
            .formulate_penalized_qubo(&two_var_problem(ProblemType::Annealing), &insufficient)
            .is_err());

        // Demonstrate that under P = 2.0, infeasible (1,1) has lower energy (-4) than feasible (1,0) (-3):
        let mut bad_qubo = QuboFormulation::new(2);
        bad_qubo.add_linear_term(0, -3.0);
        bad_qubo.add_linear_term(1, -3.0);
        bad_qubo.add_quadratic_term(0, 1, 2.0);
        assert!(bad_qubo.evaluate_binary(&[1, 1]) < bad_qubo.evaluate_binary(&[1, 0]));

        // Compute sufficient penalty P > (U - L) / delta = 3.0:
        let p_suff = minimum_sufficient_penalty(-6.0, -3.0, 1.0, 1.0).unwrap();
        assert!((p_suff - 4.0).abs() < 1e-12);
        let sufficient = QuboPenaltyBounds {
            objective_lower_bound: -6.0,
            feasible_upper_bound: -3.0,
            min_violation_delta: 1.0,
            penalty_weight: p_suff,
        };
        assert!(verify_qubo_penalty_sufficiency(&sufficient).is_ok());
        assert!(solver
            .formulate_penalized_qubo(&two_var_problem(ProblemType::Annealing), &sufficient)
            .is_ok());

        // Under P = 4.0, infeasible (1,1) has energy -2 > feasible ground state energy -3:
        let mut good_qubo = QuboFormulation::new(2);
        good_qubo.add_linear_term(0, -3.0);
        good_qubo.add_linear_term(1, -3.0);
        good_qubo.add_quadratic_term(0, 1, p_suff);
        assert_eq!(good_qubo.evaluate_binary(&[1, 0]), -3.0);
        assert_eq!(good_qubo.evaluate_binary(&[0, 1]), -3.0);
        assert_eq!(good_qubo.evaluate_binary(&[1, 1]), -2.0);
        assert!(good_qubo.evaluate_binary(&[1, 1]) > good_qubo.evaluate_binary(&[1, 0]));
    }
}
