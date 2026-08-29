//! Validated requests for P2 infrastructure inspector panels.

use super::helpers::field_value;
use super::request_parse::{optional_f64, optional_f64_list, optional_u64, required_assignment};
use web_sys::Document;

pub(super) fn infra_request(
    document: &Document,
    mode: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    let args = match mode {
        "bytecode-vm" | "bytecode-vm-stats" => serde_json::json!({
            "mode": "bytecode",
            "operation": if mode.ends_with("stats") { "stats" } else { "trace" },
            "source": field_value(document, "bytecode-vm-input")
        }),
        "slg-arena" => {
            let source = field_value(document, "slg-arena-input");
            serde_json::json!({
                "mode": "slg_arena",
                "used_slots": optional_u64(&source, "used_slots")?.unwrap_or(0)
            })
        }
        op if op.starts_with("forge-compute") => {
            let operation = op.strip_prefix("forge-compute-").unwrap_or("top_k");
            let source = field_value(document, "forge-compute-input");
            let selected = field_value(document, "forge-compute-op");
            let operation = if operation == "forge-compute" || operation.is_empty() {
                selected
            } else {
                operation.to_string()
            };
            serde_json::json!({
                "mode": "forge",
                "operation": operation,
                "backend": field_value(document, "forge-compute-backend"),
                "values": optional_f64_list(&source, "values")?.unwrap_or_default(),
                "k": optional_u64(&source, "k")?,
                "a": optional_f64_list(&source, "a")?.unwrap_or_default(),
                "b": optional_f64_list(&source, "b")?.unwrap_or_default(),
                "m": optional_u64(&source, "m")?,
                "n": optional_u64(&source, "n")?,
                "k_dim": optional_u64(&source, "K")?,
                "samples": optional_f64_list(&source, "samples")?.unwrap_or_default(),
                "flops": optional_f64(&source, "flops")?,
                "bytes": optional_f64(&source, "bytes")?
            })
        }
        "compute-profile" => serde_json::json!({ "mode": "compute_profile" }),
        op if op.starts_with("privacy") => {
            let source = field_value(document, "privacy-input");
            serde_json::json!({
                "mode": "privacy",
                "operation": field_value(document, "privacy-op"),
                "plaintext": optional_f64_list(&source, "plaintext")?.unwrap_or_default(),
                "sensitivity": optional_f64(&source, "sensitivity")?,
                "epsilon": optional_f64(&source, "epsilon")?,
                "delta": optional_f64(&source, "delta")?,
                "participants": optional_u64(&source, "participants")?,
                "threshold": optional_u64(&source, "threshold")?
            })
        }
        "model-lifecycle" | "model-lifecycle-evict" => {
            let source = field_value(document, "model-lifecycle-input");
            serde_json::json!({
                "mode": "model_lifecycle",
                "model": field_value(document, "model-lifecycle-name"),
                "state": required_assignment(&source, "state").unwrap_or_else(|_| "discovered".into()),
                "action": if mode.ends_with("evict") {
                    "evict".into()
                } else {
                    required_assignment(&source, "action").unwrap_or_else(|_| "status".into())
                }
            })
        }
        "inference-monitor" => serde_json::json!({ "mode": "inference_monitor" }),
        "gguf-tokenizer" => {
            let source = field_value(document, "gguf-tokenizer-input");
            let text = match required_assignment(&source, "text") {
                Ok(text) => text,
                Err(_) => {
                    let model = field_value(document, "gguf-tokenizer-model");
                    if model.trim().is_empty() {
                        return Err(
                            "Enter `text=...` to tokenize with the default byte-level tokenizer."
                                .into(),
                        );
                    }
                    model
                }
            };
            serde_json::json!({
                "mode": "gguf_tokenizer",
                "text": text
            })
        }
        "p64-weight" => {
            let source = field_value(document, "p64-weight-input");
            serde_json::json!({
                "mode": "p64",
                "bytes": super::request_parse::assignment(&source, "bytes").unwrap_or("")
            })
        }
        _ => return Err(format!("Unknown infrastructure request `{mode}`.")),
    };
    Ok(("InfraLogic.compute", args))
}
