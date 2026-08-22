//! Media manifold seed — 3D kinematics, grapheme, acoustic.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

use super::super::core::registry::{DockPosition, ManifoldSeed, SeedContainer, SeedPanel};

pub fn media_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "media".into(),
        label: "Media".into(),
        icon: "film".into(),
        ontology_prefix: "hm".into(),
        description: "3D kinematics, vision, audio, triad, grapheme.".into(),
        containers: vec![
            SeedContainer {
                container_type: "media".into(),
                title: "3D Viewport".into(),
                x: 80.0,
                y: 60.0,
                width: 480.0,
                height: 360.0,
                z: 100.0,
                honesty: "live".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "3d".into(),
                title: "3D Asset Browser".into(),
                x: 580.0,
                y: 60.0,
                width: 420.0,
                height: 360.0,
                z: 100.0,
                honesty: "missing".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "vision".into(),
                title: "Vision".into(),
                x: 80.0,
                y: 440.0,
                width: 420.0,
                height: 280.0,
                z: 100.0,
                honesty: "missing".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "listen".into(),
                title: "Listen".into(),
                x: 520.0,
                y: 440.0,
                width: 420.0,
                height: 280.0,
                z: 100.0,
                honesty: "missing".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "triad".into(),
                title: "Triad (q42+p64+d10)".into(),
                x: 80.0,
                y: 740.0,
                width: 860.0,
                height: 200.0,
                z: 100.0,
                honesty: "missing".into(),
                ..Default::default()
            },
        ],
        connections: vec![],
        panels: vec![SeedPanel {
            panel_type: "inspector".into(),
            dock: DockPosition::Right,
        }],
    }
}
