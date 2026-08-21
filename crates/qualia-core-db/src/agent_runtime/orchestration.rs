//! Multi-agent orchestration — roster agents, blackboard wiring, governance gates.
//!
//! Connects the agent_runtime planner, DAG executor, blackboard bus, and
//! deontic governance into a single orchestration layer that can be driven
//! from VibeScript.
//!
//! The orchestration flow:
//! 1. Register agents in a roster with their capabilities and DID.
//! 2. Create an orchestration session with a task description.
//! 3. The planner produces a DAG from the task + available capabilities.
//! 4. The DAG executor runs nodes in topological order, wiring blackboard I/O.
//! 5. Governance gates check capabilities before each node executes.
//! 6. Results are collected and returned as a structured record.

use crate::modalities::blackboard::BlackboardBus;
use crate::NQuin;
use poet_vibe::dag::{DagEdge, DagNode, DagPipeline, NodeEffect};
use poet_vibe::deontic_interrupt::{Phase, PhaseLeaser};
use std::collections::BTreeMap;

/// An agent in the roster.
#[derive(Debug, Clone)]
pub struct RosterAgent {
    pub id: String,
    pub did: String,
    pub name: String,
    pub capabilities: Vec<String>,
    pub role: AgentRole,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRole {
    Researcher,
    Analyst,
    Synthesiser,
    Verifier,
    Reporter,
    Orchestrator,
    Custom,
}

impl RosterAgent {
    pub fn new(id: &str, did: &str, name: &str, role: AgentRole) -> Self {
        Self {
            id: id.to_string(),
            did: did.to_string(),
            name: name.to_string(),
            capabilities: Vec::new(),
            role,
            active: true,
        }
    }

    pub fn add_capability(&mut self, cap: &str) {
        self.capabilities.push(cap.to_string());
    }

    pub fn has_capability(&self, cap: &str) -> bool {
        self.capabilities.iter().any(|c| c == cap)
    }
}

/// The agent roster — a collection of agents available for orchestration.
#[derive(Debug, Clone, Default)]
pub struct AgentRoster {
    pub agents: BTreeMap<String, RosterAgent>,
}

impl AgentRoster {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, agent: RosterAgent) {
        self.agents.insert(agent.id.clone(), agent);
    }

    pub fn unregister(&mut self, agent_id: &str) -> bool {
        self.agents.remove(agent_id).is_some()
    }

    pub fn get(&self, agent_id: &str) -> Option<&RosterAgent> {
        self.agents.get(agent_id)
    }

    pub fn all_capabilities(&self) -> Vec<String> {
        let mut caps: Vec<String> = self
            .agents
            .values()
            .filter(|a| a.active)
            .flat_map(|a| a.capabilities.iter().cloned())
            .collect();
        caps.sort();
        caps.dedup();
        caps
    }

    pub fn active_agents(&self) -> Vec<&RosterAgent> {
        self.agents.values().filter(|a| a.active).collect()
    }

    pub fn find_agent_for_capability(&self, cap: &str) -> Option<&RosterAgent> {
        self.agents
            .values()
            .find(|a| a.active && a.has_capability(cap))
    }

    pub fn count(&self) -> usize {
        self.agents.len()
    }
}

/// An orchestration session — a task to be executed by the roster.
#[derive(Debug, Clone)]
pub struct OrchestrationSession {
    pub id: String,
    pub task: String,
    pub roster: AgentRoster,
    pub plan: Option<crate::agent_runtime::planner::AgentPlan>,
    pub status: OrchestrationStatus,
    pub results: Vec<NodeExecutionResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestrationStatus {
    Created,
    Planning,
    Executing,
    Completed,
    Failed,
    Halted,
}

impl OrchestrationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Planning => "planning",
            Self::Executing => "executing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Halted => "halted",
        }
    }
}

/// Result of executing a single node in the orchestration.
#[derive(Debug, Clone)]
pub struct NodeExecutionResult {
    pub node_id: u32,
    pub node_name: String,
    pub agent_id: Option<String>,
    pub success: bool,
    pub outputs: Vec<(String, String)>, // (channel, summary)
    pub diagnostics: Vec<String>,
}

/// Create a new orchestration session.
pub fn create_session(id: &str, task: &str) -> OrchestrationSession {
    OrchestrationSession {
        id: id.to_string(),
        task: task.to_string(),
        roster: AgentRoster::new(),
        plan: None,
        status: OrchestrationStatus::Created,
        results: Vec::new(),
    }
}

/// Plan the orchestration — use the planner to produce a DAG from the task
/// and the roster's combined capabilities.
pub fn plan_session(session: &mut OrchestrationSession) -> Result<(), String> {
    let caps = session.roster.all_capabilities();
    if caps.is_empty() {
        return Err("no capabilities available in roster".into());
    }
    let plan = crate::agent_runtime::planner::plan_task(&session.task, &caps);
    session.plan = Some(plan);
    session.status = OrchestrationStatus::Planning;
    Ok(())
}

/// Assign agents to planned steps based on capability matching.
pub fn assign_agents(session: &mut OrchestrationSession) -> Vec<(u32, String)> {
    let plan = match &session.plan {
        Some(p) => p,
        None => return Vec::new(),
    };
    let mut assignments = Vec::new();
    for step in &plan.steps {
        if let Some(agent) = session.roster.find_agent_for_capability(&step.capability) {
            assignments.push((step.id, agent.id.clone()));
        }
    }
    assignments
}

/// A NodeExecutor that routes DAG node execution to roster agents.
///
/// Each node's capability is matched to an agent in the roster. The executor
/// produces structured outputs for each node based on the plan's output
/// channels. This is a deterministic executor — it does not call LLM
/// inference, but it does run the full DAG pipeline through the real
/// `dag_executor::execute_pipeline`, including blackboard I/O, phase lease
/// gating, and deontic interrupt handling.
struct RosterNodeExecutor {
    assignments: Vec<(u32, String)>,
}

impl crate::poet_host::invoke::agent::dag_executor::NodeExecutor for RosterNodeExecutor {
    fn execute(
        &mut self,
        node_id: u32,
        node_name: &str,
        inputs: &[(String, Vec<NQuin>)],
        capabilities: &[String],
    ) -> Result<Vec<(String, Vec<NQuin>)>, poet_vibe::Diagnostic> {
        // Find the assigned agent for this node.
        let _agent_id = self
            .assignments
            .iter()
            .find(|(id, _)| *id == node_id)
            .map(|(_, aid)| aid.clone());

        // Produce outputs: for each input channel, echo a zero-quin marker.
        // In a full implementation, this would call the agent's capability
        // with the inputs and produce real outputs. For now, we produce
        // deterministic placeholder outputs so the pipeline completes.
        let mut outputs = Vec::new();
        for (channel, quins) in inputs {
            if !quins.is_empty() {
                outputs.push((channel.clone(), vec![quins[0]]));
            }
        }
        // If no inputs, produce a single marker output on the node's name.
        if outputs.is_empty() {
            outputs.push((
                format!("{node_name}_out"),
                vec![NQuin {
                    subject: node_id as u64,
                    predicate: 0,
                    object: 0,
                    context: 0,
                    metadata: 0,
                    parity: 0,
                }],
            ));
        }
        let _ = capabilities; // capabilities checked by phase leaser
        Ok(outputs)
    }
}

/// Execute the orchestration session — run the planned DAG through the real
/// `dag_executor::execute_pipeline` with blackboard I/O, phase lease gating,
/// and deontic interrupt handling.
pub fn execute_session(session: &mut OrchestrationSession) -> Result<(), String> {
    let pipeline = session_to_pipeline(session)
        .ok_or_else(|| "no plan — call plan_session first".to_string())?;
    session.status = OrchestrationStatus::Executing;
    let assignments = assign_agents(session);

    // Build blackboard and phase leaser.
    let mut bus = BlackboardBus::new();
    let mut leaser = create_phase_leaser(session);

    // Create the roster-based node executor.
    let mut executor = RosterNodeExecutor {
        assignments: assignments.clone(),
    };

    // Run the real DAG executor.
    let result = crate::poet_host::invoke::agent::dag_executor::execute_pipeline(
        &pipeline,
        &mut bus,
        Some(&mut leaser),
        &mut executor,
    );

    // Convert pipeline results to session results.
    match result {
        Ok(pipeline_result) => {
            for nr in &pipeline_result.node_results {
                let agent_id = assignments
                    .iter()
                    .find(|(id, _)| *id == nr.node_id)
                    .map(|(_, aid)| aid.clone());
                session.results.push(NodeExecutionResult {
                    node_id: nr.node_id,
                    node_name: nr.node_name.clone(),
                    agent_id,
                    success: nr.success,
                    outputs: nr
                        .outputs
                        .iter()
                        .map(|(ch, quins)| (ch.clone(), format!("{} quins", quins.len())))
                        .collect(),
                    diagnostics: nr.diagnostics.iter().map(|d| format!("{:?}", d)).collect(),
                });
            }
            if pipeline_result.success {
                session.status = OrchestrationStatus::Completed;
            } else if pipeline_result.interrupt.is_some() {
                session.status = OrchestrationStatus::Halted;
            } else {
                session.status = OrchestrationStatus::Failed;
            }
            Ok(())
        }
        Err(e) => {
            session.status = OrchestrationStatus::Failed;
            Err(format!("DAG execution failed: {e:?}"))
        }
    }
}

/// Build a DAG pipeline from an orchestration session's plan.
pub fn session_to_pipeline(session: &OrchestrationSession) -> Option<DagPipeline> {
    let plan = session.plan.as_ref()?;
    let mut pipeline = DagPipeline::new();
    // Add all nodes first.
    for step in &plan.steps {
        let effect = match step.effect {
            "pure" => NodeEffect::Pure,
            "hot" => NodeEffect::Hot,
            "cold" => NodeEffect::Cold,
            "async" => NodeEffect::Async,
            "external" => NodeEffect::External,
            _ => NodeEffect::Pure,
        };
        let mut node = DagNode::new(step.id, &step.name, effect).with_budget(step.budget);
        if !step.capability.is_empty() {
            node = node.with_capability(&step.capability);
        }
        for input in &step.inputs {
            node = node.with_input(input);
        }
        for output in &step.outputs {
            node = node.with_output(output);
        }
        let _ = pipeline.add_node(node);
    }
    // Add dependency edges.
    for step in &plan.steps {
        for &dep_id in &step.depends_on {
            let _ = pipeline.add_edge(DagEdge::new(dep_id, step.id));
        }
    }
    Some(pipeline)
}

/// Create a blackboard bus for an orchestration session.
pub fn create_blackboard(_session: &OrchestrationSession) -> BlackboardBus {
    BlackboardBus::new()
}

/// Create a phase leaser for governance gating, seeded from the roster's
/// capabilities. The leaser is registered with a single "execution" phase
/// that allows all roster capabilities.
pub fn create_phase_leaser(session: &OrchestrationSession) -> PhaseLeaser {
    let mut leaser = PhaseLeaser::new();
    let caps = session.roster.all_capabilities();
    let mut phase = Phase::new("execution");
    for cap in caps {
        phase = phase.allow(&cap);
    }
    let _ = leaser.register_phase(phase);
    let _ = leaser.enter_phase("execution");
    leaser
}

/// Summarise the orchestration session as a structured record.
pub fn session_summary(session: &OrchestrationSession) -> BTreeMap<String, String> {
    let mut summary = BTreeMap::new();
    summary.insert("id".into(), session.id.clone());
    summary.insert("task".into(), session.task.clone());
    summary.insert("status".into(), session.status.as_str().into());
    summary.insert("agent_count".into(), session.roster.count().to_string());
    summary.insert("node_count".into(), session.results.len().to_string());
    let completed = session.results.iter().filter(|r| r.success).count();
    summary.insert("nodes_completed".into(), completed.to_string());
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roster_register_and_lookup() {
        let mut roster = AgentRoster::new();
        let mut agent = RosterAgent::new("a1", "did:q42:alice", "Alice", AgentRole::Researcher);
        agent.add_capability("NLP.substrate_extract");
        roster.register(agent);
        assert_eq!(roster.count(), 1);
        assert!(roster.get("a1").is_some());
        assert!(roster
            .find_agent_for_capability("NLP.substrate_extract")
            .is_some());
    }

    #[test]
    fn roster_unregister() {
        let mut roster = AgentRoster::new();
        roster.register(RosterAgent::new(
            "a1",
            "did:q42:a",
            "A",
            AgentRole::Researcher,
        ));
        assert!(roster.unregister("a1"));
        assert_eq!(roster.count(), 0);
    }

    #[test]
    fn roster_all_capabilities() {
        let mut roster = AgentRoster::new();
        let mut a1 = RosterAgent::new("a1", "d1", "A", AgentRole::Researcher);
        a1.add_capability("cap_a");
        let mut a2 = RosterAgent::new("a2", "d2", "B", AgentRole::Analyst);
        a2.add_capability("cap_b");
        a2.add_capability("cap_a");
        roster.register(a1);
        roster.register(a2);
        let caps = roster.all_capabilities();
        assert_eq!(caps.len(), 2);
        assert!(caps.contains(&"cap_a".to_string()));
        assert!(caps.contains(&"cap_b".to_string()));
    }

    #[test]
    fn create_session_basic() {
        let session = create_session("s1", "Research climate impacts");
        assert_eq!(session.id, "s1");
        assert_eq!(session.status, OrchestrationStatus::Created);
        assert_eq!(session.roster.count(), 0);
    }

    #[test]
    fn plan_session_no_caps() {
        let mut session = create_session("s1", "Research");
        let result = plan_session(&mut session);
        assert!(result.is_err());
    }

    #[test]
    fn plan_session_with_caps() {
        let mut session = create_session("s1", "Research climate");
        let mut agent = RosterAgent::new("a1", "did:q42:a", "A", AgentRole::Researcher);
        agent.add_capability("NLP.substrate_extract");
        session.roster.register(agent);
        let result = plan_session(&mut session);
        assert!(result.is_ok());
        assert!(session.plan.is_some());
        assert_eq!(session.status, OrchestrationStatus::Planning);
    }

    #[test]
    fn execute_session_completes() {
        let mut session = create_session("s1", "Research climate");
        let mut agent = RosterAgent::new("a1", "did:q42:a", "A", AgentRole::Researcher);
        agent.add_capability("NLP.substrate_extract");
        session.roster.register(agent);
        plan_session(&mut session).unwrap();
        execute_session(&mut session).unwrap();
        assert_eq!(session.status, OrchestrationStatus::Completed);
        assert!(!session.results.is_empty());
    }

    #[test]
    fn assign_agents_matches() {
        let mut session = create_session("s1", "Research climate");
        let mut agent = RosterAgent::new("a1", "did:q42:a", "A", AgentRole::Researcher);
        agent.add_capability("NLP.substrate_extract");
        session.roster.register(agent);
        plan_session(&mut session).unwrap();
        let assignments = assign_agents(&mut session);
        assert!(!assignments.is_empty());
    }

    #[test]
    fn session_summary_basic() {
        let mut session = create_session("s1", "Research");
        let mut agent = RosterAgent::new("a1", "did:q42:a", "A", AgentRole::Researcher);
        agent.add_capability("NLP.substrate_extract");
        session.roster.register(agent);
        plan_session(&mut session).unwrap();
        execute_session(&mut session).unwrap();
        let summary = session_summary(&session);
        assert_eq!(summary.get("status"), Some(&"completed".to_string()));
        assert_eq!(summary.get("agent_count"), Some(&"1".to_string()));
    }

    #[test]
    fn create_blackboard_basic() {
        let session = create_session("s1", "Research");
        let _bb = create_blackboard(&session);
    }

    #[test]
    fn create_phase_leaser_basic() {
        let mut session = create_session("s1", "Research");
        let mut agent = RosterAgent::new("a1", "did:q42:a", "A", AgentRole::Researcher);
        agent.add_capability("cap_x");
        session.roster.register(agent);
        let leaser = create_phase_leaser(&session);
        assert!(leaser.is_leased("cap_x"));
    }
}
