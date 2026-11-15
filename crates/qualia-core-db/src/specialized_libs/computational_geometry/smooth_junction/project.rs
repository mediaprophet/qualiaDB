use super::super::{ddg_operators, tapered_branch_field::tapered_branch_field_into};
use super::*;

fn evaluate(
    p: Point3,
    segments: &[TaperedFieldSegment],
    controls: Controls,
    meter: &mut Meter,
) -> Result<f64, Error> {
    meter.charge(segments.len())?;
    let mut value = [0.];
    tapered_branch_field_into(
        &[p],
        segments,
        controls.blend,
        1e-9,
        controls.tolerance,
        1,
        segments.len(),
        segments.len(),
        &mut value,
    )
    .map_err(|_| Error::Field)?;
    Ok(value[0])
}
fn gradient(
    p: Point3,
    segments: &[TaperedFieldSegment],
    c: Controls,
    meter: &mut Meter,
) -> Result<Point3, Error> {
    let mut g = [0.; 3];
    for axis in 0..3 {
        let mut delta = [0.; 3];
        delta[axis] = c.gradient_step;
        let d = Point3::new(delta[0], delta[1], delta[2]);
        g[axis] = (evaluate(add(p, d), segments, c, meter)?
            - evaluate(sub(p, d), segments, c, meter)?)
            / (2. * c.gradient_step);
    }
    let g = Point3::new(g[0], g[1], g[2]);
    if !finite(g) || dot(g, g) <= 1e-20 {
        return Err(Error::Normal);
    }
    Ok(g)
}
fn target(
    start: Point3,
    segments: &[TaperedFieldSegment],
    c: Controls,
    meter: &mut Meter,
    cancelled: &dyn Fn() -> bool,
) -> Result<Point3, Error> {
    let mut p = start;
    for _ in 0..c.iterations {
        check_cancel(cancelled)?;
        let f = evaluate(p, segments, c, meter)?;
        if f.abs() <= c.tolerance {
            return Ok(p);
        }
        let g = gradient(p, segments, c, meter)?;
        let mut step = scale(g, f / dot(g, g));
        let length = length(step);
        if !length.is_finite() {
            return Err(Error::Field);
        }
        if length > c.max_step {
            step = scale(step, c.max_step / length)
        }
        p = sub(p, step);
        if !finite(p) || length_between(p, start) > c.max_displacement {
            return Err(Error::Displacement);
        }
    }
    if evaluate(p, segments, c, meter)?.abs() <= c.tolerance {
        Ok(p)
    } else {
        Err(Error::Convergence)
    }
}
fn length_between(a: Point3, b: Point3) -> f64 {
    length(sub(a, b))
}
fn valid(c: Controls) -> bool {
    [
        c.blend,
        c.tolerance,
        c.gradient_step,
        c.max_step,
        c.max_displacement,
        c.max_dihedral,
        c.min_normal_alignment,
        c.min_area_sq,
    ]
    .iter()
    .all(|v| v.is_finite())
        && c.blend > 0.
        && c.blend <= 100.
        && c.tolerance > 0.
        && c.gradient_step > 0.
        && c.gradient_step <= c.blend
        && c.max_step > 0.
        && c.max_displacement > 0.
        && (1..=128).contains(&c.iterations)
        && c.refinements <= 6
        && c.max_dihedral > 0.
        && c.max_dihedral < std::f64::consts::PI
        && c.min_normal_alignment > 0.
        && c.min_normal_alignment <= 1.
        && c.min_area_sq > 0.
}
pub fn validate_controls(c: Controls) -> Result<(), Error> {
    if valid(c) {
        Ok(())
    } else {
        Err(Error::Invalid)
    }
}
/// Caller outputs stay untouched on every error; all construction/projection is
/// detached bounded scratch. Cancellation and work checks precede expensive work.
pub fn smooth_into(
    input: Input<'_>,
    c: Controls,
    limits: Limits,
    out: Output<'_>,
    cancelled: &dyn Fn() -> bool,
) -> Result<Receipt, Error> {
    let bytes = storage_bytes(limits)?;
    if bytes > limits.bytes {
        return Err(Error::Bytes);
    }
    if !valid(c)
        || input.positions.len() < 4
        || input.triangles.len() < 4
        || input.sources.len() != input.triangles.len()
        || input.positions.len() > limits.vertices
        || input.triangles.len() > limits.faces
        || limits.vertices > u32::MAX as usize
        || input.segments.len() < 2
        || input.segments.len() > 256
        || input.fade_coordinates.len() != input.positions.len()
        || out.positions.len() < limits.vertices
        || out.references.len() < limits.vertices
        || out.normals.len() < limits.vertices
        || out.weights.len() < limits.vertices
        || out.triangles.len() < limits.faces
        || out.sources.len() < limits.faces
    {
        return Err(Error::Invalid);
    }
    let mut meter = Meter {
        initial: limits.work,
        remaining: limits.work,
    };
    meter.charge(
        mul(limits.vertices, 8)?
            .checked_add(mul(limits.faces, 8)?)
            .ok_or(Error::Overflow)?,
    )?;
    check_cancel(cancelled)?;
    if input.positions.iter().any(|&p| !finite(p))
        || input
            .triangles
            .iter()
            .flatten()
            .any(|&i| i as usize >= input.positions.len())
        || input
            .fade_coordinates
            .iter()
            .any(|d| !d.is_finite() || d.abs() > 1e6)
    {
        return Err(Error::Invalid);
    }
    // Force canonical field preflight even if every seed vertex is pinned.
    evaluate(input.positions[0], input.segments, c, &mut meter)?;
    let mut positions = capacity(limits.vertices)?;
    positions.extend_from_slice(input.positions);
    let mut triangles = capacity(limits.faces)?;
    triangles.extend_from_slice(input.triangles);
    let mut sources = capacity(limits.faces)?;
    sources.extend_from_slice(input.sources);
    let mut coordinates = capacity(limits.vertices)?;
    coordinates.extend_from_slice(input.fade_coordinates);
    let edges = topology_edges(&positions, &triangles, &mut meter)?;
    meter.charge(mul(edges.len(), 4)?)?;
    // Each actual union branch-face interface, not merely selected landmarks.
    // Source fade coordinates interpolate affinely on each seed face: both
    // endpoints >=1 keep the complete interface edge in full support.
    for (i, e) in edges.iter().enumerate() {
        let j = e.twin as usize;
        if i < j && sources[i / 3].piece != sources[j / 3].piece {
            let a = coordinates[e.origin as usize];
            let b = coordinates[edges[e.next as usize].origin as usize];
            if a < 1. || b < 1. {
                return Err(Error::BoundaryContact);
            }
        }
    }
    drop(edges);
    for _ in 0..c.refinements {
        refine::refine(
            &mut positions,
            &mut triangles,
            &mut sources,
            &mut coordinates,
            limits,
            &mut meter,
            cancelled,
        )?;
    }
    let mut references = capacity(positions.len())?;
    references.extend_from_slice(&positions);
    let mut weights = capacity(positions.len())?;
    meter.charge(mul(positions.len(), 4)?)?;
    let mut moved = 0usize;
    let mut max_residual = 0f64;
    for (i, p) in positions.iter_mut().enumerate() {
        check_cancel(cancelled)?;
        let w = weight(coordinates[i]);
        weights.push(w);
        if w == 0. {
            continue;
        }
        let projected = target(*p, input.segments, c, &mut meter, cancelled)?;
        *p = add(*p, scale(sub(projected, *p), w));
        if *p != references[i] {
            moved += 1
        }
        if w == 1. {
            let residual = evaluate(*p, input.segments, c, &mut meter)?.abs();
            max_residual = max_residual.max(residual);
            if residual > c.tolerance {
                return Err(Error::Convergence);
            }
        }
    }
    if moved == 0 {
        return Err(Error::Convergence);
    } // No normals-only success.
    let edges = topology_edges(&positions, &triangles, &mut meter)?;
    let zero = Point3::new(0., 0., 0.);
    let mut faces = capacity(triangles.len())?;
    faces.resize(triangles.len(), zero);
    let mut normals = capacity(positions.len())?;
    normals.resize(positions.len(), zero);
    meter.charge(
        mul(triangles.len(), 32)?
            .checked_add(mul(positions.len(), 16)?)
            .ok_or(Error::Overflow)?,
    )?;
    ddg_operators::face_vector_areas(&positions, &triangles, &mut faces)
        .map_err(|_| Error::Normal)?;
    ddg_operators::vertex_normals(&positions, &triangles, &mut normals)
        .map_err(|_| Error::Normal)?;
    for (i, t) in triangles.iter().enumerate() {
        let original = normal(
            references[t[0] as usize],
            references[t[1] as usize],
            references[t[2] as usize],
        );
        let current = normal(
            positions[t[0] as usize],
            positions[t[1] as usize],
            positions[t[2] as usize],
        );
        if dot(current, current) <= c.min_area_sq
            || !finite(current)
            || dot(current, original) <= 0.
        {
            return Err(Error::Orientation);
        }
        faces[i] = unit(current)?;
    }
    let mut worst_dihedral = 0f64;
    let mut worst_alignment = 1f64;
    for (i, e) in edges.iter().enumerate() {
        let j = e.twin as usize;
        if i >= j {
            continue;
        }
        let a = e.origin as usize;
        let b = edges[e.next as usize].origin as usize;
        if weights[a] == 1. && weights[b] == 1. {
            let angle = dot(faces[i / 3], faces[j / 3]).clamp(-1., 1.).acos();
            worst_dihedral = worst_dihedral.max(angle);
            if angle > c.max_dihedral {
                return Err(Error::Dihedral);
            }
        }
    }
    for i in 0..positions.len() {
        check_cancel(cancelled)?;
        normals[i] = unit(normals[i])?;
        if weights[i] == 1. {
            let g = unit(gradient(positions[i], input.segments, c, &mut meter)?)?;
            let alignment = dot(normals[i], g);
            worst_alignment = worst_alignment.min(alignment);
            if alignment < c.min_normal_alignment {
                return Err(Error::Normal);
            }
        }
        if weights[i] == 0. && positions[i] != references[i] {
            return Err(Error::BoundaryContact);
        }
    }
    check_cancel(cancelled)?;
    out.positions[..positions.len()].copy_from_slice(&positions);
    out.references[..positions.len()].copy_from_slice(&references);
    out.normals[..positions.len()].copy_from_slice(&normals);
    out.weights[..positions.len()].copy_from_slice(&weights);
    out.triangles[..triangles.len()].copy_from_slice(&triangles);
    out.sources[..sources.len()].copy_from_slice(&sources);
    Ok(Receipt {
        vertices: positions.len(),
        faces: triangles.len(),
        work_used: meter.initial - meter.remaining,
        logical_bytes: bytes,
        moved_vertices: moved,
        max_field_residual: max_residual,
        max_dihedral: worst_dihedral,
        min_normal_alignment: worst_alignment,
    })
}
