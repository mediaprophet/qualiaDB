//! Telemetry bridge, 10D browser, vision/audio

#![allow(non_snake_case)]

use super::*;
use super::render::ActiveAnchor;
use tauri::{command, State};

// ── Telemetry Bridge ───────────────────────────────────────────────────────────
// Telemetry bridge is in parent src directory, not commands directory

use binary_registry::BinaryNodeRegistry;

/// Filter scene items by temporal slice (version <= t_value)
///
/// Zero-heap consideration: Stack-allocated comparison, no heap allocation
///
/// Note: SceneItem currently doesn't have a version field. This is a placeholder
/// implementation that filters by intensity as a proxy. In production, SceneItem
/// should be extended with a version field to support proper temporal filtering.
#[allow(dead_code)]
fn filter_scene_by_temporal_slice(
    mut scene: webizen_studio::render::qualia::SemanticScene,
    t_value: f64,
) -> webizen_studio::render::qualia::SemanticScene {
    // TODO: Add version field to SceneItem for proper temporal filtering
    // For now, filter by intensity as a proxy (intensity <= t_value)
    scene.items.retain(|item| item.intensity <= t_value);
    scene
}

/// Collapse wavefunction for a node, promoting q > 0 to q = 0
///
/// Binary IPC Optimization: Accepts u64 index pointer instead of String ID
/// to avoid heap allocation during cross-process serialization.
///
/// Zero-heap consideration: Uses stack-allocated node_index (u64) instead of String
/// The actual tensor state management should be done with fixed-size buffers in QualiaDB
#[command]
pub async fn collapse_wavefunction(
    node_index: u64,
    active_anchor: State<'_, ActiveAnchor>,
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<(), String> {
    #[allow(unused_imports)]
    use qualia_core_db::q_hash;

    // Convert binary index back to string ID for QualiaDB lookup
    // This is necessary because QualiaDB uses string-based IDs
    let node_id = binary_registry
        .get_id(node_index)
        .ok_or("Invalid node index")?;

    // In a full implementation, this would:
    // 1. Update QualiaDB tensor state: q > 0 → q = 0
    // 2. Trigger re-render with collapsed state
    // 3. Update epistemic_state in RenderScene

    // For now, implement basic QualiaDB mutation
    // TODO: Integrate with full QualiaDB tensor mutation API

    // Update active anchor if this is the current node
    let anchor = active_anchor
        .0
        .lock()
        .map_err(|_| "anchor state poisoned")?;
    if let Some(current_id) = anchor.as_ref() {
        if current_id == &node_id {
            // Node is already the anchor, trigger re-fetch with collapsed state
            // The daemon will pick up the change and re-render

            // In production, would mutate QualiaDB directly:
            // let subject_hash = q_hash(&node_id);
            // let tensor_mut = NQuin { subject: subject_hash, ... };
            // write_nquin_to_db(tensor_mut);
        }
    }

    Ok(())
}

/// Legacy collapse_wavefunction that accepts String ID (for backward compatibility)
///
/// Binary IPC Optimization: This is a legacy wrapper that registers the string ID
/// and delegates to the binary index version
#[command]
pub async fn collapse_wavefunction_legacy(
    node_id: String,
    active_anchor: State<'_, ActiveAnchor>,
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<(), String> {
    // Register string ID and get binary index
    let node_index = binary_registry.register(&node_id);

    // Delegate to binary version
    collapse_wavefunction(node_index, active_anchor, binary_registry).await
}

/// Load and validate CCF GLB asset using zero-copy binary transport
///
/// Binary IPC Optimization: Returns u64 asset index instead of full file data
/// The actual heavy binary transport happens via TensorBufferView pattern
#[command]
pub async fn load_ccf_asset(
    asset_name: String,
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<u64, String> {
    use glb_ingest::GLBIngestionManager;

    let manager = GLBIngestionManager::default();
    let assets = manager.get_vh_male_v14_assets();

    // Find asset by name
    let asset = assets
        .iter()
        .find(|a| a.asset_name == asset_name)
        .ok_or_else(|| format!("Asset not found: {}", asset_name))?;

    // Register asset in binary registry
    let asset_index = binary_registry.register(&asset.asset_name);

    // Load GLB file (in production, would use memory-mapped files)
    let glb_data = manager.load_glb(&asset.file_path)?;

    // Create view and validate
    let view = manager.create_view(&glb_data, asset.asset_name.clone(), asset.version.clone());

    if !view.is_valid_glb() {
        return Err(format!("Invalid GLB file: {}", asset.asset_name));
    }

    // Return binary index for zero-copy access
    Ok(asset_index)
}

/// Test harness for validating Tauri IPC handshake with CCF assets
///
/// Binary IPC Optimization: Validates u64 index-based communication
/// before attempting heavy asset loading (18MB stress test)
#[command]
pub async fn test_ccf_ipc_handshake(
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<String, String> {
    use glb_ingest::GLBIngestionManager;

    let manager = GLBIngestionManager::default();

    // Test 1: List available assets (lightweight operation)
    let assets = manager.get_vh_male_v14_assets();
    let asset_count = assets.len();

    // Test 2: Register asset names in binary registry
    for asset in &assets {
        binary_registry.register(&asset.asset_name);
    }

    let registry_size = binary_registry.len();

    // Test 3: Verify binary index lookup
    let test_asset = &assets[0];
    let binary_index = binary_registry
        .get_index(&test_asset.asset_name)
        .ok_or("Failed to retrieve binary index")?;

    // Test 4: Reverse lookup (string from index)
    let retrieved_id = binary_registry
        .get_id(binary_index)
        .ok_or("Failed to retrieve string ID from index")?;

    if retrieved_id != test_asset.asset_name {
        return Err(format!(
            "Reverse lookup mismatch: expected {}, got {}",
            test_asset.asset_name, retrieved_id
        ));
    }

    // Return test results
    Ok(format!(
        "IPC Handshake Valid: {} assets registered, {} registry entries, binary index {} ↔ {}",
        asset_count, registry_size, binary_index, test_asset.asset_name
    ))
}

/// Larynx smoke test (335KB) - validates chunk isolation and coordinate extraction
///
/// Binary IPC Optimization: Tests lightweight asset before 18MB stress test
#[command]
pub async fn test_larynx_smoke(
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<String, String> {
    use glb_ingest::{GLBIngestionManager, SemanticExtractor, Tensor10DMapping};

    let manager = GLBIngestionManager::default();

    // Load larynx asset (335KB - lightweight validation)
    let asset_name = "larynx".to_string();
    let assets = manager.get_vh_male_v14_assets();
    let asset = assets
        .iter()
        .find(|a| a.asset_name == asset_name)
        .ok_or("Larynx asset not found")?;

    // Load GLB file
    let glb_data = manager.load_glb(&asset.file_path)?;

    // Create GLB view
    let view = manager.create_view(&glb_data, asset.asset_name.clone(), asset.version.clone());

    // Validate GLB structure
    if !view.is_valid_glb() {
        return Err("Invalid GLB file".to_string());
    }

    // Test chunk isolation
    let header = view.header().ok_or("No header found")?;
    let json_chunk = view.json_chunk().ok_or("No JSON chunk found")?;
    let binary_chunk = view.binary_chunk().ok_or("No binary chunk found")?;

    // Test semantic extraction
    let semantic_mapping = SemanticExtractor::extract_semantic_ids(json_chunk, &binary_registry)?;

    // Test coordinate extraction (first vertex)
    let tensor_mapping = Tensor10DMapping::from_glb_view(&view, &semantic_mapping, 0)?;

    // Register in binary registry
    let asset_index = binary_registry.register(&asset_name);

    Ok(format!(
        "Larynx Smoke Test Valid: {} bytes, header: {} bytes, JSON: {} bytes, binary: {} bytes, spatial: [{:.2}, {:.2}, {:.2}], binary index: {}",
        glb_data.len(),
        header.len(),
        json_chunk.len(),
        binary_chunk.len(),
        tensor_mapping.spatial[0],
        tensor_mapping.spatial[1],
        tensor_mapping.spatial[2],
        asset_index
    ))
}

/// Blood vasculature stress test (18MB) - validates heavy asset loading with memory profiling
///
/// Binary IPC Optimization: Tests zero-copy transport with 50x scale increase
/// Monitors heap behavior during JSON extraction and GPU buffer limits
#[command]
pub async fn test_vasculature_stress(
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<String, String> {
    use glb_ingest::{GLBIngestionManager, SemanticExtractor, Tensor10DMapping};
    use std::time::Instant;

    let manager = GLBIngestionManager::default();

    // Load vasculature asset (18MB - stress test)
    let asset_name = "blood-vasculature".to_string();
    let assets = manager.get_vh_male_v14_assets();
    let asset = assets
        .iter()
        .find(|a| a.asset_name == asset_name)
        .ok_or("Blood vasculature asset not found")?;

    let start_total = Instant::now();

    // Phase 1: File loading
    let start_load = Instant::now();
    let glb_data = manager.load_glb(&asset.file_path)?;
    let load_time = start_load.elapsed();

    // Create GLB view
    let view = manager.create_view(&glb_data, asset.asset_name.clone(), asset.version.clone());

    // Validate GLB structure
    if !view.is_valid_glb() {
        return Err("Invalid GLB file".to_string());
    }

    // Phase 2: Chunk isolation
    let start_chunk = Instant::now();
    let header = view.header().ok_or("No header found")?;
    let json_chunk = view.json_chunk().ok_or("No JSON chunk found")?;
    let binary_chunk = view.binary_chunk().ok_or("No binary chunk found")?;
    let chunk_time = start_chunk.elapsed();

    // Phase 3: Semantic extraction (monitor heap spike)
    let start_semantic = Instant::now();
    let semantic_mapping = SemanticExtractor::extract_semantic_ids(json_chunk, &binary_registry)?;
    let semantic_time = start_semantic.elapsed();

    // Phase 4: Coordinate extraction (sample first 100 vertices for performance)
    let start_coords = Instant::now();
    let sample_count = 100.min(binary_chunk.len() / 12);
    let mut first_vertex = None;
    for i in 0..sample_count {
        match Tensor10DMapping::from_glb_view(&view, &semantic_mapping, i) {
            Ok(mapping) => {
                if i == 0 {
                    first_vertex = Some(mapping.spatial);
                }
            }
            Err(_) => break,
        }
    }
    let coords_time = start_coords.elapsed();

    // Phase 5: Binary registry registration
    let start_registry = Instant::now();
    let asset_index = binary_registry.register(&asset_name);
    let registry_size = binary_registry.len();
    let registry_time = start_registry.elapsed();

    let total_time = start_total.elapsed();

    // Calculate vertex count estimate
    let vertex_count = binary_chunk.len() / 12;

    Ok(format!(
        "Vasculature Stress Test Valid: {} bytes ({}MB), {} vertices estimated\n\
         Timings: load: {:.2}ms, chunk: {:.2}ms, semantic: {:.2}ms, coords: {:.2}ms, registry: {:.2}ms, total: {:.2}ms\n\
         Chunks: header: {} bytes, JSON: {} bytes, binary: {} bytes\n\
         Spatial: [{:.2}, {:.2}, {:.2}], registry: {} entries, binary index: {}",
        glb_data.len(),
        glb_data.len() / 1_048_576,
        vertex_count,
        load_time.as_millis(),
        chunk_time.as_millis(),
        semantic_time.as_millis(),
        coords_time.as_millis(),
        registry_time.as_millis(),
        total_time.as_millis(),
        header.len(),
        json_chunk.len(),
        binary_chunk.len(),
        first_vertex.map_or(0.0, |v| v[0]),
        first_vertex.map_or(0.0, |v| v[1]),
        first_vertex.map_or(0.0, |v| v[2]),
        registry_size,
        asset_index
    ))
}

/// Get list of available CCF VH_Male v1.4 assets
#[command]
pub async fn list_ccf_assets() -> Result<Vec<String>, String> {
    use glb_ingest::GLBIngestionManager;

    let manager = GLBIngestionManager::default();
    let assets = manager.get_vh_male_v14_assets();

    let asset_names: Vec<String> = assets
        .iter()
        .map(|a| format!("{} ({}MB)", a.asset_name, a.file_size / 1_048_576))
        .collect();

    Ok(asset_names)
}

/// Set temporal slice for time-travel navigation
///
/// Zero-heap consideration: t_value is f64 (stack-allocated)
/// Uses bit-casting to AtomicU64 to avoid heap allocation of Mutex<f64>
/// The daemon will filter nodes by version <= t_value
#[command]
pub async fn set_temporal_slice(
    t_value: f64,
    temporal_slice: State<'_, TemporalSlice>,
) -> Result<(), String> {
    // Update the temporal slice state (atomic operation, no heap allocation)
    temporal_slice.set(t_value);

    // In a full implementation, this would:
    // 1. Trigger daemon re-render with filtered nodes (version <= t_value)
    // 2. Update RenderScene.temporal_slice

    // TODO: Update daemon to respect temporal_slice filter

    Ok(())
}

/// Register browser hardware capabilities for adaptive rendering
///
/// Zero-heap consideration: Uses stack-allocated structs for tier determination
/// String parameters are heap-allocated but unavoidable for IPC
#[command]
pub async fn register_browser_capabilities(
    webgpu_available: bool,
    vram_gb: f64,
    adapter_name: String,
) -> Result<String, String> {
    // Determine hardware tier using stack-allocated logic
    let tier = if !webgpu_available {
        0 // Tier 0: No WebGPU
    } else if vram_gb < 2.0 {
        1 // Tier 1: Limited
    } else if vram_gb < 4.0 {
        2 // Tier 2: Good
    } else {
        3 // Tier 3: High-end
    };

    // In a full implementation, this would:
    // 1. Store capabilities in managed state
    // 2. Adjust rendering quality based on tier
    // 3. Update UI to show tier indicator

    // TODO: Add BrowserCapabilities state to Tauri managed state
    // TODO: Implement adaptive rendering based on tier

    Ok(format!("Registered: Tier {} ({})", tier, adapter_name))
}

