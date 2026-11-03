//! Bounded facet constraint recovery over the existing Core Delaunay owner.
//! No Steiner point is introduced: both sides retain shared registry IDs.
use super::super::super::{
    delaunay_triangulation_2, orientation_2, Orientation, Point2, Point3, Sign,
};
use super::exact_registry::{self as exact, Registry, Scratch};
use super::{charge, Cut, Error};
const MAX_POINTS: usize = 256;
fn axis(a: Point3, b: Point3) -> usize {
    let d = [(a.x - b.x).abs(), (a.y - b.y).abs(), (a.z - b.z).abs()];
    (0..3).max_by(|&a, &b| d[a].total_cmp(&d[b])).unwrap()
}
fn coord(p: Point3, a: usize) -> f64 {
    [p.x, p.y, p.z][a]
}
fn chain(
    ids: &mut Vec<u32>,
    a: usize,
    r: &Registry,
    edges: &mut Vec<(u32, u32)>,
    work: &mut usize,
    arithmetic: &mut Scratch<'_>,
) -> Result<(), Error> {
    charge(
        work,
        ids.len().checked_mul(ids.len()).ok_or(Error::Overflow)?,
    )?;
    ids.sort_by(|&a0, &b0| {
        coord(r.nodes[a0 as usize].p, a).total_cmp(&coord(r.nodes[b0 as usize].p, a))
    });
    ids.dedup();
    for pair in ids.windows(2) {
        if exact::cmp_axis(
            &r.nodes[pair[0] as usize],
            &r.nodes[pair[1] as usize],
            a,
            work,
            arithmetic,
        )? != Sign::Negative
        {
            return Err(Error::RoundedCollision);
        }
        if edges.len() >= 2048 {
            return Err(Error::Constraints);
        }
        edges.push((pair[0], pair[1]));
    }
    Ok(())
}
fn orientation(p: &[Point2], a: u32, b: u32, c: u32) -> Orientation {
    orientation_2(p[a as usize], p[b as usize], p[c as usize])
}
fn crosses(p: &[Point2], a: u32, b: u32, c: u32, d: u32) -> bool {
    if a == c || a == d || b == c || b == d {
        return false;
    }
    let u = orientation(p, a, b, c);
    let v = orientation(p, a, b, d);
    let x = orientation(p, c, d, a);
    let y = orientation(p, c, d, b);
    u != Orientation::Collinear
        && v != Orientation::Collinear
        && u != v
        && x != Orientation::Collinear
        && y != Orientation::Collinear
        && x != y
}
fn has(t: &[[u32; 3]], a: u32, b: u32) -> bool {
    t.iter().any(|f| {
        (0..3).any(|i| f[i] == a && f[(i + 1) % 3] == b || f[i] == b && f[(i + 1) % 3] == a)
    })
}
fn ccw(p: &[Point2], mut t: [u32; 3]) -> Result<[u32; 3], Error> {
    match orientation(p, t[0], t[1], t[2]) {
        Orientation::CounterClockwise => {}
        Orientation::Clockwise => t.swap(1, 2),
        Orientation::Collinear => {
            return Err(Error::FacetChild {
                stage: "constraint flip collinear child",
                points: t.map(|i| p[i as usize]),
            })
        }
    }
    Ok(t)
}
fn recover(
    p: &[Point2],
    t: &mut [[u32; 3]],
    constraints: &[(u32, u32)],
    work: &mut usize,
    cancel: &dyn Fn() -> bool,
) -> Result<(), Error> {
    for (done, &(a, b)) in constraints.iter().enumerate() {
        let mut attempts = 0usize;
        loop {
            if cancel() {
                return Err(Error::Cancelled);
            }
            charge(work, t.len().checked_mul(3).ok_or(Error::Overflow)?)?;
            if has(t, a, b) {
                break;
            }
            attempts += 1;
            if attempts > p.len() * p.len() {
                return Err(Error::Constraints);
            }
            let mut flip = None;
            'faces: for i in 0..t.len() {
                for e in 0..3 {
                    charge(work, 16)?;
                    let u = t[i][e];
                    let v = t[i][(e + 1) % 3];
                    let x = t[i][(e + 2) % 3];
                    if !crosses(p, a, b, u, v) {
                        continue;
                    }
                    charge(work, done)?;
                    if constraints[..done]
                        .iter()
                        .any(|&(c, d)| (c == u && d == v) || (c == v && d == u))
                    {
                        return Err(Error::Constraints);
                    }
                    charge(work, t.len().checked_mul(3).ok_or(Error::Overflow)?)?;
                    for j in i + 1..t.len() {
                        if let Some(f) = (0..3).find(|&f| t[j][f] == v && t[j][(f + 1) % 3] == u) {
                            let y = t[j][(f + 2) % 3];
                            // Recover a constraint without adding unshared points.
                            // Only convex flips that remove a crossing are admitted.
                            if !crosses(p, u, v, x, y) || crosses(p, a, b, x, y) {
                                continue;
                            }
                            charge(work, done.checked_mul(16).ok_or(Error::Overflow)?)?;
                            if constraints[..done]
                                .iter()
                                .any(|&(c, d)| crosses(p, x, y, c, d))
                            {
                                continue;
                            }
                            flip = Some((i, j, ccw(p, [x, y, v])?, ccw(p, [y, x, u])?));
                            break 'faces;
                        }
                    }
                }
            }
            let Some((i, j, x, y)) = flip else {
                return Err(Error::Constraints);
            };
            t[i] = x;
            t[j] = y;
        }
    }
    Ok(())
}
pub(super) fn triangulate(
    face: usize,
    triangle: [Point3; 3],
    r: &Registry,
    cuts: &[Cut],
    all_faces: &[[Point3; 3]],
    work: &mut usize,
    cancel: &dyn Fn() -> bool,
    arithmetic: &mut Scratch<'_>,
) -> Result<Vec<[u32; 3]>, Error> {
    let checked_triangle = exact::triangle(triangle, work, arithmetic)?;
    let mut ids = Vec::new();
    for (i, node) in r.nodes.iter().enumerate() {
        if cancel() {
            return Err(Error::Cancelled);
        }
        let p = node.p;
        charge(work, 1)?;
        if [p.x, p.y, p.z].iter().enumerate().any(|(a, &v)| {
            v < triangle
                .iter()
                .map(|p| coord(*p, a))
                .fold(f64::INFINITY, f64::min)
                - 1e-12
                || v > triangle
                    .iter()
                    .map(|p| coord(*p, a))
                    .fold(f64::NEG_INFINITY, f64::max)
                    + 1e-12
        }) {
            continue;
        }
        if exact::inside_on(node, &checked_triangle, work, arithmetic)? {
            if ids.len() >= MAX_POINTS {
                return Err(Error::Points);
            }
            ids.push(i as u32);
        }
    }
    charge(work, ids.len().checked_mul(3).ok_or(Error::Overflow)?)?;
    let corner_ids = triangle.map(|corner| {
        ids.iter()
            .copied()
            .find(|&id| r.nodes[id as usize].is_original_point(corner))
    });
    if ids.len() < 3 || corner_ids.iter().any(Option::is_none) {
        return Err(Error::FacetIncidence {
            admitted: ids.len(),
            corners: corner_ids.map(|id| id.is_some()),
        });
    }
    let corner_ids = corner_ids.map(Option::unwrap);
    let normal = super::super::tri_normal(triangle[0], triangle[1], triangle[2]);
    let drop = checked_triangle.drop_axis();
    let project = |p: Point3| match drop {
        0 => Point2::new(p.y, p.z),
        1 => Point2::new(p.x, p.z),
        _ => Point2::new(p.x, p.y),
    };
    let points: Vec<_> = ids
        .iter()
        .map(|&i| project(r.nodes[i as usize].p))
        .collect();
    charge(work, points.len() * points.len())?;
    for i in 0..points.len() {
        if points[..i].iter().any(|p| *p == points[i]) {
            return Err(Error::RoundedCollision);
        }
    }
    let mut edges = Vec::new();
    for side in 0..3 {
        let a = triangle[side];
        let b = triangle[(side + 1) % 3];
        let mut line = Vec::new();
        for &i in &ids {
            if exact::on_edge(&r.nodes[i as usize], &checked_triangle, side, work, arithmetic)? {
                line.push(i);
            }
        }
        charge(work, line.len().checked_mul(2).ok_or(Error::Overflow)?)?;
        if !line.contains(&corner_ids[side]) || !line.contains(&corner_ids[(side + 1) % 3]) {
            return Err(Error::Incidence);
        }
        chain(&mut line, axis(a, b), r, &mut edges, work, arithmetic)?;
    }
    charge(work, cuts.len().checked_mul(2).ok_or(Error::Overflow)?)?;
    for cut in cuts.iter().filter(|c| c.faces.contains(&face)) {
        let other = if cut.faces[0] == face {
            cut.faces[1]
        } else {
            cut.faces[0]
        };
        let a = cut.nodes[0];
        let b = cut.nodes[1];
        let other_triangle = exact::triangle(all_faces[other], work, arithmetic)?;
        let axis = axis(r.nodes[a as usize].p, r.nodes[b as usize].p);
        let (low, high) = if coord(r.nodes[a as usize].p, axis) < coord(r.nodes[b as usize].p, axis)
        {
            (a, b)
        } else {
            (b, a)
        };
        let mut line = Vec::new();
        for &id in &ids {
            if other_triangle.plane.orient(
                &r.nodes[id as usize].exact(work)?, &mut arithmetic.checked()?, work,
            ).map_err(exact::construction_error)? != Sign::Zero {
                continue;
            }
            if exact::cmp_axis(
                &r.nodes[id as usize],
                &r.nodes[low as usize],
                axis,
                work,
                arithmetic,
            )? == Sign::Negative
                || exact::cmp_axis(
                    &r.nodes[id as usize],
                    &r.nodes[high as usize],
                    axis,
                    work,
                    arithmetic,
                )? == Sign::Positive
            {
                continue;
            }
            line.push(id);
        }
        charge(work, line.len().checked_mul(2).ok_or(Error::Overflow)?)?;
        if !line.contains(&a) || !line.contains(&b) {
            return Err(Error::Incidence);
        }
        chain(&mut line, axis, r, &mut edges, work, arithmetic)?;
    }
    let mut constraints = Vec::new();
    for (a, b) in edges {
        charge(
            work,
            ids.len()
                .checked_mul(2)
                .and_then(|n| n.checked_add(constraints.len()))
                .ok_or(Error::Overflow)?,
        )?;
        let a = ids.iter().position(|&v| v == a).ok_or(Error::Incidence)? as u32;
        let b = ids.iter().position(|&v| v == b).ok_or(Error::Incidence)? as u32;
        let edge = (a.min(b), a.max(b));
        if !constraints.contains(&edge) {
            constraints.push(edge);
        }
    }
    let n = points.len();
    // Existing Bowyer-Watson owner: <=2n+1 planar triangles; its cavity
    // vectors are O(n), and sorting/visits are conservatively O(n^3).
    charge(
        work,
        n.checked_mul(n)
            .and_then(|v| v.checked_mul(n))
            .and_then(|v| v.checked_mul(64))
            .ok_or(Error::Overflow)?,
    )?;
    let mut scratch = vec![0; n];
    let mut triangles = vec![[0; 3]; 2 * n + 1];
    let count = delaunay_triangulation_2(&points, &mut scratch, &mut triangles)
        .map_err(|reason| Error::FacetDelaunay { points: n, reason })?;
    triangles.truncate(count);
    recover(&points, &mut triangles, &constraints, work, cancel)?;
    for &(a, b) in &constraints {
        charge(work, triangles.len() * 3)?;
        if !has(&triangles, a, b) {
            return Err(Error::Constraints);
        }
    }
    for t in &mut triangles {
        *t = t.map(|i| ids[i as usize]);
        let p = t.map(|i| r.nodes[i as usize].p);
        let n = super::super::tri_normal(p[0], p[1], p[2]);
        let alignment = n.x * normal.x + n.y * normal.y + n.z * normal.z;
        if alignment == 0. || !alignment.is_finite() {
            return Err(Error::FacetAlignment { child: p });
        }
        if alignment < 0. {
            t.swap(1, 2);
        }
    }
    Ok(triangles)
}
