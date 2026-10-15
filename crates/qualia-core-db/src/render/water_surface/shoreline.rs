use crate::specialized_libs::computational_geometry::{point_segment_distance_sq_2d, Point2};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShoreError {
    InvalidInput,
    OutputTooSmall,
    ScratchTooSmall,
    NonManifold,
    NoBoundary,
    SpatialIndex,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShoreReceipt {
    pub boundary_edges: usize,
    pub max_distance: f32,
    pub segment_evaluations: u64,
}
pub struct ShoreTopology<'a> {
    pub(crate) edges: &'a [u64],
    pub(crate) parents: &'a [u64],
}
impl ShoreTopology<'_> {
    pub fn boundary_edges(&self) -> usize {
        self.edges.len()
    }
    pub fn vertices(&self) -> usize {
        self.parents.len()
    }
    pub(crate) fn referenced(&self, index: usize) -> bool {
        self.parents[index] & REFERENCED != 0
    }
    pub(crate) fn component(&self, index: usize) -> u32 {
        self.parents[index] as u32
    }
}
const REFERENCED: u64 = 1 << 62;
const BOUNDARY: u64 = 1 << 63;
fn root(parents: &mut [u64], index: usize) -> usize {
    let mut at = index;
    while parents[at] as u32 as usize != at {
        at = parents[at] as u32 as usize;
    }
    let result = at;
    let mut at = index;
    while parents[at] as u32 as usize != at {
        let next = parents[at] as u32 as usize;
        parents[at] = (parents[at] & !0xffff_ffff) | result as u64;
        at = next;
    }
    result
}
fn join(parents: &mut [u64], a: usize, b: usize) {
    let a = root(parents, a);
    let b = root(parents, b);
    if a == b {
        return;
    }
    let ar = (parents[a] >> 32) & 0xffff;
    let br = (parents[b] >> 32) & 0xffff;
    let (child, parent) = if ar < br { (a, b) } else { (b, a) };
    parents[child] = (parents[child] & !0xffff_ffff) | parent as u64;
    if ar == br {
        parents[parent] += 1 << 32;
    }
}
/// Distance in mesh X/Z units to the nearest topological boundary edge.
/// `edges` is reusable scratch, one u64 per triangle index plus input vertex. Sorted
/// undirected edge keys compact boundary edges in place; no heap is allocated.
/// Separate surfaces keep separate boundaries. Streamed tile seams need shared
/// topology before this authoring pass, not a shader guess about their outline.
pub fn shore_topology_into<'a>(
    positions: &[[f32; 3]],
    indices: &[u32],
    edges: &'a mut [u64],
) -> Result<ShoreTopology<'a>, ShoreError> {
    if positions.is_empty()
        || indices.is_empty()
        || indices.len() % 3 != 0
        || positions.len() > u32::MAX as usize
        || positions
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 1e8)
        || indices
            .iter()
            .any(|index| *index as usize >= positions.len())
        || indices
            .chunks_exact(3)
            .any(|tri| tri[0] == tri[1] || tri[1] == tri[2] || tri[0] == tri[2])
    {
        return Err(ShoreError::InvalidInput);
    }
    let needed = indices
        .len()
        .checked_add(positions.len())
        .ok_or(ShoreError::ScratchTooSmall)?;
    if edges.len() < needed {
        return Err(ShoreError::ScratchTooSmall);
    }
    let (edges, parents) = edges.split_at_mut(indices.len());
    let parents = &mut parents[..positions.len()];
    for (index, parent) in parents.iter_mut().enumerate() {
        *parent = index as u64;
    }
    let mut write = 0;
    for tri in indices.chunks_exact(3) {
        for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            parents[a as usize] |= REFERENCED;
            parents[b as usize] |= REFERENCED;
            join(parents, a as usize, b as usize);
            edges[write] = ((a.min(b) as u64) << 32) | a.max(b) as u64;
            write += 1;
        }
    }
    edges[..write].sort_unstable();
    let mut read = 0;
    let mut count = 0;
    while read < write {
        let edge = edges[read];
        let start = read;
        while read < write && edges[read] == edge {
            read += 1;
        }
        match read - start {
            1 => {
                edges[count] = edge;
                count += 1;
            }
            2 => {}
            _ => return Err(ShoreError::NonManifold),
        }
    }
    if count == 0 {
        return Err(ShoreError::NoBoundary);
    }
    for index in 0..parents.len() {
        let component = root(parents, index);
        parents[index] = (parents[index] & REFERENCED) | component as u64;
    }
    for edge in &edges[..count] {
        let component = parents[(*edge >> 32) as usize] as u32 as usize;
        parents[component] |= BOUNDARY;
    }
    for index in 0..parents.len() {
        let component = parents[index] as u32 as usize;
        if parents[index] & REFERENCED != 0 && parents[component] & BOUNDARY == 0 {
            return Err(ShoreError::NoBoundary);
        }
    }
    Ok(ShoreTopology {
        edges: &edges[..count],
        parents,
    })
}
/// Allocation-free reference calculation; accelerated loading shares its topology.
pub fn shore_distance_into(
    positions: &[[f32; 3]],
    indices: &[u32],
    distances: &mut [f32],
    depth_scales: &mut [f32],
    edges: &mut [u64],
) -> Result<ShoreReceipt, ShoreError> {
    if distances.len() < positions.len() || depth_scales.len() < positions.len() {
        return Err(ShoreError::OutputTooSmall);
    }
    let topology = shore_topology_into(positions, indices, edges)?;
    let parents = topology.parents;
    let edges = topology.edges;
    let count = edges.len();
    let point = |index: usize| {
        let p = positions[index];
        Point2::new(p[0] as f64, p[2] as f64)
    };
    depth_scales[..positions.len()].fill(0.0);
    let mut max_distance = 0.0_f32;
    let mut segment_evaluations = 0;
    for index in 0..positions.len() {
        if parents[index] & REFERENCED == 0 {
            distances[index] = 0.0;
            continue;
        }
        let component = parents[index] as u32;
        let p = point(index);
        let mut distance = f64::INFINITY;
        for edge in &edges[..count] {
            let a = (*edge >> 32) as usize;
            let b = (*edge as u32) as usize;
            if parents[a] as u32 != component {
                continue;
            }
            segment_evaluations += 1;
            distance = distance.min(point_segment_distance_sq_2d(p, point(a), point(b)));
        }
        let distance = distance.sqrt() as f32;
        distances[index] = distance;
        max_distance = max_distance.max(distance);
        let component = component as usize;
        depth_scales[component] = depth_scales[component].max(distance);
    }
    for index in 0..positions.len() {
        depth_scales[index] = depth_scales[parents[index] as u32 as usize];
    }
    Ok(ShoreReceipt {
        boundary_edges: count,
        max_distance,
        segment_evaluations,
    })
}
