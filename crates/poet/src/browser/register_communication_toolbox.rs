//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_communication_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "comm:place_social".into(),
                label: "+ Social Graph".into(),
                icon: "social".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "soc".into(),
                description: "Place a social graph container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "comm:place_webrtc".into(),
                label: "+ WebRTC Audio/Video".into(),
                icon: "webrtc".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: None,
                ontology_prefix: "comm".into(),
                description: "Place a WebRTC stream container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "comm:place_webview".into(),
                label: "+ Web Presence".into(),
                icon: "webview".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: None,
                ontology_prefix: "hm".into(),
                description: "Place a web frame container.".into(),
            },
            ActionType::Query,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "communication".into(),
            label: "Communication & Presence".into(),
            icon: "comm".into(),
            ontology_prefix: "comm".into(),
            description: "Pulse streams, social graphs, WebRTC, and web presence.".into(),
            enabled_by_default: true,
            family: "mail".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "comm:pulse".into(),
                    label: "Pulse Streams & Messaging".into(),
                    icon: "comm".into(),
                    description: "Select protocol and encryption tiers.".into(),
                },
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "comm:containers".into(),
                    label: "Presence Containers".into(),
                    icon: "containers".into(),
                    description: "Communication and streaming containers.".into(),
                },
                tools,
            ),
        ],
    ));
}
