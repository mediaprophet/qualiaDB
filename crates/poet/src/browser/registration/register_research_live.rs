//! Research Live Tool Chest chain — Host-bound `Research.*` dual-path tools.

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

pub(super) fn research_live_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
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
        research_live_tool(
            "research:live_new_investigation",
            "New investigation",
            "Research.new_investigation",
            "Create an investigation via Research.new_investigation.",
        ),
        research_live_tool(
            "research:live_collect_evidence",
            "Collect evidence",
            "Research.collect_evidence",
            "Collect evidence via Research.collect_evidence.",
        ),
        research_live_tool(
            "research:live_set_reliability",
            "Set reliability",
            "Research.set_reliability",
            "Set evidence reliability via Research.set_reliability.",
        ),
        research_live_tool(
            "research:live_propose_hypothesis",
            "Propose hypothesis",
            "Research.propose_hypothesis",
            "Propose a hypothesis via Research.propose_hypothesis.",
        ),
        research_live_tool(
            "research:live_evaluate_evidence",
            "Evaluate evidence",
            "Research.evaluate_evidence",
            "Evaluate evidence against a hypothesis.",
        ),
        research_live_tool(
            "research:live_create_timeline",
            "Create timeline",
            "Research.create_timeline",
            "Add a timeline event via Research.create_timeline.",
        ),
        research_live_tool(
            "research:live_add_link",
            "Add link",
            "Research.add_link",
            "Add an investigation link via Research.add_link.",
        ),
        research_live_tool(
            "research:live_find_path",
            "Find path",
            "Research.find_path",
            "Find a path via Research.find_path.",
        ),
        research_live_tool(
            "research:live_create_hypothesis_graph",
            "Hypothesis graph",
            "Research.create_hypothesis_graph",
            "Create a hypothesis graph via Research.create_hypothesis_graph.",
        ),
        research_live_tool(
            "research:live_contribute_evaluation",
            "Contribute evaluation",
            "Research.contribute_evaluation",
            "Contribute an evaluation via Research.contribute_evaluation.",
        ),
        research_live_tool(
            "research:live_bridge_dark_link",
            "Bridge dark link",
            "Research.bridge_dark_link",
            "Bridge a dark link via Research.bridge_dark_link.",
        ),
        research_live_tool(
            "research:live_reframe_hypothesis",
            "Reframe hypothesis",
            "Research.reframe_hypothesis",
            "Reframe a hypothesis via Research.reframe_hypothesis.",
        ),
        research_live_tool(
            "research:live_merge_hypotheses",
            "Merge hypotheses",
            "Research.merge_hypotheses",
            "Merge two hypotheses via Research.merge_hypotheses.",
        ),
        research_live_tool(
            "research:live_flag_gap",
            "Flag gap",
            "Research.flag_gap",
            "Flag a gap via Research.flag_gap.",
        ),
        research_live_tool(
            "research:live_close_gap",
            "Close gap",
            "Research.close_gap",
            "Close a gap via Research.close_gap.",
        ),
        research_live_tool(
            "research:live_create_revision",
            "Create revision",
            "Research.create_revision",
            "Create a revision via Research.create_revision.",
        ),
        research_live_tool(
            "research:live_diff_revisions",
            "Diff revisions",
            "Research.diff_revisions",
            "Diff two revisions via Research.diff_revisions.",
        ),
        research_live_tool(
            "research:live_subscribe_updates",
            "Subscribe updates",
            "Research.subscribe_updates",
            "Subscribe to updates via Research.subscribe_updates.",
        ),
        research_live_tool(
            "research:live_create_assessment",
            "Create assessment",
            "Research.create_assessment",
            "Create an epistemic assessment via Research.create_assessment.",
        ),
        research_live_tool(
            "research:live_set_epistemic_mode",
            "Set epistemic mode",
            "Research.set_epistemic_mode",
            "Set epistemic mode via Research.set_epistemic_mode.",
        ),
    ]
}
