//! Wave 28 — post-PlantGL geometry host seams (P21, flora/fauna/biosphere).
//!
//! Exposes the modern-biome `computational_geometry` modules (Poisson-disk
//! sampling, heat geodesics, sparse voxel octree ray casting, power diagrams,
//! differential growth, curve skeletonization, dual-quaternion skinning,
//! position-based dynamics, medial-axis extraction, smooth SDF blending)
//! through `ComputationalGeometry.*` invoke IDs. Host adapters are cold-path:
//! they allocate parsing/result buffers at the Vibe boundary while the
//! underlying kernels stay caller-buffered.

use super::super::args;
use crate::specialized_libs::computational_geometry as cg;
use vibe::{Diagnostic, Span, Value};

fn pts3(v: &Value, span: Span, what: &str) -> Result<Vec<cg::Point3>, Diagnostic> {
    let xs =
        args::f64s(v).ok_or_else(|| args::bad(span, format!("{what} must be a flat f64 list")))?;
    if xs.len() % 3 != 0 {
        return Err(args::bad(
            span,
            format!("{what} length must be a multiple of 3"),
        ));
    }
    Ok(xs
        .chunks_exact(3)
        .map(|c| cg::Point3::new(c[0], c[1], c[2]))
        .collect())
}

fn tri3(v: &Value, span: Span, what: &str) -> Result<Vec<[u32; 3]>, Diagnostic> {
    let xs = args::f64s(v)
        .ok_or_else(|| args::bad(span, format!("{what} must be a flat index list")))?;
    if xs.len() % 3 != 0 {
        return Err(args::bad(
            span,
            format!("{what} length must be a multiple of 3"),
        ));
    }
    let mut out = Vec::with_capacity(xs.len() / 3);
    for c in xs.chunks_exact(3) {
        if c.iter()
            .any(|&x| !x.is_finite() || x < 0.0 || x > u32::MAX as f64 || x.fract() != 0.0)
        {
            return Err(args::bad(
                span,
                format!("{what} contains a negative or non-integral index"),
            ));
        }
        out.push([c[0] as u32, c[1] as u32, c[2] as u32]);
    }
    Ok(out)
}

fn checked_index(x: f64, count: usize, span: Span, what: &str) -> Result<u32, Diagnostic> {
    if !x.is_finite() || x < 0.0 || x > u32::MAX as f64 || x.fract() != 0.0 || x as usize >= count {
        return Err(args::bad(
            span,
            format!("{what} contains an invalid vertex index"),
        ));
    }
    Ok(x as u32)
}

// The Host owns three scratch arrays and a materialized result. Account for
// their combined worst-case size before allocation. High-detail callers may
// request a larger budget than the 256-MiB cold-geometry default.
const MAX_POISSON_HOST_BUDGET_BYTES: u64 = 16 * 1024 * 1024 * 1024;

fn try_zeroed_vec<T: Default>(len: usize, span: Span) -> Result<Vec<T>, Diagnostic> {
    let mut out = Vec::new();
    out.try_reserve_exact(len)
        .map_err(|_| args::bad(span, "Poisson Host allocation exceeds available memory"))?;
    out.resize_with(len, T::default);
    Ok(out)
}

fn checked_grid_capacity(
    extents: &[f64],
    cell: f64,
    max_cells: usize,
    span: Span,
) -> Result<usize, Diagnostic> {
    if !cell.is_finite() || cell <= 0.0 {
        return Err(args::bad(span, "radius must be finite and positive"));
    }
    let mut capacity = 1usize;
    for &extent in extents {
        if !extent.is_finite() || extent <= 0.0 {
            return Err(args::bad(span, "bounds must be finite and ordered"));
        }
        let cells = (extent / cell).ceil();
        if !cells.is_finite() || cells < 1.0 || cells > max_cells as f64 {
            return Err(args::bad(span, "Poisson grid exceeds Host budget"));
        }
        capacity = capacity
            .checked_mul(cells as usize)
            .filter(|&n| n <= max_cells)
            .ok_or_else(|| args::bad(span, "Poisson grid exceeds Host budget"))?;
    }
    Ok(capacity)
}

fn poisson_max_cells(
    input: &Value,
    bytes_per_cell: usize,
    span: Span,
) -> Result<usize, Diagnostic> {
    let budget = args::rec_u64(input, "grid_budget_bytes")
        .unwrap_or(cg::geometry_workspace::DEFAULT_COLD_GEOMETRY_BUDGET as u64);
    let budget = usize::try_from(budget)
        .map_err(|_| args::bad(span, "Poisson budget exceeds address space"))?;
    if budget < bytes_per_cell || (budget as u64) > MAX_POISSON_HOST_BUDGET_BYTES {
        return Err(args::bad(
            span,
            "Poisson budget must be between one cell and 16 GiB",
        ));
    }
    Ok(budget / bytes_per_cell)
}

fn pt3_value(p: cg::Point3) -> Value {
    Value::List(vec![Value::F64(p.x), Value::F64(p.y), Value::F64(p.z)])
}

fn pts3_value(ps: &[cg::Point3]) -> Value {
    Value::List(
        ps.iter()
            .flat_map(|p| [Value::F64(p.x), Value::F64(p.y), Value::F64(p.z)])
            .collect(),
    )
}

fn pts2_value(ps: &[cg::Point2]) -> Value {
    Value::List(
        ps.iter()
            .flat_map(|p| [Value::F64(p.x), Value::F64(p.y)])
            .collect(),
    )
}

fn tris_value(ts: &[[u32; 3]], n: usize) -> Value {
    Value::List(
        ts[..n]
            .iter()
            .flat_map(|t| {
                [
                    Value::U64(t[0] as u64),
                    Value::U64(t[1] as u64),
                    Value::U64(t[2] as u64),
                ]
            })
            .collect(),
    )
}

fn rec_pt3(args_v: &Value, key: &str, span: Span) -> Result<cg::Point3, Diagnostic> {
    let xs = args::rec_f64_list(args_v, key)
        .ok_or_else(|| args::bad(span, format!("missing or invalid '{key}' list")))?;
    if xs.len() != 3 {
        return Err(args::bad(
            span,
            format!("'{key}' must have exactly 3 components"),
        ));
    }
    Ok(cg::Point3::new(xs[0], xs[1], xs[2]))
}

fn rec_pt2(args_v: &Value, key: &str, span: Span) -> Result<cg::Point2, Diagnostic> {
    let xs = args::rec_f64_list(args_v, key)
        .ok_or_else(|| args::bad(span, format!("missing or invalid '{key}' list")))?;
    if xs.len() != 2 {
        return Err(args::bad(
            span,
            format!("'{key}' must have exactly 2 components"),
        ));
    }
    Ok(cg::Point2::new(xs[0], xs[1]))
}

fn map_err(span: Span, e: impl core::fmt::Debug) -> Diagnostic {
    args::bad(span, format!("{e:?}"))
}

/// `ComputationalGeometry.poisson_disk_2d` — Bridson blue-noise sampling.
///
/// `{min:[x,y], max:[x,y], radius, seed, k?, grid_budget_bytes?}` → `{points:[x,y,…], count}`.
/// Scratch/output capacity is the Bridson bound `gx·gy` (≤1 sample/cell).
pub fn poisson_disk_2d_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (min, max) = (rec_pt2(args_v, "min", span)?, rec_pt2(args_v, "max", span)?);
    let radius =
        args::rec_f64(args_v, "radius").ok_or_else(|| args::bad(span, "missing 'radius'"))?;
    let seed = args::rec_u64(args_v, "seed").unwrap_or(0x9E37_79B9_7F4A_7C15);
    let k = args::rec_u64(args_v, "k").unwrap_or(30) as usize;
    let cell = radius / std::f64::consts::SQRT_2;
    let max_cells = poisson_max_cells(
        args_v,
        2 * core::mem::size_of::<u32>()
            + core::mem::size_of::<cg::Point2>()
            + 2 * core::mem::size_of::<Value>(),
        span,
    )?;
    let cap = checked_grid_capacity(&[max.x - min.x, max.y - min.y], cell, max_cells, span)?;
    let mut grid = try_zeroed_vec::<u32>(cap, span)?;
    let mut active = try_zeroed_vec::<u32>(cap, span)?;
    let mut out = try_zeroed_vec::<cg::Point2>(cap, span)?;
    let n = cg::poisson_disk_2d(min, max, radius, seed, k, &mut grid, &mut active, &mut out)
        .map_err(|e| map_err(span, e))?;
    out.truncate(n);
    Ok(args::record([
        ("points", pts2_value(&out)),
        ("count", Value::U64(n as u64)),
    ]))
}

/// `ComputationalGeometry.poisson_disk_3d` — Bridson blue-noise sampling in 3-D.
///
/// `{min:[x,y,z], max:[x,y,z], radius, seed, k?, grid_budget_bytes?}` → `{points:[x,y,z,…], count}`.
pub fn poisson_disk_3d_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (min, max) = (rec_pt3(args_v, "min", span)?, rec_pt3(args_v, "max", span)?);
    let radius =
        args::rec_f64(args_v, "radius").ok_or_else(|| args::bad(span, "missing 'radius'"))?;
    let seed = args::rec_u64(args_v, "seed").unwrap_or(0x9E37_79B9_7F4A_7C15);
    let k = args::rec_u64(args_v, "k").unwrap_or(30) as usize;
    let cell = radius / 3.0f64.sqrt();
    let max_cells = poisson_max_cells(
        args_v,
        2 * core::mem::size_of::<u32>()
            + core::mem::size_of::<cg::Point3>()
            + 3 * core::mem::size_of::<Value>(),
        span,
    )?;
    let cap = checked_grid_capacity(
        &[max.x - min.x, max.y - min.y, max.z - min.z],
        cell,
        max_cells,
        span,
    )?;
    let mut grid = try_zeroed_vec::<u32>(cap, span)?;
    let mut active = try_zeroed_vec::<u32>(cap, span)?;
    let mut out = try_zeroed_vec::<cg::Point3>(cap, span)?;
    let n = cg::poisson_disk_3d(min, max, radius, seed, k, &mut grid, &mut active, &mut out)
        .map_err(|e| map_err(span, e))?;
    out.truncate(n);
    Ok(args::record([
        ("points", pts3_value(&out)),
        ("count", Value::U64(n as u64)),
    ]))
}

/// `ComputationalGeometry.heat_geodesics` — discrete heat-method distances.
///
/// `{vertices:[…], triangles:[…], source, t_factor?, iterations?}` →
/// `{distances:[f64,…]}`.
pub fn heat_geodesics_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let vertices = pts3(
        args::rec(args_v, "vertices").ok_or_else(|| args::bad(span, "missing 'vertices'"))?,
        span,
        "vertices",
    )?;
    let triangles = tri3(
        args::rec(args_v, "triangles").ok_or_else(|| args::bad(span, "missing 'triangles'"))?,
        span,
        "triangles",
    )?;
    let source = args::rec_u64(args_v, "source")
        .ok_or_else(|| args::bad(span, "missing 'source'"))? as usize;
    let t_factor = args::rec_f64(args_v, "t_factor").unwrap_or(1.0);
    let iters = args::rec_u64(args_v, "iterations").unwrap_or(64) as u32;
    let mut out = vec![f64::NAN; vertices.len()];
    cg::heat_geodesic_distances(&vertices, &triangles, source, t_factor, iters, &mut out)
        .map_err(|e| map_err(span, e))?;
    Ok(args::record([("distances", args::f64_list_value(out))]))
}

/// `ComputationalGeometry.svo_raycast` — build a sparse voxel octree over a
/// point cloud and cast a ray.
///
/// `{vertices:[…], depth?, pad?, node_budget_bytes?, origin:[x,y,z], direction:[x,y,z], t_max?}` →
/// `{hit, t, leaf:[ix,iy,iz]}`.
pub fn svo_raycast_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let vertices = pts3(
        args::rec(args_v, "vertices").ok_or_else(|| args::bad(span, "missing 'vertices'"))?,
        span,
        "vertices",
    )?;
    let depth = args::rec_u64(args_v, "depth").unwrap_or(6) as u8;
    let pad = args::rec_f64(args_v, "pad").unwrap_or(0.01);
    let origin = rec_pt3(args_v, "origin", span)?;
    let dir = rec_pt3(args_v, "direction", span)?;
    let t_max = args::rec_f64(args_v, "t_max").unwrap_or(f64::INFINITY);
    let node_budget_bytes = args::rec_u64(args_v, "node_budget_bytes")
        .unwrap_or(cg::geometry_workspace::DEFAULT_COLD_GEOMETRY_BUDGET as u64);
    let node_budget_bytes = usize::try_from(node_budget_bytes)
        .map_err(|_| args::bad(span, "node budget exceeds address space"))?;
    let svo = cg::Svo::from_vertices_with_budget(&vertices, depth, pad, node_budget_bytes)
        .map_err(|e| map_err(span, e))?;
    match svo.raycast(origin, dir, t_max) {
        Some(hit) => Ok(args::record([
            ("hit", Value::Bool(true)),
            ("t", Value::F64(hit.t)),
            (
                "leaf",
                Value::List(hit.leaf.iter().map(|&i| Value::U64(i as u64)).collect()),
            ),
            ("occupied", Value::U64(svo.occupied_leaf_count() as u64)),
        ])),
        None => Ok(args::record([
            ("hit", Value::Bool(false)),
            ("occupied", Value::U64(svo.occupied_leaf_count() as u64)),
        ])),
    }
}

/// `ComputationalGeometry.power_diagram_2d` — regular triangulation and
/// Laguerre–Voronoi dual vertices for weighted sites.
///
/// `{sites:[[x,y,weight],…]}` → `{triangles:[i,j,k,…], dual:[x,y,…], count}`.
pub fn power_diagram_2d_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let raw = args::rec_f64_list(args_v, "sites")
        .ok_or_else(|| args::bad(span, "missing 'sites' flat [x,y,w,…] list"))?;
    if raw.len() % 3 != 0 || raw.is_empty() {
        return Err(args::bad(
            span,
            "'sites' length must be a non-zero multiple of 3",
        ));
    }
    let sites: Vec<cg::WeightedSite2d> = raw
        .chunks_exact(3)
        .map(|c| cg::WeightedSite2d {
            pos: cg::Point2::new(c[0], c[1]),
            weight: c[2],
        })
        .collect();
    let cap = 3 * sites.len() + 12;
    let mut tris = vec![[0u32; 3]; cap];
    let mut dual = vec![cg::Point2::default(); cap];
    let n =
        cg::regular_triangulation_2d(&sites, &mut tris, &mut dual).map_err(|e| map_err(span, e))?;
    Ok(args::record([
        ("triangles", tris_value(&tris, n)),
        ("dual", pts2_value(&dual[..n])),
        ("count", Value::U64(n as u64)),
    ]))
}

/// `ComputationalGeometry.differential_growth_step` — Laplace–Beltrami-style
/// edge-length growth on a curve network.
///
/// `{vertices:[…], edges:[a,b,…], growth:[f64,…], stiffness?, dt}` →
/// `{vertices:[…]}`.
pub fn differential_growth_step_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut vertices = pts3(
        args::rec(args_v, "vertices").ok_or_else(|| args::bad(span, "missing 'vertices'"))?,
        span,
        "vertices",
    )?;
    let raw_edges = args::rec_f64_list(args_v, "edges")
        .ok_or_else(|| args::bad(span, "missing 'edges' flat index list"))?;
    if raw_edges.len() % 2 != 0 {
        return Err(args::bad(span, "'edges' length must be even"));
    }
    let edges: Vec<(u32, u32)> = raw_edges
        .chunks_exact(2)
        .map(|c| {
            Ok((
                checked_index(c[0], vertices.len(), span, "edges")?,
                checked_index(c[1], vertices.len(), span, "edges")?,
            ))
        })
        .collect::<Result<_, Diagnostic>>()?;
    let growth = args::rec_f64_list(args_v, "growth")
        .ok_or_else(|| args::bad(span, "missing 'growth' list"))?;
    let stiffness = args::rec_f64(args_v, "stiffness").unwrap_or(1.0);
    let dt = args::rec_f64(args_v, "dt").ok_or_else(|| args::bad(span, "missing 'dt'"))?;
    let mut rest = vec![0.0f64; edges.len()];
    cg::edge_rest_lengths(&vertices, &edges, &mut rest).map_err(|e| map_err(span, e))?;
    cg::differential_growth_step(&mut vertices, &edges, &growth, &mut rest, stiffness, dt)
        .map_err(|e| map_err(span, e))?;
    Ok(args::record([("vertices", pts3_value(&vertices))]))
}

/// `ComputationalGeometry.curve_skeleton` — contract a mesh to a 1-D skeleton
/// and extract the node/parent graph.
///
/// `{vertices:[…], triangles:[…], iterations?, initial_weight?, merge_eps?}` →
/// `{nodes:[x,y,z,…], parents:[i32,…], count, contracted:[x,y,z,…]}`.
pub fn curve_skeleton_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let vertices = pts3(
        args::rec(args_v, "vertices").ok_or_else(|| args::bad(span, "missing 'vertices'"))?,
        span,
        "vertices",
    )?;
    let triangles = tri3(
        args::rec(args_v, "triangles").ok_or_else(|| args::bad(span, "missing 'triangles'"))?,
        span,
        "triangles",
    )?;
    let iters = args::rec_u64(args_v, "iterations").unwrap_or(8) as usize;
    let w0 = args::rec_f64(args_v, "initial_weight").unwrap_or(1.0);
    let merge_eps = args::rec_f64(args_v, "merge_eps").unwrap_or(1e-3);
    let n = vertices.len();
    let mut sum = vec![cg::Point3::default(); n];
    let mut cnt = vec![0u32; n];
    let mut work = vertices.clone();
    cg::contract_to_skeleton(
        &vertices, &triangles, iters, w0, &mut sum, &mut cnt, &mut work,
    )
    .map_err(|e| map_err(span, e))?;
    let mut node_of = vec![0u32; n];
    let mut nodes = vec![cg::Point3::default(); n];
    let mut parents = vec![0u32; n];
    let mut queue = vec![0u32; n];
    let nn = cg::extract_skeleton_graph(
        &work,
        &triangles,
        merge_eps,
        &mut node_of,
        &mut nodes,
        &mut parents,
        &mut queue,
    )
    .map_err(|e| map_err(span, e))?;
    Ok(args::record([
        ("nodes", pts3_value(&nodes[..nn])),
        (
            "parents",
            Value::List(
                parents[..nn]
                    .iter()
                    .map(|&p| Value::U64(p as u64))
                    .collect(),
            ),
        ),
        ("count", Value::U64(nn as u64)),
        ("contracted", pts3_value(&work)),
    ]))
}

/// `ComputationalGeometry.dqs_skin` — dual-quaternion blend skinning.
///
/// `{vertices:[…], bones:[{q:[x,y,z,w], t:[x,y,z]},…], weights:[[w,…],…]}` →
/// `{vertices:[…]}`.
pub fn dqs_skin_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let vertices = pts3(
        args::rec(args_v, "vertices").ok_or_else(|| args::bad(span, "missing 'vertices'"))?,
        span,
        "vertices",
    )?;
    let bone_list = args::rec(args_v, "bones")
        .and_then(args::list)
        .ok_or_else(|| args::bad(span, "missing 'bones' list of {q,t} records"))?;
    let mut bones = Vec::with_capacity(bone_list.len());
    for b in bone_list {
        let q = args::rec_f64_list(b, "q")
            .ok_or_else(|| args::bad(span, "bone 'q' must be [x,y,z,w]"))?;
        let t = args::rec_f64_list(b, "t")
            .ok_or_else(|| args::bad(span, "bone 't' must be [x,y,z]"))?;
        if q.len() != 4 || t.len() != 3 {
            return Err(args::bad(span, "bone 'q' must have 4, 't' 3 components"));
        }
        bones.push(cg::DualQuaternion::from_rotation_translation(
            cg::math_geometry::Quaternion {
                w: q[3],
                x: q[0],
                y: q[1],
                z: q[2],
            },
            cg::Point3::new(t[0], t[1], t[2]),
        ));
    }
    let weight_rows = args::rec(args_v, "weights")
        .and_then(args::list)
        .ok_or_else(|| args::bad(span, "missing 'weights' list of per-vertex rows"))?;
    if weight_rows.len() != vertices.len() {
        return Err(args::bad(
            span,
            "'weights' row count must equal vertex count",
        ));
    }
    let mut out = Vec::with_capacity(vertices.len());
    for (v, row_v) in vertices.iter().zip(weight_rows.iter()) {
        let row =
            args::f64s(row_v).ok_or_else(|| args::bad(span, "weight rows must be f64 lists"))?;
        out.push(cg::dqs_skin_vertex(*v, &bones, &row).map_err(|e| map_err(span, e))?);
    }
    Ok(args::record([("vertices", pts3_value(&out))]))
}

/// `ComputationalGeometry.pbd_step` — one position-based-dynamics step.
///
/// `{positions:[…], inv_masses?, edges:[[a,b,rest?],…], tets:[[a,b,c,d,rest?],…],
///  gravity?, damping?, dt, iterations?, ground_z?}` →
/// `{positions:[…], max_residual, iterations_run}`.
pub fn pbd_step_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut positions = pts3(
        args::rec(args_v, "positions").ok_or_else(|| args::bad(span, "missing 'positions'"))?,
        span,
        "positions",
    )?;
    let n = positions.len();
    let inv_masses = args::rec_f64_list(args_v, "inv_masses").unwrap_or_else(|| vec![1.0; n]);
    let mut constraints = Vec::new();
    if let Some(e) = args::rec(args_v, "edges").and_then(args::list) {
        for row in e {
            let c =
                args::f64s(row).ok_or_else(|| args::bad(span, "edge rows must be [a,b,rest?]"))?;
            if c.len() < 2 {
                return Err(args::bad(span, "edge rows must be [a,b,rest?]"));
            }
            let (a, b) = (
                checked_index(c[0], n, span, "edges")?,
                checked_index(c[1], n, span, "edges")?,
            );
            let rest = if c.len() > 2 {
                c[2]
            } else {
                let (pa, pb) = (positions[a as usize], positions[b as usize]);
                ((pa.x - pb.x).powi(2) + (pa.y - pb.y).powi(2) + (pa.z - pb.z).powi(2)).sqrt()
            };
            constraints.push(cg::PbdConstraint::Distance { a, b, rest });
        }
    }
    if let Some(t) = args::rec(args_v, "tets").and_then(args::list) {
        for row in t {
            let c = args::f64s(row)
                .ok_or_else(|| args::bad(span, "tet rows must be [a,b,c,d,rest?]"))?;
            if c.len() < 4 {
                return Err(args::bad(span, "tet rows must be [a,b,c,d,rest?]"));
            }
            let rest = if c.len() > 4 { c[4] } else { 0.0 };
            constraints.push(cg::PbdConstraint::Volume {
                a: checked_index(c[0], n, span, "tets")?,
                b: checked_index(c[1], n, span, "tets")?,
                c: checked_index(c[2], n, span, "tets")?,
                d: checked_index(c[3], n, span, "tets")?,
                rest,
            });
        }
    }
    let gravity = if args::rec_f64_list(args_v, "gravity").is_some() {
        rec_pt3(args_v, "gravity", span)?
    } else {
        cg::Point3::new(0.0, 0.0, -9.81)
    };
    let damping = args::rec_f64(args_v, "damping").unwrap_or(0.02);
    let dt = args::rec_f64(args_v, "dt").ok_or_else(|| args::bad(span, "missing 'dt'"))?;
    let iters = args::rec_u64(args_v, "iterations").unwrap_or(10) as usize;
    let ground_z = args::rec_f64(args_v, "ground_z").unwrap_or(f64::NEG_INFINITY);
    let mut previous = positions.clone();
    let receipt = cg::pbd_step(
        &mut positions,
        &mut previous,
        &inv_masses,
        &constraints,
        gravity,
        damping,
        dt,
        iters,
        ground_z,
    )
    .map_err(|e| map_err(span, e))?;
    Ok(args::record([
        ("positions", pts3_value(&positions)),
        ("max_residual", Value::F64(receipt.max_residual)),
        ("iterations_run", Value::U64(receipt.iterations_run as u64)),
    ]))
}

/// `ComputationalGeometry.medial_axis_3d` — Voronoi-pole medial-axis
/// extraction from a sampled oriented surface.
///
/// `{points:[…], normals:[…]}` → `{samples:[{pole,radius,outer},…], count}`.
pub fn medial_axis_3d_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let points = pts3(
        args::rec(args_v, "points").ok_or_else(|| args::bad(span, "missing 'points'"))?,
        span,
        "points",
    )?;
    let normals = pts3(
        args::rec(args_v, "normals").ok_or_else(|| args::bad(span, "missing 'normals'"))?,
        span,
        "normals",
    )?;
    let cap = 2 * points.len() + 16;
    let mut out = vec![
        cg::MedialSample {
            pole: cg::Point3::default(),
            radius: 0.0,
            outer: false,
        };
        cap
    ];
    let mut tets = vec![[0u32; 4]; 4 * points.len() + 16];
    let n = cg::extract_medial_axis_3d(&points, &normals, &mut out, &mut tets)
        .map_err(|e| map_err(span, e))?;
    let samples = Value::List(
        out[..n]
            .iter()
            .map(|s| {
                args::record([
                    ("pole", pt3_value(s.pole)),
                    ("radius", Value::F64(s.radius)),
                    ("outer", Value::Bool(s.outer)),
                ])
            })
            .collect(),
    );
    Ok(args::record([
        ("samples", samples),
        ("count", Value::U64(n as u64)),
    ]))
}

/// `ComputationalGeometry.sdf_smooth` — smooth Boolean blend of two signed
/// distances (`union`, `intersection`/`intersect`, `difference`/`subtract`).
///
/// `{d1, d2, op, k?}` → `{d}`.
pub fn sdf_smooth_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let d1 = args::rec_f64(args_v, "d1").ok_or_else(|| args::bad(span, "missing 'd1'"))?;
    let d2 = args::rec_f64(args_v, "d2").ok_or_else(|| args::bad(span, "missing 'd2'"))?;
    let op = args::rec_str(args_v, "op").ok_or_else(|| args::bad(span, "missing 'op'"))?;
    let k = args::rec_f64(args_v, "k").unwrap_or(0.1);
    let op = match op {
        "union" => cg::SdfOp::Union,
        "intersection" | "intersect" => cg::SdfOp::Intersection,
        "difference" | "subtract" => cg::SdfOp::Difference,
        other => {
            return Err(args::bad(
                span,
                format!("unknown sdf op '{other}' (union|intersect|subtract)"),
            ))
        }
    };
    Ok(args::record([(
        "d",
        Value::F64(cg::sdf_compose_smooth(d1, d2, op, k)),
    )]))
}

#[cfg(test)]
#[path = "wave28_host_tests.rs"]
mod tests;
