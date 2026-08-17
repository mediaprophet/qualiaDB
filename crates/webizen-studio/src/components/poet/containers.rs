//! Container interiors on the HyperCanvas stage.

use super::bodies::{
    DocBody, HealthBody, MapBody, MediaBody, OntologyBody, SheetBody, SocialBody, SubmanifoldBody,
};
use super::kinds::ContainerKind;
use super::vibe_console::VibeConsole;
use dioxus::prelude::*;

#[component]
pub fn NodeBody(kind: ContainerKind) -> Element {
    rsx! {
        match kind {
            ContainerKind::Code => rsx! { VibeConsole {} },
            ContainerKind::Doc => rsx! { DocBody {} },
            ContainerKind::Sheet => rsx! { SheetBody {} },
            ContainerKind::Ontology => rsx! { OntologyBody {} },
            ContainerKind::Map => rsx! { MapBody {} },
            ContainerKind::Media => rsx! { MediaBody {} },
            ContainerKind::Social => rsx! { SocialBody {} },
            ContainerKind::Health => rsx! { HealthBody {} },
            ContainerKind::Subcanvas => rsx! { SubmanifoldBody {} },
            ContainerKind::Mesh3d => rsx! {
                p { style: muted(), "3D vocal-tract / .d10 mesh. Native swapchain is /gpu-viewport. Present — no second adapter." }
            },
            ContainerKind::WebRtc => rsx! {
                p { style: muted(), "WebRTC container is Present. No fake live stream. Peer hash via Net.peer_hash from a Vibe cell." }
            },
            ContainerKind::Webview => rsx! {
                p { style: muted(), "Sandboxed webview is Present. Capability-gated q-web-frame comes next." }
            },
            ContainerKind::Portal => rsx! {
                p { style: muted(), "Wormhole portal is Present. Target IRI is recorded; enter-sub-manifold zoom is later." }
            },
        }
    }
}

fn muted() -> &'static str {
    "margin:0;color:var(--text-secondary);font-size:12px;line-height:1.45;"
}
