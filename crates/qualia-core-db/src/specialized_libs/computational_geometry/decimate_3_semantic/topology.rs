//! Allocation-free strict surface admission. Repeated full scans are charged and bounded.
use super::super::face_plane;
use super::*;
use crate::specialized_libs::computational_geometry::{GeometryKernel, Point3};

fn invalid(field: &'static str) -> AdmissionError {
    error(ReasonCode::InvalidInput, field)
}
fn sorted(mut t: [u32; 3]) -> [u32; 3] {
    t.sort_unstable();
    t
}

pub(super) fn incidence(
    s: &[AdmissionSlot],
    m: usize,
    a: u32,
    b: u32,
    budget: &mut ProductionBudget<'_>,
) -> Result<(usize, i32), AdmissionError> {
    let mut count = 0;
    let mut direction = 0;
    for slot in &s[..m] {
        budget.checkpoint(1)?;
        if !slot.face_alive {
            continue;
        }
        let t = slot.triangle;
        for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            if u == a && v == b {
                count += 1;
                direction += 1;
            }
            if u == b && v == a {
                count += 1;
                direction -= 1;
            }
        }
    }
    Ok((count, direction))
}

fn connected_link(
    s: &mut [AdmissionSlot],
    m: usize,
    v: u32,
    budget: &mut ProductionBudget<'_>,
) -> Result<(), AdmissionError> {
    let mut first = None;
    let mut total = 0;
    for (f, slot) in s[..m].iter_mut().enumerate() {
        budget.checkpoint(1)?;
        slot.mark = false;
        if slot.face_alive && slot.triangle.contains(&v) {
            total += 1;
            if first.is_none() {
                first = Some(f);
            }
        }
    }
    let Some(first) = first else {
        return Err(invalid("isolated_vertex"));
    };
    s[first].mark = true;
    let mut reached = 1;
    loop {
        let mut changed = false;
        for f in 0..m {
            budget.checkpoint(1)?;
            if !s[f].face_alive || s[f].mark || !s[f].triangle.contains(&v) {
                continue;
            }
            for g in 0..m {
                budget.checkpoint(1)?;
                if !s[g].face_alive || !s[g].mark {
                    continue;
                }
                if s[f].triangle.iter().any(|&w| w != v && s[g].triangle.contains(&w)) {
                    s[f].mark = true;
                    reached += 1;
                    changed = true;
                    break;
                }
            }
        }
        if !changed {
            break;
        }
    }
    if reached != total {
        return Err(invalid("disconnected_vertex_link"));
    }
    Ok(())
}

pub(super) fn validate_input(
    input: &AdmissionInput<'_>,
    s: &mut [AdmissionSlot],
    budget: &mut ProductionBudget<'_>,
) -> Result<(), AdmissionError> {
    let n = input.vertices.len();
    let m = input.triangles.len();
    for slot in &mut s[..n.max(m)] {
        budget.checkpoint(1)?;
        *slot = AdmissionSlot::empty();
    }
    for v in 0..n {
        budget.checkpoint(1)?;
        let p = input.vertices[v];
        let a = input.attributes[v];
        if ![p.x, p.y, p.z, a.normal[0], a.normal[1], a.normal[2], a.uv[0], a.uv[1], a.bifurcation_sensitivity]
            .iter()
            .all(|x| x.is_finite())
            || !(0.0..=1.0).contains(&a.bifurcation_sensitivity)
            || !(-1..=1).contains(&input.domains[v].tangent_handedness)
        {
            return Err(invalid("vertex_attributes"));
        }
        for w in 0..v {
            budget.checkpoint(1)?;
            if input.stable_vertex_ids[v] == input.stable_vertex_ids[w] {
                return Err(invalid("duplicate_stable_vertex_id"));
            }
        }
        s[v].position = p;
        s[v].attributes = a;
        s[v].vertex_alive = true;
        s[v].lineage = v as u32;
    }
    for f in 0..m {
        budget.checkpoint(1)?;
        let t = input.triangles[f];
        if t.iter().any(|&v| v as usize >= n) || t[0] == t[1] || t[1] == t[2] || t[2] == t[0] {
            return Err(invalid("triangle_indices"));
        }
        if face_plane(s[t[0] as usize].position, s[t[1] as usize].position, s[t[2] as usize].position).is_none() {
            return Err(invalid("degenerate_triangle"));
        }
        for g in 0..f {
            budget.checkpoint(1)?;
            if sorted(t) == sorted(input.triangles[g]) {
                return Err(invalid("duplicate_triangle"));
            }
        }
        s[f].triangle = t;
        s[f].face_alive = true;
    }
    for f in 0..m {
        let t = s[f].triangle;
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (count, direction) = incidence(s, m, a, b, budget)?;
            if count > 2 || (count == 2 && direction != 0) {
                return Err(invalid("edge_incidence_or_orientation"));
            }
        }
    }
    for v in 0..n {
        connected_link(s, m, v as u32, budget)?;
    }
    Ok(())
}

pub(super) fn boundary_vertex(
    s: &[AdmissionSlot],
    m: usize,
    v: u32,
    budget: &mut ProductionBudget<'_>,
) -> Result<bool, AdmissionError> {
    for slot in &s[..m] {
        budget.checkpoint(1)?;
        if !slot.face_alive || !slot.triangle.contains(&v) {
            continue;
        }
        for &w in &slot.triangle {
            if w != v && incidence(s, m, v, w, budget)?.0 == 1 {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
fn neighbors(
    s: &[AdmissionSlot],
    m: usize,
    a: u32,
    b: u32,
    budget: &mut ProductionBudget<'_>,
) -> Result<bool, AdmissionError> {
    for slot in &s[..m] {
        budget.checkpoint(1)?;
        if slot.face_alive && slot.triangle.contains(&a) && slot.triangle.contains(&b) {
            return Ok(true);
        }
    }
    Ok(false)
}
pub(super) fn after(t: [u32; 3], lo: u32, hi: u32) -> Option<[u32; 3]> {
    if t.contains(&lo) && t.contains(&hi) {
        None
    } else {
        Some(t.map(|v| if v == lo { hi } else { v }))
    }
}

pub(super) fn legal<K: GeometryKernel>(
    kernel: &K,
    s: &[AdmissionSlot],
    n: usize,
    m: usize,
    lo: u32,
    hi: u32,
    target: Point3,
    preserve_boundary: bool,
    length_scale: f64,
    budget: &mut ProductionBudget<'_>,
) -> Result<bool, AdmissionError> {
    let count = incidence(s, m, lo, hi, budget)?.0;
    if count == 0 || count > 2 {
        return Ok(false);
    }
    let blo = boundary_vertex(s, m, lo, budget)?;
    let bhi = boundary_vertex(s, m, hi, budget)?;
    if (preserve_boundary && (blo || bhi)) || (blo && bhi && count != 1) {
        return Ok(false);
    }
    // Link(edge)=Link(lo) intersection Link(hi), without allocating neighbor sets.
    for w in 0..n as u32 {
        if w == lo || w == hi || !s[w as usize].vertex_alive {
            continue;
        }
        if neighbors(s, m, lo, w, budget)? && neighbors(s, m, hi, w, budget)? {
            let mut apex = false;
            for slot in &s[..m] {
                budget.checkpoint(1)?;
                if slot.face_alive
                    && slot.triangle.contains(&lo)
                    && slot.triangle.contains(&hi)
                    && slot.triangle.contains(&w)
                {
                    apex = true;
                    break;
                }
            }
            if !apex {
                return Ok(false);
            }
        }
    }
    for f in 0..m {
        budget.checkpoint(1)?;
        if !s[f].face_alive {
            continue;
        }
        let Some(t) = after(s[f].triangle, lo, hi) else {
            continue;
        };
        for g in 0..f {
            budget.checkpoint(1)?;
            if !s[g].face_alive {
                continue;
            }
            if after(s[g].triangle, lo, hi).is_some_and(|u| sorted(u) == sorted(t)) {
                return Ok(false);
            }
        }
        if !s[f].triangle.contains(&lo) && !s[f].triangle.contains(&hi) {
            continue;
        }
        let old = s[f].triangle.map(|v| s[v as usize].position);
        let new = s[f].triangle.map(|v| if v == lo || v == hi { target } else { s[v as usize].position });
        let Some((nx, ny, nz, _, area)) = face_plane(old[0], old[1], old[2]) else {
            return Ok(false);
        };
        let Some((mx, my, mz, _, _)) = face_plane(new[0], new[1], new[2]) else {
            return Ok(false);
        };
        if nx * mx + ny * my + nz * mz <= 0.0 {
            return Ok(false);
        }
        let extent = area.sqrt().max(length_scale * 1e-12);
        let apex = Point3::new(
            (old[0].x + old[1].x + old[2].x) / 3.0 + nx * extent,
            (old[0].y + old[1].y + old[2].y) / 3.0 + ny * extent,
            (old[0].z + old[1].z + old[2].z) / 3.0 + nz * extent,
        );
        let old_sign = kernel.orient_3d(old[0], old[1], old[2], apex);
        let new_sign = kernel.orient_3d(new[0], new[1], new[2], apex);
        if old_sign == crate::specialized_libs::computational_geometry::expansion::Sign::Zero || old_sign != new_sign {
            return Ok(false);
        }
    }
    // Preserve a 2-dimensional component; never silently discard surviving orphan vertices.
    for v in 0..n as u32 {
        if v == lo || !s[v as usize].vertex_alive {
            continue;
        }
        let mut referenced = false;
        for slot in &s[..m] {
            budget.checkpoint(1)?;
            if slot.face_alive && after(slot.triangle, lo, hi).is_some_and(|t| t.contains(&v)) {
                referenced = true;
                break;
            }
        }
        if !referenced {
            return Ok(false);
        }
    }
    Ok(true)
}
