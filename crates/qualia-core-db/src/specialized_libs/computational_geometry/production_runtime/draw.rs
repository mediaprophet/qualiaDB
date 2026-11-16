//! P18: mask → bounded exclusive scan/compact → indexed draw commands.
//! Overflow zeros the caller command buffer so indirect args cannot go stale.

use crate::specialized_libs::computational_geometry::geometry_workspace::production_budget::ProductionBudget;

use super::selection::GPU_CULL_WORKGROUP_SIZE;
use super::{RuntimeError, ValidatedHierarchy};

/// WebGPU/D3D12 default max compute workgroups per dimension.
pub const MAX_DISPATCH_WORKGROUPS: u32 = 65_535;
/// 20-byte indexed-indirect record padded to 32-byte backend alignment.
pub const DRAW_INDEXED_STRIDE_BYTES: usize = 32;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawIndexedCommand {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
    pub pad: [u32; 3],
}

impl DrawIndexedCommand {
    pub const EMPTY: Self = Self {
        index_count: 0,
        instance_count: 0,
        first_index: 0,
        base_vertex: 0,
        first_instance: 0,
        pad: [0; 3],
    };
}

const _: () = assert!(core::mem::size_of::<DrawIndexedCommand>() == DRAW_INDEXED_STRIDE_BYTES);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawGeneration {
    pub compacted: usize,
    pub command_count: usize,
    pub dispatch_x: u32,
}

pub fn dispatch_workgroups(item_count: u32) -> Result<u32, RuntimeError> {
    if item_count == 0 {
        return Ok(0);
    }
    let groups = item_count
        .checked_add(GPU_CULL_WORKGROUP_SIZE - 1)
        .ok_or(RuntimeError::Capacity)?
        / GPU_CULL_WORKGROUP_SIZE;
    if groups == 0 || groups > MAX_DISPATCH_WORKGROUPS {
        return Err(RuntimeError::Capacity);
    }
    Ok(groups)
}

fn zero_commands(out: &mut [DrawIndexedCommand]) {
    for command in out.iter_mut() {
        *command = DrawIndexedCommand::EMPTY;
    }
}

/// Exclusive prefix-scan compact of a 0/1 mask. `out_indices` is the worklist.
pub fn scan_compact(
    mask: &[u32],
    scratch_prefix: &mut [u32],
    out_indices: &mut [u32],
    budget: &mut ProductionBudget<'_>,
) -> Result<usize, RuntimeError> {
    if scratch_prefix.len() < mask.len() {
        return Err(RuntimeError::Capacity);
    }
    budget.checkpoint(mask.len() as u64)?;
    let mut total = 0u32;
    for (i, &bit) in mask.iter().enumerate() {
        if bit > 1 {
            return Err(RuntimeError::InvalidCut);
        }
        scratch_prefix[i] = total;
        total = total.checked_add(bit).ok_or(RuntimeError::Capacity)?;
    }
    let compacted = total as usize;
    if compacted > out_indices.len() {
        out_indices.fill(0);
        return Err(RuntimeError::Capacity);
    }
    budget.checkpoint(mask.len() as u64)?;
    for i in 0..mask.len() {
        if mask[i] == 1 {
            out_indices[scratch_prefix[i] as usize] = i as u32;
        }
    }
    for slot in out_indices.iter_mut().skip(compacted) {
        *slot = 0;
    }
    Ok(compacted)
}

pub fn generate_indexed_draws(
    hierarchy: &ValidatedHierarchy<'_>,
    cut: &[u32],
    compact_indices: &[u32],
    out_commands: &mut [DrawIndexedCommand],
    budget: &mut ProductionBudget<'_>,
) -> Result<DrawGeneration, RuntimeError> {
    zero_commands(out_commands);
    if compact_indices.len() > out_commands.len() {
        return Err(RuntimeError::Capacity);
    }
    budget.checkpoint(compact_indices.len() as u64)?;
    for (slot, &mask_index) in compact_indices.iter().enumerate() {
        let node_index = *cut
            .get(mask_index as usize)
            .ok_or(RuntimeError::InvalidCut)? as usize;
        let node = hierarchy
            .nodes()
            .get(node_index)
            .ok_or(RuntimeError::InvalidCut)?;
        let index_count = u32::from(node.triangle_count).saturating_mul(3);
        if node.vertex_data_offset > i32::MAX as u32 {
            zero_commands(out_commands);
            return Err(RuntimeError::Capacity);
        }
        out_commands[slot] = DrawIndexedCommand {
            index_count,
            instance_count: 1,
            first_index: node.triangle_data_offset,
            base_vertex: node.vertex_data_offset as i32,
            first_instance: 0,
            pad: [0; 3],
        };
    }
    let dispatch_x = dispatch_workgroups(compact_indices.len() as u32)?;
    Ok(DrawGeneration {
        compacted: compact_indices.len(),
        command_count: compact_indices.len(),
        dispatch_x,
    })
}

pub fn compact_and_draw(
    hierarchy: &ValidatedHierarchy<'_>,
    cut: &[u32],
    mask: &[u32],
    scratch_prefix: &mut [u32],
    compact_indices: &mut [u32],
    out_commands: &mut [DrawIndexedCommand],
    budget: &mut ProductionBudget<'_>,
) -> Result<DrawGeneration, RuntimeError> {
    hierarchy.validate_cut(cut, budget)?;
    if mask.len() != cut.len() {
        zero_commands(out_commands);
        return Err(RuntimeError::InvalidCut);
    }
    match scan_compact(mask, scratch_prefix, compact_indices, budget) {
        Ok(count) => generate_indexed_draws(
            hierarchy,
            cut,
            &compact_indices[..count],
            out_commands,
            budget,
        ),
        Err(error) => {
            zero_commands(out_commands);
            Err(error)
        }
    }
}
