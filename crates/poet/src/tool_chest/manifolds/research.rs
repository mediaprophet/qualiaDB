//! Research manifold seed — GIS, clinical, rights alignment.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

use super::super::core::registry::{DockPosition, ManifoldSeed, SeedContainer, SeedPanel};

pub fn research_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "research".into(),
        label: "Research".into(),
        icon: "flask".into(),
        ontology_prefix: "hm".into(),
        description: "GIS, library, documents, LaTeX, slides, graph exploration.".into(),
        containers: vec![
            SeedContainer {
                container_type: "map".into(),
                title: "Map".into(),
                x: 80.0,
                y: 60.0,
                width: 480.0,
                height: 360.0,
                z: 100.0,
                honesty: "live".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "library".into(),
                title: "Library".into(),
                x: 580.0,
                y: 60.0,
                width: 420.0,
                height: 360.0,
                z: 100.0,
                honesty: "live".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "doc".into(),
                title: "Document".into(),
                x: 80.0,
                y: 440.0,
                width: 420.0,
                height: 280.0,
                z: 100.0,
                honesty: "missing".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "latex".into(),
                title: "LaTeX".into(),
                x: 520.0,
                y: 440.0,
                width: 480.0,
                height: 280.0,
                z: 100.0,
                honesty: "missing".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "slide".into(),
                title: "Slides".into(),
                x: 80.0,
                y: 740.0,
                width: 440.0,
                height: 240.0,
                z: 100.0,
                honesty: "missing".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "domain_lab".into(),
                title: "Domain Lab".into(),
                x: 540.0,
                y: 740.0,
                width: 460.0,
                height: 240.0,
                z: 100.0,
                honesty: "live".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "construct_portal".into(),
                title: "Knowledge construct".into(),
                x: 1020.0,
                y: 740.0,
                width: 220.0,
                height: 110.0,
                z: 100.0,
                honesty: "live".into(),
                target_construct: "knowledge".into(),
                target_manifold: "knowledge".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "nested_manifold".into(),
                title: "Health manifold".into(),
                x: 1020.0,
                y: 870.0,
                width: 220.0,
                height: 110.0,
                z: 100.0,
                honesty: "live".into(),
                target_manifold: "health".into(),
                ..Default::default()
            },
        ],
        connections: vec![],
        panels: vec![
            SeedPanel {
                panel_type: "graph-panel".into(),
                dock: DockPosition::Right,
            },
            SeedPanel {
                panel_type: "aura-tray".into(),
                dock: DockPosition::Right,
            },
        ],
        ..Default::default()
    }
}
