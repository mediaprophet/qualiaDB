//! Validated requests for Allen/RCC8 and manifold-logic panels.

use super::helpers::field_value;
use super::request_parse::{
    assignment, optional_f64, optional_f64_list, optional_u64, required_f64, required_f64_list,
};
use web_sys::Document;

pub(super) fn spatial_request(
    document: &Document,
    mode: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    let args = match mode {
        "allen" => {
            let source = field_value(document, "allen-rcc8-input");
            serde_json::json!({
                "mode": "allen",
                "a": required_f64_list(&source, "a")?,
                "b": required_f64_list(&source, "b")?
            })
        }
        "rcc8" | "rcc8_points" => {
            let source = field_value(document, "allen-rcc8-input");
            serde_json::json!({
                "mode": mode,
                "a_id": optional_u64(&source, "a_id")?.unwrap_or(1),
                "b_id": optional_u64(&source, "b_id")?.unwrap_or(2),
                "a_points": required_f64_list(&source, "a_points")?,
                "b_points": required_f64_list(&source, "b_points")?
            })
        }
        "spatial_index" => {
            let source = field_value(document, "allen-rcc8-input");
            serde_json::json!({
                "mode": "spatial_index",
                "query": required_f64_list(&source, "query")?,
                "boxes": required_f64_list(&source, "boxes")?
            })
        }
        "minkowski" | "causally_connectable" => {
            let source = field_value(document, "allen-rcc8-input");
            serde_json::json!({
                "mode": mode,
                "dt": required_f64(&source, "dt")?,
                "dx": optional_f64(&source, "dx")?.unwrap_or(0.0),
                "dy": optional_f64(&source, "dy")?.unwrap_or(0.0),
                "dz": optional_f64(&source, "dz")?.unwrap_or(0.0),
                "c": optional_f64(&source, "c")?.unwrap_or(1.0)
            })
        }
        "heat_equation" => {
            let source = field_value(document, "allen-rcc8-input");
            serde_json::json!({
                "mode": "heat_equation",
                "u": required_f64_list(&source, "u")?,
                "alpha": required_f64(&source, "alpha")?,
                "dt": required_f64(&source, "dt")?,
                "dx": required_f64(&source, "dx")?
            })
        }
        "manifold-logic" => {
            let source = field_value(document, "manifold-logic-input");
            let operation = assignment(&source, "operation")
                .unwrap_or("continuous_to_fact")
                .to_string();
            serde_json::json!({
                "mode": "manifold",
                "operation": operation,
                "x": optional_f64(&source, "x")?.unwrap_or(0.0),
                "y": optional_f64(&source, "y")?.unwrap_or(0.0),
                "z": optional_f64(&source, "z")?.unwrap_or(0.0),
                "t": optional_f64(&source, "t")?.unwrap_or(0.0),
                "f": optional_f64(&source, "f")?.unwrap_or(1.0),
                "a": optional_f64(&source, "a")?.unwrap_or(1.0),
                "phi": optional_f64(&source, "phi")?.unwrap_or(0.0),
                "samples": optional_f64_list(&source, "samples")?.unwrap_or_default(),
                "threshold": optional_f64(&source, "threshold")?.unwrap_or(0.5),
                "fact_id": optional_u64(&source, "fact_id")?.unwrap_or(1)
            })
        }
        _ => return Err(format!("Unknown spatial request `{mode}`.")),
    };
    Ok(("SpatialLogic.compute", args))
}
