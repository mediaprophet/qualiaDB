//! MCP tools for native audio features, 3D spatial HRTF, and loudness metering.
//!
//! Exposes `qualia-audio` spatial processing, pitch tracking, and EBU R128 loudness.
//! Follows `AGENTS.md` §0-B (<500 lines).

use crate::types::{parse_tool_args, McpSystemError};
use serde_json::{json, Value};

/// `audio_features` tool — log mel, pitch yin, loudness r128, and capability inspection.
pub fn audio_features(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::audio_features(args).map_err(|e| match e {
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

/// `audio_spatial_hrtf` tool — 3D spatial audio positioning, attenuation, and binaural panning.
pub fn audio_spatial_hrtf(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let emitter_x = v.get("emitter_x").and_then(Value::as_f64).unwrap_or(0.0) as f32;
    let emitter_y = v.get("emitter_y").and_then(Value::as_f64).unwrap_or(0.0) as f32;
    let emitter_z = v.get("emitter_z").and_then(Value::as_f64).unwrap_or(0.0) as f32;

    let listener_x = v.get("listener_x").and_then(Value::as_f64).unwrap_or(0.0) as f32;
    let listener_y = v.get("listener_y").and_then(Value::as_f64).unwrap_or(0.0) as f32;
    let listener_z = v.get("listener_z").and_then(Value::as_f64).unwrap_or(0.0) as f32;

    let dx = emitter_x - listener_x;
    let dy = emitter_y - listener_y;
    let dz = emitter_z - listener_z;
    let distance = (dx * dx + dy * dy + dz * dz).sqrt().max(0.001);

    let model = qualia_audio::spatial::DistanceModel::default();
    let distance_gain = model.calculate_gain(distance);

    let azimuth_rad = (-dx).atan2(dz);
    let azimuth_deg = azimuth_rad.to_degrees();

    // Equal-power stereo panning
    let pan = (azimuth_rad / (std::f32::consts::PI / 2.0)).clamp(-1.0, 1.0);
    let angle = (pan + 1.0) * (std::f32::consts::PI / 4.0);
    let left_gain = angle.cos() * distance_gain;
    let right_gain = angle.sin() * distance_gain;

    Ok(json!({
        "status": "success",
        "distance": distance,
        "azimuth_degrees": azimuth_deg,
        "attenuation_gain": distance_gain,
        "stereo_panning": {
            "left_gain": left_gain,
            "right_gain": right_gain,
        }
    })
    .to_string())
}
