use super::layout::{LayoutInput, LayoutProfile, PlanInput, TensorLayout};
use super::{BuildBudget, BuildError, BuildInput, BuildProfile};
use super::{PlanError, PlanErrorContext, PlanField, PlanOperation};
use crate::p64::layout::*;
use PlanField::*;

#[derive(Debug)]
pub struct BuildPlan<'a, 's> {
    pub(super) input: BuildInput<'a>,
    pub(super) entries: &'s [P64TensorEntry],
    pub(super) header: P64WeightHeader,
    pub(super) output_len: usize,
}

impl BuildPlan<'_, '_> {
    pub fn header(&self) -> &P64WeightHeader {
        &self.header
    }
    pub fn entries(&self) -> &[P64TensorEntry] {
        self.entries
    }
    pub fn output_len(&self) -> usize {
        self.output_len
    }
}

fn at(field: PlanField) -> PlanErrorContext {
    PlanErrorContext::at(field, None)
}

fn tensor_at(field: PlanField, index: usize) -> PlanErrorContext {
    PlanErrorContext::at(field, Some(index))
}

pub(super) fn layer_count(input: &BuildInput<'_>) -> u32 {
    u32::from_le_bytes(input.hparams_bytes[..4].try_into().unwrap())
}

/// Preflight all constraints before touching scratch. The returned plan exclusively
/// borrows the used scratch entries so validated offsets cannot subsequently change.
pub fn plan_into<'a, 's>(
    input: BuildInput<'a>,
    scratch: &'s mut [P64TensorEntry],
    budget: BuildBudget,
) -> Result<BuildPlan<'a, 's>, BuildError> {
    plan_into_detailed(input, scratch, budget).map_err(|error| error.kind)
}

/// The same prepared-payload planner with the exact failing field and operation.
pub fn plan_into_detailed<'a, 's>(
    input: BuildInput<'a>,
    scratch: &'s mut [P64TensorEntry],
    budget: BuildBudget,
) -> Result<BuildPlan<'a, 's>, PlanError> {
    let planned = plan_shared(input.into(), scratch, budget)?;
    Ok(BuildPlan {
        input,
        entries: planned.entries,
        header: planned.header,
        output_len: planned.output_len,
    })
}

#[derive(Debug)]
pub struct LayoutPlan<'a, 's> {
    pub(super) input: PlanInput<'a>,
    pub(super) entries: &'s [P64TensorEntry],
    pub(super) header: P64WeightHeader,
    pub(super) output_len: usize,
}
impl LayoutPlan<'_, '_> {
    pub fn header(&self) -> &P64WeightHeader {
        &self.header
    }
    pub fn entries(&self) -> &[P64TensorEntry] {
        self.entries
    }
    pub fn output_len(&self) -> usize {
        self.output_len
    }
}

pub fn plan_layout_into<'a, 's>(
    input: LayoutInput<'a>,
    scratch: &'s mut [P64TensorEntry],
    budget: BuildBudget,
) -> Result<LayoutPlan<'a, 's>, BuildError> {
    plan_layout_into_detailed(input, scratch, budget).map_err(|error| error.kind)
}

/// Detailed diagnostics preserve the existing planner's validation order.
pub fn plan_layout_into_detailed<'a, 's>(
    input: LayoutInput<'a>,
    scratch: &'s mut [P64TensorEntry],
    budget: BuildBudget,
) -> Result<LayoutPlan<'a, 's>, PlanError> {
    plan_shared(input.into(), scratch, budget)
}

fn plan_shared<'a, 's>(
    input: PlanInput<'a>,
    scratch: &'s mut [P64TensorEntry],
    budget: BuildBudget,
) -> Result<LayoutPlan<'a, 's>, PlanError> {
    if input.count() == 0 {
        return Err(at(TensorCount).error(BuildError::EmptyTensors));
    }
    let count = input.count();
    let scratch_bytes = at(EntryScratch).mul(count, P64_TENSOR_ENTRY_BYTES)?;
    if count > scratch.len() {
        return Err(at(EntryScratch).error(BuildError::ScratchCapacity {
            required_entries: count,
            available_entries: scratch.len(),
        }));
    }
    if scratch_bytes > budget.scratch_bytes {
        return Err(at(EntryScratch).error(BuildError::ScratchBudget {
            required_bytes: scratch_bytes,
            available_bytes: budget.scratch_bytes,
        }));
    }
    let page_log2 = if input.page_log2 == 0 {
        P64_DEFAULT_PAGE_LOG2
    } else {
        input.page_log2
    };
    if !(8..=30).contains(&page_log2) {
        return Err(at(PageSize).error(BuildError::InvalidPageSize));
    }
    let page = 1usize << page_log2;
    let structural = P64_FLAG_LITTLE_ENDIAN
        | P64_FLAG_RAW_TRANSCODE
        | P64_FLAG_LAYER_MAJOR
        | P64_FLAG_LAYER_PACK
        | P64_FLAG_LAYER_SCHEDULE;
    let expected_flags = match input.profile {
        LayoutProfile::FfnLegacy | LayoutProfile::Standard(BuildProfile::GgufLayerPacked) => {
            P64_FLAG_LITTLE_ENDIAN
                | P64_FLAG_LAYER_MAJOR
                | P64_FLAG_LAYER_PACK
                | P64_FLAG_LAYER_SCHEDULE
        }
        LayoutProfile::Standard(BuildProfile::Safetensor) => {
            P64_FLAG_LITTLE_ENDIAN | P64_FLAG_RAW_TRANSCODE
        }
        LayoutProfile::Standard(BuildProfile::Vision) => P64_FLAG_LITTLE_ENDIAN,
    };
    if input.flags & structural != expected_flags {
        return Err(at(ProfileFlags).error(BuildError::ProfileFlags));
    }
    let layers = input.layers();
    if !input.has_schedule() && !input.tokenizer_bytes.is_empty()
        || input.profile == LayoutProfile::Standard(BuildProfile::Vision) && layers != 0
    {
        return Err(at(ProfileSections).error(BuildError::ProfileSections));
    }
    let manifolds = layers.checked_add(1).ok_or_else(|| {
        PlanErrorContext {
            operation: PlanOperation::Add,
            ..at(ManifoldCount)
        }
        .error(BuildError::ArithmeticOverflow)
    })? as usize;
    if input.manifold_bytes.len() != at(ManifoldTable).mul(manifolds, P64_MANIFOLD_ENTRY_BYTES)? {
        return Err(at(ManifoldTable).error(BuildError::ManifoldCount));
    }
    let schedule_bytes = if input.has_schedule() {
        at(ScheduleTable).mul(layers as usize, 64)?
    } else {
        0
    };
    if (input.profile == LayoutProfile::FfnLegacy && input.schedule_bytes.len() != schedule_bytes)
        || (input.profile != LayoutProfile::FfnLegacy && !input.schedule_bytes.is_empty())
    {
        return Err(at(ScheduleTable).error(BuildError::ProfileSections));
    }
    let tensor_table = at(TensorTable).add(128, schedule_bytes)?;
    let strings = at(StringTable).add(tensor_table, scratch_bytes)?;
    let mut string_bytes =
        usize::from(input.profile != LayoutProfile::Standard(BuildProfile::Vision));
    for index in 0..input.count() {
        let tensor = input.tensor(index);
        if !(1..=4).contains(&tensor.rank) {
            return Err(tensor_at(TensorRank, index).error(BuildError::InvalidRank(index)));
        }
        if tensor.manifold_idx > layers {
            return Err(tensor_at(TensorManifold, index).error(BuildError::InvalidManifold(index)));
        }
        let blob = tensor_at(TensorBlobSize, index);
        blob.offset(blob.usize(tensor.blob_len)?)?;
        let name = tensor_at(TensorName, index);
        string_bytes = name.add(string_bytes, name.add(tensor.name.len(), 1)?)?;
    }
    let manifold_table =
        at(ManifoldTable).align(at(ManifoldTable).add(strings, string_bytes)?, 64)?;
    let tokenizer = at(Tokenizer).add(manifold_table, input.manifold_bytes.len())?;
    let checksum = at(ChecksumTable).align(
        at(ChecksumTable).add(tokenizer, input.tokenizer_bytes.len())?,
        64,
    )?;
    let checksum_end = at(ChecksumTable).add(
        checksum,
        at(ChecksumTable).mul(at(ChecksumTable).add(count, 1)?, 4)?,
    )?;
    let blob_start = at(BlobRegion).align(checksum_end, page)?;
    let output_len = walk_entries(&input, blob_start, page, None)?;
    at(ContainerEnd).offset(output_len)?;
    if output_len > budget.output_bytes {
        return Err(at(OutputBudget).error(BuildError::OutputBudget {
            required_bytes: output_len,
            available_bytes: budget.output_bytes,
        }));
    }
    let header = P64WeightHeader {
        magic: P64_MAGIC,
        version: P64_VERSION,
        flags: input.flags,
        role_table_offset: match input.profile {
            LayoutProfile::FfnLegacy | LayoutProfile::Standard(BuildProfile::GgufLayerPacked) => {
                128
            }
            LayoutProfile::Standard(BuildProfile::Safetensor) => 0,
            LayoutProfile::Standard(BuildProfile::Vision) => {
                at(TensorTable).offset(tensor_table)?
            }
        },
        tensor_table_offset: at(TensorTable).offset(tensor_table)?,
        tokenizer_offset: at(Tokenizer).offset(tokenizer)?,
        hparams_offset: 64,
        string_table_offset: at(StringTable).offset(strings)?,
        checksum_offset: at(ChecksumTable).offset(checksum)?,
        manifold_table_offset: at(ManifoldTable).offset(manifold_table)?,
        tensor_count: at(TensorCount).offset(count)?,
        page_size: at(PageSize).offset(page)?,
        reserved: input.header_reserved,
    };
    // Identical immutable inputs have already passed this deterministic walk.
    walk_entries(&input, blob_start, page, Some(&mut scratch[..count]))?;
    Ok(LayoutPlan {
        input,
        entries: &scratch[..count],
        header,
        output_len,
    })
}

fn walk_entries(
    input: &PlanInput<'_>,
    mut cursor: usize,
    page: usize,
    mut output: Option<&mut [P64TensorEntry]>,
) -> Result<usize, PlanError> {
    let mut name_offset =
        usize::from(input.profile != LayoutProfile::Standard(BuildProfile::Vision));
    let mut previous_manifold = None;
    for index in 0..input.count() {
        let tensor = input.tensor(index);
        let alignment = if input.profile == LayoutProfile::Standard(BuildProfile::GgufLayerPacked)
            && previous_manifold == Some(tensor.manifold_idx)
        {
            256
        } else {
            page
        };
        cursor = tensor_at(TensorBlobOffset, index).align(cursor, alignment)?;
        let entry = entry(
            &tensor,
            tensor_at(TensorName, index).offset(name_offset)?,
            tensor_at(TensorBlobOffset, index).offset(cursor)?,
            index,
        )?;
        if let Some(entries) = output.as_deref_mut() {
            entries[index] = entry;
        }
        cursor = tensor_at(TensorBlobEnd, index).add(
            cursor,
            tensor_at(TensorBlobSize, index).usize(tensor.blob_len)?,
        )?;
        let name = tensor_at(TensorName, index);
        name_offset = name.add(name_offset, name.add(tensor.name.len(), 1)?)?;
        previous_manifold = Some(tensor.manifold_idx);
    }
    match input.profile {
        LayoutProfile::FfnLegacy | LayoutProfile::Standard(BuildProfile::GgufLayerPacked) => {
            at(ContainerEnd).align(cursor, 64)
        }
        LayoutProfile::Standard(BuildProfile::Safetensor) => Ok(cursor),
        LayoutProfile::Standard(BuildProfile::Vision) => at(ContainerEnd).align(cursor, page),
    }
}

fn entry(
    tensor: &TensorLayout<'_>,
    name_offset: u32,
    blob_offset: u32,
    index: usize,
) -> Result<P64TensorEntry, PlanError> {
    Ok(P64TensorEntry {
        name_offset,
        role_id: tensor.role_id,
        dtype: tensor.dtype,
        manifold_idx: tensor.manifold_idx,
        rank: tensor.rank,
        dimensions: tensor.dimensions,
        blob_offset,
        blob_size: tensor_at(TensorBlobSize, index)
            .offset(tensor_at(TensorBlobSize, index).usize(tensor.blob_len)?)?,
        source_offset: tensor.source_offset,
        source_name_hash: tensor.source_name_hash,
        alt_dtype: tensor.alt_dtype,
        precision_views_mask: tensor.precision_views_mask,
        alt_blob_offset: tensor.alt_blob_offset,
    })
}
