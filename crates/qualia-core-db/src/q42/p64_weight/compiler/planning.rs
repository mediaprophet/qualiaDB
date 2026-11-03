use super::*;

/// Caller storage for semantic ordering and bounded canonical names. Fields are
/// private; every compile reconstructs them from the borrowed source index.
#[derive(Clone, Copy, Debug)]
pub struct GgufCompileTensor {
    pub(super) role: u16,
    pub(super) layer: u16,
    pub(super) hash: u64,
    pub(super) info: GgufTensorInfo,
    pub(super) name: [u8; 64],
    pub(super) name_len: usize,
}
impl Default for GgufCompileTensor {
    fn default() -> Self {
        Self {
            role: 0,
            layer: 0,
            hash: 0,
            info: GgufTensorInfo {
                dims: [0; 4],
                n_dims: 0,
                ggml_type: 0,
                byte_offset: 0,
            },
            name: [0; 64],
            name_len: 0,
        }
    }
}

pub(super) fn order_into(
    index: &GgufTensorIndex,
    out: &mut [GgufCompileTensor],
) -> Result<usize, GgufCompileError> {
    let mut records = FixedRecords { out, used: 0 };
    order(index, &mut records)?;
    Ok(records.used)
}

fn order(index: &GgufTensorIndex, out: &mut impl OrderStorage) -> Result<(), GgufCompileError> {
    let mut push_known =
        |role, layer, candidate: Option<GgufTensorInfo>| -> Result<(), GgufCompileError> {
            if let Some(info) = candidate {
                if out.contains_offset(info.byte_offset) {
                    return Ok(());
                }
                let hash = index
                    .entries
                    .iter()
                    .find(|(_, source)| source.byte_offset == info.byte_offset)
                    .map(|(hash, _)| *hash)
                    .unwrap_or(0);
                out.push_tensor(role, layer, hash, info)?;
            }
            Ok(())
        };
    for layer in 0..index.hyperparams.n_layer {
        let t = index.get_layer_tensors(layer);
        let layer = u16::try_from(layer).map_err(|_| GgufCompileError::LayerIndex)?;
        push_known(P64_ROLE_ATTN_NORM, layer, t.attn_norm)?;
        push_known(P64_ROLE_ATTN_Q, layer, t.attn_q)?;
        push_known(P64_ROLE_ATTN_K, layer, t.attn_k)?;
        push_known(P64_ROLE_ATTN_V, layer, t.attn_v)?;
        push_known(P64_ROLE_ATTN_OUTPUT, layer, t.attn_output)?;
        push_known(P64_ROLE_FFN_NORM, layer, t.ffn_norm)?;
        push_known(P64_ROLE_FFN_GATE, layer, t.ffn_gate)?;
        push_known(P64_ROLE_FFN_UP, layer, t.ffn_up)?;
        push_known(P64_ROLE_FFN_DOWN, layer, t.ffn_down)?;
        push_known(P64_ROLE_MOE_ROUTER, layer, t.moe_router)?;
        push_known(P64_ROLE_MOE_GATE_EXPS, layer, t.moe_gate_exps)?;
        push_known(P64_ROLE_MOE_UP_EXPS, layer, t.moe_up_exps)?;
        push_known(P64_ROLE_MOE_DOWN_EXPS, layer, t.moe_down_exps)?;
        push_known(P64_ROLE_MOE_SHARED_GATE, layer, t.moe_shared_gate)?;
        push_known(P64_ROLE_MOE_SHARED_UP, layer, t.moe_shared_up)?;
        push_known(P64_ROLE_MOE_SHARED_DOWN, layer, t.moe_shared_down)?;
    }
    push_known(
        P64_ROLE_TOKEN_EMBD,
        P64_LAYER_GLOBAL,
        index.token_embd_info().copied(),
    )?;
    push_known(
        P64_ROLE_OUTPUT,
        P64_LAYER_GLOBAL,
        index.output_weight_info().copied(),
    )?;
    push_known(
        P64_ROLE_OUTPUT_NORM,
        P64_LAYER_GLOBAL,
        index.output_norm_info().copied(),
    )?;
    let known = out.len();
    for (hash, info) in &index.entries {
        if !out.contains_offset(info.byte_offset) {
            out.push_tensor(P64_ROLE_UNKNOWN, P64_LAYER_GLOBAL, *hash, *info)?;
        }
    }
    // Offsets are unique after deduplication, so unstable sorting has the same
    // order as the original stable sort and needs no heap scratch.
    out.sort_unknowns(known);
    Ok(())
}

trait OrderStorage {
    fn len(&self) -> usize;
    fn contains_offset(&self, offset: u64) -> bool;
    fn push_tensor(
        &mut self,
        role: u16,
        layer: u16,
        hash: u64,
        info: GgufTensorInfo,
    ) -> Result<(), GgufCompileError>;
    fn sort_unknowns(&mut self, start: usize);
}

struct FixedRecords<'a> {
    out: &'a mut [GgufCompileTensor],
    used: usize,
}
impl OrderStorage for FixedRecords<'_> {
    fn len(&self) -> usize {
        self.used
    }
    fn contains_offset(&self, offset: u64) -> bool {
        self.out[..self.used]
            .iter()
            .any(|record| record.info.byte_offset == offset)
    }
    fn push_tensor(
        &mut self,
        role: u16,
        layer: u16,
        hash: u64,
        info: GgufTensorInfo,
    ) -> Result<(), GgufCompileError> {
        let available = self.out.len();
        let target = self
            .out
            .get_mut(self.used)
            .ok_or(GgufCompileError::Capacity {
                buffer: GgufCompileBuffer::Order,
                required: self.used + 1,
                available,
            })?;
        *target = GgufCompileTensor {
            role,
            layer,
            hash,
            info,
            ..Default::default()
        };
        self.used += 1;
        Ok(())
    }
    fn sort_unknowns(&mut self, start: usize) {
        self.out[start..self.used].sort_unstable_by_key(|record| record.info.byte_offset);
    }
}

type PlannedTensor = (u16, u16, u64, GgufTensorInfo);
impl OrderStorage for Vec<PlannedTensor> {
    fn len(&self) -> usize {
        Vec::len(self)
    }
    fn contains_offset(&self, offset: u64) -> bool {
        self.iter().any(|record| record.3.byte_offset == offset)
    }
    fn push_tensor(
        &mut self,
        role: u16,
        layer: u16,
        hash: u64,
        info: GgufTensorInfo,
    ) -> Result<(), GgufCompileError> {
        self.push((role, layer, hash, info));
        Ok(())
    }
    fn sort_unknowns(&mut self, start: usize) {
        self[start..].sort_unstable_by_key(|record| record.3.byte_offset);
    }
}

pub(super) fn ordered_tensors(index: &GgufTensorIndex) -> Result<Vec<PlannedTensor>, String> {
    let mut records = Vec::with_capacity(index.entries.len());
    order(index, &mut records).map_err(|error| error.to_string())?;
    Ok(records)
}
