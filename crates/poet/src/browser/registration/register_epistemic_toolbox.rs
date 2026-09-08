//! Part of poet browser toolbox registration.

use super::*;

fn research_live_tool(
    id: &'static str,
    label: &'static str,
    scope: &'static str,
    description: &'static str,
) -> Box<dyn crate::tool_chest::core::tool::Tool> {
    Box::new(SimpleTool::new(
        ToolMetadata {
            id: id.into(),
            label: label.into(),
            icon: "evaluate".into(),
            kind: ToolKind::RunAction,
            capability_scope: Some(scope.into()),
            ontology_prefix: "epi".into(),
            description: description.into(),
        },
        ActionType::Invoke,
    ))
}

pub(super) fn register_epistemic_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "epistemic:tag_objective".into(),
                label: "Tag Objective".into(),
                icon: "objective".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "epi".into(),
                description: "Tag selected node as objective epistemic modality.".into(),
            },
            ActionType::Annotate,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "epistemic:tag_subjective".into(),
                label: "Tag Subjective".into(),
                icon: "subjective".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "epi".into(),
                description: "Tag selected node as subjective epistemic modality.".into(),
            },
            ActionType::Annotate,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "epistemic:tag_intersubjective".into(),
                label: "Tag Intersubjective".into(),
                icon: "intersubjective".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "epi".into(),
                description: "Tag selected node as intersubjective epistemic modality.".into(),
            },
            ActionType::Annotate,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "epistemic:tag_normative".into(),
                label: "Tag Normative".into(),
                icon: "normative".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "epi".into(),
                description: "Tag selected node as normative epistemic modality.".into(),
            },
            ActionType::Annotate,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "epistemic".into(),
            label: "Epistemic Toolbox".into(),
            icon: "epistemic".into(),
            ontology_prefix: "epi".into(),
            description: "Tag nodes with epistemic modalities (objective, subjective, intersubjective, normative).".into(),
            enabled_by_default: true,
            family: "epistemic".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "epistemic:modalities".into(),
                    label: "Epistemic Modalities".into(),
                    icon: "modalities".into(),
                    description: "Set the epistemic modality of a selected node.".into(),
                },
                tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "epistemic:frame".into(),
                    label: "Epistemic Frame".into(),
                    icon: "evaluate".into(),
                    description: "Scan the live graph for knows/believes verdicts.".into(),
                },
                vec![
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "epistemic:evaluate".into(),
                            label: "Evaluate frame".into(),
                            icon: "evaluate".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("EpistemicLogic.evaluate".into()),
                            ontology_prefix: "epi".into(),
                            description: "Run EpistemicLogic.evaluate on the live quin frame."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "epistemic:paraconsistent_route".into(),
                            label: "Route contradictions".into(),
                            icon: "route".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("ParaconsistentLogic.route".into()),
                            ontology_prefix: "epi".into(),
                            description: "Route contradictory claims into an isolated context."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                ],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "research:live".into(),
                    label: "Live research".into(),
                    icon: "evaluate".into(),
                    description:
                        "Curated Research.* enquiry, corpus, dark-link, and inference binds."
                            .into(),
                },
                vec![
                    research_live_tool(
                        "research:live_new",
                        "New enquiry",
                        "Research.new",
                        "Create a research enquiry (data-research-id / data-purpose).",
                    ),
                    research_live_tool(
                        "research:live_set_purpose",
                        "Set purpose",
                        "Research.set_purpose",
                        "Set enquiry purpose via Research.set_purpose.",
                    ),
                    research_live_tool(
                        "research:live_define_scope",
                        "Define scope",
                        "Research.define_scope",
                        "Define enquiry scope from surface lines.",
                    ),
                    research_live_tool(
                        "research:live_add_constraint",
                        "Add constraint",
                        "Research.add_constraint",
                        "Add a constraint via Research.add_constraint.",
                    ),
                    research_live_tool(
                        "research:live_add_question",
                        "Add question",
                        "Research.add_question",
                        "Add a question via Research.add_question (data-question-id).",
                    ),
                    research_live_tool(
                        "research:live_link_questions",
                        "Link questions",
                        "Research.link_questions",
                        "Link two questions via Research.link_questions.",
                    ),
                    research_live_tool(
                        "research:live_add_corpus_item",
                        "Add corpus item",
                        "Research.add_corpus_item",
                        "Add a corpus item via Research.add_corpus_item.",
                    ),
                    research_live_tool(
                        "research:live_import_literature",
                        "Import literature",
                        "Research.import_literature",
                        "Import literature via Research.import_literature.",
                    ),
                    research_live_tool(
                        "research:live_import_dataset",
                        "Import dataset",
                        "Research.import_dataset",
                        "Import a dataset via Research.import_dataset.",
                    ),
                    research_live_tool(
                        "research:live_set_corpus_confidence",
                        "Corpus confidence",
                        "Research.set_corpus_confidence",
                        "Set corpus confidence (data-confidence).",
                    ),
                    research_live_tool(
                        "research:live_extract_from_corpus",
                        "Extract from corpus",
                        "Research.extract_from_corpus",
                        "Extract facts for a keyword via Research.extract_from_corpus.",
                    ),
                    research_live_tool(
                        "research:live_infer_dark_link",
                        "Infer dark link",
                        "Research.infer_dark_link",
                        "Infer a dark link via Research.infer_dark_link.",
                    ),
                    research_live_tool(
                        "research:live_detect_provenance_gaps",
                        "Provenance gaps",
                        "Research.detect_provenance_gaps",
                        "Detect provenance gaps in surface items.",
                    ),
                    research_live_tool(
                        "research:live_detect_concealment",
                        "Detect concealment",
                        "Research.detect_concealment",
                        "Detect concealment patterns via Research.detect_concealment.",
                    ),
                    research_live_tool(
                        "research:live_confirm_dark_link",
                        "Confirm dark link",
                        "Research.confirm_dark_link",
                        "Confirm a dark link via Research.confirm_dark_link.",
                    ),
                    research_live_tool(
                        "research:live_refute_dark_link",
                        "Refute dark link",
                        "Research.refute_dark_link",
                        "Refute a dark link via Research.refute_dark_link.",
                    ),
                    research_live_tool(
                        "research:live_make_inference",
                        "Make inference",
                        "Research.make_inference",
                        "Record a premise→conclusion via Research.make_inference.",
                    ),
                    research_live_tool(
                        "research:live_chain_inference",
                        "Chain inference",
                        "Research.chain_inference",
                        "Chain an inference via Research.chain_inference.",
                    ),
                    research_live_tool(
                        "research:live_set_inference_confidence",
                        "Inference confidence",
                        "Research.set_inference_confidence",
                        "Set inference confidence via Research.set_inference_confidence.",
                    ),
                    research_live_tool(
                        "research:live_validate_inference",
                        "Validate inference",
                        "Research.validate_inference",
                        "Validate an inference via Research.validate_inference.",
                    ),
                ],
            ),
        ],
    ));
}
