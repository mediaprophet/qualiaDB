#[path = "extended_records.rs"]
pub(in crate::render::portal_load_emf) mod extended_records;
#[path = "extended_registry.rs"]
pub(in crate::render::portal_load_emf) mod extended_registry;
#[path = "extended_admission.rs"]
pub(in crate::render::portal_load_emf) mod extended_admission;

pub(in crate::render::portal_load_emf) use extended_admission::{
    AdmittedEmf, ExtendedChunk, ExtendedKind,
};
use extended_registry::canonical_emf_mode;
use profiles::{ProfileChunk, ProfileReader};
use super::{charge, profiles, sem, vector, Error, PortalEmfLimits, StructuralView};

fn needs_extended_admission(view: &StructuralView<'_>) -> Result<bool, Error> {
    for index in 1..=view.entry_count() {
        let e = view.entry(index)?;
        if e.mode() == 0x106 && e.stride() == 1 {
            continue;
        }
        if !matches!(e.mode(), 0 | 1 | 3) || (e.mode() == 0 && (e.flags() & 4) == 0) {
            return Ok(true);
        }
    }
    for b in view.blocks() {
        if matches!(b.schema_type(), 105..=107 | 133 | 134 | 136 | 137) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn admit_all<'a>(
    view: &StructuralView<'a>,
    limits: PortalEmfLimits,
    work: &mut u64,
) -> Result<AdmittedEmf<'a>, Error> {
    let entries = view.entry_count() as usize;
    let blocks = view.blocks().count();
    charge(work, blocks as u64)?;
    if entries > limits.profiles.shared.entries || blocks > limits.profiles.shared.blocks {
        return Err(Error::Limit);
    }
    let budget = entries
        .checked_mul(
            std::mem::size_of::<ProfileChunk<'_>>() + 1 + std::mem::size_of::<sem::NodeState>(),
        )
        .and_then(|n| {
            blocks
                .checked_mul(std::mem::size_of::<sem::NodeState>())
                .and_then(|m| n.checked_add(m))
        })
        .ok_or(Error::Overflow)?;
    if budget > limits.peak_owned_bytes {
        return Err(Error::Limit);
    }
    let mut metadata = vector(blocks, limits.peak_owned_bytes)?;
    metadata.resize(blocks, sem::NodeState::default());
    let mut nodes = vector(entries, limits.peak_owned_bytes)?;
    nodes.resize(entries, sem::NodeState::default());
    let mut referenced = vector(entries, limits.peak_owned_bytes)?;
    referenced.resize(entries, 0u8);
    let mut mask_bytes = 0u64;
    for index in 1..=view.entry_count() {
        charge(work, 1)?;
        let e = view.entry(index)?;
        if e.mode() == 0x106 && e.stride() == 1 {
            mask_bytes = mask_bytes
                .checked_add(e.stored_bytes())
                .ok_or(Error::Overflow)?;
        } else if canonical_emf_mode(e.mode()).is_none() {
            return Err(Error::Profile(profiles::Error::UnsupportedProfile));
        }
    }
    let payload_budget = limits
        .total_payload_work
        .checked_sub(mask_bytes)
        .ok_or(Error::Limit)?;

    if needs_extended_admission(view)? {
        return Ok(AdmittedEmf::Extended(
            extended_admission::admit_extended(view, limits, work)?,
        ));
    }

    let mut chunks = vector(entries, limits.peak_owned_bytes)?;
    let mut reader = match ProfileReader::open(
        view,
        &mut sem::Scratch {
            metadata: &mut metadata,
            entries: &mut nodes,
        },
        limits.profiles,
        payload_budget,
    ) {
        Ok(r) => r,
        Err(profiles::Error::UnsupportedProfile | profiles::Error::Flags) => {
            return Ok(AdmittedEmf::Extended(
                extended_admission::admit_extended(view, limits, work)?,
            ));
        }
        Err(e) => return Err(Error::Profile(e)),
    };

    for index in 1..=view.entry_count() {
        charge(work, 1)?;
        let e = view.entry(index)?;
        if e.mode() == 0x106 {
            continue;
        }
        let chunk = reader.chunk(index)?;
        let mut descriptor = None;
        for b in view.blocks() {
            charge(work, 1)?;
            if b.id() == e.descriptor_id() {
                descriptor = Some(b);
                break;
            }
        }
        let descriptor = descriptor.ok_or(Error::Profile(profiles::Error::Descriptor))?;
        charge(work, descriptor.fields().count() as u64)?;
        if let Some(mask) = sem::field(descriptor, 16) {
            let id = u32::from_le_bytes(
                mask.value()
                    .try_into()
                    .map_err(|_| Error::OrphanValidityMask)?,
            );
            let slot = usize::try_from(id.checked_sub(1).ok_or(Error::OrphanValidityMask)?)
                .map_err(|_| Error::Overflow)?;
            *referenced.get_mut(slot).ok_or(Error::OrphanValidityMask)? = 1;
        }
        chunks.push(chunk);
    }
    for index in 1..=view.entry_count() {
        let e = view.entry(index)?;
        if e.mode() == 0x106 {
            if referenced[index as usize - 1] == 0 {
                return Err(Error::OrphanValidityMask);
            }
            charge(work, e.stored_bytes())?;
            let payload = view.verify_chunk(index)?;
            if payload.iter().any(|b| *b > 1) {
                return Err(Error::Profile(profiles::Error::Validity));
            }
        }
    }
    Ok(AdmittedEmf::Profile(chunks))
}
