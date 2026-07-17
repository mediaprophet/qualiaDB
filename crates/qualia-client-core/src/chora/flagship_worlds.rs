//! P8 flagship world configurations for the Chora canvas.
//!
//! Each world is a curated layer-stack + norms bundle — substrate validation, not new engine code.

use chrono::{TimeZone, Utc};

use crate::canvas_store::{CanvasWorldStore, UpsertError};
use crate::canvas_world::{
    CanvasAssetRef, CanvasLayer, CanvasNorm, CanvasWorldConfig, WorldStratum,
};

/// Sydney CBD reference origin (degrees).
const SYDNEY_LAT: f64 = -33.8688;
const SYDNEY_LON: f64 = 151.2093;

/// Mid-year temporal anchor for calendar-year valid-time intervals.
///
/// Pre-1970 dates have **negative** Unix timestamps; casting those to `u64`
/// wraps to huge values and inverts `valid_from <= valid_until` (kent-brewery
/// failed validation). We store post-epoch as real Unix seconds; pre-epoch as
/// ordered synthetic stamps in `0..FIRST_UNIX_YEAR_MID` so relative order of
/// historical years is preserved under `u64`.
fn year_mid(y: u32) -> u64 {
    const FIRST_UNIX: u64 = 0; // 1970-01-01
    // ~ mid 1970
    const YEAR_1970_MID: u64 = 15_778_800;
    match Utc.with_ymd_and_hms(y as i32, 7, 1, 0, 0, 0).single() {
        Some(dt) => {
            let ts = dt.timestamp();
            if ts >= 0 {
                ts as u64
            } else {
                // Map years [0, 1970) into [0, YEAR_1970_MID) linearly by year.
                let y = y.min(1969) as u64;
                FIRST_UNIX + (y * YEAR_1970_MID) / 1970
            }
        }
        None => y as u64,
    }
}

/// OpenHistoricalMap-style historical world: temporal scrub over Sydney built fabric.
pub fn history_world() -> CanvasWorldConfig {
    CanvasWorldConfig {
        id: "q42:world:history-sydney".to_string(),
        title: "Sydney History (OpenHistoricalMap)".to_string(),
        temporal_range: Some((1800, 2026)),
        layer_stack: vec![
            CanvasLayer::Historical {
                endpoint: "adapter://openhistoricalmap/v1".to_string(),
            },
            CanvasLayer::GeoSpatial {
                endpoint: "adapter://osm/terrain/sydney".to_string(),
                stratum: WorldStratum::WorldOfGod,
            },
        ],
        assets: vec![
            CanvasAssetRef {
                asset_id: "urn:qualia:asset:history:customs-house".to_string(),
                lat: Some(-33.8672),
                lon: Some(151.2113),
                alt_m: Some(0.0),
                valid_from: Some(year_mid(1845)),
                valid_until: None,
                licence: "CC0".to_string(),
            },
            CanvasAssetRef {
                asset_id: "urn:qualia:asset:history:general-post-office".to_string(),
                lat: Some(-33.8720),
                lon: Some(151.2075),
                alt_m: Some(0.0),
                valid_from: Some(year_mid(1874)),
                valid_until: None,
                licence: "CC0".to_string(),
            },
            CanvasAssetRef {
                asset_id: "urn:qualia:asset:history:town-hall".to_string(),
                lat: Some(-33.8732),
                lon: Some(151.2060),
                alt_m: Some(0.0),
                valid_from: Some(year_mid(1889)),
                valid_until: None,
                licence: "CC0".to_string(),
            },
            CanvasAssetRef {
                asset_id: "urn:qualia:asset:history:harbour-bridge".to_string(),
                lat: Some(-33.8523),
                lon: Some(151.2108),
                alt_m: Some(0.0),
                valid_from: Some(year_mid(1932)),
                valid_until: None,
                licence: "CC0".to_string(),
            },
            CanvasAssetRef {
                asset_id: "urn:qualia:asset:history:opera-house".to_string(),
                lat: Some(-33.8568),
                lon: Some(151.2153),
                alt_m: Some(0.0),
                valid_from: Some(year_mid(1973)),
                valid_until: None,
                licence: "CC0".to_string(),
            },
            CanvasAssetRef {
                asset_id: "urn:qualia:asset:history:kent-brewery".to_string(),
                lat: Some(-33.8795),
                lon: Some(151.1948),
                alt_m: Some(0.0),
                valid_from: Some(year_mid(1835)),
                valid_until: Some(year_mid(2005)),
                licence: "CC0".to_string(),
            },
        ],
        norms: vec![
            CanvasNorm {
                rule_uri: "urn:qualia:canvas:public-commons".to_string(),
                description: "Historical commons read; HGIS attribution required on export".to_string(),
            },
            CanvasNorm {
                rule_uri: "urn:qualia:canvas:temporal-scrub".to_string(),
                description: "Spawn/decay governed by valid-time intervals".to_string(),
            },
        ],
        origin_lat: SYDNEY_LAT,
        origin_lon: SYDNEY_LON,
        origin_alt_m: 0.0,
        ..Default::default()
    }
}

/// Biosphere + geospatial world: GBIF occurrence layer over terrain (world-of-god stratum).
pub fn biosphere_world() -> CanvasWorldConfig {
    CanvasWorldConfig {
        id: "q42:world:biosphere".to_string(),
        title: "Biosphere (GBIF)".to_string(),
        temporal_range: None,
        layer_stack: vec![
            CanvasLayer::Biosphere {
                endpoint: "adapter://gbif/v1/occurrence".to_string(),
            },
            CanvasLayer::GeoSpatial {
                endpoint: "adapter://terrain/global-dem".to_string(),
                stratum: WorldStratum::WorldOfGod,
            },
        ],
        assets: vec![],
        norms: vec![CanvasNorm {
            rule_uri: "urn:qualia:canvas:world-of-god".to_string(),
            description: "Biosphere outputs are Hypothesis under F/A — never ground truth".to_string(),
        }],
        origin_lat: 0.0,
        origin_lon: 0.0,
        origin_alt_m: 0.0,
        ..Default::default()
    }
}

/// Council municipal open-data world (world-of-man stratum).
pub fn council_world() -> CanvasWorldConfig {
    CanvasWorldConfig {
        id: "q42:world:council-sydney".to_string(),
        title: "Sydney Council Open Data".to_string(),
        temporal_range: None,
        layer_stack: vec![
            CanvasLayer::Council {
                endpoint: "adapter://council/sydney-open-data/v1".to_string(),
            },
            CanvasLayer::GeoSpatial {
                endpoint: "adapter://osm/buildings/sydney".to_string(),
                stratum: WorldStratum::WorldOfMan,
            },
        ],
        assets: vec![],
        norms: vec![CanvasNorm {
            rule_uri: "urn:qualia:canvas:council-commons".to_string(),
            description: "Municipal open data; placement requires council placement right".to_string(),
        }],
        origin_lat: SYDNEY_LAT,
        origin_lon: SYDNEY_LON,
        origin_alt_m: 0.0,
        ..Default::default()
    }
}

/// SDG dashboard walkthrough: infosphere metrics composited with council context.
pub fn sdg_world() -> CanvasWorldConfig {
    CanvasWorldConfig {
        id: "q42:world:sdg-dashboard".to_string(),
        title: "SDG Dashboard Walkthrough".to_string(),
        temporal_range: Some((2015, 2030)),
        layer_stack: vec![
            CanvasLayer::Infosphere {
                endpoint: "adapter://infosphere/sdg-indicators/v1".to_string(),
            },
            CanvasLayer::Council {
                endpoint: "adapter://council/sdg-local-metrics/v1".to_string(),
            },
        ],
        assets: vec![CanvasAssetRef {
            asset_id: "urn:qualia:asset:sdg:walkthrough-anchor".to_string(),
            lat: Some(SYDNEY_LAT),
            lon: Some(SYDNEY_LON),
            alt_m: Some(0.0),
            valid_from: Some(year_mid(2015)),
            valid_until: Some(year_mid(2030)),
            licence: "CC-BY-4.0".to_string(),
        }],
        norms: vec![
            CanvasNorm {
                rule_uri: "urn:qualia:canvas:sdg-alignment".to_string(),
                description: "Constructed-vs-natural interaction metrics for SDG walkthrough".to_string(),
            },
            CanvasNorm {
                rule_uri: "urn:qualia:canvas:public-commons".to_string(),
                description: "Indicator dashboards readable under permissive commons".to_string(),
            },
        ],
        origin_lat: SYDNEY_LAT,
        origin_lon: SYDNEY_LON,
        origin_alt_m: 0.0,
        ..Default::default()
    }
}

/// Library / GLAM publishing flagship (P8 curation-led).
pub fn glam_world() -> CanvasWorldConfig {
    CanvasWorldConfig {
        id: "q42:world:glam-commons".to_string(),
        title: "GLAM Commons Publishing".to_string(),
        temporal_range: None,
        layer_stack: vec![
            CanvasLayer::Infosphere {
                endpoint: "adapter://glam/iiif-manifests/v1".to_string(),
            },
            CanvasLayer::Historical {
                endpoint: "adapter://hgis/world-historical-gazetteer/v1".to_string(),
            },
        ],
        assets: vec![CanvasAssetRef {
            asset_id: "urn:qualia:asset:glam:sample-map-tile".to_string(),
            lat: Some(SYDNEY_LAT),
            lon: Some(SYDNEY_LON),
            alt_m: Some(0.0),
            valid_from: None,
            valid_until: None,
            licence: "CC0".to_string(),
        }],
        norms: vec![CanvasNorm {
            rule_uri: "urn:qualia:canvas:glam-stewardship".to_string(),
            description: "GLAM holdings explorable; export requires provenance sidecar".to_string(),
        }],
        origin_lat: SYDNEY_LAT,
        origin_lon: SYDNEY_LON,
        origin_alt_m: 0.0,
        ..Default::default()
    }
}

/// All P8 flagship world configurations (distinct ids).
pub fn all_flagship_worlds() -> Vec<CanvasWorldConfig> {
    vec![
        history_world(),
        biosphere_world(),
        council_world(),
        sdg_world(),
        glam_world(),
    ]
}

/// Upsert each flagship world when its id is not already present in the store.
pub fn seed_all_flagships(store: &CanvasWorldStore, now_unix: u64) -> std::io::Result<usize> {
    let existing: std::collections::HashSet<String> =
        store.list()?.into_iter().map(|w| w.id).collect();
    let mut seeded = 0usize;
    for config in all_flagship_worlds() {
        if existing.contains(&config.id) {
            continue;
        }
        store.upsert(config, now_unix).map_err(|e| match e {
            UpsertError::Io(e) => e,
            UpsertError::Invalid(err) => std::io::Error::other(err.to_string()),
        })?;
        seeded += 1;
    }
    Ok(seeded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;

    #[test]
    fn year_mid_preserves_historical_order() {
        let a = year_mid(1835);
        let b = year_mid(2005);
        assert!(a <= b, "1835 mid ({a}) must be <= 2005 mid ({b})");
        let c = year_mid(1973);
        let d = year_mid(2015);
        assert!(c <= d);
    }

    #[test]
    fn flagship_worlds_validate_five_distinct_ids() {
        let worlds = all_flagship_worlds();
        assert_eq!(worlds.len(), 5, "expected five flagship worlds");
        let ids: HashSet<_> = worlds.iter().map(|w| w.id.as_str()).collect();
        assert_eq!(ids.len(), 5, "flagship world ids must be distinct");
        for world in &worlds {
            world.validate().expect("flagship world should validate");
        }
    }

    #[test]
    fn seed_all_flagships_upserts_only_missing() {
        let dir = std::env::temp_dir().join(format!("chora-flagship-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let store = CanvasWorldStore::open(&dir).unwrap();

        let first = seed_all_flagships(&store, 1_700_000_000).unwrap();
        assert_eq!(first, 5);
        assert_eq!(store.list().unwrap().len(), 5);

        let second = seed_all_flagships(&store, 1_700_000_100).unwrap();
        assert_eq!(second, 0);
        assert_eq!(store.list().unwrap().len(), 5);

        let _ = fs::remove_dir_all(&dir);
    }
}