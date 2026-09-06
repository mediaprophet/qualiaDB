//! Capability-specific payloads for live spec-tool rows.
//!
//! A live row must not call a real host kernel with `{}` when that kernel has
//! required inputs. Inputs come from the selected surface when they exist;
//! otherwise dispatch reports the missing prerequisite before invoking.

use super::row::SpecTool;
use serde_json::{json, Value};
use web_sys::Element;

const MAX_LIVE_PIXELS: u64 = 512 * 512;
const MAX_N3_SOURCE_BYTES: usize = 256 * 1024;
const MAX_INFERENCE_TEXT_BYTES: usize = 64 * 1024;

pub fn supports(capability: &str) -> bool {
    matches!(
        capability,
        "Animation.bezier_eval"
            | "Animation.easing"
            | "Animation.evaluate_preset"
            | "CausalFuzzyAndControl.caused"
            | "ComputationalGeometry.convex_hull_2"
            | "ComputationalGeometry.triangulate_polygon"
            | "ComputationalGeometry.distance_2d"
            | "ComputationalGeometry.surface_area"
            | "ComputerVision.canny_edges"
            | "ComputerVision.cosine_similarity"
            | "ComputerVision.dhash"
            | "ComputerVision.equalize_hist"
            | "ComputerVision.gaussian_blur"
            | "ComputerVision.hamming_distance"
            | "ComputerVision.histogram"
            | "ComputerVision.rgb_to_gray"
            | "ComputerVision.sobel_magnitude"
            | "DeonticLogic.evaluate"
            | "EpistemicLogic.evaluate"
            | "Inference.detect_ungrounded"
            | "Inference.grounding"
            | "Inference.verify_turn"
            | "N3Logic.evaluate"
            | "ParaconsistentLogic.route"
            | "Pulse.publish"
            | "Pulse.publish_presence"
            | "Render.gpu_set_camera"
            | "Render.scene"
            | "SHACL.validate"
            | "SymbolicAlgebra.eval"
            | "TemporalAndDescriptionLogic.ltl.evaluate"
    )
}

pub fn build(
    selected: Option<&Element>,
    tool: &SpecTool,
    capability: &str,
) -> Result<Value, String> {
    if !supports(capability) {
        return Err(format!(
            "{capability} has no checked Tool Chest input adapter yet."
        ));
    }
    match capability {
        "Animation.bezier_eval" | "Animation.easing" => Ok(json!({
            "curve": animation_curve(tool.id),
            "t": numeric_attr(selected, "data-animation-progress").unwrap_or(0.5),
        })),
        "Animation.evaluate_preset" => Ok(json!({
            "family": "spatial_kinematics",
            "preset": "orbit_spin",
            "t": numeric_attr(selected, "data-animation-progress").unwrap_or(0.5),
        })),
        "Render.scene" => Ok(json!({ "kind": scene_kind(tool.toolbox) })),
        "Pulse.publish" | "Pulse.publish_presence" => Ok(json!({
            "channel": format!("{}/{}", tool.toolbox, tool.chain),
            "payload_type": tool.id,
        })),
        "Render.gpu_set_camera" => camera_args(selected),
        "ComputerVision.gaussian_blur"
        | "ComputerVision.histogram"
        | "ComputerVision.canny_edges"
        | "ComputerVision.sobel_magnitude"
        | "ComputerVision.equalize_hist"
        | "ComputerVision.dhash" => grayscale_args(selected),
        "ComputerVision.rgb_to_gray" => rgb_args(selected),
        "ComputerVision.hamming_distance" => hamming_args(selected),
        "ComputerVision.cosine_similarity" => cosine_args(selected),
        "ComputationalGeometry.convex_hull_2"
        | "ComputationalGeometry.triangulate_polygon"
        | "ComputationalGeometry.distance_2d"
        | "ComputationalGeometry.surface_area" => geometry_points_2d(selected),
        "DeonticLogic.evaluate" => deontic_args(selected),
        "EpistemicLogic.evaluate" | "ParaconsistentLogic.route" => epistemic_args(selected),
        "TemporalAndDescriptionLogic.ltl.evaluate" => Ok(json!({
            "formula": "Globally",
            "property": numeric_attr(selected, "data-property-hash").unwrap_or(0.0) as u64,
        })),
        "SymbolicAlgebra.eval" => symbolic_args(selected),
        "Inference.grounding" => inference_args(selected, "text"),
        "Inference.detect_ungrounded" | "Inference.verify_turn" => {
            inference_args(selected, "draft")
        }
        "CausalFuzzyAndControl.caused" => causal_args(selected),
        "SHACL.validate" => shacl_args(selected),
        "N3Logic.evaluate" => n3_args(selected),
        _ => unreachable!("supports and build capability tables drifted"),
    }
}

fn animation_curve(tool_id: &str) -> &'static str {
    if tool_id.contains("fade-in") {
        "cubic-in"
    } else if tool_id.contains("fade-out") {
        "cubic-out"
    } else if tool_id.contains("bounce") {
        "bounce-out"
    } else {
        "cubic-in-out"
    }
}

fn scene_kind(toolbox: &str) -> &'static str {
    match toolbox {
        "audio" | "image" | "video" | "productions" => "media",
        "portals" | "spatial" | "spatial3d" => "submanifold",
        _ => "research",
    }
}

fn camera_args(selected: Option<&Element>) -> Result<Value, String> {
    let selected = selected.ok_or_else(|| "Select a live 3D viewport first.".to_string())?;
    let handle = selected
        .get_attribute("data-render-handle")
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| "The selected viewport has no live renderer handle yet.".to_string())?;
    Ok(json!({
        "handle": handle,
        "yaw": numeric_attr(Some(selected), "data-camera-yaw").unwrap_or(0.0),
        "pitch": numeric_attr(Some(selected), "data-camera-pitch").unwrap_or(0.0),
        "zoom": numeric_attr(Some(selected), "data-camera-zoom").unwrap_or(1.0),
    }))
}

fn grayscale_args(selected: Option<&Element>) -> Result<Value, String> {
    let selected = selected.ok_or_else(|| "Select a picture with pixel data first.".to_string())?;
    let raw = selected
        .get_attribute("data-grayscale-u8")
        .ok_or_else(|| "The selected picture has no decoded greyscale pixels yet.".to_string())?;
    let width = integer_attr(selected, "data-pixel-width")
        .ok_or_else(|| "The selected picture has no pixel width.".to_string())?;
    let height = integer_attr(selected, "data-pixel-height")
        .ok_or_else(|| "The selected picture has no pixel height.".to_string())?;
    let expected = width
        .checked_mul(height)
        .ok_or_else(|| "The selected picture dimensions are too large.".to_string())?;
    if expected > MAX_LIVE_PIXELS || raw.len() > (MAX_LIVE_PIXELS as usize * 4) {
        return Err("Live picture tools are limited to 512 × 512 greyscale pixels.".to_string());
    }
    let data = parse_u8_csv(&raw)?;
    if data.len() as u64 != expected {
        return Err(format!(
            "The selected picture has {} pixels, but its dimensions require {expected}.",
            data.len()
        ));
    }
    Ok(json!({ "data": data, "width": width, "height": height, "stride": width }))
}

fn rgb_args(selected: Option<&Element>) -> Result<Value, String> {
    let selected = selected.ok_or_else(|| "Select a picture with RGB pixel data first.".to_string())?;
    let raw = selected
        .get_attribute("data-rgb-u8")
        .ok_or_else(|| "The selected picture has no decoded RGB pixels yet (data-rgb-u8).".to_string())?;
    let width = integer_attr(selected, "data-pixel-width")
        .ok_or_else(|| "The selected picture has no pixel width.".to_string())?;
    let height = integer_attr(selected, "data-pixel-height")
        .ok_or_else(|| "The selected picture has no pixel height.".to_string())?;
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| "The selected picture dimensions are too large.".to_string())?;
    let expected = pixels
        .checked_mul(3)
        .ok_or_else(|| "The selected picture dimensions are too large.".to_string())?;
    if pixels > MAX_LIVE_PIXELS || raw.len() > (MAX_LIVE_PIXELS as usize * 12) {
        return Err("Live picture tools are limited to 512 × 512 RGB pixels.".to_string());
    }
    let data = parse_u8_csv(&raw)?;
    if data.len() as u64 != expected {
        return Err(format!(
            "The selected picture has {} RGB bytes, but its dimensions require {expected}.",
            data.len()
        ));
    }
    Ok(json!({ "data": data, "width": width, "height": height, "stride": width * 3 }))
}

fn hamming_args(selected: Option<&Element>) -> Result<Value, String> {
    let selected =
        selected.ok_or_else(|| "Select a surface with two perceptual hashes first.".to_string())?;
    let a = selected
        .get_attribute("data-hash-a")
        .or_else(|| selected.get_attribute("data-dhash"))
        .and_then(|v| parse_u64_token(&v))
        .ok_or_else(|| "Set data-hash-a (u64) on the selected surface.".to_string())?;
    let b = selected
        .get_attribute("data-hash-b")
        .or_else(|| selected.get_attribute("data-ahash"))
        .and_then(|v| parse_u64_token(&v))
        .ok_or_else(|| "Set data-hash-b (u64) on the selected surface.".to_string())?;
    Ok(json!({ "a": a, "b": b }))
}

fn cosine_args(selected: Option<&Element>) -> Result<Value, String> {
    let selected =
        selected.ok_or_else(|| "Select a surface with two embedding vectors first.".to_string())?;
    let a = selected
        .get_attribute("data-embedding-a")
        .ok_or_else(|| "Set data-embedding-a (CSV floats) on the selected surface.".to_string())?;
    let b = selected
        .get_attribute("data-embedding-b")
        .ok_or_else(|| "Set data-embedding-b (CSV floats) on the selected surface.".to_string())?;
    let a = parse_f64_csv(&a)?;
    let b = parse_f64_csv(&b)?;
    if a.is_empty() || b.is_empty() {
        return Err("Embedding vectors must contain at least one finite float.".to_string());
    }
    if a.len() != b.len() {
        return Err(format!(
            "Embedding lengths differ ({} vs {}).",
            a.len(),
            b.len()
        ));
    }
    Ok(json!({ "a": a, "b": b }))
}

fn geometry_points_2d(selected: Option<&Element>) -> Result<Value, String> {
    let raw = selected
        .and_then(|s| s.get_attribute("data-points-2d"))
        .unwrap_or_else(|| "0.0,0.0; 1.0,0.0; 0.5,1.0".into());
    let mut pts = Vec::new();
    for pair in raw.split(';').map(str::trim).filter(|p| !p.is_empty()) {
        let coords: Vec<f64> = pair
            .split(',')
            .map(str::trim)
            .filter_map(|c| c.parse::<f64>().ok())
            .collect();
        if coords.len() == 2 {
            pts.push(vec![coords[0], coords[1]]);
        }
    }
    if pts.is_empty() {
        return Err("No valid 2D coordinates found in points dataset.".to_string());
    }
    Ok(json!({ "points": pts }))
}

fn epistemic_args(selected: Option<&Element>) -> Result<Value, String> {
    let agent = selected
        .and_then(|s| s.get_attribute("data-agent-did"))
        .unwrap_or_else(|| "did:q42:agent:default".into());
    let world = selected
        .and_then(|s| s.get_attribute("data-epistemic-world"))
        .unwrap_or_else(|| "did:q42:world:actual".into());
    Ok(json!({
        "agent": agent,
        "world": world,
        "certainty": numeric_attr(selected, "data-certainty").unwrap_or(1.0),
    }))
}

fn deontic_args(selected: Option<&Element>) -> Result<Value, String> {
    let subject = selected
        .and_then(|s| s.get_attribute("data-norm-subject"))
        .or_else(|| selected.and_then(|s| s.get_attribute("data-subject")))
        .unwrap_or_else(|| "did:q42:agent:principal".into());
    let action = selected
        .and_then(|s| s.get_attribute("data-norm-action"))
        .unwrap_or_else(|| "read".into());
    let modality = selected
        .and_then(|s| s.get_attribute("data-norm-modality"))
        .unwrap_or_else(|| "permit".into());
    Ok(json!({
        "subject": subject,
        "action": action,
        "modality": modality,
    }))
}

fn symbolic_args(selected: Option<&Element>) -> Result<Value, String> {
    let expr = selected
        .and_then(|s| s.get_attribute("data-formula"))
        .or_else(|| selected.and_then(|s| s.text_content()))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "x^2 + 2*x + 1".into());
    Ok(json!({ "expr": expr.trim() }))
}

fn shacl_args(selected: Option<&Element>) -> Result<Value, String> {
    let selected = selected.ok_or_else(|| "Select a graph-backed surface first.".to_string())?;
    let subject = selected
        .get_attribute("data-quin-subject")
        .or_else(|| selected.get_attribute("data-subject"))
        .ok_or_else(|| "The selected surface has no graph subject to validate.".to_string())?;
    Ok(json!({ "subject": subject, "kind": "minCount", "value": 1 }))
}

fn inference_args(selected: Option<&Element>, text_key: &str) -> Result<Value, String> {
    let selected = selected.ok_or_else(|| "Select a text-backed surface first.".to_string())?;
    let prompt = selected
        .get_attribute("data-grounding-prompt")
        .unwrap_or_else(|| "Check the selected surface against its recorded evidence.".into());
    let text = selected
        .get_attribute("data-grounding-text")
        .or_else(|| selected.text_content())
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| "The selected surface has no text to check.".to_string())?;
    if prompt.len() + text.len() > MAX_INFERENCE_TEXT_BYTES {
        return Err("Grounding input exceeds the 64 KiB workbench limit.".to_string());
    }
    let mut args = serde_json::Map::new();
    args.insert("prompt".into(), Value::String(prompt));
    args.insert(text_key.into(), Value::String(text));
    Ok(Value::Object(args))
}

fn causal_args(selected: Option<&Element>) -> Result<Value, String> {
    let selected = selected.ok_or_else(|| "Select a graph-backed surface first.".to_string())?;
    let effect = selected
        .get_attribute("data-causal-effect")
        .ok_or_else(|| "Choose the effect node for this causal query.".to_string())?;
    let roots: Vec<_> = selected
        .get_attribute("data-causal-roots")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|root| !root.is_empty())
        .take(32)
        .map(str::to_string)
        .collect();
    if roots.is_empty() {
        return Err("Choose at least one causal root node.".to_string());
    }
    Ok(json!({ "effect": effect, "roots": roots }))
}

fn n3_args(selected: Option<&Element>) -> Result<Value, String> {
    let selected = selected.ok_or_else(|| "Select a meaning document first.".to_string())?;
    let source = selected
        .get_attribute("data-n3-source")
        .or_else(|| selected.text_content())
        .filter(|source| !source.trim().is_empty())
        .ok_or_else(|| "The selected meaning document has no N3 source.".to_string())?;
    if source.len() > MAX_N3_SOURCE_BYTES {
        return Err("The selected N3 source exceeds the 256 KiB workbench limit.".to_string());
    }
    Ok(json!({ "source": source, "mode": "evaluate" }))
}

fn numeric_attr(selected: Option<&Element>, name: &str) -> Option<f64> {
    selected?
        .get_attribute(name)?
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn integer_attr(selected: &Element, name: &str) -> Option<u64> {
    selected.get_attribute(name)?.parse::<u64>().ok()
}

fn parse_u8_csv(raw: &str) -> Result<Vec<u8>, String> {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse::<u8>()
                .map_err(|_| "Greyscale pixels must be comma-separated bytes.".to_string())
        })
        .collect()
}

fn parse_f64_csv(raw: &str) -> Result<Vec<f64>, String> {
    let values: Vec<f64> = raw
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';'))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse::<f64>()
                .map_err(|_| "Embedding values must be comma-separated floats.".to_string())
                .and_then(|n| {
                    if n.is_finite() {
                        Ok(n)
                    } else {
                        Err("Embedding values must be finite floats.".to_string())
                    }
                })
        })
        .collect::<Result<_, _>>()?;
    Ok(values)
}

fn parse_u64_token(raw: &str) -> Option<u64> {
    let trimmed = raw.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16).ok();
    }
    trimmed.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animation_curves_are_deterministic_from_tool_ids() {
        assert_eq!(animation_curve("video:fade-in"), "cubic-in");
        assert_eq!(animation_curve("video:fade-out"), "cubic-out");
        assert_eq!(animation_curve("video:crossfade"), "cubic-in-out");
    }

    #[test]
    fn scene_kinds_follow_toolbox_surfaces() {
        assert_eq!(scene_kind("video"), "media");
        assert_eq!(scene_kind("spatial3d"), "submanifold");
        assert_eq!(scene_kind("research"), "research");
    }

    #[test]
    fn grayscale_csv_rejects_non_bytes() {
        assert_eq!(parse_u8_csv("0, 127, 255").unwrap(), [0, 127, 255]);
        assert!(parse_u8_csv("0, 256").is_err());
    }

    #[test]
    fn capabilities_coverage_includes_geometry_and_logic() {
        assert!(supports("Render.scene"));
        assert!(supports("ComputationalGeometry.convex_hull_2"));
        assert!(supports("DeonticLogic.evaluate"));
        assert!(supports("EpistemicLogic.evaluate"));
        assert!(supports("Inference.verify_turn"));
        assert!(supports("Inference.detect_ungrounded"));
        assert!(supports("ComputerVision.histogram"));
        assert!(supports("ComputerVision.equalize_hist"));
        assert!(supports("ComputerVision.rgb_to_gray"));
        assert!(supports("ComputerVision.dhash"));
        assert!(supports("ComputerVision.hamming_distance"));
        assert!(supports("ComputerVision.cosine_similarity"));
        assert!(supports("SymbolicAlgebra.eval"));
        assert!(!supports("Future.unimplemented"));
    }

    #[test]
    fn parse_u64_token_accepts_hex() {
        assert_eq!(parse_u64_token("0x10"), Some(16));
        assert_eq!(parse_u64_token("42"), Some(42));
    }

    #[test]
    fn parse_f64_csv_rejects_non_finite() {
        assert_eq!(parse_f64_csv("1.0, 2.5").unwrap(), [1.0, 2.5]);
        assert!(parse_f64_csv("1.0, nan").is_err());
    }
}
