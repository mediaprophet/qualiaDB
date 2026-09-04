//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_office_toolbox(reg: &mut Registry) {
    let typography_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "office:typography_bold".into(),
                label: "Bold".into(),
                icon: "bold".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "hm".into(),
                description: "Apply bold styling to the selected document editor.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "office:typography_italic".into(),
                label: "Italic".into(),
                icon: "italic".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "hm".into(),
                description: "Apply italic styling to the selected document editor.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "office:typography_code".into(),
                label: "Code".into(),
                icon: "code".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "hm".into(),
                description: "Use a monospace code treatment for the selected editor.".into(),
            },
            ActionType::Invoke,
        )),
    ];
    let paragraph_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "office:paragraph_heading".into(),
                label: "Heading".into(),
                icon: "heading".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "hm".into(),
                description: "Promote the selected document editor to a heading block.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "office:paragraph_align_left".into(),
                label: "Align left".into(),
                icon: "align-left".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "hm".into(),
                description: "Align the selected document editor to the left.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "office:paragraph_align_center".into(),
                label: "Align center".into(),
                icon: "align-center".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "hm".into(),
                description: "Center the selected document editor.".into(),
            },
            ActionType::Invoke,
        )),
    ];
    let container_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "office:place_doc".into(),
                label: "+ Document".into(),
                icon: "doc".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "hm".into(),
                description: "Place a rich text document container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "office:place_ontology".into(),
                label: "+ Ontology".into(),
                icon: "ontology".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "ont".into(),
                description: "Place an ontology browser container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "office:place_slide".into(),
                label: "+ Slide".into(),
                icon: "slide".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "hm".into(),
                description: "Place a presentation slide container.".into(),
            },
            ActionType::Query,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "office".into(),
            label: "Word Processor & CML".into(),
            icon: "office".into(),
            ontology_prefix: "hm".into(),
            description: "Documents, typography, ontologies, and presentation slides.".into(),
            enabled_by_default: true,
            family: "authoring".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "office:typography".into(),
                    label: "Typography & Fonts".into(),
                    icon: "doc".into(),
                    description:
                        "Select font family, size, styles (Bold/Italic/Code), and text colors."
                            .into(),
                },
                typography_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "office:paragraph".into(),
                    label: "Paragraph & Headings".into(),
                    icon: "slide".into(),
                    description: "Configure heading levels, text alignment, and block formats."
                        .into(),
                },
                paragraph_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "office:containers".into(),
                    label: "Office Containers".into(),
                    icon: "containers".into(),
                    description: "Place documents, ontology lenses, and slides on canvas.".into(),
                },
                container_tools,
            ),
            // G-POET-TOOLCHEST: first live ALL_BOUND bind — GraphDatabase.sparql
            ToolChain::new(
                ToolChainMetadata {
                    id: "office:graph".into(),
                    label: "Graph Query".into(),
                    icon: "graph".into(),
                    description: "Run SPARQL against the live QualiaDB graph (GraphDatabase.sparql).".into(),
                },
                vec![Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "graph:sparql_query".into(),
                        label: "Run SPARQL".into(),
                        icon: "query".into(),
                        kind: ToolKind::Query,
                        capability_scope: Some("GraphDatabase.sparql".into()),
                        ontology_prefix: "ont".into(),
                        description: "Invoke live Capability.method GraphDatabase.sparql via the local daemon. Select SPARQL text first, or a bounded ASK is used.".into(),
                    },
                    ActionType::Query,
                ))],
            ),
        ],
    ));
}
