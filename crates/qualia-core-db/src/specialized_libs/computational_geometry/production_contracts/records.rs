use super::{ContentId, ReasonCode, TransferContract};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaVersion {
    #[serde(rename = "q42.production-contracts/v1")]
    V1,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractDocument {
    pub schema: SchemaVersion,
    pub record: ContractRecord,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ContractRecord {
    Asset(AssetContract),
    Quality(QualityProfile),
    Candidate(RepresentationCandidate),
    Transfer(TransferContract),
    Kernel(KernelContract),
    Variant(ExecutionVariant),
    Receipt(ValidationReceipt),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    MeshContainer,
    FemcBindings,
    ClusterHierarchy,
    AttributeData,
    DomainLabels,
    IndexRemap,
    SparseMap,
    Operator,
    KernelSpecification,
    ShaderSource,
    ForgeManifest,
    EtalonRunReceipt,
    Fixture,
    Evidence,
    DomainDefinition,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "codec", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResourceEncoding {
    Existing10d,
    FemcV1,
    ClusterDagV1,
    ForgeCertification { schema_version: u32 },
    EtalonReceiptV1,
    WgslUtf8,
    ExternalTyped { schema: ContentId },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRef {
    pub id: ContentId,
    pub kind: ResourceKind,
    pub encoding: ResourceEncoding,
    pub byte_len: u64,
    pub element_count: Option<u64>,
}
impl ResourceRef {
    pub fn verify(&self, bytes: &[u8]) -> Result<(), super::ContractError> {
        if self.byte_len != bytes.len() as u64 {
            return Err(super::ContractError::new(
                ReasonCode::StaleIdentity,
                "resource_length",
            ));
        }
        self.id.verify(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepresentationRole {
    Authoritative,
    VisualSurface,
    IntrinsicSurface,
    ContactSurface,
    VolumetricSolver,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyMode {
    ClosedManifold,
    OpenManifold,
    DisconnectedManifold,
    StratifiedNonManifold,
    ExplicitRepair,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityDomain {
    Vertex,
    Face,
    Corner,
    Cell,
    DegreeOfFreedom,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelKind {
    Chart,
    Material,
    Provenance,
    Epistemic,
    Subpart,
    Feature,
    Attachment,
    Protected,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldSemantics {
    Intensive,
    Extensive,
    Categorical,
    TangentVector,
    Displacement,
    Force,
    Normal,
    Tangent,
    SkinWeights,
    CornerUv,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoordinateFrame {
    pub identity: ContentId,
    pub length_unit_metres: f64,
    pub right_handed: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeDescriptor {
    pub name: String,
    pub domain: EntityDomain,
    pub semantics: FieldSemantics,
    pub units: String,
    pub frame: Option<ContentId>,
    pub data: ResourceRef,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainLabel {
    pub domain: EntityDomain,
    pub kind: LabelKind,
    pub labels: ResourceRef,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetContract {
    pub content: ResourceRef,
    pub role: RepresentationRole,
    pub frame: CoordinateFrame,
    pub topology: TopologyMode,
    pub attributes: Vec<AttributeDescriptor>,
    pub domains: Vec<DomainLabel>,
    pub protected_entities: Vec<ResourceRef>,
    pub source_assertions: Vec<ResourceRef>,
    pub admitted_operations: Vec<String>,
    pub quality_profile: ContentId,
    pub transfer_dependencies: Vec<ContentId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementClass {
    Measured,
    SampledEstimate,
    Probabilistic,
    EnclosingBound,
    ProofUnderAssumptions,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorCriterion {
    pub name: String,
    pub units: String,
    pub threshold: f64,
    pub class: MeasurementClass,
    pub method: String,
    pub required: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Precision {
    F16,
    F32,
    F64,
    PairedF32,
    I32,
    U32,
    I64,
    U64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReductionOrder {
    FixedSerial,
    FixedTree,
    BackendReassociation,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExceptionalValues {
    RejectNonFinite,
    DeclaredIeeePropagation,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArithmeticProfile {
    pub storage: Precision,
    pub arithmetic: Precision,
    pub accumulation: Precision,
    pub reduction: ReductionOrder,
    pub exceptional_values: ExceptionalValues,
    pub minimum_magnitude: Option<f64>,
    pub maximum_magnitude: Option<f64>,
    pub scaling: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceBounds {
    pub cold_bytes: u64,
    pub work_items: u64,
    pub output_elements: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualityProfile {
    pub name: String,
    pub reference_scale: f64,
    pub criteria: Vec<ErrorCriterion>,
    pub domains: Vec<ResourceRef>,
    pub assurance_label: String,
    pub fidelity_label: String,
    pub arithmetic: ArithmeticProfile,
    pub resources: ResourceBounds,
    pub expected_rejections: Vec<ReasonCode>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum MetricValue {
    NotMeasured {
        reason: String,
    },
    NotApplicable {
        reason: String,
    },
    Available {
        value: f64,
        units: String,
        class: MeasurementClass,
        method: String,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedMetric {
    pub name: String,
    pub value: MetricValue,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepresentationCandidate {
    pub parent: ContentId,
    pub asset: ContentId,
    pub algorithm: String,
    pub source_revision: ContentId,
    #[serde(deserialize_with = "super::identity::unique_parameters")]
    pub parameters: std::collections::BTreeMap<String, String>,
    pub remaps: Vec<ResourceRef>,
    pub transfers: Vec<ContentId>,
    pub achieved_vertices: u64,
    pub achieved_faces: u64,
    pub unresolved_constraints: Vec<String>,
    pub errors: Vec<NamedMetric>,
    pub dependencies: Vec<ContentId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TensorShape {
    pub name: String,
    pub dimensions: Vec<u64>,
    pub units: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryAccess {
    Read,
    Write,
    ReadWrite,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BufferEffect {
    pub binding: u32,
    pub access: MemoryAccess,
    pub alias_group: Option<u32>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lowering {
    Wgsl,
    Hlsl,
    Msl,
    Cuda,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelContract {
    pub semantics: ContentId,
    pub discretisation: ContentId,
    pub specification: ResourceRef,
    pub shapes: Vec<TensorShape>,
    pub effects: Vec<BufferEffect>,
    pub assumptions: Vec<String>,
    pub lowerings: Vec<Lowering>,
    pub arithmetic: ArithmeticProfile,
    pub resources: ResourceBounds,
    pub reference_evaluator: ResourceRef,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleContract {
    pub workgroup: [u32; 3],
    pub items_per_invocation: u32,
    pub vector_width: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterScope {
    pub backend: String,
    pub adapter: ContentId,
    pub driver: String,
    pub compiler: String,
    pub features: Vec<String>,
    pub limits: ResourceRef,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionVariant {
    pub kernel: ContentId,
    pub schedule: ScheduleContract,
    pub abi: ResourceRef,
    pub emitted_source: ResourceRef,
    pub precision: ArithmeticProfile,
    pub adapter: AdapterScope,
    pub toolchain: ContentId,
    pub dependencies: Vec<ContentId>,
    pub forge_manifest: Option<ResourceRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultStatus {
    Passed,
    Failed,
    Inconclusive,
    Skipped,
    Unsupported,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimingSource {
    GpuTimestamp,
    CompletionClock,
    CpuMonotonic,
    NotMeasured,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStage {
    Generated,
    NagaValidated,
    PipelineCreated,
    OracleVerified,
    Profiled,
    Certified,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationReceipt {
    pub targets: Vec<ContentId>,
    pub stage: ValidationStage,
    pub fixtures: Vec<ResourceRef>,
    pub seeds: Vec<u64>,
    pub methods: Vec<String>,
    pub metrics: Vec<NamedMetric>,
    pub status: ResultStatus,
    pub reason: Option<ReasonCode>,
    pub explanation: Option<String>,
    pub timing_source: TimingSource,
    pub resources: Vec<NamedMetric>,
    pub provenance: Vec<ResourceRef>,
    pub etalon_receipt: Option<ResourceRef>,
}
