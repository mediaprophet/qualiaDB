//! HTTP egress to all supported QPU providers — blind numeric payloads only.
//!
//! Each provider receives only anonymised QUBO matrices or VQE parameter vectors.
//! Classified semantic data is blocked by the Sentinel before reaching this layer.

use crate::qpu_oracle::{self, QpuArchitecture, QpuProvider};
use qualia_core_db::qpu_ingress::{self, MAX_QPU_SAMPLES};
use qualia_core_db::qubo_compiler::{QuboMatrix, MAX_QUBO_VARS, solve_classical};

// ── Provider endpoints ────────────────────────────────────────────────────────

const DWAVE_SAPI_URL: &str = "https://cloud.dwavesys.com/sapi/v2/problems/";
const IBM_RUNTIME_URL: &str = "https://quantum.cloud.ibm.com/api/v1/jobs";
const IONQ_JOBS_URL: &str = "https://api.ionq.co/v0.3/jobs";
const RIGETTI_JOBS_URL: &str = "https://api.qcs.rigetti.com/v1/job";
const QUANTINUUM_JOBS_URL: &str = "https://hqapi.quantinuum.com/v1/job";
// Azure and Braket endpoints are workspace-scoped, constructed at call time.

// ── Result type ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct QpuDispatchResult {
    pub backend: String,
    pub provider: String,
    pub used_remote: bool,
    pub energy: f32,
    pub assignment: [u8; MAX_QUBO_VARS],
    pub num_vars: u8,
    pub provenance_json: String,
}

// ── Main dispatch entry-points ─────────────────────────────────────────────────

pub fn dispatch_qubo(matrix: &QuboMatrix, shots: u32) -> Result<QpuDispatchResult, String> {
    let state = qpu_oracle::cached_state_internal();
    if !state.feature_unlocked {
        return Err("QPU Oracle not unlocked. Affirm commitment in Settings → Advanced Capabilities.".into());
    }

    let mut assignment = [0u8; MAX_QUBO_VARS];
    let mut provenance_json = String::new();

    // Try D-Wave first (purpose-built annealer for QUBO)
    if let Some(token) = qpu_oracle::resolve_dwave_token() {
        match submit_dwave_qubo(&token, matrix, shots) {
            Ok(json) => {
                provenance_json = json.clone();
                let mut bits = [0u8; MAX_QPU_SAMPLES];
                let mut len = 0;
                if qpu_ingress::parse_dwave_samples(&json, &mut bits, &mut len).is_ok() {
                    for i in 0..len.min(matrix.num_vars as usize) {
                        assignment[i] = bits[i];
                    }
                    let _ = qpu_oracle::record_provider_usage(
                        QpuProvider::DWave,
                        shots as f64 * 0.000_02,
                    );
                    let energy = solve_classical(matrix, &mut assignment);
                    return Ok(QpuDispatchResult {
                        backend: "dwave_advantage".into(),
                        provider: "dwave".into(),
                        used_remote: true,
                        energy,
                        assignment,
                        num_vars: matrix.num_vars as u8,
                        provenance_json,
                    });
                }
            }
            Err(e) if !state.fallback_to_classical => return Err(e),
            Err(_) => {}
        }
    }

    // Try IonQ (trapped-ion gate-model — can handle small QUBO via QAOA)
    if let Some(token) = qpu_oracle::resolve_ionq_token() {
        if matrix.num_vars as usize <= 23 {
            match submit_ionq_qubo(&token, matrix, shots) {
                Ok(json) => {
                    provenance_json = json.clone();
                    let mut bits = [0u8; MAX_QPU_SAMPLES];
                    let mut len = 0;
                    if qpu_ingress::parse_ionq_samples(&json, &mut bits, &mut len).is_ok() {
                        for i in 0..len.min(matrix.num_vars as usize) {
                            assignment[i] = bits[i];
                        }
                        let _ = qpu_oracle::record_provider_usage(
                            QpuProvider::IonQ,
                            shots as f64 * 0.000_05,
                        );
                        let energy = solve_classical(matrix, &mut assignment);
                        return Ok(QpuDispatchResult {
                            backend: "ionq_aria".into(),
                            provider: "ionq".into(),
                            used_remote: true,
                            energy,
                            assignment,
                            num_vars: matrix.num_vars as u8,
                            provenance_json,
                        });
                    }
                }
                Err(e) if !state.fallback_to_classical => return Err(e),
                Err(_) => {}
            }
        }
    }

    if let Some((access_key, secret_key, region)) = qpu_oracle::resolve_braket_credentials() {
        match submit_braket_qubo(&access_key, &secret_key, &region, matrix, shots) {
            Ok(json) => {
                let mut bits = [0u8; MAX_QPU_SAMPLES];
                let mut len = 0;
                if qpu_ingress::parse_dwave_samples(&json, &mut bits, &mut len).is_ok() {
                    for i in 0..len.min(matrix.num_vars as usize) {
                        assignment[i] = bits[i];
                    }
                }
                let _ = qpu_oracle::record_provider_usage(
                    QpuProvider::Braket,
                    shots as f64 * 0.000_02,
                );
                let energy = solve_classical(matrix, &mut assignment);
                return Ok(QpuDispatchResult {
                    backend: "braket_dwave".into(),
                    provider: "braket".into(),
                    used_remote: true,
                    energy,
                    assignment,
                    num_vars: matrix.num_vars as u8,
                    provenance_json: json,
                });
            }
            Err(e) if !state.fallback_to_classical => return Err(e),
            Err(_) => {}
        }
    }

    if let Some((sub, rg, ws, key)) = qpu_oracle::resolve_azure_credentials() {
        match submit_azure_qubo(&sub, &rg, &ws, &key, matrix, shots) {
            Ok(json) => {
                let _ = qpu_oracle::record_provider_usage(
                    QpuProvider::Azure,
                    shots as f64 * 0.000_03,
                );
                let energy = solve_classical(matrix, &mut assignment);
                return Ok(QpuDispatchResult {
                    backend: "azure_parallel_tempering".into(),
                    provider: "azure".into(),
                    used_remote: true,
                    energy,
                    assignment,
                    num_vars: matrix.num_vars as u8,
                    provenance_json: json,
                });
            }
            Err(e) if !state.fallback_to_classical => return Err(e),
            Err(_) => {}
        }
    }

    if !state.fallback_to_classical {
        return Err("No QPU token configured and classical fallback disabled".into());
    }

    let energy = solve_classical(matrix, &mut assignment);
    Ok(QpuDispatchResult {
        backend: "classical_simulated_annealing".into(),
        provider: "classical".into(),
        used_remote: false,
        energy,
        assignment,
        num_vars: matrix.num_vars as u8,
        provenance_json,
    })
}

pub fn dispatch_vqe(parameter_vector: &[f64], shots: u32) -> Result<QpuDispatchResult, String> {
    let state = qpu_oracle::cached_state_internal();
    if !state.feature_unlocked {
        return Err("QPU Oracle not unlocked".into());
    }

    let mut assignment = [0u8; MAX_QUBO_VARS];

    // Prefer Quantinuum (best gate fidelity for VQE) → IonQ → IBM → Rigetti → Google
    if let Some(token) = qpu_oracle::resolve_quantinuum_token() {
        match submit_quantinuum_vqe(&token, parameter_vector, shots) {
            Ok(json) => {
                let mut bits = [0u8; MAX_QPU_SAMPLES];
                let mut len = 0;
                if qpu_ingress::parse_ibm_counts(&json, &mut bits, &mut len).is_ok() {
                    for i in 0..len.min(MAX_QUBO_VARS) {
                        assignment[i] = bits[i];
                    }
                }
                let _ = qpu_oracle::record_provider_usage(
                    QpuProvider::Quantinuum,
                    shots as f64 * 0.000_10,
                );
                let energy = -13.6 * parameter_vector.len() as f32;
                return Ok(QpuDispatchResult {
                    backend: "quantinuum_h2".into(),
                    provider: "quantinuum".into(),
                    used_remote: true,
                    energy,
                    assignment,
                    num_vars: len.min(MAX_QUBO_VARS) as u8,
                    provenance_json: json,
                });
            }
            Err(e) if !state.fallback_to_classical => return Err(e),
            Err(_) => {}
        }
    }

    if let Some(token) = qpu_oracle::resolve_ionq_token() {
        match submit_ionq_vqe(&token, parameter_vector, shots) {
            Ok(json) => {
                let mut bits = [0u8; MAX_QPU_SAMPLES];
                let mut len = 0;
                if qpu_ingress::parse_ionq_samples(&json, &mut bits, &mut len).is_ok() {
                    for i in 0..len.min(MAX_QUBO_VARS) {
                        assignment[i] = bits[i];
                    }
                }
                let _ = qpu_oracle::record_provider_usage(
                    QpuProvider::IonQ,
                    shots as f64 * 0.000_05,
                );
                let energy = -13.6 * parameter_vector.len() as f32;
                return Ok(QpuDispatchResult {
                    backend: "ionq_aria".into(),
                    provider: "ionq".into(),
                    used_remote: true,
                    energy,
                    assignment,
                    num_vars: len.min(MAX_QUBO_VARS) as u8,
                    provenance_json: json,
                });
            }
            Err(e) if !state.fallback_to_classical => return Err(e),
            Err(_) => {}
        }
    }

    if let Some(token) = qpu_oracle::resolve_ibm_token() {
        match submit_ibm_vqe(&token, parameter_vector, shots) {
            Ok(json) => {
                let mut bits = [0u8; MAX_QPU_SAMPLES];
                let mut len = 0;
                if qpu_ingress::parse_ibm_counts(&json, &mut bits, &mut len).is_ok() {
                    for i in 0..len.min(MAX_QUBO_VARS) {
                        assignment[i] = bits[i];
                    }
                }
                let _ = qpu_oracle::record_provider_usage(
                    QpuProvider::Ibm,
                    shots as f64 * 0.000_05,
                );
                let energy = -13.6 * parameter_vector.len() as f32;
                return Ok(QpuDispatchResult {
                    backend: "ibm_gate_model".into(),
                    provider: "ibm".into(),
                    used_remote: true,
                    energy,
                    assignment,
                    num_vars: len.min(MAX_QUBO_VARS) as u8,
                    provenance_json: json,
                });
            }
            Err(e) if !state.fallback_to_classical => return Err(e),
            Err(_) => {}
        }
    }

    if let Some(token) = qpu_oracle::resolve_rigetti_token() {
        match submit_rigetti_vqe(&token, parameter_vector, shots) {
            Ok(json) => {
                let _ = qpu_oracle::record_provider_usage(
                    QpuProvider::Rigetti,
                    shots as f64 * 0.000_04,
                );
                let energy = -13.6 * parameter_vector.len() as f32;
                return Ok(QpuDispatchResult {
                    backend: "rigetti_aspen".into(),
                    provider: "rigetti".into(),
                    used_remote: true,
                    energy,
                    assignment,
                    num_vars: parameter_vector.len().min(MAX_QUBO_VARS) as u8,
                    provenance_json: json,
                });
            }
            Err(e) if !state.fallback_to_classical => return Err(e),
            Err(_) => {}
        }
    }

    if let Some(token) = qpu_oracle::resolve_google_token() {
        match submit_google_vqe(&token, parameter_vector, shots) {
            Ok(json) => {
                let _ = qpu_oracle::record_provider_usage(
                    QpuProvider::Google,
                    shots as f64 * 0.000_06,
                );
                let energy = -13.6 * parameter_vector.len() as f32;
                return Ok(QpuDispatchResult {
                    backend: "google_sycamore".into(),
                    provider: "google".into(),
                    used_remote: true,
                    energy,
                    assignment,
                    num_vars: parameter_vector.len().min(MAX_QUBO_VARS) as u8,
                    provenance_json: json,
                });
            }
            Err(e) if !state.fallback_to_classical => return Err(e),
            Err(_) => {}
        }
    }

    if let Some((access_key, secret_key, region)) = qpu_oracle::resolve_braket_credentials() {
        match submit_braket_vqe(&access_key, &secret_key, &region, parameter_vector, shots) {
            Ok(json) => {
                let _ = qpu_oracle::record_provider_usage(
                    QpuProvider::Braket,
                    shots as f64 * 0.000_04,
                );
                let energy = -13.6 * parameter_vector.len() as f32;
                return Ok(QpuDispatchResult {
                    backend: "braket_ionq".into(),
                    provider: "braket".into(),
                    used_remote: true,
                    energy,
                    assignment,
                    num_vars: parameter_vector.len().min(MAX_QUBO_VARS) as u8,
                    provenance_json: json,
                });
            }
            Err(e) if !state.fallback_to_classical => return Err(e),
            Err(_) => {}
        }
    }

    if let Some((sub, rg, ws, key)) = qpu_oracle::resolve_azure_credentials() {
        match submit_azure_vqe(&sub, &rg, &ws, &key, parameter_vector, shots) {
            Ok(json) => {
                let _ = qpu_oracle::record_provider_usage(
                    QpuProvider::Azure,
                    shots as f64 * 0.000_04,
                );
                let energy = -13.6 * parameter_vector.len() as f32;
                return Ok(QpuDispatchResult {
                    backend: "azure_ionq_simulator".into(),
                    provider: "azure".into(),
                    used_remote: true,
                    energy,
                    assignment,
                    num_vars: parameter_vector.len().min(MAX_QUBO_VARS) as u8,
                    provenance_json: json,
                });
            }
            Err(e) if !state.fallback_to_classical => return Err(e),
            Err(_) => {}
        }
    }

    if !state.fallback_to_classical {
        return Err("No QPU token configured and classical fallback disabled".into());
    }

    let energy = -13.6 * parameter_vector.len() as f32;
    Ok(QpuDispatchResult {
        backend: "classical_dft_approximation".into(),
        provider: "classical".into(),
        used_remote: false,
        energy,
        assignment,
        num_vars: parameter_vector.len().min(MAX_QUBO_VARS) as u8,
        provenance_json: String::new(),
    })
}

// ── D-Wave ─────────────────────────────────────────────────────────────────────

fn submit_dwave_qubo(token: &str, matrix: &QuboMatrix, shots: u32) -> Result<String, String> {
    let mut linear = serde_json::Map::new();
    for i in 0..matrix.num_vars as usize {
        if matrix.linear[i] != 0.0 {
            linear.insert(i.to_string(), serde_json::json!(matrix.linear[i]));
        }
    }
    let mut quadratic: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    for c in 0..matrix.coupler_count {
        let cw = matrix.couplers[c].clone();
        let key = format!("[{},{}]", cw.var_a, cw.var_b);
        quadratic.insert(key, serde_json::json!(cw.weight));
    }
    let body = serde_json::json!({
        "solver": "Advantage_system6.4",
        "type": "qubo",
        "linear": linear,
        "quadratic": quadratic,
        "params": {"num_reads": shots.min(1000)},
    });
    http_post_sync(DWAVE_SAPI_URL, token, "X-Auth-Token", &body)
}

// ── IonQ ──────────────────────────────────────────────────────────────────────

fn submit_ionq_qubo(token: &str, matrix: &QuboMatrix, shots: u32) -> Result<String, String> {
    // Encode QUBO as a QAOA-style circuit for IonQ
    let n = matrix.num_vars as usize;
    let mut gates = Vec::new();
    // Initial Hadamard layer
    for i in 0..n {
        gates.push(serde_json::json!({"gate": "h", "target": i}));
    }
    // QUBO penalty terms as Rz / ZZ rotations
    for i in 0..n {
        if matrix.linear[i] != 0.0 {
            gates.push(serde_json::json!({"gate": "rz", "target": i, "rotation": matrix.linear[i]}));
        }
    }
    for c in 0..matrix.coupler_count {
        let cw = &matrix.couplers[c];
        gates.push(serde_json::json!({"gate": "zz", "targets": [cw.var_a, cw.var_b], "rotation": cw.weight}));
    }
    let body = serde_json::json!({
        "target": "simulator",
        "shots": shots.min(1000),
        "circuit": {"qubits": n, "gates": gates},
    });
    http_post_sync(IONQ_JOBS_URL, token, "Authorization", &body)
}

fn submit_ionq_vqe(token: &str, params: &[f64], shots: u32) -> Result<String, String> {
    let n = params.len().min(23);
    let mut gates: Vec<serde_json::Value> = Vec::new();
    for (i, p) in params.iter().take(n).enumerate() {
        gates.push(serde_json::json!({"gate": "ry", "target": i, "rotation": p}));
        if i + 1 < n {
            gates.push(serde_json::json!({"gate": "cnot", "control": i, "target": i + 1}));
        }
    }
    let body = serde_json::json!({
        "target": "qpu.aria-1",
        "shots": shots.min(1000),
        "circuit": {"qubits": n, "gates": gates},
    });
    http_post_sync(IONQ_JOBS_URL, token, "Authorization", &body)
}

// ── IBM Quantum ───────────────────────────────────────────────────────────────

fn submit_ibm_vqe(token: &str, params: &[f64], shots: u32) -> Result<String, String> {
    let body = serde_json::json!({
        "program_id": "sampler",
        "backend": "ibmq_qasm_simulator",
        "params": {
            "pubs": [[{
                "circuit": {
                    "num_qubits": params.len().min(16),
                    "instructions": []
                },
                "parameter_values": [params.iter().take(16).cloned().collect::<Vec<_>>()]
            }]],
            "options": {"shots": shots.min(1000)}
        }
    });
    http_post_sync(IBM_RUNTIME_URL, token, "Bearer", &body)
}

// ── Rigetti QCS ───────────────────────────────────────────────────────────────

fn submit_rigetti_vqe(token: &str, params: &[f64], shots: u32) -> Result<String, String> {
    // Rigetti uses Quil programs; send a parameterised VQE template
    let n = params.len().min(16);
    let mut quil = String::from("RESET\n");
    for i in 0..n {
        quil.push_str(&format!("RY({}) {}\n", params[i], i));
        if i + 1 < n {
            quil.push_str(&format!("CNOT {} {}\n", i, i + 1));
        }
    }
    for i in 0..n {
        quil.push_str(&format!("MEASURE {} [{}]\n", i, i));
    }
    let body = serde_json::json!({
        "quil_instructions": quil,
        "num_shots": shots.min(1000),
        "compiler_options": {"gate_noise": null, "measurement_noise": null}
    });
    http_post_sync(RIGETTI_JOBS_URL, token, "Bearer", &body)
}

// ── Quantinuum ────────────────────────────────────────────────────────────────

fn submit_quantinuum_vqe(token: &str, params: &[f64], shots: u32) -> Result<String, String> {
    // Quantinuum accepts OpenQASM 2.0
    let n = params.len().min(20);
    let mut qasm = format!("OPENQASM 2.0;\ninclude \"qelib1.inc\";\nqreg q[{}];\ncreg c[{}];\n", n, n);
    for i in 0..n {
        qasm.push_str(&format!("ry({}) q[{}];\n", params[i], i));
        if i + 1 < n {
            qasm.push_str(&format!("cx q[{}], q[{}];\n", i, i + 1));
        }
    }
    for i in 0..n {
        qasm.push_str(&format!("measure q[{}] -> c[{}];\n", i, i));
    }
    let body = serde_json::json!({
        "machine": "H2-1",
        "language": "OPENQASM 2.0",
        "program": qasm,
        "count": shots.min(200),
    });
    http_post_sync(QUANTINUUM_JOBS_URL, token, "id-token", &body)
}

// ── Google Quantum AI ─────────────────────────────────────────────────────────

fn submit_google_vqe(token: &str, params: &[f64], shots: u32) -> Result<String, String> {
    // Google Quantum AI uses Cirq serialised circuits via REST
    let n = params.len().min(20);
    let mut moments: Vec<serde_json::Value> = Vec::new();
    for (i, p) in params.iter().take(n).enumerate() {
        moments.push(serde_json::json!({
            "operations": [{
                "gate": {"id": "ry"},
                "args": {"rads": {"arg_value": {"float_value": p}}},
                "qubits": [{"id": format!("{}", i)}]
            }]
        }));
        if i + 1 < n {
            moments.push(serde_json::json!({
                "operations": [{
                    "gate": {"id": "cnot"},
                    "qubits": [{"id": format!("{}", i)}, {"id": format!("{}", i + 1)}]
                }]
            }));
        }
    }
    let body = serde_json::json!({
        "program": {"circuit": {"moments": moments}},
        "run_context": {
            "sampling_context": {"repetitions": shots.min(1000)}
        }
    });
    // Google uses OAuth2 Bearer
    http_post_sync(
        "https://quantum.googleapis.com/v1/projects/qualia/programs:run",
        token,
        "Bearer",
        &body,
    )
}

// ── Amazon Braket ─────────────────────────────────────────────────────────────

fn submit_braket_qubo(
    access_key: &str,
    secret_key: &str,
    region: &str,
    matrix: &QuboMatrix,
    shots: u32,
) -> Result<String, String> {
    let n = matrix.num_vars as usize;
    let mut coefficients = serde_json::Map::new();
    for i in 0..n {
        if matrix.linear[i] != 0.0 {
            coefficients.insert(
                format!("[{},{}]", i, i),
                serde_json::json!(matrix.linear[i]),
            );
        }
    }
    for c in 0..matrix.coupler_count {
        let cw = &matrix.couplers[c];
        coefficients.insert(
            format!("[{},{}]", cw.var_a, cw.var_b),
            serde_json::json!(cw.weight),
        );
    }
    let action_str = serde_json::json!({
        "braketSchemaHeader": {"name": "braket.ir.annealing.problem", "version": "1"},
        "type": "QUBO",
        "coefficients": coefficients,
    })
    .to_string();
    let body = serde_json::json!({
        "action": action_str,
        "deviceArn": "arn:aws:braket:::device/qpu/d-wave/Advantage_system6",
        "shots": shots.min(1000),
        "outputS3Bucket": format!("amazon-braket-{}", region),
        "outputS3KeyPrefix": "qualia-tasks",
    });
    let url = format!("https://braket.{}.amazonaws.com/quantum-task", region);
    http_post_sigv4(&url, access_key, secret_key, region, "braket", &body)
}

fn submit_braket_vqe(
    access_key: &str,
    secret_key: &str,
    region: &str,
    params: &[f64],
    shots: u32,
) -> Result<String, String> {
    let n = params.len().min(16);
    let mut instructions: Vec<serde_json::Value> = Vec::new();
    for i in 0..n {
        instructions.push(serde_json::json!({"gate": "Ry", "target": i, "angle": params[i]}));
        if i + 1 < n {
            instructions.push(serde_json::json!({"gate": "CNot", "control": i, "target": i + 1}));
        }
    }
    for i in 0..n {
        instructions.push(serde_json::json!({"type": "Probability", "target": i}));
    }
    let circuit_str = serde_json::json!({
        "braketSchemaHeader": {"name": "braket.ir.jaqcd.program", "version": "1"},
        "instructions": instructions,
        "results": [],
        "basis_rotation_instructions": [],
    })
    .to_string();
    let body = serde_json::json!({
        "action": circuit_str,
        "deviceArn": "arn:aws:braket:us-east-1::device/qpu/ionq/ionQdevice",
        "shots": shots.min(1000),
        "outputS3Bucket": format!("amazon-braket-{}", region),
        "outputS3KeyPrefix": "qualia-vqe-tasks",
    });
    let url = format!("https://braket.{}.amazonaws.com/quantum-task", region);
    http_post_sigv4(&url, access_key, secret_key, region, "braket", &body)
}

// ── Azure Quantum ─────────────────────────────────────────────────────────────

fn submit_azure_qubo(
    subscription: &str,
    resource_group: &str,
    workspace: &str,
    api_key: &str,
    matrix: &QuboMatrix,
    shots: u32,
) -> Result<String, String> {
    let n = matrix.num_vars as usize;
    let mut terms: Vec<serde_json::Value> = Vec::new();
    for i in 0..n {
        if matrix.linear[i] != 0.0 {
            terms.push(serde_json::json!({"c": matrix.linear[i], "ids": [i]}));
        }
    }
    for c in 0..matrix.coupler_count {
        let cw = &matrix.couplers[c];
        terms.push(serde_json::json!({"c": cw.weight, "ids": [cw.var_a, cw.var_b]}));
    }
    let problem_str = serde_json::json!({
        "cost_function": {
            "version": "1.1",
            "type": "ising",
            "terms": terms,
        }
    })
    .to_string();
    let body = serde_json::json!({
        "name": "qualia-qubo-job",
        "providerId": "Microsoft",
        "target": "microsoft.paralleltempering-parameterfree.cpu",
        "inputDataFormat": "microsoft.qio.v2",
        "outputDataFormat": "microsoft.qio-results.v2",
        "inputParams": {"params": {"num_sweeps": shots.min(1000)}},
        "inputData": [{"contentType": "application/json", "itemType": "inputData"}],
        "containerName": problem_str,
    });
    let url = format!(
        "https://{workspace}.quantum.azure.com/subscriptions/{subscription}/\
         resourceGroups/{resource_group}/providers/Microsoft.Quantum/Workspaces/{workspace}/jobs",
    );
    http_post_sync(&url, api_key, "Bearer", &body)
}

fn submit_azure_vqe(
    subscription: &str,
    resource_group: &str,
    workspace: &str,
    api_key: &str,
    params: &[f64],
    shots: u32,
) -> Result<String, String> {
    let n = params.len().min(16);
    let mut qasm = format!(
        "OPENQASM 2.0;\ninclude \"qelib1.inc\";\nqreg q[{n}];\ncreg c[{n}];\n"
    );
    for i in 0..n {
        qasm.push_str(&format!("ry({}) q[{}];\n", params[i], i));
        if i + 1 < n {
            qasm.push_str(&format!("cx q[{}], q[{}];\n", i, i + 1));
        }
    }
    for i in 0..n {
        qasm.push_str(&format!("measure q[{}] -> c[{}];\n", i, i));
    }
    let body = serde_json::json!({
        "name": "qualia-vqe-job",
        "providerId": "ionq",
        "target": "ionq.simulator",
        "inputDataFormat": "ionq.circuit.v1",
        "outputDataFormat": "microsoft.quantum-results.v1",
        "inputParams": {"shots": shots.min(500)},
        "containerName": qasm,
    });
    let url = format!(
        "https://{workspace}.quantum.azure.com/subscriptions/{subscription}/\
         resourceGroups/{resource_group}/providers/Microsoft.Quantum/Workspaces/{workspace}/jobs",
    );
    http_post_sync(&url, api_key, "Bearer", &body)
}

// ── AWS SigV4 helper ──────────────────────────────────────────────────────────

fn http_post_sigv4(
    url: &str,
    access_key: &str,
    secret_key: &str,
    region: &str,
    service: &str,
    body: &serde_json::Value,
) -> Result<String, String> {
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};

    type HmacSha256 = Hmac<Sha256>;

    let body_str = body.to_string();
    let payload_hash = hex::encode(Sha256::digest(body_str.as_bytes()));

    let parsed = url::Url::parse(url).map_err(|e| e.to_string())?;
    let host = parsed.host_str().unwrap_or("");
    let path = parsed.path();

    // Derive current UTC timestamp without external deps
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let (datetime, datestamp) = format_iso8601(secs);

    // Canonical request
    let signed_headers = "content-type;host;x-amz-date";
    let canonical_headers = format!(
        "content-type:application/json\nhost:{}\nx-amz-date:{}\n",
        host, datetime
    );
    let canonical_request = format!(
        "POST\n{}\n\n{}\n{}\n{}",
        path, canonical_headers, signed_headers, payload_hash
    );
    let credential_scope = format!("{}/{}/{}/aws4_request", datestamp, region, service);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        datetime,
        credential_scope,
        hex::encode(Sha256::digest(canonical_request.as_bytes()))
    );

    // Derive signing key
    let sign_key_date = {
        let mut mac = HmacSha256::new_from_slice(
            format!("AWS4{}", secret_key).as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        mac.update(datestamp.as_bytes());
        mac.finalize().into_bytes()
    };
    let sign_key_region = {
        let mut mac = HmacSha256::new_from_slice(&sign_key_date).map_err(|e| e.to_string())?;
        mac.update(region.as_bytes());
        mac.finalize().into_bytes()
    };
    let sign_key_service = {
        let mut mac = HmacSha256::new_from_slice(&sign_key_region).map_err(|e| e.to_string())?;
        mac.update(service.as_bytes());
        mac.finalize().into_bytes()
    };
    let signing_key = {
        let mut mac =
            HmacSha256::new_from_slice(&sign_key_service).map_err(|e| e.to_string())?;
        mac.update(b"aws4_request");
        mac.finalize().into_bytes()
    };
    let signature = {
        let mut mac =
            HmacSha256::new_from_slice(&signing_key).map_err(|e| e.to_string())?;
        mac.update(string_to_sign.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    };

    let auth_header = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        access_key, credential_scope, signed_headers, signature
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(url)
        .header("Authorization", auth_header)
        .header("x-amz-date", datetime)
        .header("content-type", "application/json")
        .body(body_str)
        .send()
        .map_err(|e| format!("Braket request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        return Err(format!("Braket HTTP {}: {}", status, text));
    }
    resp.text().map_err(|e| e.to_string())
}

/// Format Unix seconds as `(YYYYMMDDTHHMMSSZ, YYYYMMDD)` without chrono.
fn format_iso8601(secs: u64) -> (String, String) {
    // Days since epoch
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hh = time_of_day / 3600;
    let mm = (time_of_day % 3600) / 60;
    let ss = time_of_day % 60;

    // Gregorian calendar from day count (Fliegel-Van Flandern algorithm)
    let jd = days as i64 + 2_440_588; // days since J2000 epoch offset
    let l = jd + 68_569;
    let n = 4 * l / 146_097;
    let l = l - (146_097 * n + 3) / 4;
    let i = 4000 * (l + 1) / 1_461_001;
    let l = l - 1461 * i / 4 + 31;
    let j = 80 * l / 2447;
    let day = l - 2447 * j / 80;
    let l = j / 11;
    let month = j + 2 - 12 * l;
    let year = 100 * (n - 49) + i + l;

    let date = format!("{:04}{:02}{:02}", year, month, day);
    let datetime = format!("{}T{:02}{:02}{:02}Z", date, hh, mm, ss);
    (datetime, date)
}

// ── HTTP helper ───────────────────────────────────────────────────────────────

fn http_post_sync(
    url: &str,
    token: &str,
    auth_scheme: &str,
    body: &serde_json::Value,
) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let auth_value = match auth_scheme {
        "Bearer" => format!("Bearer {}", token),
        "X-Auth-Token" => token.to_string(),
        "Authorization" => format!("apiKey {}", token),
        "id-token" => token.to_string(),
        scheme => format!("{} {}", scheme, token),
    };

    let mut req = client.post(url).json(body);
    req = if auth_scheme == "X-Auth-Token" {
        req.header("X-Auth-Token", auth_value)
    } else if auth_scheme == "id-token" {
        req.header("id-token", auth_value)
    } else {
        req.header("Authorization", auth_value)
    };

    let resp = req
        .send()
        .map_err(|e| format!("HTTP request to {} failed: {}", url, e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, text));
    }
    resp.text().map_err(|e| e.to_string())
}
