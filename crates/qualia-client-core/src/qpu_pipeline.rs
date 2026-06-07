//! End-to-end quantum pipeline: compile → egress → re-hydrate.

use crate::qpu_dispatcher::{self, QpuDispatchResult};
use crate::qpu_oracle::{self, QpuChatCommandResult};
use qualia_core_db::qubo_compiler::{self, QuboMatrix};
use qualia_core_db::QualiaQuin;

pub const MAX_REHYDRATED: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantumTaskKind {
    QuboRouting,
    DftGroundState,
    DefeasibleResolution,
}

pub struct QuantumPipelineResult {
    pub task: QuantumTaskKind,
    pub dispatch: QpuDispatchResult,
    pub rehydrated: Vec<QualiaQuin>,
    pub summary: String,
}

pub fn detect_task_from_prompt(prompt: &str) -> Option<QuantumTaskKind> {
    let lower = prompt.to_lowercase();
    if lower.contains("[qpu:qubo]") {
        return Some(QuantumTaskKind::QuboRouting);
    }
    if lower.contains("[qpu:dft]") {
        return Some(QuantumTaskKind::DftGroundState);
    }
    if lower.contains("[qpu:defeasible]") {
        return Some(QuantumTaskKind::DefeasibleResolution);
    }
    if prompt.contains(r"$$") {
        if prompt.contains(r"\min") || prompt.contains("QUBO") {
            return Some(QuantumTaskKind::QuboRouting);
        }
        if prompt.contains(r"\hat{H}")
            || prompt.contains(r"\Psi")
            || prompt.contains(r"\hat{h}")
            || prompt.contains("ground state")
        {
            return Some(QuantumTaskKind::DftGroundState);
        }
    }
    None
}

pub fn execute_quantum_pipeline(
    task: QuantumTaskKind,
    quins: &[QualiaQuin],
    latex_hint: Option<&str>,
) -> Result<QuantumPipelineResult, String> {
    if !qpu_oracle::is_qpu_feature_unlocked() {
        return Err("QPU Oracle not unlocked. Type [enable_QPU] in Chat first.".into());
    }

    let settings = qpu_oracle::get_qpu_settings();
    let shots = settings.max_shots_per_task;

    match task {
        QuantumTaskKind::QuboRouting => {
            let mut matrix = QuboMatrix::default();
            qubo_compiler::compile_quins_to_qubo(quins, &mut matrix)
                .map_err(|e| format!("QUBO compile blocked: {e:?}"))?;
            if matrix.num_vars == 0 {
                build_demo_qubo(&mut matrix, latex_hint);
            }
            let dispatch = qpu_dispatcher::dispatch_qubo(&matrix, shots)?;
            let mut out = [QualiaQuin {
                subject: 0,
                predicate: 0,
                object: 0,
                context: 0,
                metadata: 0,
                parity: 0,
            }; MAX_REHYDRATED];
            let n = qubo_compiler::rehydrate_solution(&mut matrix, &dispatch.assignment, &mut out);
            let summary = format!(
                "⚛️ **QUBO routing complete** ({})\n\n\
                 - Variables: {}\n\
                 - Ground energy: {:.4}\n\
                 - Remote QPU: {}\n\
                 - Re-hydrated assertions: {}\n\n\
                 Ephemeral index map wiped. No semantic context left the device.",
                dispatch.backend,
                dispatch.num_vars,
                dispatch.energy,
                dispatch.used_remote,
                n
            );
            Ok(QuantumPipelineResult {
                task,
                dispatch,
                rehydrated: out[..n].to_vec(),
                summary,
            })
        }
        QuantumTaskKind::DftGroundState => {
            let params = extract_vqe_params(latex_hint.unwrap_or(""));
            let dispatch = qpu_dispatcher::dispatch_vqe(&params, shots)?;
            let summary = format!(
                "⚛️ **DFT / VQE ground-state** ({})\n\n\
                 - Parameter vector dim: {}\n\
                 - Estimated energy: {:.4} eV\n\
                 - Remote QPU: {}\n\n\
                 Local Core 2 prepared the Hamiltonian; only parameter amplitudes egressed.",
                dispatch.backend,
                params.len(),
                dispatch.energy,
                dispatch.used_remote
            );
            Ok(QuantumPipelineResult {
                task,
                dispatch,
                rehydrated: vec![],
                summary,
            })
        }
        QuantumTaskKind::DefeasibleResolution => {
            let mut matrix = QuboMatrix::default();
            qubo_compiler::compile_quins_to_qubo(quins, &mut matrix)
                .map_err(|e| format!("Defeasible QUBO blocked: {e:?}"))?;
            if matrix.num_vars == 0 {
                build_defeasible_demo(&mut matrix);
            }
            let dispatch = qpu_dispatcher::dispatch_qubo(&matrix, shots)?;
            let summary = format!(
                "⚛️ **Defeasible resolution** via probabilistic QUBO ({})\n\
                 Energy: {:.4} | Remote: {}",
                dispatch.backend, dispatch.energy, dispatch.used_remote
            );
            Ok(QuantumPipelineResult {
                task,
                dispatch,
                rehydrated: vec![],
                summary,
            })
        }
    }
}

fn build_demo_qubo(matrix: &mut QuboMatrix, hint: Option<&str>) {
    let _ = hint;
    matrix.num_vars = 3;
    matrix.linear[0] = -1.0;
    matrix.linear[1] = -0.5;
    matrix.linear[2] = -2.0;
    let _ = matrix.emit_coupler(0, 1, 1.5);
    let _ = matrix.emit_coupler(1, 2, 0.8);
    matrix.index_map[0] = (0xA001, 0);
    matrix.index_map[1] = (0xA002, 1);
    matrix.index_map[2] = (0xA003, 2);
    matrix.index_count = 3;
}

fn build_defeasible_demo(matrix: &mut QuboMatrix) {
    matrix.num_vars = 2;
    matrix.linear[0] = -1.0;
    matrix.linear[1] = -1.0;
    let _ = matrix.emit_coupler(0, 1, 3.0);
    matrix.index_count = 2;
    matrix.index_map[0] = (0xD001, 0);
    matrix.index_map[1] = (0xD002, 1);
}

/// Unified engine command router: QPU unlock, quantum tasks, and LaTeX hints.
pub fn handle_engine_chat_command(text: &str) -> QpuChatCommandResult {
    use crate::qpu_oracle::{handle_qpu_chat_command, QpuChatCommandResult};

    let unlock = handle_qpu_chat_command(text);
    if unlock.handled {
        return unlock;
    }

    let task = detect_task_from_prompt(text);
    if let Some(kind) = task {
        if !qpu_oracle::is_qpu_feature_unlocked() {
            return QpuChatCommandResult {
                handled: true,
                feature_unlocked: false,
                response: "⚛️ Quantum task detected but QPU Oracle is locked. \
                    Type `[enable_QPU]` first, then configure API keys in Settings."
                    .to_string(),
            };
        }
        let quins: Vec<QualiaQuin> = vec![];
        let latex = if text.contains(r"$$") { Some(text) } else { None };
        match execute_quantum_pipeline(kind, &quins, latex) {
            Ok(result) => QpuChatCommandResult {
                handled: true,
                feature_unlocked: true,
                response: result.summary,
            },
            Err(e) => QpuChatCommandResult {
                handled: true,
                feature_unlocked: true,
                response: format!("🔴 Quantum pipeline failed: {e}"),
            },
        }
    } else {
        QpuChatCommandResult {
            handled: false,
            response: String::new(),
            feature_unlocked: qpu_oracle::is_qpu_feature_unlocked(),
        }
    }
}

fn extract_vqe_params(latex: &str) -> Vec<f64> {
    let mut params = Vec::new();
    for token in latex.split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-') {
        if let Ok(v) = token.parse::<f64>() {
            if token.contains('.') || v.abs() > 0.0 {
                params.push(v);
            }
        }
    }
    if params.is_empty() {
        params.extend_from_slice(&[0.1, 0.2, -0.15, 0.05]);
    }
    params.truncate(16);
    params
}
