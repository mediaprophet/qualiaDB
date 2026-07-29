use super::{PagedKvConfig, INVALID_BLOCK};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttentionError {
    InvalidShape,
    MissingPage,
    OutputTooSmall,
}

/// Scalar oracle for the production paged GQA online-softmax kernel.
///
/// `block_table` is one layer's logical-to-physical page slice. The function is zero-allocation
/// and writes one query head at a time into `out`.
pub fn paged_gqa_attention_into(
    query: &[f32],
    arena: &[f32],
    block_table: &[u32],
    config: &PagedKvConfig,
    position: u32,
    n_head: u32,
    out: &mut [f32],
) -> Result<(), AttentionError> {
    let head_dim = config.head_dim as usize;
    let n_kv = config.n_kv_head as usize;
    let n_head = n_head as usize;
    if !config.is_valid()
        || n_head == 0
        || n_head % n_kv != 0
        || position >= config.max_context
        || query.len() < n_head * head_dim
        || out.len() < n_head * head_dim
        || block_table.len() < config.logical_blocks_per_layer() as usize
        || arena.len() < config.block_elems() * config.physical_blocks as usize
    {
        return Err(AttentionError::InvalidShape);
    }
    let q_per_kv = n_head / n_kv;
    let slot_kv = config.slot_kv_elems() as usize;
    let block_elems = config.block_elems();
    let scale = 1.0f32 / (head_dim as f32).sqrt();

    for q_head in 0..n_head {
        let kv_head = q_head / q_per_kv;
        let q = &query[q_head * head_dim..(q_head + 1) * head_dim];
        let dst = &mut out[q_head * head_dim..(q_head + 1) * head_dim];
        dst.fill(0.0);
        let mut running_max = f32::NEG_INFINITY;
        let mut running_sum = 0.0f32;
        for past in 0..=position {
            let logical = (past / config.block_size) as usize;
            let physical = block_table[logical];
            if physical == INVALID_BLOCK || physical >= config.physical_blocks {
                return Err(AttentionError::MissingPage);
            }
            let offset = (past % config.block_size) as usize;
            let base = physical as usize * block_elems + offset * slot_kv * 2;
            let k_base = base + kv_head * head_dim;
            let v_base = base + slot_kv + kv_head * head_dim;
            let mut dot = 0.0f32;
            for dim in 0..head_dim {
                dot = q[dim].mul_add(arena[k_base + dim], dot);
            }
            let score = dot * scale;
            let next_max = running_max.max(score);
            let alpha = if running_sum == 0.0 {
                0.0
            } else {
                (running_max - next_max).exp()
            };
            let beta = (score - next_max).exp();
            running_sum = running_sum * alpha + beta;
            running_max = next_max;
            for dim in 0..head_dim {
                dst[dim] = dst[dim] * alpha + arena[v_base + dim] * beta;
            }
        }
        if running_sum <= 0.0 {
            return Err(AttentionError::InvalidShape);
        }
        for value in dst {
            *value /= running_sum;
        }
    }
    Ok(())
}
