use super::*;
pub(super) fn validate(ctx: &Context<'_, '_>, registry: &impl DomainRegistry) -> Result<(), Error> {
    for i in 1..=ctx.view.entry_count() {
        let entry = ctx.entry(i)?;
        let desc = ctx.require_schema(entry.descriptor_id(), 3)?;
        let frame = field(desc, 2).map_or(0, |f| u32_at(f.value(), 0));
        if frame != entry.frame_id() {
            return Err(Error::Frame);
        }
        if entry.flags() & 2 != 0 {
            for key in [1, 13, 14] {
                if required(desc, key)?.flags() & 1 == 0 {
                    return Err(Error::RequiredCritical);
                }
            }
            ctx.require_schema(scalar32(desc, 1)?, 1)?;
            shared::rate(scalar64(desc, 13)?, scalar64(desc, 14)?)?;
        } else if [1, 13, 14].iter().any(|&key| field(desc, key).is_some()) {
            return Err(Error::Time);
        }
        if entry.flags() & 4 != 0 {
            let grid = ctx.require_schema(frame, 2)?;
            grid_key(grid, entry.spatial_key())?;
            registry.validate_spatial_entry(entry, grid, ctx)?;
        }
        provenance(ctx, entry, desc)?;
        validity(ctx, entry, desc)?;
    }
    Ok(())
}
fn grid_key(frame: Block<'_>, key: u64) -> Result<(), Error> {
    let spacing = required(frame, 5)?;
    let dimensions = required(frame, 6)?;
    let order = field(frame, 7);
    if [Some(spacing), Some(dimensions), order]
        .iter()
        .flatten()
        .any(|f| f.flags() & 1 == 0)
    {
        return Err(Error::RequiredCritical);
    }
    let dims = dimensions.value();
    let Some(order) = order else {
        return Ok(());
    };
    match u32_at(order.value(), 0) {
        0 => {
            let count = (0..3).try_fold(1u64, |n, i| {
                n.checked_mul(u64::from(u32_at(dims, i * 4)))
                    .ok_or(Error::Overflow)
            })?;
            if key >= count {
                return Err(Error::Frame);
            }
        }
        1 => {
            if key >> 63 != 0 {
                return Err(Error::Frame);
            }
            for axis in 0..3 {
                let mut coordinate = 0u32;
                for bit in 0..21 {
                    coordinate |= (((key >> (bit * 3 + axis)) & 1) as u32) << bit;
                }
                if coordinate >= u32_at(dims, axis * 4) {
                    return Err(Error::Frame);
                }
            }
        }
        _ => return Err(Error::Frame),
    }
    Ok(())
}
fn provenance(ctx: &Context<'_, '_>, _entry: Entry<'_>, desc: Block<'_>) -> Result<(), Error> {
    let prov = ctx.require_schema(scalar32(desc, 3)?, 4)?;
    let sources = field(desc, 17).map_or(&[][..], |f| f.value());
    if sources.is_empty() {
        if field(prov, 2).is_some() {
            return Err(Error::Provenance);
        }
    } else {
        let digest_field = required(prov, 2)?;
        if digest_field.flags() & 1 == 0 {
            return Err(Error::RequiredCritical);
        }
        let digests = digest_field.value();
        let expected = (sources.len() / 4).checked_mul(32).ok_or(Error::Overflow)?;
        if digests.len() != expected {
            return Err(Error::Provenance);
        }
        for (source, digest) in sources.chunks_exact(4).zip(digests.chunks_exact(32)) {
            ctx.charge_work(32)?;
            if ctx.entry(u32_at(source, 0))?.digest() != digest {
                return Err(Error::Provenance);
            }
        }
    }
    Ok(())
}
fn validity(ctx: &Context<'_, '_>, entry: Entry<'_>, desc: Block<'_>) -> Result<(), Error> {
    if let Some(mask) = field(desc, 16) {
        if entry.mode() == 0x106 && entry.stride() == 1 {
            return Err(Error::Validity);
        }
        let count = required(desc, 15)?;
        if count.flags() & 1 == 0 {
            return Err(Error::RequiredCritical);
        }
        let mask_index = u32_at(mask.value(), 0);
        let mask_entry = ctx.entry(mask_index)?;
        if mask_entry.mode() != 0x106
            || mask_entry.record_count() != entry.record_count()
            || mask_entry.decoded_bytes() != entry.record_count()
            || mask_entry.stride() != 1
        {
            return Err(Error::Validity);
        }
        let mask_desc = ctx.require_schema(mask_entry.descriptor_id(), 3)?;
        if field(mask_desc, 16).is_some() {
            return Err(Error::Validity);
        }
        ctx.charge_work(
            mask_entry
                .stored_bytes()
                .checked_mul(2)
                .ok_or(Error::Overflow)?,
        )?;
        let bytes = ctx
            .view
            .verify_chunk(mask_index)
            .map_err(|_| Error::PayloadIntegrity)?;
        let mut valid = 0u64;
        for &b in bytes {
            if b > 1 {
                return Err(Error::Validity);
            }
            valid += u64::from(b);
        }
        if u64_at(count.value(), 0) != valid {
            return Err(Error::Validity);
        }
    } else if field(desc, 15).is_some_and(|f| u64_at(f.value(), 0) != entry.record_count()) {
        return Err(Error::Validity);
    }
    Ok(())
}
