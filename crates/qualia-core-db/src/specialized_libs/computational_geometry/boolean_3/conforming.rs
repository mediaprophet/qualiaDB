//! Bounded shared-constraint union. Legacy Boolean APIs remain unchanged.
//! Exact construction identities are shared across original facets. No
//! proximity weld or independently inserted Steiner midpoint is permitted.
//! Output slices are unpublished scratch until Ok: errors may leave prefixes
//! but never return successful counts. Consumers still qualify topology and
//! attribute projection; the legacy fragment classifier is not replaced here.
#[path = "exact_registry.rs"]
mod exact_registry;
#[path = "facet_constraints.rs"]
mod facet_constraints;
#[path = "edge_collection.rs"]
mod edge_collection;
#[cfg(test)]
#[path = "conforming_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "edge_collection_tests.rs"]
mod edge_collection_tests;
use super::super::Point3;
#[cfg(test)]
use super::super::{ExactPoint3, Sign};
use super::{
    bounded::{self, SourceFace, UnionBudget, UnionBuffers, UnionReceipt},
    *,
};
#[derive(Debug)]
pub enum Error {
    Input,
    Overflow,
    Bytes,
    Work,
    Cancelled,
    Points,
    Output,
    Arithmetic,
    Construction(super::super::exact_construct_3::checked::Error),
    RoundedCollision,
    RegistryCollision {
        node: usize,
        existing_plain: bool,
        rounded: Point3,
        old_lengths: [usize; 4],
        new_lengths: [usize; 4],
        old_components: [[f64; 8]; 4],
        new_components: [[f64; 8]; 4],
    },
    PairAt {
        faces: [usize; 2],
        triangles: [[Point3; 3]; 2],
        reason: Box<Error>,
    },
    Facet,
    FacetIncidence {
        admitted: usize,
        corners: [bool; 3],
    },
    FacetDelaunay {
        points: usize,
        reason: super::super::DelaunayError,
    },
    FacetChild {
        stage: &'static str,
        points: [super::super::Point2; 3],
    },
    FacetAlignment {
        child: [Point3; 3],
    },
    Constraints,
    Incidence,
    CoplanarOverlap,
    FacetAt {
        face: usize,
        triangle: [Point3; 3],
        reason: Box<Error>,
    },
}
pub(super) fn charge(work: &mut usize, n: usize) -> Result<(), Error> {
    *work = work.checked_sub(n).ok_or(Error::Work)?;
    Ok(())
}
pub(super) struct Cut {
    faces: [usize; 2],
    nodes: [u32; 2],
}
/// Fixed cold logical envelope: 2048 constructed ExactPoint3 values (<27MiB),
/// registry up to8192 nodes, <=8192 cuts, per-facet256point Delaunay/recovery,
/// expansion scratch, geometry/source arrays and geometric Vec growth. Includes
/// actual caller union buffers separately. Allocator bookkeeping is excluded.
pub fn union_buffer_bytes(
    a: usize,
    b: usize,
    f: usize,
    v: usize,
    t: usize,
) -> Result<usize, Error> {
    bounded::union_buffer_bytes(a, b, f, v, t)
        .map_err(|_| Error::Overflow)?
        .checked_add(64 * 1024 * 1024)
        .ok_or(Error::Overflow)
}
pub fn union_conforming_bounded_into(
    va: &[Point3],
    ta: &[[u32; 3]],
    vb: &[Point3],
    tb: &[[u32; 3]],
    budget: UnionBudget,
    b: UnionBuffers<'_>,
    cancel: &dyn Fn() -> bool,
) -> Result<UnionReceipt, Error> {
    let logical = union_buffer_bytes(
        b.corners_a.len(),
        b.corners_b.len(),
        b.current.len().max(b.next.len()),
        b.vertices.len(),
        b.triangles.len().max(b.source_faces.len()),
    )?;
    if logical > budget.bytes {
        return Err(Error::Bytes);
    }
    if ta.len().checked_add(tb.len()).ok_or(Error::Overflow)? > 16384
        || ta.len() < 4
        || tb.len() < 4
        || va.len() < 4
        || vb.len() < 4
        || b.corners_a.len() < ta.len()
        || b.corners_b.len() < tb.len()
    {
        return Err(Error::Input);
    }
    let mut work = budget.work;
    for (v, t) in [(va, ta), (vb, tb)] {
        charge(
            &mut work,
            v.len()
                .checked_add(t.len().checked_mul(4).ok_or(Error::Overflow)?)
                .ok_or(Error::Overflow)?,
        )?;
        for p in v {
            if cancel() { return Err(Error::Cancelled); }
            if [p.x, p.y, p.z].iter().any(|x| !x.is_finite() || x.abs() > 1e6) {
                return Err(Error::Input);
            }
        }
        for t in t {
            if cancel() { return Err(Error::Cancelled); }
            if t.iter().any(|&i| i as usize >= v.len()) { return Err(Error::Input); }
            let n = tri_normal(v[t[0] as usize], v[t[1] as usize], v[t[2] as usize]);
            if n.x * n.x + n.y * n.y + n.z * n.z <= 1e-24 {
                return Err(Error::Input);
            }
        }
    }
    if cancel() {
        return Err(Error::Cancelled);
    }
    let mut registry = exact_registry::Registry::new(b.vertices.len());
    let mut arithmetic = exact_registry::Scratch::new(&mut work, cancel)?;
    for &p in va.iter().chain(vb) {
        if cancel() { return Err(Error::Cancelled); }
        registry.plain(p, &mut work)?;
    }
    let mut all_faces = Vec::new();
    for (vertices, triangles, out) in [(va, ta, &mut *b.corners_a), (vb, tb, &mut *b.corners_b)] {
        for (i, t) in triangles.iter().enumerate() {
            if cancel() { return Err(Error::Cancelled); }
            charge(&mut work, 10)?;
            let p = t.map(|j| vertices[j as usize]);
            out[i] = p;
            all_faces.push(p);
        }
    }
    let mut cuts = Vec::new();
    for (a, &one) in b.corners_a[..ta.len()].iter().enumerate() {
        for (j, &two) in b.corners_b[..tb.len()].iter().enumerate() {
            if cancel() {
                return Err(Error::Cancelled);
            }
            charge(&mut work, 1)?;
            if !aabb_overlap(&tri_aabb(one), &tri_aabb(two)) {
                continue;
            }
            let segment = edge_collection::collect(one, two, &mut registry, &mut work, &mut arithmetic)
                .map_err(|e| Error::PairAt {
                    faces: [a, ta.len() + j], triangles: [one, two], reason: Box::new(e),
                })?;
            if let Some([p, q]) = segment {
                if cuts.len() >= 8192 {
                    return Err(Error::Constraints);
                }
                charge(&mut work, 1)?;
                cuts.push(Cut {
                    faces: [a, ta.len() + j],
                    nodes: [p, q],
                });
            }
        }
    }
    let mut nv = 0usize;
    let mut nt = 0usize;
    charge(&mut work, registry.nodes.len())?;
    let mut output_ids = vec![u32::MAX; registry.nodes.len()];
    for (face, &original) in all_faces.iter().enumerate() {
        if cancel() {
            return Err(Error::Cancelled);
        }
        let fragments = facet_constraints::triangulate(
            face,
            original,
            &registry,
            &cuts,
            &all_faces,
            &mut work,
            cancel,
            &mut arithmetic,
        )
        .map_err(|e| Error::FacetAt {
            face,
            triangle: original,
            reason: Box::new(e),
        })?;
        let (mesh, local, other) = if face < ta.len() {
            (0, face, &b.corners_b[..tb.len()])
        } else {
            (1, face - ta.len(), &b.corners_a[..ta.len()])
        };
        for ids in fragments {
            if cancel() {
                return Err(Error::Cancelled);
            }
            let fragment = ids.map(|i| registry.nodes[i as usize].p);
            charge(
                &mut work,
                other.len().checked_mul(5).ok_or(Error::Overflow)?,
            )?;
            let keep = match classify_point(tri_centroid(fragment), other) {
                MeshSide::Outside => true,
                MeshSide::Inside => false,
                MeshSide::OnSurface => mesh == 0 && normals_align(fragment, other),
            };
            if !keep {
                continue;
            }
            if nt >= b.triangles.len() || nt >= b.source_faces.len() {
                return Err(Error::Output);
            }
            let mut out = [0; 3];
            for (k, id) in ids.into_iter().enumerate() {
                charge(&mut work, 1)?;
                let mapped = &mut output_ids[id as usize];
                if *mapped == u32::MAX {
                    if nv >= b.vertices.len() {
                        return Err(Error::Output);
                    }
                    b.vertices[nv] = registry.nodes[id as usize].p;
                    *mapped = nv as u32;
                    nv += 1;
                }
                out[k] = *mapped;
            }
            b.triangles[nt] = out;
            b.source_faces[nt] = SourceFace {
                mesh,
                face: local as u32,
            };
            nt += 1;
        }
    }
    if cancel() {
        return Err(Error::Cancelled);
    }
    Ok(UnionReceipt {
        vertices: nv,
        triangles: nt,
        work: budget.work - work,
        logical_bytes: logical,
    })
}
