//! Typed constitutive, load, time, units and authority records.

use serde::{Deserialize, Serialize};

use super::super::geometry_workspace::production_budget::{BudgetError, ProductionBudget};
use super::super::pbd_softbody::{pbd_step, PbdConstraint, PbdError, PbdReceipt};
use super::super::primitives::Point3;
use super::super::production_contracts::{
    ContentId, ReasonCode, RepresentationRole, ResultStatus,
};

/// SI conversion factors. Force must equal mass·length/time².
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitsDeclaration {
    pub length_unit_metres: f64,
    pub mass_unit_kilograms: f64,
    pub time_unit_seconds: f64,
    pub force_unit_newtons: f64,
    pub right_handed: bool,
}

impl UnitsDeclaration {
    pub const SI: Self = Self {
        length_unit_metres: 1.0,
        mass_unit_kilograms: 1.0,
        time_unit_seconds: 1.0,
        force_unit_newtons: 1.0,
        right_handed: true,
    };

    pub fn validate(self) -> Result<(), PhysicalError> {
        let fields = [
            self.length_unit_metres,
            self.mass_unit_kilograms,
            self.time_unit_seconds,
            self.force_unit_newtons,
        ];
        if fields.iter().any(|x| !x.is_finite() || *x <= 0.0) {
            return Err(PhysicalError::InvalidInput("units_scale"));
        }
        let derived = self.mass_unit_kilograms * self.length_unit_metres
            / (self.time_unit_seconds * self.time_unit_seconds);
        let scale = derived.abs().max(self.force_unit_newtons.abs()).max(1.0e-18);
        if (derived - self.force_unit_newtons).abs() > 1.0e-12 * scale {
            return Err(PhysicalError::InvalidInput("force_unit_inconsistent"));
        }
        Ok(())
    }
}

/// Constitutive family actually executed by the scoped kernel.
/// Linear elasticity, Neo-Hookean force density and reduced-order models are
/// not implemented here (optional research R03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstitutiveFamily {
    /// Hard distance + tetrahedral-volume projection on a shared-vertex mesh.
    PbdDistanceAndVolume,
    /// Same projection with four unshared vertices per tetrahedron.
    IndependentTetrahedronPbdVolume,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstitutiveDeclaration {
    pub family: ConstitutiveFamily,
    pub density_kg_per_m3: f64,
    /// Recorded XPBD compliance. Non-zero is unsupported: `pbd_softbody` is hard.
    pub volume_compliance: f64,
    pub distance_compliance: f64,
}

impl ConstitutiveDeclaration {
    pub fn validate(self) -> Result<(), PhysicalError> {
        if !self.density_kg_per_m3.is_finite() || self.density_kg_per_m3 <= 0.0 {
            return Err(PhysicalError::InvalidInput("density"));
        }
        if !self.volume_compliance.is_finite() || !self.distance_compliance.is_finite() {
            return Err(PhysicalError::InvalidInput("compliance"));
        }
        if self.volume_compliance != 0.0 || self.distance_compliance != 0.0 {
            return Err(PhysicalError::Unsupported("xpbd_compliance"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum LoadKind {
    Gravity { acceleration_si: [f64; 3] },
    PrescribedDisplacement,
    NodalForce,
    ContactUnsupported,
    FrictionUnsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadDeclaration {
    pub kind: LoadKind,
}

impl LoadDeclaration {
    pub fn validate(self) -> Result<(), PhysicalError> {
        match self.kind {
            LoadKind::Gravity { acceleration_si } => {
                if acceleration_si.iter().any(|x| !x.is_finite()) {
                    Err(PhysicalError::InvalidInput("gravity"))
                } else {
                    Ok(())
                }
            }
            LoadKind::PrescribedDisplacement | LoadKind::NodalForce => Ok(()),
            LoadKind::ContactUnsupported => Err(PhysicalError::Unsupported("contact_load")),
            LoadKind::FrictionUnsupported => Err(PhysicalError::Unsupported("friction_load")),
        }
    }
}

/// Integrators this packet will actually run. Implicit Newmark/IPC are unsupported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegratorKind {
    /// Sequential shared-vertex Gauss–Seidel. Order-dependent; not race-free.
    ConnectedGaussSeidelPbd,
    /// Unshared tetrahedron vertices; no assembled coupling.
    IndependentTetrahedronPbd,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeDeclaration {
    pub integrator: IntegratorKind,
    pub timestep_seconds: f64,
    pub substeps: u32,
    pub iterations_per_substep: u32,
    /// Compared to `PbdReceipt.max_residual` (max constraint error, not ||b−Ax||).
    pub solve_tolerance: f64,
    pub damping: f64,
}

impl TimeDeclaration {
    pub fn validate(self) -> Result<(), PhysicalError> {
        if !(self.timestep_seconds > 0.0 && self.timestep_seconds.is_finite())
            || self.substeps == 0
            || self.iterations_per_substep == 0
            || !(self.solve_tolerance >= 0.0 && self.solve_tolerance.is_finite())
            || !(self.damping >= 0.0 && self.damping <= 1.0)
        {
            return Err(PhysicalError::InvalidInput("time_declaration"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshAuthority {
    pub identity: ContentId,
    pub role: RepresentationRole,
    pub vertex_count: u32,
    pub cell_or_face_count: u32,
    pub topology_generation: u64,
}

impl MeshAuthority {
    pub fn validate(self) -> Result<(), PhysicalError> {
        if self.identity.digest == [0; 32] {
            return Err(PhysicalError::InvalidInput("uninitialised_identity"));
        }
        if self.vertex_count == 0 || self.cell_or_face_count == 0 {
            return Err(PhysicalError::InvalidInput("empty_mesh_authority"));
        }
        Ok(())
    }
}

/// Solver topology is authoritative. Visual LOD is a separate identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityDeclaration {
    pub solver_mesh: MeshAuthority,
    pub visual_mesh: MeshAuthority,
    pub visual_lod_identity: Option<ContentId>,
}

impl AuthorityDeclaration {
    pub fn validate(self) -> Result<(), PhysicalError> {
        self.solver_mesh.validate()?;
        self.visual_mesh.validate()?;
        match self.solver_mesh.role {
            RepresentationRole::Authoritative | RepresentationRole::VolumetricSolver => {}
            _ => return Err(PhysicalError::InvalidInput("solver_role")),
        }
        if self.visual_mesh.role != RepresentationRole::VisualSurface {
            return Err(PhysicalError::InvalidInput("visual_role"));
        }
        if self.solver_mesh.identity == self.visual_mesh.identity {
            return Err(PhysicalError::IdentityMismatch);
        }
        if self
            .visual_lod_identity
            .is_some_and(|id| id == self.solver_mesh.identity)
        {
            return Err(PhysicalError::InvalidInput("lod_replaced_solver_identity"));
        }
        Ok(())
    }

    /// Replace the visual mesh only. Solver counts and generation stay fixed.
    pub fn with_visual_lod(self, visual: MeshAuthority) -> Result<Self, PhysicalError> {
        visual.validate()?;
        if visual.role != RepresentationRole::VisualSurface {
            return Err(PhysicalError::InvalidInput("visual_role"));
        }
        if visual.identity == self.solver_mesh.identity {
            return Err(PhysicalError::InvalidInput("lod_replaced_solver_identity"));
        }
        let next = Self {
            solver_mesh: self.solver_mesh,
            visual_mesh: visual,
            visual_lod_identity: Some(visual.identity),
        };
        next.validate()?;
        Ok(next)
    }
}

pub fn solver_topology_preserved(
    before: &AuthorityDeclaration,
    after: &AuthorityDeclaration,
) -> Result<(), PhysicalError> {
    before.validate()?;
    after.validate()?;
    if before.solver_mesh != after.solver_mesh {
        return Err(PhysicalError::InvalidInput("solver_topology_changed"));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimId {
    RestStateTransfer,
    DeformedStateTransfer,
    VirtualWorkIdentity,
    PrescribedResponse,
    SolverTopologyPreservedUnderLod,
    ContactFreeTrajectory,
    FrictionResponse,
    GlobalCoupling,
    RaceFreeConnectedGlobalSolve,
    UniversalCollisionGuarantee,
    InversionFreeTrajectory,
    PostStepPositiveVolumeSample,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceMethod {
    FemcBarycentricInterpolation,
    JacobianTransposeIdentity,
    ConnectedPbdVolumeProjection,
    IndependentTetrahedronPbdVolume,
    AuthorityIdentityCompare,
    DeclaredUnsupported,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalEvidence {
    pub status: ResultStatus,
    pub claim: ClaimId,
    pub method: EvidenceMethod,
    pub value: Option<f64>,
    pub units: &'static str,
    pub reason: &'static str,
}

pub const SCOPED_KERNEL_NAME: &str =
    "pbd_softbody Verlet + Gauss-Seidel hard distance/volume projection";

pub const INDEPENDENT_TET_KERNEL_NAME: &str =
    "independent-tetrahedron PBD volume kernel (unshared vertices, no assembly)";

pub const FORBIDDEN_INDEPENDENT_TET_LABELS: &[&str] = &[
    "race-free connected global solver",
    "universal collision guarantee",
    "IPC contact solver",
    "connected shared-vertex global XPBD",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolverKernelLabel {
    pub integrator: IntegratorKind,
    pub advertised_as: &'static str,
    pub forbidden_labels: &'static [&'static str],
}

pub const CONNECTED_PBD_LABEL: SolverKernelLabel = SolverKernelLabel {
    integrator: IntegratorKind::ConnectedGaussSeidelPbd,
    advertised_as: SCOPED_KERNEL_NAME,
    forbidden_labels: &[
        "race-free connected global solver",
        "universal collision guarantee",
        "IPC contact solver",
    ],
};

pub const INDEPENDENT_TET_PBD_LABEL: SolverKernelLabel = SolverKernelLabel {
    integrator: IntegratorKind::IndependentTetrahedronPbd,
    advertised_as: INDEPENDENT_TET_KERNEL_NAME,
    forbidden_labels: FORBIDDEN_INDEPENDENT_TET_LABELS,
};

pub fn kernel_label(integrator: IntegratorKind) -> SolverKernelLabel {
    match integrator {
        IntegratorKind::ConnectedGaussSeidelPbd => CONNECTED_PBD_LABEL,
        IntegratorKind::IndependentTetrahedronPbd => INDEPENDENT_TET_PBD_LABEL,
    }
}

pub fn constitutive_matches_integrator(
    constitutive: ConstitutiveFamily,
    integrator: IntegratorKind,
) -> Result<(), PhysicalError> {
    match (constitutive, integrator) {
        (
            ConstitutiveFamily::PbdDistanceAndVolume,
            IntegratorKind::ConnectedGaussSeidelPbd,
        )
        | (
            ConstitutiveFamily::IndependentTetrahedronPbdVolume,
            IntegratorKind::IndependentTetrahedronPbd,
        ) => Ok(()),
        _ => Err(PhysicalError::InvalidInput("constitutive_integrator_mismatch")),
    }
}

/// Kinematic z-clamp already present in `pbd_softbody`. Not a contact model.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GroundPlaneTreatment {
    Disabled,
    KinematicZClamp { ground_z: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepParams {
    pub gravity: Point3,
    pub damping: f64,
    pub dt: f64,
    pub iterations: usize,
    pub ground: GroundPlaneTreatment,
}

impl StepParams {
    pub fn from_time(time: TimeDeclaration, gravity: Point3) -> Result<Self, PhysicalError> {
        time.validate()?;
        Ok(Self {
            gravity,
            damping: time.damping,
            dt: time.timestep_seconds,
            iterations: time.iterations_per_substep as usize,
            ground: GroundPlaneTreatment::Disabled,
        })
    }

    fn ground_z(self) -> f64 {
        match self.ground {
            GroundPlaneTreatment::Disabled => f64::NAN,
            GroundPlaneTreatment::KinematicZClamp { ground_z } => ground_z,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalError {
    InvalidInput(&'static str),
    Unsupported(&'static str),
    IdentityMismatch,
    Capacity { required: usize, available: usize },
    Budget(BudgetError),
    Kernel(PbdError),
    Correspondence,
    Inconclusive(&'static str),
}

impl From<BudgetError> for PhysicalError {
    fn from(e: BudgetError) -> Self {
        Self::Budget(e)
    }
}

impl PhysicalError {
    pub fn status(self) -> ResultStatus {
        match self {
            Self::Unsupported(_) => ResultStatus::Unsupported,
            Self::Inconclusive(_) => ResultStatus::Inconclusive,
            _ => ResultStatus::Failed,
        }
    }

    pub fn reason_code(self) -> ReasonCode {
        match self {
            Self::Unsupported(_) => ReasonCode::UnsupportedCapability,
            Self::IdentityMismatch => ReasonCode::StaleIdentity,
            Self::Capacity { .. } | Self::Budget(BudgetError::WorkLimitExceeded { .. })
            | Self::Budget(BudgetError::ByteLimitExceeded { .. })
            | Self::Budget(BudgetError::ArithmeticOverflow) => ReasonCode::CapacityExhausted,
            Self::Budget(BudgetError::Cancelled) => ReasonCode::Cancelled,
            Self::Inconclusive(_) => ReasonCode::InconclusiveVerification,
            _ => ReasonCode::InvalidInput,
        }
    }
}

impl core::fmt::Display for PhysicalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PhysicalError {}

pub fn charge_work(
    budget: &mut ProductionBudget<'_>,
    factors: &[usize],
) -> Result<(), PhysicalError> {
    let work = factors
        .iter()
        .try_fold(1u64, |a, &b| a.checked_mul(b as u64))
        .ok_or(PhysicalError::InvalidInput("work_overflow"))?;
    budget.checkpoint(work)?;
    Ok(())
}

pub fn require_capacity(required: usize, available: usize) -> Result<(), PhysicalError> {
    if required > available {
        Err(PhysicalError::Capacity {
            required,
            available,
        })
    } else {
        Ok(())
    }
}

pub fn tet_signed_volume(a: Point3, b: Point3, c: Point3, d: Point3) -> f64 {
    let b0 = Point3::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let c0 = Point3::new(c.x - a.x, c.y - a.y, c.z - a.z);
    let d0 = Point3::new(d.x - a.x, d.y - a.y, d.z - a.z);
    let cx = b0.y * c0.z - b0.z * c0.y;
    let cy = b0.z * c0.x - b0.x * c0.z;
    let cz = b0.x * c0.y - b0.y * c0.x;
    (cx * d0.x + cy * d0.y + cz * d0.z) / 6.0
}

pub fn min_signed_volume(positions: &[Point3], tets: &[[u32; 4]]) -> Result<f64, PhysicalError> {
    if tets.is_empty() {
        return Err(PhysicalError::InvalidInput("empty_tets"));
    }
    let mut min_v = f64::INFINITY;
    for tet in tets {
        let pts = tet_points(positions, *tet)?;
        let v = tet_signed_volume(pts[0], pts[1], pts[2], pts[3]);
        if !v.is_finite() {
            return Err(PhysicalError::InvalidInput("volume_nonfinite"));
        }
        min_v = min_v.min(v);
    }
    Ok(min_v)
}

pub fn tet_points(positions: &[Point3], tet: [u32; 4]) -> Result<[Point3; 4], PhysicalError> {
    let mut pts = [Point3::new(0.0, 0.0, 0.0); 4];
    for k in 0..4 {
        let i = tet[k] as usize;
        if i >= positions.len() {
            return Err(PhysicalError::InvalidInput("tet_vertex"));
        }
        pts[k] = positions[i];
    }
    Ok(pts)
}

/// Caller-buffered independent-tet volume constraints: vertices `4i..4i+3`.
pub fn fill_independent_volume_constraints(
    rest_volumes: &[f64],
    out: &mut [PbdConstraint],
    budget: &mut ProductionBudget<'_>,
) -> Result<usize, PhysicalError> {
    require_capacity(rest_volumes.len(), out.len())?;
    charge_work(budget, &[rest_volumes.len().max(1), 2])?;
    for (i, &rest) in rest_volumes.iter().enumerate() {
        if !rest.is_finite() || rest == 0.0 {
            return Err(PhysicalError::InvalidInput("rest_volume"));
        }
        let base = (i as u32)
            .checked_mul(4)
            .ok_or(PhysicalError::InvalidInput("tet_index_overflow"))?;
        out[i] = PbdConstraint::Volume {
            a: base,
            b: base + 1,
            c: base + 2,
            d: base + 3,
            rest,
        };
    }
    budget.checkpoint(0)?;
    Ok(rest_volumes.len())
}

pub fn fill_connected_volume_constraints(
    tets: &[[u32; 4]],
    rest_positions: &[Point3],
    out: &mut [PbdConstraint],
    budget: &mut ProductionBudget<'_>,
) -> Result<usize, PhysicalError> {
    require_capacity(tets.len(), out.len())?;
    charge_work(budget, &[tets.len().max(1), 8])?;
    for (i, tet) in tets.iter().enumerate() {
        let pts = tet_points(rest_positions, *tet)?;
        let rest = tet_signed_volume(pts[0], pts[1], pts[2], pts[3]);
        if !rest.is_finite() || rest == 0.0 {
            return Err(PhysicalError::InvalidInput("rest_volume"));
        }
        out[i] = PbdConstraint::Volume {
            a: tet[0],
            b: tet[1],
            c: tet[2],
            d: tet[3],
            rest,
        };
    }
    budget.checkpoint(0)?;
    Ok(tets.len())
}

/// In-place scoped kernel. Independent-tet mode requires unshared vertices.
#[allow(clippy::too_many_arguments)]
pub fn step_scoped_kernel(
    constitutive: ConstitutiveDeclaration,
    time: TimeDeclaration,
    params: StepParams,
    positions: &mut [Point3],
    previous: &mut [Point3],
    inv_masses: &[f64],
    constraints: &[PbdConstraint],
    budget: &mut ProductionBudget<'_>,
) -> Result<PbdReceipt, PhysicalError> {
    constitutive.validate()?;
    time.validate()?;
    constitutive_matches_integrator(constitutive.family, time.integrator)?;
    if previous.len() != positions.len() || inv_masses.len() != positions.len() {
        return Err(PhysicalError::InvalidInput("state_dimensions"));
    }
    if let IntegratorKind::IndependentTetrahedronPbd = time.integrator {
        if positions.len() % 4 != 0 {
            return Err(PhysicalError::InvalidInput("independent_tet_layout"));
        }
        for (ci, c) in constraints.iter().enumerate() {
            match *c {
                PbdConstraint::Volume { a, b, c, d, .. } => {
                    let expected = (ci as u32)
                        .checked_mul(4)
                        .ok_or(PhysicalError::InvalidInput("tet_index_overflow"))?;
                    if [a, b, c, d] != [expected, expected + 1, expected + 2, expected + 3] {
                        return Err(PhysicalError::InvalidInput("independent_tet_sharing"));
                    }
                }
                PbdConstraint::Distance { .. } => {
                    return Err(PhysicalError::Unsupported(
                        "independent_tet_distance_coupling",
                    ));
                }
            }
        }
    }
    charge_work(
        budget,
        &[
            positions.len().max(1),
            constraints.len().max(1),
            params.iterations.max(1),
            4,
        ],
    )?;
    let receipt = pbd_step(
        positions,
        previous,
        inv_masses,
        constraints,
        params.gravity,
        params.damping,
        params.dt,
        params.iterations,
        params.ground_z(),
    )
    .map_err(PhysicalError::Kernel)?;
    budget.checkpoint(0)?;
    Ok(receipt)
}

pub fn post_step_positive_volume_sample(
    positions: &[Point3],
    tets: &[[u32; 4]],
) -> Result<PhysicalEvidence, PhysicalError> {
    let min_v = min_signed_volume(positions, tets)?;
    if min_v > 0.0 {
        Ok(PhysicalEvidence {
            status: ResultStatus::Passed,
            claim: ClaimId::PostStepPositiveVolumeSample,
            method: EvidenceMethod::ConnectedPbdVolumeProjection,
            value: Some(min_v),
            units: "m3",
            reason: "post-step signed-volume sample only; not an inversion-free trajectory",
        })
    } else {
        Ok(PhysicalEvidence {
            status: ResultStatus::Failed,
            claim: ClaimId::PostStepPositiveVolumeSample,
            method: EvidenceMethod::ConnectedPbdVolumeProjection,
            value: Some(min_v),
            units: "m3",
            reason: "non-positive post-step volume sample",
        })
    }
}

pub fn evidence_unsupported(claim: ClaimId) -> PhysicalEvidence {
    let reason = match claim {
        ClaimId::ContactFreeTrajectory => {
            "no collision detection, feasibility, or contact barrier; kinematic z-clamp is not contact"
        }
        ClaimId::FrictionResponse => "no friction model is implemented",
        ClaimId::GlobalCoupling => {
            "no assembled global implicit coupling; independent tets have none, GS is sequential"
        }
        ClaimId::RaceFreeConnectedGlobalSolve => {
            "independent-tetrahedron PBD is not a connected solver; GS PBD is order-dependent"
        }
        ClaimId::UniversalCollisionGuarantee => {
            "no universal collision guarantee; REQ-19 assumptions are absent"
        }
        ClaimId::InversionFreeTrajectory => {
            "a post-step determinant/volume sample is insufficient for an inversion-free trajectory"
        }
        _ => "claim is outside the scoped kernel",
    };
    PhysicalEvidence {
        status: ResultStatus::Unsupported,
        claim,
        method: EvidenceMethod::DeclaredUnsupported,
        value: None,
        units: "",
        reason,
    }
}

pub const UNSUPPORTED_COUPLING_CLAIMS: [ClaimId; 3] = [
    ClaimId::ContactFreeTrajectory,
    ClaimId::FrictionResponse,
    ClaimId::GlobalCoupling,
];

/// Lawful status for a capability claim. Passing an over-claim fails closed.
pub fn evaluate_claim(
    _integrator: IntegratorKind,
    claim: ClaimId,
    asserted: ResultStatus,
) -> PhysicalEvidence {
    let lawful = match claim {
        ClaimId::ContactFreeTrajectory
        | ClaimId::FrictionResponse
        | ClaimId::GlobalCoupling
        | ClaimId::RaceFreeConnectedGlobalSolve
        | ClaimId::UniversalCollisionGuarantee
        | ClaimId::InversionFreeTrajectory => evidence_unsupported(claim),
        ClaimId::SolverTopologyPreservedUnderLod
        | ClaimId::RestStateTransfer
        | ClaimId::DeformedStateTransfer
        | ClaimId::VirtualWorkIdentity
        | ClaimId::PrescribedResponse
        | ClaimId::PostStepPositiveVolumeSample => PhysicalEvidence {
            status: ResultStatus::Inconclusive,
            claim,
            method: EvidenceMethod::DeclaredUnsupported,
            value: None,
            units: "",
            reason: "claim requires a measured fixture, not a kernel advertisement",
        },
    };
    if asserted == ResultStatus::Passed && lawful.status != ResultStatus::Passed {
        PhysicalEvidence {
            status: ResultStatus::Failed,
            claim,
            method: lawful.method,
            value: lawful.value,
            units: lawful.units,
            reason: "overclaim: kernel cannot lawfully pass this capability",
        }
    } else {
        lawful
    }
}

/// True when the advertised kernel name itself contains a forbidden overclaim.
pub fn advertised_label_is_lawful(label: SolverKernelLabel) -> bool {
    let hay = label.advertised_as.to_ascii_lowercase();
    !label
        .forbidden_labels
        .iter()
        .any(|phrase| hay.contains(&phrase.to_ascii_lowercase()))
}
