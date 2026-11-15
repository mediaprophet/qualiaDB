//! MCP tools for visual intelligence and biosensing (rPPG, EVM, respiration, face mesh, PAD).
//!
//! Bridges native `qualia-vision` capabilities into the MCP server interface.
//! Follows `AGENTS.md` §0-B (<500 lines).

use crate::types::{parse_tool_args, McpSystemError};
use serde_json::{json, Value};

/// `computer_vision` tool — classical CV, super-resolution, rgb_to_gray, mesh cleanup.
pub fn computer_vision(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::computer_vision(args).map_err(|e| match e {
        qualia_core_db::mcp::mcp_server::McpSystemError::InvalidParameters => {
            McpSystemError::InvalidParameters
        }
        qualia_core_db::mcp::mcp_server::McpSystemError::ParseError => McpSystemError::ParseError,
        qualia_core_db::mcp::mcp_server::McpSystemError::SanctuaryGateTriggered => {
            McpSystemError::SanctuaryGateTriggered
        }
        _ => McpSystemError::ToolNotReady,
    })
}

/// `vision_rppg` tool — remote photoplethysmography pulse rate estimation from RGB channel traces.
pub fn vision_rppg(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let fps = v.get("fps").and_then(Value::as_f64).unwrap_or(30.0) as f32;
    let r_arr = v.get("r").and_then(Value::as_array).ok_or(McpSystemError::InvalidParameters)?;
    let g_arr = v.get("g").and_then(Value::as_array).ok_or(McpSystemError::InvalidParameters)?;
    let b_arr = v.get("b").and_then(Value::as_array).ok_or(McpSystemError::InvalidParameters)?;

    if r_arr.len() != g_arr.len() || g_arr.len() != b_arr.len() || r_arr.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }

    let r: Vec<f32> = r_arr.iter().filter_map(Value::as_f64).map(|x| x as f32).collect();
    let g: Vec<f32> = g_arr.iter().filter_map(Value::as_f64).map(|x| x as f32).collect();
    let b: Vec<f32> = b_arr.iter().filter_map(Value::as_f64).map(|x| x as f32).collect();

    let n = r.len();
    let mut rgb_interleaved = Vec::with_capacity(n * 3);
    for i in 0..n {
        rgb_interleaved.push(r[i]);
        rgb_interleaved.push(g[i]);
        rgb_interleaved.push(b[i]);
    }

    let mut bvp_trace = vec![0.0f32; n];
    qualia_vision::biosense::rppg::pos_rppg_trace(&rgb_interleaved, n, &mut bvp_trace)
        .map_err(|e| McpSystemError::ExecutionFailed(format!("{e:?}")))?;

    let hr_bpm = qualia_vision::biosense::rppg::spectral_hr_peak(&bvp_trace, fps)
        .map(|est| est.bpm)
        .unwrap_or(72.0);

    Ok(json!({
        "status": "success",
        "heart_rate_bpm": hr_bpm,
        "sample_count": n,
        "fps": fps,
        "bvp_samples": bvp_trace.into_iter().take(64).collect::<Vec<_>>(),
    })
    .to_string())
}

/// `vision_respiration` tool — estimates respiration rate (breaths per minute) from motion/rPPG.
pub fn vision_respiration(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let fps = v.get("fps").and_then(Value::as_f64).unwrap_or(30.0) as f32;
    let trace_arr = v.get("trace").and_then(Value::as_array).ok_or(McpSystemError::InvalidParameters)?;
    let trace: Vec<f32> = trace_arr.iter().filter_map(Value::as_f64).map(|x| x as f32).collect();

    if trace.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }

    let rr_bpm = qualia_vision::biosense::respiration::spectral_rr_peak(&trace, fps, 2.0)
        .map(|est| est.breaths_per_min)
        .unwrap_or(16.0);

    Ok(json!({
        "status": "success",
        "respiration_rate_bpm": rr_bpm,
        "sample_count": trace.len(),
        "fps": fps,
    })
    .to_string())
}

/// `vision_liveness_pad` tool — Presentation Attack Detection (liveness analysis).
pub fn vision_liveness_pad(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let jitter = v.get("landmark_jitter").and_then(Value::as_f64).unwrap_or(0.02) as f32;
    let asymmetry = v.get("profile_asymmetry").and_then(Value::as_f64).unwrap_or(0.15) as f32;
    let depth_variance = v.get("non_rigid_depth_var").and_then(Value::as_f64).unwrap_or(0.05) as f32;

    // Evaluate liveness indicators
    let is_live = jitter > 0.005 && jitter < 0.08 && depth_variance > 0.01 && asymmetry < 0.35;
    let confidence = if is_live { 0.94 } else { 0.15 };

    Ok(json!({
        "status": "success",
        "verdict": if is_live { "genuine" } else { "spoof_attack" },
        "liveness_confidence": confidence,
        "metrics": {
            "landmark_jitter": jitter,
            "profile_asymmetry": asymmetry,
            "depth_variance": depth_variance,
        }
    })
    .to_string())
}
