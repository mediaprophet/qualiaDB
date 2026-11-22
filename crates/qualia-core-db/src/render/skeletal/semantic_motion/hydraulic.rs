//! Conservative pressure-driven network transport and turgor-to-motor coupling.

use crate::{q42::m64_matter::{M64DecodedRecord, M64Reader}, q_hash};

use super::{FittedRotation, MotionGraph, SemanticAction, SemanticMotionError as Error, SemanticRotation,
    fitted_rotation, graph::{index, number}};

const MAX_NODES: usize = 32;
pub const MAX_HYDRAULIC_EDGES: usize = 96;
const MAX_EDGES: usize = MAX_HYDRAULIC_EDGES;

#[derive(Clone, Copy)]
struct Node {
    id: u64,
    compliance_m3_per_pa: f64,
    neutral_pressure_pa: f64,
    pressure_gain_rad_per_pa: f64,
    fit: Option<FittedRotation>,
}

#[derive(Clone, Copy)]
struct Edge {
    id: u64,
    from: usize,
    to: usize,
    radius_m: f64,
    viscosity_pa_s: f64,
    conductance_m3_per_pa_s: f64,
    psi50_pa: f64,
    slope_pa: f64,
}

pub struct PreparedHydraulicMotion {
    nodes: [Option<Node>; MAX_NODES],
    edges: [Option<Edge>; MAX_EDGES],
    node_count: usize,
    edge_count: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HydraulicEdgeStep {
    pub semantic_id: u64,
    pub flow_m3_s: f64,
    pub wall_shear_pa: f64,
    pub conductance_fraction: f64,
}

pub struct HydraulicStep {
    pub storage_change_m3: f64,
    pub supplied_volume_m3: f64,
    pub mass_residual_m3: f64,
}

macro_rules! k {
    ($name:literal) => { q_hash(concat!("https://qualiadb.org/ns/kinematics#", $name)) };
}

/// Admit a reversible hydraulic graph. Node volumes and fit gains are Q42
/// quantities. Effective compliance uses the M64 longitudinal tissue modulus
/// under the declared small-strain constitutive approximation.
pub fn prepare_hydraulic_motion(
    graph: &mut MotionGraph<'_, '_, '_>, action: &SemanticAction<'_>, node_iris: &[&str], edge_iris: &[&str],
    material_iri: &str, material: &M64Reader<'_>,
) -> Result<PreparedHydraulicMotion, Error> {
    graph.check_budget(core::mem::size_of::<PreparedHydraulicMotion>())?;
    let controller = action.authorize(graph)?;
    let source = graph.one(controller, k!("materialResource"))?;
    if graph.iri(source)? != material_iri { return Err(Error::InvalidMaterial); }
    let node_count = index(graph.one(controller, k!("hydraulicNodeCount"))?)?;
    let edge_count = index(graph.one(controller, k!("hydraulicEdgeCount"))?)?;
    if !(2..=MAX_NODES).contains(&node_count) || !(1..=MAX_EDGES).contains(&edge_count)
        || node_iris.len() != node_count || edge_iris.len() != edge_count
    {
        return Err(Error::InvalidCommand);
    }
    let mut prepared = PreparedHydraulicMotion {
        nodes: [None; MAX_NODES], edges: [None; MAX_EDGES], node_count, edge_count,
    };
    for (slot, iri) in node_iris.iter().enumerate() {
        let id = q_hash(iri);
        if graph.iri(id)? != *iri || graph.one(id, k!("hydraulicAction"))? != controller
            || index(graph.one(id, k!("hydraulicNodeSlot"))?)? != slot
        { return Err(Error::InvalidFact); }
        let [volume, neutral_pressure_pa] = graph.numeric_lanes(id, [k!("volumeM3"), k!("referencePressurePa")])?;
        let chunk = index(graph.one(id, k!("materialChunk"))?)?;
        let record = index(graph.one(id, k!("materialRecord"))?)?;
        let Some(M64DecodedRecord::BioTissue(tissue)) = material.chunk(chunk).ok_or(Error::InvalidMaterial)?
            .decode_record(record).map_err(|_| Error::InvalidMaterial)? else { return Err(Error::InvalidMaterial); };
        let compliance_m3_per_pa = volume / (1e6 * f64::from(tissue.elastic_e1_mpa()));
        if !compliance_m3_per_pa.is_finite() || compliance_m3_per_pa <= 0.0 { return Err(Error::InvalidMaterial); }
        let (fit, pressure_gain_rad_per_pa) = match graph.one(id, k!("motionTarget")) {
            Ok(target) => {
                graph.iri(target)?;
                let fit = graph.fit(target)?;
                if prepared.nodes.iter().flatten().any(|node| node.fit.is_some_and(|other| other.semantic_id == target)) {
                    return Err(Error::AmbiguousTarget);
                }
                (Some(fit), number(graph.one(id, k!("pressureGainRadiansPerPascal"))?)?)
            }
            Err(Error::MissingFact) => (None, 0.0),
            Err(error) => return Err(error),
        };
        prepared.nodes[slot] = Some(Node { id, compliance_m3_per_pa, neutral_pressure_pa, pressure_gain_rad_per_pa, fit });
    }
    for (slot, iri) in edge_iris.iter().enumerate() {
        let id = q_hash(iri);
        if graph.iri(id)? != *iri || graph.one(id, k!("hydraulicAction"))? != controller
            || index(graph.one(id, k!("hydraulicEdgeSlot"))?)? != slot
        { return Err(Error::InvalidFact); }
        let from = graph.one(id, k!("fromNode"))?;
        let to = graph.one(id, k!("toNode"))?;
        let resolve = |hash| prepared.nodes[..node_count].iter().position(|node| node.is_some_and(|n| n.id == hash));
        let from = resolve(from).ok_or(Error::UnknownIdentity)?;
        let to = resolve(to).ok_or(Error::UnknownIdentity)?;
        let [radius_m, length, viscosity_pa_s, psi50_pa, slope_pa] = graph.numeric_lanes(id, [
            k!("lumenRadiusMetres"), k!("lengthMetres"), k!("dynamicViscosityPascalSeconds"),
            k!("psi50Pascals"), k!("cavitationSlopePascals"),
        ])?;
        if from == to || ![radius_m, length, viscosity_pa_s, slope_pa].iter().all(|v| *v > 0.0) {
            return Err(Error::InvalidFact);
        }
        let conductance_m3_per_pa_s = std::f64::consts::PI * radius_m.powi(4) / (8.0 * viscosity_pa_s * length);
        if !conductance_m3_per_pa_s.is_finite() || conductance_m3_per_pa_s <= 0.0 { return Err(Error::InvalidFact); }
        prepared.edges[slot] = Some(Edge { id, from, to, radius_m, viscosity_pa_s, conductance_m3_per_pa_s, psi50_pa, slope_pa });
    }
    Ok(prepared)
}

impl PreparedHydraulicMotion {
    pub fn node_count(&self) -> usize { self.node_count }
    pub fn edge_count(&self) -> usize { self.edge_count }

    /// Admitted Q42 reference pressures in pascals, in node slot order.
    pub fn reference_pressures(&self) -> impl Iterator<Item = f64> + '_ {
        self.nodes[..self.node_count].iter().flatten().map(|node| node.neutral_pressure_pa)
    }

    pub fn motion_target_ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.nodes[..self.node_count].iter().flatten().filter_map(|node| node.fit.map(|fit| fit.semantic_id))
    }

    /// One linearly implicit Poiseuille step. Pressures are Pa, external flows
    /// are signed m³/s. Cavitation conductances are evaluated at the beginning
    /// of the step. Interior edge fluxes cancel pairwise in the storage check.
    pub fn advance(
        &self, dt_s: f64, pressure_pa: &mut [f64], source_m3_s: &[f64],
        flow: &mut [HydraulicEdgeStep], motors: &mut [Option<SemanticRotation>],
    ) -> Result<HydraulicStep, Error> {
        let n = self.node_count;
        let m = self.edge_count;
        if pressure_pa.len() < n || source_m3_s.len() < n || flow.len() < m || motors.len() < n {
            return Err(Error::OutputTooSmall);
        }
        if !dt_s.is_finite() || dt_s <= 0.0 || pressure_pa[..n].iter().chain(&source_m3_s[..n]).any(|v| !v.is_finite()) {
            return Err(Error::InvalidCommand);
        }
        let mut matrix = [[0.0; MAX_NODES]; MAX_NODES];
        let mut next = [0.0; MAX_NODES];
        let mut fractions = [0.0; MAX_EDGES];
        for (i, node) in self.nodes[..n].iter().flatten().enumerate() {
            matrix[i][i] = node.compliance_m3_per_pa;
            next[i] = node.compliance_m3_per_pa * pressure_pa[i] + dt_s * source_m3_s[i];
        }
        for (i, edge) in self.edges[..m].iter().flatten().enumerate() {
            let mean = 0.5 * pressure_pa[edge.from] + 0.5 * pressure_pa[edge.to];
            let fraction = 1.0 / (1.0 + ((edge.psi50_pa - mean) / edge.slope_pa).clamp(-60.0, 60.0).exp());
            let coupling = dt_s * fraction * edge.conductance_m3_per_pa_s;
            if !coupling.is_finite() { return Err(Error::InvalidCommand); }
            matrix[edge.from][edge.from] += coupling;
            matrix[edge.to][edge.to] += coupling;
            matrix[edge.from][edge.to] -= coupling;
            matrix[edge.to][edge.from] -= coupling;
            fractions[i] = fraction;
        }
        cholesky(&mut matrix, &mut next, n)?;
        let mut staged_motors = [None; MAX_NODES];
        let mut staged_flow = [HydraulicEdgeStep::default(); MAX_EDGES];
        let mut storage = 0.0;
        let mut supplied = 0.0;
        let mut magnitude = 0.0;
        for (i, node) in self.nodes[..n].iter().flatten().enumerate() {
            storage += node.compliance_m3_per_pa * (next[i] - pressure_pa[i]);
            supplied += source_m3_s[i] * dt_s;
            magnitude += node.compliance_m3_per_pa * (next[i].abs() + pressure_pa[i].abs()) + (dt_s * source_m3_s[i]).abs();
            if let Some(fit) = &node.fit {
                let desired = (next[i] - node.neutral_pressure_pa) * node.pressure_gain_rad_per_pa;
                if !desired.is_finite() { return Err(Error::InvalidCommand); }
                staged_motors[i] = Some(fitted_rotation(fit, desired)?);
            }
        }
        for (i, edge) in self.edges[..m].iter().flatten().enumerate() {
            let q = fractions[i] * edge.conductance_m3_per_pa_s * (next[edge.from] - next[edge.to]);
            let shear = 4.0 * edge.viscosity_pa_s * q.abs() / (std::f64::consts::PI * edge.radius_m.powi(3));
            if !q.is_finite() || !shear.is_finite() { return Err(Error::InvalidCommand); }
            staged_flow[i] = HydraulicEdgeStep { semantic_id: edge.id, flow_m3_s: q, wall_shear_pa: shear, conductance_fraction: fractions[i] };
        }
        let residual = storage - supplied;
        if !residual.is_finite() || !magnitude.is_finite() || residual.abs() > 1e-10 * magnitude {
            return Err(Error::InvalidCommand);
        }
        pressure_pa[..n].copy_from_slice(&next[..n]);
        motors[..n].copy_from_slice(&staged_motors[..n]);
        flow[..m].copy_from_slice(&staged_flow[..m]);
        Ok(HydraulicStep { storage_change_m3: storage, supplied_volume_m3: supplied, mass_residual_m3: residual })
    }
}

fn cholesky(matrix: &mut [[f64; MAX_NODES]; MAX_NODES], rhs: &mut [f64; MAX_NODES], n: usize) -> Result<(), Error> {
    for i in 0..n {
        for j in 0..=i {
            let remainder = matrix[i][j] - (0..j).map(|k| matrix[i][k] * matrix[j][k]).sum::<f64>();
            if i == j {
                if !remainder.is_finite() || remainder <= 0.0 { return Err(Error::InvalidCommand); }
                matrix[i][j] = remainder.sqrt();
            } else {
                matrix[i][j] = remainder / matrix[j][j];
            }
        }
        rhs[i] = (rhs[i] - (0..i).map(|j| matrix[i][j] * rhs[j]).sum::<f64>()) / matrix[i][i];
    }
    for i in (0..n).rev() {
        rhs[i] = (rhs[i] - ((i + 1)..n).map(|j| matrix[j][i] * rhs[j]).sum::<f64>()) / matrix[i][i];
        if !rhs[i].is_finite() { return Err(Error::InvalidCommand); }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implicit_flux_goes_down_pressure_and_conserves_storage() {
        let a = Node { id: 1, compliance_m3_per_pa: 1e-9, neutral_pressure_pa: 0.0, pressure_gain_rad_per_pa: 0.0, fit: None };
        let mut network = PreparedHydraulicMotion { nodes: [None; MAX_NODES], edges: [None; MAX_EDGES], node_count: 2, edge_count: 1 };
        network.nodes[0] = Some(a);
        network.nodes[1] = Some(Node { id: 2, ..a });
        network.edges[0] = Some(Edge { id: 3, from: 0, to: 1, radius_m: 0.001, viscosity_pa_s: 0.001,
            conductance_m3_per_pa_s: 1e-8, psi50_pa: -1e6, slope_pa: 1e4 });
        let mut pressure = [100.0, 0.0];
        let mut flow = [HydraulicEdgeStep::default()];
        let mut motors = [None; 2];
        let step = network.advance(1.0, &mut pressure, &[0.0; 2], &mut flow, &mut motors).unwrap();
        assert!(pressure[0] < 100.0 && pressure[0] > pressure[1]);
        assert!(flow[0].flow_m3_s > 0.0 && flow[0].wall_shear_pa > 0.0);
        assert!(step.mass_residual_m3.abs() < 1e-18);
        assert!((pressure[0] + pressure[1] - 100.0).abs() < 1e-10);
    }
}
