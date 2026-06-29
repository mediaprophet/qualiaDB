//! Graph & relational reasoning solvers — WASM exports.
//!
//! Wraps the engine's wasm-clean solver math (`crate::solvers::graph_opt`,
//! `crate::solvers::graph_match`, `crate::solvers::learning::kg_embedding`). Same
//! code the native MCP tools and the solver unit tests exercise. Pure deterministic
//! math: no time, no threads, no RNG.
#![cfg(target_arch = "wasm32")]

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use super::jserr;

use crate::solvers::graph_match::{fuzzy_dice, fuzzy_jaccard, FuzzyTriple};
use crate::solvers::graph_opt::{dijkstra, spreading_activation, top_k, Edge};
use crate::solvers::learning::kg_embedding::ScoreModel;

/// Build a directed adjacency list (`edges_of[i] = [(neighbour, weight), ..]`) from a
/// flat edge list `[[u, v, w], ..]`, validating indices and non-negative weights.
/// Returns `(n_nodes, edges_of)`. `n` is `max index + 1` unless `n_hint` is larger.
fn build_adjacency(
    edges: &[Vec<f64>],
    n_hint: usize,
) -> Result<(usize, Vec<Vec<(usize, f64)>>), JsValue> {
    let mut max_idx = 0usize;
    for e in edges {
        if e.len() != 3 {
            return Err(JsValue::from_str(
                "each edge must be [u, v, w] (three numbers)",
            ));
        }
        let u = e[0];
        let v = e[1];
        let w = e[2];
        if !u.is_finite() || u < 0.0 || u.fract() != 0.0 {
            return Err(JsValue::from_str("edge source u must be a non-negative integer"));
        }
        if !v.is_finite() || v < 0.0 || v.fract() != 0.0 {
            return Err(JsValue::from_str("edge target v must be a non-negative integer"));
        }
        if !w.is_finite() || w < 0.0 {
            return Err(JsValue::from_str(
                "edge weight w must be finite and non-negative (Dijkstra/activation)",
            ));
        }
        max_idx = max_idx.max(u as usize).max(v as usize);
    }
    let n = n_hint.max(max_idx + 1);
    let mut edges_of: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for e in edges {
        let u = e[0] as usize;
        let v = e[1] as usize;
        let w = e[2];
        edges_of[u].push((v, w));
    }
    Ok((n, edges_of))
}

/// Single-source single-target shortest path over a directed, non-negative weighted
/// graph (Dijkstra, the engine's exact reference). The distance comes straight from
/// `solvers::graph_opt::dijkstra`; the node sequence is reconstructed by backtracking
/// on that distance field (`dist[u] + w == dist[v]`), so the math stays owned by the
/// solver.
///
/// Input `{ edges:[[u,v,w],..], source, target, n? }` ->
/// `{ distance, reachable, path:[node,..] }` (path empty and reachable=false when
/// `target` is unreachable; `distance` is then null).
#[wasm_bindgen]
pub fn graph_shortest_path(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        edges: Vec<Vec<f64>>,
        source: usize,
        target: usize,
        #[serde(default)]
        n: usize,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let (n, edges_of) = build_adjacency(&p.edges, p.n)?;
    if n == 0 {
        return Err(JsValue::from_str("graph has no nodes"));
    }
    if p.source >= n {
        return Err(JsValue::from_str("source index out of range"));
    }
    if p.target >= n {
        return Err(JsValue::from_str("target index out of range"));
    }

    let dist = dijkstra(n, &edges_of, p.source);
    let d = dist[p.target];

    #[derive(Serialize)]
    struct Out {
        distance: Option<f64>,
        reachable: bool,
        path: Vec<usize>,
    }

    if !d.is_finite() {
        return Ok(serde_wasm_bindgen::to_value(&Out {
            distance: None,
            reachable: false,
            path: Vec::new(),
        })?);
    }

    // Reconstruct one shortest path by walking backwards on the distance field.
    // For each node `v` on the path, find a predecessor `u` with an edge u->v such
    // that dist[u] + w == dist[v] (within a tiny tolerance). Build a reverse
    // adjacency once for the search.
    let mut rev: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for (u, adj) in edges_of.iter().enumerate() {
        for &(v, w) in adj {
            rev[v].push((u, w));
        }
    }
    let tol = 1e-9;
    let mut path = vec![p.target];
    let mut cur = p.target;
    let mut guard = 0usize;
    while cur != p.source {
        // Safety bound: a simple path visits each node at most once.
        guard += 1;
        if guard > n {
            return Err(JsValue::from_str(
                "path reconstruction exceeded node count (graph inconsistency)",
            ));
        }
        let mut found = false;
        for &(u, w) in &rev[cur] {
            if dist[u].is_finite() && (dist[u] + w - dist[cur]).abs() <= tol * (1.0 + dist[cur].abs())
            {
                path.push(u);
                cur = u;
                found = true;
                break;
            }
        }
        if !found {
            return Err(JsValue::from_str(
                "could not reconstruct shortest path (no consistent predecessor)",
            ));
        }
    }
    path.reverse();

    Ok(serde_wasm_bindgen::to_value(&Out {
        distance: Some(d),
        reachable: true,
        path,
    })?)
}

/// Spreading activation (Kornai, *Vector Semantics*) — propagate activation from seed
/// concepts through directed weighted edges, decaying each hop and pruning below a
/// threshold. Returns per-node total activation and a top-k relevance ranking.
///
/// Input `{ edges:[[u,v,w],..], seeds:[[node,activation],..], decay, threshold?,
/// max_hops?, top_k?, n? }` -> `{ activation:[f64;n], ranking:[node,..] }`.
#[wasm_bindgen]
pub fn graph_spreading_activation(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        edges: Vec<Vec<f64>>,
        seeds: Vec<Vec<f64>>,
        decay: f64,
        #[serde(default = "default_threshold")]
        threshold: f64,
        #[serde(default = "default_max_hops")]
        max_hops: usize,
        #[serde(default)]
        top_k: usize,
        #[serde(default)]
        n: usize,
    }
    fn default_threshold() -> f64 {
        1e-9
    }
    fn default_max_hops() -> usize {
        16
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if !p.decay.is_finite() || p.decay <= 0.0 || p.decay > 1.0 {
        return Err(JsValue::from_str("decay must be in (0, 1]"));
    }
    if !p.threshold.is_finite() || p.threshold < 0.0 {
        return Err(JsValue::from_str("threshold must be finite and non-negative"));
    }
    // Reuse the validated edge builder to get n and bounds; then flatten to Edge[].
    let (n_from_edges, _adj) = build_adjacency(&p.edges, p.n)?;

    // Parse + validate seeds, extending n to cover any seed node index.
    let mut seeds: Vec<(usize, f64)> = Vec::with_capacity(p.seeds.len());
    let mut max_seed = 0usize;
    for s in &p.seeds {
        if s.len() != 2 {
            return Err(JsValue::from_str("each seed must be [node, activation]"));
        }
        let node = s[0];
        let act = s[1];
        if !node.is_finite() || node < 0.0 || node.fract() != 0.0 {
            return Err(JsValue::from_str("seed node must be a non-negative integer"));
        }
        if !act.is_finite() {
            return Err(JsValue::from_str("seed activation must be finite"));
        }
        max_seed = max_seed.max(node as usize);
        seeds.push((node as usize, act));
    }
    if seeds.is_empty() {
        return Err(JsValue::from_str("at least one seed is required"));
    }
    let n = n_from_edges.max(max_seed + 1);
    if n == 0 {
        return Err(JsValue::from_str("graph has no nodes"));
    }

    let edge_list: Vec<Edge> = p
        .edges
        .iter()
        .map(|e| Edge {
            from: e[0] as usize,
            to: e[1] as usize,
            weight: e[2],
        })
        .collect();

    let activation = spreading_activation(n, &edge_list, &seeds, p.decay, p.threshold, p.max_hops);
    let k = if p.top_k == 0 { n } else { p.top_k.min(n) };
    let ranking = top_k(&activation, k);

    #[derive(Serialize)]
    struct Out {
        activation: Vec<f64>,
        ranking: Vec<usize>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        activation,
        ranking,
    })?)
}

/// Parse a list of `[s, p, o, degree]` fuzzy RDF triples (degree in `[0,1]`).
fn parse_fuzzy_triples(raw: &[Vec<f64>], label: &str) -> Result<Vec<FuzzyTriple>, JsValue> {
    let mut out = Vec::with_capacity(raw.len());
    for t in raw {
        if t.len() != 4 {
            return Err(JsValue::from_str(&format!(
                "each triple in {label} must be [s, p, o, degree]"
            )));
        }
        for (idx, name) in [(0usize, "s"), (1, "p"), (2, "o")] {
            let v = t[idx];
            if !v.is_finite() || v < 0.0 || v.fract() != 0.0 {
                return Err(JsValue::from_str(&format!(
                    "{label} term {name} must be a non-negative integer term id"
                )));
            }
        }
        let d = t[3];
        if !d.is_finite() || !(0.0..=1.0).contains(&d) {
            return Err(JsValue::from_str(&format!(
                "{label} degree must be in [0, 1]"
            )));
        }
        out.push(FuzzyTriple {
            s: t[0] as usize,
            p: t[1] as usize,
            o: t[2] as usize,
            degree: d,
        });
    }
    Ok(out)
}

/// Fuzzy RDF graph similarity (Ma, Li & Ma) — degree-aware Jaccard and Dice over two
/// sets of weighted triples. Terms are interned term ids (non-negative integers);
/// degrees are membership values in `[0,1]`. Two empty graphs are defined as 1.0.
///
/// Input `{ g1:[[s,p,o,degree],..], g2:[[s,p,o,degree],..] }` ->
/// `{ jaccard, dice }`.
#[wasm_bindgen]
pub fn graph_fuzzy_similarity(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        g1: Vec<Vec<f64>>,
        g2: Vec<Vec<f64>>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let g1 = parse_fuzzy_triples(&p.g1, "g1")?;
    let g2 = parse_fuzzy_triples(&p.g2, "g2")?;

    let jaccard = fuzzy_jaccard(&g1, &g2);
    let dice = fuzzy_dice(&g1, &g2);

    #[derive(Serialize)]
    struct Out {
        jaccard: f64,
        dice: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { jaccard, dice })?)
}

/// Knowledge-graph embedding plausibility score for a single triple
/// `(head, relation, tail)` under one of the four embedding families. Higher = more
/// plausible (translational models return the negative distance). Vector layout by
/// model (rank `k`):
/// * `transe` / `distmult` — head, relation, tail are length `k`.
/// * `complex` — all three length `2k` (`[re(0..k), im(k..2k)]`).
/// * `rotate` — head/tail length `2k` (`[re, im]`); relation length `k` (phase angles).
///
/// `k` is inferred from the vector lengths; mismatched lengths fail closed.
///
/// Input `{ model, head:[f64], relation:[f64], tail:[f64], p? }` -> `{ score, model,
/// rank }`. `p` (1 or 2) is the TransE norm order (default 2); ignored by other models.
#[wasm_bindgen]
pub fn graph_kge_score(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        model: String,
        head: Vec<f64>,
        relation: Vec<f64>,
        tail: Vec<f64>,
        #[serde(default = "default_p")]
        p: u8,
    }
    fn default_p() -> u8 {
        2
    }
    let inp: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;

    if inp.head.is_empty() || inp.relation.is_empty() || inp.tail.is_empty() {
        return Err(JsValue::from_str(
            "head, relation and tail vectors must be non-empty",
        ));
    }
    for (name, v) in [
        ("head", &inp.head),
        ("relation", &inp.relation),
        ("tail", &inp.tail),
    ] {
        if v.iter().any(|x| !x.is_finite()) {
            return Err(JsValue::from_str(&format!("{name} contains a non-finite value")));
        }
    }
    if inp.head.len() != inp.tail.len() {
        return Err(JsValue::from_str(
            "head and tail must have the same length",
        ));
    }

    // Resolve the model and infer rank k from the entity vector length.
    let model_name = inp.model.to_ascii_lowercase();
    let ent_len = inp.head.len();
    let (model, k) = match model_name.as_str() {
        "transe" => {
            if inp.p != 1 && inp.p != 2 {
                return Err(JsValue::from_str("transe norm order p must be 1 or 2"));
            }
            (ScoreModel::TransE { p: inp.p }, ent_len)
        }
        "distmult" => (ScoreModel::DistMult, ent_len),
        "complex" => {
            if ent_len % 2 != 0 {
                return Err(JsValue::from_str(
                    "complex entity vectors must have even length (2k: [re, im])",
                ));
            }
            (ScoreModel::ComplEx, ent_len / 2)
        }
        "rotate" => {
            if ent_len % 2 != 0 {
                return Err(JsValue::from_str(
                    "rotate entity vectors must have even length (2k: [re, im])",
                ));
            }
            (ScoreModel::RotatE, ent_len / 2)
        }
        other => {
            return Err(JsValue::from_str(&format!(
                "unknown model '{other}' (expected transe | distmult | complex | rotate)"
            )));
        }
    };

    // ScoreModel::score validates relation length against the model's dims and fails
    // closed on any mismatch.
    let score = model
        .score(&inp.head, &inp.relation, &inp.tail, k)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

    #[derive(Serialize)]
    struct Out {
        score: f64,
        model: String,
        rank: usize,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        score,
        model: model_name,
        rank: k,
    })?)
}
