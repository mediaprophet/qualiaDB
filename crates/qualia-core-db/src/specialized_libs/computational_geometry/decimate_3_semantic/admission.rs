//! Caller-buffered, strict-manifold QEM fallback. Triangle scans trade throughput for bounded storage.
//! Corner discontinuities must be authored as split vertices with distinct domain tags.
//! No global self-intersection/contact guarantee or scalable-throughput claim is made.
use super::normalized_cost::{NormalizedCostProfile, NormalizedCostTerms};
use super::{Quadric4, SemanticVertexAttributes};
use crate::specialized_libs::computational_geometry::geometry_workspace::production_budget::{
    BudgetError, BufferClass, CapacityPlan, ProductionBudget,
};
use crate::specialized_libs::computational_geometry::geometry_workspace::{
    Cancellation, GeometryWorkspace, WorkspaceError,
};
use crate::specialized_libs::computational_geometry::production_contracts::{
    CollapseEvent, ContentId, ContractError, CornerDelta, FaceDelta, ReasonCode, RemapBuffers, TransferAdmission,
};
use crate::specialized_libs::computational_geometry::{FilteredF64Kernel, GeometryKernel, Point3};

#[path = "bounded_driver.rs"]
mod bounded_driver;
#[cfg(test)]
#[path = "admission_tests.rs"]
mod tests;
#[path = "topology.rs"]
mod topology;

/// All these identities are hard domains, not confidence or sensitivity weights.
/// Zero means the author declares no such domain; it is not a wildcard.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SemanticDomains {
    pub material: u64,
    pub provenance: u64,
    pub epistemic: u64,
    pub subpart: u64,
    pub feature_curve: u64,
    pub attachment: u64,
    pub protected_region: u64,
    pub uv_chart: u64,
    pub tangent_handedness: i8,
}
impl SemanticDomains {
    fn immovable(self) -> bool {
        self.feature_curve != 0 || self.attachment != 0 || self.protected_region != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyMode {
    StrictManifold,
    StratifiedNonManifold,
    Repair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainLayout {
    VertexAlignedSplitCorners,
    UnsplitFaceOrCornerDomains,
}

#[derive(Debug, Clone, Copy)]
pub struct AdmissionPolicy {
    pub topology: TopologyMode,
    /// Face/corner discontinuities must already have explicit vertex splits.
    pub domain_layout: DomainLayout,
    pub target_faces: usize,
    pub preserve_boundary_vertices: bool,
    pub cost: NormalizedCostProfile,
    /// Dimensionless ranking ceiling, separate from any measured surface-distance limit.
    pub maximum_normalized_cost: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub struct AdmissionIdentity {
    pub run: ContentId,
    pub source: ContentId,
    pub policy: ContentId,
}

pub struct AdmissionInput<'a> {
    pub vertices: &'a [Point3],
    pub attributes: &'a [SemanticVertexAttributes],
    pub triangles: &'a [[u32; 3]],
    pub domains: &'a [SemanticDomains],
    pub stable_vertex_ids: &'a [u64],
}

/// Allocate this single typed slice through GeometryWorkspace, then pass it as scratch.
#[derive(Clone, Copy)]
pub struct AdmissionSlot {
    position: Point3,
    attributes: SemanticVertexAttributes,
    quadric: Quadric4,
    triangle: [u32; 3],
    vertex_alive: bool,
    face_alive: bool,
    lineage: u32,
    generation: u64,
    mark: bool,
}
impl AdmissionSlot {
    pub fn empty() -> Self {
        Self {
            position: Point3::default(),
            attributes: SemanticVertexAttributes::default(),
            quadric: Quadric4::zero(),
            triangle: [0; 3],
            vertex_alive: false,
            face_alive: false,
            lineage: u32::MAX,
            generation: 0,
            mark: false,
        }
    }
}
pub fn allocate_admission_slots<'a>(
    workspace: &'a mut GeometryWorkspace<'_>,
    vertices: usize,
    faces: usize,
) -> Result<&'a mut [AdmissionSlot], WorkspaceError> {
    workspace.alloc_slice_fill(vertices.max(faces), AdmissionSlot::empty())
}

pub struct AdmissionScratch<'a> {
    pub slots: &'a mut [AdmissionSlot],
    pub faces: &'a mut [FaceDelta],
    pub corners: &'a mut [CornerDelta],
    pub transfer: &'a mut [u8],
}
pub struct AdmissionOutput<'a> {
    pub vertices: &'a mut [Point3],
    pub attributes: &'a mut [SemanticVertexAttributes],
    pub domains: &'a mut [SemanticDomains],
    pub triangles: &'a mut [[u32; 3]],
    pub remaps: RemapBuffers<'a>,
    pub events: &'a mut [Option<CollapseEvent>],
}
#[derive(Clone, Copy)]
pub struct AdmissionLimits {
    pub bytes: usize,
    pub work: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Termination {
    ReachedTarget,
    ConstraintLimited,
    CostLimited,
    WorkExhausted,
    Cancelled,
    EventCapacityExhausted,
    TransferFailure(ReasonCode),
}
#[derive(Debug, Default, Clone, Copy)]
pub struct RejectionCounts {
    pub semantic: u64,
    pub topology: u64,
    pub transfer: u64,
    pub numeric: u64,
}
#[derive(Debug, Clone, Copy)]
pub struct AdmissionReport {
    pub termination: Termination,
    pub vertices: usize,
    pub faces: usize,
    pub committed_collapses: usize,
    pub rejections: RejectionCounts,
    pub largest_cost: NormalizedCostTerms,
    pub work_used: u64,
    pub admitted_bytes: usize,
    /// Interpolated legacy normals are diagnostics only; recompute derived normals/tangents for rendering.
    pub derived_frames_require_recomputation: bool,
}
#[derive(Debug, Clone, Copy)]
pub enum AdmissionError {
    Contract(ContractError),
    Budget(BudgetError),
}
impl From<ContractError> for AdmissionError {
    fn from(e: ContractError) -> Self {
        Self::Contract(e)
    }
}
impl From<BudgetError> for AdmissionError {
    fn from(e: BudgetError) -> Self {
        Self::Budget(e)
    }
}
fn error(code: ReasonCode, field: &'static str) -> AdmissionError {
    ContractError::new(code, field).into()
}

fn capacity_plan(
    input: &AdmissionInput<'_>,
    scratch: &AdmissionScratch<'_>,
    output: &AdmissionOutput<'_>,
) -> Result<CapacityPlan, BudgetError> {
    let mut p = CapacityPlan::default();
    p.add_slice::<Point3>(BufferClass::Input, input.vertices.len())?;
    p.add_slice::<SemanticVertexAttributes>(BufferClass::Input, input.attributes.len())?;
    p.add_slice::<[u32; 3]>(BufferClass::Input, input.triangles.len())?;
    p.add_slice::<SemanticDomains>(BufferClass::Input, input.domains.len())?;
    p.add_slice::<u64>(BufferClass::Input, input.stable_vertex_ids.len())?;
    p.add_arena_slice::<AdmissionSlot>(scratch.slots.len())?;
    p.add_slice::<FaceDelta>(BufferClass::Scratch, scratch.faces.len())?;
    p.add_slice::<CornerDelta>(BufferClass::Scratch, scratch.corners.len())?;
    p.add_bytes(BufferClass::Scratch, scratch.transfer.len())?;
    p.add_slice::<Point3>(BufferClass::Output, output.vertices.len())?;
    p.add_slice::<SemanticVertexAttributes>(BufferClass::Output, output.attributes.len())?;
    p.add_slice::<SemanticDomains>(BufferClass::Output, output.domains.len())?;
    p.add_slice::<[u32; 3]>(BufferClass::Output, output.triangles.len())?;
    p.add_slice::<u32>(BufferClass::Output, output.remaps.vertex_lineage.len())?;
    p.add_slice::<u32>(BufferClass::Output, output.remaps.surviving_vertex_compaction.len())?;
    p.add_slice::<u32>(BufferClass::Output, output.remaps.face_compaction.len())?;
    p.add_slice::<Option<crate::specialized_libs::computational_geometry::production_contracts::CornerId>>(
        BufferClass::Output,
        output.remaps.corner_compaction.len(),
    )?;
    p.add_slice::<Option<CollapseEvent>>(BufferClass::Output, output.events.len())?;
    Ok(p)
}

/// No allocation occurs here. Caller buffers and the transfer adapter are part of the resource contract.
/// A non-ReachedTarget report exposes only a committed prefix, never an accepted asset.
pub fn decimate_semantic_admitted_3<T: TransferAdmission>(
    input: AdmissionInput<'_>,
    policy: AdmissionPolicy,
    identity: AdmissionIdentity,
    limits: AdmissionLimits,
    cancel: &Cancellation,
    scratch: AdmissionScratch<'_>,
    output: AdmissionOutput<'_>,
    transfer: &mut T,
) -> Result<AdmissionReport, AdmissionError> {
    decimate_semantic_admitted_3_with_kernel(
        &FilteredF64Kernel::default(),
        input,
        policy,
        identity,
        limits,
        cancel,
        scratch,
        output,
        transfer,
    )
}

pub fn decimate_semantic_admitted_3_with_kernel<K: GeometryKernel, T: TransferAdmission>(
    kernel: &K,
    input: AdmissionInput<'_>,
    policy: AdmissionPolicy,
    identity: AdmissionIdentity,
    limits: AdmissionLimits,
    cancel: &Cancellation,
    scratch: AdmissionScratch<'_>,
    output: AdmissionOutput<'_>,
    transfer: &mut T,
) -> Result<AdmissionReport, AdmissionError> {
    let n = input.vertices.len();
    let m = input.triangles.len();
    if policy.topology != TopologyMode::StrictManifold {
        return Err(error(ReasonCode::UnsupportedCapability, "topology_mode"));
    }
    if policy.domain_layout != DomainLayout::VertexAlignedSplitCorners {
        return Err(error(ReasonCode::UnsupportedCapability, "domain_layout"));
    }
    if n == 0
        || m == 0
        || n >= u32::MAX as usize
        || m >= u32::MAX as usize
        || input.attributes.len() != n
        || input.domains.len() != n
        || input.stable_vertex_ids.len() != n
        || policy.target_faces > m
        || !policy.cost.is_valid()
        || policy.maximum_normalized_cost.is_some_and(|x| !x.is_finite() || x < 0.0)
    {
        return Err(error(ReasonCode::InvalidInput, "admission_input"));
    }
    let corners = m.checked_mul(3).ok_or_else(|| error(ReasonCode::CapacityExhausted, "corner_count"))?;
    if scratch.slots.len() < n.max(m)
        || scratch.faces.len() < m
        || scratch.corners.len() < corners
        || output.vertices.len() < n
        || output.attributes.len() < n
        || output.domains.len() < n
        || output.triangles.len() < m
    {
        return Err(error(ReasonCode::CapacityExhausted, "admission_buffers"));
    }
    output.remaps.validate_capacity(n, m)?;
    if output.events.len() < n.saturating_sub(1) || scratch.transfer.len() < transfer.scratch_bytes()? {
        return Err(error(ReasonCode::CapacityExhausted, "transfer_or_event_capacity"));
    }
    let mut plan = capacity_plan(&input, &scratch, &output)?;
    plan.add_bytes(BufferClass::Scratch, transfer.additional_storage_bytes()?)?;
    let mut budget = ProductionBudget::admit(plan, limits.bytes, limits.work, cancel)?;
    // Reserve the complete final compaction/lineage traversal before any mesh mutation.
    let final_work = n
        .checked_mul(n)
        .and_then(|x| x.checked_add(n))
        .and_then(|x| x.checked_add(corners))
        .and_then(|x| x.checked_add(m))
        .ok_or(BudgetError::ArithmeticOverflow)?;
    budget.checkpoint(u64::try_from(final_work).map_err(|_| BudgetError::ArithmeticOverflow)?)?;
    topology::validate_input(&input, scratch.slots, &mut budget)?;
    bounded_driver::run(kernel, input, policy, identity, limits, scratch, output, transfer, &mut budget)
}
