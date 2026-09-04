//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_epistemic_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "epistemic:tag_objective".into(),
                label: "Tag Objective".into(),
                icon: "objective".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:annotate".into()),
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
                capability_scope: Some("graph:annotate".into()),
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
                capability_scope: Some("graph:annotate".into()),
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
                capability_scope: Some("graph:annotate".into()),
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
        vec![ToolChain::new(
            ToolChainMetadata {
                id: "epistemic:modalities".into(),
                label: "Epistemic Modalities".into(),
                icon: "modalities".into(),
                description: "Set the epistemic modality of a selected node.".into(),
            },
            tools,
        )],
    ));
}
