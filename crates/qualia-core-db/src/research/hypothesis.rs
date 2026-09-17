//! Hypothesis graph — multi-agent collaborative hypothesis revision.

use std::collections::BTreeMap;

/// A node in the hypothesis graph.
#[derive(Debug, Clone)]
pub struct HypothesisNode {
    pub id: String,
    pub statement: String,
    pub agent_id: String,
    pub evaluations: Vec<Evaluation>,
    pub dark_links: Vec<String>,
    pub gaps: Vec<String>,
    pub status: NodeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStatus {
    Open,
    Bridged,
    Closed,
    Reframed,
    Merged,
}

/// An evaluation contributed by an agent.
#[derive(Debug, Clone)]
pub struct Evaluation {
    pub agent_id: String,
    pub assessment: String,
    pub score: f64,
    pub timestamp: String,
}

/// A revision of a hypothesis node.
#[derive(Debug, Clone)]
pub struct HypothesisRevision {
    pub node_id: String,
    pub revision_id: String,
    pub old_statement: String,
    pub new_statement: String,
    pub agent_id: String,
    pub timestamp: String,
}

/// The hypothesis graph — multi-agent collaborative structure.
#[derive(Debug, Clone, Default)]
pub struct HypothesisGraph {
    pub nodes: BTreeMap<String, HypothesisNode>,
    pub revisions: Vec<HypothesisRevision>,
    pub subscribers: BTreeMap<String, Vec<String>>, // node_id -> agent_ids
}

impl HypothesisGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(&mut self, node: HypothesisNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_to_graph(&mut self, node_id: &str, evaluation: Evaluation) -> bool {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.evaluations.push(evaluation);
            true
        } else {
            false
        }
    }

    pub fn contribute_evaluation(&mut self, node_id: &str, evaluation: Evaluation) -> bool {
        self.add_to_graph(node_id, evaluation)
    }

    pub fn bridge_dark_link(&mut self, node_id: &str, dark_link_id: &str) -> bool {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.dark_links.push(dark_link_id.to_string());
            node.status = NodeStatus::Bridged;
            true
        } else {
            false
        }
    }

    pub fn reframe_hypothesis(
        &mut self,
        node_id: &str,
        new_statement: &str,
        agent_id: &str,
        timestamp: &str,
    ) -> bool {
        if let Some(node) = self.nodes.get_mut(node_id) {
            let revision = HypothesisRevision {
                node_id: node_id.to_string(),
                revision_id: format!("rev_{}", self.revisions.len()),
                old_statement: node.statement.clone(),
                new_statement: new_statement.to_string(),
                agent_id: agent_id.to_string(),
                timestamp: timestamp.to_string(),
            };
            node.statement = new_statement.to_string();
            node.status = NodeStatus::Reframed;
            self.revisions.push(revision);
            true
        } else {
            false
        }
    }

    pub fn merge_hypotheses(
        &mut self,
        node1_id: &str,
        node2_id: &str,
        merged_statement: &str,
        agent_id: &str,
        timestamp: &str,
    ) -> bool {
        if !self.nodes.contains_key(node1_id) || !self.nodes.contains_key(node2_id) {
            return false;
        }
        // Mark both as merged and create a combined statement on node1.
        if let Some(node) = self.nodes.get_mut(node1_id) {
            node.statement = merged_statement.to_string();
            node.status = NodeStatus::Merged;
        }
        if let Some(node) = self.nodes.get_mut(node2_id) {
            node.status = NodeStatus::Merged;
        }
        let revision = HypothesisRevision {
            node_id: node1_id.to_string(),
            revision_id: format!("merge_{}", self.revisions.len()),
            old_statement: "pre-merge".to_string(),
            new_statement: merged_statement.to_string(),
            agent_id: agent_id.to_string(),
            timestamp: timestamp.to_string(),
        };
        self.revisions.push(revision);
        true
    }

    pub fn flag_gap(&mut self, node_id: &str, gap: &str) -> bool {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.gaps.push(gap.to_string());
            true
        } else {
            false
        }
    }

    pub fn close_gap(&mut self, node_id: &str, gap: &str) -> bool {
        if let Some(node) = self.nodes.get_mut(node_id) {
            if let Some(pos) = node.gaps.iter().position(|g| g == gap) {
                node.gaps.remove(pos);
                if node.gaps.is_empty() {
                    node.status = NodeStatus::Closed;
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn create_revision(
        &mut self,
        node_id: &str,
        new_statement: &str,
        agent_id: &str,
        timestamp: &str,
    ) -> Option<String> {
        let rev_id = format!("rev_{}", self.revisions.len());
        if self.reframe_hypothesis(node_id, new_statement, agent_id, timestamp) {
            Some(rev_id)
        } else {
            None
        }
    }

    pub fn diff_revisions(
        &self,
        rev1_id: &str,
        rev2_id: &str,
    ) -> Option<(&HypothesisRevision, &HypothesisRevision)> {
        let r1 = self.revisions.iter().find(|r| r.revision_id == rev1_id)?;
        let r2 = self.revisions.iter().find(|r| r.revision_id == rev2_id)?;
        Some((r1, r2))
    }

    pub fn subscribe_updates(&mut self, node_id: &str, agent_id: &str) {
        self.subscribers
            .entry(node_id.to_string())
            .or_default()
            .push(agent_id.to_string());
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn revision_count(&self) -> usize {
        self.revisions.len()
    }
}

impl HypothesisNode {
    pub fn new(id: &str, statement: &str, agent_id: &str) -> Self {
        Self {
            id: id.to_string(),
            statement: statement.to_string(),
            agent_id: agent_id.to_string(),
            evaluations: Vec::new(),
            dark_links: Vec::new(),
            gaps: Vec::new(),
            status: NodeStatus::Open,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_create() {
        let mut graph = HypothesisGraph::new();
        graph.create(HypothesisNode::new("h1", "X causes Y", "agent_a"));
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn graph_add_evaluation() {
        let mut graph = HypothesisGraph::new();
        graph.create(HypothesisNode::new("h1", "X causes Y", "agent_a"));
        graph.add_to_graph(
            "h1",
            Evaluation {
                agent_id: "agent_b".into(),
                assessment: "Strong support".into(),
                score: 0.9,
                timestamp: "2024-01-01".into(),
            },
        );
        assert_eq!(graph.nodes.get("h1").unwrap().evaluations.len(), 1);
    }

    #[test]
    fn graph_bridge_dark_link() {
        let mut graph = HypothesisGraph::new();
        graph.create(HypothesisNode::new("h1", "X causes Y", "agent_a"));
        graph.bridge_dark_link("h1", "dl1");
        assert_eq!(graph.nodes.get("h1").unwrap().status, NodeStatus::Bridged);
    }

    #[test]
    fn graph_reframe() {
        let mut graph = HypothesisGraph::new();
        graph.create(HypothesisNode::new("h1", "X causes Y", "agent_a"));
        graph.reframe_hypothesis("h1", "X and Z jointly cause Y", "agent_b", "2024-02-01");
        assert_eq!(graph.nodes.get("h1").unwrap().status, NodeStatus::Reframed);
        assert_eq!(graph.revision_count(), 1);
    }

    #[test]
    fn graph_merge() {
        let mut graph = HypothesisGraph::new();
        graph.create(HypothesisNode::new("h1", "X causes Y", "agent_a"));
        graph.create(HypothesisNode::new("h2", "Z causes Y", "agent_b"));
        assert!(graph.merge_hypotheses(
            "h1",
            "h2",
            "X and Z jointly cause Y",
            "agent_c",
            "2024-03-01"
        ));
        assert_eq!(graph.nodes.get("h1").unwrap().status, NodeStatus::Merged);
        assert_eq!(graph.nodes.get("h2").unwrap().status, NodeStatus::Merged);
    }

    #[test]
    fn graph_flag_and_close_gap() {
        let mut graph = HypothesisGraph::new();
        graph.create(HypothesisNode::new("h1", "X causes Y", "agent_a"));
        graph.flag_gap("h1", "Missing evidence for X");
        assert_eq!(graph.nodes.get("h1").unwrap().gaps.len(), 1);
        graph.close_gap("h1", "Missing evidence for X");
        assert!(graph.nodes.get("h1").unwrap().gaps.is_empty());
        assert_eq!(graph.nodes.get("h1").unwrap().status, NodeStatus::Closed);
    }

    #[test]
    fn graph_subscribe() {
        let mut graph = HypothesisGraph::new();
        graph.create(HypothesisNode::new("h1", "X causes Y", "agent_a"));
        graph.subscribe_updates("h1", "agent_b");
        assert!(graph
            .subscribers
            .get("h1")
            .unwrap()
            .contains(&"agent_b".to_string()));
    }

    #[test]
    fn graph_diff_revisions() {
        let mut graph = HypothesisGraph::new();
        graph.create(HypothesisNode::new("h1", "Original", "agent_a"));
        graph.reframe_hypothesis("h1", "Revised 1", "agent_b", "t1");
        graph.reframe_hypothesis("h1", "Revised 2", "agent_c", "t2");
        let diff = graph.diff_revisions("rev_0", "rev_1");
        assert!(diff.is_some());
    }
}
