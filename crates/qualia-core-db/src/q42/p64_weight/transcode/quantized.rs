use super::*;

/// Phase 6 / task #12 — **streaming, versioned transcode: safetensor (high-fidelity) → P64**.
///
/// Writes a valid P64 container to `out` forward-only (round-trips through [`P64TensorIndex::from_p64`]).
/// The full layout is computed from the safetensor *header* alone (no tensor reads), so each tensor's
/// bytes pass through **one reused scratch buffer** — the transcoder's peak working memory is ≈ the
/// largest single tensor, not the whole file. On the real path `src` is an `mmap` (demand-paged by
/// the OS), so the file is never resident in full.
///
/// Rejects low-precision (`Q4`-class) inputs — high-fidelity (`F32/F16/BF16/Q8`) only. The legacy
/// `compile_gguf_to_q42` path is untouched (GGUF support unchanged).
pub(super) use etalon_model::p64::convert::SafetensorQuantization;

pub(super) fn transcode_safetensor_with_policy<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
    policy: SafetensorQuantization,
) -> Result<TranscodeReport, String> {
    if matches!(policy, SafetensorQuantization::Verbatim) {
        return verbatim::transcode_compat(src, page_log2, out);
    }
    use crate::safetensor::{
        ggml_elem_bytes, is_high_fidelity_ggml, parse_safetensor_header, safetensor_dtype_to_ggml,
    };
    let page_log2 = if page_log2 == 0 {
        P64_DEFAULT_PAGE_LOG2
    } else {
        page_log2
    };
    if !(8..=30).contains(&page_log2) {
        return Err(format!("p64: page_log2 {page_log2} out of range"));
    }
    let plan = parse_safetensor_header(src)?;
    if plan.tensors.is_empty() {
        return Err("p64: Safetensors source contains no tensors".to_string());
    }

    let mut source_types = Vec::with_capacity(plan.tensors.len());
    let mut roles = Vec::with_capacity(plan.tensors.len());
    let mut n_layer = 0u32;
    for tensor in &plan.tensors {
        let ggml_type = safetensor_dtype_to_ggml(&tensor.dtype).ok_or_else(|| {
            format!(
                "p64: tensor '{}' dtype {} is not a supported high-fidelity source",
                tensor.name, tensor.dtype
            )
        })?;
        if !is_high_fidelity_ggml(ggml_type) {
            return Err(format!(
                "p64: tensor '{}' is low precision and cannot be transcoded",
                tensor.name
            ));
        }
        let element_count = tensor
            .shape
            .iter()
            .try_fold(1usize, |count, dimension| count.checked_mul(*dimension))
            .ok_or_else(|| format!("p64: tensor '{}' shape overflow", tensor.name))?;
        let expected_bytes = element_count
            .checked_mul(ggml_elem_bytes(ggml_type).ok_or("p64: unsupported element width")?)
            .ok_or_else(|| format!("p64: tensor '{}' byte-size overflow", tensor.name))?;
        if expected_bytes != tensor.byte_len() {
            return Err(format!(
                "p64: tensor '{}' declares {} bytes but shape/dtype requires {}",
                tensor.name,
                tensor.byte_len(),
                expected_bytes
            ));
        }
        let role = crate::tensor_roles::name_to_role(&tensor.name);
        if let Some(mapped) = role {
            if mapped.layer != P64_LAYER_GLOBAL {
                n_layer = n_layer.max(mapped.layer as u32 + 1);
            }
        }
        source_types.push(ggml_type);
        roles.push(role);
    }

    let tensors = plan
        .tensors
        .iter()
        .enumerate()
        .map(|(i, t)| etalon_model::p64::convert::QuantizedTensor {
            name: &t.name,
            shape: &t.shape,
            begin: t.begin,
            end: t.end,
            ggml_type: source_types[i],
            role: roles[i],
        })
        .collect::<Vec<_>>();
    let mut manifolds = vec![
        0u8;
        (n_layer as usize + 1)
            .checked_mul(64)
            .ok_or("p64: manifold table overflow")?
    ];
    for (layer, bytes) in manifolds.chunks_exact_mut(64).enumerate() {
        let coordinate = crate::modalities::manifold::ManifoldCoordinate10D::from_sequential_layer(
            layer.min(n_layer as usize) as u32,
            n_layer.max(1),
        );
        write_manifold_coordinate(&coordinate, bytes);
    }
    etalon_model::p64::convert::transcode_quantized(
        src,
        plan.data_start,
        &tensors,
        n_layer,
        &manifolds,
        Default::default(),
        page_log2,
        out,
        policy,
    )
}
