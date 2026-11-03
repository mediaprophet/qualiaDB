use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GgufCompileBudget {
    pub output_bytes: usize,
    pub scratch_bytes: usize,
}

/// Capacity bounds, not an output-size prediction or proof of source validity.
/// Three extra slots cover cached global bindings in a caller-modified index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GgufCompileRequirements {
    pub tensor_capacity: usize,
    pub manifold_bytes: usize,
    pub scratch_bytes: usize,
}

/// Descriptor storage can be inspected after compilation. Its retained names
/// still borrow order records: reinitialize the whole descriptor array before
/// rebuilding order. Calling `.fill` alone does not end that Rust lifetime.
pub struct GgufCompileWorkspace<'names, 'work> {
    pub order: &'names mut [GgufCompileTensor],
    pub descriptors: &'work mut [TensorDescriptor<'names>],
    pub entries: &'work mut [etalon_model::p64::P64TensorEntry],
    pub manifolds: &'work mut [u8],
    pub hparams: &'work mut [u8],
}

pub fn gguf_verbatim_requirements(
    index: &GgufTensorIndex,
) -> Result<GgufCompileRequirements, GgufCompileError> {
    if index.hyperparams.n_layer > u16::MAX as u32 + 1 {
        return Err(GgufCompileError::LayerIndex);
    }
    let count = index
        .entries
        .len()
        .checked_add(3)
        .ok_or(GgufCompileError::WorkspaceOverflow)?;
    let manifolds = index
        .hyperparams
        .n_layer
        .checked_add(1)
        .ok_or(GgufCompileError::ManifoldCount)?;
    let manifold_bytes = usize::try_from(manifolds)
        .ok()
        .and_then(|n| n.checked_mul(64))
        .ok_or(GgufCompileError::WorkspaceOverflow)?;
    let scratch_bytes = charge(count, count, count, manifold_bytes, 64)?;
    Ok(GgufCompileRequirements {
        tensor_capacity: count,
        manifold_bytes,
        scratch_bytes,
    })
}

fn charge(
    order: usize,
    descriptors: usize,
    entries: usize,
    manifolds: usize,
    hparams: usize,
) -> Result<usize, GgufCompileError> {
    let mut total = 0usize;
    for (count, width) in [
        (order, core::mem::size_of::<GgufCompileTensor>()),
        (descriptors, core::mem::size_of::<TensorDescriptor<'_>>()),
        (entries, 64),
        (manifolds, 1),
        (hparams, 1),
    ] {
        total = total
            .checked_add(
                count
                    .checked_mul(width)
                    .ok_or(GgufCompileError::WorkspaceOverflow)?,
            )
            .ok_or(GgufCompileError::WorkspaceOverflow)?;
    }
    Ok(total)
}

pub(super) fn admit(
    workspace: &GgufCompileWorkspace<'_, '_>,
    required: GgufCompileRequirements,
    budget: GgufCompileBudget,
) -> Result<(), GgufCompileError> {
    let bytes = charge(
        workspace.order.len(),
        workspace.descriptors.len(),
        workspace.entries.len(),
        workspace.manifolds.len(),
        workspace.hparams.len(),
    )?;
    if bytes > budget.scratch_bytes {
        return Err(GgufCompileError::ScratchBudget {
            required: bytes,
            available: budget.scratch_bytes,
        });
    }
    for (buffer, required, available) in [
        (
            GgufCompileBuffer::Order,
            required.tensor_capacity,
            workspace.order.len(),
        ),
        (
            GgufCompileBuffer::Descriptors,
            required.tensor_capacity,
            workspace.descriptors.len(),
        ),
        (
            GgufCompileBuffer::Entries,
            required.tensor_capacity,
            workspace.entries.len(),
        ),
        (
            GgufCompileBuffer::Manifolds,
            required.manifold_bytes,
            workspace.manifolds.len(),
        ),
        (GgufCompileBuffer::Hparams, 64, workspace.hparams.len()),
    ] {
        if available < required {
            return Err(GgufCompileError::Capacity {
                buffer,
                required,
                available,
            });
        }
    }
    Ok(())
}
