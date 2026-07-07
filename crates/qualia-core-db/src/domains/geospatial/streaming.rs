use std::collections::{HashMap, HashSet};

/// Camera pose for frustum-based tile prediction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraPose {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f64,
    pub pitch: f64,
}

/// VRAM / residency budget for a single streaming frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamBudget {
    pub max_tiles: usize,
    pub timestamp: u64,
}

/// Output of the scene streaming planner for one camera step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamPlan {
    /// Tiles that must be fetched into VRAM (not yet resident).
    pub required_tiles: Vec<u64>,
    /// Tiles to evict to honour the budget after loading required tiles.
    pub evict_tiles: Vec<u64>,
}

/// Combines tile pyramid indexing, frustum prediction, and VRAM eviction.
pub struct SceneStreamingPlanner {
    pub pyramid: TilePyramid,
    pub predictor: FrustumPredictor,
    pub eviction: VramBudgetEviction,
    /// LOD level consulted when resolving predicted tiles.
    pub active_lod: u8,
}

impl SceneStreamingPlanner {
    pub fn new(
        pyramid: TilePyramid,
        predictor: FrustumPredictor,
        max_tiles: usize,
        active_lod: u8,
    ) -> Self {
        Self {
            pyramid,
            predictor,
            eviction: VramBudgetEviction::new(max_tiles),
            active_lod,
        }
    }

    /// Plan which tiles to load and evict for the given camera pose and budget.
    pub fn plan_stream(&mut self, camera_pose: &CameraPose, budget: &StreamBudget) -> StreamPlan {
        self.eviction.max_tiles = budget.max_tiles;

        let predicted = self.predictor.predict_intersecting_tiles(
            camera_pose.x,
            camera_pose.y,
            camera_pose.z,
            camera_pose.yaw,
            camera_pose.pitch,
        );

        let mut required_tiles = Vec::new();
        for tile_id in predicted {
            if !self.pyramid.has_tile(self.active_lod, tile_id) {
                continue;
            }
            if !self.eviction.active_tiles.contains_key(&tile_id) {
                required_tiles.push(tile_id);
            }
            self.eviction.access_tile(tile_id, budget.timestamp);
        }

        required_tiles.sort_unstable();
        required_tiles.dedup();

        let mut evict_tiles = self.eviction.enforce_budget();
        evict_tiles.sort_unstable();

        StreamPlan {
            required_tiles,
            evict_tiles,
        }
    }
}

/// Predicts the next required spatial tiles given a camera pose.
/// Used for streaming prediction in Phase 6.
pub struct FrustumPredictor {
    pub fov_degrees: f64,
    pub aspect_ratio: f64,
    pub near_clip: f64,
    pub far_clip: f64,
}

impl FrustumPredictor {
    pub fn new(fov: f64, aspect: f64, near: f64, far: f64) -> Self {
        Self {
            fov_degrees: fov,
            aspect_ratio: aspect,
            near_clip: near,
            far_clip: far,
        }
    }

    /// Given a camera pose (mocked here as xyz + pitch/yaw), return required tile IDs
    pub fn predict_intersecting_tiles(
        &self,
        camera_x: f64,
        camera_y: f64,
        camera_z: f64,
        _yaw: f64,
        _pitch: f64,
    ) -> Vec<u64> {
        // Structural stub: return a few mock tile IDs around the camera position
        // In reality, this would intersect a frustum math volume with the TilePyramid.
        let center_tile = ((camera_x / 100.0).floor() as u64) ^ ((camera_y / 100.0).floor() as u64) ^ ((camera_z / 100.0).floor() as u64);
        vec![center_tile, center_tile.wrapping_add(1), center_tile.wrapping_sub(1)]
    }
}

/// Eviction policy to manage strict VRAM budgets in 42MB limits.
pub struct VramBudgetEviction {
    pub max_tiles: usize,
    pub active_tiles: HashMap<u64, u64>, // tile_id -> last_access_timestamp
}

impl VramBudgetEviction {
    pub fn new(max_tiles: usize) -> Self {
        Self {
            max_tiles,
            active_tiles: HashMap::new(),
        }
    }

    pub fn access_tile(&mut self, tile_id: u64, timestamp: u64) {
        self.active_tiles.insert(tile_id, timestamp);
    }

    /// Returns a list of tile IDs to evict to stay under the budget.
    pub fn enforce_budget(&mut self) -> Vec<u64> {
        if self.active_tiles.len() <= self.max_tiles {
            return vec![];
        }

        let mut sorted: Vec<_> = self.active_tiles.iter().collect();
        // Sort by timestamp ascending (oldest first)
        sorted.sort_by_key(|&(_, ts)| *ts);

        let to_evict_count = self.active_tiles.len() - self.max_tiles;
        let mut evicted = Vec::with_capacity(to_evict_count);

        for (id, _) in sorted.into_iter().take(to_evict_count) {
            evicted.push(*id);
        }

        for id in &evicted {
            self.active_tiles.remove(id);
        }

        evicted
    }
}

/// A hierarchical index of .10d asset streaming chunks
pub struct TilePyramid {
    pub layers: HashMap<u8, HashSet<u64>>, // LOD level -> active tile IDs
}

impl TilePyramid {
    pub fn new() -> Self {
        Self {
            layers: HashMap::new(),
        }
    }

    pub fn register_tile(&mut self, lod: u8, tile_id: u64) {
        self.layers.entry(lod).or_default().insert(tile_id);
    }

    pub fn has_tile(&self, lod: u8, tile_id: u64) -> bool {
        self.layers.get(&lod).map_or(false, |s| s.contains(&tile_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile_id_at(x: f64, y: f64, z: f64) -> u64 {
        ((x / 100.0).floor() as u64) ^ ((y / 100.0).floor() as u64) ^ ((z / 100.0).floor() as u64)
    }

    #[test]
    fn vram_eviction_drops_oldest_first() {
        let mut evict = VramBudgetEviction::new(2);
        evict.access_tile(10, 1);
        evict.access_tile(20, 2);
        evict.access_tile(30, 3);
        let dropped = evict.enforce_budget();
        assert_eq!(dropped, vec![10]);
        assert!(!evict.active_tiles.contains_key(&10));
    }

    #[test]
    fn scene_streaming_planner_requests_missing_tiles() {
        let mut pyramid = TilePyramid::new();
        let pose = CameraPose {
            x: 150.0,
            y: 250.0,
            z: 50.0,
            yaw: 0.0,
            pitch: 0.0,
        };
        let center = tile_id_at(pose.x, pose.y, pose.z);
        pyramid.register_tile(0, center);
        pyramid.register_tile(0, center.wrapping_add(1));

        let mut planner = SceneStreamingPlanner::new(
            pyramid,
            FrustumPredictor::new(60.0, 16.0 / 9.0, 0.1, 1000.0),
            8,
            0,
        );

        let plan = planner.plan_stream(
            &pose,
            &StreamBudget {
                max_tiles: 8,
                timestamp: 100,
            },
        );

        assert!(plan.required_tiles.contains(&center));
        assert!(plan.evict_tiles.is_empty());
    }

    #[test]
    fn scene_streaming_planner_evicts_when_over_budget() {
        let mut pyramid = TilePyramid::new();
        let pose = CameraPose {
            x: 100.0,
            y: 200.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
        };
        let center = tile_id_at(pose.x, pose.y, pose.z);
        pyramid.register_tile(0, center);
        pyramid.register_tile(0, center.wrapping_add(1));
        pyramid.register_tile(0, center.wrapping_sub(1));

        let mut planner = SceneStreamingPlanner::new(
            pyramid,
            FrustumPredictor::new(90.0, 1.0, 0.1, 500.0),
            2,
            0,
        );

        // Pre-load two tiles so the next prediction forces an eviction.
        planner.eviction.access_tile(999, 1);
        planner.eviction.access_tile(888, 2);

        let plan = planner.plan_stream(
            &pose,
            &StreamBudget {
                max_tiles: 2,
                timestamp: 50,
            },
        );

        assert!(!plan.required_tiles.is_empty());
        assert!(!plan.evict_tiles.is_empty());
        assert!(plan.evict_tiles.contains(&999));
    }

    #[test]
    fn scene_streaming_planner_skips_unregistered_tiles() {
        let pyramid = TilePyramid::new();
        let mut planner = SceneStreamingPlanner::new(
            pyramid,
            FrustumPredictor::new(60.0, 16.0 / 9.0, 0.1, 1000.0),
            4,
            0,
        );
        let plan = planner.plan_stream(
            &CameraPose {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                yaw: 0.0,
                pitch: 0.0,
            },
            &StreamBudget {
                max_tiles: 4,
                timestamp: 1,
            },
        );
        assert!(plan.required_tiles.is_empty());
    }
}
