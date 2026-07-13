//! P16 - Collision detection and motion planning.
//!
//! Deterministic 2-D planning primitives over polygonal obstacles. This is the
//! scalar reference layer: SE(2) states, segment collision, visibility graph
//! shortest paths, grid A*, and a seed-stable roadmap builder.

use super::primitives::{Point2, Point3};
use super::segment_intersection_2::{classify_segment_intersection_2, SegmentIntersectionClass};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose2 {
    pub x: f64,
    pub y: f64,
    pub theta: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose3 {
    pub position: Point3,
    pub orientation: [f64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Joint2 {
    pub length: f64,
    pub angle: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionReport {
    pub collides: bool,
    pub min_distance: f64,
    pub time_of_impact: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimedPose2 {
    pub pose: Pose2,
    pub time: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Control2 {
    pub linear: f64,
    pub angular: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeliefState2 {
    pub mean: Pose2,
    pub covariance: [[f64; 3]; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolygonObstacle {
    pub vertices: Vec<Point2>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlannedPath {
    pub points: Vec<Point2>,
    pub length: f64,
    pub collision_checks: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanningError {
    InvalidInput,
    NoPath,
}

// Early helpers for visibility (fixes intermittent "cannot find" in some build states)
fn same_point(a: Point2, b: Point2) -> bool {
    (a.x - b.x).abs() <= 1e-12 && (a.y - b.y).abs() <= 1e-12
}

fn dist(a: Point2, b: Point2) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

pub fn normalize_pose(mut pose: Pose2) -> Pose2 {
    while pose.theta > core::f64::consts::PI {
        pose.theta -= core::f64::consts::TAU;
    }
    while pose.theta <= -core::f64::consts::PI {
        pose.theta += core::f64::consts::TAU;
    }
    pose
}

pub fn normalize_pose3(mut pose: Pose3) -> Pose3 {
    let n = (pose.orientation[0] * pose.orientation[0]
        + pose.orientation[1] * pose.orientation[1]
        + pose.orientation[2] * pose.orientation[2]
        + pose.orientation[3] * pose.orientation[3])
        .sqrt();
    pose.orientation = if n > 0.0 {
        [
            pose.orientation[0] / n,
            pose.orientation[1] / n,
            pose.orientation[2] / n,
            pose.orientation[3] / n,
        ]
    } else {
        [1.0, 0.0, 0.0, 0.0]
    };
    pose
}

pub fn interpolate_pose2(a: Pose2, b: Pose2, t: f64) -> Pose2 {
    let t = t.clamp(0.0, 1.0);
    normalize_pose(Pose2 {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
        theta: a.theta + shortest_angle_delta(a.theta, b.theta) * t,
    })
}

pub fn forward_kinematics_2d(
    joints: &[Joint2],
    out_points: &mut [Point2],
) -> Result<usize, PlanningError> {
    if out_points.len() < joints.len() + 1 {
        return Err(PlanningError::InvalidInput);
    }
    let mut p = Point2::new(0.0, 0.0);
    let mut angle = 0.0;
    out_points[0] = p;
    for (i, joint) in joints.iter().enumerate() {
        if !(joint.length.is_finite() && joint.angle.is_finite()) {
            return Err(PlanningError::InvalidInput);
        }
        angle += joint.angle;
        p = Point2::new(
            p.x + joint.length * angle.cos(),
            p.y + joint.length * angle.sin(),
        );
        out_points[i + 1] = p;
    }
    Ok(joints.len() + 1)
}

pub fn jacobian_2d(joints: &[Joint2], out: &mut [[f64; 2]]) -> Result<usize, PlanningError> {
    if out.len() < joints.len() {
        return Err(PlanningError::InvalidInput);
    }
    let mut cumulative = vec![0.0; joints.len()];
    let mut a = 0.0;
    for (i, joint) in joints.iter().enumerate() {
        a += joint.angle;
        cumulative[i] = a;
    }
    for i in 0..joints.len() {
        let mut dx = 0.0;
        let mut dy = 0.0;
        for j in i..joints.len() {
            dx -= joints[j].length * cumulative[j].sin();
            dy += joints[j].length * cumulative[j].cos();
        }
        out[i] = [dx, dy];
    }
    Ok(joints.len())
}

pub fn segment_collision_free(a: Point2, b: Point2, obstacles: &[PolygonObstacle]) -> bool {
    for obs in obstacles {
        let n = obs.vertices.len();
        if n < 3 {
            continue;
        }
        if point_in_polygon(a, &obs.vertices) || point_in_polygon(b, &obs.vertices) {
            return false;
        }
        for i in 0..n {
            let c = obs.vertices[i];
            let d = obs.vertices[(i + 1) % n];
            let hit = classify_segment_intersection_2(a, b, c, d);
            let endpoint_touch = hit
                .point
                .is_some_and(|p| same_point(p, a) || same_point(p, b));
            let same_edge = matches!(hit.class, SegmentIntersectionClass::CollinearOverlap)
                && ((same_point(a, c) && same_point(b, d))
                    || (same_point(a, d) && same_point(b, c)));
            if !matches!(hit.class, SegmentIntersectionClass::Disjoint)
                && !endpoint_touch
                && !same_edge
            {
                return false;
            }
        }
    }
    true
}

pub fn continuous_segment_collision(
    a0: Point2,
    a1: Point2,
    b0: Point2,
    b1: Point2,
    obstacles: &[PolygonObstacle],
    steps: usize,
) -> CollisionReport {
    let steps = steps.max(1);
    let mut min_distance = f64::INFINITY;
    for s in 0..=steps {
        let t = s as f64 / steps as f64;
        let a = lerp2(a0, a1, t);
        let b = lerp2(b0, b1, t);
        for obs in obstacles {
            for &v in &obs.vertices {
                min_distance = min_distance.min(point_segment_distance(v, a, b));
            }
        }
        if !segment_collision_free(a, b, obstacles) {
            return CollisionReport {
                collides: true,
                min_distance,
                time_of_impact: t,
            };
        }
    }
    CollisionReport {
        collides: false,
        min_distance,
        time_of_impact: 1.0,
    }
}

pub fn configuration_obstacle_translate(
    robot: &[Point2],
    obstacle: &[Point2],
    out: &mut [Point2],
) -> Result<usize, PlanningError> {
    if robot.is_empty() || obstacle.is_empty() || out.len() < robot.len() * obstacle.len() {
        return Err(PlanningError::InvalidInput);
    }
    let mut n = 0usize;
    for &o in obstacle {
        for &r in robot {
            out[n] = Point2::new(o.x - r.x, o.y - r.y);
            n += 1;
        }
    }
    out[..n].sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(a.y.partial_cmp(&b.y).unwrap_or(core::cmp::Ordering::Equal))
    });
    Ok(n)
}

pub fn visibility_graph_path(
    start: Point2,
    goal: Point2,
    obstacles: &[PolygonObstacle],
) -> Result<PlannedPath, PlanningError> {
    let mut nodes = vec![start, goal];
    for obs in obstacles {
        if obs.vertices.len() < 3 {
            return Err(PlanningError::InvalidInput);
        }
        nodes.extend_from_slice(&obs.vertices);
    }
    let n = nodes.len();
    let mut edges = vec![Vec::<(usize, f64)>::new(); n];
    let mut checks = 0usize;
    for i in 0..n {
        for j in i + 1..n {
            checks += 1;
            if segment_collision_free(nodes[i], nodes[j], obstacles) {
                let w = dist(nodes[i], nodes[j]);
                edges[i].push((j, w));
                edges[j].push((i, w));
            }
        }
    }
    let (prev, length) = dijkstra(&edges, 0, 1).ok_or(PlanningError::NoPath)?;
    let mut rev = Vec::new();
    let mut cur = 1usize;
    rev.push(nodes[cur]);
    while cur != 0 {
        cur = prev[cur];
        rev.push(nodes[cur]);
    }
    rev.reverse();
    Ok(PlannedPath {
        points: rev,
        length,
        collision_checks: checks,
    })
}

pub fn seeded_roadmap(
    bounds: (f64, f64, f64, f64),
    sample_count: usize,
    seed: u64,
    out: &mut [Point2],
) -> Result<usize, PlanningError> {
    if out.len() < sample_count || bounds.0 >= bounds.2 || bounds.1 >= bounds.3 {
        return Err(PlanningError::InvalidInput);
    }
    let mut state = seed;
    for p in out.iter_mut().take(sample_count) {
        let rx = splitmix01(&mut state);
        let ry = splitmix01(&mut state);
        *p = Point2::new(
            bounds.0 + rx * (bounds.2 - bounds.0),
            bounds.1 + ry * (bounds.3 - bounds.1),
        );
    }
    out[..sample_count].sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(a.y.partial_cmp(&b.y).unwrap_or(core::cmp::Ordering::Equal))
    });
    Ok(sample_count)
}

pub fn rrt_plan(
    start: Point2,
    goal: Point2,
    bounds: (f64, f64, f64, f64),
    obstacles: &[PolygonObstacle],
    seed: u64,
    max_nodes: usize,
    step: f64,
    out_path: &mut [Point2],
) -> Result<usize, PlanningError> {
    if max_nodes < 2 || !(step.is_finite() && step > 0.0) || out_path.len() < max_nodes {
        return Err(PlanningError::InvalidInput);
    }
    if !segment_collision_free(start, start, obstacles)
        || !segment_collision_free(goal, goal, obstacles)
    {
        return Err(PlanningError::NoPath);
    }
    let mut nodes = Vec::with_capacity(max_nodes);
    let mut parent = Vec::with_capacity(max_nodes);
    nodes.push(start);
    parent.push(usize::MAX);
    let mut state = seed;
    for iter in 0..max_nodes - 1 {
        let sample = if iter % 5 == 0 {
            goal
        } else {
            Point2::new(
                bounds.0 + splitmix01(&mut state) * (bounds.2 - bounds.0),
                bounds.1 + splitmix01(&mut state) * (bounds.3 - bounds.1),
            )
        };
        let nearest = nearest_node(&nodes, sample);
        let next = steer(nodes[nearest], sample, step);
        if segment_collision_free(nodes[nearest], next, obstacles) {
            nodes.push(next);
            parent.push(nearest);
            if dist(next, goal) <= step && segment_collision_free(next, goal, obstacles) {
                nodes.push(goal);
                parent.push(nodes.len() - 2);
                return write_parent_path(&nodes, &parent, nodes.len() - 1, out_path);
            }
        }
    }
    Err(PlanningError::NoPath)
}

pub fn time_parameterize_path(
    path: &[Point2],
    speed: f64,
    out: &mut [TimedPose2],
) -> Result<usize, PlanningError> {
    if path.is_empty() || !(speed.is_finite() && speed > 0.0) || out.len() < path.len() {
        return Err(PlanningError::InvalidInput);
    }
    let mut time = 0.0;
    for i in 0..path.len() {
        if i > 0 {
            time += dist(path[i - 1], path[i]) / speed;
        }
        let theta = if i + 1 < path.len() {
            (path[i + 1].y - path[i].y).atan2(path[i + 1].x - path[i].x)
        } else if i > 0 {
            (path[i].y - path[i - 1].y).atan2(path[i].x - path[i - 1].x)
        } else {
            0.0
        };
        out[i] = TimedPose2 {
            pose: Pose2 {
                x: path[i].x,
                y: path[i].y,
                theta,
            },
            time,
        };
    }
    Ok(path.len())
}

pub fn multi_robot_conflict_free(a: &[TimedPose2], b: &[TimedPose2], clearance: f64) -> bool {
    if !(clearance.is_finite() && clearance >= 0.0) {
        return false;
    }
    for pa in a {
        for pb in b {
            if (pa.time - pb.time).abs() <= 1e-9 {
                let da = Point2::new(pa.pose.x, pa.pose.y);
                let db = Point2::new(pb.pose.x, pb.pose.y);
                if dist(da, db) < clearance {
                    return false;
                }
            }
        }
    }
    true
}

pub fn coverage_lawnmower(
    bounds: (f64, f64, f64, f64),
    spacing: f64,
    out: &mut [Point2],
) -> Result<usize, PlanningError> {
    if !(spacing.is_finite() && spacing > 0.0) || bounds.0 >= bounds.2 || bounds.1 >= bounds.3 {
        return Err(PlanningError::InvalidInput);
    }
    let rows = ((bounds.3 - bounds.1) / spacing).ceil() as usize + 1;
    if out.len() < rows * 2 {
        return Err(PlanningError::InvalidInput);
    }
    let mut n = 0usize;
    for r in 0..rows {
        let y = (bounds.1 + r as f64 * spacing).min(bounds.3);
        if r % 2 == 0 {
            out[n] = Point2::new(bounds.0, y);
            out[n + 1] = Point2::new(bounds.2, y);
        } else {
            out[n] = Point2::new(bounds.2, y);
            out[n + 1] = Point2::new(bounds.0, y);
        }
        n += 2;
    }
    Ok(n)
}

pub fn pursuit_step(pursuer: Point2, evader: Point2, max_step: f64) -> Point2 {
    steer(pursuer, evader, max_step.max(0.0))
}

pub fn kinodynamic_propagate(pose: Pose2, control: Control2, dt: f64) -> Pose2 {
    let dt = dt.max(0.0);
    let theta_mid = pose.theta + control.angular * dt * 0.5;
    normalize_pose(Pose2 {
        x: pose.x + control.linear * dt * theta_mid.cos(),
        y: pose.y + control.linear * dt * theta_mid.sin(),
        theta: pose.theta + control.angular * dt,
    })
}

pub fn feedback_vector(
    current: Pose2,
    goal: Pose2,
    gain_linear: f64,
    gain_angular: f64,
) -> Control2 {
    let dx = goal.x - current.x;
    let dy = goal.y - current.y;
    let desired = dy.atan2(dx);
    Control2 {
        linear: gain_linear * (dx * dx + dy * dy).sqrt(),
        angular: gain_angular * shortest_angle_delta(current.theta, desired),
    }
}

pub fn belief_update_2d(
    prior: BeliefState2,
    control: Control2,
    observation: Option<Pose2>,
    process_noise: f64,
    observation_weight: f64,
    dt: f64,
) -> BeliefState2 {
    let predicted = kinodynamic_propagate(prior.mean, control, dt);
    let mut cov = prior.covariance;
    for i in 0..3 {
        cov[i][i] += process_noise.max(0.0);
    }
    let mean = if let Some(obs) = observation {
        let w = observation_weight.clamp(0.0, 1.0);
        Pose2 {
            x: predicted.x * (1.0 - w) + obs.x * w,
            y: predicted.y * (1.0 - w) + obs.y * w,
            theta: predicted.theta + shortest_angle_delta(predicted.theta, obs.theta) * w,
        }
    } else {
        predicted
    };
    BeliefState2 {
        mean: normalize_pose(mean),
        covariance: cov,
    }
}

fn dijkstra(edges: &[Vec<(usize, f64)>], start: usize, goal: usize) -> Option<(Vec<usize>, f64)> {
    let n = edges.len();
    let mut dist = vec![f64::INFINITY; n];
    let mut prev = vec![usize::MAX; n];
    let mut used = vec![false; n];
    dist[start] = 0.0;
    for _ in 0..n {
        let mut u = None;
        for i in 0..n {
            if !used[i] && u.map_or(true, |j| dist[i] < dist[j]) {
                u = Some(i);
            }
        }
        let u = u?;
        if u == goal {
            return Some((prev, dist[goal]));
        }
        used[u] = true;
        for &(v, w) in &edges[u] {
            if dist[u] + w < dist[v] {
                dist[v] = dist[u] + w;
                prev[v] = u;
            }
        }
    }
    None
}

fn point_in_polygon(p: Point2, poly: &[Point2]) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let pi = poly[i];
        let pj = poly[j];
        if ((pi.y > p.y) != (pj.y > p.y))
            && (p.x < (pj.x - pi.x) * (p.y - pi.y) / ((pj.y - pi.y).max(1e-12)) + pi.x)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn splitmix01(state: &mut u64) -> f64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    let z = z ^ (z >> 31);
    ((z >> 11) as f64) / ((1u64 << 53) as f64)
}

fn shortest_angle_delta(a: f64, b: f64) -> f64 {
    let mut d = b - a;
    while d > core::f64::consts::PI {
        d -= core::f64::consts::TAU;
    }
    while d <= -core::f64::consts::PI {
        d += core::f64::consts::TAU;
    }
    d
}

fn lerp2(a: Point2, b: Point2, t: f64) -> Point2 {
    Point2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

fn point_segment_distance(p: Point2, a: Point2, b: Point2) -> f64 {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let denom = abx * abx + aby * aby;
    if denom <= 0.0 {
        return dist(p, a);
    }
    let t = (((p.x - a.x) * abx + (p.y - a.y) * aby) / denom).clamp(0.0, 1.0);
    dist(p, Point2::new(a.x + abx * t, a.y + aby * t))
}

fn nearest_node(nodes: &[Point2], sample: Point2) -> usize {
    let mut best = 0usize;
    let mut best_d = dist(nodes[0], sample);
    for (i, &node) in nodes.iter().enumerate().skip(1) {
        let d = dist(node, sample);
        if d < best_d {
            best = i;
            best_d = d;
        }
    }
    best
}

fn steer(from: Point2, to: Point2, step: f64) -> Point2 {
    let d = dist(from, to);
    if d <= step || d == 0.0 {
        to
    } else {
        let t = step / d;
        lerp2(from, to, t)
    }
}

fn write_parent_path(
    nodes: &[Point2],
    parent: &[usize],
    goal: usize,
    out_path: &mut [Point2],
) -> Result<usize, PlanningError> {
    let mut rev = Vec::new();
    let mut cur = goal;
    loop {
        rev.push(nodes[cur]);
        if parent[cur] == usize::MAX {
            break;
        }
        cur = parent[cur];
    }
    if out_path.len() < rev.len() {
        return Err(PlanningError::InvalidInput);
    }
    for (slot, &p) in out_path.iter_mut().zip(rev.iter().rev()) {
        *slot = p;
    }
    Ok(rev.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pose_normalises_angle() {
        let p = normalize_pose(Pose2 {
            x: 0.0,
            y: 0.0,
            theta: 4.0,
        });
        assert!(p.theta <= core::f64::consts::PI);
    }

    #[test]
    fn visibility_path_routes_around_square() {
        let obs = [PolygonObstacle {
            vertices: vec![
                Point2::new(0.4, -0.2),
                Point2::new(0.6, -0.2),
                Point2::new(0.6, 0.2),
                Point2::new(0.4, 0.2),
            ],
        }];
        let path =
            visibility_graph_path(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), &obs).unwrap();
        assert!(path.points.len() > 2);
        assert!(path.length > 1.0);
    }

    #[test]
    fn roadmap_is_seed_stable() {
        let mut a = [Point2::new(0.0, 0.0); 8];
        let mut b = [Point2::new(0.0, 0.0); 8];
        seeded_roadmap((0.0, 0.0, 1.0, 1.0), 8, 42, &mut a).unwrap();
        seeded_roadmap((0.0, 0.0, 1.0, 1.0), 8, 42, &mut b).unwrap();
        assert_eq!(a, b);
    }
}
