//! Dual-path leftovers: remaining curated Host singles (wave 39). Exhausts helper-aware Q2.

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

pub(super) fn run_agency_evaluate(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Agency.evaluate",
        "Agency.evaluate sketch needs 32-byte key + 64-byte sig".to_string(),
        json!({ "frame": [1, 2, 3], "author_did": 1, "verifying_key": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], "signature": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0] }),
    );
}

pub(super) fn run_bio_align(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Bioinformatics.align",
        "Bioinformatics.align sketch ACGT vs AGGT".to_string(),
        json!({ "query": "ACGT", "target": "AGGT" }),
    );
}

pub(super) fn run_dmp_holds(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Calculus.discrete_maximum_principle_holds",
        "Calculus.discrete_maximum_principle_holds sketch 1x1".to_string(),
        json!({ "width": 1, "height": 1, "spacing": 1.0, "source": [0.0], "boundary": [0.0], "solution": [0.0] }),
    );
}

pub(super) fn run_poisson(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Calculus.solve_poisson_dirichlet",
        "Calculus.solve_poisson_dirichlet sketch 1x1".to_string(),
        json!({ "width": 1, "height": 1, "spacing": 1.0, "source": [0.0], "boundary": [0.0] }),
    );
}

pub(super) fn run_t_norm(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "CausalFuzzyAndControl.t_norm",
        "CausalFuzzyAndControl.t_norm sketch [0.4, 0.7]".to_string(),
        json!([0.4, 0.7]),
    );
}

pub(super) fn run_parse_bse(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Chemistry.parse_bse_json",
        "Chemistry.parse_bse_json sketch empty json".to_string(),
        json!({ "json": "{}", "atoms": [{ "z": 1, "symbol": "H", "x": 0.0, "y": 0.0, "z": 0.0 }] }),
    );
}

pub(super) fn run_parse_did(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "ContractsIdentityAndConsensus.parse_did_q42",
        "ContractsIdentityAndConsensus.parse_did_q42 sketch did:q42:demo".to_string(),
        json!("did:q42:demo"),
    );
}

pub(super) fn run_board_project(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "CooperativeWork.board_project",
        "CooperativeWork.board_project sketch one task".to_string(),
        json!({ "items": [{ "id": "wi-1", "project_id": "proj", "item_type": "Task", "title": "demo", "description": "", "priority": "Normal", "created_at_unix": 10 }], "events": [] }),
    );
}

pub(super) fn run_econ_aggregate_wealth(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Econ.aggregate_wealth",
        "Econ.aggregate_wealth sketch availability".to_string(),
        json!({}),
    );
}

pub(super) fn run_econ_cumulative_wealth(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Econ.cumulative_wealth",
        "Econ.cumulative_wealth sketch returns".to_string(),
        json!({ "returns": [0.1, -0.05, 0.02] }),
    );
}

pub(super) fn run_econ_narrative_divergence(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Econ.narrative_divergence",
        "Econ.narrative_divergence sketch availability".to_string(),
        json!({}),
    );
}

pub(super) fn run_analyze_conduction(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "EngineeringAnalysis.analyze_conduction",
        "EngineeringAnalysis.analyze_conduction sketch 1-D bar".to_string(),
        json!({ "length": 1.0, "thermal_conductivity": 1.0, "left_bc": { "type": "Temperature", "value": 300.0 }, "right_bc": { "type": "Temperature", "value": 300.0 } }),
    );
}

pub(super) fn run_fem_static(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "EngineeringAnalysis.fem_static",
        "EngineeringAnalysis.fem_static sketch two-node truss".to_string(),
        json!({ "nodes": [[0.0, 0.0], [1.0, 0.0]], "elements": [{ "type": "Truss", "ni": 0, "nj": 1, "e": 1.0, "area": 1.0 }], "constraints": [[0, 0.0]], "loads": [[1, 1.0]] }),
    );
}

pub(super) fn run_peer_hash(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Net.peer_hash",
        "Net.peer_hash sketch did:q42:demo".to_string(),
        json!({ "did": "did:q42:demo" }),
    );
}

pub(super) fn run_sonic_pack(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Net.sonic_pack",
        "Net.sonic_pack sketch note 60".to_string(),
        json!({ "note": 60, "velocity": 80, "channel": 0, "event": "on" }),
    );
}

pub(super) fn run_simpson(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "NumericalCalculus.simpson",
        "NumericalCalculus.simpson sketch x^2 on [0,1]".to_string(),
        json!({ "a": 0.0, "b": 1.0, "power": 2.0 }),
    );
}

pub(super) fn run_ontology_align(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "OntologyAlignment.align",
        "OntologyAlignment.align sketch 1x1 sim".to_string(),
        json!({ "sim": [1.0], "n_source": 1, "n_target": 1, "threshold": 0.5 }),
    );
}

pub(super) fn run_units_convert(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "PhysicalUnits.convert",
        "PhysicalUnits.convert sketch 1000 mm to m".to_string(),
        json!({ "value": 1000.0, "from": "mm", "to": "m" }),
    );
}

pub(super) fn run_projectile(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "PhysicsAndODE.projectile",
        "PhysicsAndODE.projectile sketch 10 m/s 45deg".to_string(),
        json!({ "v0": 10.0, "angle_rad": 0.78539816339, "samples": 8 }),
    );
}

pub(super) fn run_poly_coeffs(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.coeffs",
        "PolynomialAlgebra.coeffs sketch 1+x".to_string(),
        json!({ "a": [1.0, 1.0] }),
    );
}

pub(super) fn run_bessel_j(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "SpecialFunctionsAndTransforms.bessel_j",
        "SpecialFunctionsAndTransforms.bessel_j sketch J0(1)".to_string(),
        json!({ "n": 0, "x": 1.0 }),
    );
}

pub(super) fn run_ltl_finally(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "TemporalAndDescriptionLogic.ltl.finally",
        "TemporalAndDescriptionLogic.ltl.finally sketch predicate p".to_string(),
        json!({ "predicate": "p" }),
    );
}

pub(super) fn run_ltl_globally(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "TemporalAndDescriptionLogic.ltl.globally",
        "TemporalAndDescriptionLogic.ltl.globally sketch predicate p".to_string(),
        json!({ "predicate": "p" }),
    );
}

pub(super) fn run_hash_iri(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "hash.iri",
        "hash.iri sketch http://example.org/a".to_string(),
        json!("http://example.org/a"),
    );
}
