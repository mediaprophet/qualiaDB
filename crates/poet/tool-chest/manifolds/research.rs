//! Research manifold seed — GIS, clinical, rights alignment.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

use super::registry::{ManifoldSeed, SeedContainer, SeedPanel, DockPosition};

pub fn research_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "research".into(),
        label: "Research".into(),
        icon: "flask".into(),
        ontology_prefix: "hm".into(),
        description: "GIS, clinical, rights alignment.".into(),
        containers: vec![
            SeedContainer {
                container_type: "map".into(),
                title: "Map".into(),
                x: 80.0, y: 60.0, width: 480.0, height: 360.0, z: 100.0,
                honesty: "live".into(),
            },
        ],
        panels: vec![
            SeedPanel { panel_type: "graph-panel".into(), dock: DockPosition::Right },
            SeedPanel { panel_type: "aura-tray".into(), dock: DockPosition::Right },
        ],
    }
}
