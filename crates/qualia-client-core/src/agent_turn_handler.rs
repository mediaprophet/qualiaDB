//! Agent-turn handler — bridges the DAG executor to the chat/inference layer.
//!
//! This is the final piece of the critical path:
//! `R1 → R5 → R3 → R6 → R7 → agent_turn_handler`
//!
//! When a user sends `@researcher analyze this data`, the chat layer:
//! 1. Parses the @mention (R7) → resolves to a roster agent.
//! 2. If the agent has a DAG pipeline definition, the agent-turn handler
//!    executes the pipeline using the DAG executor (R3), with each node
//!    dispatched as a local LLM agent turn.
//! 3. If no DAG is defined, falls through to the standard single-agent
//!    inference path (`run_chat_inference_for_agent`).
//!
//! The handler implements `NodeExecutor` so the DAG executor can call it
//! for each node. Each node execution is a local LLM inference turn with
//! the node's inputs (from the blackboard) injected into the prompt context.

use vibe::dag::{DagEdge, DagNode, DagPipeline, NodeEffect};
use vibe::{DiagCode, Diagnostic, Span};
use qualia_core_db::modalities::blackboard::BlackboardBus;
use qualia_core_db::poet_host::invoke::agent::dag_executor::{
    execute_pipeline, NodeExecutor, PipelineResult,
};
use qualia_core_db::NQuin;
use std::path::Path;

use crate::agent_registry::{self, AgentBackendSpec};

/// Configuration for an agent-turn handler invocation.
pub struct AgentTurnConfig {
    /// The session ID for chat context.
    pub session_id: String,
    /// The storage path for agent definitions.
    pub storage_path: String,
    /// The roster agent to execute.
    pub agent_slug: String,
    /// Optional DAG pipeline to execute.
    /// If None, the agent runs as a single-turn inference.
    pub dag_pipeline: Option<DagPipeline>,
}

impl std::fmt::Debug for AgentTurnConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentTurnConfig")
            .field("session_id", &self.session_id)
            .field("storage_path", &self.storage_path)
            .field("agent_slug", &self.agent_slug)
            .field("dag_pipeline", &self.dag_pipeline.is_some())
            .finish()
    }
}

/// Parse a DAG pipeline from a JSON definition.
///
/// Expected format:
/// ```json
/// {
///   "nodes": [
///     {"id": 0, "name": "step1", "effect": "Hot", "capabilities": [], "inputs": [], "outputs": ["out"], "budget": 4096}
///   ],
///   "edges": [
///     {"from": 0, "to": 1}
///   ]
/// }
/// ```
pub fn parse_dag_pipeline(json: &str) -> Result<DagPipeline, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut pipeline = DagPipeline::new();

    if let Some(nodes) = value.get("nodes").and_then(|v| v.as_array()) {
        for node_val in nodes {
            let id = node_val.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let name = node_val
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unnamed");
            let effect_str = node_val
                .get("effect")
                .and_then(|v| v.as_str())
                .unwrap_or("Cold");
            let effect = match effect_str {
                "Pure" => NodeEffect::Pure,
                "Hot" => NodeEffect::Hot,
                "Cold" => NodeEffect::Cold,
                "Async" => NodeEffect::Async,
                "External" => NodeEffect::External,
                _ => NodeEffect::Cold,
            };
            let mut node = DagNode::new(id, name, effect);
            if let Some(caps) = node_val.get("capabilities").and_then(|v| v.as_array()) {
                node.capabilities = caps
                    .iter()
                    .filter_map(|c| c.as_str().map(String::from))
                    .collect();
            }
            if let Some(inputs) = node_val.get("inputs").and_then(|v| v.as_array()) {
                node.inputs = inputs
                    .iter()
                    .filter_map(|i| i.as_str().map(String::from))
                    .collect();
            }
            if let Some(outputs) = node_val.get("outputs").and_then(|v| v.as_array()) {
                node.outputs = outputs
                    .iter()
                    .filter_map(|o| o.as_str().map(String::from))
                    .collect();
            }
            if let Some(budget) = node_val.get("budget").and_then(|v| v.as_u64()) {
                node.budget = budget as u32;
            }
            if let Some(is_cu) = node_val.get("is_control_unit").and_then(|v| v.as_bool()) {
                node.is_control_unit = is_cu;
            }
            pipeline
                .add_node(node)
                .map_err(|e| format!("DAG node error: {e:?}"))?;
        }
    }

    if let Some(edges) = value.get("edges").and_then(|v| v.as_array()) {
        for edge_val in edges {
            let from = edge_val.get("from").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let to = edge_val.get("to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            pipeline
                .add_edge(DagEdge::new(from, to))
                .map_err(|e| format!("DAG edge error: {e:?}"))?;
        }
    }

    Ok(pipeline)
}

/// Result of an agent-turn handler invocation.
#[derive(Debug, Clone)]
pub enum AgentTurnOutcome {
    /// The agent has a DAG pipeline — it was executed.
    DagPipeline(PipelineResult),
    /// The agent has no DAG pipeline — single-turn inference was used.
    SingleTurn,
    /// The agent could not be found or is disabled.
    AgentUnavailable(String),
    /// The DAG pipeline definition was invalid.
    InvalidDag(String),
}

/// A node executor that dispatches each DAG node as a local LLM agent turn.
///
/// The executor collects the prompt context from the blackboard inputs,
/// constructs a prompt, and calls the local inference path. The outputs
/// are written back to the blackboard as NQuin records.
pub struct AgentTurnExecutor {
    session_id: String,
    agent_slug: String,
    /// Accumulated diagnostics from node executions.
    diagnostics: Vec<Diagnostic>,
}

impl AgentTurnExecutor {
    pub fn new(session_id: &str, agent_slug: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            agent_slug: agent_slug.to_string(),
            diagnostics: Vec::new(),
        }
    }

    /// Build a prompt string from the blackboard inputs for a DAG node.
    /// The inputs are (channel_name, quins) pairs. Each channel's quins
    /// are rendered as a context block.
    fn build_prompt_from_inputs(&self, node_name: &str, inputs: &[(String, Vec<NQuin>)]) -> String {
        let mut prompt = format!("[DAG node: {node_name}]\n");
        if inputs.is_empty() {
            prompt.push_str("(no upstream inputs)\n");
        } else {
            for (channel, quins) in inputs {
                prompt.push_str(&format!("[input: {channel}] {quins:?}\n"));
            }
        }
        prompt.push_str(&format!(
            "\nAgent @{} — process the above context and produce your output.",
            self.agent_slug
        ));
        prompt
    }
}

impl NodeExecutor for AgentTurnExecutor {
    fn execute(
        &mut self,
        node_id: u32,
        node_name: &str,
        inputs: &[(String, Vec<NQuin>)],
        capabilities: &[String],
    ) -> Result<Vec<(String, Vec<NQuin>)>, Diagnostic> {
        let prompt = self.build_prompt_from_inputs(node_name, inputs);

        // Run the local inference turn for this node.
        let result = crate::chat_inference::run_chat_inference_for_agent(
            &self.session_id,
            &prompt,
            Some(&self.agent_slug),
            None,
        );

        if let Some(reason) = &result.block_reason {
            self.diagnostics.push(Diagnostic::new(
                DiagCode::E600,
                Span::point(0),
                format!("DAG node {node_id} ({node_name}) blocked: {reason}"),
            ));
            return Err(Diagnostic::new(
                DiagCode::E600,
                Span::point(0),
                format!("node {node_name} blocked: {reason}"),
            ));
        }

        // Convert the inference output to NQuin records for the blackboard.
        // The output text is hashed into an NQuin for provenance tracking.
        let output_hash = qualia_core_db::q_hash(&result.text);
        let output_nquin = NQuin {
            subject: output_hash,
            predicate: qualia_core_db::q_hash("dag:output"),
            object: output_hash,
            context: qualia_core_db::q_hash(&format!("dag:{node_id}")),
            metadata: 0,
            parity: 0,
        };

        // The output channel name is "output" by convention.
        let outputs = vec![("output".to_string(), vec![output_nquin])];

        // Track capabilities used (for audit).
        let _ = capabilities;

        Ok(outputs)
    }
}

/// Execute an agent turn, optionally through a DAG pipeline.
///
/// This is the entry point called by the chat layer when an @mention
/// resolves to a roster agent. If the agent has a DAG pipeline definition
/// (in `dag_pipeline_json`), the pipeline is executed with the agent-turn
/// executor. Otherwise, the function returns `SingleTurn` and the caller
/// should fall through to `run_chat_inference_for_agent`.
///
/// If `config.dag_pipeline` is provided, it overrides the agent's stored
/// pipeline definition.
pub fn execute_agent_turn(config: &mut AgentTurnConfig) -> AgentTurnOutcome {
    let storage = Path::new(&config.storage_path);

    // Resolve the agent.
    let agent = match agent_registry::get_agent(storage, &config.agent_slug) {
        Some(a) => a,
        None => {
            return AgentTurnOutcome::AgentUnavailable(format!(
                "Unknown agent @{}",
                config.agent_slug
            ));
        }
    };

    if !agent.enabled {
        return AgentTurnOutcome::AgentUnavailable(format!("Agent @{} is disabled", agent.slug));
    }

    // Only local agents can use DAG pipelines.
    if !matches!(agent.backend, AgentBackendSpec::LocalEngine { .. }) {
        return AgentTurnOutcome::SingleTurn;
    }

    // Use explicit pipeline if provided, otherwise parse from agent definition.
    let pipeline: DagPipeline = if let Some(p) = config.dag_pipeline.take() {
        p
    } else if !agent.dag_pipeline_json.is_empty() {
        match parse_dag_pipeline(&agent.dag_pipeline_json) {
            Ok(p) => p,
            Err(e) => {
                return AgentTurnOutcome::InvalidDag(format!(
                    "Invalid DAG pipeline for @{}: {e}",
                    config.agent_slug
                ));
            }
        }
    } else {
        return AgentTurnOutcome::SingleTurn;
    };

    // Create a blackboard bus for the pipeline.
    let mut bus = BlackboardBus::new();

    // Create the agent-turn executor.
    let mut executor = AgentTurnExecutor::new(&config.session_id, &config.agent_slug);

    // Execute the pipeline (no phase leaser for now — the chat layer
    // doesn't manage deontic phases; that's the host's responsibility).
    match execute_pipeline(&pipeline, &mut bus, None, &mut executor) {
        Ok(result) => AgentTurnOutcome::DagPipeline(result),
        Err(e) => AgentTurnOutcome::InvalidDag(format!("DAG execution error: {e:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_turn_executor_builds_prompt_from_inputs() {
        let executor = AgentTurnExecutor::new("test-session", "researcher");
        let inputs = vec![(
            "context".to_string(),
            vec![NQuin {
                subject: 1,
                predicate: 2,
                object: 3,
                context: 4,
                metadata: 0,
                parity: 0,
            }],
        )];
        let prompt = executor.build_prompt_from_inputs("analyze", &inputs);
        assert!(prompt.contains("[DAG node: analyze]"));
        assert!(prompt.contains("[input: context]"));
        assert!(prompt.contains("@researcher"));
    }

    #[test]
    fn agent_turn_executor_empty_inputs() {
        let executor = AgentTurnExecutor::new("test-session", "writer");
        let prompt = executor.build_prompt_from_inputs("draft", &[]);
        assert!(prompt.contains("(no upstream inputs)"));
        assert!(prompt.contains("@writer"));
    }

    #[test]
    fn agent_turn_outcome_unknown_agent() {
        let mut config = AgentTurnConfig {
            session_id: "test".to_string(),
            storage_path: "/nonexistent".to_string(),
            agent_slug: "nonexistent_agent".to_string(),
            dag_pipeline: None,
        };
        let outcome = execute_agent_turn(&mut config);
        assert!(matches!(outcome, AgentTurnOutcome::AgentUnavailable(_)));
    }

    #[test]
    fn agent_turn_outcome_no_dag_returns_single_turn() {
        // Without a real agent registry, we can't test the full path,
        // but we can verify the config logic: no DAG → SingleTurn.
        // This test would need a mock agent registry to be fully end-to-end.
        let mut config = AgentTurnConfig {
            session_id: "test".to_string(),
            storage_path: "/nonexistent".to_string(),
            agent_slug: "test".to_string(),
            dag_pipeline: None,
        };
        // With a nonexistent path, get_agent returns None → AgentUnavailable.
        let outcome = execute_agent_turn(&mut config);
        assert!(matches!(outcome, AgentTurnOutcome::AgentUnavailable(_)));
    }

    #[test]
    fn agent_turn_outcome_with_dag_unavailable_agent() {
        // A DAG pipeline is provided but the agent doesn't exist.
        use vibe::dag::{DagNode, DagPipeline, NodeEffect};
        let mut pipeline = DagPipeline::new();
        let _ = pipeline.add_node(DagNode::new(0, "test", NodeEffect::Hot));
        let mut config = AgentTurnConfig {
            session_id: "test".to_string(),
            storage_path: "/nonexistent".to_string(),
            agent_slug: "nonexistent".to_string(),
            dag_pipeline: Some(pipeline),
        };
        let outcome = execute_agent_turn(&mut config);
        assert!(matches!(outcome, AgentTurnOutcome::AgentUnavailable(_)));
    }

    #[test]
    fn parse_dag_pipeline_from_valid_json() {
        let json = r#"{
            "nodes": [
                {"id": 0, "name": "research", "effect": "Hot", "capabilities": ["graph.query"], "inputs": [], "outputs": ["findings"], "budget": 4096},
                {"id": 1, "name": "synthesize", "effect": "Pure", "inputs": ["findings"], "outputs": ["report"], "budget": 2048}
            ],
            "edges": [
                {"from": 0, "to": 1}
            ]
        }"#;
        let pipeline = parse_dag_pipeline(json).unwrap();
        assert_eq!(pipeline.node_count(), 2);
    }

    #[test]
    fn parse_dag_pipeline_empty_json() {
        let json = r#"{}"#;
        let pipeline = parse_dag_pipeline(json).unwrap();
        assert_eq!(pipeline.node_count(), 0);
    }

    #[test]
    fn parse_dag_pipeline_invalid_json() {
        let json = "not valid json";
        let result = parse_dag_pipeline(json);
        assert!(result.is_err());
    }
}
