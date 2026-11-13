use super::plan::layer_count;
use super::{BuildError, BuildPlan, BuildProfile};
use crate::p64::integrity::crc32c;
use crate::p64::layout::P64TensorEntry;

/// Writes exactly the planned prefix, leaving spare output capacity untouched.
/// Insufficient capacity leaves the entire output unchanged. No allocation occurs.
pub fn write_into(plan: &BuildPlan<'_, '_>, output: &mut [u8]) -> Result<usize, BuildError> {
    if output.len() < plan.output_len {
        return Err(BuildError::OutputCapacity {
            required_bytes: plan.output_len,
            available_bytes: output.len(),
        });
    }
    let bytes = &mut output[..plan.output_len];
    bytes.fill(0);
    plan.header.write_le(&mut bytes[..64]);
    bytes[64..128].copy_from_slice(plan.input.hparams_bytes);
    let strings = plan.header.string_table_offset as usize;
    let manifold = plan.header.manifold_table_offset as usize;
    let tokenizer = plan.header.tokenizer_offset as usize;
    let checksums = plan.header.checksum_offset as usize;
    bytes[manifold..manifold + plan.input.manifold_bytes.len()]
        .copy_from_slice(plan.input.manifold_bytes);
    bytes[tokenizer..tokenizer + plan.input.tokenizer_bytes.len()]
        .copy_from_slice(plan.input.tokenizer_bytes);
    for (index, (entry, tensor)) in plan.entries.iter().zip(plan.input.tensors).enumerate() {
        let start = plan.header.tensor_table_offset as usize + index * 64;
        write_entry(entry, &mut bytes[start..start + 64]);
        let name_start = strings + entry.name_offset as usize;
        bytes[name_start..name_start + tensor.name.len()].copy_from_slice(tensor.name);
        let blob_start = entry.blob_offset as usize;
        bytes[blob_start..blob_start + tensor.blob.len()].copy_from_slice(tensor.blob);
        put32(bytes, checksums + 4 + index * 4, crc32c(tensor.blob));
    }
    if plan.input.profile == BuildProfile::GgufLayerPacked {
        write_schedule(plan, bytes);
    }
    put32(bytes, checksums, crc32c(&bytes[..checksums]));
    Ok(plan.output_len)
}

fn write_schedule(plan: &BuildPlan<'_, '_>, bytes: &mut [u8]) {
    let layers = layer_count(&plan.input);
    for layer in 0..layers as usize {
        let start = 128 + layer * 64;
        put32(bytes, start, layer as u32);
        put32(bytes, start + 4, u32::MAX);
    }
    for entry in plan.entries {
        if entry.manifold_idx >= layers {
            continue;
        }
        let start = 128 + entry.manifold_idx as usize * 64;
        put32(
            bytes,
            start + 4,
            get32(bytes, start + 4).min(entry.blob_offset),
        );
        put32(
            bytes,
            start + 8,
            get32(bytes, start + 8).max(entry.blob_offset.saturating_add(entry.blob_size)),
        );
        put16(
            bytes,
            start + 12,
            get16(bytes, start + 12).saturating_add(1),
        );
        if entry.role_id < 16 {
            put16(
                bytes,
                start + 14,
                get16(bytes, start + 14) | (1u16 << entry.role_id),
            );
        }
    }
    for layer in 0..layers as usize {
        let start = 128 + layer * 64;
        if get32(bytes, start + 4) == u32::MAX {
            put32(bytes, start + 4, 0);
        }
    }
}

#[inline]
pub(crate) fn write_entry(entry: &P64TensorEntry, bytes: &mut [u8]) {
    entry.write_le(bytes);
}

fn put16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}
fn put32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn get16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}
fn get32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
