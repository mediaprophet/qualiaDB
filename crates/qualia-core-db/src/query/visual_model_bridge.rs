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
    let _quin_count = translate_graph_to_quins(graph, &mut out_quins);

    // Mock evaluation for now (Paraconsistent Logic / SHACL checking would hook here)
    // We just return dummy contradiction detection for demonstration.
    let mut contradictions = Vec::new();
    let mut validations = Vec::new();

    for edge in &graph.edges {
        if edge.label == "object" {
            contradictions.push(edge.from); // dummy logic
        } else {
            validations.push(edge.from);
        }
    }

    EvaluationReport {
        contradictions,
        validations,
    }
}
