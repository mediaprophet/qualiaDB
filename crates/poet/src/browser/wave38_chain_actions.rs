//! Dual-path leftovers: Medical / Manifold / crypto / gemm / Privacy / Sentinel / CapabilityDiscovery / agent.dag / FinancialModeling (wave 38).

use serde_json::json;
use web_sys::Document;

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: serde_json::Value,
) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(cap_id, &local_message);
        super::interactions::show_tool_status(document, &label, &report.message, report.status_kind);
        return;
    }
    super::interactions::show_tool_status(document, &label, &format!("Running {cap_id}…"), "running");
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_invoke(cap_id, args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(cap_id, &response.value);
                super::interactions::show_tool_status(&document, &label, &report.message, report.status_kind);
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response.diagnostic.as_deref().unwrap_or("capability invoke failed."),
                );
                super::interactions::show_tool_status(&document, &label, &report.message, report.status_kind);
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(cap_id, &error);
                super::interactions::show_tool_status(&document, &label, &report.message, report.status_kind);
            }
        }
    });
}

pub(super) fn run_tanimoto(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Medical.tanimoto",
        "Medical.tanimoto sketch two 4-bit fingerprints".to_string(),
        json!({ "a": [true, false, true, false], "b": [true, true, false, false] }),
    );
}

pub(super) fn run_structural_fingerprint(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Medical.structural_fingerprint",
        "Medical.structural_fingerprint sketch CCO".to_string(),
        json!({ "smiles": "CCO" }),
    );
}

pub(super) fn run_analyze_intensity_grid(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Medical.analyze_intensity_grid",
        "Medical.analyze_intensity_grid sketch 2x2".to_string(),
        json!({ "data": [0.0, 1.0, 1.0, 0.0], "width": 2, "height": 2, "bins": 4 }),
    );
}

pub(super) fn run_analyze_differential(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "MedicalComputing.analyze_differential",
        "MedicalComputing.analyze_differential sketch one finding".to_string(),
        json!({ "findings": ["fever"], "conditions": [{ "id": "c1", "prior": 0.5, "likelihoods": { "fever": 0.8 } }] }),
    );
}

pub(super) fn run_screen_compounds(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "MedicalComputing.screen_compounds",
        "MedicalComputing.screen_compounds sketch ethanol".to_string(),
        json!({ "compounds": [{ "id": "c1", "name": "ethanol", "smiles": "CCO", "molecular_weight": 46.0, "logp": -0.3, "solubility": 1.0, "acute_toxicity": 0.1, "chronic_toxicity": 0.1, "mutagenicity": false, "carcinogenicity": false }], "target": { "id": "t1", "name": "demo", "target_type": "enzyme", "biological_function": "demo", "disease_association": [] } }),
    );
}

pub(super) fn run_manifold_axes(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Manifold.axes",
        "Manifold.axes sketch taxonomy".to_string(),
        json!({}),
    );
}

pub(super) fn run_manifold_distance(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Manifold.distance",
        "Manifold.distance sketch two 10-D origins".to_string(),
        json!({ "a": [0.0,0.0,0.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0], "b": [1.0,0.0,0.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0] }),
    );
}

pub(super) fn run_manifold_project(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Manifold.project",
        "Manifold.project sketch spatial desk".to_string(),
        json!({ "x": 0.0, "y": 0.0, "z": 0.0, "t": 0.0, "level": 2, "entity": 1 }),
    );
}

pub(super) fn run_sha256(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "QuantumAndCryptographic.sha256",
        "QuantumAndCryptographic.sha256 sketch hello".to_string(),
        json!("hello"),
    );
}

pub(super) fn run_sha512(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "QuantumAndCryptographic.sha512",
        "QuantumAndCryptographic.sha512 sketch hello".to_string(),
        json!({ "text": "hello" }),
    );
}

pub(super) fn run_blake3(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "QuantumAndCryptographic.blake3",
        "QuantumAndCryptographic.blake3 sketch hello".to_string(),
        json!({ "text": "hello" }),
    );
}

pub(super) fn run_gaussian_sigma(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Privacy.gaussian_sigma",
        "Privacy.gaussian_sigma sketch eps=1 delta=1e-6".to_string(),
        json!({ "sensitivity": 1.0, "epsilon": 1.0, "delta": 1e-6 }),
    );
}

pub(super) fn run_sentinel_gate(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Sentinel.gate",
        "Sentinel.gate sketch action=read".to_string(),
        json!({ "action": "read", "is_sentinel": false }),
    );
}

pub(super) fn run_cap_catalog(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "CapabilityDiscovery.catalog",
        "CapabilityDiscovery.catalog sketch".to_string(),
        json!({}),
    );
}

pub(super) fn run_cap_coverage(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "CapabilityDiscovery.coverage",
        "CapabilityDiscovery.coverage sketch".to_string(),
        json!({}),
    );
}

pub(super) fn run_dag_execute(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "agent.dag.execute",
        "agent.dag.execute sketch one node".to_string(),
        json!({ "nodes": [{ "id": 1, "name": "n1", "effect": "pure" }] }),
    );
}

pub(super) fn run_dag_validate(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "agent.dag.validate",
        "agent.dag.validate sketch one node".to_string(),
        json!({ "nodes": [{ "id": 1, "name": "n1", "effect": "pure" }] }),
    );
}

pub(super) fn run_dag_status(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "agent.dag.status",
        "agent.dag.status sketch ready".to_string(),
        json!(null),
    );
}

pub(super) fn run_fm_black_scholes(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "FinancialModeling.black_scholes",
        "FinancialModeling.black_scholes sketch ATM call".to_string(),
        json!({ "kind": "call", "spot": 100.0, "strike": 100.0, "vol": 0.2, "time": 1.0, "rate": 0.01 }),
    );
}

pub(super) fn run_fm_portfolio_risk(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "FinancialModeling.portfolio_risk",
        "FinancialModeling.portfolio_risk sketch three prices".to_string(),
        json!({ "prices": [100.0, 101.0, 99.0] }),
    );
}
