//! Media manifold seed — 3D kinematics, grapheme, acoustic.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

use super::registry::{ManifoldSeed, SeedContainer, SeedPanel, DockPosition};

pub fn media_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "media".into(),
        label: "Media".into(),
        icon: "film".into(),
        ontology_prefix: "hm".into(),
        description: "3D kinematics, grapheme, acoustic.".into(),
        containers: vec![
            SeedContainer {
                container_type: "media".into(),
                title: "3D Viewport".into(),
                x: 80.0, y: 60.0, width: 480.0, height: 360.0, z: 100.0,
                honesty: "live".into(),
            },
        ],
        panels: vec![
            SeedPanel { panel_type: "inspector".into(), dock: DockPosition::Right },
        ],
    }
}
