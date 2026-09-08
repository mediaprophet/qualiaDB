//! Wave 36 leftover Live chains — Social / Finance / Forensic / ChatGraph / Corpus.

use super::*;

fn live_tool(
    id: &'static str,
    label: &'static str,
    scope: &'static str,
    description: &'static str,
    icon: &'static str,
    ontology_prefix: &'static str,
) -> Box<dyn crate::tool_chest::core::tool::Tool> {
    Box::new(SimpleTool::new(
        ToolMetadata {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            kind: ToolKind::RunAction,
            capability_scope: Some(scope.into()),
            ontology_prefix: ontology_prefix.into(),
            description: description.into(),
        },
        ActionType::Invoke,
    ))
}

pub(super) fn social_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool("social:live_gini", "Social Gini", "Social.gini", "Inequality via Social.gini.", "finance", "soc"),
        live_tool("social:live_lorenz", "Social Lorenz", "Social.lorenz", "Lorenz curve via Social.lorenz.", "finance", "soc"),
        live_tool("social:live_degree_centrality", "Degree centrality", "Social.degree_centrality", "Degree centrality via Social.degree_centrality.", "finance", "soc"),
        live_tool("social:live_lww", "LWW merge", "Social.lww", "Last-writer-wins merge via Social.lww.", "social", "soc"),
        live_tool("forensic:live_malfeasance_delta", "Malfeasance delta", "Forensic.malfeasance_delta", "Capital vs utility via Forensic.malfeasance_delta.", "finance", "econ"),
        live_tool("forensic:live_narrative_divergence", "Narrative divergence", "Forensic.narrative_divergence", "Factual vs fantasy via Forensic.narrative_divergence.", "finance", "econ"),
    ]
}

pub(super) fn finance_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool("finance:live_convert_currency", "Convert currency", "Finance.convert_currency", "Convert via Finance.convert_currency (rate_micros).", "finance", "econ"),
        live_tool("finance:live_multisig_check", "Multisig check", "Finance.multisig_check", "k-of-N check via Finance.multisig_check.", "finance", "econ"),
        live_tool("finance:live_ledger_balance", "Ledger balance", "Finance.ledger_balance", "Account balances via Finance.ledger_balance.", "finance", "econ"),
    ]
}

pub(super) fn graph_live_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool("comm:graph_live_corpus_load", "Corpus load", "Corpus.load", "Load a corpus path via Corpus.load.", "comm", "comm"),
        live_tool("comm:graph_live_corpus_parse", "Corpus parse", "Corpus.parse", "Parse corpus text via Corpus.parse.", "comm", "comm"),
        live_tool("comm:graph_live_validate_fragment", "Validate fragment", "ChatGraph.validate_fragment", "Validate a chat fragment via ChatGraph.validate_fragment.", "comm", "comm"),
        live_tool("comm:graph_live_link_reply", "Link reply", "ChatGraph.link_reply", "Link a reply via ChatGraph.link_reply.", "comm", "comm"),
        live_tool("comm:graph_live_add_social_post", "Add social post", "Interactive.add_social_post", "Add a post via Interactive.add_social_post.", "social", "soc"),
        live_tool("comm:graph_live_add_trigger", "Add trigger", "Interactive.add_trigger", "Add a timed trigger via Interactive.add_trigger.", "comm", "comm"),
        live_tool("comm:graph_live_second_screen_sync", "Second screen sync", "SecondScreen.sync", "Companion sync via SecondScreen.sync.", "comm", "comm"),
    ]
}
