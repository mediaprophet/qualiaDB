//! Vibe manifold seed — VibeScript console + diagnose.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

use super::registry::{ManifoldSeed, SeedContainer, SeedPanel, DockPosition};

pub fn vibe_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "vibe".into(),
        label: "Vibe".into(),
        icon: "terminal".into(),
        ontology_prefix: "vibe".into(),
        description: "VibeScript console + diagnose (human door into Qualia).".into(),
        containers: vec![
            SeedContainer {
                container_type: "code".into(),
                title: "VibeScript Console".into(),
                x: 80.0, y: 60.0, width: 600.0, height: 400.0, z: 100.0,
                honesty: "live".into(),
            },
        ],
        panels: vec![
            SeedPanel { panel_type: "inspector".into(), dock: DockPosition::Right },
            SeedPanel { panel_type: "status-bar".into(), dock: DockPosition::Bottom },
        ],
    }
}
