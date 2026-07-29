use crate::cli::{
    LinalgAction, OdeAction, OptimizeAction, QuantumSolveAction, SolveAction, SymbolicSolveAction,
};
use crate::solve;

pub fn handle(action: &SolveAction) {
    match action {
        SolveAction::Linalg { action } => match action {
            LinalgAction::Multiply { matrix_a, matrix_b } => {
                solve::run_matrix_multiply(matrix_a, matrix_b);
            }
            LinalgAction::Determinant { matrix } => {
                solve::run_determinant(matrix);
            }
            LinalgAction::SolveSystem { matrix, vector } => {
                solve::run_solve_system(matrix, vector);
            }
            LinalgAction::Eigenvalues { matrix, count } => {
                solve::run_eigenvalues(matrix, *count);
            }
            LinalgAction::TensorContract { tensor } => {
                solve::run_tensor_contract(tensor);
            }
        },
        SolveAction::Optimize { action } => match action {
            OptimizeAction::Simplex {
                initial,
                iterations,
            } => {
                solve::run_simplex(initial, *iterations);
            }
            OptimizeAction::Root {
                initial,
                lower,
                upper,
                tolerance,
            } => {
                solve::run_root(*initial, *lower, *upper, *tolerance);
            }
            OptimizeAction::CurveFit {
                initial_params,
                x_data,
                y_data,
            } => {
                solve::run_curve_fit(initial_params, x_data, y_data);
            }
        },
        SolveAction::Ode { action } => match action {
            OdeAction::Rk4 {
                lambda,
                t_start,
                t_end,
                y0,
                step_size,
            } => {
                solve::run_ode_rk4(*lambda, *t_start, *t_end, *y0, *step_size);
            }
            OdeAction::Harmonic {
                omega,
                t_start,
                t_end,
                y0,
                step_size,
            } => {
                solve::run_ode_harmonic(*omega, *t_start, *t_end, *y0, *step_size);
            }
            OdeAction::Bvp {
                t_start,
                t_end,
                y_left,
                y_right,
                threshold,
            } => {
                solve::run_ode_bvp(*t_start, *t_end, *y_left, *y_right, *threshold);
            }
            OdeAction::QuantumSpectrum {
                planck_mass,
                coupling,
                max_n,
                frequency,
            } => {
                solve::run_ode_quantum_spectrum(*planck_mass, *coupling, *max_n, *frequency);
            }
        },
        SolveAction::Quantum { action } => match action {
            QuantumSolveAction::Qaoa { depth, beta, gamma } => {
                solve::run_quantum_qaoa(*depth, beta, gamma);
            }
            QuantumSolveAction::Spsa {
                num_params,
                initial,
            } => {
                solve::run_quantum_spsa(*num_params, initial);
            }
        },
        SolveAction::Symbolic { action } => match action {
            SymbolicSolveAction::Defeasible { facts, rules } => {
                solve::run_symbolic_defeasible(facts, rules);
            }
            SymbolicSolveAction::Sat { clauses } => {
                solve::run_symbolic_sat(clauses);
            }
        },
    }
}
