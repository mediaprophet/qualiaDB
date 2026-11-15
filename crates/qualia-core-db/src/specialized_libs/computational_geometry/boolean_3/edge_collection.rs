//! Finite six-edge collection using checked original-coordinate constructions.
//! Shared endpoints are original points; no distance authorizes an identity.
use super::super::super::{ParameterSpan, Point3, Sign};
use super::exact_registry::{self as exact, Registry, Scratch, Triangle3};
use super::{charge, Error};

fn append(ids: &mut [u32; 12], count: &mut usize, id: u32, work: &mut usize) -> Result<(), Error> {
    charge(work, *count + 1)?;
    if !ids[..*count].contains(&id) {
        if *count == ids.len() { return Err(Error::Constraints); }
        ids[*count] = id;
        *count += 1;
    }
    Ok(())
}

// For coplanar convex triangles a supporting edge of either triangle separates
// all zero-area contacts/disjoint cases. Partial positive-area overlap still
// refuses: this collector does not pretend to construct a coplanar arrangement.
fn coplanar_has_area(
    one: [Point3; 3], two: [Point3; 3], a: &Triangle3, b: &Triangle3,
    work: &mut usize, arithmetic: &mut Scratch<'_>,
) -> Result<bool, Error> {
    charge(work, 9)?;
    if one.iter().all(|p| two.contains(p)) { return Ok(false); }
    for (triangle, other) in [(a, two), (b, one)] {
        for edge in 0..3 {
            let mut inside = false;
            for p in other {
                let side = triangle.side_plain(p, edge, &mut arithmetic.checked()?, work)
                    .map_err(exact::construction_error)?;
                inside |= side == triangle.winding();
            }
            if !inside { return Ok(false); }
        }
    }
    Ok(true)
}

pub(super) fn collect(
    one: [Point3; 3], two: [Point3; 3], registry: &mut Registry,
    work: &mut usize, arithmetic: &mut Scratch<'_>,
) -> Result<Option<[u32; 2]>, Error> {
    let a = exact::triangle(one, work, arithmetic)?;
    let b = exact::triangle(two, work, arithmetic)?;
    let mut sides_a = [Sign::Zero; 3];
    let mut sides_b = [Sign::Zero; 3];
    charge(work, 6)?;
    for i in 0..3 {
        sides_a[i] = b.plane.orient_plain(one[i], &mut arithmetic.checked()?, work)
            .map_err(exact::construction_error)?;
        sides_b[i] = a.plane.orient_plain(two[i], &mut arithmetic.checked()?, work)
            .map_err(exact::construction_error)?;
    }
    if sides_a.iter().all(|&s| s == Sign::Zero) {
        if coplanar_has_area(one, two, &a, &b, work, arithmetic)? {
            return Err(Error::CoplanarOverlap);
        }
        return Ok(None);
    }
    // Strictly one side of either plane implies no finite intersection.
    for sides in [sides_a, sides_b] {
        charge(work, 3)?;
        if sides[0] != Sign::Zero && sides.iter().all(|&s| s == sides[0]) { return Ok(None); }
    }
    charge(work, 12)?;
    let mut ids = [u32::MAX; 12];
    let mut count = 0;
    for (source, source_facet, target, sides) in [(one, &a, &b, sides_a), (two, &b, &a, sides_b)] {
        for i in 0..3 {
            let j = (i + 1) % 3;
            if sides[i] == Sign::Zero {
                if target.contains_plain(source[i], &mut arithmetic.checked()?, work)
                    .map_err(exact::construction_error)? {
                    let id = registry.plain(source[i], work)?;
                    append(&mut ids, &mut count, id, work)?;
                }
            }
            // An in-plane edge needs no construction: endpoints are tested
            // here, and opposite-triangle edges provide any clipped endpoints.
            if sides[i] == Sign::Zero || sides[j] == Sign::Zero || sides[i] == sides[j] { continue; }
            let (p, span) = target.plane.intersect(source[i], source[j], &mut arithmetic.checked()?, work)
                .map_err(exact::construction_error)?;
            if !matches!(span, ParameterSpan::Interior) { return Err(Error::Incidence); }
            if target.contains(&p, &mut arithmetic.checked()?, work).map_err(exact::construction_error)? {
                if !source_facet.contains(&p, &mut arithmetic.checked()?, work).map_err(exact::construction_error)? {
                    return Err(Error::Incidence);
                }
                let id = registry.constructed(p, work, arithmetic)?;
                append(&mut ids, &mut count, id, work)?;
            }
        }
    }
    if count < 2 { return Ok(None); }
    // Rounded coordinates choose only a nonconstant sorting axis. Every
    // ordering/identity decision still uses the exact rational comparator.
    let p = registry.nodes[ids[0] as usize].p;
    let q = registry.nodes[ids[1] as usize].p;
    let delta = [(p.x-q.x).abs(), (p.y-q.y).abs(), (p.z-q.z).abs()];
    let axis = (0..3).max_by(|&i,&j| delta[i].total_cmp(&delta[j])).unwrap();
    if delta[axis] == 0. { return Err(Error::RoundedCollision); }
    let mut low = ids[0];
    let mut high = ids[0];
    for &id in &ids[1..count] {
        if exact::cmp_axis(&registry.nodes[id as usize], &registry.nodes[low as usize], axis, work, arithmetic)? == Sign::Negative { low = id; }
        if exact::cmp_axis(&registry.nodes[id as usize], &registry.nodes[high as usize], axis, work, arithmetic)? == Sign::Positive { high = id; }
    }
    if low == high { return Err(Error::Incidence); }
    Ok(Some([low, high]))
}
