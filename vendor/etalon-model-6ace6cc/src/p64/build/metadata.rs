use super::{LayoutPlan, LayoutProfile};

pub(super) fn emit_metadata<E>(
    plan: &LayoutPlan<'_, '_>,
    mut emit: impl FnMut(&[u8]) -> Result<(), E>,
) -> Result<(), E> {
    let mut record = [0u8; 64];
    plan.header.write_le(&mut record);
    emit(&record)?;
    emit(plan.input.hparams_bytes)?;
    let mut position = 128;
    if plan.input.profile == LayoutProfile::FfnLegacy {
        emit(plan.input.schedule_bytes)?;
        position += plan.input.schedule_bytes.len();
    } else if plan.input.has_schedule() {
        for layer in 0..plan.input.layers() {
            record.fill(0);
            record[..4].copy_from_slice(&layer.to_le_bytes());
            let mut begin = u32::MAX;
            let mut end = 0;
            let mut count = 0u16;
            let mut roles = 0u16;
            for entry in plan.entries {
                if entry.manifold_idx != layer {
                    continue;
                }
                begin = begin.min(entry.blob_offset);
                end = end.max(entry.blob_offset.saturating_add(entry.blob_size));
                count = count.saturating_add(1);
                if entry.role_id < 16 {
                    roles |= 1 << entry.role_id;
                }
            }
            if begin == u32::MAX {
                begin = 0;
            }
            record[4..8].copy_from_slice(&begin.to_le_bytes());
            record[8..12].copy_from_slice(&end.to_le_bytes());
            record[12..14].copy_from_slice(&count.to_le_bytes());
            record[14..16].copy_from_slice(&roles.to_le_bytes());
            emit(&record)?;
            position += 64;
        }
    }
    for entry in plan.entries {
        record.fill(0);
        super::write::write_entry(entry, &mut record);
        emit(&record)?;
        position += 64;
    }
    for (index, entry) in plan.entries.iter().enumerate() {
        let name = plan.input.tensor(index).name;
        let name_start = plan.header.string_table_offset as usize + entry.name_offset as usize;
        zeros(name_start - position, &mut emit)?;
        emit(name)?;
        emit(&[0])?;
        position = name_start + name.len() + 1;
    }
    zeros(
        plan.header.manifold_table_offset as usize - position,
        &mut emit,
    )?;
    emit(plan.input.manifold_bytes)?;
    emit(plan.input.tokenizer_bytes)?;
    position = plan.header.tokenizer_offset as usize + plan.input.tokenizer_bytes.len();
    zeros(plan.header.checksum_offset as usize - position, &mut emit)
}

pub(super) fn zeros<E>(
    mut count: usize,
    emit: &mut impl FnMut(&[u8]) -> Result<(), E>,
) -> Result<(), E> {
    let bytes = [0; 1024];
    while count != 0 {
        let chunk = count.min(bytes.len());
        emit(&bytes[..chunk])?;
        count -= chunk;
    }
    Ok(())
}
