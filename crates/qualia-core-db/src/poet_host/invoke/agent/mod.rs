//! Agent orchestration invoke family.
//!
//! Contains the DAG executor (R3), agent context builder, and stalk
//! isolation. These are the wiring layers that connect VibeScript agent
//! primitives (A1–A9) to the host capability dispatch and blackboard.

pub mod dag_executor;

use crate::modalities::blackboard::BlackboardBus;
use crate::NQuin;
use dag_executor::{execute_pipeline, NodeExecutor};
use poet_vibe::dag::{DagEdge, DagNode, DagPipeline, NodeEffect};
use poet_vibe::{DiagCode, Diagnostic, Span, Value};
use std::collections::HashMap;

fn parse_node_effect(s: &str) -> NodeEffect {
    match s.to_ascii_lowercase().as_str() {
        "pure" => NodeEffect::Pure,
        "hot" => NodeEffect::Hot,
        "cold" => NodeEffect::Cold,
        "async" => NodeEffect::Async,
        "external" => NodeEffect::External,
        _ => NodeEffect::Cold,
    }
}

pub fn parse_dag_node(val: &Value, span: Span) -> Result<(DagNode, Vec<u32>), Diagnostic> {
    match val {
        Value::Record(map) => {
            let id = map
                .get("id")
                .and_then(|v| match v {
                    Value::I64(n) => Some(*n as u32),
                    Value::U64(n) => Some(*n as u32),
                    _ => None,
                })
                .unwrap_or(0);

            let name = map
                .get("name")
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| format!("node_{id}"));

            let effect = map
                .get("effect")
                .and_then(|v| match v {
                    Value::String(s) => Some(parse_node_effect(s)),
                    _ => None,
                })
                .unwrap_or(NodeEffect::Cold);

            let mut node = DagNode::new(id, &name, effect);

            if let Some(Value::List(caps)) = map.get("capabilities") {
                for c in caps {
                    if let Value::String(s) = c {
                        node.capabilities.push(s.clone());
                    }
                }
            }

            if let Some(Value::List(inputs)) = map.get("inputs") {
                for inp in inputs {
                    if let Value::String(s) = inp {
                        node.inputs.push(s.clone());
                    }
                }
            }

            if let Some(Value::List(outputs)) = map.get("outputs") {
                for out in outputs {
                    if let Value::String(s) = out {
                        node.outputs.push(s.clone());
                    }
                }
            }

            if let Some(b) = map.get("budget").and_then(|v| match v {
                Value::I64(n) => Some(*n as u32),
                Value::U64(n) => Some(*n as u32),
                _ => None,
            }) {
                node.budget = b;
            }

            let mut deps = Vec::new();
            if let Some(Value::List(d_list)) = map.get("depends_on") {
                for d in d_list {
                    if let Value::I64(n) = d {
                        deps.push(*n as u32);
                    } else if let Value::U64(n) = d {
                        deps.push(*n as u32);
                    }
                }
            }

            Ok((node, deps))
        }
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "expected Record for DAG node definition",
        )),
    }
}

pub fn build_pipeline_from_value(args: &Value, span: Span) -> Result<DagPipeline, Diagnostic> {
    let mut pipeline = DagPipeline::new();
    let (node_values, edge_values) = match args {
        Value::List(nodes) => (nodes.as_slice(), None),
        Value::Record(map) => {
            let nodes = match map.get("nodes") {
                Some(Value::List(n)) => n.as_slice(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "DAG pipeline record must contain 'nodes' List",
                    ))
                }
            };
            let edges = match map.get("edges") {
                Some(Value::List(e)) => Some(e.as_slice()),
                _ => None,
            };
            (nodes, edges)
        }
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "DAG pipeline expects a List of nodes or a Record with 'nodes'",
            ))
        }
    };

    let mut all_deps = Vec::new();
    let mut channel_producers: HashMap<String, u32> = HashMap::new();

    for nv in node_values {
        let (node, deps) = parse_dag_node(nv, span)?;
        let id = node.id;
        for out in &node.outputs {
            channel_producers.insert(out.clone(), id);
        }
        all_deps.push((id, node.inputs.clone(), deps));
        pipeline.add_node(node).map_err(|e| {
            Diagnostic::new(DiagCode::E100, span, format!("DAG node error: {e:?}"))
        })?;
    }

    if let Some(edges) = edge_values {
        for ev in edges {
            if let Value::List(pair) = ev {
                if pair.len() == 2 {
                    if let (Some(from), Some(to)) = (
                        pair[0].as_i64().map(|n| n as u32),
                        pair[1].as_i64().map(|n| n as u32),
                    ) {
                        pipeline.add_edge(DagEdge::new(from, to)).map_err(|e| {
                            Diagnostic::new(DiagCode::E100, span, format!("DAG edge error: {e:?}"))
                        })?;
                    }
                }
            }
        }
    } else {
        for (id, inputs, deps) in all_deps {
            for dep in deps {
                let _ = pipeline.add_edge(DagEdge::new(dep, id));
            }
            for inp in inputs {
                if let Some(&producer_id) = channel_producers.get(&inp) {
                    if producer_id != id {
                        let _ = pipeline.add_edge(DagEdge::new(producer_id, id));
                    }
                }
            }
        }
    }

    Ok(pipeline)
}

struct NoopNodeExecutor;

impl NodeExecutor for NoopNodeExecutor {
    fn execute(
        &mut self,
        _node_id: u32,
        _node_name: &str,
        _inputs: &[(String, Vec<NQuin>)],
        _capabilities: &[String],
    ) -> Result<Vec<(String, Vec<NQuin>)>, Diagnostic> {
        Ok(Vec::new())
    }
}

/// Execute a DAG pipeline via capability.invoke (R3).
pub fn dag_execute(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let pipeline = build_pipeline_from_value(args, span)?;
    let mut bus = BlackboardBus::new();
    let mut executor = NoopNodeExecutor;
    let result = execute_pipeline(&pipeline, &mut bus, None, &mut executor)
        .map_err(|e| Diagnostic::new(DiagCode::E600, span, format!("DAG execution error: {e:?}")))?;
    Ok(Value::String(result.summary()))
}

/// Validate a DAG pipeline via capability.invoke (R3).
pub fn dag_validate(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let pipeline = build_pipeline_from_value(args, span)?;
    pipeline
        .validate(&[])
        .map_err(|e| Diagnostic::new(DiagCode::E100, span, format!("DAG validation error: {e:?}")))?;
    Ok(Value::Bool(true))
}

/// Status of a DAG pipeline via capability.invoke (R3).
pub fn dag_status(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn make_node(id: i64, name: &str, inputs: &[&str], outputs: &[&str], budget: i64) -> Value {
        let mut map = BTreeMap::new();
        map.insert("id".into(), Value::I64(id));
        map.insert("name".into(), Value::String(name.into()));
        map.insert("effect".into(), Value::String("Cold".into()));
        map.insert(
            "inputs".into(),
            Value::List(inputs.iter().map(|s| Value::String((*s).into())).collect()),
        );
        map.insert(
            "outputs".into(),
            Value::List(outputs.iter().map(|s| Value::String((*s).into())).collect()),
        );
        map.insert("budget".into(), Value::I64(budget));
        Value::Record(map)
    }

    #[test]
    fn dag_execute_single_node() {
        let node = make_node(0, "researcher", &[], &["summary"], 100);
        let args = Value::List(vec![node]);
        let res = dag_execute(&args, Span::point(0)).unwrap();
        match res {
            Value::String(s) => {
                assert!(s.contains("success=true"));
                assert!(s.contains("nodes=1/1"));
            }
            _ => panic!("expected String result"),
        }
    }

    #[test]
    fn dag_validate_cycle() {
        let mut map = BTreeMap::new();
        let n0 = make_node(0, "node_a", &["ch_b"], &["ch_a"], 100);
        let n1 = make_node(1, "node_b", &["ch_a"], &["ch_b"], 100);
        map.insert("nodes".into(), Value::List(vec![n0, n1]));
        map.insert(
            "edges".into(),
            Value::List(vec![
                Value::List(vec![Value::I64(0), Value::I64(1)]),
                Value::List(vec![Value::I64(1), Value::I64(0)]),
            ]),
        );
        let args = Value::Record(map);
        let err = dag_validate(&args, Span::point(0));
        assert!(err.is_err());
        let diag = err.unwrap_err();
        assert!(diag.message.contains("CycleDetected") || diag.message.contains("DAG validation error"));
    }

    #[test]
    fn dag_execute_sequential() {
        let n0 = make_node(0, "step1", &[], &["step1_out"], 100);
        let n1 = make_node(1, "step2", &["step1_out"], &["step2_out"], 100);
        let args = Value::List(vec![n0, n1]);

        let res = dag_execute(&args, Span::point(0)).unwrap();
        match res {
            Value::String(s) => {
                assert!(s.contains("success=true"));
                assert!(s.contains("nodes=2/2"));
            }
            _ => panic!("expected String result"),
        }
    }
}
