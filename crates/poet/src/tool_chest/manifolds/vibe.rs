//! Vibe manifold seed — VibeScript console + diagnose.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

use super::super::core::registry::{
    DockPosition, ManifoldSeed, SeedConnection, SeedContainer, SeedPanel,
};

pub fn vibe_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "vibe".into(),
        label: "Vibe".into(),
        icon: "terminal".into(),
        ontology_prefix: "vibe".into(),
        description:
            "VibeScript console, Dual Studio GPU viewport, diagnose (human door into Qualia)."
                .into(),
        containers: vec![
            SeedContainer {
                container_type: "code".into(),
                title: "VibeScript Console".into(),
                x: 80.0,
                y: 60.0,
                width: 600.0,
                height: 400.0,
                z: 100.0,
                honesty: "live".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "nested_manifold".into(),
                title: "Studio nested manifold".into(),
                x: 700.0,
                y: 60.0,
                width: 400.0,
                height: 400.0,
                z: 100.0,
                honesty: "live".into(),
                target_manifold: "studio".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "dual_studio".into(),
                title: "Dual Studio".into(),
                x: 80.0,
                y: 480.0,
                width: 1020.0,
                height: 420.0,
                z: 100.0,
                honesty: "live".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "pulse".into(),
                title: "Diagnose Pulse".into(),
                x: 80.0,
                y: 920.0,
                width: 1020.0,
                height: 180.0,
                z: 100.0,
                honesty: "present".into(),
                ..Default::default()
            },
        ],
        connections: vec![
            SeedConnection {
                id: "wire-v1".into(),
                from: 0,
                to: 1,
                wire_type: "active".into(),
                label: "vibe:rendersTo".into(),
            },
            SeedConnection {
                id: "wire-v2".into(),
                from: 0,
                to: 2,
                wire_type: "active".into(),
                label: "vibe:dualStudio".into(),
            },
        ],
        panels: vec![
            SeedPanel {
                panel_type: "inspector".into(),
                dock: DockPosition::Right,
            },
            SeedPanel {
                panel_type: "status-bar".into(),
                dock: DockPosition::Bottom,
            },
        ],
        ..Default::default()
    }
}
