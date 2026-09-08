//! Part of poet browser toolbox registration.

use super::*;

fn office_live_tool(
    id: &'static str,
    label: &'static str,
    scope: &'static str,
    description: &'static str,
) -> Box<dyn crate::tool_chest::core::tool::Tool> {
    Box::new(SimpleTool::new(
        ToolMetadata {
            id: id.into(),
            label: label.into(),
            icon: "doc".into(),
            kind: ToolKind::RunAction,
            capability_scope: Some(scope.into()),
            ontology_prefix: "hm".into(),
            description: description.into(),
        },
        ActionType::Invoke,
    ))
}

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
                capability_scope: Some(SCOPE_PLACE.into()),
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
                capability_scope: Some(SCOPE_PLACE.into()),
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
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "hm".into(),
                description: "Place a presentation slide container.".into(),
            },
            ActionType::Query,
        )),
    ];

    let asset_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        office_live_tool("office:asset_create", "Create asset", "Asset.create", "Create an aspect-graph record."),
        office_live_tool(
            "office:asset_add_temporal",
            "Add temporal aspect",
            "Asset.add_temporal",
            "Add a temporal aspect to an in-memory asset.",
        ),
        office_live_tool("office:asset_add_topic", "Add topic", "Asset.add_topic", "Associate a topic with an asset."),
        office_live_tool(
            "office:asset_set_spatial",
            "Set spatial anchor",
            "Asset.set_spatial",
            "Set a spatial anchor on an in-memory asset.",
        ),
        office_live_tool("office:asset_compile", "Compile asset", "Asset.compile", "Compile an asset to graph quins."),
        office_live_tool(
            "office:asset_temporal_span",
            "Temporal span",
            "Asset.temporal_span",
            "Span between earliest and latest aspects.",
        ),
        office_live_tool(
            "office:asset_query_aspects",
            "Query aspects",
            "Asset.query_aspects",
            "Query temporal aspects by kind.",
        ),
        office_live_tool("office:asset_persist", "Persist asset", "Asset.persist", "Persist an asset record."),
        office_live_tool("office:asset_resolve", "Resolve asset", "Asset.resolve", "Resolve a persisted asset by id."),
        office_live_tool(
            "office:asset_resolve_by_spatial",
            "Resolve by spatial",
            "Asset.resolve_by_spatial",
            "Resolve assets by spatial anchor.",
        ),
        office_live_tool(
            "office:asset_resolve_by_topic",
            "Resolve by topic",
            "Asset.resolve_by_topic",
            "Resolve assets by topic.",
        ),
        office_live_tool(
            "office:asset_resolve_by_temporal",
            "Resolve by temporal",
            "Asset.resolve_by_temporal",
            "Resolve assets by temporal aspect kind.",
        ),
        office_live_tool("office:asset_list", "List assets", "Asset.list", "List persisted asset ids."),
        office_live_tool("office:asset_count", "Count assets", "Asset.count", "Count persisted assets."),
        office_live_tool(
            "office:asset_persist_create",
            "Persist create",
            "Asset.persist_create",
            "Create and persist an asset in one call.",
        ),
        office_live_tool(
            "office:asset_persist_add_temporal",
            "Persist add temporal",
            "Asset.persist_add_temporal",
            "Add a temporal aspect to a persisted asset.",
        ),
        office_live_tool(
            "office:asset_persist_add_topic",
            "Persist add topic",
            "Asset.persist_add_topic",
            "Add a topic to a persisted asset.",
        ),
        office_live_tool(
            "office:asset_persist_set_spatial",
            "Persist set spatial",
            "Asset.persist_set_spatial",
            "Set a spatial anchor on a persisted asset.",
        ),
        office_live_tool(
            "office:asset_persist_compile",
            "Persist compile",
            "Asset.persist_compile",
            "Compile a persisted asset to graph quins.",
        ),
        office_live_tool(
            "office:asset_persist_temporal_span",
            "Persist temporal span",
            "Asset.persist_temporal_span",
            "Span between earliest and latest persisted aspects.",
        ),
        office_live_tool(
            "office:asset_persist_query_aspects",
            "Persist query aspects",
            "Asset.persist_query_aspects",
            "Query persisted temporal aspects by kind.",
        ),
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
            // G-POET-TOOLCHEST: second live chain — N3Logic.evaluate + SHACL.validate
            ToolChain::new(
                ToolChainMetadata {
                    id: "office:shapes".into(),
                    label: "Shapes & Rules".into(),
                    icon: "ontology".into(),
                    description: "Evaluate N3 and validate SHACL on live N3Logic.evaluate / SHACL.validate.".into(),
                },
                vec![
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "n3:evaluate".into(),
                            label: "Evaluate N3".into(),
                            icon: "query".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("N3Logic.evaluate".into()),
                            ontology_prefix: "ont".into(),
                            description: "Invoke live Capability.method N3Logic.evaluate. Selected document text is the N3 source; a small UTF-8 sample is used if empty. Standalone Poet reports a local sketch; rule firing needs the daemon.".into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "shacl:validate".into(),
                            label: "Validate SHACL".into(),
                            icon: "ontology".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("SHACL.validate".into()),
                            ontology_prefix: "ont".into(),
                            description: "Invoke live Capability.method SHACL.validate (minCount 1) on the selected container subject. Standalone Poet checks for a semantic annotation; quin-backed SHACL needs the daemon.".into(),
                        },
                        ActionType::Query,
                    )),
                ],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "office:asset".into(),
                    label: "Live Asset".into(),
                    icon: "doc".into(),
                    description: "Curated Asset.* create, aspect, persist, persist_* remainder, resolve, list, and count binds."
                        .into(),
                },
                asset_tools,
            ),
        ],
    ));
}
