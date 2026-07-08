//! P15 - Deterministic parallel primitives, dynamic indices, and spatial graphs.
//!
//! These routines are scalar reference implementations with deterministic
//! ordering. They are the CPU/WASM oracle surface for later multicore/GPU
//! acceleration: scan/reduce, batch-dynamic point index queries, WSPD-style
//! pair coverage, nearest-pair, EMST, kNN graph, and greedy spanners.

use super::primitives::{orientation_2, Orientation, Point2, Point3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeightedEdge {
    pub a: u32,
    pub b: u32,
    pub weight: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConflictEdge {
    pub owner: u32,
    pub candidate: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BenchmarkReport {
    pub seed: u64,
    pub generated: u32,
    pub checksum: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearestPair {
    pub a: u32,
    pub b: u32,
    pub distance_sq: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnclosingBall {
    pub center: Point3,
    pub radius: f64,
    pub support: [u32; 4],
    pub support_count: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WellSeparatedPair {
    pub a_begin: u32,
    pub a_len: u32,
    pub b_begin: u32,
    pub b_len: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicKdRecord {
    pub point: Point3,
    pub id: u64,
    pub tombstone: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BatchDynamicKdTree {
    pub records: Vec<DynamicKdRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialGeneratorKind {
    UniformCube,
    Clustered,
    SkewLine,
    PathologicalDuplicates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeterministicGeometryError {
    OutputTooSmall { required: usize },
    InvalidParameter,
    EmptyInput,
}

pub fn deterministic_scan_u64(
    input: &[u64],
    out_prefix: &mut [u64],
) -> Result<u64, DeterministicGeometryError> {
    if out_prefix.len() < input.len() {
        return Err(DeterministicGeometryError::OutputTooSmall {
            required: input.len(),
        });
    }
    let mut acc = 0u64;
    for (i, &x) in input.iter().enumerate() {
        out_prefix[i] = acc;
        acc = acc.wrapping_add(x);
    }
    Ok(acc)
}

pub fn deterministic_reduce_f64(input: &[f64]) -> f64 {
    let mut vals = input.to_vec();
    vals.sort_by(|a, b| {
        a.abs()
            .partial_cmp(&b.abs())
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    vals.into_iter().fold(0.0, |a, b| a + b)
}

pub fn deterministic_pack_u32(
    input: &[u32],
    keep: &[bool],
    out: &mut [u32],
) -> Result<usize, DeterministicGeometryError> {
    if input.len() != keep.len() {
        return Err(DeterministicGeometryError::InvalidParameter);
    }
    let required = keep.iter().filter(|&&k| k).count();
    if out.len() < required {
        return Err(DeterministicGeometryError::OutputTooSmall { required });
    }
    let mut n = 0usize;
    for (&x, &k) in input.iter().zip(keep.iter()) {
        if k {
            out[n] = x;
            n += 1;
        }
    }
    Ok(n)
}

pub fn list_rank_successors(
    successors: &[u32],
    head: usize,
    out_rank: &mut [u32],
) -> Result<usize, DeterministicGeometryError> {
    if successors.is_empty() || head >= successors.len() {
        return Err(DeterministicGeometryError::EmptyInput);
    }
    if out_rank.len() < successors.len() {
        return Err(DeterministicGeometryError::OutputTooSmall {
            required: successors.len(),
        });
    }
    out_rank.fill(u32::MAX);
    let mut cur = head;
    let mut rank = 0u32;
    while cur != u32::MAX as usize {
        if cur >= successors.len() || out_rank[cur] != u32::MAX {
            return Err(DeterministicGeometryError::InvalidParameter);
        }
        out_rank[cur] = rank;
        rank += 1;
        cur = successors[cur] as usize;
    }
    Ok(rank as usize)
}

pub fn tree_contract_roots(
    parents: &[u32],
    out_root: &mut [u32],
) -> Result<usize, DeterministicGeometryError> {
    if out_root.len() < parents.len() {
        return Err(DeterministicGeometryError::OutputTooSmall {
            required: parents.len(),
        });
    }
    let mut roots = 0usize;
    for i in 0..parents.len() {
        let mut cur = i;
        let mut guard = 0usize;
        while parents[cur] as usize != cur {
            cur = parents[cur] as usize;
            guard += 1;
            if cur >= parents.len() || guard > parents.len() {
                return Err(DeterministicGeometryError::InvalidParameter);
            }
        }
        out_root[i] = cur as u32;
        if cur == i {
            roots += 1;
        }
    }
    Ok(roots)
}

pub fn nearest_pair(points: &[Point3]) -> Result<NearestPair, DeterministicGeometryError> {
    if points.len() < 2 {
        return Err(DeterministicGeometryError::EmptyInput);
    }
    let mut best = NearestPair {
        a: 0,
        b: 1,
        distance_sq: distance_sq(points[0], points[1]),
    };
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            let d = distance_sq(points[i], points[j]);
            if d < best.distance_sq
                || (d == best.distance_sq && (i as u32, j as u32) < (best.a, best.b))
            {
                best = NearestPair {
                    a: i as u32,
                    b: j as u32,
                    distance_sq: d,
                };
            }
        }
    }
    Ok(best)
}

pub fn bichromatic_nearest_pair(
    red: &[Point3],
    blue: &[Point3],
) -> Result<NearestPair, DeterministicGeometryError> {
    if red.is_empty() || blue.is_empty() {
        return Err(DeterministicGeometryError::EmptyInput);
    }
    let mut best = NearestPair {
        a: 0,
        b: 0,
        distance_sq: distance_sq(red[0], blue[0]),
    };
    for (i, &r) in red.iter().enumerate() {
        for (j, &b) in blue.iter().enumerate() {
            let d = distance_sq(r, b);
            if d < best.distance_sq
                || (d == best.distance_sq && (i as u32, j as u32) < (best.a, best.b))
            {
                best = NearestPair {
                    a: i as u32,
                    b: j as u32,
                    distance_sq: d,
                };
            }
        }
    }
    Ok(best)
}

pub fn all_nearest_neighbours(
    points: &[Point3],
    out: &mut [NearestPair],
) -> Result<usize, DeterministicGeometryError> {
    if out.len() < points.len() {
        return Err(DeterministicGeometryError::OutputTooSmall {
            required: points.len(),
        });
    }
    for i in 0..points.len() {
        let mut best = NearestPair {
            a: i as u32,
            b: i as u32,
            distance_sq: f64::INFINITY,
        };
        for j in 0..points.len() {
            if i == j {
                continue;
            }
            let d = distance_sq(points[i], points[j]);
            if d < best.distance_sq || (d == best.distance_sq && (j as u32) < best.b) {
                best = NearestPair {
                    a: i as u32,
                    b: j as u32,
                    distance_sq: d,
                };
            }
        }
        out[i] = best;
    }
    Ok(points.len())
}

pub fn knn_graph(
    points: &[Point3],
    k: usize,
    out: &mut [WeightedEdge],
) -> Result<usize, DeterministicGeometryError> {
    if k == 0 || k >= points.len() {
        return Err(DeterministicGeometryError::InvalidParameter);
    }
    let required = points.len() * k;
    if out.len() < required {
        return Err(DeterministicGeometryError::OutputTooSmall { required });
    }
    let mut written = 0usize;
    for i in 0..points.len() {
        let mut ds: Vec<(f64, u32)> = (0..points.len())
            .filter(|&j| j != i)
            .map(|j| (distance_sq(points[i], points[j]), j as u32))
            .collect();
        ds.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(core::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
        });
        for &(d, j) in ds.iter().take(k) {
            out[written] = WeightedEdge {
                a: i as u32,
                b: j,
                weight: d.sqrt(),
            };
            written += 1;
        }
    }
    Ok(written)
}

pub fn euclidean_mst(
    points: &[Point3],
    out: &mut [WeightedEdge],
) -> Result<usize, DeterministicGeometryError> {
    if points.len() < 2 {
        return Err(DeterministicGeometryError::EmptyInput);
    }
    if out.len() < points.len() - 1 {
        return Err(DeterministicGeometryError::OutputTooSmall {
            required: points.len() - 1,
        });
    }
    let mut edges = Vec::new();
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            edges.push(WeightedEdge {
                a: i as u32,
                b: j as u32,
                weight: distance_sq(points[i], points[j]).sqrt(),
            });
        }
    }
    edges.sort_by(|a, b| {
        a.weight
            .partial_cmp(&b.weight)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(a.a.cmp(&b.a))
            .then(a.b.cmp(&b.b))
    });
    let mut parent: Vec<usize> = (0..points.len()).collect();
    let mut n = 0usize;
    for e in edges {
        let ra = find(&mut parent, e.a as usize);
        let rb = find(&mut parent, e.b as usize);
        if ra != rb {
            parent[ra] = rb;
            out[n] = e;
            n += 1;
            if n == points.len() - 1 {
                break;
            }
        }
    }
    Ok(n)
}

pub fn reservation_batch_hull_2d(
    points: &[Point2],
    out_indices: &mut [u32],
) -> Result<usize, DeterministicGeometryError> {
    if points.len() < 3 {
        return Err(DeterministicGeometryError::EmptyInput);
    }
    let mut order: Vec<usize> = (0..points.len()).collect();
    order.sort_by(|&a, &b| {
        points[a]
            .x
            .partial_cmp(&points[b].x)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then_with(|| {
                points[a]
                    .y
                    .partial_cmp(&points[b].y)
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .then(a.cmp(&b))
    });
    let mut hull: Vec<usize> = Vec::with_capacity(points.len() * 2);
    for &idx in &order {
        while hull.len() >= 2
            && orientation_2(
                points[hull[hull.len() - 2]],
                points[hull[hull.len() - 1]],
                points[idx],
            ) != Orientation::CounterClockwise
        {
            hull.pop();
        }
        hull.push(idx);
    }
    let lower_len = hull.len();
    for &idx in order.iter().rev().skip(1) {
        while hull.len() > lower_len
            && orientation_2(
                points[hull[hull.len() - 2]],
                points[hull[hull.len() - 1]],
                points[idx],
            ) != Orientation::CounterClockwise
        {
            hull.pop();
        }
        hull.push(idx);
    }
    hull.pop();
    if out_indices.len() < hull.len() {
        return Err(DeterministicGeometryError::OutputTooSmall {
            required: hull.len(),
        });
    }
    for (slot, idx) in out_indices.iter_mut().zip(hull.iter()) {
        *slot = *idx as u32;
    }
    Ok(hull.len())
}

pub fn conflict_graph_pairs(
    owners: usize,
    candidates: usize,
    seed: u64,
    out: &mut [ConflictEdge],
) -> Result<usize, DeterministicGeometryError> {
    let required = owners.saturating_mul(candidates);
    if out.len() < required {
        return Err(DeterministicGeometryError::OutputTooSmall { required });
    }
    let mut order = vec![0u32; required];
    seeded_incremental_order(required, seed, &mut order)?;
    for (i, &flat) in order.iter().enumerate() {
        out[i] = ConflictEdge {
            owner: flat / candidates as u32,
            candidate: flat % candidates as u32,
        };
    }
    Ok(required)
}

pub fn gabriel_graph(
    points: &[Point3],
    out: &mut [WeightedEdge],
) -> Result<usize, DeterministicGeometryError> {
    empty_region_graph(points, 1.0, out)
}

pub fn beta_skeleton_graph(
    points: &[Point3],
    beta: f64,
    out: &mut [WeightedEdge],
) -> Result<usize, DeterministicGeometryError> {
    if !(beta.is_finite() && beta > 0.0) {
        return Err(DeterministicGeometryError::InvalidParameter);
    }
    empty_region_graph(points, beta, out)
}

pub fn greedy_spanner(
    points: &[Point3],
    stretch: f64,
    out: &mut [WeightedEdge],
) -> Result<usize, DeterministicGeometryError> {
    if !(stretch.is_finite() && stretch >= 1.0) {
        return Err(DeterministicGeometryError::InvalidParameter);
    }
    let mut candidates = Vec::new();
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            candidates.push(WeightedEdge {
                a: i as u32,
                b: j as u32,
                weight: distance_sq(points[i], points[j]).sqrt(),
            });
        }
    }
    candidates.sort_by(|a, b| {
        a.weight
            .partial_cmp(&b.weight)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(a.a.cmp(&b.a))
            .then(a.b.cmp(&b.b))
    });
    let mut graph: Vec<WeightedEdge> = Vec::new();
    for e in candidates {
        if shortest_path(points.len(), &graph, e.a, e.b) > stretch * e.weight {
            if graph.len() >= out.len() {
                return Err(DeterministicGeometryError::OutputTooSmall {
                    required: graph.len() + 1,
                });
            }
            graph.push(e);
        }
    }
    out[..graph.len()].copy_from_slice(&graph);
    Ok(graph.len())
}

pub fn seeded_incremental_order(
    count: usize,
    seed: u64,
    out: &mut [u32],
) -> Result<usize, DeterministicGeometryError> {
    if out.len() < count {
        return Err(DeterministicGeometryError::OutputTooSmall { required: count });
    }
    for (i, slot) in out.iter_mut().take(count).enumerate() {
        *slot = i as u32;
    }
    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    for i in (1..count).rev() {
        let j = (splitmix64_next(&mut state) as usize) % (i + 1);
        out.swap(i, j);
    }
    Ok(count)
}

pub fn smallest_enclosing_ball(
    points: &[Point3],
) -> Result<EnclosingBall, DeterministicGeometryError> {
    if points.is_empty() {
        return Err(DeterministicGeometryError::EmptyInput);
    }
    let mut best = EnclosingBall {
        center: points[0],
        radius: 0.0,
        support: [0, 0, 0, 0],
        support_count: 1,
    };
    for i in 0..points.len() {
        if distance_sq(points[i], best.center) > best.radius * best.radius + 1e-12 {
            best = ball_from_one(points, i);
            for j in 0..i {
                if distance_sq(points[j], best.center) > best.radius * best.radius + 1e-12 {
                    best = ball_from_two(points, i, j);
                    for k in 0..j {
                        if distance_sq(points[k], best.center) > best.radius * best.radius + 1e-12 {
                            if let Some(ball) = ball_from_three(points, i, j, k) {
                                best = ball;
                            }
                            for l in 0..k {
                                if distance_sq(points[l], best.center)
                                    > best.radius * best.radius + 1e-12
                                {
                                    if let Some(ball) = ball_from_four(points, i, j, k, l) {
                                        best = ball;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(best)
}

pub fn build_batch_dynamic_kd_tree(points: &[Point3]) -> BatchDynamicKdTree {
    let mut records: Vec<DynamicKdRecord> = points
        .iter()
        .enumerate()
        .map(|(i, &point)| DynamicKdRecord {
            point,
            id: i as u64,
            tombstone: false,
        })
        .collect();
    records.sort_by(record_order);
    BatchDynamicKdTree { records }
}

pub fn dynamic_kd_insert(tree: &mut BatchDynamicKdTree, point: Point3, id: u64) {
    tree.records.push(DynamicKdRecord {
        point,
        id,
        tombstone: false,
    });
    tree.records.sort_by(record_order);
}

pub fn dynamic_kd_delete(tree: &mut BatchDynamicKdTree, id: u64) -> bool {
    let mut deleted = false;
    for record in &mut tree.records {
        if record.id == id && !record.tombstone {
            record.tombstone = true;
            deleted = true;
        }
    }
    deleted
}

pub fn dynamic_kd_compact(tree: &mut BatchDynamicKdTree) {
    tree.records.retain(|r| !r.tombstone);
}

pub fn dynamic_kd_nearest(tree: &BatchDynamicKdTree, query: Point3) -> Option<NearestPair> {
    let mut best: Option<NearestPair> = None;
    for record in &tree.records {
        if record.tombstone {
            continue;
        }
        let d = distance_sq(record.point, query);
        let candidate = NearestPair {
            a: record.id as u32,
            b: u32::MAX,
            distance_sq: d,
        };
        if best.map_or(true, |b| {
            d < b.distance_sq || (d == b.distance_sq && candidate.a < b.a)
        }) {
            best = Some(candidate);
        }
    }
    best
}

pub fn well_separated_pairs(
    points: &[Point3],
    separation: f64,
    out: &mut [WellSeparatedPair],
) -> Result<usize, DeterministicGeometryError> {
    if !(separation.is_finite() && separation > 0.0) {
        return Err(DeterministicGeometryError::InvalidParameter);
    }
    let mut written = 0usize;
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            if written >= out.len() {
                return Err(DeterministicGeometryError::OutputTooSmall {
                    required: written + 1,
                });
            }
            let d = distance_sq(points[i], points[j]).sqrt();
            if d >= separation {
                out[written] = WellSeparatedPair {
                    a_begin: i as u32,
                    a_len: 1,
                    b_begin: j as u32,
                    b_len: 1,
                };
                written += 1;
            }
        }
    }
    Ok(written)
}

pub fn external_memory_nearest_tiles(
    points: &[Point3],
    queries: &[Point3],
    tile_size: usize,
    out: &mut [NearestPair],
) -> Result<usize, DeterministicGeometryError> {
    if tile_size == 0 {
        return Err(DeterministicGeometryError::InvalidParameter);
    }
    if points.is_empty() || queries.is_empty() {
        return Err(DeterministicGeometryError::EmptyInput);
    }
    if out.len() < queries.len() {
        return Err(DeterministicGeometryError::OutputTooSmall {
            required: queries.len(),
        });
    }
    for (qi, &query) in queries.iter().enumerate() {
        let mut best = NearestPair {
            a: qi as u32,
            b: 0,
            distance_sq: f64::INFINITY,
        };
        let mut base = 0usize;
        while base < points.len() {
            let end = (base + tile_size).min(points.len());
            for (offset, &point) in points[base..end].iter().enumerate() {
                let pi = base + offset;
                let d = distance_sq(point, query);
                if d < best.distance_sq || (d == best.distance_sq && pi as u32 <= best.b) {
                    best = NearestPair {
                        a: qi as u32,
                        b: pi as u32,
                        distance_sq: d,
                    };
                }
            }
            base = end;
        }
        out[qi] = best;
    }
    Ok(queries.len())
}

pub fn generate_spatial_points(
    kind: SpatialGeneratorKind,
    count: usize,
    seed: u64,
    out: &mut [Point3],
) -> Result<usize, DeterministicGeometryError> {
    if out.len() < count {
        return Err(DeterministicGeometryError::OutputTooSmall { required: count });
    }
    let mut state = seed ^ 0xD1B5_4A32_D192_ED03;
    for (i, slot) in out.iter_mut().take(count).enumerate() {
        let u = unit_f64(&mut state);
        let v = unit_f64(&mut state);
        let w = unit_f64(&mut state);
        *slot = match kind {
            SpatialGeneratorKind::UniformCube => Point3::new(u, v, w),
            SpatialGeneratorKind::Clustered => {
                let center = if i % 2 == 0 { 0.25 } else { 0.75 };
                Point3::new(center + (u - 0.5) * 0.1, center + (v - 0.5) * 0.1, w)
            }
            SpatialGeneratorKind::SkewLine => {
                let t = i as f64 / count.max(1) as f64;
                Point3::new(t, t * 1e-6 + u * 1e-9, t * 2e-6 + v * 1e-9)
            }
            SpatialGeneratorKind::PathologicalDuplicates => {
                Point3::new((i % 4) as f64, ((i / 4) % 4) as f64, 0.0)
            }
        };
    }
    Ok(count)
}

pub fn reproducible_benchmark_report(points: &[Point3], seed: u64) -> BenchmarkReport {
    let mut checksum = seed ^ points.len() as u64;
    for &p in points {
        checksum = checksum.rotate_left(13) ^ p.x.to_bits();
        checksum = checksum.rotate_left(17) ^ p.y.to_bits();
        checksum = checksum.rotate_left(19) ^ p.z.to_bits();
    }
    BenchmarkReport {
        seed,
        generated: points.len() as u32,
        checksum,
    }
}

fn empty_region_graph(
    points: &[Point3],
    beta: f64,
    out: &mut [WeightedEdge],
) -> Result<usize, DeterministicGeometryError> {
    let mut written = 0usize;
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            let d2 = distance_sq(points[i], points[j]);
            let radius2 = d2 * beta * beta * 0.25;
            let center = scale(add(points[i], points[j]), 0.5);
            let mut empty = true;
            for (k, &p) in points.iter().enumerate() {
                if k != i && k != j && distance_sq(p, center) < radius2 - 1e-12 {
                    empty = false;
                    break;
                }
            }
            if empty {
                if written >= out.len() {
                    return Err(DeterministicGeometryError::OutputTooSmall {
                        required: written + 1,
                    });
                }
                out[written] = WeightedEdge {
                    a: i as u32,
                    b: j as u32,
                    weight: d2.sqrt(),
                };
                written += 1;
            }
        }
    }
    Ok(written)
}

fn shortest_path(n: usize, edges: &[WeightedEdge], a: u32, b: u32) -> f64 {
    let mut dist = vec![f64::INFINITY; n];
    let mut used = vec![false; n];
    dist[a as usize] = 0.0;
    for _ in 0..n {
        let mut best = None;
        for i in 0..n {
            if !used[i] && best.map_or(true, |j| dist[i] < dist[j]) {
                best = Some(i);
            }
        }
        let Some(u) = best else { break };
        used[u] = true;
        for e in edges {
            let v = if e.a as usize == u {
                e.b as usize
            } else if e.b as usize == u {
                e.a as usize
            } else {
                continue;
            };
            dist[v] = dist[v].min(dist[u] + e.weight);
        }
    }
    dist[b as usize]
}

fn find(parent: &mut [usize], x: usize) -> usize {
    if parent[x] != x {
        parent[x] = find(parent, parent[x]);
    }
    parent[x]
}

fn distance_sq(a: Point3, b: Point3) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    dx * dx + dy * dy + dz * dz
}

fn ball_from_one(points: &[Point3], i: usize) -> EnclosingBall {
    EnclosingBall {
        center: points[i],
        radius: 0.0,
        support: [i as u32, 0, 0, 0],
        support_count: 1,
    }
}

fn ball_from_two(points: &[Point3], i: usize, j: usize) -> EnclosingBall {
    let center = scale(add(points[i], points[j]), 0.5);
    EnclosingBall {
        center,
        radius: distance_sq(points[i], points[j]).sqrt() * 0.5,
        support: [i as u32, j as u32, 0, 0],
        support_count: 2,
    }
}

fn ball_from_three(points: &[Point3], i: usize, j: usize, k: usize) -> Option<EnclosingBall> {
    let a = points[i];
    let b = points[j];
    let c = points[k];
    let ab = sub(b, a);
    let ac = sub(c, a);
    let n = cross(ab, ac);
    let denom = 2.0 * dot(n, n);
    if denom.abs() < 1e-18 {
        return None;
    }
    let term1 = scale(cross(n, ab), dot(ac, ac));
    let term2 = scale(cross(ac, n), dot(ab, ab));
    let center = add(a, scale(add(term1, term2), 1.0 / denom));
    Some(EnclosingBall {
        center,
        radius: distance_sq(center, a).sqrt(),
        support: [i as u32, j as u32, k as u32, 0],
        support_count: 3,
    })
}

fn ball_from_four(
    points: &[Point3],
    i: usize,
    j: usize,
    k: usize,
    l: usize,
) -> Option<EnclosingBall> {
    let a = points[i];
    let ba = sub(points[j], a);
    let ca = sub(points[k], a);
    let da = sub(points[l], a);
    let rhs = [0.5 * dot(ba, ba), 0.5 * dot(ca, ca), 0.5 * dot(da, da)];
    let det = dot(ba, cross(ca, da));
    if det.abs() < 1e-18 {
        return None;
    }
    let x = dot(
        Point3::new(rhs[0], ba.y, ba.z),
        cross(
            Point3::new(rhs[1], ca.y, ca.z),
            Point3::new(rhs[2], da.y, da.z),
        ),
    ) / det;
    let y = dot(
        Point3::new(ba.x, rhs[0], ba.z),
        cross(
            Point3::new(ca.x, rhs[1], ca.z),
            Point3::new(da.x, rhs[2], da.z),
        ),
    ) / det;
    let z = dot(
        Point3::new(ba.x, ba.y, rhs[0]),
        cross(
            Point3::new(ca.x, ca.y, rhs[1]),
            Point3::new(da.x, da.y, rhs[2]),
        ),
    ) / det;
    let center = add(a, Point3::new(x, y, z));
    Some(EnclosingBall {
        center,
        radius: distance_sq(center, a).sqrt(),
        support: [i as u32, j as u32, k as u32, l as u32],
        support_count: 4,
    })
}

fn record_order(a: &DynamicKdRecord, b: &DynamicKdRecord) -> core::cmp::Ordering {
    a.point
        .x
        .partial_cmp(&b.point.x)
        .unwrap_or(core::cmp::Ordering::Equal)
        .then_with(|| {
            a.point
                .y
                .partial_cmp(&b.point.y)
                .unwrap_or(core::cmp::Ordering::Equal)
        })
        .then_with(|| {
            a.point
                .z
                .partial_cmp(&b.point.z)
                .unwrap_or(core::cmp::Ordering::Equal)
        })
        .then(a.id.cmp(&b.id))
}

fn splitmix64_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn unit_f64(state: &mut u64) -> f64 {
    ((splitmix64_next(state) >> 11) as f64) / ((1u64 << 53) as f64)
}

fn add(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn scale(a: Point3, s: f64) -> Point3 {
    Point3::new(a.x * s, a.y * s, a.z * s)
}

fn dot(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn cross(a: Point3, b: Point3) -> Point3 {
    Point3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts() -> Vec<Point3> {
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ]
    }

    #[test]
    fn scan_is_deterministic() {
        let mut out = [0; 4];
        assert_eq!(deterministic_scan_u64(&[1, 2, 3, 4], &mut out).unwrap(), 10);
        assert_eq!(out, [0, 1, 3, 6]);
    }

    #[test]
    fn mst_has_n_minus_one_edges() {
        let p = pts();
        let mut out = vec![
            WeightedEdge {
                a: 0,
                b: 0,
                weight: 0.0
            };
            p.len() - 1
        ];
        assert_eq!(euclidean_mst(&p, &mut out).unwrap(), 3);
    }

    #[test]
    fn nearest_pair_tie_breaks_canonically() {
        let p = pts();
        let pair = nearest_pair(&p).unwrap();
        assert_eq!((pair.a, pair.b), (0, 1));
    }
}
