//! Spreading activation — `solvers::graph_opt::spreading_activation`.
//!
//! Wraps the engine's associative-relevance propagation (Kornai, *Vector
//! Semantics*). The solver owns the wavefront decay/prune math and the top-k
//! ranking; this seam marshals `Value`, validates the edge list and seeds, and
//! packs the per-node activation + ranking into a record. Mirrors
//! `wasm_bridge/engine/graph.rs::graph_spreading_activation` validation exactly.

use super::super::args;
use crate::solvers::graph_opt::{spreading_activation as sa_solver, top_k, Edge};
use vibe::{Diagnostic, Span, Value};

/// Parse a flat edge list `[[u, v, w], ..]` (each a list of three f64) into the
/// validated `(n_from_edges, Edge[])` pair. Validates that indices are
/// non-negative integers and weights are finite and non-negative. Replicates
/// the edge half of `wasm_bridge/engine/graph.rs::build_adjacency`.
fn parse_edges(edges_raw: &[Value], span: Span) -> Result<(usize, Vec<Edge>), Diagnostic> {
    let mut max_idx = 0usize;
    let mut out: Vec<Edge> = Vec::with_capacity(edges_raw.len());
    for e in edges_raw {
        let triple = args::list(e)
            .ok_or_else(|| args::bad(span, "each edge must be [u, v, w] (three numbers)"))?;
        if triple.len() != 3 {
            return Err(args::bad(
                span,
                "each edge must be [u, v, w] (three numbers)",
            ));
        }
        let u = args::as_f64(&triple[0])
            .ok_or_else(|| args::bad(span, "edge source u must be a number"))?;
        let v = args::as_f64(&triple[1])
            .ok_or_else(|| args::bad(span, "edge target v must be a number"))?;
        let w = args::as_f64(&triple[2])
            .ok_or_else(|| args::bad(span, "edge weight w must be a number"))?;
        if !u.is_finite() || u < 0.0 || u.fract() != 0.0 {
            return Err(args::bad(
                span,
                "edge source u must be a non-negative integer",
            ));
        }
        if !v.is_finite() || v < 0.0 || v.fract() != 0.0 {
            return Err(args::bad(
                span,
                "edge target v must be a non-negative integer",
            ));
        }
        if !w.is_finite() || w < 0.0 {
            return Err(args::bad(
                span,
                "edge weight w must be finite and non-negative (Dijkstra/activation)",
            ));
        }
        max_idx = max_idx.max(u as usize).max(v as usize);
        out.push(Edge {
            from: u as usize,
            to: v as usize,
            weight: w,
        });
    }
    Ok((max_idx + 1, out))
}

/// Parse a seed list `[[node, activation], ..]` into validated `(max_seed,
/// seeds)`. Node indices must be non-negative integers; activations finite.
/// Replicates the seed half of `wasm_bridge/engine/graph.rs::
/// graph_spreading_activation`.
fn parse_seeds(seeds_raw: &[Value], span: Span) -> Result<(usize, Vec<(usize, f64)>), Diagnostic> {
    let mut max_seed = 0usize;
    let mut out: Vec<(usize, f64)> = Vec::with_capacity(seeds_raw.len());
    for s in seeds_raw {
        let pair =
            args::list(s).ok_or_else(|| args::bad(span, "each seed must be [node, activation]"))?;
        if pair.len() != 2 {
            return Err(args::bad(span, "each seed must be [node, activation]"));
        }
        let node =
            args::as_f64(&pair[0]).ok_or_else(|| args::bad(span, "seed node must be a number"))?;
        let act = args::as_f64(&pair[1])
            .ok_or_else(|| args::bad(span, "seed activation must be a number"))?;
        if !node.is_finite() || node < 0.0 || node.fract() != 0.0 {
            return Err(args::bad(span, "seed node must be a non-negative integer"));
        }
        if !act.is_finite() {
            return Err(args::bad(span, "seed activation must be finite"));
        }
        max_seed = max_seed.max(node as usize);
        out.push((node as usize, act));
    }
    Ok((max_seed, out))
}

/// Spreading activation — propagate activation from seed concepts through
/// directed weighted edges, decaying each hop and pruning below a threshold.
/// Returns per-node total activation and a top-k relevance ranking.
///
/// Input: record `{ edges: [[u,v,w],..], seeds: [[node,activation],..],
/// decay: f64, threshold?: f64 (default 1e-9), max_hops?: u64 (default 16),
/// top_k?: u64 (default 0 = all), n?: u64 (optional) }`.
/// Output: record `{ activation: [f64], ranking: [u64] }`.
pub fn spreading_activation(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let edges_raw = args::rec(args_v, "edges")
        .and_then(args::list)
        .ok_or_else(|| {
            args::bad(
                span,
                "spreading_activation needs edges: a list of [u, v, w]",
            )
        })?;
    let seeds_raw = args::rec(args_v, "seeds")
        .and_then(args::list)
        .ok_or_else(|| {
            args::bad(
                span,
                "spreading_activation needs seeds: a list of [node, activation]",
            )
        })?;
    let decay = args::rec_f64(args_v, "decay")
        .ok_or_else(|| args::bad(span, "spreading_activation needs decay: a number"))?;
    let threshold = args::rec_f64(args_v, "threshold").unwrap_or(1e-9);
    let max_hops = args::rec_u64(args_v, "max_hops").unwrap_or(16) as usize;
    let top_k_val = args::rec_u64(args_v, "top_k").unwrap_or(0) as usize;
    let n_hint = args::rec_u64(args_v, "n").unwrap_or(0) as usize;

    if !decay.is_finite() || decay <= 0.0 || decay > 1.0 {
        return Err(args::bad(span, "decay must be in (0, 1]"));
    }
    if !threshold.is_finite() || threshold < 0.0 {
        return Err(args::bad(span, "threshold must be finite and non-negative"));
    }

    let (n_from_edges, edge_list) = parse_edges(edges_raw, span)?;
    let (max_seed, seeds) = parse_seeds(seeds_raw, span)?;
    if seeds.is_empty() {
        return Err(args::bad(span, "at least one seed is required"));
    }
    let n = n_hint.max(n_from_edges).max(max_seed + 1);
    if n == 0 {
        return Err(args::bad(span, "graph has no nodes"));
    }

    let activation = sa_solver(n, &edge_list, &seeds, decay, threshold, max_hops);
    let k = if top_k_val == 0 { n } else { top_k_val.min(n) };
    let ranking = top_k(&activation, k);

    Ok(args::record([
        ("activation", args::f64_list_value(activation)),
        (
            "ranking",
            Value::List(ranking.into_iter().map(|i| Value::U64(i as u64)).collect()),
        ),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn edge(u: f64, v: f64, w: f64) -> Value {
        Value::List(vec![Value::F64(u), Value::F64(v), Value::F64(w)])
    }

    fn seed(node: f64, act: f64) -> Value {
        Value::List(vec![Value::F64(node), Value::F64(act)])
    }

    fn case(edges: Vec<Value>, seeds: Vec<Value>, decay: f64) -> Value {
        let mut m = BTreeMap::new();
        m.insert("edges".into(), Value::List(edges));
        m.insert("seeds".into(), Value::List(seeds));
        m.insert("decay".into(), Value::F64(decay));
        Value::Record(m)
    }

    #[test]
    fn activation_decays_along_a_chain() {
        // 0 -> 1 -> 2 -> 3, all weight 1. Seed node 0.
        let v = spreading_activation(
            &case(
                vec![
                    edge(0.0, 1.0, 1.0),
                    edge(1.0, 2.0, 1.0),
                    edge(2.0, 3.0, 1.0),
                ],
                vec![seed(0.0, 1.0)],
                0.5,
            ),
            Span { start: 0, end: 0 },
        )
        .unwrap();
        match v {
            Value::Record(r) => {
                let act = match r.get("activation") {
                    Some(Value::List(xs)) => xs,
                    other => panic!("activation: {other:?}"),
                };
                let a: Vec<f64> = act
                    .iter()
                    .map(|x| match x {
                        Value::F64(n) => *n,
                        _ => panic!("activation must be f64"),
                    })
                    .collect();
                assert_eq!(a.len(), 4);
                assert!(a[0] > a[1] && a[1] > a[2] && a[2] > a[3]);
                assert!(a[3] > 0.0);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn ranking_seeds_node_first() {
        // Star: 0 -> {1,2,3}; seed 0. The seed itself is most active.
        let v = spreading_activation(
            &case(
                vec![
                    edge(0.0, 1.0, 1.0),
                    edge(0.0, 2.0, 1.0),
                    edge(0.0, 3.0, 1.0),
                ],
                vec![seed(0.0, 1.0)],
                0.6,
            ),
            Span { start: 0, end: 0 },
        )
        .unwrap();
        match v {
            Value::Record(r) => match r.get("ranking") {
                Some(Value::List(xs)) => {
                    assert_eq!(xs.first(), Some(&Value::U64(0)));
                }
                other => panic!("ranking: {other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn decay_out_of_range_fails() {
        assert!(spreading_activation(
            &case(vec![edge(0.0, 1.0, 1.0)], vec![seed(0.0, 1.0)], 1.5),
            Span { start: 0, end: 0 },
        )
        .is_err());
    }

    #[test]
    fn empty_seeds_fail() {
        assert!(spreading_activation(
            &case(vec![edge(0.0, 1.0, 1.0)], vec![], 0.5),
            Span { start: 0, end: 0 },
        )
        .is_err());
    }
}
