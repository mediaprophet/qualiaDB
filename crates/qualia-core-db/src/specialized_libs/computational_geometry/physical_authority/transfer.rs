//! Visual ↔ authoritative transfer on existing FEMC surface-to-tetra bindings.

use crate::container_10d::correspondence_section::{
    deform_surface_vertices_zero_heap, VertexTetBinding, BARYCENTRIC_UNORM16_SUM,
};

use super::super::geometry_workspace::production_budget::ProductionBudget;
use super::super::primitives::Point3;
use super::super::production_contracts::{
    ContentId, FieldSemantics, ReasonCode, ResourceKind, ResultStatus, TransferContract,
    TransferRule,
};
use super::state::{
    charge_work, require_capacity, solver_topology_preserved, AuthorityDeclaration, ClaimId,
    EvidenceMethod, MeshAuthority, PhysicalError, PhysicalEvidence, UnitsDeclaration,
};

/// One nonzero FEMC weight. Not a new `.10d` section; derived from `VertexTetBinding`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FemcMapEntry {
    pub visual_vertex: u32,
    pub solver_vertex: u32,
    pub weight: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FemcTransferPlan<'a> {
    pub solver: MeshAuthority,
    pub visual: MeshAuthority,
    pub bindings: &'a [VertexTetBinding],
    pub tets: &'a [[u32; 4]],
}

impl<'a> FemcTransferPlan<'a> {
    pub fn validate(&self) -> Result<(), PhysicalError> {
        self.solver.validate()?;
        self.visual.validate()?;
        if self.bindings.len() != self.visual.vertex_count as usize {
            return Err(PhysicalError::InvalidInput("femc_visual_count"));
        }
        if self.tets.len() != self.solver.cell_or_face_count as usize {
            return Err(PhysicalError::InvalidInput("femc_tet_count"));
        }
        for b in self.bindings {
            let tet = b.tet_element_index as usize;
            if tet >= self.tets.len() {
                return Err(PhysicalError::InvalidInput("femc_tet_index"));
            }
            let sum: u32 = b.barycentric_unorm16.iter().map(|w| *w as u32).sum();
            if sum != BARYCENTRIC_UNORM16_SUM {
                return Err(PhysicalError::InvalidInput("femc_barycentric_sum"));
            }
            for &v in &self.tets[tet] {
                if v >= self.solver.vertex_count {
                    return Err(PhysicalError::InvalidInput("femc_solver_vertex"));
                }
            }
        }
        Ok(())
    }
}

pub fn barycentric_f64(binding: &VertexTetBinding) -> [f64; 4] {
    const INV: f64 = 1.0 / BARYCENTRIC_UNORM16_SUM as f64;
    [
        binding.barycentric_unorm16[0] as f64 * INV,
        binding.barycentric_unorm16[1] as f64 * INV,
        binding.barycentric_unorm16[2] as f64 * INV,
        binding.barycentric_unorm16[3] as f64 * INV,
    ]
}

/// unorm16 weight error plus f32 FEMC codec rounding, scaled by length unit.
pub fn femc_position_tolerance(units: UnitsDeclaration) -> f64 {
    (4.0 / BARYCENTRIC_UNORM16_SUM as f64 + 1.0e-6) * units.length_unit_metres
}

pub fn copy_points_f32(src: &[Point3], dst: &mut [[f32; 3]]) -> Result<usize, PhysicalError> {
    require_capacity(src.len(), dst.len())?;
    for (i, p) in src.iter().enumerate() {
        if ![p.x, p.y, p.z].iter().all(|x| x.is_finite()) {
            return Err(PhysicalError::InvalidInput("position_nonfinite"));
        }
        dst[i] = [p.x as f32, p.y as f32, p.z as f32];
    }
    Ok(src.len())
}

/// Export positive FEMC weights as COO (visual row, solver column), caller-buffered.
pub fn femc_to_map_entries(
    plan: &FemcTransferPlan<'_>,
    out: &mut [FemcMapEntry],
    budget: &mut ProductionBudget<'_>,
) -> Result<usize, PhysicalError> {
    plan.validate()?;
    charge_work(budget, &[plan.bindings.len().max(1), 4, 2])?;
    let mut n = 0usize;
    for (vi, binding) in plan.bindings.iter().enumerate() {
        let tet = plan.tets[binding.tet_element_index as usize];
        let w = barycentric_f64(binding);
        for k in 0..4 {
            if w[k] > 0.0 {
                require_capacity(n + 1, out.len())?;
                out[n] = FemcMapEntry {
                    visual_vertex: vi as u32,
                    solver_vertex: tet[k],
                    weight: w[k],
                };
                n += 1;
            }
        }
    }
    budget.checkpoint(0)?;
    Ok(n)
}

/// Authoritative tet positions → visual surface (FEMC interpolation).
pub fn interpolate_visual_positions(
    plan: &FemcTransferPlan<'_>,
    tet_positions: &[Point3],
    visual_out: &mut [[f64; 3]],
    scratch: &mut [[f64; 3]],
    budget: &mut ProductionBudget<'_>,
) -> Result<usize, PhysicalError> {
    plan.validate()?;
    if tet_positions.len() != plan.solver.vertex_count as usize {
        return Err(PhysicalError::InvalidInput("solver_vertex_count"));
    }
    let n = plan.bindings.len();
    require_capacity(n, visual_out.len())?;
    require_capacity(n, scratch.len())?;
    charge_work(budget, &[n.max(1), 12])?;
    for p in tet_positions {
        if ![p.x, p.y, p.z].iter().all(|x| x.is_finite()) {
            return Err(PhysicalError::InvalidInput("position_nonfinite"));
        }
    }
    scratch[..n].fill([0.0; 3]);
    for (i, binding) in plan.bindings.iter().enumerate() {
        let tet = plan.tets[binding.tet_element_index as usize];
        let w = barycentric_f64(binding);
        for k in 0..4 {
            let p = tet_positions[tet[k] as usize];
            scratch[i][0] += w[k] * p.x;
            scratch[i][1] += w[k] * p.y;
            scratch[i][2] += w[k] * p.z;
        }
        if scratch[i].iter().any(|x| !x.is_finite()) {
            return Err(PhysicalError::InvalidInput("interpolation_overflow"));
        }
    }
    budget.checkpoint(0)?;
    visual_out[..n].copy_from_slice(&scratch[..n]);
    Ok(n)
}

/// Existing f32 FEMC codec path. Scratch is the codec output buffer.
pub fn interpolate_visual_femc_f32(
    plan: &FemcTransferPlan<'_>,
    tet_positions_f32: &[[f32; 3]],
    visual_out: &mut [[f32; 3]],
    scratch: &mut [[f32; 3]],
    budget: &mut ProductionBudget<'_>,
) -> Result<usize, PhysicalError> {
    plan.validate()?;
    if tet_positions_f32.len() != plan.solver.vertex_count as usize {
        return Err(PhysicalError::InvalidInput("solver_vertex_count"));
    }
    let n = plan.bindings.len();
    require_capacity(n, visual_out.len())?;
    require_capacity(n, scratch.len())?;
    charge_work(budget, &[n.max(1), 12])?;
    deform_surface_vertices_zero_heap(plan.bindings, plan.tets, tet_positions_f32, scratch)
        .map_err(|_| PhysicalError::Correspondence)?;
    if scratch[..n].iter().flatten().any(|x| !x.is_finite()) {
        return Err(PhysicalError::InvalidInput("femc_deformation_overflow"));
    }
    budget.checkpoint(0)?;
    visual_out[..n].copy_from_slice(&scratch[..n]);
    Ok(n)
}

/// Visual forces → solver forces via Jᵀ of the FEMC interpolation map.
pub fn scatter_visual_forces_to_solver(
    plan: &FemcTransferPlan<'_>,
    visual_forces: &[[f64; 3]],
    solver_forces_out: &mut [[f64; 3]],
    scratch: &mut [[f64; 3]],
    budget: &mut ProductionBudget<'_>,
) -> Result<usize, PhysicalError> {
    plan.validate()?;
    let nv = plan.bindings.len();
    let ns = plan.solver.vertex_count as usize;
    if visual_forces.len() != nv {
        return Err(PhysicalError::InvalidInput("visual_force_count"));
    }
    require_capacity(ns, solver_forces_out.len())?;
    require_capacity(ns, scratch.len())?;
    charge_work(budget, &[nv.max(1), 12])?;
    if visual_forces.iter().flatten().any(|x| !x.is_finite()) {
        return Err(PhysicalError::InvalidInput("force_nonfinite"));
    }
    scratch[..ns].fill([0.0; 3]);
    for (i, binding) in plan.bindings.iter().enumerate() {
        let tet = plan.tets[binding.tet_element_index as usize];
        let w = barycentric_f64(binding);
        for k in 0..4 {
            let s = tet[k] as usize;
            scratch[s][0] += w[k] * visual_forces[i][0];
            scratch[s][1] += w[k] * visual_forces[i][1];
            scratch[s][2] += w[k] * visual_forces[i][2];
        }
    }
    if scratch[..ns].iter().flatten().any(|x| !x.is_finite()) {
        return Err(PhysicalError::InvalidInput("force_overflow"));
    }
    budget.checkpoint(0)?;
    solver_forces_out[..ns].copy_from_slice(&scratch[..ns]);
    Ok(ns)
}

/// FEMC does not define a unique visual-displacement → tet-displacement restriction.
pub fn transfer_visual_displacement_to_solver() -> PhysicalError {
    PhysicalError::Unsupported("femc_is_surface_to_tetra_only")
}

pub fn virtual_work_residual(
    visual_forces: &[[f64; 3]],
    visual_displacements: &[[f64; 3]],
    solver_forces: &[[f64; 3]],
    solver_displacements: &[[f64; 3]],
) -> Result<f64, PhysicalError> {
    if visual_forces.len() != visual_displacements.len()
        || solver_forces.len() != solver_displacements.len()
    {
        return Err(PhysicalError::InvalidInput("virtual_work_dimensions"));
    }
    let lhs = dot3_pairs(visual_forces, visual_displacements)?;
    let rhs = dot3_pairs(solver_forces, solver_displacements)?;
    let residual = (lhs - rhs).abs();
    if !residual.is_finite() {
        return Err(PhysicalError::InvalidInput("virtual_work_nonfinite"));
    }
    Ok(residual)
}

fn dot3_pairs(a: &[[f64; 3]], b: &[[f64; 3]]) -> Result<f64, PhysicalError> {
    let mut acc = 0.0;
    for (fa, ua) in a.iter().zip(b) {
        acc += fa[0] * ua[0] + fa[1] * ua[1] + fa[2] * ua[2];
    }
    if !acc.is_finite() {
        Err(PhysicalError::InvalidInput("virtual_work_overflow"))
    } else {
        Ok(acc)
    }
}

pub fn virtual_work_evidence(
    residual: f64,
    scale: f64,
    atol: f64,
    rtol: f64,
) -> Result<PhysicalEvidence, PhysicalError> {
    if ![residual, scale, atol, rtol]
        .iter()
        .all(|x| x.is_finite() && *x >= 0.0)
    {
        return Err(PhysicalError::InvalidInput("virtual_work_tolerance"));
    }
    if scale == 0.0 && residual == 0.0 {
        return Ok(PhysicalEvidence {
            status: ResultStatus::Passed,
            claim: ClaimId::VirtualWorkIdentity,
            method: EvidenceMethod::JacobianTransposeIdentity,
            value: Some(0.0),
            units: "J",
            reason: "both virtual-work sides are zero",
        });
    }
    if scale == 0.0 {
        return Err(PhysicalError::Inconclusive("virtual_work_scale"));
    }
    let ok = residual <= atol || residual <= rtol * scale;
    Ok(PhysicalEvidence {
        status: if ok {
            ResultStatus::Passed
        } else {
            ResultStatus::Failed
        },
        claim: ClaimId::VirtualWorkIdentity,
        method: EvidenceMethod::JacobianTransposeIdentity,
        value: Some(residual),
        units: "J",
        reason: "serial f64 <f_vis, P u> vs <P^T f_vis, u>",
    })
}

pub fn rest_or_deformed_transfer_evidence(
    claim: ClaimId,
    predicted: &[[f64; 3]],
    observed: &[[f64; 3]],
    absolute_tolerance: f64,
) -> Result<PhysicalEvidence, PhysicalError> {
    if claim != ClaimId::RestStateTransfer && claim != ClaimId::DeformedStateTransfer {
        return Err(PhysicalError::InvalidInput("transfer_claim"));
    }
    if predicted.len() != observed.len() {
        return Err(PhysicalError::InvalidInput("transfer_dimensions"));
    }
    if !(absolute_tolerance >= 0.0 && absolute_tolerance.is_finite()) {
        return Err(PhysicalError::InvalidInput("transfer_tolerance"));
    }
    let mut max_err = 0.0f64;
    for (a, b) in predicted.iter().zip(observed) {
        let e = (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]);
        if !e.is_finite() {
            return Err(PhysicalError::InvalidInput("transfer_nonfinite"));
        }
        max_err = max_err.max(e);
    }
    Ok(PhysicalEvidence {
        status: if max_err <= absolute_tolerance {
            ResultStatus::Passed
        } else {
            ResultStatus::Failed
        },
        claim,
        method: EvidenceMethod::FemcBarycentricInterpolation,
        value: Some(max_err),
        units: "m",
        reason: "barycentric interpolation of tet vertices versus visual positions",
    })
}

pub fn prescribed_response_evidence(
    observed: f64,
    reference: f64,
    rms_fraction: f64,
    max_fraction: f64,
) -> Result<PhysicalEvidence, PhysicalError> {
    if ![observed, reference, rms_fraction, max_fraction]
        .iter()
        .all(|x| x.is_finite())
        || reference == 0.0
        || rms_fraction < 0.0
        || max_fraction < 0.0
    {
        return Err(PhysicalError::InvalidInput("prescribed_response_scale"));
    }
    let rel = (observed - reference).abs() / reference.abs();
    Ok(PhysicalEvidence {
        status: if rel <= rms_fraction.max(max_fraction) {
            ResultStatus::Passed
        } else {
            ResultStatus::Failed
        },
        claim: ClaimId::PrescribedResponse,
        method: EvidenceMethod::ConnectedPbdVolumeProjection,
        value: Some(rel),
        units: "1",
        reason: "relative error against the declared single-tet volume/height reference",
    })
}

pub fn lod_topology_evidence(
    before: &AuthorityDeclaration,
    after: &AuthorityDeclaration,
) -> PhysicalEvidence {
    match solver_topology_preserved(before, after) {
        Ok(()) => PhysicalEvidence {
            status: ResultStatus::Passed,
            claim: ClaimId::SolverTopologyPreservedUnderLod,
            method: EvidenceMethod::AuthorityIdentityCompare,
            value: Some(after.solver_mesh.topology_generation as f64),
            units: "1",
            reason: "visual LOD identity changed; solver topology identity did not",
        },
        Err(_) => PhysicalEvidence {
            status: ResultStatus::Failed,
            claim: ClaimId::SolverTopologyPreservedUnderLod,
            method: EvidenceMethod::AuthorityIdentityCompare,
            value: None,
            units: "1",
            reason: "visual LOD mutated authoritative solver topology",
        },
    }
}

pub fn validate_femc_displacement_contract(
    contract: &TransferContract,
    solver: ContentId,
    visual: ContentId,
) -> Result<(), PhysicalError> {
    if contract.source != solver || contract.target != visual {
        return Err(PhysicalError::IdentityMismatch);
    }
    if contract.rule != TransferRule::Interpolation
        || contract.semantics != FieldSemantics::Displacement
        || contract.support.kind != ResourceKind::FemcBindings
    {
        return Err(PhysicalError::Unsupported("femc_displacement_contract"));
    }
    Ok(())
}

pub fn validate_femc_force_contract(
    contract: &TransferContract,
    visual: ContentId,
    solver: ContentId,
) -> Result<(), PhysicalError> {
    if contract.source != visual || contract.target != solver {
        return Err(PhysicalError::IdentityMismatch);
    }
    if contract.rule != TransferRule::JacobianTranspose
        || contract.semantics != FieldSemantics::Force
        || contract.support.kind != ResourceKind::FemcBindings
    {
        return Err(PhysicalError::Unsupported("femc_force_contract"));
    }
    Ok(())
}

pub fn physical_error_status(err: PhysicalError) -> (ResultStatus, ReasonCode) {
    (err.status(), err.reason_code())
}
