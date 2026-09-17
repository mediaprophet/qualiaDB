//! Settings manifold seed — capabilities, fiduciary VM, preferences.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

use super::registry::{ManifoldSeed, SeedContainer, SeedPanel, DockPosition};

/// Settings manifold seed: property sheet + capability badges + protection policies.
pub fn settings_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "settings".into(),
        label: "Settings".into(),
        icon: "gear".into(),
        ontology_prefix: "set".into(),
        description: "Capabilities, preferences, configuration, parameters, and vulnerable person protection policies.".into(),
        containers: vec![
            SeedContainer {
                container_type: "settings".into(),
                title: "Preferences".into(),
                x: 80.0,
                y: 60.0,
                width: 420.0,
                height: 320.0,
                z: 100.0,
                honesty: "missing".into(),
            },
            SeedContainer {
                container_type: "capabilities".into(),
                title: "Capabilities".into(),
                x: 520.0,
                y: 60.0,
                width: 380.0,
                height: 280.0,
                z: 100.0,
                honesty: "missing".into(),
            },
            SeedContainer {
                container_type: "protection-policies".into(),
                title: "Protection Policies".into(),
                x: 520.0,
                y: 360.0,
                width: 380.0,
                height: 200.0,
                z: 100.0,
                honesty: "missing".into(),
            },
        ],
        panels: vec![
            SeedPanel {
                panel_type: "property-sheet".into(),
                dock: DockPosition::Right,
            },
            SeedPanel {
                panel_type: "aura-tray".into(),
                dock: DockPosition::Right,
            },
        ],
    }
}
