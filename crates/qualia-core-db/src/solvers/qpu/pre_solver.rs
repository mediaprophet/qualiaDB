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
    pub fn new(num_variables: u32) -> Self {
        Self {
            num_variables,
            linear_terms: Vec::new(),
            quadratic_terms: Vec::new(),
            offset: 0.0,
        }
    }

    pub fn add_linear_term(&mut self, variable: u32, coefficient: f64) {
        self.linear_terms.push((variable, coefficient));
    }

    pub fn add_quadratic_term(&mut self, var_a: u32, var_b: u32, coefficient: f64) {
        self.quadratic_terms.push((var_a, var_b, coefficient));
    }

    pub fn to_job_parameters(&self) -> JobParameters {
        JobParameters {
            num_qubits: self.num_variables,
            hamiltonian: serde_json::to_string(self).ok(),
            circuit: None,
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
    pub fn new(num_qubits: u32) -> Self {
        Self {
            num_qubits,
            gates: Vec::new(),
            measurements: Vec::new(),
        }
    }

    pub fn add_gate(&mut self, gate: Gate) {
        self.gates.push(gate);
    }

    pub fn add_measurement(&mut self, m: Measurement) {
        self.measurements.push(m);
    }

    pub fn to_job_parameters(&self) -> JobParameters {
        JobParameters {
            num_qubits: self.num_qubits,
            hamiltonian: None,
            circuit: serde_json::to_string(self).ok(),
            shots: 1000,
            extra: serde_json::json!({"formulation": "circuit"}),
        }
    }
}

// ── Pre-solver ────────────────────────────────────────────────────────────────

pub struct PreSolver {
    variable_map: HashMap<String, u32>,
}

impl PreSolver {
    pub fn new() -> Self {
        Self {
            variable_map: HashMap::new(),
        }
    }

    pub fn formulate(&mut self, problem: &ProblemDescription) -> Result<JobParameters, QpuError> {
        match problem.problem_type {
            ProblemType::Annealing => self.formulate_qubo(problem),
            ProblemType::GateModel => self.formulate_circuit(problem),
            ProblemType::Vqe | ProblemType::Qaoa => self.formulate_circuit(problem),
        }
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
}
