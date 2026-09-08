//! Wave 37 leftover Live chains — GraphMatch / GraphReasoning / Optimization / sampler / Capability.

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

pub(super) fn graph_reason_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool("scientific:graph_live_fuzzy_jaccard", "Fuzzy Jaccard", "GraphMatch.fuzzy_jaccard", "Fuzzy Jaccard via GraphMatch.fuzzy_jaccard.", "lab", "sci"),
        live_tool("scientific:graph_live_fuzzy_dice", "Fuzzy Dice", "GraphMatch.fuzzy_dice", "Fuzzy Dice via GraphMatch.fuzzy_dice.", "lab", "sci"),
        live_tool("scientific:graph_live_approximate_match", "Approximate match", "GraphMatch.approximate_match", "Hill-climb correspondence via GraphMatch.approximate_match.", "lab", "sci"),
        live_tool("scientific:graph_live_shortest_path", "Shortest path", "GraphReasoning.shortest_path", "Dijkstra via GraphReasoning.shortest_path.", "lab", "sci"),
        live_tool("scientific:graph_live_spreading_activation", "Spreading activation", "GraphReasoning.spreading_activation", "Spreading activation via GraphReasoning.spreading_activation.", "lab", "sci"),
        live_tool("scientific:graph_live_top_k", "Top-k activations", "GraphReasoning.top_k", "Top-k indices via GraphReasoning.top_k.", "lab", "sci"),
        live_tool("scientific:opt_live_hill_climb", "Hill climb", "Optimization.hill_climb", "1-D hill climb via Optimization.hill_climb.", "lab", "sci"),
        live_tool("scientific:opt_live_simulated_annealing", "Simulated annealing", "Optimization.simulated_annealing", "Bounded SA via Optimization.simulated_annealing.", "lab", "sci"),
        live_tool("scientific:opt_live_artificial_bee_colony", "Artificial bee colony", "Optimization.artificial_bee_colony", "ABC via Optimization.artificial_bee_colony.", "lab", "sci"),
    ]
}

pub(super) fn sampler_cap_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool("ai:sampler_live_configure", "Sampler configure", "sampler.configure", "Install sampler config via sampler.configure.", "ai", "ai"),
        live_tool("ai:sampler_live_constrain_enable", "Constrain enable", "sampler.constrain_enable", "Enable GBNF via sampler.constrain_enable.", "ai", "ai"),
        live_tool("ai:sampler_live_constrain_disable", "Constrain disable", "sampler.constrain_disable", "Disable GBNF via sampler.constrain_disable.", "ai", "ai"),
        live_tool("ai:sampler_live_constrain_reset", "Constrain reset", "sampler.constrain_reset", "Reset GBNF via sampler.constrain_reset.", "ai", "ai"),
        live_tool("ai:sampler_live_sample", "Sampler sample", "sampler.sample", "CPU sample via sampler.sample.", "ai", "ai"),
        live_tool("ai:cap_live_grant", "Capability grant", "Capability.grant", "Authorization grant via Capability.grant.", "ai", "ai"),
        live_tool("ai:cap_live_revoke", "Capability revoke", "Capability.revoke", "Revoke priority via Capability.revoke.", "ai", "ai"),
        live_tool("ai:cap_live_test_gating", "Test gating", "Capability.test_gating", "Sentinel gating via Capability.test_gating.", "ai", "ai"),
        live_tool("ai:cap_live_audit", "Capability audit", "Capability.audit", "Audit traces via Capability.audit.", "ai", "ai"),
        live_tool("ai:cap_live_declare", "Capability declare", "Capability.declare", "Declare a scope via Capability.declare.", "ai", "ai"),
    ]
}
