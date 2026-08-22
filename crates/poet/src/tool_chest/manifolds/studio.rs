//! Studio manifold seed — 3D / Animation / Audio containers (Workstream D).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::super::core::registry::{
    DockPosition, ManifoldSeed, SeedConnection, SeedContainer, SeedPanel,
};

pub fn studio_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "studio".into(),
        label: "Studio".into(),
        icon: "studio".into(),
        ontology_prefix: "vis".into(),
        description: "3D modelling, animation, audio desk, and immersive spatial audio. \
             PortalGpu wgpu viewport, AnimationPlayer, AudioWorklet, HRTF/Ambisonic."
            .into(),
        containers: vec![
            SeedContainer {
                container_type: "scene_view".into(),
                title: "Scene View".into(),
                x: 30.0,
                y: 30.0,
                width: 900.0,
                height: 400.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "animation_timeline".into(),
                title: "Animation Timeline".into(),
                x: 30.0,
                y: 450.0,
                width: 900.0,
                height: 300.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "desk_surface".into(),
                title: "Desk Surface".into(),
                x: 30.0,
                y: 770.0,
                width: 900.0,
                height: 300.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "transport".into(),
                title: "Transport".into(),
                x: 950.0,
                y: 30.0,
                width: 420.0,
                height: 200.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "routing_matrix".into(),
                title: "Routing Matrix".into(),
                x: 950.0,
                y: 250.0,
                width: 420.0,
                height: 300.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "spatial_audio".into(),
                title: "Spatial Audio".into(),
                x: 950.0,
                y: 570.0,
                width: 420.0,
                height: 400.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "scene_graph".into(),
                title: "Scene Graph".into(),
                x: 1390.0,
                y: 30.0,
                width: 360.0,
                height: 250.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "material_editor".into(),
                title: "Material Editor".into(),
                x: 1390.0,
                y: 300.0,
                width: 360.0,
                height: 300.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "lighting_editor".into(),
                title: "Lighting Editor".into(),
                x: 1390.0,
                y: 620.0,
                width: 360.0,
                height: 300.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "tensor_inspector".into(),
                title: "Tensor Inspector".into(),
                x: 30.0,
                y: 1090.0,
                width: 660.0,
                height: 300.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "asset_library".into(),
                title: "Asset Library".into(),
                x: 710.0,
                y: 1090.0,
                width: 660.0,
                height: 300.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "channel_strip".into(),
                title: "Channel Strip".into(),
                x: 950.0,
                y: 990.0,
                width: 420.0,
                height: 400.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "meter_bridge".into(),
                title: "Meter Bridge".into(),
                x: 1390.0,
                y: 940.0,
                width: 360.0,
                height: 350.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "automation_lanes".into(),
                title: "Automation Lanes".into(),
                x: 30.0,
                y: 1410.0,
                width: 1340.0,
                height: 200.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
        ],
        connections: vec![
            SeedConnection {
                id: "wire-s1".into(),
                from: 0,
                to: 1,
                wire_type: "active".into(),
                label: "vis:drivesTimeline".into(),
            },
            SeedConnection {
                id: "wire-s2".into(),
                from: 2,
                to: 3,
                wire_type: "active".into(),
                label: "aud:transportControl".into(),
            },
            SeedConnection {
                id: "wire-s3".into(),
                from: 2,
                to: 4,
                wire_type: "active".into(),
                label: "aud:routingGraph".into(),
            },
            SeedConnection {
                id: "wire-s4".into(),
                from: 2,
                to: 5,
                wire_type: "active".into(),
                label: "aud:spatialBinding".into(),
            },
            SeedConnection {
                id: "wire-s5".into(),
                from: 1,
                to: 3,
                wire_type: "active".into(),
                label: "ani:timeSync".into(),
            },
        ],
        panels: vec![SeedPanel {
            panel_type: "aura".into(),
            dock: DockPosition::Right,
        }],
    }
}
