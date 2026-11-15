//! Higher-Order Simplicial Complexes, Hypergraphs, Cellular Sheaves & Persistent Homology (§22).

use super::{extract_i64, extract_list, extract_record, extract_str, fnv1a_64, fnv1a_value};
use crate::error::{DiagCode, Diagnostic};
use crate::sheaf::{Simplex, Stalk, TopologicalTear};
use crate::span::Span;
use crate::value::Value;
use std::collections::{BTreeMap, BTreeSet};

pub fn call_simplex_and_sheaf(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "simplex.define" => Ok(Some(simplex_define(args, span)?)),
        "simplex.fold" => Ok(Some(simplex_fold(args, span)?)),
        "hypergraph.bind_hyperedge" => Ok(Some(hypergraph_bind_hyperedge(args, span)?)),
        "sheaf.project_stalk" => Ok(Some(sheaf_project_stalk(args, span)?)),
        "sheaf.verify_intersection" => Ok(Some(sheaf_verify_intersection(args, span)?)),
        "sheaf.apply_glue" => Ok(Some(sheaf_apply_glue(args, span)?)),
        "topology.betti_numbers" | "topology.detect_voids" => {
            Ok(Some(topology_betti_and_voids(args, span)?))
        }
        "homotopy.verify_equivalence" => Ok(Some(homotopy_verify_equivalence(args, span)?)),
        _ => Ok(None),
    }
}

fn simplex_define(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let name = extract_str(args, 0, "simplex.define", span)?;
    let dim_i64 = extract_i64(args, 1, "simplex.define", span)?.max(0);
    let dim = dim_i64.min(255) as u8;
    let vertices = extract_list(args, 2, "simplex.define", span)?;
    let member_hashes: Vec<u64> = vertices.iter().map(fnv1a_value).collect();
    let simplex = Simplex::new(name, dim, member_hashes);

    let expected_vertices = (dim as usize) + 1;
    let mut boundary_faces = Vec::new();
    let mut missing_face = None;

    if let Some(Value::List(explicit_faces)) = args.get(3) {
        boundary_faces = explicit_faces.clone();
        if dim > 0 && boundary_faces.len() < expected_vertices {
            missing_face = Some(format!(
                "expected {} boundary faces for {dim}-simplex, found {}",
                expected_vertices,
                boundary_faces.len()
            ));
        }
    } else if dim > 0 && vertices.len() == expected_vertices {
        for omit in 0..vertices.len() {
            let face: Vec<Value> = vertices
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != omit)
                .map(|(_, v)| v.clone())
                .collect();
            boundary_faces.push(Value::List(face));
        }
    }

    let intact = simplex.dimension == dim
        && vertices.len() == expected_vertices
        && missing_face.is_none();
    let tear_diagnostic = if intact {
        Value::Null
    } else {
        let reason = missing_face.unwrap_or_else(|| {
            format!("expected {expected_vertices} vertices, got {}", vertices.len())
        });
        let tear = TopologicalTear::new(
            &format!(
                "TopologicalTear({name}): expected dim {dim}, got {} vertices ({reason})",
                vertices.len()
            ),
            1.0,
            0.5,
            0,
            span,
        );
        Value::String(tear.diagnostic)
    };

    let mut out = BTreeMap::new();
    out.insert("__simplex".into(), Value::Bool(true));
    out.insert("name".into(), Value::String(name.to_string()));
    out.insert("dimension".into(), Value::I64(dim as i64));
    out.insert("vertices".into(), Value::List(vertices.to_vec()));
    out.insert("boundary_faces".into(), Value::List(boundary_faces));
    out.insert("intact".into(), Value::Bool(intact));
    out.insert("tear_diagnostic".into(), tear_diagnostic);
    Ok(Value::Record(out))
}

fn simplex_fold(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let rec = extract_record(args, 0, "simplex.fold", span)?;
    let transform = args
        .get(1)
        .and_then(|v| v.as_str())
        .unwrap_or("barycentric");
    let mut out = rec.clone();
    let vertices = rec
        .get("vertices")
        .and_then(|v| match v {
            Value::List(l) => Some(l.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let folded_hash = vertices.iter().fold(0u64, |acc, v| acc ^ fnv1a_value(v));
    out.insert("transform".into(), Value::String(transform.to_string()));
    out.insert("folded_invariant".into(), Value::U64(folded_hash));
    out.insert(
        "intact".into(),
        rec.get("intact").cloned().unwrap_or(Value::Bool(!vertices.is_empty())),
    );
    Ok(Value::Record(out))
}

fn hypergraph_bind_hyperedge(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let edge_id = extract_str(args, 0, "hypergraph.bind_hyperedge", span)?;
    let nodes = extract_list(args, 1, "hypergraph.bind_hyperedge", span)?;
    let inv_satisfied = match args.get(2) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Record(r)) => {
            let min_arity = r.get("min_arity").and_then(|v| v.as_i64()).unwrap_or(2) as usize;
            nodes.len() >= min_arity
        }
        _ => nodes.len() >= 2,
    };
    let mut out = BTreeMap::new();
    out.insert("hyperedge_id".into(), Value::String(edge_id.to_string()));
    out.insert("arity".into(), Value::I64(nodes.len() as i64));
    out.insert("nodes".into(), Value::List(nodes.to_vec()));
    out.insert("satisfied".into(), Value::Bool(inv_satisfied));
    Ok(Value::Record(out))
}

fn sheaf_project_stalk(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let global_rec = extract_record(args, 0, "sheaf.project_stalk", span)?;
    let keys_list = extract_list(args, 1, "sheaf.project_stalk", span)?;
    let agent_did = args
        .get(2)
        .and_then(|v| v.as_str())
        .unwrap_or("did:q42:agent:local");

    let mut neighborhood = Vec::new();
    let mut local_section = BTreeMap::new();
    for k_val in keys_list {
        if let Some(k) = k_val.as_str() {
            neighborhood.push(Value::String(k.to_string()));
            if let Some(v) = global_rec.get(k) {
                local_section.insert(k.to_string(), v.clone());
            }
        }
    }
    let snap_hash = fnv1a_value(&Value::Record(local_section.clone()));
    let stalk = Stalk::new(snap_hash, snap_hash ^ 0x42, "sheaf:stalk", agent_did);

    let mut out = BTreeMap::new();
    out.insert("__stalk".into(), Value::Bool(true));
    out.insert("agent_did".into(), Value::Iri(stalk.agent_did));
    out.insert("neighborhood".into(), Value::List(neighborhood));
    out.insert("local_section".into(), Value::Record(local_section));
    out.insert("snapshot_hash".into(), Value::U64(stalk.snapshot_id));
    Ok(Value::Record(out))
}

fn extract_section(stalk_rec: &BTreeMap<String, Value>) -> &BTreeMap<String, Value> {
    if let Some(Value::Record(sec)) = stalk_rec.get("local_section") {
        sec
    } else {
        stalk_rec
    }
}

fn sheaf_verify_intersection(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let rec_a = extract_record(args, 0, "sheaf.verify_intersection", span)?;
    let rec_b = extract_record(args, 1, "sheaf.verify_intersection", span)?;
    let sec_a = extract_section(rec_a);
    let sec_b = extract_section(rec_b);

    let mut overlap_keys = Vec::new();
    let mut conflicts = Vec::new();
    for (k, va) in sec_a {
        if k.starts_with("__") {
            continue;
        }
        if let Some(vb) = sec_b.get(k) {
            overlap_keys.push(Value::String(k.clone()));
            if va != vb {
                conflicts.push(Value::String(k.clone()));
            }
        }
    }
    let mut out = BTreeMap::new();
    out.insert("agrees".into(), Value::Bool(conflicts.is_empty()));
    out.insert("overlap_keys".into(), Value::List(overlap_keys));
    out.insert("conflicts".into(), Value::List(conflicts));
    Ok(Value::Record(out))
}

fn sheaf_apply_glue(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let stalks = extract_list(args, 0, "sheaf.apply_glue", span)?;
    let mut global_section: BTreeMap<String, Value> = BTreeMap::new();
    let mut conflict_set: BTreeSet<String> = BTreeSet::new();

    for item in stalks {
        let Value::Record(rec) = item else {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "sheaf.apply_glue: every stalk must be a Record",
            ));
        };
        let sec = extract_section(rec);
        for (k, v) in sec {
            if k.starts_with("__") {
                continue;
            }
            if let Some(existing) = global_section.get(k) {
                if existing != v {
                    conflict_set.insert(k.clone());
                }
            } else {
                global_section.insert(k.clone(), v.clone());
            }
        }
    }

    let tear_conflicts: Vec<Value> = conflict_set.into_iter().map(Value::String).collect();
    let glued = tear_conflicts.is_empty();
    let mut out = BTreeMap::new();
    out.insert("glued".into(), Value::Bool(glued));
    out.insert(
        "global_section".into(),
        if glued {
            Value::Record(global_section)
        } else {
            Value::Record(BTreeMap::new())
        },
    );
    out.insert("tear_conflicts".into(), Value::List(tear_conflicts));
    Ok(Value::Record(out))
}

/// Compute rank of a binary matrix over GF(2) where each column is a bit-vector of row indices.
fn gf2_rank(num_rows: usize, cols: &[Vec<usize>]) -> usize {
    if num_rows == 0 || cols.is_empty() {
        return 0;
    }
    let words = (num_rows + 63) / 64;
    let mut basis: Vec<(usize, Vec<u64>)> = Vec::new();
    for col in cols {
        let mut vec = vec![0u64; words];
        for &r in col {
            if r < num_rows {
                vec[r / 64] |= 1u64 << (r % 64);
            }
        }
        for (pivot, bvec) in &basis {
            if (vec[pivot / 64] >> (pivot % 64)) & 1 == 1 {
                for w in 0..words {
                    vec[w] ^= bvec[w];
                }
            }
        }
        let mut lead = None;
        for (w_idx, &word) in vec.iter().enumerate() {
            if word != 0 {
                lead = Some(w_idx * 64 + word.trailing_zeros() as usize);
                break;
            }
        }
        if let Some(p) = lead {
            basis.push((p, vec));
            basis.sort_by_key(|(piv, _)| *piv);
        }
    }
    basis.len()
}

fn compute_homology(rec: &BTreeMap<String, Value>) -> (i64, i64, i64, i64) {
    let mut v_map: BTreeMap<u64, usize> = BTreeMap::new();
    if let Some(Value::List(verts)) = rec.get("vertices") {
        for v in verts {
            let h = fnv1a_value(v);
            let next = v_map.len();
            v_map.entry(h).or_insert(next);
        }
    }
    let mut edges: Vec<[usize; 2]> = Vec::new();
    let mut e_map: BTreeMap<[usize; 2], usize> = BTreeMap::new();
    if let Some(Value::List(raw_edges)) = rec.get("edges") {
        for e in raw_edges {
            if let Value::List(pair) = e {
                if pair.len() >= 2 {
                    let h0 = fnv1a_value(&pair[0]);
                    let h1 = fnv1a_value(&pair[1]);
                    let n0 = v_map.len();
                    let u = *v_map.entry(h0).or_insert(n0);
                    let n1 = v_map.len();
                    let v = *v_map.entry(h1).or_insert(n1);
                    if u != v {
                        let key = if u < v { [u, v] } else { [v, u] };
                        if !e_map.contains_key(&key) {
                            let idx = edges.len();
                            edges.push(key);
                            e_map.insert(key, idx);
                        }
                    }
                }
            }
        }
    }
    let mut d2_cols: Vec<Vec<usize>> = Vec::new();
    let mut face_count = 0usize;
    if let Some(Value::List(raw_faces)) = rec.get("faces") {
        for f in raw_faces {
            if let Value::List(tri) = f {
                if tri.len() >= 3 {
                    let mut vs: Vec<usize> = tri
                        .iter()
                        .take(3)
                        .map(|val| {
                            let h = fnv1a_value(val);
                            let n = v_map.len();
                            *v_map.entry(h).or_insert(n)
                        })
                        .collect();
                    vs.sort_unstable();
                    vs.dedup();
                    if vs.len() == 3 {
                        face_count += 1;
                        let pairs = [[vs[0], vs[1]], [vs[0], vs[2]], [vs[1], vs[2]]];
                        let mut col = Vec::with_capacity(3);
                        for p in pairs {
                            let idx = *e_map.entry(p).or_insert_with(|| {
                                let i = edges.len();
                                edges.push(p);
                                i
                            });
                            col.push(idx);
                        }
                        d2_cols.push(col);
                    }
                }
            }
        }
    }
    let tet_count = match rec.get("tetrahedra") {
        Some(Value::List(t)) => t.len() as i64,
        _ => 0,
    };
    let v_count = v_map.len();
    let e_count = edges.len();
    let d1_cols: Vec<Vec<usize>> = edges.iter().map(|e| vec![e[0], e[1]]).collect();
    let rank_d1 = gf2_rank(v_count, &d1_cols) as i64;
    let rank_d2 = gf2_rank(e_count, &d2_cols) as i64;

    let beta_0 = (v_count as i64 - rank_d1).max(0);
    let beta_1 = ((e_count as i64 - rank_d1) - rank_d2).max(0);
    let beta_2 = (face_count as i64 - rank_d2 - tet_count).max(0);
    let euler = (v_count as i64) - (e_count as i64) + (face_count as i64) - tet_count;
    (beta_0, beta_1, beta_2, euler)
}

fn topology_betti_and_voids(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let rec = extract_record(args, 0, "topology.betti_numbers", span)?;
    let (beta_0, beta_1, beta_2, euler) = compute_homology(rec);
    let mut out = BTreeMap::new();
    out.insert("beta_0".into(), Value::I64(beta_0));
    out.insert("beta_1".into(), Value::I64(beta_1));
    out.insert("beta_2".into(), Value::I64(beta_2));
    out.insert("euler_characteristic".into(), Value::I64(euler));
    out.insert("has_voids".into(), Value::Bool(beta_1 > 0 || beta_2 > 0));
    out.insert("unbridged_cycles".into(), Value::I64(beta_1));
    Ok(Value::Record(out))
}

fn homotopy_verify_equivalence(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let rec_a = extract_record(args, 0, "homotopy.verify_equivalence", span)?;
    let rec_b = extract_record(args, 1, "homotopy.verify_equivalence", span)?;
    let (b0_a, b1_a, b2_a, chi_a) = compute_homology(rec_a);
    let (b0_b, b1_b, b2_b, chi_b) = compute_homology(rec_b);
    let inv_match = b0_a == b0_b && b1_a == b1_b && b2_a == b2_b && chi_a == chi_b;
    let endpoints_match = rec_a.get("endpoints") == rec_b.get("endpoints");
    let homotopic = inv_match && endpoints_match;
    let dist = ((b0_a - b0_b).abs() + (b1_a - b1_b).abs() + (b2_a - b2_b).abs()) as f64;
    let _ = fnv1a_64(b"homotopy");

    let mut out = BTreeMap::new();
    out.insert("homotopic".into(), Value::Bool(homotopic));
    out.insert("invariant_match".into(), Value::Bool(inv_match));
    out.insert("distance".into(), Value::F64(dist));
    Ok(Value::Record(out))
}
