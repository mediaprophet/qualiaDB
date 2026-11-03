use super::super::{P64_DEFAULT_PAGE_LOG2, P64_LAYER_GLOBAL, P64_ROLE_UNKNOWN};
use super::{
    workspace, SafetensorTranscodeRequirements, SafetensorTranscodeValidationError as Error,
};
use crate::safetensor::{
    ggml_elem_bytes, is_high_fidelity_ggml, safetensor_dtype_to_ggml, SafeTensorPlan,
};
use etalon_model::p64::build::TensorLayout;

pub(super) struct Summary {
    pub requirements: SafetensorTranscodeRequirements,
    pub layers: u32,
    pub largest: usize,
    pub total: usize,
}

pub(super) fn page(page: u16) -> Result<u16, Error> {
    let value = if page == 0 {
        P64_DEFAULT_PAGE_LOG2
    } else {
        page
    };
    if !(8..=30).contains(&value) {
        return Err(Error::Page(value));
    }
    Ok(value)
}

pub(super) fn inspect(src: &[u8], plan: &SafeTensorPlan, page_log2: u16) -> Result<Summary, Error> {
    page(page_log2)?;
    if plan.data_start > src.len() {
        return Err(Error::DataStart);
    }
    if plan.tensors.is_empty() {
        return Err(Error::Empty);
    }
    let data_len = src.len() - plan.data_start;
    for (index, tensor) in plan.tensors.iter().enumerate() {
        if tensor.end < tensor.begin || tensor.end > data_len {
            return Err(Error::Offsets(index));
        }
    }
    let mut layers = 0;
    for (index, tensor) in plan.tensors.iter().enumerate() {
        let dtype = safetensor_dtype_to_ggml(&tensor.dtype).ok_or(Error::UnsupportedType(index))?;
        if !is_high_fidelity_ggml(dtype) {
            return Err(Error::LowPrecision(index));
        }
        let count = tensor
            .shape
            .iter()
            .try_fold(1usize, |n, d| n.checked_mul(*d))
            .ok_or(Error::ShapeOverflow(index))?;
        let expected = count
            .checked_mul(ggml_elem_bytes(dtype).ok_or(Error::UnsupportedWidth)?)
            .ok_or(Error::ByteSizeOverflow(index))?;
        if expected != tensor.byte_len() {
            return Err(Error::ShapeMismatch {
                index,
                declared: tensor.byte_len(),
                expected,
            });
        }
        if let Some(mapped) = crate::tensor_roles::name_to_role(&tensor.name) {
            if mapped.layer != P64_LAYER_GLOBAL {
                layers = layers.max(u32::from(mapped.layer) + 1);
            }
        }
    }
    let mut names = 1usize;
    for tensor in &plan.tensors {
        u32::try_from(names).map_err(|_| Error::NameTable)?;
        names = names
            .checked_add(tensor.name.len())
            .and_then(|n| n.checked_add(1))
            .ok_or(Error::NameTable)?;
    }
    let mut largest = 0;
    let mut total = 0usize;
    for tensor in &plan.tensors {
        for dimension in tensor.shape.iter().take(4) {
            u32::try_from(*dimension).map_err(|_| Error::Dimension)?;
        }
        u32::try_from(tensor.byte_len()).map_err(|_| Error::TensorSize)?;
        largest = largest.max(tensor.byte_len());
        total = total
            .checked_add(tensor.byte_len())
            .ok_or(Error::TensorTotal)?;
    }
    let count = plan.tensors.len();
    let manifolds = (layers as usize)
        .checked_add(1)
        .and_then(|n| n.checked_mul(64))
        .ok_or(Error::WorkspaceOverflow)?;
    let scratch = workspace::bytes(count, count, count, count, manifolds)?;
    Ok(Summary {
        requirements: SafetensorTranscodeRequirements {
            tensor_count: count,
            manifold_bytes: manifolds,
            scratch_bytes: scratch,
        },
        layers,
        largest,
        total,
    })
}

pub(super) fn fill<'a>(plan: &'a SafeTensorPlan, layers: u32, out: &mut [TensorLayout<'a>]) {
    for (tensor, target) in plan.tensors.iter().zip(out) {
        let mapped = crate::tensor_roles::name_to_role(&tensor.name);
        let mut dimensions = [0; 4];
        for (to, from) in dimensions.iter_mut().zip(&tensor.shape) {
            *to = *from as u32;
        }
        *target = TensorLayout {
            name: tensor.name.as_bytes(),
            role_id: mapped.map(|r| r.role).unwrap_or(P64_ROLE_UNKNOWN),
            dtype: safetensor_dtype_to_ggml(&tensor.dtype).expect("validated immutable dtype")
                as u16,
            manifold_idx: mapped
                .filter(|r| r.layer != P64_LAYER_GLOBAL)
                .map(|r| u32::from(r.layer))
                .unwrap_or(layers),
            rank: tensor.shape.len().clamp(1, 4) as u32,
            dimensions,
            source_offset: tensor.begin as u64,
            source_name_hash: crate::q_hash(&tensor.name),
            alt_dtype: 0,
            precision_views_mask: 0,
            alt_blob_offset: 0,
            blob_len: tensor.byte_len() as u64,
        };
    }
}
