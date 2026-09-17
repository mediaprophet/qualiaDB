//! Dual-path Tool Chest actions for Graph / Optimization / sampler / Capability leftovers (wave 37).

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
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("capability invoke failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(cap_id, &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

fn tiny_fuzzy() -> serde_json::Value {
    json!([{ "s": 0, "p": 1, "o": 0, "degree": 1.0 }])
}

pub(super) fn run_fuzzy_jaccard(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "GraphMatch.fuzzy_jaccard",
        "GraphMatch.fuzzy_jaccard sketch one triple each".to_string(),
        json!({ "g1": tiny_fuzzy(), "g2": tiny_fuzzy() }),
    );
}

pub(super) fn run_fuzzy_dice(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "GraphMatch.fuzzy_dice",
        "GraphMatch.fuzzy_dice sketch one triple each".to_string(),
        json!({ "g1": tiny_fuzzy(), "g2": tiny_fuzzy() }),
    );
}

pub(super) fn run_approximate_match(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "GraphMatch.approximate_match",
        "GraphMatch.approximate_match sketch 1-node graphs".to_string(),
        json!({
            "pattern": tiny_fuzzy(),
            "data": tiny_fuzzy(),
            "n_pattern_nodes": 1,
            "n_data_nodes": 1,
            "restarts": 2,
            "seed": 1
        }),
    );
}

pub(super) fn run_shortest_path(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "GraphReasoning.shortest_path",
        "GraphReasoning.shortest_path sketch 0→1 weight 1".to_string(),
        json!({ "edges": [[0, 1, 1.0]], "source": 0, "target": 1, "n": 2 }),
    );
}

pub(super) fn run_spreading_activation(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "GraphReasoning.spreading_activation",
        "GraphReasoning.spreading_activation sketch seed 0".to_string(),
        json!({
            "edges": [[0, 1, 1.0]],
            "seeds": [[0, 1.0]],
            "decay": 0.5,
            "n": 2
        }),
    );
}

pub(super) fn run_top_k(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "GraphReasoning.top_k",
        "GraphReasoning.top_k sketch activation=[0.1,0.9,0.3] k=1".to_string(),
        json!({ "activation": [0.1, 0.9, 0.3], "k": 1 }),
    );
}

pub(super) fn run_hill_climb(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Optimization.hill_climb",
        "Optimization.hill_climb sketch target=2".to_string(),
        json!({ "start": 0.0, "target": 2.0, "step": 0.5, "max_iter": 16 }),
    );
}

pub(super) fn run_simulated_annealing(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Optimization.simulated_annealing",
        "Optimization.simulated_annealing sketch (x-1)^2 on [0,2]".to_string(),
        json!({
            "objective": "(x - 1.0) * (x - 1.0)",
            "vars": ["x"],
            "bounds": [[0.0, 2.0]],
            "max_iter": 8
        }),
    );
}

pub(super) fn run_artificial_bee_colony(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Optimization.artificial_bee_colony",
        "Optimization.artificial_bee_colony sketch (x-1)^2 on [0,2]".to_string(),
        json!({
            "objective": "(x - 1.0) * (x - 1.0)",
            "vars": ["x"],
            "bounds": [[0.0, 2.0]]
        }),
    );
}

pub(super) fn run_sampler_configure(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "sampler.configure",
        "sampler.configure sketch temperature=0 top_k=1".to_string(),
        json!({ "temperature": 0.0, "top_k": 1 }),
    );
}

pub(super) fn run_sampler_constrain_enable(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "sampler.constrain_enable",
        "sampler.constrain_enable sketch vocab [[1,\"hi\"]]".to_string(),
        json!([[1, "hi"]]),
    );
}

pub(super) fn run_sampler_constrain_disable(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "sampler.constrain_disable",
        "sampler.constrain_disable sketch".to_string(),
        json!(null),
    );
}

pub(super) fn run_sampler_constrain_reset(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "sampler.constrain_reset",
        "sampler.constrain_reset sketch".to_string(),
        json!(null),
    );
}

pub(super) fn run_sampler_sample(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "sampler.sample",
        "sampler.sample sketch logits=[0.1,0.9,0.3]".to_string(),
        json!({ "logits": [0.1, 0.9, 0.3] }),
    );
}

pub(super) fn run_cap_grant(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Capability.grant",
        "Capability.grant sketch agent=1 root=1 delegated".to_string(),
        json!({
            "agent_did_hash": 1,
            "root_did_hash": 1,
            "delegated": true,
            "current_epoch": 1
        }),
    );
}

pub(super) fn run_cap_revoke(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Capability.revoke",
        "Capability.revoke sketch faults=0".to_string(),
        json!({ "windowed_faults": 0, "usury_event": false }),
    );
}

pub(super) fn run_cap_test_gating(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Capability.test_gating",
        "Capability.test_gating sketch is_sentinel=false".to_string(),
        json!({ "is_sentinel": false }),
    );
}

pub(super) fn run_cap_audit(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Capability.audit",
        "Capability.audit sketch".to_string(),
        json!({}),
    );
}

pub(super) fn run_cap_declare(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Capability.declare",
        "Capability.declare sketch scope=capability.invoke".to_string(),
        json!({ "scope": "capability.invoke", "reason": "poet-demo" }),
    );
}
