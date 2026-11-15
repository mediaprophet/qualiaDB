//! Bounded cold union adapter around the existing exact-intersection, splitting,
//! classification and assembly conventions. This does NOT certify manifoldness.
//! Caller buffers are unpublished scratch: any error invalidates all output.
//! No successful counts are returned on exhaustion. Legacy APIs are unchanged.
use super::super::exact_construct_3::ExactPoint3;
use super::*;
#[path = "bounded_tests.rs"]
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceFace {
    pub mesh: u8,
    pub face: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundedUnionError {
    InvalidInput,
    Overflow,
    Bytes,
    Work,
    Fragments,
    Output,
    Cancelled,
}
#[derive(Clone, Copy, Debug)]
pub struct UnionBudget {
    pub bytes: usize,
    pub work: usize,
}
pub struct UnionBuffers<'a> {
    pub corners_a: &'a mut [[Point3; 3]],
    pub corners_b: &'a mut [[Point3; 3]],
    pub current: &'a mut [[Point3; 3]],
    pub next: &'a mut [[Point3; 3]],
    pub vertices: &'a mut [Point3],
    pub triangles: &'a mut [[u32; 3]],
    pub source_faces: &'a mut [SourceFace],
}
#[derive(Clone, Copy, Debug)]
pub struct UnionReceipt {
    pub vertices: usize,
    pub triangles: usize,
    pub work: usize,
    pub logical_bytes: usize,
}

fn charge(work: &mut usize, n: usize) -> Result<(), BoundedUnionError> {
    *work = work.checked_sub(n).ok_or(BoundedUnionError::Work)?;
    Ok(())
}
fn mul(a: usize, b: usize) -> Result<usize, BoundedUnionError> {
    a.checked_mul(b).ok_or(BoundedUnionError::Overflow)
}
/// Logical buffer/primitive heap envelope, not allocator metadata or peak stack.
/// Exact triangle intersection collects at most 15 points (9 edge crossings +
/// 6 contained corners), capacity <=24, plus <=15 rounded points. Its exact
/// coordinates use fixed arrays. split_one returns at most 3 triangles.
pub fn union_buffer_bytes(
    a: usize,
    b: usize,
    fragments: usize,
    vertices: usize,
    triangles: usize,
) -> Result<usize, BoundedUnionError> {
    let corners = a
        .checked_add(b)
        .and_then(|n| fragments.checked_mul(2).and_then(|f| n.checked_add(f)))
        .ok_or(BoundedUnionError::Overflow)?;
    let terms = [
        mul(corners, std::mem::size_of::<[Point3; 3]>())?,
        mul(vertices, std::mem::size_of::<Point3>())?,
        mul(
            triangles,
            std::mem::size_of::<[u32; 3]>() + std::mem::size_of::<SourceFace>(),
        )?,
        mul(24, std::mem::size_of::<ExactPoint3>())?,
        mul(24, std::mem::size_of::<Point3>())?,
        mul(3, std::mem::size_of::<[Point3; 3]>())?,
    ];
    terms.into_iter().try_fold(0usize, |n, v| {
        n.checked_add(v).ok_or(BoundedUnionError::Overflow)
    })
}

/// Union only; source-face order is stable A then B (no legacy output sorting).
/// Work units: input elements, broad-phase pairs, 1024 per exact intersection
/// call, 1 per split/append/search and reserved 5*other_faces per classification.
/// Units bound structural invocations, not time/instructions. Geometry tolerances
/// and C0 intersection seams are inherited; consumers MUST qualify the result.
pub fn union_exact_bounded_into(
    va: &[Point3],
    ta: &[[u32; 3]],
    vb: &[Point3],
    tb: &[[u32; 3]],
    budget: UnionBudget,
    b: UnionBuffers<'_>,
    cancelled: &dyn Fn() -> bool,
) -> Result<UnionReceipt, BoundedUnionError> {
    let logical = union_buffer_bytes(
        b.corners_a.len(),
        b.corners_b.len(),
        b.current.len().max(b.next.len()),
        b.vertices.len(),
        b.triangles.len().max(b.source_faces.len()),
    )?;
    if logical > budget.bytes {
        return Err(BoundedUnionError::Bytes);
    }
    if va.len() < 4
        || vb.len() < 4
        || ta.len() < 4
        || tb.len() < 4
        || ta.len() > u32::MAX as usize
        || tb.len() > u32::MAX as usize
        || b.vertices.len() > u32::MAX as usize
        || b.corners_a.len() < ta.len()
        || b.corners_b.len() < tb.len()
        || b.current.is_empty()
        || b.next.is_empty()
    {
        return Err(BoundedUnionError::InvalidInput);
    }
    let mut work = budget.work;
    for (v, t) in [(va, ta), (vb, tb)] {
        charge(
            &mut work,
            v.len()
                .checked_add(mul(t.len(), 4)?)
                .ok_or(BoundedUnionError::Overflow)?,
        )?;
        if v.iter().any(|p| {
            ![p.x, p.y, p.z]
                .iter()
                .all(|x| x.is_finite() && x.abs() <= 1_000_000.0)
        }) || t.iter().flatten().any(|&i| i as usize >= v.len())
        {
            return Err(BoundedUnionError::InvalidInput);
        }
        for t in t {
            let n = tri_normal(v[t[0] as usize], v[t[1] as usize], v[t[2] as usize]);
            let a = n.x * n.x + n.y * n.y + n.z * n.z;
            if !a.is_finite() || a <= 1e-24 {
                return Err(BoundedUnionError::InvalidInput);
            }
        }
    }
    for (v, t, out) in [(va, ta, &mut *b.corners_a), (vb, tb, &mut *b.corners_b)] {
        for (i, t) in t.iter().enumerate() {
            out[i] = [v[t[0] as usize], v[t[1] as usize], v[t[2] as usize]];
        }
    }
    let mut nv = 0usize;
    let mut nt = 0usize;
    let mut current = b.current;
    let mut next = b.next;
    for (mesh, own, other) in [
        (0u8, &b.corners_a[..ta.len()], &b.corners_b[..tb.len()]),
        (1u8, &b.corners_b[..tb.len()], &b.corners_a[..ta.len()]),
    ] {
        for (face, &original) in own.iter().enumerate() {
            if cancelled() {
                return Err(BoundedUnionError::Cancelled);
            }
            charge(&mut work, 1)?;
            current[0] = original;
            let mut count = 1usize;
            for &opposing in other {
                if cancelled() {
                    return Err(BoundedUnionError::Cancelled);
                }
                charge(&mut work, 1)?;
                if !aabb_overlap(&tri_aabb(original), &tri_aabb(opposing)) {
                    continue;
                }
                charge(&mut work, 1024)?;
                let (_, segment) = tri_tri_intersect_3_exact(
                    original[0],
                    original[1],
                    original[2],
                    opposing[0],
                    opposing[1],
                    opposing[2],
                );
                let Some(segment) = segment else { continue };
                let p = segment.start.to_point3();
                let q = segment.end.to_point3();
                let mut n = 0usize;
                for &fragment in &current[..count] {
                    if cancelled() {
                        return Err(BoundedUnionError::Cancelled);
                    }
                    charge(&mut work, 1)?;
                    // Existing mathematical splitter; fixed <=3 temporary children.
                    let children = split_one(fragment, p, q);
                    let end = n
                        .checked_add(children.len())
                        .ok_or(BoundedUnionError::Overflow)?;
                    if end > next.len() {
                        return Err(BoundedUnionError::Fragments);
                    }
                    charge(&mut work, children.len())?;
                    next[n..end].copy_from_slice(&children);
                    n = end;
                }
                std::mem::swap(&mut current, &mut next);
                count = n;
            }
            for &fragment in &current[..count] {
                if cancelled() {
                    return Err(BoundedUnionError::Cancelled);
                }
                charge(
                    &mut work,
                    mul(other.len(), 5)?
                        .checked_add(1)
                        .ok_or(BoundedUnionError::Overflow)?,
                )?;
                let keep = match classify_point(tri_centroid(fragment), other) {
                    MeshSide::Outside => true,
                    MeshSide::Inside => false,
                    MeshSide::OnSurface => mesh == 0 && normals_align(fragment, other),
                };
                if !keep {
                    continue;
                }
                let n = tri_normal(fragment[0], fragment[1], fragment[2]);
                if n.x * n.x + n.y * n.y + n.z * n.z < 1e-24 {
                    continue;
                }
                if nt >= b.triangles.len() || nt >= b.source_faces.len() {
                    return Err(BoundedUnionError::Output);
                }
                let mut ids = [0; 3];
                for (k, p) in fragment.into_iter().enumerate() {
                    if ![p.x, p.y, p.z].iter().all(|x| x.is_finite()) {
                        return Err(BoundedUnionError::InvalidInput);
                    }
                    let mut found = None;
                    for (i, &v) in b.vertices[..nv].iter().enumerate() {
                        charge(&mut work, 1)?;
                        if dist_sq(p, v) < 1e-18 {
                            found = Some(i);
                            break;
                        }
                    }
                    ids[k] = if let Some(i) = found {
                        i as u32
                    } else {
                        if nv >= b.vertices.len() {
                            return Err(BoundedUnionError::Output);
                        }
                        charge(&mut work, 1)?;
                        b.vertices[nv] = p;
                        nv += 1;
                        (nv - 1) as u32
                    };
                }
                charge(&mut work, 1)?;
                b.triangles[nt] = ids;
                b.source_faces[nt] = SourceFace {
                    mesh,
                    face: face as u32,
                };
                nt += 1;
            }
        }
    }
    if cancelled() {
        return Err(BoundedUnionError::Cancelled);
    }
    Ok(UnionReceipt {
        vertices: nv,
        triangles: nt,
        work: budget.work - work,
        logical_bytes: logical,
    })
}
