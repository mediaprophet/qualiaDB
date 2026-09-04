//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_ai_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:co_author".into(),
                label: "Co-Author".into(),
                icon: "coauthor".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("capability:invoke".into()),
                ontology_prefix: "ai".into(),
                description: "Invoke co-author assistance.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:extractor".into(),
                label: "Extractor".into(),
                icon: "extractor".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("NLP.gazetteer_run".into()),
                ontology_prefix: "ai".into(),
                description: "Extract entities from text.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:sentinel".into(),
                label: "Sentinel Guard".into(),
                icon: "sentinel".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Sentinel.inspect".into()),
                ontology_prefix: "ai".into(),
                description: "Invoke sentinel monitoring.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:triad".into(),
                label: "+ Triad Viewport".into(),
                icon: "triad".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: None,
                ontology_prefix: "ai".into(),
                description: "Place a triad (q42+p64+d10) container.".into(),
            },
            ActionType::Query,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "ai".into(),
            label: "AI Co-Pilot & Sentinel".into(),
            icon: "ai".into(),
            ontology_prefix: "ai".into(),
            description: "Resident GGUF LLMs, Epistemic Halo guard, and Triad execution.".into(),
            enabled_by_default: true,
            family: "ai".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "ai:copilot".into(),
                    label: "Sentinel Guard & Model".into(),
                    icon: "ai".into(),
                    description:
                        "Select resident GGUF model, halo confidence threshold, and temperature."
                            .into(),
                },
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "ai:tools".into(),
                    label: "Co-Pilot Capabilities".into(),
                    icon: "tools".into(),
                    description: "Invoke co-authoring, text extraction, and triad viewports."
                        .into(),
                },
                tools,
            ),
        ],
    ));
}
