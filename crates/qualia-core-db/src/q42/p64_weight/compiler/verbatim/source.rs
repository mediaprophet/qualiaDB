use super::*;

pub(super) fn source_blob<'a>(
    input: &'a [u8],
    index: &GgufTensorIndex,
    tensor: &GgufCompileTensor,
    position: usize,
) -> Result<(&'a [u8], u16, [u32; 4]), GgufCompileError> {
    let info = &tensor.info;
    let bytes = crate::ggml_quants::tensor_byte_len(info)
        .ok_or(GgufCompileError::UnsupportedType(info.ggml_type))?;
    let data_start =
        usize::try_from(index.tensor_data_start).map_err(|_| GgufCompileError::SourceOffset)?;
    let offset = usize::try_from(info.byte_offset).map_err(|_| GgufCompileError::SourceOffset)?;
    let start = data_start
        .checked_add(offset)
        .ok_or(GgufCompileError::SourceOffset)?;
    let end = start
        .checked_add(bytes)
        .ok_or(GgufCompileError::SourceLength)?;
    let raw = input
        .get(start..end)
        .ok_or(GgufCompileError::SourceBounds(position))?;
    let dtype = u16::try_from(info.ggml_type).map_err(|_| GgufCompileError::Dtype)?;
    let mut dimensions = [0; 4];
    for (target, source) in dimensions.iter_mut().zip(info.dims) {
        *target = u32::try_from(source).map_err(|_| GgufCompileError::Dimension)?;
    }
    Ok((raw, dtype, dimensions))
}

pub(super) fn prepare<'names: 'work, 'work>(
    input: &'names [u8],
    index: &GgufTensorIndex,
    tokenizer: &'names [u8],
    page: u16,
    workspace: GgufCompileWorkspace<'names, 'work>,
    budget: GgufCompileBudget,
) -> Result<BuildPlan<'work, 'work>, GgufCompileError> {
    validate_header(index, page)?;
    let required = gguf_verbatim_requirements(index)?;
    workspace::admit(&workspace, required, budget)?;
    let count = planning::order_into(index, workspace.order)?;
    for (position, tensor) in workspace.order[..count].iter_mut().enumerate() {
        source_blob(input, index, tensor, position)?;
        tensor.name_len =
            names::name_into(tensor.role, tensor.layer, tensor.hash, &mut tensor.name)?;
    }
    metadata::prepare(
        index,
        workspace.hparams,
        &mut workspace.manifolds[..required.manifold_bytes],
    );
    for (position, tensor) in workspace.order[..count].iter().enumerate() {
        let (blob, dtype, dimensions) = source_blob(input, index, tensor, position)?;
        workspace.descriptors[position] = TensorDescriptor {
            name: &tensor.name[..tensor.name_len],
            role_id: tensor.role,
            dtype,
            manifold_idx: if tensor.layer == P64_LAYER_GLOBAL {
                index.hyperparams.n_layer
            } else {
                tensor.layer as u32
            },
            rank: tensor.info.n_dims,
            dimensions,
            source_offset: tensor.info.byte_offset,
            source_name_hash: tensor.hash,
            alt_dtype: 0,
            precision_views_mask: 0,
            alt_blob_offset: 0,
            blob,
        };
    }
    let hparams: &[u8; 64] = (&workspace.hparams[..64]).try_into().unwrap();
    let request = BuildInput {
        profile: BuildProfile::GgufLayerPacked,
        page_log2: page,
        flags: P64_FLAG_LITTLE_ENDIAN
            | P64_FLAG_LAYER_MAJOR
            | P64_FLAG_LAYER_PACK
            | P64_FLAG_LAYER_SCHEDULE,
        header_reserved: [0; 20],
        hparams_bytes: hparams,
        manifold_bytes: &workspace.manifolds[..required.manifold_bytes],
        tokenizer_bytes: tokenizer,
        tensors: &workspace.descriptors[..count],
    };
    plan_into_detailed(
        request,
        &mut workspace.entries[..count],
        BuildBudget {
            output_bytes: budget.output_bytes,
            scratch_bytes: count
                .checked_mul(64)
                .ok_or(GgufCompileError::WorkspaceOverflow)?,
        },
    )
    .map_err(GgufCompileError::Planning)
}

pub(super) fn validate_header(index: &GgufTensorIndex, page: u16) -> Result<(), GgufCompileError> {
    if index.tensor_data_start == 0 || index.entries.is_empty() {
        return Err(GgufCompileError::NoTensors);
    }
    if page != 0 && !(8..=30).contains(&page) {
        return Err(GgufCompileError::Page(page));
    }
    Ok(())
}
