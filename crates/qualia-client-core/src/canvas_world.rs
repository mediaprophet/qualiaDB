//! Chora — spatio-temporal commons canvas world configuration.
//!
//! A "world" is a bounded projection over the canvas substrate (doc 02 §2):
//! `{ temporal range, ordered layer-stack, permitted nquins, governance norms }`.
//! Worlds are configurations, not engine forks.

use serde::{Deserialize, Serialize};

/// Ontological stratum for a layer (doc 02 §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldStratum {
    /// Infrastructure, buildings, boundaries — authored human intent (a true digital twin).
    WorldOfMan,
    /// Biosphere, terrain, physics — computational approximation, never a twin.
    WorldOfGod,
    /// Records, claims, provenance — the informational environment.
    Infosphere,
    /// Human social relations — society as a stratum.
    Sociosphere,
}

/// A dataset layer in the ordered stack (doc 02 §3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CanvasLayer {
    GeoSpatial { endpoint: String, stratum: WorldStratum },
    Council { endpoint: String },
    Historical { endpoint: String },
    Celestial { endpoint: String },
    Infosphere { endpoint: String },
    MicroScale { endpoint: String },
    Biosphere { endpoint: String },
    Custom { name: String, endpoint: String, stratum: WorldStratum },
}

impl CanvasLayer {
    pub fn endpoint(&self) -> &str {
        match self {
            Self::GeoSpatial { endpoint, .. }
            | Self::Council { endpoint }
            | Self::Historical { endpoint }
            | Self::Celestial { endpoint }
            | Self::Infosphere { endpoint }
            | Self::MicroScale { endpoint }
            | Self::Biosphere { endpoint }
            | Self::Custom { endpoint, .. } => endpoint,
        }
    }

    pub fn stratum(&self) -> WorldStratum {
        match self {
            Self::GeoSpatial { stratum, .. } | Self::Custom { stratum, .. } => *stratum,
            Self::Council { .. } | Self::Historical { .. } => WorldStratum::WorldOfMan,
            Self::Celestial { .. } | Self::Biosphere { .. } | Self::MicroScale { .. } => {
                WorldStratum::WorldOfGod
            }
            Self::Infosphere { .. } => WorldStratum::Infosphere,
        }
    }
}

/// A governed norm applied to this world (N3 rule URI or deontic contract hash).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasNorm {
    pub rule_uri: String,
    #[serde(default)]
    pub description: String,
}

/// A planted or referenced asset in the world (content-addressed `.10d` or remote hash).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasAssetRef {
    /// Content hash (hex) or `urn:qualia:asset:…` URI.
    pub asset_id: String,
    /// Optional geodetic anchor (degrees).
    #[serde(default)]
    pub lat: Option<f64>,
    #[serde(default)]
    pub lon: Option<f64>,
    #[serde(default)]
    pub alt_m: Option<f64>,
    /// Valid-time interval (unix seconds). None = always visible.
    #[serde(default)]
    pub valid_from: Option<u64>,
    #[serde(default)]
    pub valid_until: Option<u64>,
    /// Permissive licence (required for render gate).
    pub licence: String,
}

/// A "world" configuration: a bounded projection over the canvas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasWorldConfig {
    /// Unique identifier (e.g. `q42:world:council-sydney`).
    pub id: String,
    /// Human-readable title shown in the Chora panel.
    pub title: String,
    /// Optional temporal range: (start_epoch, end_epoch). None = full timeline.
    pub temporal_range: Option<(u64, u64)>,
    /// Ordered stack of dataset layers to composite.
    pub layer_stack: Vec<CanvasLayer>,
    /// Offline or seeded assets rendered in this world.
    #[serde(default)]
    pub assets: Vec<CanvasAssetRef>,
    /// Whitelist of permitted nquin subject hashes. Empty = layer-stack bounds only.
    #[serde(default)]
    pub permitted_nquins: Vec<u64>,
    /// Deontic norms / governance rules applied to this world.
    #[serde(default)]
    pub norms: Vec<CanvasNorm>,
    /// Reference-frame origin for local ENU coordinates (degrees).
    #[serde(default)]
    pub origin_lat: f64,
    #[serde(default)]
    pub origin_lon: f64,
    #[serde(default)]
    pub origin_alt_m: f64,
}

impl Default for CanvasWorldConfig {
    fn default() -> Self {
        Self {
            id: "q42:world:default".to_string(),
            title: "Default World".to_string(),
            temporal_range: None,
            layer_stack: vec![],
            assets: vec![],
            permitted_nquins: vec![],
            norms: vec![],
            origin_lat: -33.8688,
            origin_lon: 151.2093,
            origin_alt_m: 0.0,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum WorldConfigError {
    EmptyId,
    InvertedTemporalRange,
    AssetMissingLicence { asset_id: String },
    AssetInvertedValidity { asset_id: String },
}

impl std::fmt::Display for WorldConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyId => write!(f, "world id must not be empty"),
            Self::InvertedTemporalRange => write!(f, "temporal_range start must be <= end"),
            Self::AssetMissingLicence { asset_id } => {
                write!(f, "asset {asset_id} requires a licence")
            }
            Self::AssetInvertedValidity { asset_id } => {
                write!(f, "asset {asset_id} valid_from must be <= valid_until")
            }
        }
    }
}
impl std::error::Error for WorldConfigError {}

impl CanvasWorldConfig {
    /// Validate the config contract (P0 freeze).
    pub fn validate(&self) -> Result<(), WorldConfigError> {
        if self.id.trim().is_empty() {
            return Err(WorldConfigError::EmptyId);
        }
        if let Some((t0, t1)) = self.temporal_range {
            if t0 > t1 {
                return Err(WorldConfigError::InvertedTemporalRange);
            }
        }
        for asset in &self.assets {
            if asset.licence.trim().is_empty() {
                return Err(WorldConfigError::AssetMissingLicence {
                    asset_id: asset.asset_id.clone(),
                });
            }
            if let (Some(vf), Some(vu)) = (asset.valid_from, asset.valid_until) {
                if vf > vu {
                    return Err(WorldConfigError::AssetInvertedValidity {
                        asset_id: asset.asset_id.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    /// P0 demo world: one offline tile, Sydney origin, CC0 licence placeholder.
    pub fn seed_demo() -> Self {
        Self {
            id: "q42:world:demo-offline".to_string(),
            title: "Chora Demo (offline)".to_string(),
            temporal_range: Some((1_700_000_000, 1_900_000_000)),
            layer_stack: vec![CanvasLayer::GeoSpatial {
                endpoint: "local://terrain/demo".to_string(),
                stratum: WorldStratum::WorldOfGod,
            }],
            assets: vec![CanvasAssetRef {
                asset_id: "local://assets/demo-tile.10d".to_string(),
                lat: Some(-33.8688),
                lon: Some(151.2093),
                alt_m: Some(0.0),
                valid_from: Some(1_700_000_000),
                valid_until: None,
                licence: "CC0".to_string(),
            }],
            norms: vec![CanvasNorm {
                rule_uri: "urn:qualia:canvas:public-commons".to_string(),
                description: "Permissive-commons read; planting requires placement right".to_string(),
            }],
            origin_lat: -33.8688,
            origin_lon: 151.2093,
            origin_alt_m: 0.0,
            ..Default::default()
        }
    }

    /// Resolve layer endpoints in stack order (deduplicated).
    pub fn layer_endpoints(&self) -> Vec<&str> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for layer in &self.layer_stack {
            let ep = layer.endpoint();
            if seen.insert(ep) {
                out.push(ep);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_world_validates() {
        let w = CanvasWorldConfig::seed_demo();
        w.validate().unwrap();
        assert_eq!(w.layer_stack.len(), 1);
        assert_eq!(w.assets[0].licence, "CC0");
    }

    #[test]
    fn rejects_inverted_temporal_range() {
        let mut w = CanvasWorldConfig::default();
        w.temporal_range = Some((2000, 1000));
        assert_eq!(w.validate(), Err(WorldConfigError::InvertedTemporalRange));
    }

    #[test]
    fn layer_endpoints_deduplicated() {
        let w = CanvasWorldConfig {
            layer_stack: vec![
                CanvasLayer::GeoSpatial {
                    endpoint: "a".into(),
                    stratum: WorldStratum::WorldOfGod,
                },
                CanvasLayer::Council {
                    endpoint: "b".into(),
                },
                CanvasLayer::GeoSpatial {
                    endpoint: "a".into(),
                    stratum: WorldStratum::WorldOfGod,
                },
            ],
            ..Default::default()
        };
        assert_eq!(w.layer_endpoints(), vec!["a", "b"]);
    }
}