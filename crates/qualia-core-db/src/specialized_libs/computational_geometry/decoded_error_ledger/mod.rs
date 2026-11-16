//! Decoded-output geometry bounds and a typed error ledger (P08).
//!
//! Compare a frozen reference triangle domain to **decoded final asset**
//! buffers, not a pre-quantization mesh. Channels stay separate: geometric
//! surface deviation, attribute, quantization, physical, and numerical.
//!
//! Enclosure arithmetic is serial IEEE-754 binary64 round-to-nearest. Finite
//! interval containers are not outward rounding and are not used as a proof.
//! A first-order ulp slack deflates `L` and inflates `U`. Budget-limited
//! inability to prove a bound is `inconclusive`, never a pass. Sampled
//! estimates remain labelled estimates and cannot pass the G1 enclosing
//! profile. Physical response is `unsupported` when no P10 `PhysicalEvidence`
//! is supplied; P10 is present in-tree and absence is not a zero error.
//!
//! G1 defaults (spec v0.2) until a shared profile change: accept only when
//! `U <= 1e-3 D` and `U-L <= 1e-5 D`; reject when `L > 1e-3 D`; protected-point
//! drift at most `1e-5 D`. Near-zero extent requires `absolute_scale`.

mod ledger;
mod surface_bracket;

pub use ledger::{
    AttributeRecord, DecodedErrorLedger, GeometricRecord, NumericalRecord, PhysicalRecord,
    QuantizationRecord, certify_decoded_output, certify_decoded_output_with_physical,
    physical_record,
};
pub use surface_bracket::SurfacePatch;

use super::bvh::{BvhError, BvhNode, MAX_BVH_DEPTH};
use super::distance::Aabb;
use super::geometry_workspace::production_budget::{
    BudgetError, BufferClass, CapacityPlan,
};
use super::primitives::Point3;

#[cfg(test)]
mod tests;

/// G1 upper bound as a fraction of the reference AABB diagonal.
pub const G1_UPPER_OVER_DIAGONAL: f64 = 1e-3;
/// G1 unresolved bracket width as a fraction of the reference AABB diagonal.
pub const G1_WIDTH_OVER_DIAGONAL: f64 = 1e-5;
/// G1 protected-point displacement as a fraction of the reference AABB diagonal.
pub const G1_PROTECTED_DRIFT_OVER_DIAGONAL: f64 = 1e-5;
/// Operation-count factor for the documented f64 slack (not outward rounding).
pub const NUMERICAL_SLACK_OPS: f64 = 64.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgerStatus {
    Passed,
    Failed,
    Inconclusive,
    Unsupported,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementClass {
    SampledEstimate,
    Probabilistic,
    EnclosingBound,
    ProofUnderAssumptions,
    Measured,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbsenceReason {
    UnsupportedCapability,
    NotSupplied,
    Cancelled,
    BudgetExhausted,
    DomainEmpty,
    IncompleteEnclosure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Units {
    AssetLength,
    Dimensionless,
}

/// Numeric evidence. Absent channels are `Absent`, never a silent `0.0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Metric {
    Absent { reason: AbsenceReason },
    Scalar {
        value: f64,
        units: Units,
        class: MeasurementClass,
    },
    Bracket {
        lower: f64,
        upper: f64,
        units: Units,
        class: MeasurementClass,
    },
}

/// Wire form that refuses to encode absence as zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MetricWire {
    Null,
    Number(f64),
    Bracket { lower: f64, upper: f64 },
}

impl Metric {
    pub const fn absent(reason: AbsenceReason) -> Self {
        Self::Absent { reason }
    }

    pub fn to_wire(self) -> MetricWire {
        match self {
            Self::Absent { .. } => MetricWire::Null,
            Self::Scalar { value, .. } => MetricWire::Number(value),
            Self::Bracket { lower, upper, .. } => MetricWire::Bracket { lower, upper },
        }
    }

    pub fn class(self) -> Option<MeasurementClass> {
        match self {
            Self::Absent { .. } => None,
            Self::Scalar { class, .. } | Self::Bracket { class, .. } => Some(class),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeometryProfile {
    pub upper_over_diagonal: f64,
    pub width_over_diagonal: f64,
    pub protected_drift_over_diagonal: f64,
    /// Required when the reference AABB diagonal is numerically degenerate.
    pub absolute_scale: Option<f64>,
}

impl GeometryProfile {
    pub const fn g1() -> Self {
        Self {
            upper_over_diagonal: G1_UPPER_OVER_DIAGONAL,
            width_over_diagonal: G1_WIDTH_OVER_DIAGONAL,
            protected_drift_over_diagonal: G1_PROTECTED_DRIFT_OVER_DIAGONAL,
            absolute_scale: None,
        }
    }

    pub fn upper_threshold(self, diagonal: f64) -> f64 {
        self.upper_over_diagonal * diagonal
    }

    pub fn width_threshold(self, diagonal: f64) -> f64 {
        self.width_over_diagonal * diagonal
    }

    pub fn protected_threshold(self, diagonal: f64) -> f64 {
        self.protected_drift_over_diagonal * diagonal
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundRoute {
    EnclosingSubdivision,
    SampledVertices,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BracketOptions {
    pub route: BoundRoute,
    pub max_subdivision_depth: u16,
}

impl Default for BracketOptions {
    fn default() -> Self {
        Self {
            route: BoundRoute::EnclosingSubdivision,
            max_subdivision_depth: 12,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QuantizationModel {
    NotSupplied,
    Binary32,
    UniformGrid { delta: [f64; 3] },
}

#[derive(Clone, Copy, Debug)]
pub struct TriangleMesh<'a> {
    pub vertices: &'a [Point3],
    pub triangles: &'a [[u32; 3]],
}

#[derive(Clone, Copy, Debug)]
pub struct DecodedMeshPair<'a> {
    pub reference: TriangleMesh<'a>,
    pub decoded: TriangleMesh<'a>,
}

#[derive(Clone, Copy, Debug)]
pub struct AttributePair<'a> {
    pub units: &'static str,
    pub reference: &'a [f64],
    pub decoded: &'a [f64],
}

#[derive(Clone, Copy, Debug)]
pub struct ProtectedCorrespondence<'a> {
    pub reference_vertices: &'a [u32],
    pub decoded_vertices: &'a [u32],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AdmissionEvidence {
    /// `None` means not supplied (absent, not zero merges).
    pub forbidden_semantic_merges: Option<u64>,
}

pub struct LedgerScratch<'a> {
    pub aabbs: &'a mut [Aabb],
    pub nodes: &'a mut [BvhNode],
    pub prim_indices: &'a mut [u32],
    pub morton_codes: &'a mut [u64],
    pub sort_indices: &'a mut [u32],
    pub stack: &'a mut [u32],
    pub overlap: &'a mut [u32],
    pub patches: &'a mut [SurfacePatch],
}

impl LedgerScratch<'_> {
    pub const fn bvh_stack_len() -> usize {
        MAX_BVH_DEPTH * 2
    }

    pub fn bvh_node_len(triangle_count: usize) -> Result<usize, LedgerError> {
        if triangle_count == 0 {
            return Ok(0);
        }
        triangle_count
            .checked_mul(2)
            .ok_or(LedgerError::InvalidInput("bvh_node_overflow"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    InvalidInput(&'static str),
    Capacity { required: usize, available: usize },
    Budget(BudgetError),
    Bvh(BvhError),
}

impl From<BudgetError> for LedgerError {
    fn from(value: BudgetError) -> Self {
        Self::Budget(value)
    }
}

impl From<BvhError> for LedgerError {
    fn from(value: BvhError) -> Self {
        Self::Bvh(value)
    }
}

impl core::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidInput(msg) => write!(f, "decoded ledger invalid input: {msg}"),
            Self::Capacity { required, available } => {
                write!(f, "decoded ledger capacity {required} > {available}")
            }
            Self::Budget(err) => write!(f, "decoded ledger budget: {err}"),
            Self::Bvh(err) => write!(f, "decoded ledger bvh: {err:?}"),
        }
    }
}

impl std::error::Error for LedgerError {}

pub fn plan_ledger_capacity(
    reference_vertices: usize,
    reference_triangles: usize,
    decoded_vertices: usize,
    decoded_triangles: usize,
    overlap: usize,
    patches: usize,
    attribute_scalars: usize,
    protected_pairs: usize,
) -> Result<CapacityPlan, BudgetError> {
    let max_tris = reference_triangles.max(decoded_triangles);
    let mut plan = CapacityPlan::default();
    plan.add_slice::<Point3>(BufferClass::Input, reference_vertices)?;
    plan.add_slice::<[u32; 3]>(BufferClass::Input, reference_triangles)?;
    plan.add_slice::<Point3>(BufferClass::Input, decoded_vertices)?;
    plan.add_slice::<[u32; 3]>(BufferClass::Input, decoded_triangles)?;
    plan.add_slice::<f64>(BufferClass::Input, attribute_scalars)?;
    plan.add_slice::<u32>(BufferClass::Input, protected_pairs.saturating_mul(2))?;
    plan.add_slice::<Aabb>(BufferClass::Scratch, max_tris)?;
    let node_len = if max_tris == 0 {
        0
    } else {
        max_tris.checked_mul(2).ok_or(BudgetError::ArithmeticOverflow)?
    };
    plan.add_slice::<BvhNode>(BufferClass::Scratch, node_len)?;
    plan.add_slice::<u32>(BufferClass::Scratch, max_tris)?;
    plan.add_slice::<u64>(BufferClass::Scratch, max_tris)?;
    plan.add_slice::<u32>(BufferClass::Scratch, max_tris)?;
    plan.add_slice::<u32>(BufferClass::Scratch, LedgerScratch::bvh_stack_len())?;
    plan.add_slice::<u32>(BufferClass::Scratch, overlap)?;
    plan.add_slice::<SurfacePatch>(BufferClass::Scratch, patches)?;
    plan.add_slice::<DecodedErrorLedger>(BufferClass::Output, 1)?;
    Ok(plan)
}

pub(crate) fn need(required: usize, available: usize) -> Result<(), LedgerError> {
    if required > available {
        Err(LedgerError::Capacity { required, available })
    } else {
        Ok(())
    }
}
