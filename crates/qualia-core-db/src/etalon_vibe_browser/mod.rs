//! Bounded browser integration probes using the canonical Vibe host and Etalon Rust API.

#[cfg(target_arch = "wasm32")]
mod gpu;

use crate::vibe_host::VibeHost;
use serde_json::{json, Value as Json};
use vibe::Value;

const GRAPH: &str = r#"
requires [capability("graph.write")];
effect fn run() {
    transaction {
        graph.stage(quin.statement(
            subject: <https://example.org/etalon/browser/s>,
            predicate: <https://example.org/etalon/browser/p>,
            object: <https://example.org/etalon/browser/o>,
            context: <https://example.org/etalon/browser/g>
        ));
        graph.commit()?;
    };
    return true;
}
"#;

fn case(name: &str, class: &str, result: Result<Json, String>) -> Json {
    match result {
        Ok(details) => json!({"name": name, "execution_class": class,
            "outcome": "passed", "details": details}),
        Err(error) => json!({"name": name, "execution_class": class,
            "outcome": "failed", "details": {"error": error}}),
    }
}

fn envelope(scope: &str, cases: Vec<Json>) -> Json {
    json!({"schema_version": 1, "scope": scope, "cases": cases,
        "limitations": ["Application integration, not WebGPU CTS conformance",
            "Browser WebGPU delegates shader translation and HAL execution to the browser",
            "Native Render.gpu_* Vibe capabilities are not available on this browser path",
            "Staged batched LLM argmax is diagnostic-only, not connected to production decode, and not exercised here"]})
}

fn graph_transaction() -> Result<Json, String> {
    let mut host = VibeHost::with_seed(vec![]);
    let result = host
        .eval_fn(GRAPH, "run", vec![])
        .map_err(|e| format!("{e:?}"))?;
    if result != Value::Bool(true) || host.committed.len() != 1 || host.staged_count() != 0 {
        return Err("checked graph transaction did not commit exactly one Quin".into());
    }
    let q = host.committed[0];
    if q.parity
        != crate::NQuin::calculate_parity(q.subject, q.predicate, q.object, q.context, q.metadata)
    {
        return Err("committed Quin parity mismatch".into());
    }
    Ok(
        json!({"host": "canonical VibeHost", "execution_path": "checked VibeScript -> Qualia graph transaction",
        "committed_quins": 1, "parity_verified": true}),
    )
}

fn capability_rejection() -> Result<Json, String> {
    let mut host = VibeHost::with_seed(vec![]);
    let source = GRAPH.replace("requires [capability(\"graph.write\")];", "");
    let error = match host.eval_fn(&source, "run", vec![]) {
        Err(error) => error,
        Ok(_) => return Err("graph write without its required capability was accepted".into()),
    };
    if error.code != vibe::DiagCode::E300 {
        return Err(format!(
            "expected missing-capability diagnostic E300, got {error:?}"
        ));
    }
    if !host.committed.is_empty() || host.staged_count() != 0 {
        return Err("rejected script changed the graph".into());
    }
    Ok(
        json!({"host": "canonical VibeHost", "diagnostic": format!("{:?}", error.code),
        "committed_quins": 0, "undeclared_graph_write_rejected": true}),
    )
}

fn cpu_report() -> Json {
    envelope(
        "qualia-vibe-cpu",
        vec![
            case(
                "canonical_vibe_graph_transaction",
                "cpu",
                graph_transaction(),
            ),
            case(
                "canonical_vibe_capability_rejection",
                "cpu",
                capability_rejection(),
            ),
        ],
    )
}

fn oracle(host: &mut VibeHost, a: f32, b: f32) -> Result<f32, String> {
    let source = format!("= {a:.2} + {b:.2}");
    match host.eval_cell_src(&source).map_err(|e| format!("{e:?}"))? {
        Value::F64(value) => Ok(value as f32),
        Value::I64(value) => Ok(value as f32),
        Value::U64(value) => Ok(value as f32),
        value => Err(format!("unexpected Vibe arithmetic result: {value:?}")),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_etalon_vibe_cpu_harness() -> String {
    cpu_report().to_string()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn run_etalon_vibe_gpu_harness() -> String {
    envelope("etalon-browser-webgpu", vec![gpu::run().await]).to_string()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn run_etalon_vibe_browser_harness() -> String {
    let mut report = cpu_report();
    report["scope"] = json!("qualia-vibe-etalon-browser");
    report["cases"]
        .as_array_mut()
        .unwrap()
        .push(gpu::run().await);
    report.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_graph_and_capability_cases_pass() {
        let report = cpu_report();
        for case in report["cases"].as_array().unwrap() {
            assert_eq!(case["outcome"], "passed", "{case}");
        }
    }

    #[test]
    fn canonical_oracle_handles_boundary_inputs() {
        let mut host = VibeHost::with_seed(vec![]);
        for index in [0, 1, 63, 64, 65, 256] {
            let a = index as f32 * 0.25;
            let b = 16.0 - index as f32 * 0.5;
            assert_eq!(oracle(&mut host, a, b).unwrap(), a + b);
        }
    }
}
