use etalon_inference::cpu_gguf::{
    inspect_dense_gguf, prepare_dense_gguf_into, BindingLimits, GgufCpuError, PackedTensor,
};
use etalon_model::gguf::ParseLimits;
use etalon_model_io::{
    lease::{LeaseIdentity, ModelLease},
    snapshot::Snapshot,
};
use qualia_inference_kernel::dense_cpu::{DenseLayer, DensePlan, DensePlanError};

/// This borrow retains the complete immutable resource owner, including snapshot
/// cleanup ownership. No pathname reopening, raw mapping or standalone lease
/// extraction is available. This does not attest who supplied the expected SHA.
pub enum ImmutableModel<'a> {
    Direct(&'a ModelLease),
    Snapshot(&'a Snapshot),
}
impl ImmutableModel<'_> {
    pub(super) fn bytes(&self) -> &[u8] {
        match self {
            Self::Direct(owner) => owner.bytes(),
            Self::Snapshot(owner) => owner.bytes(),
        }
    }
    pub(super) fn identity(&self) -> LeaseIdentity {
        match self {
            Self::Direct(owner) => owner.identity(),
            Self::Snapshot(owner) => owner.identity(),
        }
    }
}

/// First profile consumes one GGUF only. Its complete digest includes embedded
/// tokenizer/config metadata. A future multi-resource loader must use the
/// canonical ordered bundle contract; do not treat this as companion coverage.
pub(super) fn model_digest(owner: &ImmutableModel<'_>) -> [u8; 32] {
    owner.identity().content_sha256
}

pub(super) struct PreparedCpuModel<'data, 'tables> {
    pub(super) plan: DensePlan<'tables, PackedTensor<'data>>,
    pub(super) model_digest: [u8; 32],
}
#[derive(Debug)]
pub enum ModelPreparationError {
    Gguf(GgufCpuError),
    Plan(DensePlanError),
}

/// The caller retains owner and storage separately; no self-referential owned
/// struct, cloned mapping or mutable-public tensor index enters this path.
pub(super) fn prepare_model<'data, 'tables>(
    owner: &'data ImmutableModel<'_>,
    layers: &'tables mut [DenseLayer<PackedTensor<'data>>],
    context: usize,
    parse: ParseLimits,
    binding: BindingLimits,
    workspace_budget: usize,
) -> Result<PreparedCpuModel<'data, 'tables>, ModelPreparationError> {
    let config =
        inspect_dense_gguf(owner.bytes(), parse, context).map_err(ModelPreparationError::Gguf)?;
    let model =
        prepare_dense_gguf_into(&config, layers, binding).map_err(ModelPreparationError::Gguf)?;
    Ok(PreparedCpuModel {
        plan: DensePlan::new(model, workspace_budget).map_err(ModelPreparationError::Plan)?,
        model_digest: model_digest(owner),
    })
}
