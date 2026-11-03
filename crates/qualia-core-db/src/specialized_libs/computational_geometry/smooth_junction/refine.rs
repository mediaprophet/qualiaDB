use super::*;
/// Shared-edge refinement with fixed-band edges unsplit. Source faces remain
/// unchanged as references: new points are affine combinations on the original
/// seed faces. Polygon fan centers avoid cracks for mixed split patterns.
pub(super) fn refine(
    positions: &mut Vec<Point3>,
    triangles: &mut Vec<[u32; 3]>,
    sources: &mut Vec<FaceSource>,
    coordinates: &mut Vec<f64>,
    limits: Limits,
    meter: &mut Meter,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), Error> {
    let edges = topology_edges(positions, triangles, meter)?;
    meter.charge(mul(edges.len(), 10)?)?;
    let mut midpoint = capacity(edges.len())?;
    midpoint.resize(edges.len(), u32::MAX);
    for i in 0..edges.len() {
        check_cancel(cancelled)?;
        let twin = edges[i].twin as usize;
        if twin >= edges.len() {
            return Err(Error::Topology);
        }
        if i > twin {
            continue;
        }
        let a = edges[i].origin as usize;
        let b = edges[edges[i].next as usize].origin as usize;
        if coordinates[a] <= 0. && coordinates[b] <= 0. {
            continue;
        }
        if positions.len() >= limits.vertices {
            return Err(Error::Capacity);
        }
        let id = positions.len() as u32;
        positions.push(scale(add(positions[a], positions[b]), 0.5));
        coordinates.push((coordinates[a] + coordinates[b]) * 0.5);
        midpoint[i] = id;
        midpoint[twin] = id;
    }
    let mut next_triangles = capacity(limits.faces)?;
    let mut next_sources = capacity(limits.faces)?;
    for (f, t) in triangles.iter().enumerate() {
        check_cancel(cancelled)?;
        meter.charge(16)?;
        let mid = [midpoint[f * 3], midpoint[f * 3 + 1], midpoint[f * 3 + 2]];
        if mid.iter().all(|&i| i == u32::MAX) {
            if next_triangles.len() >= limits.faces {
                return Err(Error::Capacity);
            }
            next_triangles.push(*t);
            next_sources.push(sources[f]);
            continue;
        }
        let mut polygon = [0u32; 6];
        let mut n = 0usize;
        for i in 0..3 {
            polygon[n] = t[i];
            n += 1;
            if mid[i] != u32::MAX {
                polygon[n] = mid[i];
                n += 1
            }
        }
        if positions.len() >= limits.vertices
            || next_triangles.len().checked_add(n).ok_or(Error::Overflow)? > limits.faces
        {
            return Err(Error::Capacity);
        }
        let center = positions.len() as u32;
        positions.push(scale(
            add(
                add(positions[t[0] as usize], positions[t[1] as usize]),
                positions[t[2] as usize],
            ),
            1. / 3.,
        ));
        coordinates.push(
            (coordinates[t[0] as usize] + coordinates[t[1] as usize] + coordinates[t[2] as usize])
                / 3.,
        );
        for i in 0..n {
            next_triangles.push([polygon[i], polygon[(i + 1) % n], center]);
            next_sources.push(sources[f]);
        }
    }
    *triangles = next_triangles;
    *sources = next_sources;
    Ok(())
}
