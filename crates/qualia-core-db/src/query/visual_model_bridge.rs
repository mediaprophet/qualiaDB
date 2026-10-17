//! Translates visual graph structures from the UI into `NQuin` evaluator inputs.

use crate::{q_hash, NQuin};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiLogicNode {
    pub id: usize,
    pub title: String,
    pub kind: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiLogicEdge {
    pub from: usize,
    pub to: usize,
    pub label: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiLogicGraph {
    pub nodes: Vec<UiLogicNode>,
    pub edges: Vec<UiLogicEdge>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvaluationReport {
    pub contradictions: Vec<usize>, // IDs of contradicting edges
    pub validations: Vec<usize>,
}

/// Converts the graph to a series of zero-heap semantic Quins.
/// Only writes to the pre-allocated slice up to its capacity, returning the number of Quins written.
pub fn translate_graph_to_quins(graph: &UiLogicGraph, out: &mut [NQuin]) -> usize {
    let mut count = 0;

    // Each edge becomes a Quin: Subject(from) -> Predicate(label) -> Object(to)
    for edge in &graph.edges {
        if count >= out.len() {
            break; // Slice full
        }

        let from_node = graph.nodes.iter().find(|n| n.id == edge.from);
        let to_node = graph.nodes.iter().find(|n| n.id == edge.to);

        if let (Some(f), Some(t)) = (from_node, to_node) {
            let mut q = NQuin::default();
            q.subject = q_hash(&f.title);
            q.predicate = q_hash(&edge.label);
            q.object = q_hash(&t.title);

            // Basic parity fold for completeness (subject ^ predicate ^ object ^ context)
            q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;

            out[count] = q;
            count += 1;
        }
    }

    count
}

/// Evaluates a translated logic graph and returns a report to update the UI.
pub fn evaluate_ui_graph(graph: &UiLogicGraph) -> EvaluationReport {
    let mut out_quins = [NQuin::default(); 128];
    let quin_count = translate_graph_to_quins(graph, &mut out_quins);

    let mut contradictions = Vec::new();
    let mut validations = Vec::new();

    let mut consistent = [NQuin::default(); 128];
    let mut isolated = [NQuin::default(); 128];

    let (_, isolated_count) = crate::modalities::paraconsistent::route_paraconsistent(
        &out_quins[..quin_count],
        &mut consistent,
        &mut isolated,
    )
    .unwrap_or((0, 0));

    let isolated_subjects: Vec<u64> = isolated[..isolated_count]
        .iter()
        .map(|q| q.subject)
        .collect();

    for edge in &graph.edges {
        let is_status_contradiction = edge.status.eq_ignore_ascii_case("contradiction")
            || edge.label.eq_ignore_ascii_case("contradicts")
            || edge.label.eq_ignore_ascii_case("conflicts_with")
            || edge.label.eq_ignore_ascii_case("not");

        let from_title_hash = graph
            .nodes
            .iter()
            .find(|n| n.id == edge.from)
            .map(|n| q_hash(&n.title))
            .unwrap_or(0);

        let is_paraconsistent_isolated = isolated_subjects.contains(&from_title_hash);

        if is_status_contradiction || is_paraconsistent_isolated {
            contradictions.push(edge.from);
        } else {
            validations.push(edge.from);
        }
    }

    EvaluationReport {
        contradictions,
        validations,
    }
}
