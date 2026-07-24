#![allow(non_snake_case)]

use super::super::*;
use super::*;
use tauri::{command, AppHandle, Manager};

// --- Chora spatio-temporal canvas ---

#[command]
pub fn chora_list_worlds(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        serde_json::to_string(&host.list_canvas_worlds()?).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn chora_get_world(app: AppHandle, world_id: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let v = host.get_canvas_world(&world_id)?;
        serde_json::to_string(&v).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn chora_save_world(app: AppHandle, config_json: String) -> Result<(), String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.save_canvas_world(&config_json)
    })?
}

#[command]
pub fn chora_delete_world(app: AppHandle, world_id: String) -> Result<bool, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.delete_canvas_world(&world_id)
    })?
}

#[command]
pub fn chora_seed_demo(app: AppHandle) -> Result<bool, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.seed_canvas_demo()
    })?
}

/// Seed P8 flagship Chora worlds when missing (history, biosphere, council, SDG, GLAM).
#[command]
pub fn chora_seed_flagships(app: AppHandle) -> Result<u32, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.seed_flagship_worlds()
    })?
}

#[command]
pub fn chora_navigation(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        Ok(host.canvas_navigation_state().to_string())
    })?
}

#[command]
pub fn chora_set_temporal(app: AppHandle, t_value: f64) -> Result<(), String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.set_temporal_slice(t_value)
    })?
}

#[command]
pub fn chora_set_active_world(app: AppHandle, world_id: String) -> Result<(), String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.set_active_canvas_world(&world_id)
    })?
}

#[command]
pub fn chora_query_region(
    app: AppHandle,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let hits = host.query_canvas_region(x1, y1, x2, y2)?;
        serde_json::to_string(&hits).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn chora_publish_asset(app: AppHandle, asset_json: String) -> Result<(), String> {
    use qualia_core_db::domains::geospatial::spatial_sync::PlantedAsset;

    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let asset: PlantedAsset = serde_json::from_str(&asset_json).map_err(|e| e.to_string())?;
        host.publish_planted_asset(asset)
    })?
}

#[command]
pub fn chora_pull_assets(app: AppHandle, cell_id: u64) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let assets = host.pull_spatial_assets(cell_id)?;
        serde_json::to_string(&assets).map_err(|e| e.to_string())
    })?
}

// --- Chora layer library + asset download pipeline ---

#[command]
pub fn chora_list_layers() -> Result<String, String> {
    let catalog = qualia_client_core::chora::layers::LAYER_CATALOG;
    serde_json::to_string(catalog).map_err(|e| e.to_string())
}

#[command]
pub fn chora_list_categories() -> Result<String, String> {
    let cats = qualia_client_core::chora::layers::all_categories();
    serde_json::to_string(&cats).map_err(|e| e.to_string())
}

#[command]
pub fn chora_get_layer(layer_id: String) -> Result<String, String> {
    let layer = qualia_client_core::chora::layers::find_layer(&layer_id)
        .ok_or_else(|| format!("Layer not found: {layer_id}"))?;
    serde_json::to_string(layer).map_err(|e| e.to_string())
}

#[command]
pub async fn chora_download_layer(
    app: AppHandle,
    layer_id: String,
    resolution: u32,
) -> Result<String, String> {
    let asset = qualia_client_core::chora::asset_pipeline::download_and_compile_layer(&layer_id, resolution)
        .await?;

    if let Some(surface) = app.try_state::<std::sync::Arc<NativeSurfaceState>>() {
        let mut renderer_guard = surface.renderer.lock().map_err(|e| e.to_string())?;
        if let Some(renderer) = renderer_guard.as_mut() {
            let _ = renderer.upload_mesh_colored(
                &asset.positions,
                &asset.colors,
                &asset.indices,
            );
        }
    }

    let result = serde_json::json!({
        "layerId": asset.layer_id,
        "vertexCount": asset.vertex_count,
        "triangleCount": asset.triangle_count,
        "sourceFormat": asset.source_format,
        "license": asset.license,
        "container10dSize": asset.container_10d.len(),
    });
    serde_json::to_string(&result).map_err(|e| e.to_string())
}

#[command]
pub fn chora_load_layer_to_gpu(
    app: AppHandle,
    layer_id: String,
    resolution: u32,
) -> Result<String, String> {
    let _ = resolution;
    let _ = layer_id;
    let _ = app;
    Err("Use chora_download_layer for async download+compile+upload".to_string())
}

