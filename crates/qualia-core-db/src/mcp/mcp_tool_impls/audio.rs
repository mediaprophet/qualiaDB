//! MCP `audio_features` tool — exposes the qualia-audio capability registry + CPU feature ops.
//!
//! Mirrors `computer_vision` (`vision.rs`): cold-path JSON in, `serde_json` out, caller-supplied
//! sample buffers, size caps + prefix sampling so responses never dump full matrices. The whole
//! module is `#[cfg(not(target_arch = "wasm32"))]` (wired in `mod.rs`), so there is no wasm variant.
//!
//! See delivery plan Wave 9. The capability registry it reads is the single honest source of truth
//! (`qualia_audio::capability_registry`).

use super::{json_f64_array, json_str, json_u64, parse_tool_args, McpSystemError};
use serde_json::{json, Value};

/// Reject absurd sample buffers before touching the DSP (edge safety, mirrors vision's caps).
const MAX_SAMPLES: usize = 2_000_000;

/// Audio feature ops over caller-supplied PCM (cold path JSON).
///
/// Ops: `list`, `capability_summary`, `log_mel`, `pitch_yin`, `loudness_r128`.
#[cfg(not(target_arch = "wasm32"))]
pub fn audio_features(args: &[u8]) -> Result<String, McpSystemError> {
    use qualia_audio::capability_registry as reg;

    let v = parse_tool_args(args)?;
    let op = json_str(&v, "op", "list");

    match op {
        "list" => {
            let caps: Vec<Value> = reg::CAPABILITIES
                .iter()
                .map(|c| {
                    json!({
                        "id": c.id,
                        "domain": c.domain.as_str(),
                        "status": c.status.as_str(),
                        "zero_heap_hot": c.zero_heap_hot,
                        "streaming": c.streaming,
                        "note": c.note,
                    })
                })
                .collect();
            Ok(json!({
                "library": "qualia_audio::capability_registry",
                "ops": ["list", "capability_summary", "log_mel", "pitch_yin", "loudness_r128"],
                "count": caps.len(),
                "capabilities": caps,
            })
            .to_string())
        }
        "capability_summary" => {
            use reg::CapabilityStatus as S;
            let present = reg::count_by_status(S::Present);
            let partial = reg::count_by_status(S::Partial);
            let missing = reg::count_by_status(S::Missing);
            let needs_weights = reg::count_by_status(S::NeedsWeights);
            let feature_disabled = reg::count_by_status(S::FeatureDisabled);
            Ok(json!({
                "present": present,
                "partial": partial,
                "missing": missing,
                "needs_weights": needs_weights,
                "feature_disabled": feature_disabled,
                "total": reg::CAPABILITIES.len(),
            })
            .to_string())
        }
        "log_mel" => {
            let samples = samples_f32(&v)?;
            let sample_rate = json_u64(&v, "sample_rate", 16000) as u32;
            let n_mel = (json_u64(&v, "n_mel", 16) as usize).max(1);
            if sample_rate == 0 {
                return Err(McpSystemError::InvalidParameters);
            }
            // Cap the number of frames we retain so the response stays bounded (edge safety).
            const MAX_FRAMES: usize = 64;
            let mut out = vec![0.0f32; MAX_FRAMES * n_mel];
            let n_frames =
                qualia_audio::log_mel_from_mono(&samples, 256, 128, sample_rate, n_mel, &mut out)
                    .map_err(|_| McpSystemError::InvalidParameters)?;
            // Return stats + a prefix, never the whole matrix.
            let mel_prefix: Vec<f32> = out
                .iter()
                .take((n_frames * n_mel).min(32))
                .copied()
                .collect();
            Ok(json!({
                "n_frames": n_frames,
                "n_mel": n_mel,
                "sample_rate": sample_rate,
                "mel_prefix": mel_prefix,
            })
            .to_string())
        }
        "pitch_yin" => {
            let samples = samples_f32(&v)?;
            let sample_rate = json_u64(&v, "sample_rate", 44100) as f32;
            let take = samples.len().min(2048);
            let frame = &samples[..take];
            let mut scratch = vec![0.0f32; frame.len()];
            let est = qualia_audio::features::pitch::yin_pitch(
                frame,
                sample_rate,
                50.0,
                2000.0,
                0.15,
                &mut scratch,
            )
            .map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(json!({
                "f0_hz": est.f0_hz,
                "confidence": est.confidence,
            })
            .to_string())
        }
        "loudness_r128" => {
            let samples = samples_f32(&v)?;
            let sample_rate = json_u64(&v, "sample_rate", 48000) as u32;
            let integrated_lufs =
                qualia_audio::features::loudness::integrated_lufs(&samples, sample_rate)
                    .map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(json!({
                "integrated_lufs": integrated_lufs,
            })
            .to_string())
        }
        _ => Err(McpSystemError::ToolNotFound),
    }
}

/// Read the `samples` array as `f32`, rejecting absurdly large buffers (edge safety).
#[cfg(not(target_arch = "wasm32"))]
fn samples_f32(v: &Value) -> Result<Vec<f32>, McpSystemError> {
    let samples = json_f64_array(v, "samples")?;
    if samples.len() > MAX_SAMPLES {
        return Err(McpSystemError::InvalidParameters);
    }
    Ok(samples.into_iter().map(|x| x as f32).collect())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn capability_summary_reports_real_coverage() {
        let out = audio_features(br#"{"op":"capability_summary"}"#).expect("summary ok");
        let v: Value = serde_json::from_str(&out).expect("valid json");
        assert!(
            v["present"].as_u64().unwrap() >= 50,
            "present coverage: {out}"
        );
        assert!(v["total"].as_u64().unwrap() >= 60);
    }

    #[test]
    fn list_returns_registry() {
        let out = audio_features(br#"{"op":"list"}"#).expect("list ok");
        let v: Value = serde_json::from_str(&out).expect("valid json");
        assert!(v["capabilities"].as_array().unwrap().len() >= 60);
    }

    #[test]
    fn pitch_yin_recovers_440hz_through_mcp() {
        // Real end-to-end: a 440 Hz sine in as JSON, real YIN out.
        let sr = 44100.0f64;
        let mut samples = String::from("[");
        for i in 0..2048 {
            let s = (std::f64::consts::TAU * 440.0 * i as f64 / sr).sin();
            if i > 0 {
                samples.push(',');
            }
            samples.push_str(&format!("{s}"));
        }
        samples.push(']');
        let req = format!(r#"{{"op":"pitch_yin","samples":{samples},"sample_rate":44100}}"#);
        let out = audio_features(req.as_bytes()).expect("pitch ok");
        let v: Value = serde_json::from_str(&out).expect("valid json");
        let f0 = v["f0_hz"].as_f64().unwrap();
        assert!((f0 - 440.0).abs() < 5.0, "recovered f0 {f0} through MCP");
    }

    #[test]
    fn unknown_op_is_tool_not_found() {
        assert!(matches!(
            audio_features(br#"{"op":"nope"}"#),
            Err(McpSystemError::ToolNotFound)
        ));
    }
}
