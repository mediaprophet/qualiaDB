//! Vision and audio first-release commands

#![allow(non_snake_case)]

use super::parse_hex_u64;
use tauri::{command, AppHandle, State};

// ── Vision first-release (overlay + synthetic + human attestation) ─────────

#[command]
pub fn vision_run_synthetic_demo(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    split: String,
    index: u32,
    persist: Option<bool>,
    backend: Option<String>,
) -> Result<serde_json::Value, String> {
    let split = match split.to_lowercase().as_str() {
        "train" => qualia_vision::DatasetSplit::Train,
        _ => qualia_vision::DatasetSplit::Test,
    };
    let be = backend.as_deref().unwrap_or("reference");
    let demo = qualia_client_core::vision_pipeline::run_synthetic_demo_with_backend(
        split, index, 96, 64, be,
    )?;
    if persist.unwrap_or(false) {
        let config = state.config.lock().unwrap().clone();
        let root = std::path::PathBuf::from(&config.storage_path);
        let _ = qualia_client_core::vision_pipeline::ingest_demo_to_wal(&root, &demo)?;
    }
    serde_json::to_value(demo).map_err(|e| e.to_string())
}

#[command]
pub fn vision_generate_image(
    prompt: String,
    seed: Option<u64>,
    steps: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<serde_json::Value, String> {
    let r = qualia_client_core::vision_pipeline::generate_image(
        &prompt,
        seed.unwrap_or(1),
        steps.unwrap_or(4),
        width.unwrap_or(64),
        height.unwrap_or(64),
    )?;
    serde_json::to_value(r).map_err(|e| e.to_string())
}

#[command]
pub fn vision_image_to_3d_demo(
    prompt: Option<String>,
    seed: Option<u64>,
) -> Result<serde_json::Value, String> {
    let (gen, mesh) = qualia_client_core::vision_pipeline::generate_and_reconstruct(
        prompt.as_deref().unwrap_or("heightfield demo"),
        seed.unwrap_or(3),
    )?;
    Ok(serde_json::json!({ "generate": gen, "mesh": mesh }))
}

#[command]
pub fn audio_ears_demo(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    persist: Option<bool>,
) -> Result<serde_json::Value, String> {
    let root = if persist.unwrap_or(true) {
        let config = state.config.lock().unwrap().clone();
        Some(std::path::PathBuf::from(&config.storage_path))
    } else {
        None
    };
    let r = qualia_client_core::audio_pipeline::ears_demo(root.as_deref())?;
    serde_json::to_value(r).map_err(|e| e.to_string())
}

#[command]
pub fn audio_cross_modal_demo() -> Result<serde_json::Value, String> {
    let r = qualia_client_core::audio_pipeline::cross_modal_demo();
    serde_json::to_value(r).map_err(|e| e.to_string())
}

#[command]
pub fn audio_section18_smoke() -> Result<String, String> {
    qualia_client_core::audio_pipeline::section18_smoke_dto()
}

#[command]
pub fn audio_import_wav(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    path: String,
) -> Result<serde_json::Value, String> {
    let config = state.config.lock().unwrap().clone();
    let root = std::path::PathBuf::from(&config.storage_path);
    let r = qualia_client_core::audio_pipeline::ears_from_wav(Some(&root), path.as_ref())?;
    serde_json::to_value(r).map_err(|e| e.to_string())
}

#[command]
pub fn audio_reject_instance(instance_hash_hex: String) -> Result<String, String> {
    qualia_client_core::audio_pipeline::audio_reject_instance(&instance_hash_hex)
}

#[command]
pub fn audio_correct_instance(
    instance_hash_hex: String,
    new_class_hash_hex: String,
) -> Result<String, String> {
    qualia_client_core::audio_pipeline::audio_correct_instance(
        &instance_hash_hex,
        &new_class_hash_hex,
    )
}

#[command]
pub fn vision_detect_image_file(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    path: String,
    backend: Option<String>,
) -> Result<serde_json::Value, String> {
    let config = state.config.lock().unwrap().clone();
    let root = std::path::PathBuf::from(&config.storage_path);
    let be = backend.as_deref().unwrap_or("reference");
    let r = qualia_client_core::vision_pipeline::detect_from_image_file(&root, path.as_ref(), be)?;
    serde_json::to_value(r).map_err(|e| e.to_string())
}

#[command]
pub fn vision_section15_smoke() -> Result<String, String> {
    qualia_client_core::vision_pipeline::section15_smoke()
}

#[command]
pub fn audio_ears_weighted(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
) -> Result<serde_json::Value, String> {
    let config = state.config.lock().unwrap().clone();
    let root = std::path::PathBuf::from(&config.storage_path);
    let r = qualia_client_core::audio_pipeline::ears_weighted_demo(Some(&root))?;
    serde_json::to_value(r).map_err(|e| e.to_string())
}

#[command]
pub fn audio_sonify_hear(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
) -> Result<serde_json::Value, String> {
    let config = state.config.lock().unwrap().clone();
    let root = std::path::PathBuf::from(&config.storage_path);
    qualia_client_core::audio_pipeline::sonify_ears_demo(Some(&root))
}

#[command]
pub fn audio_speech_demo(supported: Option<bool>) -> Result<serde_json::Value, String> {
    qualia_client_core::audio_pipeline::speech_demo(supported.unwrap_or(true))
}

#[command]
pub fn audio_capture_policy_demo() -> Result<serde_json::Value, String> {
    qualia_client_core::audio_pipeline::capture_policy_demo()
}

#[command]
pub fn audio_pick_wav_path(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("WAV", &["wav"])
        .blocking_pick_file();
    Ok(picked.and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned()))
}

#[command]
pub fn audio_mic_start() -> Result<String, String> {
    crate::mic_capture::grant_and_start(qualia_audio::CapturePurpose::Analysis)
}

#[command]
pub fn audio_mic_stop() -> Result<String, String> {
    crate::mic_capture::stop_capture()
}

#[command]
pub fn audio_mic_status() -> Result<serde_json::Value, String> {
    crate::mic_capture::status_json()
}

#[command]
pub fn audio_ensure_weights(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
) -> Result<serde_json::Value, String> {
    let config = state.config.lock().unwrap().clone();
    let root = std::path::PathBuf::from(&config.storage_path);
    let aed = qualia_client_core::audio_pipeline::ensure_aed_weights(&root)?;
    let speech = qualia_client_core::audio_pipeline::ensure_speech_weights(&root)?;
    Ok(serde_json::json!({ "aed": aed, "speech": speech }))
}

#[command]
pub fn audio_daw_history_demo() -> Result<serde_json::Value, String> {
    qualia_client_core::audio_pipeline::daw_history_demo()
}

/// Pull mic ring → weighted AED (disk weights if present).
#[command]
pub fn audio_live_aed(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
) -> Result<serde_json::Value, String> {
    let config = state.config.lock().unwrap().clone();
    let root = std::path::PathBuf::from(&config.storage_path);
    let mono = crate::mic_capture::pull_mono(16_000)?;
    if mono.is_empty() {
        return Err("mic ring empty — press Mic start and speak, then retry".into());
    }
    let sr = crate::mic_capture::status_json()?
        .get("sample_rate")
        .and_then(|v| v.as_u64())
        .unwrap_or(16_000) as u32;
    let r = qualia_client_core::audio_pipeline::analyze_mono_pcm(&mono, sr, Some(&root))?;
    serde_json::to_value(r).map_err(|e| e.to_string())
}

#[command]
pub fn audio_speech_disk(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    supported: Option<bool>,
) -> Result<serde_json::Value, String> {
    let config = state.config.lock().unwrap().clone();
    let root = std::path::PathBuf::from(&config.storage_path);
    qualia_client_core::audio_pipeline::speech_from_disk(&root, supported.unwrap_or(true))
}

#[command]
pub fn vision_ensure_weights(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
) -> Result<String, String> {
    let config = state.config.lock().unwrap().clone();
    let root = std::path::PathBuf::from(&config.storage_path);
    qualia_client_core::vision_pipeline::ensure_vision_weights(&root)
}

#[command]
pub fn vision_detect_disk_weights_demo(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
) -> Result<serde_json::Value, String> {
    let config = state.config.lock().unwrap().clone();
    let root = std::path::PathBuf::from(&config.storage_path);
    let mut rgb = vec![0u8; 32 * 32 * 3];
    for p in rgb.chunks_mut(3) {
        p[0] = 220;
        p[1] = 30;
        p[2] = 30;
    }
    let r = qualia_client_core::vision_pipeline::detect_with_disk_weights(&root, &rgb, 32, 32)?;
    serde_json::to_value(r).map_err(|e| e.to_string())
}

#[command]
pub fn vision_twin_elasticity_demo() -> Result<serde_json::Value, String> {
    qualia_client_core::vision_pipeline::twin_elasticity_demo()
}

/// Enhance (classical super-resolution) an image; returns before/after PNG data URLs.
#[command]
pub fn vision_super_resolve(
    png_bytes: Vec<u8>,
    scale: u8,
    kernel: String,
    device: String,
) -> Result<serde_json::Value, String> {
    let prefer_gpu = device != "cpu";
    serde_json::to_value(qualia_client_core::vision_pipeline::super_resolve_image(
        &png_bytes, scale, &kernel, prefer_gpu,
    )?)
    .map_err(|e| e.to_string())
}

#[command]
pub fn audio_music_demo() -> Result<serde_json::Value, String> {
    qualia_client_core::audio_pipeline::music_analysis_demo()
}

#[command]
pub fn audio_daw_fx_demo() -> Result<serde_json::Value, String> {
    qualia_client_core::audio_pipeline::daw_fx_demo()
}

#[command]
pub fn audio_gen_demo() -> Result<serde_json::Value, String> {
    qualia_client_core::audio_pipeline::gen_audio_demo()
}

#[command]
pub fn audio_shared_clock_demo() -> Result<serde_json::Value, String> {
    qualia_client_core::audio_pipeline::shared_clock_demo()
}

#[command]
pub fn audio_mixer_default() -> Result<serde_json::Value, String> {
    Ok(qualia_client_core::audio_pipeline::mixer_default_session())
}

#[command]
pub fn audio_mixer_bounce(tracks: serde_json::Value) -> Result<serde_json::Value, String> {
    let tracks: Vec<qualia_client_core::audio_pipeline::MixerTrackDto> =
        serde_json::from_value(tracks).map_err(|e| format!("tracks json: {e}"))?;
    qualia_client_core::audio_pipeline::mixer_bounce(&tracks)
}

#[command]
pub fn audio_capabilities() -> Result<serde_json::Value, String> {
    serde_json::to_value(qualia_client_core::audio_pipeline::audio_capabilities())
        .map_err(|e| e.to_string())
}

/// Full generate → store → recon → OBJ/.10d continuum (pre-auditory handoff).
#[command]
pub fn vision_gs_continuum(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    prompt: Option<String>,
    seed: Option<u64>,
    steps: Option<u32>,
    media_time_ms: Option<u64>,
) -> Result<serde_json::Value, String> {
    let config = state.config.lock().unwrap().clone();
    let root = std::path::PathBuf::from(&config.storage_path);
    let r = qualia_client_core::vision_pipeline::run_gs_continuum(
        &root,
        prompt.as_deref().unwrap_or("vision continuum"),
        seed.unwrap_or(7),
        steps.unwrap_or(4),
        48,
        48,
        10,
        media_time_ms.unwrap_or(0),
    )?;
    serde_json::to_value(r).map_err(|e| e.to_string())
}

#[command]
pub fn vision_reject_instance(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    instance_hash_hex: String,
    human_did_hex: Option<String>,
    reason_hex: Option<String>,
) -> Result<serde_json::Value, String> {
    let config = state.config.lock().unwrap().clone();
    let root = std::path::PathBuf::from(&config.storage_path);
    let instance = parse_hex_u64(&instance_hash_hex)?;
    let human = human_did_hex
        .as_deref()
        .map(parse_hex_u64)
        .transpose()?
        .unwrap_or(qualia_core_db::q_hash("did:webizen:local-principal"));
    let reason = reason_hex
        .as_deref()
        .map(parse_hex_u64)
        .transpose()?
        .unwrap_or(0);
    qualia_client_core::vision_pipeline::reject_instance(&root, human, instance, reason)?;
    Ok(serde_json::json!({
        "ok": true,
        "action": "reject",
        "instance_hash": format!("0x{instance:016x}"),
        "note": "Machine observation retained; human reject edge appended."
    }))
}

#[command]
pub fn vision_correct_instance(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    instance_hash_hex: String,
    new_class_hash_hex: String,
    human_did_hex: Option<String>,
) -> Result<serde_json::Value, String> {
    let config = state.config.lock().unwrap().clone();
    let root = std::path::PathBuf::from(&config.storage_path);
    let instance = parse_hex_u64(&instance_hash_hex)?;
    let new_class = parse_hex_u64(&new_class_hash_hex)?;
    let human = human_did_hex
        .as_deref()
        .map(parse_hex_u64)
        .transpose()?
        .unwrap_or(qualia_core_db::q_hash("did:webizen:local-principal"));
    qualia_client_core::vision_pipeline::correct_instance(&root, human, instance, new_class)?;
    Ok(serde_json::json!({
        "ok": true,
        "action": "correct",
        "instance_hash": format!("0x{instance:016x}"),
        "new_class_hash": format!("0x{new_class:016x}"),
        "note": "Machine proposesClass retained; human correct edge appended."
    }))
}

