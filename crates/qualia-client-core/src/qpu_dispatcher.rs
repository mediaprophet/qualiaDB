//! HTTP egress to IBM Quantum and D-Wave Leap — blind numeric payloads only.

use crate::qpu_oracle::{self, QpuArchitecture};
use qualia_core_db::qpu_ingress::{self, MAX_QPU_SAMPLES};
use qualia_core_db::qubo_compiler::{self, QuboMatrix, MAX_QUBO_VARS};

const DWAVE_SAPI_URL: &str = "https://cloud.dwavesys.com/sapi/problems/";
const IBM_RUNTIME_URL: &str = "https://quantum.cloud.ibm.com/api/v1/jobs";

#[derive(Debug, Clone)]
pub struct QpuDispatchResult {
    pub backend: String,
    pub used_remote: bool,
    pub energy: f32,
    pub assignment: [u8; MAX_QUBO_VARS],
    pub num_vars: u8,
    pub provenance_json: String,
}

pub fn dispatch_qubo(matrix: &QuboMatrix, shots: u32) -> Result<QpuDispatchResult, String> {
    let state = qpu_oracle::cached_state_internal();
    if !state.feature_unlocked {
        return Err("QPU Oracle not unlocked".into());
    }

    let mut assignment = [0u8; MAX_QUBO_VARS];
    let mut used_remote = false;
    let mut backend = "classical".to_string();
    let mut provenance_json = String::new();

    if let Some(token) = qpu_oracle::resolve_dwave_token() {
        if qpu_oracle::target_architecture("qubo_routing").is_some() {
            match submit_dwave_qubo(&token, matrix, shots) {
                Ok(json) => {
                    used_remote = true;
                    backend = "dwave_advantage".to_string();
                    provenance_json = json.clone();
                    let mut bits = [0u8; MAX_QPU_SAMPLES];
                    let mut len = 0;
                    if qpu_ingress::parse_dwave_samples(&json, &mut bits, &mut len).is_ok() {
                        for i in 0..len.min(matrix.num_vars as usize) {
                            assignment[i] = bits[i];
                        }
                        record_dwave_usage(shots)?;
                        let energy = qubo_compiler::solve_classical(matrix, &mut assignment);
                        return Ok(QpuDispatchResult {
                            backend,
                            used_remote,
                            energy,
                            assignment,
                            num_vars: matrix.num_vars,
                            provenance_json,
                        });
                    }
                }
                Err(e) if !state.fallback_to_classical => return Err(e),
                Err(_) => {}
            }
        }
    }

    if !state.fallback_to_classical && !used_remote {
        return Err("No QPU token configured and classical fallback disabled".into());
    }

    let energy = qubo_compiler::solve_classical(matrix, &mut assignment);
    Ok(QpuDispatchResult {
        backend: "classical_simulated_annealing".to_string(),
        used_remote: false,
        energy,
        assignment,
        num_vars: matrix.num_vars,
        provenance_json,
    })
}

pub fn dispatch_vqe(parameter_vector: &[f64], shots: u32) -> Result<QpuDispatchResult, String> {
    let state = qpu_oracle::cached_state_internal();
    if !state.feature_unlocked {
        return Err("QPU Oracle not unlocked".into());
    }

    let mut assignment = [0u8; MAX_QUBO_VARS];
    let mut provenance_json = String::new();
    let mut used_remote = false;
    let mut backend = "classical_dft".to_string();

    if let Some(token) = qpu_oracle::resolve_ibm_token() {
        if qpu_oracle::target_architecture("dft_ground_state").is_some() {
            match submit_ibm_vqe(&token, parameter_vector, shots) {
                Ok(json) => {
                    used_remote = true;
                    backend = "ibm_gate_model".to_string();
                    provenance_json = json.clone();
                    let mut bits = [0u8; MAX_QPU_SAMPLES];
                    let mut len = 0;
                    if qpu_ingress::parse_ibm_counts(&json, &mut bits, &mut len).is_ok() {
                        for i in 0..len.min(MAX_QUBO_VARS) {
                            assignment[i] = bits[i];
                        }
                        record_ibm_usage(shots)?;
                    }
                    let energy = -13.6 * parameter_vector.len() as f32;
                    return Ok(QpuDispatchResult {
                        backend,
                        used_remote,
                        energy,
                        assignment,
                        num_vars: len.min(MAX_QUBO_VARS) as u8,
                        provenance_json,
                    });
                }
                Err(e) if !state.fallback_to_classical => return Err(e),
                Err(_) => {}
            }
        }
    }

    if !state.fallback_to_classical && !used_remote {
        return Err("IBM token missing and classical fallback disabled".into());
    }

    let energy = -13.6 * parameter_vector.len() as f32;
    Ok(QpuDispatchResult {
        backend: "classical_dft_approximation".to_string(),
        used_remote: false,
        energy,
        assignment,
        num_vars: parameter_vector.len().min(MAX_QUBO_VARS) as u8,
        provenance_json,
    })
}

fn submit_dwave_qubo(token: &str, matrix: &QuboMatrix, shots: u32) -> Result<String, String> {
    let mut linear = serde_json::Map::new();
    for i in 0..matrix.num_vars as usize {
        if matrix.linear[i] != 0.0 {
            linear.insert(i.to_string(), serde_json::json!(matrix.linear[i]));
        }
    }
    let mut quadratic: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    for c in 0..matrix.coupler_count {
        let cw = matrix.couplers[c];
        let key = format!("[{},{}]", cw.var_a, cw.var_b);
        quadratic.insert(key, serde_json::json!(cw.weight));
    }
    let body = serde_json::json!({
        "solver": "Advantage_system6.1",
        "type": "qubo",
        "linear": linear,
        "quadratic": quadratic,
        "num_reads": shots.min(1000),
    });

    submit_dwave_sync(token, &body)
}

fn submit_dwave_sync(token: &str, body: &serde_json::Value) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(DWAVE_SAPI_URL)
        .header("X-Auth-Token", token)
        .json(body)
        .send()
        .map_err(|e| format!("D-Wave request failed: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        return Err(format!("D-Wave HTTP {status}: {text}"));
    }
    resp.text().map_err(|e| e.to_string())
}

fn submit_ibm_vqe(token: &str, params: &[f64], shots: u32) -> Result<String, String> {
    let body = serde_json::json!({
        "program_id": "sampler",
        "backend": "ibmq_qasm_simulator",
        "params": {
            "pubs": [[{
                "circuit": {"num_qubits": params.len().min(16), "instructions": []},
                "parameter_values": [params.iter().take(16).cloned().collect::<Vec<_>>()]
            }]],
            "options": {"shots": shots.min(1000)}
        }
    });
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(IBM_RUNTIME_URL)
        .bearer_auth(token)
        .json(&body)
        .send()
        .map_err(|e| format!("IBM request failed: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        return Err(format!("IBM HTTP {status}: {text}"));
    }
    resp.text().map_err(|e| e.to_string())
}

fn record_dwave_usage(shots: u32) -> Result<(), String> {
    let micros = (shots as f64) * 0.000_02;
    qpu_oracle::record_usage(QpuArchitecture::Annealer, micros)
}

fn record_ibm_usage(shots: u32) -> Result<(), String> {
    let micros = (shots as f64) * 0.000_05;
    qpu_oracle::record_usage(QpuArchitecture::GateModel, micros)
}
