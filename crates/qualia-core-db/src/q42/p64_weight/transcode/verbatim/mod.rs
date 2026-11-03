//! Qualia source interpretation with Etalon-owned layout and serialization.
mod emit;
mod error;
mod source;
#[cfg(test)]
mod tests;
mod workspace;

use super::TranscodeReport;
use crate::safetensor::SafeTensorPlan;
pub use error::{SafetensorTranscodeError, SafetensorTranscodeValidationError};
use etalon_model::p64::build::{
    self, BuildBudget, BuildProfile, LayoutInput, LayoutProfile, StreamScratch, TensorLayout,
    WorkspaceRequirements,
};
use etalon_model::p64::P64TensorEntry;
pub use workspace::{
    SafetensorTranscodeBudget, SafetensorTranscodeRequirements, SafetensorTranscodeWorkspace,
};

/// Sizing excludes the caller's existing parsed plan and source bytes.
pub fn safetensor_transcode_requirements(
    src: &[u8],
    plan: &SafeTensorPlan,
    page_log2: u16,
) -> Result<SafetensorTranscodeRequirements, SafetensorTranscodeValidationError> {
    Ok(source::inspect(src, plan, page_log2)?.requirements)
}

/// Allocates no adapter storage. Source/capacity/scratch refusals leave workspace
/// unchanged; later planning refusals may leave it prepared but never write output.
/// Sink failures may leave incomplete output. Parsing and sink storage are external.
pub fn transcode_safetensor_plan_to_p64<'p, W: std::io::Write>(
    src: &'p [u8],
    source_plan: &'p SafeTensorPlan,
    page_log2: u16,
    out: &mut W,
    workspace: SafetensorTranscodeWorkspace<'_, 'p>,
    budget: SafetensorTranscodeBudget,
) -> Result<TranscodeReport, SafetensorTranscodeError> {
    let summary = source::inspect(src, source_plan, page_log2)?;
    workspace.admit(summary.requirements, budget)?;
    let count = summary.requirements.tensor_count;
    let layouts = &mut workspace.layouts[..count];
    source::fill(source_plan, summary.layers, layouts);
    let manifolds = &mut workspace.manifolds[..summary.requirements.manifold_bytes];
    for (index, record) in manifolds.chunks_exact_mut(64).enumerate() {
        let coordinate = crate::modalities::manifold::ManifoldCoordinate10D::from_sequential_layer(
            index.min(summary.layers as usize) as u32,
            summary.layers.max(1),
        );
        super::write_manifold_coordinate(&coordinate, record);
    }
    let mut hparams = [0u8; 64];
    hparams[..4].copy_from_slice(&summary.layers.to_le_bytes());
    let build_budget = BuildBudget {
        output_bytes: budget.output_bytes,
        scratch_bytes: budget.scratch_bytes,
    };
    let plan = build::plan_layout_into_detailed(
        LayoutInput {
            profile: LayoutProfile::Standard(BuildProfile::Safetensor),
            page_log2,
            flags: 3,
            header_reserved: [0; 20],
            hparams_bytes: &hparams,
            manifold_bytes: manifolds,
            tokenizer_bytes: &[],
            schedule_bytes: &[],
            tensors: layouts,
        },
        &mut workspace.entries[..count],
        build_budget,
    )
    .map_err(SafetensorTranscodeError::Plan)?;
    let report = build::stream_into(
        &plan,
        &mut emit::Source {
            bytes: src,
            plan: source_plan,
        },
        &mut emit::Sink(out),
        StreamScratch {
            requirements: &mut workspace.requirements[..count],
            checksums: &mut workspace.checksums[..count],
            bytes: &mut [],
            floats: &mut [],
        },
        build_budget,
        &|| false,
    )
    .map_err(SafetensorTranscodeError::Stream)?;
    Ok(TranscodeReport {
        n_tensors: count,
        bytes_written: report.bytes_written,
        largest_tensor_bytes: summary.largest,
        total_tensor_bytes: summary.total,
        peak_working_bytes: summary.largest,
    })
}

// Retain the legacy wrapper's unbounded parser policy; new allocation sizes are
// checked first. The historical report does not measure these metadata buffers.
pub(super) fn transcode_compat<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    source::page(page_log2).map_err(|_| format!("p64: page_log2 {page_log2} out of range"))?;
    let plan = crate::safetensor::parse_safetensor_header(src)?;
    let required = safetensor_transcode_requirements(src, &plan, page_log2)
        .map_err(|error| error::validation_message(error, &plan))?;
    let count = required.tensor_count;
    let mut layouts = allocate(count, empty_layout())?;
    let mut entries = allocate(count, empty_entry())?;
    let mut requirements = allocate(count, WorkspaceRequirements::default())?;
    let mut checksums = allocate(count, 0u32)?;
    let mut manifolds = allocate(required.manifold_bytes, 0u8)?;
    transcode_safetensor_plan_to_p64(
        src,
        &plan,
        page_log2,
        out,
        SafetensorTranscodeWorkspace {
            layouts: &mut layouts,
            entries: &mut entries,
            requirements: &mut requirements,
            checksums: &mut checksums,
            manifolds: &mut manifolds,
        },
        SafetensorTranscodeBudget {
            output_bytes: u32::MAX as usize,
            scratch_bytes: required.scratch_bytes,
        },
    )
    .map_err(|error| error::legacy_message(error, &plan))
}

fn allocate<T: Clone>(count: usize, value: T) -> Result<Vec<T>, String> {
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(count)
        .map_err(|_| "p64: transcode workspace allocation failed".to_owned())?;
    buffer.resize(count, value);
    Ok(buffer)
}

pub(super) fn empty_layout<'a>() -> TensorLayout<'a> {
    TensorLayout {
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
        blob_len: 0,
    }
}

pub(super) fn empty_entry() -> P64TensorEntry {
    P64TensorEntry {
        name_offset: 0,
        role_id: 0,
        dtype: 0,
        manifold_idx: 0,
        rank: 1,
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
