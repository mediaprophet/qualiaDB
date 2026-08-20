//! Graph shortest path — `solvers::graph_opt::dijkstra`.
//!
//! Wraps the engine's exact Dijkstra reference. The solver owns the distance
//! math; this seam marshals `Value`, validates the edge list (non-negative
//! integer indices, non-negative finite weights), builds the adjacency list,
//! and reconstructs one shortest path by backtracking on the distance field
//! (`dist[u] + w == dist[v]`). Mirrors `wasm_bridge/engine/graph.rs::
//! graph_shortest_path` validation exactly.

use super::super::args;
use crate::solvers::graph_opt::{dijkstra, Edge};
use poet_vibe::{Diagnostic, Span, Value};

/// Build a directed adjacency list (`edges_of[i] = [(neighbour, weight), ..]`)
/// from a flat edge list `[[u, v, w], ..]`, validating indices and non-negative
/// weights. Returns `(n_nodes, edges_of, edge_list)`. `n` is `max index + 1`
/// unless `n_hint` is larger. Replicates `wasm_bridge/engine/graph.rs::
/// build_adjacency` plus the retained `Edge` list used for path reconstruction.
fn build_adjacency(
    edges: &[Vec<f64>],
    n_hint: usize,
    span: Span,
) -> Result<(usize, Vec<Vec<(usize, f64)>>, Vec<Edge>), Diagnostic> {
    let mut max_idx = 0usize;
    for e in edges {
        if e.len() != 3 {
            return Err(args::bad(
                span,
                "each edge must be [u, v, w] (three numbers)",
            ));
        }
        let u = e[0];
        let v = e[1];
        let w = e[2];
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
    }
    let n = n_hint.max(max_idx + 1);
    let mut edges_of: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    let mut edge_list: Vec<Edge> = Vec::with_capacity(edges.len());
    for e in edges {
        let u = e[0] as usize;
        let v = e[1] as usize;
        let w = e[2];
        edges_of[u].push((v, w));
        edge_list.push(Edge {
            from: u,
            to: v,
            weight: w,
        });
    }
    Ok((n, edges_of, edge_list))
}

/// Single-source single-target shortest path over a directed, non-negative
/// weighted graph (Dijkstra, the engine's exact reference). The distance comes
/// straight from `solvers::graph_opt::dijkstra`; the node sequence is
/// reconstructed by backtracking on that distance field.
///
/// Input: record `{ edges: [[u,v,w],..], source: u64, target: u64, n?: u64 }`.
/// Output: record `{ distance: f64 | null, reachable: bool, path: [u64, ..] }`
/// (path empty and `reachable=false` when `target` is unreachable; `distance`
/// is then `null`).
pub fn shortest_path(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let edges_raw = args::rec(args_v, "edges")
        .and_then(args::list)
        .ok_or_else(|| args::bad(span, "shortest_path needs edges: a list of [u, v, w]"))?;
    // Each edge is a [u, v, w] number triple.
    let mut edges: Vec<Vec<f64>> = Vec::with_capacity(edges_raw.len());
    for e in edges_raw {
        edges.push(
            args::list(e)
                .ok_or_else(|| args::bad(span, "each edge must be [u, v, w]"))?
                .iter()
                .map(|x| {
                    args::as_f64(x)
                        .ok_or_else(|| args::bad(span, "each edge must be [u, v, w] (numbers)"))
                })
                .collect::<Result<Vec<f64>, Diagnostic>>()?,
        );
    }
    let source = args::rec_u64(args_v, "source")
        .ok_or_else(|| args::bad(span, "shortest_path needs source: a non-negative integer"))?
        as usize;
    let target = args::rec_u64(args_v, "target")
        .ok_or_else(|| args::bad(span, "shortest_path needs target: a non-negative integer"))?
        as usize;
    let n_hint = args::rec_u64(args_v, "n").unwrap_or(0) as usize;

    let (n, edges_of, _edge_list) = build_adjacency(&edges, n_hint, span)?;
    if n == 0 {
        return Err(args::bad(span, "graph has no nodes"));
    }
    if source >= n {
        return Err(args::bad(span, "source index out of range"));
    }
    if target >= n {
        return Err(args::bad(span, "target index out of range"));
    }

    let dist = dijkstra(n, &edges_of, source);
    let d = dist[target];

    if !d.is_finite() {
        return Ok(args::record([
            ("distance", Value::Null),
            ("reachable", Value::Bool(false)),
            ("path", Value::List(Vec::new())),
        ]));
    }

    // Reconstruct one shortest path by walking backwards on the distance field.
    // For each node `v` on the path, find a predecessor `u` with an edge u->v
    // such that dist[u] + w == dist[v] (within a tiny tolerance).
    let mut rev: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for (u, adj) in edges_of.iter().enumerate() {
        for &(v, w) in adj {
            rev[v].push((u, w));
        }
    }
    let tol = 1e-9;
    let mut path = vec![target];
    let mut cur = target;
    let mut guard = 0usize;
    while cur != source {
        guard += 1;
        if guard > n {
            return Err(args::bad(
                span,
                "path reconstruction exceeded node count (graph inconsistency)",
            ));
        }
        let mut found = false;
        for &(u, w) in &rev[cur] {
            if dist[u].is_finite()
                && (dist[u] + w - dist[cur]).abs() <= tol * (1.0 + dist[cur].abs())
            {
                path.push(u);
                cur = u;
                found = true;
                break;
            }
        }
        if !found {
            return Err(args::bad(
                span,
                "could not reconstruct shortest path (no consistent predecessor)",
            ));
        }
    }
    path.reverse();
    let path_value = Value::List(path.into_iter().map(|i| Value::U64(i as u64)).collect());

    Ok(args::record([
        ("distance", Value::F64(d)),
        ("reachable", Value::Bool(true)),
        ("path", path_value),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn edge(u: f64, v: f64, w: f64) -> Value {
        Value::List(vec![Value::F64(u), Value::F64(v), Value::F64(w)])
    }

    fn case(edges: Vec<Value>, source: u64, target: u64) -> Value {
        let mut m = BTreeMap::new();
        m.insert("edges".into(), Value::List(edges));
        m.insert("source".into(), Value::U64(source));
        m.insert("target".into(), Value::U64(target));
        Value::Record(m)
    }

    fn case_with_n(edges: Vec<Value>, source: u64, target: u64, n: u64) -> Value {
        let mut m = BTreeMap::new();
        m.insert("edges".into(), Value::List(edges));
        m.insert("source".into(), Value::U64(source));
        m.insert("target".into(), Value::U64(target));
        m.insert("n".into(), Value::U64(n));
        Value::Record(m)
    }

    #[test]
    fn shortest_path_via_intermediate() {
        // 0->1 (1), 1->2 (2), 0->2 (4): shortest 0->2 is via 1 = 3, path [0,1,2].
        let v = shortest_path(
            &case(
                vec![
                    edge(0.0, 1.0, 1.0),
                    edge(1.0, 2.0, 2.0),
                    edge(0.0, 2.0, 4.0),
                ],
                0,
                2,
            ),
            Span { start: 0, end: 0 },
        )
        .unwrap();
        match v {
            Value::Record(r) => {
                assert!(match r.get("distance") {
                    Some(Value::F64(x)) => (x - 3.0).abs() < 1e-9,
                    other => panic!("distance: {other:?}"),
                });
                assert!(match r.get("reachable") {
                    Some(Value::Bool(b)) => *b,
                    other => panic!("reachable: {other:?}"),
                });
                match r.get("path") {
                    Some(Value::List(xs)) => {
                        assert_eq!(xs, &vec![Value::U64(0), Value::U64(1), Value::U64(2)]);
                    }
                    other => panic!("path: {other:?}"),
                }
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn unreachable_target_is_null() {
        // 0->1 only; target 2 has no incoming edge. n=3 so node 2 exists.
        let v = shortest_path(
            &case_with_n(vec![edge(0.0, 1.0, 1.0)], 0, 2, 3),
            Span { start: 0, end: 0 },
        )
        .unwrap();
        match v {
            Value::Record(r) => {
                assert!(matches!(r.get("distance"), Some(Value::Null)));
                assert!(matches!(r.get("reachable"), Some(Value::Bool(false))));
                assert!(matches!(r.get("path"), Some(Value::List(xs)) if xs.is_empty()));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn negative_weight_rejected() {
        assert!(shortest_path(
            &case(vec![edge(0.0, 1.0, -1.0)], 0, 1),
            Span { start: 0, end: 0 },
        )
        .is_err());
    }
}
