//! Symmetric triangle-surface deviation bracket on a frozen admitted domain.
//!
//! Directed Hausdorff: vertex samples lower-bound `L`; `U` is the min of a
//! covering-triangle vertex maximum and a Lipschitz remainder. Distance to a
//! fixed triangle is convex, so `max_p min_S dist(p,S) <= min_S max_v dist(v,S)`.
//! Midpoint leaves replace a parent so `U` can tighten. IEEE-754 f64 is not
//! outward rounding; slack is applied by the caller of [`certify_surface`].
//! Target acceleration reuses `bvh.rs` (its builder still allocates centroids).

use super::{
    AbsenceReason, BoundRoute, BracketOptions, DecodedMeshPair, GeometryProfile, LedgerError,
    LedgerScratch, LedgerStatus, MeasurementClass, Metric, NUMERICAL_SLACK_OPS, TriangleMesh,
    Units, need,
};
use crate::specialized_libs::computational_geometry::bvh::{
    build_bvh_recursive, query_closest, query_overlap,
};
use crate::specialized_libs::computational_geometry::distance::{
    distance_3d, point_triangle_distance_sq_3d, Aabb,
};
use crate::specialized_libs::computational_geometry::geometry_workspace::production_budget::{
    BudgetError, ProductionBudget,
};
use crate::specialized_libs::computational_geometry::mesh_quality::tri_quality_points;
use crate::specialized_libs::computational_geometry::primitives::Point3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfacePatch {
    pub a: Point3,
    pub b: Point3,
    pub c: Point3,
    pub depth: u16,
}

impl SurfacePatch {
    pub const fn empty() -> Self {
        Self {
            a: Point3::new(0.0, 0.0, 0.0),
            b: Point3::new(0.0, 0.0, 0.0),
            c: Point3::new(0.0, 0.0, 0.0),
            depth: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SurfaceOutcome {
    pub status: LedgerStatus,
    pub enclosing: Metric,
    pub sample: Metric,
    pub diagonal: f64,
    pub slack: f64,
    pub domain_min: Point3,
    pub domain_max: Point3,
    pub method: &'static str,
}

#[derive(Clone, Copy)]
enum Stop { Cont, Cancel, Exhaust }

struct Directed {
    lower: f64,
    upper: f64,
    sample: f64,
    complete: bool,
    stop: Stop,
}

#[derive(Clone, Copy)]
struct BuiltBvh { node_count: usize, root: usize, prims: usize }

pub(crate) fn certify_surface(
    pair: DecodedMeshPair<'_>,
    profile: GeometryProfile,
    options: BracketOptions,
    scratch: &mut LedgerScratch<'_>,
    budget: &mut ProductionBudget<'_>,
) -> Result<SurfaceOutcome, LedgerError> {
    let domain = mesh_aabb(pair.reference)?;
    let mag = coord_mag(pair.reference.vertices).max(coord_mag(pair.decoded.vertices));
    let Some(diagonal) = scale(domain, mag, profile)? else {
        return Ok(outcome(None, f64::NAN, 0.0, numerical_slack(mag), domain, LedgerStatus::Unsupported, "degenerate_extent"));
    };
    let slack = numerical_slack(mag.max(diagonal));
    let width_goal = profile.width_threshold(diagonal);
    if options.route == BoundRoute::SampledVertices {
        let s = sampled_symmetric(pair, scratch, budget)?;
        let lower = (s.sample - slack).max(0.0);
        let status = if matches!(s.stop, Stop::Cancel) {
            LedgerStatus::Cancelled
        } else if lower > profile.upper_threshold(diagonal) {
            LedgerStatus::Failed
        } else {
            LedgerStatus::Inconclusive
        };
        return Ok(outcome(None, s.sample, diagonal, slack, domain, status, "sampled_vertices_estimate"));
    }
    let ab = directed(pair.reference, pair.decoded, options, width_goal, scratch, budget)?;
    if matches!(ab.stop, Stop::Cancel) {
        return Ok(outcome(None, ab.sample, diagonal, slack, domain, LedgerStatus::Cancelled, "cancelled"));
    }
    let ba = directed(pair.decoded, pair.reference, options, width_goal, scratch, budget)?;
    if matches!(ba.stop, Stop::Cancel) {
        return Ok(outcome(None, ba.sample, diagonal, slack, domain, LedgerStatus::Cancelled, "cancelled"));
    }
    let complete = ab.complete && ba.complete && matches!(ab.stop, Stop::Cont) && matches!(ba.stop, Stop::Cont);
    let lower = (ab.lower.max(ba.lower) - slack).max(0.0);
    let upper = ab.upper.max(ba.upper) + slack;
    let enclosing = if complete && lower.is_finite() && upper.is_finite() && upper >= lower {
        Some((lower, upper))
    } else {
        None
    };
    Ok(outcome(
        enclosing,
        ab.sample.max(ba.sample),
        diagonal,
        slack,
        domain,
        decide(enclosing, diagonal, profile),
        "symmetric_triangle_covering_lipschitz_subdivision",
    ))
}

pub(crate) fn decide(enclosing: Option<(f64, f64)>, diagonal: f64, profile: GeometryProfile) -> LedgerStatus {
    let Some((lower, upper)) = enclosing else { return LedgerStatus::Inconclusive };
    let tau = profile.upper_threshold(diagonal);
    if lower > tau {
        LedgerStatus::Failed
    } else if upper <= tau && (upper - lower) <= profile.width_threshold(diagonal) {
        LedgerStatus::Passed
    } else {
        LedgerStatus::Inconclusive
    }
}

pub(crate) fn numerical_slack(mag: f64) -> f64 {
    NUMERICAL_SLACK_OPS * f64::EPSILON * (1.0 + mag.max(0.0))
}

fn sampled_symmetric(
    pair: DecodedMeshPair<'_>, scratch: &mut LedgerScratch<'_>, budget: &mut ProductionBudget<'_>,
) -> Result<Directed, LedgerError> {
    let ab = sampled_directed(pair.reference, pair.decoded, scratch, budget)?;
    if !matches!(ab.stop, Stop::Cont) { return Ok(ab); }
    let ba = sampled_directed(pair.decoded, pair.reference, scratch, budget)?;
    Ok(Directed { lower: ab.lower.max(ba.lower), upper: f64::INFINITY, sample: ab.sample.max(ba.sample), complete: false, stop: ba.stop })
}

fn sampled_directed(
    source: TriangleMesh<'_>, target: TriangleMesh<'_>, scratch: &mut LedgerScratch<'_>, budget: &mut ProductionBudget<'_>,
) -> Result<Directed, LedgerError> {
    let built = match build_target(target, scratch, budget)? {
        Ok(b) => b,
        Err(stop) => return Ok(halt(0.0, 0.0, stop)),
    };
    let mut sample = 0.0;
    for &p in source.vertices {
        match charge(budget, 1)? {
            Stop::Cont => {}
            stop => return Ok(halt(sample, sample, stop)),
        }
        let dist = point_mesh(p, target, built, scratch, budget)?;
        sample = sample.max(dist.lower);
        if !matches!(dist.stop, Stop::Cont) { return Ok(halt(sample, sample, dist.stop)); }
    }
    Ok(Directed { lower: sample, upper: f64::INFINITY, sample, complete: false, stop: Stop::Cont })
}

fn directed(
    source: TriangleMesh<'_>, target: TriangleMesh<'_>, options: BracketOptions, width_goal: f64,
    scratch: &mut LedgerScratch<'_>, budget: &mut ProductionBudget<'_>,
) -> Result<Directed, LedgerError> {
    let built = match build_target(target, scratch, budget)? {
        Ok(b) => b,
        Err(stop) => return Ok(halt(0.0, 0.0, stop)),
    };
    let mut lower: f64 = 0.0;
    let mut upper: f64 = 0.0;
    let mut sample: f64 = 0.0;
    let mut have = false;
    for tri in source.triangles {
        match charge(budget, 1)? {
            Stop::Cont => {}
            stop => return Ok(halt(lower, sample, stop)),
        }
        let (a, b, c) = fetch(source.vertices, *tri)?;
        let one = refine_patch(a, b, c, target, built, options, width_goal, scratch, budget)?;
        lower = lower.max(one.lower);
        sample = sample.max(one.sample);
        if !matches!(one.stop, Stop::Cont) { return Ok(halt(lower, sample, one.stop)); }
        if !one.complete { return Ok(halt(lower, sample, Stop::Exhaust)); }
        upper = if have { upper.max(one.upper) } else { one.upper };
        have = true;
    }
    Ok(Directed { lower, upper: if have { upper } else { 0.0 }, sample, complete: have, stop: Stop::Cont })
}

fn refine_patch(
    a: Point3, b: Point3, c: Point3, target: TriangleMesh<'_>, built: BuiltBvh,
    options: BracketOptions, width_goal: f64, scratch: &mut LedgerScratch<'_>, budget: &mut ProductionBudget<'_>,
) -> Result<Directed, LedgerError> {
    let cap = scratch.patches.len();
    if cap == 0 {
        let e = eval_patch(a, b, c, target, built, scratch, budget)?;
        return Ok(Directed { lower: e.lower, upper: e.upper, sample: e.sample, complete: matches!(e.stop, Stop::Cont) && e.upper.is_finite(), stop: e.stop });
    }
    scratch.patches[0] = SurfacePatch { a, b, c, depth: 0 };
    let mut n = 1usize;
    let mut lower: f64 = 0.0;
    let mut upper: f64 = 0.0;
    let mut sample: f64 = 0.0;
    let mut have = false;
    while n > 0 {
        n -= 1;
        let patch = scratch.patches[n];
        match charge(budget, 1)? {
            Stop::Cont => {}
            stop => return Ok(halt(lower, sample, stop)),
        }
        let e = eval_patch(patch.a, patch.b, patch.c, target, built, scratch, budget)?;
        lower = lower.max(e.lower);
        sample = sample.max(e.sample);
        if !matches!(e.stop, Stop::Cont) { return Ok(halt(lower, sample, e.stop)); }
        let wide = e.upper.is_finite() && (e.upper - e.lower) > width_goal;
        if patch.depth < options.max_subdivision_depth && n + 4 <= cap && wide {
            let ab = mid(patch.a, patch.b);
            let bc = mid(patch.b, patch.c);
            let ca = mid(patch.c, patch.a);
            let d = patch.depth + 1;
            scratch.patches[n] = SurfacePatch { a: patch.a, b: ab, c: ca, depth: d };
            scratch.patches[n + 1] = SurfacePatch { a: ab, b: patch.b, c: bc, depth: d };
            scratch.patches[n + 2] = SurfacePatch { a: ca, b: bc, c: patch.c, depth: d };
            scratch.patches[n + 3] = SurfacePatch { a: ab, b: bc, c: ca, depth: d };
            n += 4;
        } else if !e.upper.is_finite() {
            return Ok(halt(lower, sample, Stop::Exhaust));
        } else {
            upper = if have { upper.max(e.upper) } else { e.upper };
            have = true;
        }
    }
    Ok(Directed { lower, upper: if have { upper } else { 0.0 }, sample, complete: have, stop: Stop::Cont })
}

struct Dist { lower: f64, upper: f64, sample: f64, stop: Stop }

fn eval_patch(
    a: Point3, b: Point3, c: Point3, target: TriangleMesh<'_>, built: BuiltBvh,
    scratch: &mut LedgerScratch<'_>, budget: &mut ProductionBudget<'_>,
) -> Result<Dist, LedgerError> {
    let da = point_mesh(a, target, built, scratch, budget)?;
    let db = point_mesh(b, target, built, scratch, budget)?;
    let dc = point_mesh(c, target, built, scratch, budget)?;
    let stop = first_stop(da.stop, db.stop, dc.stop);
    let lower = da.lower.max(db.lower).max(dc.lower);
    if !matches!(stop, Stop::Cont) {
        return Ok(Dist { lower, upper: f64::INFINITY, sample: lower, stop });
    }
    let _ = tri_quality_points(a, b, c);
    let lip = da.upper.max(db.upper).max(dc.upper) + longest_edge(a, b, c);
    let cover = covering_upper(a, b, c, target, built, scratch, budget)?;
    if !matches!(cover.stop, Stop::Cont) {
        return Ok(Dist { lower, upper: f64::INFINITY, sample: lower, stop: cover.stop });
    }
    Ok(Dist { lower, upper: lip.min(cover.upper), sample: lower, stop: Stop::Cont })
}

fn build_target(
    target: TriangleMesh<'_>, scratch: &mut LedgerScratch<'_>, budget: &mut ProductionBudget<'_>,
) -> Result<Result<BuiltBvh, Stop>, LedgerError> {
    let n = target.triangles.len();
    need(n, scratch.aabbs.len())?;
    need(n, scratch.prim_indices.len())?;
    need(n, scratch.morton_codes.len())?;
    need(n, scratch.sort_indices.len())?;
    need(LedgerScratch::bvh_stack_len(), scratch.stack.len())?;
    need(LedgerScratch::bvh_node_len(n)?, scratch.nodes.len())?;
    match charge(budget, n as u64)? {
        Stop::Cont => {}
        stop => return Ok(Err(stop)),
    }
    for (i, tri) in target.triangles.iter().enumerate() {
        let (a, b, c) = fetch(target.vertices, *tri)?;
        scratch.aabbs[i] = tri_aabb(a, b, c);
    }
    let (node_count, root) = build_bvh_recursive(
        &scratch.aabbs[..n], scratch.nodes, scratch.prim_indices, scratch.morton_codes, scratch.sort_indices,
    )?;
    Ok(Ok(BuiltBvh { node_count, root, prims: n }))
}

fn point_mesh(
    p: Point3, target: TriangleMesh<'_>, built: BuiltBvh, scratch: &mut LedgerScratch<'_>, budget: &mut ProductionBudget<'_>,
) -> Result<Dist, LedgerError> {
    match charge(budget, 1)? {
        Stop::Cont => {}
        stop => return Ok(Dist { lower: 0.0, upper: f64::INFINITY, sample: 0.0, stop }),
    }
    if built.prims == 0 {
        return Ok(Dist { lower: 0.0, upper: f64::INFINITY, sample: 0.0, stop: Stop::Cont });
    }
    if scratch.overlap.len() >= built.prims {
        let mut best = f64::INFINITY;
        for tri in target.triangles {
            let (a, b, c) = fetch(target.vertices, *tri)?;
            best = best.min(point_triangle_distance_sq_3d(p, a, b, c));
        }
        let d = best.max(0.0).sqrt();
        return Ok(Dist { lower: d, upper: d, sample: d, stop: Stop::Cont });
    }
    let Some((idx, aabb_d2)) = query_closest(
        scratch.nodes, &scratch.aabbs[..built.prims], scratch.prim_indices, built.root, built.node_count, p, scratch.stack,
    )? else {
        return Ok(Dist { lower: 0.0, upper: f64::INFINITY, sample: 0.0, stop: Stop::Cont });
    };
    let (a, b, c) = fetch(target.vertices, target.triangles[idx as usize])?;
    let mut best = point_triangle_distance_sq_3d(p, a, b, c);
    let r = best.max(0.0).sqrt();
    let q = expand(Aabb::new(p, p), r);
    let k = query_overlap(
        scratch.nodes, &scratch.aabbs[..built.prims], scratch.prim_indices, built.root, built.node_count, &q, scratch.overlap, scratch.stack,
    )?;
    match charge(budget, k as u64)? {
        Stop::Cont => {}
        stop => return Ok(Dist { lower: aabb_d2.max(0.0).sqrt(), upper: best.max(0.0).sqrt(), sample: aabb_d2.max(0.0).sqrt(), stop }),
    }
    for i in 0..k {
        let (ta, tb, tc) = fetch(target.vertices, target.triangles[scratch.overlap[i] as usize])?;
        best = best.min(point_triangle_distance_sq_3d(p, ta, tb, tc));
    }
    let upper = best.max(0.0).sqrt();
    if k == scratch.overlap.len() {
        Ok(Dist { lower: aabb_d2.max(0.0).sqrt(), upper, sample: aabb_d2.max(0.0).sqrt(), stop: Stop::Cont })
    } else {
        Ok(Dist { lower: upper, upper, sample: upper, stop: Stop::Cont })
    }
}

fn covering_upper(
    a: Point3, b: Point3, c: Point3, target: TriangleMesh<'_>, built: BuiltBvh,
    scratch: &mut LedgerScratch<'_>, budget: &mut ProductionBudget<'_>,
) -> Result<Dist, LedgerError> {
    match charge(budget, 1)? {
        Stop::Cont => {}
        stop => return Ok(Dist { lower: 0.0, upper: f64::INFINITY, sample: 0.0, stop }),
    }
    let mut best = f64::INFINITY;
    let scan = if scratch.overlap.len() >= built.prims {
        built.prims
    } else {
        let q = expand(tri_aabb(a, b, c), longest_edge(a, b, c));
        let k = query_overlap(
            scratch.nodes, &scratch.aabbs[..built.prims], scratch.prim_indices, built.root, built.node_count, &q, scratch.overlap, scratch.stack,
        )?;
        match charge(budget, k as u64)? {
            Stop::Cont => {}
            stop => return Ok(Dist { lower: 0.0, upper: f64::INFINITY, sample: 0.0, stop }),
        }
        if k == 0 { return Ok(Dist { lower: 0.0, upper: f64::INFINITY, sample: 0.0, stop: Stop::Cont }); }
        k
    };
    for i in 0..scan {
        let tri = if scratch.overlap.len() >= built.prims {
            target.triangles[i]
        } else {
            target.triangles[scratch.overlap[i] as usize]
        };
        let s = fetch(target.vertices, tri)?;
        best = best.min(vert_tri_max(a, b, c, s.0, s.1, s.2));
    }
    Ok(Dist { lower: 0.0, upper: best, sample: 0.0, stop: Stop::Cont })
}

fn vert_tri_max(a: Point3, b: Point3, c: Point3, sa: Point3, sb: Point3, sc: Point3) -> f64 {
    point_triangle_distance_sq_3d(a, sa, sb, sc)
        .max(point_triangle_distance_sq_3d(b, sa, sb, sc))
        .max(point_triangle_distance_sq_3d(c, sa, sb, sc))
        .max(0.0)
        .sqrt()
}

fn fetch(vertices: &[Point3], tri: [u32; 3]) -> Result<(Point3, Point3, Point3), LedgerError> {
    let pick = |i: u32| vertices.get(i as usize).copied().ok_or(LedgerError::InvalidInput("triangle_index"));
    let (a, b, c) = (pick(tri[0])?, pick(tri[1])?, pick(tri[2])?);
    if ![a.x, a.y, a.z, b.x, b.y, b.z, c.x, c.y, c.z].iter().all(|v| v.is_finite()) {
        return Err(LedgerError::InvalidInput("non_finite_vertex"));
    }
    Ok((a, b, c))
}

fn tri_aabb(a: Point3, b: Point3, c: Point3) -> Aabb {
    Aabb::new(
        Point3::new(a.x.min(b.x).min(c.x), a.y.min(b.y).min(c.y), a.z.min(b.z).min(c.z)),
        Point3::new(a.x.max(b.x).max(c.x), a.y.max(b.y).max(c.y), a.z.max(b.z).max(c.z)),
    )
}

fn expand(aabb: Aabb, r: f64) -> Aabb {
    Aabb::new(
        Point3::new(aabb.min.x - r, aabb.min.y - r, aabb.min.z - r),
        Point3::new(aabb.max.x + r, aabb.max.y + r, aabb.max.z + r),
    )
}

fn longest_edge(a: Point3, b: Point3, c: Point3) -> f64 {
    distance_3d(a, b).max(distance_3d(b, c)).max(distance_3d(c, a))
}

fn mid(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x + 0.5 * (b.x - a.x), a.y + 0.5 * (b.y - a.y), a.z + 0.5 * (b.z - a.z))
}

fn coord_mag(vertices: &[Point3]) -> f64 {
    vertices.iter().fold(0.0, |m, p| m.max(p.x.abs()).max(p.y.abs()).max(p.z.abs()))
}

fn mesh_aabb(mesh: TriangleMesh<'_>) -> Result<Aabb, LedgerError> {
    if mesh.vertices.is_empty() || mesh.triangles.is_empty() {
        return Err(LedgerError::InvalidInput("empty_mesh"));
    }
    let mut aabb = Aabb::new(mesh.vertices[0], mesh.vertices[0]);
    for &p in mesh.vertices {
        if ![p.x, p.y, p.z].iter().all(|v| v.is_finite()) {
            return Err(LedgerError::InvalidInput("non_finite_vertex"));
        }
        aabb = aabb.union(&Aabb::new(p, p));
    }
    for &tri in mesh.triangles { let _ = fetch(mesh.vertices, tri)?; }
    Ok(aabb)
}

fn scale(domain: Aabb, mag: f64, profile: GeometryProfile) -> Result<Option<f64>, LedgerError> {
    let d = distance_3d(domain.min, domain.max);
    if d.is_finite() && d > 8.0 * f64::EPSILON * (1.0 + mag) { return Ok(Some(d)); }
    match profile.absolute_scale {
        Some(s) if s.is_finite() && s > 0.0 => Ok(Some(s)),
        Some(_) => Err(LedgerError::InvalidInput("absolute_scale")),
        None => Ok(None),
    }
}

fn charge(budget: &mut ProductionBudget<'_>, work: u64) -> Result<Stop, LedgerError> {
    match budget.checkpoint(work) {
        Ok(()) => Ok(Stop::Cont),
        Err(BudgetError::Cancelled) => Ok(Stop::Cancel),
        Err(BudgetError::WorkLimitExceeded { .. }) => Ok(Stop::Exhaust),
        Err(err) => Err(LedgerError::Budget(err)),
    }
}

fn halt(lower: f64, sample: f64, stop: Stop) -> Directed {
    Directed { lower, upper: f64::INFINITY, sample, complete: false, stop }
}

fn first_stop(a: Stop, b: Stop, c: Stop) -> Stop {
    if matches!(a, Stop::Cancel) || matches!(b, Stop::Cancel) || matches!(c, Stop::Cancel) {
        Stop::Cancel
    } else if matches!(a, Stop::Exhaust) || matches!(b, Stop::Exhaust) || matches!(c, Stop::Exhaust) {
        Stop::Exhaust
    } else {
        Stop::Cont
    }
}

fn outcome(
    enclosing: Option<(f64, f64)>, sample: f64, diagonal: f64, slack: f64, domain: Aabb,
    status: LedgerStatus, method: &'static str,
) -> SurfaceOutcome {
    let sample = if sample.is_finite() {
        Metric::Scalar { value: sample, units: Units::AssetLength, class: MeasurementClass::SampledEstimate }
    } else {
        Metric::absent(AbsenceReason::IncompleteEnclosure)
    };
    let enclosing = match enclosing {
        Some((lower, upper)) => Metric::Bracket { lower, upper, units: Units::AssetLength, class: MeasurementClass::EnclosingBound },
        None => Metric::absent(match status {
            LedgerStatus::Cancelled => AbsenceReason::Cancelled,
            LedgerStatus::Unsupported => AbsenceReason::DomainEmpty,
            _ => AbsenceReason::IncompleteEnclosure,
        }),
    };
    SurfaceOutcome { status, enclosing, sample, diagonal, slack, domain_min: domain.min, domain_max: domain.max, method }
}
