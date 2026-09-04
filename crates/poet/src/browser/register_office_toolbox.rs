//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_office_toolbox(reg: &mut Registry) {
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
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "office:paragraph".into(),
                    label: "Paragraph & Headings".into(),
                    icon: "slide".into(),
                    description: "Configure heading levels, text alignment, and block formats."
                        .into(),
                },
                vec![],
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
