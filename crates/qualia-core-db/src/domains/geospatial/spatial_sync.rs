use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A planted asset representation for the spatial sync protocol.
/// Defines "who-planted-what-where" across a shared world, keyed by spatial index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlantedAsset {
    /// A unique identifier for this placement (UUID or similar).
    pub asset_id: String,
    /// The H3 cell (or quadtree node) index this asset was planted in.
    pub cell_id: u64,
    /// The exact world-space position.
    pub position: [f64; 3],
    /// Orientation/rotation (quaternion or euler angles), stubbed as [f64; 4] for quaternion.
    pub rotation: [f64; 4],
    /// DID of the participant who planted this.
    pub creator_did: String,
    /// Lamport clock tracking the latest update/deletion state.
    pub lamport: u64,
    /// True if the asset was deleted/uprooted by the creator.
    pub deleted: bool,
}

/// A spatial sync cell keyed by H3/quadtree cell id, holding the merged planted assets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialSyncCell {
    pub cell_id: u64,
    pub planted_assets: Vec<PlantedAsset>,
}

impl SpatialSyncCell {
    pub fn new(cell_id: u64, planted_assets: Vec<PlantedAsset>) -> Self {
        Self {
            cell_id,
            planted_assets,
        }
    }

    /// Build a cell view from merged assets, filtering to the target cell and dropping tombstones.
    pub fn from_assets(cell_id: u64, assets: &[PlantedAsset]) -> Self {
        let mut planted_assets: Vec<PlantedAsset> = assets
            .iter()
            .filter(|a| a.cell_id == cell_id && !a.deleted)
            .cloned()
            .collect();
        planted_assets.sort_by(|a, b| a.asset_id.cmp(&b.asset_id));
        Self {
            cell_id,
            planted_assets,
        }
    }

    /// Merge incoming remote/local assets into this cell using LWW resolution.
    pub fn merge(&mut self, incoming: &[PlantedAsset]) {
        let merged = merge_planted_assets(&self.planted_assets, incoming);
        self.planted_assets = merged
            .into_iter()
            .filter(|a| a.cell_id == self.cell_id && !a.deleted)
            .collect();
    }
}

/// Last-Write-Wins (LWW) conflict resolution for overlapping asset placements.
/// Merges two sets of `PlantedAsset`s, resolving conflicts by `asset_id` using the Lamport clock.
/// Higher Lamport wins. Deterministic tie-breaking on `asset_id` string if clocks match.
pub fn merge_planted_assets(local: &[PlantedAsset], remote: &[PlantedAsset]) -> Vec<PlantedAsset> {
    let mut state: HashMap<String, PlantedAsset> = HashMap::new();

    for asset in local.iter().chain(remote.iter()) {
        if let Some(existing) = state.get(&asset.asset_id) {
            if asset.lamport > existing.lamport {
                state.insert(asset.asset_id.clone(), asset.clone());
            } else if asset.lamport == existing.lamport {
                // Deterministic tie breaker using position string just in case
                // but really should not happen for the exact same asset_id unless they are identical
                // We'll prefer the one that is somehow "larger" in creator_did as a stable tie-break,
                // or just leave it. If identical, leaving `existing` is fine.
                if asset.creator_did > existing.creator_did {
                    state.insert(asset.asset_id.clone(), asset.clone());
                }
            }
        } else {
            state.insert(asset.asset_id.clone(), asset.clone());
        }
    }

    // Convert to sorted vec to ensure deterministic output
    let mut merged: Vec<PlantedAsset> = state.into_values().collect();
    merged.sort_by(|a, b| a.asset_id.cmp(&b.asset_id));
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_planted_assets() {
        let asset1_v1 = PlantedAsset {
            asset_id: "obj-1".into(),
            cell_id: 1234,
            position: [1.0, 2.0, 3.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            creator_did: "did:q42:alice".into(),
            lamport: 1,
            deleted: false,
        };

        let asset1_v2 = PlantedAsset {
            asset_id: "obj-1".into(),
            cell_id: 1234,
            position: [1.0, 5.0, 3.0], // moved
            rotation: [0.0, 0.0, 0.0, 1.0],
            creator_did: "did:q42:alice".into(),
            lamport: 2,
            deleted: false,
        };

        let asset2 = PlantedAsset {
            asset_id: "obj-2".into(),
            cell_id: 1234,
            position: [5.0, 5.0, 5.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            creator_did: "did:q42:bob".into(),
            lamport: 1,
            deleted: false,
        };

        let local = vec![asset1_v1.clone()];
        let remote = vec![asset1_v2.clone(), asset2.clone()];

        let merged = merge_planted_assets(&local, &remote);
        assert_eq!(merged.len(), 2);

        // V2 should overwrite V1 for obj-1
        assert_eq!(merged[0].asset_id, "obj-1");
        assert_eq!(merged[0].position[1], 5.0);

        assert_eq!(merged[1].asset_id, "obj-2");
    }

    #[test]
    fn spatial_sync_cell_filters_by_cell_and_tombstones() {
        let active = PlantedAsset {
            asset_id: "obj-a".into(),
            cell_id: 42,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            creator_did: "did:q42:alice".into(),
            lamport: 1,
            deleted: false,
        };
        let tombstone = PlantedAsset {
            asset_id: "obj-b".into(),
            cell_id: 42,
            position: [1.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            creator_did: "did:q42:bob".into(),
            lamport: 2,
            deleted: true,
        };
        let other_cell = PlantedAsset {
            asset_id: "obj-c".into(),
            cell_id: 99,
            position: [2.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            creator_did: "did:q42:carol".into(),
            lamport: 1,
            deleted: false,
        };

        let cell = SpatialSyncCell::from_assets(42, &[active.clone(), tombstone, other_cell]);
        assert_eq!(cell.cell_id, 42);
        assert_eq!(cell.planted_assets.len(), 1);
        assert_eq!(cell.planted_assets[0].asset_id, "obj-a");
    }

    #[test]
    fn spatial_sync_cell_merge_applies_lww() {
        let local = PlantedAsset {
            asset_id: "obj-1".into(),
            cell_id: 7,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            creator_did: "did:q42:alice".into(),
            lamport: 1,
            deleted: false,
        };
        let remote = PlantedAsset {
            asset_id: "obj-1".into(),
            cell_id: 7,
            position: [5.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            creator_did: "did:q42:bob".into(),
            lamport: 3,
            deleted: false,
        };

        let mut cell = SpatialSyncCell::from_assets(7, &[local]);
        cell.merge(&[remote]);
        assert_eq!(cell.planted_assets.len(), 1);
        assert_eq!(cell.planted_assets[0].position[0], 5.0);
    }
}
