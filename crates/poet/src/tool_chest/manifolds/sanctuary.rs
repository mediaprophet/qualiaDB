//! Sanctuary manifold seed — vault, notes, pulse.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::super::core::registry::{
    DockPosition, ManifoldSeed, SeedConnection, SeedContainer, SeedPanel,
};

pub fn sanctuary_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "sanctuary".into(),
        label: "Sanctuary".into(),
        icon: "sanctuary".into(),
        ontology_prefix: "sanctuary".into(),
        description:
            "Vault, health records, anatomy, protected notes, pulse stream. Zero-motion mode."
                .into(),
        containers: vec![
            SeedContainer {
                container_type: "doc".into(),
                title: "Vault".into(),
                x: 30.0,
                y: 30.0,
                width: 400.0,
                height: 300.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "pulse".into(),
                title: "Pulse Stream".into(),
                x: 450.0,
                y: 30.0,
                width: 400.0,
                height: 300.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "health".into(),
                title: "Health Record".into(),
                x: 30.0,
                y: 350.0,
                width: 400.0,
                height: 280.0,
                z: 1.0,
                honesty: "missing".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "anatomy".into(),
                title: "Anatomy".into(),
                x: 450.0,
                y: 350.0,
                width: 400.0,
                height: 280.0,
                z: 1.0,
                honesty: "missing".into(),
                ..Default::default()
            },
        ],
        connections: vec![SeedConnection {
            id: "wire-san1".into(),
            from: 2,
            to: 3,
            wire_type: "active".into(),
            label: "med:hasAnatomy".into(),
        }],
        panels: vec![SeedPanel {
            panel_type: "aura".into(),
            dock: DockPosition::Right,
        }],
    }
}
