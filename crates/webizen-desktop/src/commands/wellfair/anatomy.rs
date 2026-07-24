#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Emitter, Manager, State};
use crate::render::AnatomyBodyState;

/// 3D Anatomy Qapp — compute a body-system view for a given audience ("person"/"clinician").
/// Read-only: no clinical data is persisted and no diagnosis is performed.
/// The returned view is educational context, not a diagnosis.
#[command]
pub fn wellfair_compute_anatomy_view(
    app: AppHandle,
    lens: String,
    threshold: Option<usize>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let report = host.compute_anatomy_view(&lens, threshold.unwrap_or(2))?;
        serde_json::to_string(&report).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_compute_scorecard(
    app: AppHandle,
    threshold: Option<usize>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let report = host.compute_scorecard(threshold.unwrap_or(2))?;
        serde_json::to_string(&report).map_err(|e| e.to_string())
    })?
}

/// The person's own score-card weight model (how their body is read) + the seed suggestion + whether they've
/// authored their own. Returns `{ model, seed, authored }`.
#[command]
pub fn wellfair_get_weight_model(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        serde_json::to_string(&serde_json::json!({
            "model": host.get_weight_model(),
            "seed": host.seed_weight_model(),
            "authored": host.weight_model_is_authored(),
        }))
        .map_err(|e| e.to_string())
    })?
}

/// Set the person's own weight model (JSON = `WeightModel`) — their authorship of how the card reads them.
#[command]
pub fn wellfair_set_weight_model(app: AppHandle, model_json: String) -> Result<String, String> {
    let model: wellfare_core::anatomy::WeightModel =
        serde_json::from_str(&model_json).map_err(|e| format!("invalid weight model JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.set_weight_model(&model)?;
        Ok("{\"set\":true}".into())
    })?
}

/// Reset the weight model to the seed suggestion (clears the person's authored model).
#[command]
pub fn wellfair_reset_weight_model(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.reset_weight_model()?;
        Ok("{\"reset\":true}".into())
    })?
}

// ── Physiological state (P6 — the reproductive-continuum declaration) ──────────────────────────
//
// The person's own statement of where they are on the reproductive continuum. Forum-internum /
// Sanctuary-class. The score-card is computed at this state so it reads them at their current life
// stage, not a neutral baseline.

/// The person's declared physiological state + whether they've declared one. Returns
/// `{ state, declared }`. `state` is the `PhysiologicalState` JSON (Baseline if not declared).
#[command]
pub fn wellfair_get_physiological_state(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        serde_json::to_string(&serde_json::json!({
            "state": host.get_physiological_state(),
            "declared": host.physiological_state_is_declared(),
        }))
        .map_err(|e| e.to_string())
    })?
}

/// Set the person's declared physiological state (JSON = `PhysiologicalState`) — their own statement of
/// where they are on the reproductive continuum. Forum-internum / Sanctuary-class.
#[command]
pub fn wellfair_set_physiological_state(app: AppHandle, state_json: String) -> Result<String, String> {
    let state: wellfare_core::anatomy::PhysiologicalState =
        serde_json::from_str(&state_json).map_err(|e| format!("invalid physiological state JSON: {e}"))?;
    let app_state = app.state::<HostApiState>();
    app_state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.set_physiological_state(&state)?;
        Ok("{\"set\":true}".into())
    })?
}

/// Clear the declared physiological state — revert to the implicit Baseline. Idempotent.
#[command]
pub fn wellfair_reset_physiological_state(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.reset_physiological_state()?;
        Ok("{\"reset\":true}".into())
    })?
}

// ── 3D Anatomy render surface (S5.7 — whole-body percept snapshot) ────────────────────────────

/// Render the whole-body 3D Anatomy snapshot to a PNG (headless GPU via `webizen_render`), coloured by
/// the person's accumulated burden at their declared physiological state. The orbit camera is driven by
/// `azimuth` (0..360°) and `elevation` (-90..90°). The PNG is stored in [`AnatomyBodyState`] and served
/// at `webizen://localhost/anatomy/body.png`; the Studio UI bumps its epoch query-string to refetch.
/// Returns `{ "ok": true, "bytes": <len> }` on success.
#[command]
pub async fn wellfair_render_body_snapshot(
    app: AppHandle,
    azimuth: Option<f64>,
    elevation: Option<f64>,
    state: State<'_, AnatomyBodyState>,
    host_state: State<'_, HostApiState>,
) -> Result<String, String> {
    let az = azimuth.unwrap_or(0.0);
    let el = elevation.unwrap_or(10.0);
    // Compute the scene while holding the host lock, then drop the guard before the await so the
    // future stays `Send` (the MutexGuard is not Send).
    let scene = {
        host_state.0.execute_sync(move |guard| {
            let host = guard
                .as_ref()
                .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
            host.compute_body_scene(az, el).map_err(|e| e.to_string())
        })??
    };

    let png = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        webizen_render::render_scene_png(&scene, 960, 540)
            .ok_or_else(|| "GPU render_scene_png returned no frame".to_string())
    })
    .await
    .map_err(|e| format!("render task join failed: {e}"))??;

    let len = png.len();
    if let Ok(mut slot) = state.png.lock() {
        *slot = png;
    }
    let _ = app.emit("anatomy-body-ready", ());
    serde_json::to_string(&serde_json::json!({ "ok": true, "bytes": len }))
        .map_err(|e| e.to_string())
}

// ── 3D Anatomy asset cache (S5.8 — user-triggered real-mesh acquisition) ───────────────────────

/// Whether the body assets for a model are cached + complete. `model` = `"male"` / `"female"`.
/// Returns `{ model, cached, organ_count, total_ten_d_bytes, acquired_at_unix }`.
#[command]
pub fn wellfair_body_assets_status(
    app: AppHandle,
    model: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let status = host.body_assets_status(&model)?;
        serde_json::to_string(&status).map_err(|e| e.to_string())
    })?
}

/// Acquire (download + compile + cache) the body assets for a model — **user-triggered**. Discovers the
/// reference-organ manifest from the HRA SPARQL endpoint, fetches each GLB, compiles to `.10d`, caches
/// both + a manifest. Emits `anatomy-acquire-progress` per organ and `anatomy-acquire-done` at the end.
/// Returns the final `AcquireReport` JSON. Blocking network I/O runs on `spawn_blocking`.
#[command]
pub async fn wellfair_acquire_body_assets(
    app: AppHandle,
    model: String,
    host_state: State<'_, HostApiState>,
) -> Result<String, String> {
    // Resolve the model + storage_root while holding the lock, then drop the guard before the await.
    let (model_enum, storage_root) = {
        host_state.0.execute_sync(move |guard| {
            let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
            let m = qualia_client_core::wellfair::api::parse_anatomy_model(&model).map_err(|e| e.to_string())?;
            Ok::<_, String>((m, host.storage_root().to_path_buf()))
        })??
    };

    let app_for_progress = app.clone();
    let report = tokio::task::spawn_blocking(move || -> Result<qualia_client_core::wellfair::anatomy_assets::AcquireReport, String> {
        qualia_client_core::wellfair::anatomy_assets::acquire_body_assets(
            &storage_root,
            model_enum,
            |p| {
                let _ = app_for_progress.emit("anatomy-acquire-progress", &p);
            },
        )
    })
    .await
    .map_err(|e| format!("acquire task join failed: {e}"))??;

    let _ = app.emit("anatomy-acquire-done", &report);
    serde_json::to_string(&report).map_err(|e| e.to_string())
}

/// Load a cached `.10d` for one organ. Returns the raw container bytes as a Tauri IPC byte response
/// (the browser portal's `load_10d_colored` consumes them). `model` = `"male"` / `"female"`.
#[command]
pub fn wellfair_load_cached_organ_10d(
    app: AppHandle,
    model: String,
    organ_key: String,
) -> Result<Vec<u8>, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.load_cached_organ_10d(&model, &organ_key)
    })?
}

/// The per-organ dual-modality percepts for the cached organ set — so the browser portal knows what
/// colour to paint each organ (σ → RGBA via `paint_organs`). Returns `{ painted, unmapped }`.
#[command]
pub fn wellfair_cached_body_organ_percepts(
    app: AppHandle,
    model: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let (painted, unmapped) = host.cached_body_organ_percepts(&model)?;
        serde_json::to_string(&serde_json::json!({ "painted": painted, "unmapped": unmapped }))
            .map_err(|e| e.to_string())
    })?
}

/// Clear the cache for a model (idempotent). The person can re-acquire later.
#[command]
pub fn wellfair_clear_body_cache(
    app: AppHandle,
    model: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.clear_body_cache(&model)?;
        Ok("{\"ok\":true}".into())
    })?
}

