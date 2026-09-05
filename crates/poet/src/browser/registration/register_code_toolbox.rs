//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_code_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "code:place_vibe".into(),
                label: "+ VibeScript Cell".into(),
                icon: "vibe".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "vibe".into(),
                description: "Place a reactive VibeScript cell container.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "code:quin_statement".into(),
                label: "quin.statement".into(),
                icon: "quin".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "vibe".into(),
                description: "Construct a quin.statement.".into(),
            },
            ActionType::Mutate,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "code".into(),
            label: "Code IDE & Vibe REPL".into(),
            icon: "code".into(),
            ontology_prefix: "vibe".into(),
            description: "VibeScript 0.1, WebGPU WGSL, SPARQL, and reactive AST cells.".into(),
            enabled_by_default: true,
            family: "code".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "code:repl".into(),
                    label: "Runtime & Language Dialects".into(),
                    icon: "code".into(),
                    description:
                        "Select language dialect (Vibe/WGSL/Turtle) and configure gas budget."
                            .into(),
                },
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "code:tools".into(),
                    label: "IDE Cells & Statements".into(),
                    icon: "tools".into(),
                    description: "Place VibeScript cells and construct quin statements.".into(),
                },
                tools,
            ),
        ],
    ));
}
