//! Container interiors on the HyperCanvas stage.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::bodies::{
    BookmarksBody, CodecsBody, DocBody, DomainsBody, EconomicsBody, GitBody, HealthBody, IdeBody,
    JobCenterBody, MapBody, MediaBody, OntologyBody, ShadersBody, SheetBody, SocialBody, SolidBody,
    SubmanifoldBody,
};
use super::kinds::ContainerKind;
use super::vibe_console::VibeConsole;
use dioxus::prelude::*;

#[component]
pub fn NodeBody(kind: ContainerKind) -> Element {
    rsx! {
        match kind {
            ContainerKind::Code => rsx! {
                div { style: "display:grid;gap:8px;padding:4px;",
                    IdeBody {}
                    VibeConsole {}
                }
            },
            ContainerKind::Doc => rsx! { DocBody {} },
            ContainerKind::Sheet => rsx! { SheetBody {} },
            ContainerKind::Ontology => rsx! { OntologyBody {} },
            ContainerKind::Map => rsx! { MapBody {} },
            ContainerKind::Media => rsx! { MediaBody {} },
            ContainerKind::Social => rsx! { SocialBody {} },
            ContainerKind::Health => rsx! { HealthBody {} },
            ContainerKind::Subcanvas => rsx! { SubmanifoldBody {} },
            ContainerKind::Mesh3d => rsx! {
                div { style: "padding:12px;",
                    h4 { style: "margin:0 0 6px;color:var(--accent-cyan);font-size:12px;", "3D Kinematics & CCF Anatomy" }
                    p { style: muted(), "3D vocal-tract & cardiac .10d mesh representation. Projected via shared wgpu 30 device." }
                }
            },
            ContainerKind::WebRtc => rsx! {
                div { style: "padding:12px;",
                    h4 { style: "margin:0 0 6px;color:var(--accent-emerald);font-size:12px;", "WebRTC P2P Data Mesh" }
                    p { style: muted(), "Zero-leak P2P direct data channel. Peer DID hash verified with Ed25519 root signatures." }
                }
            },
            ContainerKind::Webview => rsx! {
                div { style: "padding:12px;",
                    h4 { style: "margin:0 0 6px;color:var(--accent-purple);font-size:12px;", "Sandboxed Dialectical Webview" }
                    p { style: muted(), "Sandboxed web reader harvesting W3C RDFa and JSON-LD semantic claims." }
                }
            },
            ContainerKind::Portal => rsx! {
                div { style: "padding:12px;",
                    h4 { style: "margin:0 0 6px;color:var(--accent-gold);font-size:12px;", "Wormhole Portal" }
                    p { style: muted(), "Wormhole portal to inalienable commons network graph." }
                }
            },
            ContainerKind::Mail => rsx! { DomainsBody {} },
            ContainerKind::Chora => rsx! {
                div { style: "padding:12px;",
                    h4 { style: "margin:0 0 6px;color:var(--accent-teal);font-size:12px;", "Chora 4D Dialectical Reader" }
                    p { style: muted(), "4D spatio-temporal knowledge graph reader with dialectical trust weights." }
                }
            },
            ContainerKind::ErpKanban => rsx! {
                div { style: "padding:10px;display:grid;grid-template-columns:repeat(3,1fr);gap:8px;",
                    div { style: "background:rgba(255,255,255,0.03);border:1px solid rgba(255,255,255,0.06);border-radius:6px;padding:6px;",
                        span { style: "font-size:10px;font-weight:700;color:#94a3b8;text-transform:uppercase;", "To Do (2)" }
                        div { style: "margin-top:6px;background:#141a23;padding:6px;border-radius:4px;font-size:11px;", "Task A: Epistemic Norms" }
                    }
                    div { style: "background:rgba(255,255,255,0.03);border:1px solid rgba(255,255,255,0.06);border-radius:6px;padding:6px;",
                        span { style: "font-size:10px;font-weight:700;color:var(--accent-cyan);text-transform:uppercase;", "In Progress (1)" }
                        div { style: "margin-top:6px;background:#141a23;padding:6px;border-radius:4px;font-size:11px;border-left:2px solid var(--accent-cyan);", "Sprint 1: Poet Shell" }
                    }
                    div { style: "background:rgba(255,255,255,0.03);border:1px solid rgba(255,255,255,0.06);border-radius:6px;padding:6px;",
                        span { style: "font-size:10px;font-weight:700;color:#00E676;text-transform:uppercase;", "Done (5)" }
                        div { style: "margin-top:6px;background:#141a23;padding:6px;border-radius:4px;font-size:11px;border-left:2px solid #00E676;", "Specs 00..23 Complete" }
                    }
                }
            },
            ContainerKind::GitForge => rsx! { GitBody {} },
            ContainerKind::SolidHub => rsx! { SolidBody {} },
            ContainerKind::Economics => rsx! { EconomicsBody {} },
            ContainerKind::JobCenter => rsx! { JobCenterBody {} },
            ContainerKind::Bookmarks => rsx! { BookmarksBody {} },
            ContainerKind::Shaders => rsx! { ShadersBody {} },
            ContainerKind::Codecs => rsx! { CodecsBody {} },
            ContainerKind::Domains => rsx! { DomainsBody {} },
        }
    }
}

fn muted() -> &'static str {
    "margin:0;color:var(--text-secondary);font-size:12px;line-height:1.45;"
}
