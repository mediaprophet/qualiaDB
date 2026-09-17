//! Validated request construction for the Calculus workbench.

use super::helpers::field_value;
use super::request_parse::{optional_f64, optional_u64, required_f64, required_u64};
use web_sys::Document;

pub(super) fn calculus_request(
    document: &Document,
    operation: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    let expression = field_value(document, "calculus-fn");
    let source = field_value(document, "calculus-input");
    if operation == "simd_width" {
        return Ok((
            "CalculusWorkbench.compute",
            serde_json::json!({ "operation": operation }),
        ));
    }
    if expression.trim().is_empty() {
        return Err("Enter a function expression before computing.".into());
    }
    let arguments = match operation {
        "rk4_step" => {
            let dt = required_f64(&source, "dt")?;
            let steps = required_u64(&source, "steps")?;
            serde_json::json!({
                "operation": operation,
                "system": expression,
                "vars": ["y"],
                "t_span": [0.0, dt * steps as f64],
                "y0": [required_f64(&source, "y0")?],
                "dt": dt
            })
        }
        "simpsons" | "trapezoidal" | "large_grid" => serde_json::json!({
            "operation": operation,
            "expr": expression,
            "a": required_f64(&source, "a")?,
            "b": required_f64(&source, "b")?,
            "panels": optional_u64(&source, "n")?.unwrap_or(if operation == "large_grid" { 100_000 } else { 1_000 })
        }),
        "adaptive" => serde_json::json!({
            "operation": operation,
            "expr": expression,
            "var": "x",
            "a": required_f64(&source, "a")?,
            "b": required_f64(&source, "b")?,
            "tolerance": optional_f64(&source, "tol")?.unwrap_or(1e-8),
            "max_evaluations": optional_u64(&source, "max_evaluations")?.unwrap_or(10_000)
        }),
        _ => return Err(format!("Unknown calculus operation `{operation}`.")),
    };
    Ok(("CalculusWorkbench.compute", arguments))
}
