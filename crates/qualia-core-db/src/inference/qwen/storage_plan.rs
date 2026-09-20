//! Qwen4Exp storage admission with an NVMe-resident PLE table.
//!
//! This is deliberately separate from the generic model-size planner: the PLE
//! hash table is selected by direct row reads and must not be charged as a
//! resident host/GPU tensor.  Remaining trunk overflow is explicit; it can
//! only be admitted when a later execution plan implements its streamed path.

use crate::gguf_sharder::GgufTensorIndex;

pub const DEFAULT_QWEN4EXP_HOST_OS_FLOOR: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qwen4ExpStorageError {
    MissingPleTensor,
    InvalidPleTensor,
    GpuBudgetExhausted,
    HostBudgetExhausted,
}

/// Byte-precise placement receipt for Qwen4Exp activation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Qwen4ExpStoragePlan {
    pub model_file_bytes: u64,
    /// Table stays in the model file and is accessed by `PleNvmeReader` only.
    pub ple_nvme_bytes: u64,
    pub trunk_weight_bytes: u64,
    pub gpu_kv_reserve_bytes: u64,
    pub gpu_trunk_bytes: u64,
    pub host_os_floor_bytes: u64,
    pub host_ple_staging_bytes: u64,
    pub host_trunk_bytes: u64,
    /// Must be serviced by an explicit trunk streaming executor. Non-zero is
    /// not a successful resident activation.
    pub trunk_streamed_bytes: u64,
}

impl Qwen4ExpStoragePlan {
    pub fn requires_trunk_streaming(&self) -> bool {
        self.trunk_streamed_bytes != 0
    }

    pub fn resident_weight_bytes(&self) -> u64 {
        self.gpu_trunk_bytes.saturating_add(self.host_trunk_bytes)
    }
}

/// Create a storage placement receipt without probing hardware or allocating.
///
/// `gpu_budget_bytes` and `host_budget_bytes` are already-reserved budgets,
/// not physical totals. `host_ple_staging_bytes` is the bounded caller-owned
/// gather slab for the current micro-batch.
pub fn plan_qwen4exp_storage(
    index: &GgufTensorIndex,
    model_file_bytes: u64,
    gpu_budget_bytes: u64,
    host_budget_bytes: u64,
    gpu_kv_reserve_bytes: u64,
    host_ple_staging_bytes: u64,
    host_os_floor_bytes: u64,
) -> Result<Qwen4ExpStoragePlan, Qwen4ExpStorageError> {
    let ple = index
        .ple_ngram_embedding_info()
        .ok_or(Qwen4ExpStorageError::MissingPleTensor)?;
    let ple_nvme_bytes = crate::ggml_quants::tensor_byte_len(ple)
        .ok_or(Qwen4ExpStorageError::InvalidPleTensor)? as u64;
    if ple_nvme_bytes > model_file_bytes || gpu_budget_bytes <= gpu_kv_reserve_bytes {
        return Err(Qwen4ExpStorageError::GpuBudgetExhausted);
    }
    let host_required = host_os_floor_bytes.saturating_add(host_ple_staging_bytes);
    if host_budget_bytes <= host_required {
        return Err(Qwen4ExpStorageError::HostBudgetExhausted);
    }

    let trunk_weight_bytes = model_file_bytes - ple_nvme_bytes;
    let gpu_trunk_bytes = trunk_weight_bytes.min(gpu_budget_bytes - gpu_kv_reserve_bytes);
    let remaining = trunk_weight_bytes - gpu_trunk_bytes;
    let host_trunk_bytes = remaining.min(host_budget_bytes - host_required);

    Ok(Qwen4ExpStoragePlan {
        model_file_bytes,
        ple_nvme_bytes,
        trunk_weight_bytes,
        gpu_kv_reserve_bytes,
        gpu_trunk_bytes,
        host_os_floor_bytes,
        host_ple_staging_bytes,
        host_trunk_bytes,
        trunk_streamed_bytes: remaining - host_trunk_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gguf_sharder::{GgufHyperparams, GgufTensorInfo};

    #[test]
    fn recognizes_when_nvme_ple_alone_is_not_the_remaining_trunk_problem() {
        let ple = GgufTensorInfo {
            dims: [32, 16, 1, 1],
            n_dims: 2,
            ggml_type: crate::ggml_quants::GGML_TYPE_F32,
            byte_offset: 0,
        };
        let index = GgufTensorIndex::from_components(
            &[(b"per_layer_token_embd.weight", ple)],
            GgufHyperparams::default(),
            0,
        );
        let plan =
            plan_qwen4exp_storage(&index, 10 * 1024, 3 * 1024, 5 * 1024, 1024, 512, 512).unwrap();

        assert_eq!(plan.ple_nvme_bytes, 32 * 16 * 4);
        assert!(plan.requires_trunk_streaming());
        assert_eq!(
            plan.resident_weight_bytes(),
            plan.gpu_trunk_bytes + plan.host_trunk_bytes
        );
    }
}
