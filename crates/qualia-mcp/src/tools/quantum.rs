//! MCP tools for Quantum Computing, QUBO Optimization, and DFT Density Functional Theory.
//!
//! Migrated from legacy `mcp_stub_impls.rs` to dedicated library module.
//! Designed according to `AGENTS.md` §0-B (<500 lines).

use crate::types::{parse_quin_slice, parse_tool_args, McpSystemError};
use serde_json::json;

/// `qpu_optimize` tool — formulates a problem via PreSolver, loads QUBO Hamiltonian into QuboMatrix, and solves classically.
pub fn qpu_optimize(args: &[u8]) -> Result<String, McpSystemError> {
    use qualia_core_db::qubo_compiler::{solve_classical, QuboMatrix};
    use qualia_core_db::solvers::qpu::pre_solver::{PreSolver, ProblemDescription, QuboFormulation};

    let v = parse_tool_args(args)?;
    let problem_val = v.get("problem").ok_or(McpSystemError::InvalidParameters)?;
    let problem: ProblemDescription = serde_json::from_value(problem_val.clone())
        .map_err(|_| McpSystemError::InvalidParameters)?;

    let mut solver = PreSolver::new();
    let job = solver
        .formulate(&problem)
        .map_err(|_| McpSystemError::ParseError)?;

    let mut matrix = QuboMatrix::new(problem.variables.len().max(1));
    if let Some(ham) = job.hamiltonian.as_deref() {
        if let Ok(form) = serde_json::from_str::<QuboFormulation>(ham) {
            for (idx, coeff) in form.linear_terms {
                matrix.set_linear(idx as usize, coeff);
            }
            for (a, b, coeff) in form.quadratic_terms {
                matrix.set_quadratic(a as usize, b as usize, coeff);
            }
        }
    }

    let mut assignment = vec![0u8; matrix.num_vars];
    let energy = solve_classical(&matrix, &mut assignment);

    let payload = json!({
        "jobParameters": job,
        "classicalEnergy": energy,
        "assignment": assignment,
        "numVariables": matrix.num_vars,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `qpu_dft` tool — computes ground-state energy of electron quins on a density grid via Thomas-Fermi / DFT.
pub fn qpu_dft(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let resolution = v
        .get("grid_resolution")
        .and_then(|x| x.as_u64())
        .unwrap_or(8) as usize;
    let quins = if v.get("quins").is_some() {
        parse_quin_slice(&v, "quins")?
    } else {
        Vec::new()
    };

    let mut dft = qualia_core_db::quantum_dft::ElectronDensity::new(resolution.max(2));
    let energy = dft.calculate_ground_state_energy(&quins);
    let payload = json!({
        "groundStateEnergy": energy,
        "gridResolution": dft.grid_resolution,
        "electronQuinCount": quins.iter().filter(|q| q.predicate == qualia_core_db::q_hash("HAS_ELECTRON")).count(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// Helper mapping `QPUJobStatus` to string label.
fn qpu_job_status_label(status: qualia_core_db::specialized_libs::qpu_bridge::QPUJobStatus) -> &'static str {
    use qualia_core_db::specialized_libs::qpu_bridge::QPUJobStatus;
    match status {
        QPUJobStatus::Queued => "queued",
        QPUJobStatus::Running => "running",
        QPUJobStatus::Completed => "completed",
        QPUJobStatus::Failed => "failed",
        QPUJobStatus::Cancelled => "cancelled",
        QPUJobStatus::Timeout => "timeout",
    }
}

/// `qpu_status` tool — inspects connection flag and current job queue status of the QPU bridge manager.
pub fn qpu_status(_args: &[u8]) -> Result<String, McpSystemError> {
    use qualia_core_db::specialized_libs::qpu_bridge::QPUBridgeManager;
    let bridge = QPUBridgeManager::new();
    let payload = json!({
        "connected": bridge.is_connected(),
        "jobStatus": qpu_job_status_label(bridge.get_job_status()),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}
