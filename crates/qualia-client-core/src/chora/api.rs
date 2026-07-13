//! Canvas / Chora host API methods for [`WebizenHostApi`].

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::Signer;
use qualia_core_db::domains::geospatial::canvas_query::query_region;
use qualia_core_db::domains::geospatial::render_surface::{
    build_surface_descriptor, RenderSurfaceDescriptor, BACKEND_WEBGPU,
};
use qualia_core_db::domains::geospatial::spatial::{SpatialElement, SpatiotemporalQuadTree};
use qualia_core_db::domains::geospatial::spatial_sync::{
    merge_planted_assets, PlantedAsset, SpatialSyncCell,
};
use qualia_core_db::query::spawn_decay::spawn_decay_alpha;

use crate::canvas_state;
use crate::canvas_store;
use crate::canvas_world;
use crate::wellfair::api::WebizenHostApi;
use crate::wellfair::blob_store::BlobStore;
use crate::wellfair::sync_protocol::SyncOperation;

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl WebizenHostApi {
    /// Set temporal slice for time-travel navigation (P3).
    pub fn set_temporal_slice(&self, t_value: f64) -> Result<(), String> {
        let storage_root = self.chora_storage_root();
        let mut state = canvas_state::load(storage_root);
        state.temporal_t = t_value.max(0.0) as u64;
        canvas_state::save(storage_root, &state).map_err(|e| e.to_string())
    }

    /// Current temporal scrub position (unix seconds).
    pub fn get_temporal_slice(&self) -> u64 {
        canvas_state::load(self.chora_storage_root()).temporal_t
    }

    fn canvas_store(&self) -> Result<canvas_store::CanvasWorldStore, String> {
        canvas_store::CanvasWorldStore::open(self.chora_storage_root()).map_err(|e| e.to_string())
    }

    /// List all saved canvas world configurations.
    pub fn list_canvas_worlds(&self) -> Result<Vec<serde_json::Value>, String> {
        let store = self.canvas_store()?;
        let worlds = store.list().map_err(|e| e.to_string())?;
        Ok(worlds
            .iter()
            .map(|w| {
                serde_json::json!({
                    "id": w.id,
                    "title": w.title,
                    "layerCount": w.layer_stack.len(),
                    "assetCount": w.assets.len(),
                    "temporalRange": w.temporal_range,
                    "origin": { "lat": w.origin_lat, "lon": w.origin_lon },
                })
            })
            .collect())
    }

    /// Load one world config by id.
    pub fn get_canvas_world(&self, world_id: &str) -> Result<serde_json::Value, String> {
        let store = self.canvas_store()?;
        let world = store
            .get(world_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("world not found: {world_id}"))?;
        serde_json::to_value(&world).map_err(|e| e.to_string())
    }

    /// Save (upsert) a world configuration.
    pub fn save_canvas_world(&self, config_json: &str) -> Result<(), String> {
        let config: canvas_world::CanvasWorldConfig =
            serde_json::from_str(config_json).map_err(|e| e.to_string())?;
        self.canvas_store()?
            .upsert(config, now_unix())
            .map_err(|e| e.to_string())
    }

    /// Remove a world by id.
    pub fn delete_canvas_world(&self, world_id: &str) -> Result<bool, String> {
        self.canvas_store()?
            .remove(world_id)
            .map_err(|e| e.to_string())
    }

    /// Seed the P0 demo world if the store is empty.
    pub fn seed_canvas_demo(&self) -> Result<bool, String> {
        self.canvas_store()?
            .seed_if_empty(now_unix())
            .map_err(|e| e.to_string())
    }

    /// Seed P8 flagship canvas worlds (history, biosphere, council, SDG, GLAM) when absent.
    pub fn seed_flagship_worlds(&self) -> Result<u32, String> {
        let seeded = super::seed_all_flagships(&self.canvas_store()?, now_unix())
            .map_err(|e| e.to_string())?;
        Ok(seeded as u32)
    }

    /// Set the active world for Chora navigation.
    pub fn set_active_canvas_world(&self, world_id: &str) -> Result<(), String> {
        let store = self.canvas_store()?;
        if store.get(world_id).map_err(|e| e.to_string())?.is_none() {
            return Err(format!("world not found: {world_id}"));
        }
        let storage_root = self.chora_storage_root();
        let mut state = canvas_state::load(storage_root);
        state.active_world_id = world_id.to_string();
        canvas_state::save(storage_root, &state).map_err(|e| e.to_string())
    }

    /// Active world id + temporal scrub state.
    pub fn canvas_navigation_state(&self) -> serde_json::Value {
        let state = canvas_state::load(self.chora_storage_root());
        serde_json::json!({
            "activeWorldId": state.active_world_id,
            "temporalT": state.temporal_t,
            "rampSecs": state.ramp_secs,
        })
    }

    /// Query assets visible in a bbox at the current temporal slice (P4 entry).
    pub fn query_canvas_region(
        &self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    ) -> Result<Vec<serde_json::Value>, String> {
        let nav = canvas_state::load(self.chora_storage_root());
        let store = self.canvas_store()?;
        let world = store
            .get(&nav.active_world_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("active world not found: {}", nav.active_world_id))?;

        let mut tree = SpatiotemporalQuadTree::new((-180.0, -90.0, 180.0, 90.0));
        for (i, asset) in world.assets.iter().enumerate() {
            let vf = asset.valid_from.unwrap_or(0);
            let vu = asset.valid_until.unwrap_or(u64::MAX);
            let alpha = spawn_decay_alpha(
                nav.temporal_t,
                vf,
                asset.valid_until,
                nav.ramp_secs,
                nav.ramp_secs,
            );
            if alpha <= 0.0 {
                continue;
            }
            let (lat, lon) = match (asset.lat, asset.lon) {
                (Some(la), Some(lo)) => (la, lo),
                _ => (world.origin_lat, world.origin_lon),
            };
            let bounds = (lon - 0.01, lat - 0.01, lon + 0.01, lat + 0.01);
            tree.insert(SpatialElement {
                h3_index: i as u64,
                bounds,
                t0: vf,
                t1: vu,
            });
        }

        let ramp = nav.ramp_secs;
        let t0 = nav.temporal_t.saturating_sub(ramp);
        let t1 = nav.temporal_t.saturating_add(ramp);
        let hits = query_region(&tree, (x1, y1, x2, y2), (t0, t1));

        Ok(hits
            .iter()
            .filter_map(|&idx| world.assets.get(idx as usize))
            .map(|a| {
                let vf = a.valid_from.unwrap_or(0);
                let alpha = spawn_decay_alpha(
                    nav.temporal_t,
                    vf,
                    a.valid_until,
                    nav.ramp_secs,
                    nav.ramp_secs,
                );
                serde_json::json!({
                    "assetId": a.asset_id,
                    "licence": a.licence,
                    "lat": a.lat,
                    "lon": a.lon,
                    "alpha": alpha,
                })
            })
            .collect())
    }

    /// Request asset bytes by content hash, local path, or urn (Phase 6).
    ///
    /// - 64-char hex hash → content-addressed blob store lookup
    /// - `local://…` or filesystem path → read from storage root
    /// - `urn:…` → not yet implemented (honest error, no fabricated bytes)
    pub fn request_asset_stream(&self, asset_id: &str) -> Result<Vec<u8>, String> {
        let asset_id = asset_id.trim();
        if asset_id.is_empty() {
            return Err("asset_id must not be empty".into());
        }

        if asset_id.starts_with("urn:") {
            return Err(format!(
                "remote urn asset fetch is not implemented offline: {asset_id}"
            ));
        }

        let storage_root = self.chora_storage_root();
        if asset_id.len() == 64 && asset_id.bytes().all(|b| b.is_ascii_hexdigit()) {
            let store = BlobStore::open(storage_root).map_err(|e| e.to_string())?;
            return store
                .get(asset_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("blob not found for hash {asset_id}"));
        }

        let path = if asset_id.starts_with("local://") {
            let rel = asset_id.trim_start_matches("local://");
            storage_root.join("wellfair/canvas").join(rel)
        } else if Path::new(asset_id).is_absolute() {
            Path::new(asset_id).to_path_buf()
        } else {
            storage_root.join(asset_id)
        };

        std::fs::read(&path).map_err(|e| format!("failed to read asset '{asset_id}': {e}"))
    }

    /// Expose a WebGPU/canvas proxy surface configuration to the qapp context (Phase 6).
    pub fn get_render_surface(&self) -> Result<String, String> {
        const DEFAULT_WIDTH: u32 = 1280;
        const DEFAULT_HEIGHT: u32 = 720;

        let storage_root = self.chora_storage_root();
        let nav = canvas_state::load(storage_root);
        let store = self.canvas_store()?;
        let world = store
            .get(&nav.active_world_id)
            .map_err(|e| e.to_string())?
            .unwrap_or_else(canvas_world::CanvasWorldConfig::default);

        let desc: RenderSurfaceDescriptor = build_surface_descriptor(
            DEFAULT_WIDTH,
            DEFAULT_HEIGHT,
            BACKEND_WEBGPU,
            nav.temporal_t,
            &nav.active_world_id,
            world.origin_lat,
            world.origin_lon,
        )?;

        serde_json::to_string(&desc).map_err(|e| e.to_string())
    }

    /// Publish a planted asset to the spatial sync protocol (Phase 7)
    pub fn publish_planted_asset(&self, asset: PlantedAsset) -> Result<(), String> {
        let payload_summary = serde_json::to_string(&asset).map_err(|e| e.to_string())?;
        let op = SyncOperation::new(
            uuid::Uuid::new_v4().to_string(),
            format!("urn:qualia:spatial_plant:{}", asset.asset_id),
            "spatial_plant",
            self.chora_owner_did().to_string(),
            "Public",
            payload_summary,
            asset.lamport,
            now_unix() as u32,
        );
        let signature = self.chora_signing_key().sign(&op.signing_payload());
        let signed_op = op.with_signature(hex::encode(signature.to_bytes()));

        // Submit directly to local inbox; daemon handles outbox/relay
        self.admit_sync_operation(&signed_op).map(|_| ())
    }

    /// Pull and merge planted assets for a specific spatial cell (Phase 7)
    pub fn pull_spatial_assets(&self, cell_id: u64) -> Result<Vec<PlantedAsset>, String> {
        let ops = self.validated_sync_operations()?;
        let mut local_assets = Vec::new();

        for op in ops {
            if op.kind == "spatial_plant" {
                if let Ok(asset) = serde_json::from_str::<PlantedAsset>(&op.payload_summary) {
                    if asset.cell_id == cell_id {
                        local_assets.push(asset);
                    }
                }
            }
        }

        // Use merge_planted_assets to resolve overlapping operations using LWW
        // Remote is empty here because the inbox already contains both local and remote operations
        // and we just need to reduce them to the final active state.
        let final_assets = merge_planted_assets(&local_assets, &[]);

        // Filter out deleted tombstones
        Ok(final_assets.into_iter().filter(|a| !a.deleted).collect())
    }

    /// Pull spatial assets for a cell and return merged state as JSON (Phase 7).
    pub fn sync_cell(&self, cell_id: u64) -> Result<String, String> {
        let assets = self.pull_spatial_assets(cell_id)?;
        let cell = SpatialSyncCell::from_assets(cell_id, &assets);
        serde_json::to_string(&cell).map_err(|e| e.to_string())
    }
}