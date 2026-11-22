//! Q42-admitted vertex-block-descent tissue and FEMC surface correspondence.

use crate::{
    container_10d::{decode_correspondence_section, deform_surface_vertices_zero_heap, parse_mesh_header,
        HolographicPhaseHeader, SectionType, TypedFieldRecord, TypedPayloadResolver, VertexTetBinding,
        CORRESPONDENCE_HEADER_SIZE, FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE, FLAG_FEMC_CONFORMING_BARYCENTRIC,
        GRAPHICS_ASSET_RING_BYTES, HOLOGRAPHIC_PHASE_HEADER_SIZE, MANIFOLD_IDENTITY_V2_MAX_FIELDS,
        VERTEX_TET_BINDING_SIZE},
    etalon_render::skeletal::{step_vbd_soft_tissue_zero_heap, VbdTetElement, VbdVertex},
    q42::m64_matter::{M64DecodedRecord, M64Reader},
    q42_volume::Q42RangeQueryCursor,
    q_hash, NQuin,
    render::portal_load_10d::{decode_load_10d_in_frame, Load10dGpuBuffers},
};

use super::{graph::index, MotionGraph, SemanticAction, SemanticMotionError as Error};

const MAX_NODES: usize = 64;
const MAX_TETS: usize = 16;
const MAX_VISUAL_VERTICES: usize = 32_768;
const MAX_VISUAL_TRIANGLES: u32 = 65_536;
const FIT_TOLERANCE_M: f64 = 1e-4;
/// Largest interval admitted by one native FEMC/VBD step.
pub const MAX_FEMC_STEP_S: f64 = 0.02;
const COORDINATES: &str = "https://qualiadb.org/ns/kinematics#EtalonLocalMetresYUp";

macro_rules! k {
    ($name:literal) => { q_hash(concat!("https://qualiadb.org/ns/kinematics#", $name)) };
}

const TET_SLOTS: [u64; MAX_TETS] = [
    k!("femcTet0"), k!("femcTet1"), k!("femcTet2"), k!("femcTet3"),
    k!("femcTet4"), k!("femcTet5"), k!("femcTet6"), k!("femcTet7"),
    k!("femcTet8"), k!("femcTet9"), k!("femcTet10"), k!("femcTet11"),
    k!("femcTet12"), k!("femcTet13"), k!("femcTet14"), k!("femcTet15"),
];

/// Admitted FEMC correspondence, native VBD cage and physical rest state.
pub struct PreparedFemcMotion {
    initial: FemcState,
    elements: Box<[VbdTetElement]>,
    cells: Box<[[u32; 4]]>,
    bindings: Box<[VertexTetBinding]>,
}

#[derive(Clone, Copy)]
/// Opaque solver state for one admitted FEMC cage.
pub struct FemcState {
    nodes: [VbdVertex; MAX_NODES],
    count: usize,
    cage_id: [u8; 32],
}

#[derive(Clone, Copy)]
/// Measured volume and fixed-enthesis bounds of the accepted substep.
pub struct FemcFrame {
    pub determinant_min: f64,
    pub determinant_max: f64,
    pub enthesis_drift_m: f64,
}

/// Admit a standalone quantized surface from its authored `.10d` frame,
/// resolving the target HOLO identity, FEMC table and native VBD cage against
/// the caller's already admitted Q42 action and M64 material resource.
pub fn prepare_femc_asset_motion(
    graph: &mut MotionGraph<'_, '_, '_>, action: &SemanticAction<'_>, group_iri: &str,
    material_iri: &str, digest: &[u8; 32], material: &M64Reader<'_>, asset: &[u8],
) -> Result<(PreparedFemcMotion, Load10dGpuBuffers), Error> {
    if asset.len() > GRAPHICS_ASSET_RING_BYTES { return Err(Error::BudgetExceeded); }
    let group = q_hash(group_iri);
    if graph.iri(group)? != group_iri { return Err(Error::UnknownIdentity); }
    let mut scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let mut fields = scratch;
    let view = TypedPayloadResolver::new(asset, &[], &mut scratch, &mut fields).map_err(|_| Error::InvalidFit)?;
    let mut holo = view.payloads_of_kind(FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE);
    let root = holo.next().ok_or(Error::UnboundTarget)?.map_err(|_| Error::InvalidFit)?;
    let header_bytes = root.bytes.get(..HOLOGRAPHIC_PHASE_HEADER_SIZE).ok_or(Error::InvalidFit)?;
    let header: HolographicPhaseHeader = bytemuck::pod_read_unaligned(header_bytes);
    if holo.next().is_some() || header.nquin_subject_hash != group { return Err(Error::UnboundTarget); }
    let mesh = view.section_payload(SectionType::QuantizedMesh).ok_or(Error::UnboundTarget)?;
    let (mesh, _) = parse_mesh_header(mesh).map_err(|_| Error::InvalidFit)?;
    if mesh.triangle_count > MAX_VISUAL_TRIANGLES { return Err(Error::BudgetExceeded); }
    let correspondence = view.section_payload(SectionType::CorrespondenceMap).ok_or(Error::UnboundTarget)?;
    let digest_iri = format!("urn:sha256:{}", hex::encode(digest));
    let controller = prepare_femc_motion(graph, action, group, material_iri, &digest_iri, material,
        correspondence, mesh.vertex_count as usize,
    )?;
    let visual = decode_load_10d_in_frame(asset, false).map_err(Error::FemcSurface)?;
    if visual.governance_refused || visual.field_representation != "QuantizedMesh" || visual.vertex_count != mesh.vertex_count {
        return Err(Error::UnboundTarget);
    }
    Ok((controller, visual))
}

/// The authorised action points to `deformableBody`; the body in turn names
/// the mounted `.m64` entry (`materialResource`) and its `femcMaterialDigest`
/// as an admitted `urn:sha256:<hex>` IRI. Q42 supplies exact tetra connectivity,
/// pins, constitutive coefficients and rest-coordinate convention. The body
/// supplies `femcPointChunk`, `femcPointStart`, `femcPointCount`, `femcTetCount`,
/// indexed `femcTet0`–`femcTet15` and repeated `femcPinnedVertex` facts. Each
/// tetrahedron declares `femcCorner0`–`femcCorner3` in positive orientation.
/// Corners and pins index points locally to the selected M64 range.
fn prepare_femc_motion(
    graph: &mut MotionGraph<'_, '_, '_>, action: &SemanticAction<'_>, group: u64,
    material_iri: &str, digest_iri: &str, material: &M64Reader<'_>,
    correspondence: &[u8], visual_count: usize,
) -> Result<PreparedFemcMotion, Error> {
    graph.check_budget(std::mem::size_of::<FemcState>() + MAX_TETS * std::mem::size_of::<VbdTetElement>())?;
    let action = action.authorize(graph)?;
    if graph.one(action, k!("deformableBody"))? != group { return Err(Error::UnboundTarget); }
    let resource = graph.one(group, k!("materialResource"))?;
    let digest = graph.one(group, k!("femcMaterialDigest"))?;
    let coordinates = graph.one(group, k!("femcCoordinateConvention"))?;
    if graph.iri(resource)? != material_iri || graph.iri(digest)? != digest_iri || graph.iri(coordinates)? != COORDINATES
        || material.georaster().is_some() || material.lrm3().is_some()
        || material.header().spatial_origin != [0.0; 2] || material.header().elevation_datum != [0.0, 1.0]
    {
        return Err(Error::InvalidMaterial);
    }
    let chunk_index = index(graph.one(group, k!("femcPointChunk"))?)?;
    let start = index(graph.one(group, k!("femcPointStart"))?)?;
    let count = index(graph.one(group, k!("femcPointCount"))?)?;
    let tet_count = index(graph.one(group, k!("femcTetCount"))?)?;
    if !(4..=MAX_NODES).contains(&count) || !(1..=MAX_TETS).contains(&tet_count) || !(1..=MAX_VISUAL_VERTICES).contains(&visual_count) {
        return Err(Error::BudgetExceeded);
    }
    if correspondence.len() != CORRESPONDENCE_HEADER_SIZE + visual_count * VERTEX_TET_BINDING_SIZE {
        return Err(Error::UnboundTarget);
    }
    let mut bindings: Box<[VertexTetBinding]> = vec![bytemuck::Zeroable::zeroed(); visual_count].into_boxed_slice();
    let (header, written) = decode_correspondence_section(correspondence, &mut bindings).map_err(|_| Error::InvalidFit)?;
    if written != visual_count || header.tet_cell_count as usize != tet_count
        || header.flags & FLAG_FEMC_CONFORMING_BARYCENTRIC == 0
    {
        return Err(Error::UnboundTarget);
    }
    let chunk = material.chunk(chunk_index).ok_or(Error::InvalidMaterial)?;
    if chunk.directory_entry().srid_geodetic_code != 0 { return Err(Error::InvalidMaterial); }
    let mut nodes = [VbdVertex { mass: 0.0, ..VbdVertex::default() }; MAX_NODES];
    for (slot, node) in nodes[..count].iter_mut().enumerate() {
        let ordinal = start.checked_add(slot).ok_or(Error::InvalidMaterial)?;
        let Some(M64DecodedRecord::Point(point)) = chunk.decode_record(ordinal).map_err(|_| Error::InvalidMaterial)?
        else { return Err(Error::InvalidMaterial); };
        let position = [point.x, point.y, point.z].map(f64::from);
        if position.iter().any(|v| !v.is_finite() || v.abs() > 10_000.0) { return Err(Error::InvalidMaterial); }
        node.pos = position;
        node.prev_pos = position;
    }
    let [density, mu, lambda, quality, peak_stress] = graph.numeric_lanes(group, [
        k!("femcDensityKgPerCubicMetre"), k!("femcShearModulusPa"), k!("femcLameLambdaPa"),
        k!("femcMuscleQuality"), k!("femcPeakActiveStressPa"),
    ])?;
    let fiber: [f64; 3] = graph.numeric_lanes(group, [k!("femcFiberX"), k!("femcFiberY"), k!("femcFiberZ")])?;
    let fiber_norm = fiber.iter().map(|v| v * v).sum::<f64>().sqrt();
    if density <= 0.0 || mu <= 0.0 || lambda <= 0.0 || peak_stress <= 0.0 || !(0.0..=1.0).contains(&quality)
        || !(0.999_999..=1.000_001).contains(&fiber_norm)
    {
        return Err(Error::InvalidMaterial);
    }
    let mut elements = Vec::with_capacity(tet_count);
    let mut cells = Vec::with_capacity(tet_count);
    let mut neighbors = [0u64; MAX_NODES];
    for &slot in &TET_SLOTS[..tet_count] {
        let cell = graph.one(group, slot)?;
        graph.iri(cell)?;
        let mut corners = [0u16; 4];
        for (i, predicate) in [k!("femcCorner0"), k!("femcCorner1"), k!("femcCorner2"), k!("femcCorner3")].into_iter().enumerate() {
            let vertex = index(graph.one(cell, predicate)?)?;
            if vertex >= count || corners[..i].contains(&(vertex as u16)) { return Err(Error::InvalidMaterial); }
            corners[i] = vertex as u16;
        }
        let index = corners.map(usize::from);
        let [p0, p1, p2, p3] = index.map(|i| nodes[i].pos);
        if signed_six_volume([p0, p1, p2, p3]) <= 1e-12 { return Err(Error::InvalidFit); }
        let mut element = VbdTetElement::from_rest_positions(corners, p0, p1, p2, p3).map_err(|_| Error::InvalidFit)?;
        element.mu_pa = mu;
        element.lambda_vol_pa = lambda;
        element.fiber_dir = fiber.map(|v| v / fiber_norm);
        element.holzapfel_k1_pa = 0.0;
        element.dermal_mu_pa = 0.0;
        element.muscle_quality_eta = quality;
        element.sigma0_max_pa = peak_stress;
        let neighbor_mask = index.iter().fold(0, |bits, &i| bits | (1u64 << i));
        for i in index {
            nodes[i].mass += density * element.rest_volume_m3 * 0.25;
            neighbors[i] |= neighbor_mask & !(1u64 << i);
        }
        let mapped = corners.map(u32::from);
        let mut ordered = mapped;
        ordered.sort_unstable();
        if cells.iter().any(|cell: &[u32; 4]| {
            let mut cell = *cell;
            cell.sort_unstable();
            cell == ordered
        }) { return Err(Error::AmbiguousTarget); }
        cells.push(mapped);
        elements.push(element);
    }
    assign_colors(&mut nodes[..count], &neighbors[..count])?;
    pin_entheses(graph, group, &mut nodes[..count])?;
    let mut cage_id = blake3::Hasher::new_derive_key("Qualia native FEMC cage identity v1");
    cage_id.update(&group.to_le_bytes());
    cage_id.update(correspondence);
    for node in &nodes[..count] {
        for value in node.pos.into_iter().chain([node.mass]) { cage_id.update(&value.to_le_bytes()); }
        cage_id.update(&[u8::from(node.pinned), node.color]);
    }
    for cell in &cells {
        for corner in cell { cage_id.update(&corner.to_le_bytes()); }
    }
    for value in [mu, lambda, quality, peak_stress].into_iter().chain(fiber) {
        cage_id.update(&value.to_le_bytes());
    }
    let cage_id = *cage_id.finalize().as_bytes();
    Ok(PreparedFemcMotion { initial: FemcState { nodes, count, cage_id }, elements: elements.into_boxed_slice(),
        cells: cells.into_boxed_slice(), bindings })
}

fn assign_colors(nodes: &mut [VbdVertex], neighbors: &[u64]) -> Result<(), Error> {
    // One connected cage prevents unpinned components with rigid drift.
    let mut connected = 1u64;
    for _ in nodes.iter() {
        let previous = connected;
        for (i, mask) in neighbors.iter().enumerate() {
            if connected & (1u64 << i) != 0 { connected |= *mask; }
        }
        if connected == previous { break; }
    }
    if connected.count_ones() as usize != nodes.len() { return Err(Error::UnboundTarget); }
    for i in 0..nodes.len() {
        if !nodes[i].mass.is_finite() || nodes[i].mass <= 0.0 { return Err(Error::InvalidMaterial); }
        let mut used = 0u8;
        for (j, other) in nodes[..i].iter().enumerate() {
            if neighbors[i] & (1u64 << j) != 0 { used |= 1 << other.color; }
        }
        let color = used.trailing_ones();
        if color >= 8 { return Err(Error::BudgetExceeded); }
        nodes[i].color = color as u8;
    }
    Ok(())
}

fn pin_entheses(graph: &mut MotionGraph<'_, '_, '_>, group: u64, nodes: &mut [VbdVertex]) -> Result<(), Error> {
    let mut cursor = Q42RangeQueryCursor::default();
    loop {
        let mut rows = [NQuin::default(); 4];
        let page = graph.page(group, k!("femcPinnedVertex"), cursor, &mut rows)?;
        for fact in &rows[..page.returned] {
            let node = nodes.get_mut(index(fact.object)?).ok_or(Error::InvalidFit)?;
            if std::mem::replace(&mut node.pinned, true) { return Err(Error::AmbiguousTarget); }
        }
        match page.next_cursor {
            Some(next) => cursor = next,
            None => break,
        }
    }
    if nodes.iter().all(|node| node.pinned) { return Err(Error::InvalidFit); }
    let mut pinned = nodes.iter().filter(|node| node.pinned).map(|node| node.pos);
    let first = pinned.next().ok_or(Error::InvalidFit)?;
    let second = pinned.find(|&node| length(sub(node, first)) > 1e-12).ok_or(Error::InvalidFit)?;
    if !pinned.any(|node| length(cross(sub(second, first), sub(node, first))) > 1e-12) {
        return Err(Error::InvalidFit);
    }
    Ok(())
}

impl PreparedFemcMotion {
    /// Resettable physical rest state for this cage.
    pub fn initial_state(&self) -> FemcState { self.initial }

    /// Verify and reconstruct the rest correspondence in the authored frame.
    pub fn rest_surface(&self, reference: &[[f32; 3]], output: &mut [[f32; 3]]) -> Result<(), Error> {
        self.deform(&self.initial, output)?;
        if reference.len() != self.bindings.len() { return Err(Error::UnboundTarget); }
        let mut max = 0.0_f64;
        for (reference, reconstructed) in reference.iter().zip(output.iter()) {
            let residual = length(sub(reference.map(f64::from), reconstructed.map(f64::from)));
            if !residual.is_finite() { return Err(Error::InvalidFit); }
            max = max.max(residual);
        }
        if max > FIT_TOLERANCE_M { return Err(Error::FemcRestResidual { metres: max }); }
        Ok(())
    }

    /// Advance the native solver and commit only a frame satisfying the volume
    /// and enthesis gates. Time is seconds and activation is a unit fraction.
    pub fn advance(&self, state: &mut FemcState, dt: f64, activation: f64) -> Result<FemcFrame, Error> {
        if state.cage_id != self.initial.cage_id || !dt.is_finite() || dt <= 0.0 || dt > MAX_FEMC_STEP_S
            || !(0.0..=1.0).contains(&activation)
        {
            return Err(Error::InvalidCommand);
        }
        let mut next = *state;
        let mut elements = [VbdTetElement::default(); MAX_TETS];
        elements[..self.elements.len()].copy_from_slice(&self.elements);
        for el in &mut elements[..self.elements.len()] { el.activation = activation; }
        step_vbd_soft_tissue_zero_heap(&mut next.nodes[..next.count], &elements[..self.elements.len()], [0.0; 3], dt, 8, 0.98)
            .map_err(|_| Error::InvalidCommand)?;
        let mut frame = FemcFrame { determinant_min: f64::INFINITY, determinant_max: f64::NEG_INFINITY, enthesis_drift_m: 0.0 };
        for (slot, element) in self.elements.iter().enumerate() {
            let corners = element.indices.map(|i| next.nodes[usize::from(i)].pos);
            let determinant = signed_six_volume(corners) / (6.0 * element.rest_volume_m3);
            if !(0.85..=1.15).contains(&determinant) { return Err(Error::FemcVolumeResidual { tet: slot, determinant }); }
            frame.determinant_min = frame.determinant_min.min(determinant);
            frame.determinant_max = frame.determinant_max.max(determinant);
        }
        for (moved, reference) in next.nodes[..next.count].iter().zip(&self.initial.nodes[..self.initial.count]) {
            if reference.pinned {
                let drift = length(sub(moved.pos, reference.pos));
                if !drift.is_finite() || drift > FIT_TOLERANCE_M { return Err(Error::FemcEnthesisResidual { metres: drift }); }
                frame.enthesis_drift_m = frame.enthesis_drift_m.max(drift);
            }
        }
        *state = next;
        Ok(frame)
    }

    /// Interpolate the posed native cage into caller-owned surface vertices.
    pub fn deform(&self, state: &FemcState, output: &mut [[f32; 3]]) -> Result<(), Error> {
        if state.cage_id != self.initial.cage_id { return Err(Error::UnboundTarget); }
        let mut corners = [[0.0; 3]; MAX_NODES];
        for (dst, node) in corners[..state.count].iter_mut().zip(&state.nodes[..state.count]) {
            *dst = node.pos.map(|v| v as f32);
            if dst.iter().any(|v| !v.is_finite()) { return Err(Error::InvalidCommand); }
        }
        deform_surface_vertices_zero_heap(&self.bindings, &self.cells, &corners[..state.count], output)
            .map(|_| ()).map_err(|_| Error::InvalidFit)
    }
}

fn signed_six_volume([a, b, c, d]: [[f64; 3]; 4]) -> f64 {
    sub(b, a).into_iter().zip(cross(sub(c, a), sub(d, a))).map(|(x, y)| x * y).sum()
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] { std::array::from_fn(|axis| a[axis] - b[axis]) }
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn length(vector: [f64; 3]) -> f64 { vector.into_iter().map(|v| v * v).sum::<f64>().sqrt() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn femc_rest_and_volume_gate_keep_failed_tetra_state_unpublished() {
        let rest = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let mut nodes = [VbdVertex::default(); MAX_NODES];
        for (slot, position) in rest.into_iter().enumerate() {
            nodes[slot] = VbdVertex { pos: position, prev_pos: position, mass: 1.0, color: slot as u8,
                pinned: slot < 3, ..Default::default() };
        }
        let [a, b, c, d] = rest;
        let element = VbdTetElement::from_rest_positions([0, 1, 2, 3], a, b, c, d).unwrap();
        let model = PreparedFemcMotion {
            initial: FemcState { nodes, count: 4, cage_id: [0; 32] },
            elements: Box::new([element]), cells: Box::new([[0, 1, 2, 3]]),
            bindings: Box::new([VertexTetBinding::from_barycentric(0, [0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 1.0])]),
        };
        let mut output = [[0.0; 3]; 1];
        model.rest_surface(&[[0.0, 0.0, 1.0]], &mut output).unwrap();
        assert_eq!(output, [[0.0, 0.0, 1.0]]);
        assert!(matches!(model.rest_surface(&[[0.0, 0.0, 1.01]], &mut output), Err(Error::FemcRestResidual { .. })));
        let mut state = model.initial_state();
        let solved = model.advance(&mut state, 0.005, 0.0).unwrap();
        assert!((solved.determinant_min - 1.0).abs() < 1e-9);
        assert_eq!(solved.enthesis_drift_m, 0.0);
        state.nodes[3].pos[2] = 2.0;
        let previous = state.nodes;
        assert!(matches!(model.advance(&mut state, 1e-7, 0.0), Err(Error::FemcVolumeResidual { tet: 0, .. })));
        assert_eq!(state.nodes, previous);
    }
}
