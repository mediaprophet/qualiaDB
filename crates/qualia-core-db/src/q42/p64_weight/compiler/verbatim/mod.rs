use super::*;
use etalon_model::p64::build::{
    plan_into_detailed, write_into, BuildBudget, BuildError, BuildInput, BuildPlan, BuildProfile,
    PlanError, PlanField, TensorDescriptor,
};
mod error;
pub(super) mod metadata;
mod source;
#[cfg(test)]
mod tests;
mod workspace;
pub use error::*;
pub use workspace::*;

/// Immutable prepared metadata and borrowed source payloads. Vec-backed caller
/// storage may be allocated once and written repeatedly without allocation.
/// While this plan or retained descriptors borrow names, their backing order
/// storage cannot be rebound. Dynamic scratch rebinding is not promised.
pub struct GgufCompilePlan<'work> {
    plan: BuildPlan<'work, 'work>,
}

impl GgufCompilePlan<'_> {
    pub fn output_len(&self) -> usize {
        self.plan.output_len()
    }

    /// Writes the planned prefix, preserving the tail and all bytes on refusal.
    pub fn write_into(&self, output: &mut [u8]) -> Result<usize, GgufCompileError> {
        write_into(&self.plan, output).map_err(GgufCompileError::Writing)
    }
}

/// Freeze canonical metadata using admitted caller workspace. The parsed index
/// is copied into immutable plan metadata; source/tokenizer/name bytes remain
/// borrowed. Preparation may change scratch on refusal, never output.
pub fn prepare_gguf_index_to_p64<'names: 'work, 'work>(
    input: &'names [u8],
    index: &GgufTensorIndex,
    tokenizer: &'names [u8],
    page_log2: u16,
    workspace: GgufCompileWorkspace<'names, 'work>,
    budget: GgufCompileBudget,
) -> Result<GgufCompilePlan<'work>, GgufCompileError> {
    source::prepare(input, index, tokenizer, page_log2, workspace, budget)
        .map(|plan| GgufCompilePlan { plan })
}

/// Compile using caller-owned output and workspace, without adapter allocation.
/// Parsed index, source and already serialized tokenizer storage are outside the
/// budget. Every supplied workspace capacity is charged before mutation. Ordering
/// and metadata preparation may change scratch on later source/planning refusal;
/// output is unchanged on every refusal and its spare tail is left untouched.
pub fn compile_gguf_index_to_p64_into<'names: 'work, 'work>(
    input: &'names [u8],
    index: &GgufTensorIndex,
    tokenizer: &'names [u8],
    page_log2: u16,
    output: &mut [u8],
    workspace: GgufCompileWorkspace<'names, 'work>,
    budget: GgufCompileBudget,
) -> Result<usize, GgufCompileError> {
    let plan = prepare_gguf_index_to_p64(input, index, tokenizer, page_log2, workspace, budget)?;
    plan.write_into(output)
}

pub(super) fn compile_compat(input: &[u8], page: u16) -> Result<Vec<u8>, String> {
    let index = GgufTensorIndex::from_gguf(input);
    source::validate_header(&index, page).map_err(|e| e.to_string())?;
    let required = gguf_verbatim_requirements(&index).map_err(|e| e.to_string())?;
    // Parsing/tokenizer allocations retain their existing cold policy; this is
    // not a total-transcoder-memory budget. Adapter buffers are checked below.
    let mut tokenizer = crate::gguf_sharder::GgufTokenizer::from_gguf(input).to_p64_section();
    native_metadata::append(&index, &mut tokenizer)?;
    let mut order = allocate(required.tensor_capacity, GgufCompileTensor::default())?;
    let mut descriptors = allocate(required.tensor_capacity, empty_descriptor())?;
    let mut entries = allocate(required.tensor_capacity, empty_entry())?;
    let mut manifolds = allocate(required.manifold_bytes, 0u8)?;
    let mut hparams = [0u8; 64];
    let plan = source::prepare(
        input,
        &index,
        &tokenizer,
        page,
        GgufCompileWorkspace {
            order: &mut order,
            descriptors: &mut descriptors,
            entries: &mut entries,
            manifolds: &mut manifolds,
            hparams: &mut hparams,
        },
        GgufCompileBudget {
            output_bytes: u32::MAX as usize,
            scratch_bytes: required.scratch_bytes,
        },
    )
    .map_err(|e| e.to_string())?;
    let mut output = allocate(plan.output_len(), 0u8)?;
    write_into(&plan, &mut output).map_err(|e| GgufCompileError::Writing(e).to_string())?;
    Ok(output)
}

fn allocate<T: Clone>(count: usize, value: T) -> Result<Vec<T>, String> {
    count
        .checked_mul(core::mem::size_of::<T>())
        .ok_or("p64: workspace size overflow")?;
    let mut out = Vec::new();
    out.try_reserve_exact(count)
        .map_err(|_| "p64: allocation failed")?;
    out.resize(count, value);
    Ok(out)
}

pub(super) fn empty_descriptor<'a>() -> TensorDescriptor<'a> {
    TensorDescriptor {
        name: &[],
        role_id: 0,
        dtype: 0,
        manifold_idx: 0,
        rank: 1,
        dimensions: [0; 4],
        source_offset: 0,
        source_name_hash: 0,
        alt_dtype: 0,
        precision_views_mask: 0,
        alt_blob_offset: 0,
        blob: &[],
    }
}

pub(super) fn empty_entry() -> etalon_model::p64::P64TensorEntry {
    etalon_model::p64::P64TensorEntry {
        name_offset: 0,
        role_id: 0,
        dtype: 0,
        manifold_idx: 0,
        rank: 0,
        dimensions: [0; 4],
        blob_offset: 0,
        blob_size: 0,
        source_offset: 0,
        source_name_hash: 0,
        alt_dtype: 0,
        precision_views_mask: 0,
        alt_blob_offset: 0,
    }
}
