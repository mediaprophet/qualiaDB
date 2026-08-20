//! R3 — DAG Executor.
//!
//! Executes a [`poet_vibe::dag::DagPipeline`] node-by-node in topological
//! order, wiring blackboard I/O, phase leases, and deontic gates between
//! nodes.
//!
//! ## Design
//!
//! The executor is a **static topological order** runner (D3 default —
//! ControlUnit autonomous routing is P2.5, not yet implemented). For each
//! node in topological order:
//!
//! 1. Read inputs from the blackboard (R5 `BlackboardBus`).
//! 2. Propagate pinned constraints from upstream channels.
//! 3. Check the node's capabilities against the phase lease (R2).
//! 4. Execute the node's agent turn (caller-provided callback — the executor
//!    itself does not call inference; it orchestrates the pipeline).
//! 5. Write outputs to the blackboard.
//! 6. Freeze output channels if the node is completed.
//! 7. If a deontic breach occurs, halt the pipeline and return the interrupt.
//!
//! The executor is generic over the node execution callback so it can be
//! used with any inference backend (local engine, remote MCP, etc.).

use crate::modalities::blackboard::BlackboardBus;
use crate::NQuin;
use poet_vibe::dag::{DagError, DagPipeline};
use poet_vibe::deontic_interrupt::{DeonticInterrupt, PhaseLeaser};
use poet_vibe::{Diagnostic, Span};

/// Result of executing a single DAG node.
#[derive(Debug, Clone)]
pub struct NodeResult {
    /// The node ID that was executed.
    pub node_id: u32,
    /// The node name.
    pub node_name: String,
    /// Whether the node completed successfully.
    pub success: bool,
    /// Outputs written to the blackboard (channel_name, quins).
    pub outputs: Vec<(String, Vec<NQuin>)>,
    /// Diagnostics produced during execution (if any).
    pub diagnostics: Vec<Diagnostic>,
    /// Number of blackboard constraint violations detected after this node.
    pub constraint_violations: usize,
}

/// Result of executing the entire DAG pipeline.
#[derive(Debug, Clone)]
pub struct PipelineResult {
    /// Whether all nodes completed successfully.
    pub success: bool,
    /// Per-node results in execution order.
    pub node_results: Vec<NodeResult>,
    /// The deontic interrupt that halted the pipeline (if any).
    pub interrupt: Option<DeonticInterrupt>,
    /// Total number of nodes executed.
    pub nodes_executed: usize,
    /// Total number of nodes in the pipeline.
    pub nodes_total: usize,
}

impl PipelineResult {
    pub fn summary(&self) -> String {
        format!(
            "dag_pipeline: success={} nodes={}/{} interrupt={}",
            self.success,
            self.nodes_executed,
            self.nodes_total,
            self.interrupt.is_some()
        )
    }
}

/// A node execution callback. The executor calls this for each node,
/// providing the node's inputs from the blackboard. The callback returns
/// the node's outputs to write to the blackboard.
///
/// The callback receives:
/// - `node_id`: the DAG node ID
/// - `node_name`: the human-readable node name
/// - `inputs`: the blackboard inputs (channel_name, quins)
/// - `capabilities`: the node's required capabilities
///
/// The callback returns:
/// - `Ok(outputs)`: the outputs to write to the blackboard
/// - `Err(diagnostic)`: a diagnostic that halts the node
pub trait NodeExecutor {
    fn execute(
        &mut self,
        node_id: u32,
        node_name: &str,
        inputs: &[(String, Vec<NQuin>)],
        capabilities: &[String],
    ) -> Result<Vec<(String, Vec<NQuin>)>, Diagnostic>;
}

/// Execute a DAG pipeline in topological order.
///
/// The executor:
/// 1. Validates the DAG (cycle detection, capability check).
/// 2. Gets the topological order.
/// 3. For each node: reads inputs, propagates constraints, checks phase
///    lease, executes the node callback, writes outputs, checks constraints.
/// 4. If any node fails or a deontic breach occurs, the pipeline halts.
///
/// The `phase_leaser` is optional — if `None`, capability checks are skipped
/// (backward-compatible with existing callers that don't use deontic gating).
pub fn execute_pipeline<E: NodeExecutor>(
    pipeline: &DagPipeline,
    bus: &mut BlackboardBus,
    leaser: Option<&mut PhaseLeaser>,
    executor: &mut E,
) -> Result<PipelineResult, DagError> {
    // Validate: check for cycles.
    if pipeline.has_cycle() {
        return Err(DagError::CycleDetected);
    }

    // Get topological order.
    let order = pipeline.topological_sort()?;
    let nodes_total = order.len();

    let mut node_results = Vec::new();
    let mut interrupt: Option<DeonticInterrupt> = None;
    let mut nodes_executed = 0;

    // Take the leaser out of the Option so we can use it conditionally.
    let mut leaser = leaser;

    for &node_id in &order {
        let node = match pipeline.get_node(node_id) {
            Some(n) => n,
            None => continue,
        };

        // Read inputs from the blackboard.
        let inputs = bus.read_inputs(&node.inputs);

        // Propagate pinned constraints from upstream nodes.
        let upstream_ids = pipeline.upstream(node_id);
        for &upstream_id in upstream_ids {
            if let Some(upstream_node) = pipeline.get_node(upstream_id) {
                let _ = bus.propagate_constraints(&upstream_node.outputs, &node.inputs);
            }
        }

        // Check phase lease (R2) — if a leaser is attached.
        if let Some(ref mut leaser) = leaser {
            if leaser.is_interrupted() {
                interrupt = Some(DeonticInterrupt::prohibition_breach(
                    "pipeline",
                    "execute",
                    Some(node_id),
                ));
                break;
            }
            for cap in &node.capabilities {
                if !leaser.is_leased(cap) {
                    // Phase violation — halt the pipeline.
                    interrupt = Some(DeonticInterrupt::phase_violation(
                        cap,
                        "execute",
                        Some(node_id),
                    ));
                    break;
                }
            }
            if interrupt.is_some() {
                break;
            }
        }

        // Execute the node.
        let exec_result = executor.execute(node_id, &node.name, &inputs, &node.capabilities);

        let (success, outputs, diagnostics) = match exec_result {
            Ok(outs) => (true, outs, Vec::new()),
            Err(diag) => (false, Vec::new(), vec![diag]),
        };

        // Write outputs to the blackboard.
        if success && !outputs.is_empty() {
            let _ = bus.write_outputs(&outputs);
        }

        // Check constraints after writing.
        let constraint_violations = if success {
            bus.check_constraints().len()
        } else {
            0
        };

        // Freeze output channels for completed nodes.
        if success {
            for (channel, _) in &outputs {
                let _ = bus.freeze_output(channel);
            }
        }

        node_results.push(NodeResult {
            node_id,
            node_name: node.name.clone(),
            success,
            outputs,
            diagnostics,
            constraint_violations,
        });

        nodes_executed += 1;

        if !success {
            break;
        }
    }

    let success = interrupt.is_none() && node_results.iter().all(|r| r.success);

    Ok(PipelineResult {
        success,
        node_results,
        interrupt,
        nodes_executed,
        nodes_total,
    })
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modalities::blackboard::BlackboardBus;
    use poet_vibe::dag::{DagEdge, DagNode, DagPipeline, NodeEffect};
    use poet_vibe::deontic_interrupt::{Phase, PhaseLeaser};

    fn make_quin(s: u64, p: u64, o: u64) -> NQuin {
        let q = NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        NQuin {
            parity: q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata,
            ..q
        }
    }

    /// A simple node executor that writes a fixed quin to the output channel.
    struct FixedExecutor {
        quin: NQuin,
    }

    impl NodeExecutor for FixedExecutor {
        fn execute(
            &mut self,
            _node_id: u32,
            _node_name: &str,
            _inputs: &[(String, Vec<NQuin>)],
            _capabilities: &[String],
        ) -> Result<Vec<(String, Vec<NQuin>)>, Diagnostic> {
            Ok(vec![("output".into(), vec![self.quin])])
        }
    }

    /// An executor that always fails.
    struct FailingExecutor;

    impl NodeExecutor for FailingExecutor {
        fn execute(
            &mut self,
            _node_id: u32,
            _node_name: &str,
            _inputs: &[(String, Vec<NQuin>)],
            _capabilities: &[String],
        ) -> Result<Vec<(String, Vec<NQuin>)>, Diagnostic> {
            Err(Diagnostic::new(
                poet_vibe::DiagCode::E600,
                Span::point(0),
                "intentional failure",
            ))
        }
    }

    #[test]
    fn r3_single_node_pipeline() {
        let mut pipeline = DagPipeline::new();
        pipeline
            .add_node(
                DagNode::new(0, "agent_a", NodeEffect::Cold)
                    .with_input("input")
                    .with_output("output"),
            )
            .unwrap();

        let mut bus = BlackboardBus::new();
        bus.write_output("input", make_quin(1, 2, 3)).unwrap();

        let mut executor = FixedExecutor {
            quin: make_quin(4, 5, 6),
        };

        let result = execute_pipeline(&pipeline, &mut bus, None, &mut executor).unwrap();
        assert!(result.success);
        assert_eq!(result.nodes_executed, 1);
        assert_eq!(result.nodes_total, 1);
        assert!(result.interrupt.is_none());
    }

    #[test]
    fn r3_sequential_pipeline() {
        let mut pipeline = DagPipeline::new();
        pipeline
            .add_node(DagNode::new(0, "researcher", NodeEffect::Cold).with_output("draft"))
            .unwrap();
        pipeline
            .add_node(
                DagNode::new(1, "reviewer", NodeEffect::Cold)
                    .with_input("draft")
                    .with_output("review"),
            )
            .unwrap();
        pipeline.add_edge(DagEdge::new(0, 1)).unwrap();

        let mut bus = BlackboardBus::new();
        let mut executor = FixedExecutor {
            quin: make_quin(7, 8, 9),
        };

        let result = execute_pipeline(&pipeline, &mut bus, None, &mut executor).unwrap();
        assert!(result.success);
        assert_eq!(result.nodes_executed, 2);
        assert_eq!(result.nodes_total, 2);
    }

    #[test]
    fn r3_failing_node_halts_pipeline() {
        let mut pipeline = DagPipeline::new();
        pipeline
            .add_node(DagNode::new(0, "bad_agent", NodeEffect::Cold))
            .unwrap();
        pipeline
            .add_node(DagNode::new(1, "good_agent", NodeEffect::Cold))
            .unwrap();
        pipeline.add_edge(DagEdge::new(0, 1)).unwrap();

        let mut bus = BlackboardBus::new();
        let mut executor = FailingExecutor;

        let result = execute_pipeline(&pipeline, &mut bus, None, &mut executor).unwrap();
        assert!(!result.success);
        assert_eq!(result.nodes_executed, 1); // Only the first node ran.
        assert!(!result.node_results[0].success);
    }

    #[test]
    fn r3_phase_leaser_blocks_unleased_capability() {
        let mut pipeline = DagPipeline::new();
        pipeline
            .add_node(
                DagNode::new(0, "agent", NodeEffect::Cold)
                    .with_capability("graph.write")
                    .with_output("output"),
            )
            .unwrap();

        let mut leaser = PhaseLeaser::new();
        // Only allow "math" — "graph" is not leased.
        leaser
            .register_phase(Phase::new("execute").allow("math"))
            .unwrap();
        leaser.enter_phase("execute").unwrap();

        let mut bus = BlackboardBus::new();
        let mut executor = FixedExecutor {
            quin: make_quin(1, 2, 3),
        };

        let result =
            execute_pipeline(&pipeline, &mut bus, Some(&mut leaser), &mut executor).unwrap();
        assert!(!result.success);
        assert!(result.interrupt.is_some());
        assert_eq!(result.nodes_executed, 0); // Halted before execution.
    }

    #[test]
    fn r3_phase_leaser_allows_leased_capability() {
        let mut pipeline = DagPipeline::new();
        pipeline
            .add_node(
                DagNode::new(0, "agent", NodeEffect::Cold)
                    .with_capability("math")
                    .with_output("output"),
            )
            .unwrap();

        let mut leaser = PhaseLeaser::new();
        leaser
            .register_phase(Phase::new("execute").allow("math"))
            .unwrap();
        leaser.enter_phase("execute").unwrap();

        let mut bus = BlackboardBus::new();
        let mut executor = FixedExecutor {
            quin: make_quin(1, 2, 3),
        };

        let result =
            execute_pipeline(&pipeline, &mut bus, Some(&mut leaser), &mut executor).unwrap();
        assert!(result.success);
        assert_eq!(result.nodes_executed, 1);
    }

    #[test]
    fn r3_cycle_detected() {
        let mut pipeline = DagPipeline::new();
        pipeline
            .add_node(DagNode::new(0, "a", NodeEffect::Cold))
            .unwrap();
        pipeline
            .add_node(DagNode::new(1, "b", NodeEffect::Cold))
            .unwrap();
        pipeline.add_edge(DagEdge::new(0, 1)).unwrap();
        pipeline.add_edge(DagEdge::new(1, 0)).unwrap();

        let mut bus = BlackboardBus::new();
        let mut executor = FixedExecutor {
            quin: make_quin(1, 2, 3),
        };

        let result = execute_pipeline(&pipeline, &mut bus, None, &mut executor);
        assert!(matches!(result, Err(DagError::CycleDetected)));
    }

    #[test]
    fn r3_summary_string() {
        let mut pipeline = DagPipeline::new();
        pipeline
            .add_node(DagNode::new(0, "agent", NodeEffect::Cold))
            .unwrap();

        let mut bus = BlackboardBus::new();
        let mut executor = FixedExecutor {
            quin: make_quin(1, 2, 3),
        };

        let result = execute_pipeline(&pipeline, &mut bus, None, &mut executor).unwrap();
        let summary = result.summary();
        assert!(summary.contains("success=true"));
        assert!(summary.contains("nodes=1/1"));
    }
}
