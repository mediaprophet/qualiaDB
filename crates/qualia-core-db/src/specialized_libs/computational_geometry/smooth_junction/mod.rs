//! Bounded geometric projection of an explicit incident/connected-cluster seed
//! onto the existing positive smooth-union field. Exact fixed boundary bands,
//! source-face correspondence and discrete quality criteria are mandatory.
//! Finite triangulations are not claimed to be analytic C1 surfaces.
mod project;
mod refine;
#[cfg(test)]
mod tests;
use super::tapered_branch_field::TaperedFieldSegment;
use super::{surface_topology_qualification as topology, EdgeSlot, HalfEdge, Point3};
pub use project::{smooth_into, validate_controls};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FaceSource {
    pub piece: u32,
    pub face: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Controls {
    pub blend: f64,
    pub tolerance: f64,
    pub gradient_step: f64,
    pub max_step: f64,
    pub max_displacement: f64,
    pub iterations: usize,
    pub refinements: usize,
    pub max_dihedral: f64,
    pub min_normal_alignment: f64,
    pub min_area_sq: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub vertices: usize,
    pub faces: usize,
    pub bytes: usize,
    pub work: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Overflow,
    Bytes,
    Work,
    Capacity,
    Cancelled,
    Topology,
    BoundaryContact,
    Field,
    Convergence,
    Displacement,
    Orientation,
    Normal,
    Dihedral,
}
pub struct Input<'a> {
    pub positions: &'a [Point3],
    pub triangles: &'a [[u32; 3]],
    pub sources: &'a [FaceSource],
    pub segments: &'a [TaperedFieldSegment],
    pub fade_coordinates: &'a [f64],
}
pub struct Output<'a> {
    pub positions: &'a mut [Point3],
    pub references: &'a mut [Point3],
    pub triangles: &'a mut [[u32; 3]],
    pub sources: &'a mut [FaceSource],
    pub normals: &'a mut [Point3],
    pub weights: &'a mut [f64],
}
#[derive(Clone, Copy, Debug)]
pub struct Receipt {
    pub vertices: usize,
    pub faces: usize,
    pub work_used: usize,
    pub logical_bytes: usize,
    pub moved_vertices: usize,
    pub max_field_residual: f64,
    pub max_dihedral: f64,
    pub min_normal_alignment: f64,
}
pub(super) struct Meter {
    remaining: usize,
    initial: usize,
}
impl Meter {
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.remaining = self.remaining.checked_sub(n).ok_or(Error::Work)?;
        Ok(())
    }
}
pub(super) fn mul(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_mul(b).ok_or(Error::Overflow)
}
pub(super) fn finite(p: Point3) -> bool {
    [p.x, p.y, p.z]
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 1e6)
}
pub(super) fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
pub(super) fn add(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
pub(super) fn scale(a: Point3, s: f64) -> Point3 {
    Point3::new(a.x * s, a.y * s, a.z * s)
}
pub(super) fn dot(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}
pub(super) fn length(a: Point3) -> f64 {
    dot(a, a).sqrt()
}
pub(super) fn normal(a: Point3, b: Point3, c: Point3) -> Point3 {
    let u = sub(b, a);
    let v = sub(c, a);
    Point3::new(
        u.y * v.z - u.z * v.y,
        u.z * v.x - u.x * v.z,
        u.x * v.y - u.y * v.x,
    )
}
pub(super) fn unit(a: Point3) -> Result<Point3, Error> {
    let l = length(a);
    if !l.is_finite() || l <= 1e-15 {
        return Err(Error::Normal);
    }
    Ok(scale(a, 1. / l))
}
pub(super) fn weight(coordinate: f64) -> f64 {
    let t = coordinate.clamp(0., 1.);
    t * t * (3. - 2. * t)
}
pub(super) fn check_cancel(cancelled: &dyn Fn() -> bool) -> Result<(), Error> {
    if cancelled() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
pub(super) fn capacity<T>(count: usize) -> Result<Vec<T>, Error> {
    let mut v = Vec::new();
    v.try_reserve_exact(count).map_err(|_| Error::Bytes)?;
    if v.capacity() > count {
        return Err(Error::Bytes);
    }
    Ok(v)
}
pub(super) fn topology_edges(
    positions: &[Point3],
    triangles: &[[u32; 3]],
    meter: &mut Meter,
) -> Result<Vec<HalfEdge>, Error> {
    let q = topology::surface_qualification_storage_bounds(
        u32::try_from(positions.len()).map_err(|_| Error::Overflow)?,
        triangles.len(),
    )
    .map_err(|_| Error::Topology)?;
    let mut edges = capacity(q.half_edges)?;
    edges.resize(q.half_edges, HalfEdge::default());
    let mut slots = capacity(q.edge_slots)?;
    slots.resize(q.edge_slots, EdgeSlot::default());
    let mut starts = capacity(q.vertex_starts)?;
    starts.resize(q.vertex_starts, 0);
    let mut visited = capacity(q.visited_half_edges)?;
    visited.resize(q.visited_half_edges, false);
    let mut labels = capacity(q.face_labels)?;
    labels.resize(q.face_labels, 0);
    let mut queue = capacity(q.face_queue)?;
    queue.resize(q.face_queue, 0);
    topology::qualify_surface_topology_budgeted_into(
        positions.len() as u32,
        triangles,
        &mut meter.remaining,
        &mut edges,
        &mut slots,
        &mut starts,
        &mut visited,
        &mut labels,
        &mut queue,
    )
    .map_err(|_| Error::Topology)?;
    Ok(edges)
}
/// Includes caller output and all retained/next/refinement/topology/normal
/// buffers. Per-face1024 covers 3 halfedges and the power-of-two edge hash,
/// dual face/source arrays, midpoint maps and field/normal scratch. This is
/// logical cold storage, not allocator bookkeeping or measured process stack.
pub fn storage_bytes(limits: Limits) -> Result<usize, Error> {
    mul(limits.vertices, 512)?
        .checked_add(mul(limits.faces, 1024)?)
        .and_then(|n| n.checked_add(4 * 1024 * 1024))
        .ok_or(Error::Overflow)
}
