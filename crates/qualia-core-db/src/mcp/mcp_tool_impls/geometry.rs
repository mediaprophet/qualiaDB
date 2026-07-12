use super::*;


pub fn computational_geometry(args: &[u8]) -> Result<String, McpSystemError> {
    let text = core::str::from_utf8(args).map_err(|_| McpSystemError::ParseError)?;
    crate::specialized_libs::computational_geometry::execute_geometry_tool_json(text)
        .map_err(|error| match error {
            crate::specialized_libs::computational_geometry::GeometryToolError::InvalidJson => {
                McpSystemError::ParseError
            }
            crate::specialized_libs::computational_geometry::GeometryToolError::InvalidOperation => {
                McpSystemError::ToolNotFound
            }
            crate::specialized_libs::computational_geometry::GeometryToolError::InvalidParameters
            | crate::specialized_libs::computational_geometry::GeometryToolError::Geometry(_) => {
                McpSystemError::InvalidParameters
            }
        })
}

/// List per-op capability manifests or run a Reserve-mode budget query.
///
/// With no `op` field: returns all manifests as JSON.
/// With `op` + `device`: returns the runnable backends for that device.
pub fn geometry_manifests(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::computational_geometry::capability_manifests;

    let v = parse_tool_args(args)?;
    let op = v.get("op").and_then(Value::as_str);

    if let Some(op_name) = op {
        // Reserve-mode budget query.
        let device = v.get("device");
        let avail = capability_manifests::DeviceAvailability {
            cpu: device
                .and_then(|d| d.get("cpu"))
                .and_then(Value::as_bool)
                .unwrap_or(true),
            simd: device
                .and_then(|d| d.get("simd"))
                .and_then(Value::as_bool)
                .unwrap_or(true),
            wgpu: device
                .and_then(|d| d.get("wgpu"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            cuda: device
                .and_then(|d| d.get("cuda"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            wasm: device
                .and_then(|d| d.get("wasm"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            exact: device
                .and_then(|d| d.get("exact"))
                .and_then(Value::as_bool)
                .unwrap_or(true),
        };
        Ok(capability_manifests::budget_query_to_json(op_name, &avail).to_string())
    } else {
        // List all manifests.
        Ok(capability_manifests::manifests_to_json().to_string())
    }
}

pub fn geometric_algebra_op(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::geometric_algebra::utils::{angle_between_vectors, cross_product, dot_product};

    let v = parse_tool_args(args)?;
    let op = json_str(&v, "op", "cross");
    let a_arr = json_f64_array(&v, "a")?;
    let b_arr = json_f64_array(&v, "b")?;
    if a_arr.len() != 3 || b_arr.len() != 3 {
        return Err(McpSystemError::InvalidParameters);
    }
    let a = [a_arr[0] as f32, a_arr[1] as f32, a_arr[2] as f32];
    let b = [b_arr[0] as f32, b_arr[1] as f32, b_arr[2] as f32];

    let result = match op {
        "angle" => json!({
            "op": "angle",
            "radians": angle_between_vectors(&a, &b),
            "degrees": angle_between_vectors(&a, &b).to_degrees()
        }),
        "dot" => json!({"op": "dot", "value": dot_product(&a, &b)}),
        _ => {
            let c = cross_product(&a, &b);
            json!({"op": "cross", "result": [c[0], c[1], c[2]]})
        }
    };
    Ok(result.to_string())
}
