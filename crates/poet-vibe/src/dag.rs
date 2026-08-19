//! A6 — Multi-Agent DAGs & Autonomous Control Units.
//!
//! Native DAG (Directed Acyclic Graph) pipeline definitions for multi-agent
//! orchestration. Each node represents an agent step; edges represent
//! dependencies. Control Units (autonomous routers) can select the next
//! node based on blackboard state and policy constraints.
//!
//! ## Design
//!
//! - **DagNode**: A single agent step with inputs, outputs, effect class,
//!   and an optional capability requirement.
//! - **DagEdge**: A dependency edge (producer → consumer).
//! - **DagPipeline**: The full DAG with topological ordering and cycle
//!   detection.
//! - **ControlUnit**: An autonomous router that selects the next node(s)
//!   to execute based on the current state and policy constraints.
//! - **JudgeFrame**: An isolated verification frame for checking agent
//!   outputs before committing them.
//!
//! ## Integration
//!
//! - Uses A4 (`ast_query`) for policy enforcement on node definitions.
//! - Uses A5 (`blackboard`) for inter-agent state channels.
//! - Verification frames use bounded arrays (zero-heap on hot paths).

use std::collections::HashMap;

// ── DAG Node ───────────────────────────────────────────────────────────────

/// Maximum nodes per DAG.
pub const MAX_DAG_NODES: usize = 256;
/// Maximum edges per DAG.
pub const MAX_DAG_EDGES: usize = 1024;
/// Maximum inputs/outputs per node.
pub const MAX_NODE_IO: usize = 16;

/// Effect class for a DAG node (mirrors `ast::EffectClass`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeEffect {
    Pure,
    Hot,
    Cold,
    Async,
    External,
}

/// A single agent step in the DAG.
#[derive(Debug, Clone)]
pub struct DagNode {
    /// Unique node ID (0..MAX_DAG_NODES).
    pub id: u32,
    /// Human-readable name.
    pub name: String,
    /// Effect class.
    pub effect: NodeEffect,
    /// Required capability IDs.
    pub capabilities: Vec<String>,
    /// Input channel names (from blackboard or upstream nodes).
    pub inputs: Vec<String>,
    /// Output channel names (to blackboard or downstream nodes).
    pub outputs: Vec<String>,
    /// Budget (max tokens / max steps).
    pub budget: u32,
    /// Whether this node is a Control Unit (autonomous router).
    pub is_control_unit: bool,
}

impl DagNode {
    pub fn new(id: u32, name: &str, effect: NodeEffect) -> Self {
        Self {
            id,
            name: name.to_string(),
            effect,
            capabilities: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            budget: 0,
            is_control_unit: false,
        }
    }

    pub fn with_capability(mut self, cap: &str) -> Self {
        self.capabilities.push(cap.to_string());
        self
    }

    pub fn with_input(mut self, channel: &str) -> Self {
        self.inputs.push(channel.to_string());
        self
    }

    pub fn with_output(mut self, channel: &str) -> Self {
        self.outputs.push(channel.to_string());
        self
    }

    pub fn with_budget(mut self, budget: u32) -> Self {
        self.budget = budget;
        self
    }

    pub fn as_control_unit(mut self) -> Self {
        self.is_control_unit = true;
        self
    }
}

// ── DAG Edge ───────────────────────────────────────────────────────────────

/// A dependency edge: `from` must complete before `to` starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DagEdge {
    pub from: u32,
    pub to: u32,
}

impl DagEdge {
    pub fn new(from: u32, to: u32) -> Self {
        Self { from, to }
    }
}

// ── DAG Pipeline ───────────────────────────────────────────────────────────

/// Errors that can occur during DAG construction or execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DagError {
    /// Node ID exceeds MAX_DAG_NODES.
    NodeIdOverflow,
    /// Too many edges.
    EdgeOverflow,
    /// Duplicate node ID.
    DuplicateNode,
    /// Edge references non-existent node.
    InvalidEdge,
    /// Cycle detected.
    CycleDetected,
    /// Node has no budget set.
    MissingBudget,
    /// Required capability not available.
    MissingCapability(String),
}

/// Execution status of a DAG node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// The DAG pipeline definition.
pub struct DagPipeline {
    nodes: HashMap<u32, DagNode>,
    edges: Vec<DagEdge>,
    /// Adjacency list: node_id → list of downstream node_ids.
    adjacency: HashMap<u32, Vec<u32>>,
    /// Reverse adjacency: node_id → list of upstream node_ids.
    reverse_adjacency: HashMap<u32, Vec<u32>>,
}

impl DagPipeline {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            adjacency: HashMap::new(),
            reverse_adjacency: HashMap::new(),
        }
    }

    /// Add a node to the DAG.
    pub fn add_node(&mut self, node: DagNode) -> Result<(), DagError> {
        if node.id as usize >= MAX_DAG_NODES {
            return Err(DagError::NodeIdOverflow);
        }
        if self.nodes.contains_key(&node.id) {
            return Err(DagError::DuplicateNode);
        }
        let id = node.id;
        self.nodes.insert(id, node);
        self.adjacency.entry(id).or_default();
        self.reverse_adjacency.entry(id).or_default();
        Ok(())
    }

    /// Add an edge to the DAG.
    pub fn add_edge(&mut self, edge: DagEdge) -> Result<(), DagError> {
        if self.edges.len() >= MAX_DAG_EDGES {
            return Err(DagError::EdgeOverflow);
        }
        if !self.nodes.contains_key(&edge.from) || !self.nodes.contains_key(&edge.to) {
            return Err(DagError::InvalidEdge);
        }
        self.edges.push(edge);
        self.adjacency.entry(edge.from).or_default().push(edge.to);
        self.reverse_adjacency.entry(edge.to).or_default().push(edge.from);
        Ok(())
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: u32) -> Option<&DagNode> {
        self.nodes.get(&id)
    }

    /// Get all nodes.
    pub fn nodes(&self) -> &HashMap<u32, DagNode> {
        &self.nodes
    }

    /// Get all edges.
    pub fn edges(&self) -> &[DagEdge] {
        &self.edges
    }

    /// Get the downstream nodes of a given node.
    pub fn downstream(&self, id: u32) -> &[u32] {
        self.adjacency.get(&id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get the upstream nodes of a given node.
    pub fn upstream(&self, id: u32) -> &[u32] {
        self.reverse_adjacency
            .get(&id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Detect cycles using DFS. Returns true if a cycle is found.
    pub fn has_cycle(&self) -> bool {
        let mut visited = HashMap::new();
        for &id in self.nodes.keys() {
            visited.insert(id, 0u8); // 0=unvisited, 1=in-progress, 2=done
        }
        for &id in self.nodes.keys() {
            if visited[&id] == 0 {
                if self.dfs_cycle(id, &mut visited) {
                    return true;
                }
            }
        }
        false
    }

    fn dfs_cycle(&self, id: u32, visited: &mut HashMap<u32, u8>) -> bool {
        visited.insert(id, 1);
        if let Some(downstream) = self.adjacency.get(&id) {
            for &next in downstream {
                let state = visited[&next];
                if state == 1 {
                    return true; // Back edge → cycle.
                }
                if state == 0 && self.dfs_cycle(next, visited) {
                    return true;
                }
            }
        }
        visited.insert(id, 2);
        false
    }

    /// Topological sort. Returns node IDs in dependency order.
    /// Returns Err if a cycle is detected.
    pub fn topological_sort(&self) -> Result<Vec<u32>, DagError> {
        if self.has_cycle() {
            return Err(DagError::CycleDetected);
        }
        let mut in_degree: HashMap<u32, usize> = HashMap::new();
        for &id in self.nodes.keys() {
            in_degree.insert(id, self.reverse_adjacency.get(&id).map(|v| v.len()).unwrap_or(0));
        }
        let mut queue: Vec<u32> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();
        queue.sort();
        let mut result = Vec::new();
        while let Some(id) = queue.pop() {
            result.push(id);
            if let Some(downstream) = self.adjacency.get(&id) {
                for &next in downstream {
                    let d = in_degree.get_mut(&next).unwrap();
                    *d -= 1;
                    if *d == 0 {
                        queue.push(next);
                        queue.sort();
                    }
                }
            }
        }
        Ok(result)
    }

    /// Get the entry nodes (nodes with no upstream dependencies).
    pub fn entry_nodes(&self) -> Vec<u32> {
        self.nodes
            .keys()
            .filter(|&&id| self.upstream(id).is_empty())
            .copied()
            .collect()
    }

    /// Get the exit nodes (nodes with no downstream dependencies).
    pub fn exit_nodes(&self) -> Vec<u32> {
        self.nodes
            .keys()
            .filter(|&&id| self.downstream(id).is_empty())
            .copied()
            .collect()
    }

    /// Validate the DAG: check for cycles, missing budgets on hot nodes,
    /// and missing capabilities.
    pub fn validate(&self, available_caps: &[String]) -> Result<(), DagError> {
        if self.has_cycle() {
            return Err(DagError::CycleDetected);
        }
        for node in self.nodes.values() {
            // Hot and Async nodes must have a budget.
            if (node.effect == NodeEffect::Hot || node.effect == NodeEffect::Async)
                && node.budget == 0
            {
                return Err(DagError::MissingBudget);
            }
            // Check capabilities.
            for cap in &node.capabilities {
                if !available_caps.contains(cap) {
                    return Err(DagError::MissingCapability(cap.clone()));
                }
            }
        }
        Ok(())
    }

    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

impl Default for DagPipeline {
    fn default() -> Self {
        Self::new()
    }
}

// ── Control Unit (Autonomous Router) ───────────────────────────────────────

/// A Control Unit selects the next node(s) to execute based on the current
/// execution state and policy constraints.
pub struct ControlUnit {
    /// The node ID this control unit is attached to.
    pub node_id: u32,
    /// Strategy for selecting the next node.
    pub strategy: RouterStrategy,
}

/// Routing strategy for the control unit.
#[derive(Debug, Clone, PartialEq)]
pub enum RouterStrategy {
    /// Execute all downstream nodes in parallel.
    AllDownstream,
    /// Execute only the first downstream node (sequential).
    FirstDownstream,
    /// Execute the downstream node whose name matches.
    Named(String),
    /// Conditionally route based on a channel value.
    Conditional {
        channel: String,
        /// If the channel has any data, route to `if_present`.
        if_present: u32,
        /// Otherwise, route to `if_empty`.
        if_empty: u32,
    },
}

impl ControlUnit {
    pub fn new(node_id: u32, strategy: RouterStrategy) -> Self {
        Self {
            node_id,
            strategy,
        }
    }

    /// Select the next node(s) to execute given the current pipeline state
    /// and channel data.
    pub fn select_next(
        &self,
        pipeline: &DagPipeline,
        channel_has_data: &dyn Fn(&str) -> bool,
    ) -> Vec<u32> {
        match &self.strategy {
            RouterStrategy::AllDownstream => {
                pipeline.downstream(self.node_id).to_vec()
            }
            RouterStrategy::FirstDownstream => {
                pipeline.downstream(self.node_id).first().copied().into_iter().collect()
            }
            RouterStrategy::Named(name) => {
                pipeline
                    .downstream(self.node_id)
                    .iter()
                    .filter(|&&id| {
                        pipeline
                            .get_node(id)
                            .map(|n| n.name == *name)
                            .unwrap_or(false)
                    })
                    .copied()
                    .collect()
            }
            RouterStrategy::Conditional {
                channel,
                if_present,
                if_empty,
            } => {
                if channel_has_data(channel) {
                    vec![*if_present]
                } else {
                    vec![*if_empty]
                }
            }
        }
    }
}

// ── Execution State ────────────────────────────────────────────────────────

/// Tracks the execution state of all nodes in the DAG.
pub struct ExecutionState {
    statuses: HashMap<u32, NodeStatus>,
    /// Completed node IDs in order.
    completed_order: Vec<u32>,
}

impl ExecutionState {
    pub fn new(pipeline: &DagPipeline) -> Self {
        let statuses: HashMap<u32, NodeStatus> = pipeline
            .nodes()
            .keys()
            .map(|&id| (id, NodeStatus::Pending))
            .collect();
        Self {
            statuses,
            completed_order: Vec::new(),
        }
    }

    /// Get the status of a node.
    pub fn status(&self, id: u32) -> NodeStatus {
        self.statuses.get(&id).copied().unwrap_or(NodeStatus::Pending)
    }

    /// Set the status of a node.
    pub fn set_status(&mut self, id: u32, status: NodeStatus) {
        if status == NodeStatus::Completed {
            self.completed_order.push(id);
        }
        self.statuses.insert(id, status);
    }

    /// Get all nodes that are ready to execute (all upstream completed).
    pub fn ready_nodes(&self, pipeline: &DagPipeline) -> Vec<u32> {
        pipeline
            .nodes()
            .keys()
            .filter(|&&id| {
                self.status(id) == NodeStatus::Pending
                    && pipeline
                        .upstream(id)
                        .iter()
                        .all(|&up| self.status(up) == NodeStatus::Completed)
            })
            .copied()
            .collect()
    }

    /// Are all nodes completed?
    pub fn all_completed(&self) -> bool {
        self.statuses.values().all(|s| *s == NodeStatus::Completed)
    }

    /// Get the completion order.
    pub fn completed_order(&self) -> &[u32] {
        &self.completed_order
    }

    /// Count nodes by status.
    pub fn count_by_status(&self, status: NodeStatus) -> usize {
        self.statuses.values().filter(|s| **s == status).count()
    }
}

// ── Judge Frame (Isolated Verification) ────────────────────────────────────

/// Maximum claims per judge frame.
pub const MAX_JUDGE_CLAIMS: usize = 64;

/// A claim submitted to the judge frame for verification.
#[derive(Debug, Clone, PartialEq)]
pub struct JudgeClaim {
    /// The node that produced this claim.
    pub node_id: u32,
    /// The output channel name.
    pub channel: String,
    /// Whether the node claims success.
    pub success: bool,
    /// Optional error message.
    pub error: Option<String>,
}

/// A judge frame verifies agent outputs before committing them.
/// Uses a bounded array (zero-heap on hot paths).
pub struct JudgeFrame {
    claims: Vec<JudgeClaim>,
    verdicts: Vec<bool>,
}

impl JudgeFrame {
    pub fn new() -> Self {
        Self {
            claims: Vec::with_capacity(MAX_JUDGE_CLAIMS),
            verdicts: Vec::with_capacity(MAX_JUDGE_CLAIMS),
        }
    }

    /// Submit a claim for verification.
    pub fn submit(&mut self, claim: JudgeClaim) -> Result<(), DagError> {
        if self.claims.len() >= MAX_JUDGE_CLAIMS {
            return Err(DagError::EdgeOverflow); // Reuse for capacity.
        }
        self.claims.push(claim);
        Ok(())
    }

    /// Evaluate all submitted claims. A claim passes if:
    /// - The node claims success and has no error, OR
    /// - The node claims failure with an error message (honest failure).
    /// A claim fails if:
    /// - The node claims success but has an error, OR
    /// - The node claims failure but has no error message.
    pub fn evaluate(&mut self) {
        self.verdicts.clear();
        for claim in &self.claims {
            let passes = if claim.success {
                claim.error.is_none()
            } else {
                claim.error.is_some()
            };
            self.verdicts.push(passes);
        }
    }

    /// Get the verdicts.
    pub fn verdicts(&self) -> &[bool] {
        &self.verdicts
    }

    /// Did all claims pass?
    pub fn all_passed(&self) -> bool {
        !self.verdicts.is_empty() && self.verdicts.iter().all(|&v| v)
    }

    /// Get the number of submitted claims.
    pub fn claim_count(&self) -> usize {
        self.claims.len()
    }

    /// Reset the judge frame for reuse.
    pub fn reset(&mut self) {
        self.claims.clear();
        self.verdicts.clear();
    }
}

impl Default for JudgeFrame {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dag_node_construction() {
        let node = DagNode::new(0, "input", NodeEffect::Pure)
            .with_capability("math")
            .with_input("raw")
            .with_output("processed")
            .with_budget(100);
        assert_eq!(node.id, 0);
        assert_eq!(node.name, "input");
        assert_eq!(node.effect, NodeEffect::Pure);
        assert_eq!(node.capabilities, vec!["math"]);
        assert_eq!(node.inputs, vec!["raw"]);
        assert_eq!(node.outputs, vec!["processed"]);
        assert_eq!(node.budget, 100);
    }

    #[test]
    fn dag_pipeline_add_nodes_and_edges() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "a", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(1, "b", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(2, "c", NodeEffect::Pure)).unwrap();
        dag.add_edge(DagEdge::new(0, 1)).unwrap();
        dag.add_edge(DagEdge::new(1, 2)).unwrap();
        assert_eq!(dag.node_count(), 3);
        assert_eq!(dag.edge_count(), 2);
    }

    #[test]
    fn dag_pipeline_duplicate_node() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "a", NodeEffect::Pure)).unwrap();
        let result = dag.add_node(DagNode::new(0, "b", NodeEffect::Pure));
        assert_eq!(result, Err(DagError::DuplicateNode));
    }

    #[test]
    fn dag_pipeline_invalid_edge() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "a", NodeEffect::Pure)).unwrap();
        let result = dag.add_edge(DagEdge::new(0, 99));
        assert_eq!(result, Err(DagError::InvalidEdge));
    }

    #[test]
    fn dag_pipeline_no_cycle() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "a", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(1, "b", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(2, "c", NodeEffect::Pure)).unwrap();
        dag.add_edge(DagEdge::new(0, 1)).unwrap();
        dag.add_edge(DagEdge::new(1, 2)).unwrap();
        assert!(!dag.has_cycle());
    }

    #[test]
    fn dag_pipeline_cycle_detected() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "a", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(1, "b", NodeEffect::Pure)).unwrap();
        dag.add_edge(DagEdge::new(0, 1)).unwrap();
        dag.add_edge(DagEdge::new(1, 0)).unwrap();
        assert!(dag.has_cycle());
    }

    #[test]
    fn dag_topological_sort() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "a", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(1, "b", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(2, "c", NodeEffect::Pure)).unwrap();
        dag.add_edge(DagEdge::new(0, 1)).unwrap();
        dag.add_edge(DagEdge::new(1, 2)).unwrap();
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted, vec![0, 1, 2]);
    }

    #[test]
    fn dag_topological_sort_cycle_fails() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "a", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(1, "b", NodeEffect::Pure)).unwrap();
        dag.add_edge(DagEdge::new(0, 1)).unwrap();
        dag.add_edge(DagEdge::new(1, 0)).unwrap();
        assert_eq!(dag.topological_sort(), Err(DagError::CycleDetected));
    }

    #[test]
    fn dag_entry_and_exit_nodes() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "a", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(1, "b", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(2, "c", NodeEffect::Pure)).unwrap();
        dag.add_edge(DagEdge::new(0, 1)).unwrap();
        dag.add_edge(DagEdge::new(1, 2)).unwrap();
        let entries = dag.entry_nodes();
        let exits = dag.exit_nodes();
        assert!(entries.contains(&0));
        assert!(exits.contains(&2));
    }

    #[test]
    fn dag_validate_missing_budget() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "a", NodeEffect::Hot)).unwrap();
        let result = dag.validate(&[]);
        assert_eq!(result, Err(DagError::MissingBudget));
    }

    #[test]
    fn dag_validate_missing_capability() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "a", NodeEffect::Pure).with_capability("missing")).unwrap();
        let result = dag.validate(&[]);
        assert_eq!(result, Err(DagError::MissingCapability("missing".to_string())));
    }

    #[test]
    fn dag_validate_passes() {
        let mut dag = DagPipeline::new();
        dag.add_node(
            DagNode::new(0, "a", NodeEffect::Hot)
                .with_capability("math")
                .with_budget(100),
        )
        .unwrap();
        let result = dag.validate(&["math".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn control_unit_all_downstream() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "router", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(1, "a", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(2, "b", NodeEffect::Pure)).unwrap();
        dag.add_edge(DagEdge::new(0, 1)).unwrap();
        dag.add_edge(DagEdge::new(0, 2)).unwrap();
        let cu = ControlUnit::new(0, RouterStrategy::AllDownstream);
        let next = cu.select_next(&dag, &|_| false);
        assert!(next.contains(&1));
        assert!(next.contains(&2));
    }

    #[test]
    fn control_unit_first_downstream() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "router", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(1, "a", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(2, "b", NodeEffect::Pure)).unwrap();
        dag.add_edge(DagEdge::new(0, 1)).unwrap();
        dag.add_edge(DagEdge::new(0, 2)).unwrap();
        let cu = ControlUnit::new(0, RouterStrategy::FirstDownstream);
        let next = cu.select_next(&dag, &|_| false);
        assert_eq!(next.len(), 1);
    }

    #[test]
    fn control_unit_named() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "router", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(1, "alpha", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(2, "beta", NodeEffect::Pure)).unwrap();
        dag.add_edge(DagEdge::new(0, 1)).unwrap();
        dag.add_edge(DagEdge::new(0, 2)).unwrap();
        let cu = ControlUnit::new(0, RouterStrategy::Named("beta".to_string()));
        let next = cu.select_next(&dag, &|_| false);
        assert_eq!(next, vec![2]);
    }

    #[test]
    fn control_unit_conditional_if_present() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "router", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(1, "yes", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(2, "no", NodeEffect::Pure)).unwrap();
        let cu = ControlUnit::new(
            0,
            RouterStrategy::Conditional {
                channel: "check".to_string(),
                if_present: 1,
                if_empty: 2,
            },
        );
        let next = cu.select_next(&dag, &|ch| ch == "check");
        assert_eq!(next, vec![1]);
    }

    #[test]
    fn control_unit_conditional_if_empty() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "router", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(1, "yes", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(2, "no", NodeEffect::Pure)).unwrap();
        let cu = ControlUnit::new(
            0,
            RouterStrategy::Conditional {
                channel: "check".to_string(),
                if_present: 1,
                if_empty: 2,
            },
        );
        let next = cu.select_next(&dag, &|_| false);
        assert_eq!(next, vec![2]);
    }

    #[test]
    fn execution_state_ready_nodes() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "a", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(1, "b", NodeEffect::Pure)).unwrap();
        dag.add_edge(DagEdge::new(0, 1)).unwrap();
        let mut state = ExecutionState::new(&dag);
        // Initially, only node 0 is ready (no upstream).
        let ready = state.ready_nodes(&dag);
        assert!(ready.contains(&0));
        assert!(!ready.contains(&1));
        // Complete node 0.
        state.set_status(0, NodeStatus::Completed);
        let ready = state.ready_nodes(&dag);
        assert!(ready.contains(&1));
    }

    #[test]
    fn execution_state_all_completed() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "a", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(1, "b", NodeEffect::Pure)).unwrap();
        dag.add_edge(DagEdge::new(0, 1)).unwrap();
        let mut state = ExecutionState::new(&dag);
        assert!(!state.all_completed());
        state.set_status(0, NodeStatus::Completed);
        assert!(!state.all_completed());
        state.set_status(1, NodeStatus::Completed);
        assert!(state.all_completed());
    }

    #[test]
    fn execution_state_completed_order() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "a", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(1, "b", NodeEffect::Pure)).unwrap();
        let mut state = ExecutionState::new(&dag);
        state.set_status(1, NodeStatus::Completed);
        state.set_status(0, NodeStatus::Completed);
        assert_eq!(state.completed_order(), &[1, 0]);
    }

    #[test]
    fn execution_state_count_by_status() {
        let mut dag = DagPipeline::new();
        dag.add_node(DagNode::new(0, "a", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(1, "b", NodeEffect::Pure)).unwrap();
        dag.add_node(DagNode::new(2, "c", NodeEffect::Pure)).unwrap();
        let mut state = ExecutionState::new(&dag);
        state.set_status(0, NodeStatus::Completed);
        state.set_status(1, NodeStatus::Failed);
        assert_eq!(state.count_by_status(NodeStatus::Completed), 1);
        assert_eq!(state.count_by_status(NodeStatus::Failed), 1);
        assert_eq!(state.count_by_status(NodeStatus::Pending), 1);
    }

    #[test]
    fn judge_frame_submit_and_evaluate() {
        let mut frame = JudgeFrame::new();
        frame
            .submit(JudgeClaim {
                node_id: 0,
                channel: "out".to_string(),
                success: true,
                error: None,
            })
            .unwrap();
        frame
            .submit(JudgeClaim {
                node_id: 1,
                channel: "out".to_string(),
                success: false,
                error: Some("timeout".to_string()),
            })
            .unwrap();
        frame.evaluate();
        assert_eq!(frame.verdicts(), &[true, true]);
        assert!(frame.all_passed());
    }

    #[test]
    fn judge_frame_inconsistent_claim_fails() {
        let mut frame = JudgeFrame::new();
        // Claims success but has an error — inconsistent.
        frame
            .submit(JudgeClaim {
                node_id: 0,
                channel: "out".to_string(),
                success: true,
                error: Some("error".to_string()),
            })
            .unwrap();
        frame.evaluate();
        assert_eq!(frame.verdicts(), &[false]);
        assert!(!frame.all_passed());
    }

    #[test]
    fn judge_frame_reset() {
        let mut frame = JudgeFrame::new();
        frame
            .submit(JudgeClaim {
                node_id: 0,
                channel: "out".to_string(),
                success: true,
                error: None,
            })
            .unwrap();
        frame.evaluate();
        frame.reset();
        assert_eq!(frame.claim_count(), 0);
        assert!(frame.verdicts().is_empty());
    }

    #[test]
    fn judge_frame_capacity() {
        let mut frame = JudgeFrame::new();
        for i in 0..MAX_JUDGE_CLAIMS {
            frame
                .submit(JudgeClaim {
                    node_id: i as u32,
                    channel: "out".to_string(),
                    success: true,
                    error: None,
                })
                .unwrap();
        }
        let result = frame.submit(JudgeClaim {
            node_id: 99,
            channel: "out".to_string(),
            success: true,
            error: None,
        });
        assert!(result.is_err());
    }

    #[test]
    fn dag_node_as_control_unit() {
        let node = DagNode::new(0, "router", NodeEffect::Pure).as_control_unit();
        assert!(node.is_control_unit);
    }
}
