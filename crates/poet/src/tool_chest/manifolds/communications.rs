//! Communications manifold seed — pulse events, channels, conversations.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

use super::super::core::registry::{DockPosition, ManifoldSeed, SeedContainer, SeedPanel};

/// Communications manifold seed: conversations + channels + presence + WebRTC + webview.
pub fn communications_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "communications".into(),
        label: "Communications".into(),
        icon: "radio".into(),
        ontology_prefix: "comm".into(),
        description:
            "Conversations, channels, notifications, presence, WebRTC, webview, and pulse events."
                .into(),
        containers: vec![
            SeedContainer {
                container_type: "conversations".into(),
                title: "Conversations".into(),
                x: 80.0,
                y: 60.0,
                width: 420.0,
                height: 360.0,
                z: 100.0,
                honesty: "missing".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "channels".into(),
                title: "Channels".into(),
                x: 520.0,
                y: 60.0,
                width: 380.0,
                height: 200.0,
                z: 100.0,
                honesty: "missing".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "presence".into(),
                title: "Presence".into(),
                x: 520.0,
                y: 280.0,
                width: 380.0,
                height: 140.0,
                z: 100.0,
                honesty: "missing".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "webrtc".into(),
                title: "WebRTC Stream".into(),
                x: 80.0,
                y: 440.0,
                width: 420.0,
                height: 280.0,
                z: 100.0,
                honesty: "missing".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "webview".into(),
                title: "Web View".into(),
                x: 520.0,
                y: 440.0,
                width: 480.0,
                height: 280.0,
                z: 100.0,
                honesty: "missing".into(),
                ..Default::default()
            },
        ],
        connections: vec![],
        panels: vec![SeedPanel {
            panel_type: "pulse-panel".into(),
            dock: DockPosition::Bottom,
        }],
    }
}
