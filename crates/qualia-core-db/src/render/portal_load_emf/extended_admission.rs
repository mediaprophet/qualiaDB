//! Extended EMF v4 admission (`0x0100..=0x0107` / `0..=7`) using `ExtendedEmfRegistry`.

use super::extended_records::*;
use super::extended_registry::*;
use crate::q42::{
    emf_profiles as profiles,
    field_envelope::StructuralView,
    field_semantics::{self as sem, field, Scratch},
    field_spectrum::{self as spectrum, AxisView, NoResponse},
};
use crate::render::portal_load_emf::{charge, vector, Error, PortalEmfLimits};
use profiles::BrickLayout;

pub(in crate::render::portal_load_emf) enum AdmittedEmf<'a> {
    Profile(Vec<profiles::ProfileChunk<'a>>),
    Extended(Vec<ExtendedChunk<'a>>),
}

pub(in crate::render::portal_load_emf) enum ExtendedKind<'a> {
    Field0(Option<BrickLayout>),
    Ray1,
    Yee2(Option<BrickLayout>),
    Spectrum3(AxisView<'a>, u32, u32),
    Mueller4,
    Iq5 {
        calib_scale: f64,
        calib_offset: f64,
        fallback_carrier_hz: f64,
    },
    Ionizing6 {
        calib_scale: f64,
    },
    Bio7 {
        calib_scale: f64,
    },
    Auxiliary,
}

pub(in crate::render::portal_load_emf) struct ExtendedChunk<'a> {
    pub entry_index: u32,
    pub canonical_mode: u16,
    pub descriptor_id: u32,
    pub frame_id: u32,
    pub clock_id: u32,
    pub stream_id: u32,
    pub start_tick: Option<i64>,
    pub temporal_step_ticks: Option<i64>,
    pub time_count: u64,
    pub payload: &'a [u8],
    pub stride: usize,
    pub record_count: u64,
    pub validity_mask: Option<&'a [u8]>,
    pub kind: ExtendedKind<'a>,
}

impl<'a> ExtendedChunk<'a> {
    #[inline]
    pub fn entry_index(&self) -> u32 {
        self.entry_index
    }

    pub fn is_valid(&self, index: u64) -> bool {
        let _ = self.entry_index();
        self.validity_mask
            .is_none_or(|m| m.get(index as usize).copied() == Some(1))
    }

    pub fn record_slice(&self, index: u64) -> Result<&'a [u8], Error> {
        let off = (index as usize)
            .checked_mul(self.stride)
            .ok_or(Error::Overflow)?;
        let end = off.checked_add(self.stride).ok_or(Error::Overflow)?;
        self.payload
            .get(off..end)
            .ok_or(Error::Profile(profiles::Error::Shape))
    }
}

fn calib_params(view: &StructuralView<'_>, ext: crate::q42::field_envelope::Block<'_>) -> (f64, f64) {
    let Some(f) = field(ext, 30) else {
        return (1.0, 0.0);
    };
    let cid = u32_at(f.value(), 0);
    if let Some(cb) = view.blocks().find(|b| b.id() == cid && b.schema_type() == 107) {
        let scale = field(cb, 4).map_or(1.0, |v| f64_at(v.value(), 0));
        let off = field(cb, 5).map_or(0.0, |v| f64_at(v.value(), 0));
        (scale, off)
    } else {
        (1.0, 0.0)
    }
}

fn iq_fallback_carrier(view: &StructuralView<'_>, ext: crate::q42::field_envelope::Block<'_>) -> f64 {
    let Some(f) = field(ext, 15) else {
        return 0.0;
    };
    let aid = u32_at(f.value(), 0);
    view.blocks()
        .find(|b| b.id() == aid && b.schema_type() == 136)
        .and_then(|ab| field(ab, 12))
        .map_or(0.0, |cf| f64_at(cf.value(), 0))
}

pub(super) fn admit_extended<'a>(
    view: &StructuralView<'a>,
    limits: PortalEmfLimits,
    work: &mut u64,
) -> Result<Vec<ExtendedChunk<'a>>, Error> {
    let entries = view.entry_count() as usize;
    let blocks = view.blocks().count();
    let mut metadata = vector(blocks, limits.peak_owned_bytes)?;
    metadata.resize(blocks, sem::NodeState::default());
    let mut nodes = vector(entries, limits.peak_owned_bytes)?;
    nodes.resize(entries, sem::NodeState::default());
    let mut referenced = vector(entries, limits.peak_owned_bytes)?;
    referenced.resize(entries, 0u8);
    let mut out = vector(entries, limits.peak_owned_bytes)?;

    sem::admit_shared(
        view,
        &ExtendedEmfRegistry,
        &mut Scratch { metadata: &mut metadata, entries: &mut nodes },
        limits.profiles.shared,
    )
    .map_err(|e| Error::Profile(profiles::Error::Shared(e)))?;

    let mut payload_budget = limits.total_payload_work;
    for index in 1..=view.entry_count() {
        let e = view.entry(index)?;
        let desc = view
            .blocks()
            .find(|b| b.id() == e.descriptor_id())
            .ok_or(Error::Profile(profiles::Error::Descriptor))?;
        if let Some(mask) = field(desc, 16) {
            let mid = u32::from_le_bytes(
                mask.value().try_into().map_err(|_| Error::OrphanValidityMask)?,
            );
            let slot = usize::try_from(mid.checked_sub(1).ok_or(Error::OrphanValidityMask)?)
                .map_err(|_| Error::Overflow)?;
            *referenced.get_mut(slot).ok_or(Error::OrphanValidityMask)? = 1;
        }
        payload_budget = payload_budget
            .checked_sub(e.stored_bytes())
            .ok_or(Error::Limit)?;
        charge(work, e.stored_bytes().saturating_add(1))?;
    }

    let mut total_records = 0u64;
    for index in 1..=view.entry_count() {
        let e = view.entry(index)?;
        let mode = canonical_emf_mode(e.mode())
            .ok_or(Error::Profile(profiles::Error::UnsupportedProfile))?;
        if mode == 0x0106 && e.stride() == 1 {
            if referenced[index as usize - 1] == 0 {
                return Err(Error::OrphanValidityMask);
            }
            let mask_bytes = view.verify_chunk(index)?;
            if mask_bytes.iter().any(|&b| b > 1) {
                return Err(Error::Profile(profiles::Error::Validity));
            }
            continue;
        }
        let payload = view.verify_chunk(index)?;
        let stride = e.stride() as usize;
        let rc = e.record_count();
        total_records = total_records.checked_add(rc).ok_or(Error::Overflow)?;
        if total_records > limits.profiles.records {
            return Err(Error::Limit);
        }
        if stride == 0 || (rc as usize).checked_mul(stride) != Some(payload.len()) {
            return Err(Error::Profile(profiles::Error::Shape));
        }
        let desc = view
            .blocks()
            .find(|b| b.id() == e.descriptor_id())
            .ok_or(Error::Profile(profiles::Error::Descriptor))?;
        let validity_mask = if let Some(mask) = field(desc, 16) {
            let mid = u32_at(mask.value(), 0);
            Some(view.verify_chunk(mid)?)
        } else {
            None
        };
        let Some(ext) = find_ext_block(view, desc)? else {
            if mode == 0x0104 || mode == 0x0107 {
                out.push(ExtendedChunk {
                    entry_index: index,
                    canonical_mode: mode,
                    descriptor_id: e.descriptor_id(),
                    frame_id: e.frame_id(),
                    clock_id: 0,
                    stream_id: e.stream_id(),
                    start_tick: None,
                    temporal_step_ticks: None,
                    time_count: e.time_count(),
                    payload,
                    stride,
                    record_count: rc,
                    validity_mask,
                    kind: ExtendedKind::Auxiliary,
                });
                continue;
            }
            return Err(Error::Profile(profiles::Error::Descriptor));
        };
        let ext_mode = field(ext, 1).ok_or(Error::Profile(profiles::Error::Descriptor))?;
        if canonical_emf_mode(u32_at(ext_mode.value(), 0) as u16) != Some(mode) {
            return Err(Error::Profile(profiles::Error::Descriptor));
        }
        let clock_id = u32_at(
            field(ext, 3).ok_or(Error::Profile(profiles::Error::Descriptor))?.value(),
            0,
        );
        let step_raw = i64_at(
            field(ext, 11).ok_or(Error::Profile(profiles::Error::Descriptor))?.value(),
            0,
        );
        let (start_tick, temporal_step_ticks) = if e.flags() & 2 != 0 {
            if step_raw <= 0 {
                return Err(Error::Profile(profiles::Error::Time));
            }
            (Some(e.start_tick()), Some(step_raw))
        } else {
            (None, None)
        };
        let frame_id_ext = field(ext, 4).map_or(0, |f| u32_at(f.value(), 0));
        if frame_id_ext != 0 && frame_id_ext != e.frame_id() {
            return Err(Error::Profile(profiles::Error::Frame));
        }
        let frame_block = view.blocks().find(|b| b.id() == frame_id_ext);
        let grid = match (frame_block, mode) {
            (Some(fb), 0x0100 | 0x0102) if e.flags() & 4 != 0 => {
                let layout = grid_from_blocks(fb, ext)?
                    .ok_or(Error::Profile(profiles::Error::Frame))?;
                Some(layout.brick(e.spatial_key())?)
            }
            _ => None,
        };
        let kind = match mode {
            0x0100 => ExtendedKind::Field0(grid),
            0x0101 => ExtendedKind::Ray1,
            0x0102 => ExtendedKind::Yee2(grid),
            0x0103 => {
                let axis_id = u32_at(
                    field(ext, 12).ok_or(Error::Profile(profiles::Error::Descriptor))?.value(),
                    0,
                );
                let bins = u32_at(
                    field(ext, 19).ok_or(Error::Profile(profiles::Error::Descriptor))?.value(),
                    0,
                );
                let axis_block = view
                    .blocks()
                    .find(|b| b.id() == axis_id)
                    .ok_or(Error::Profile(profiles::Error::Descriptor))?;
                let axis = spectrum::admit_axis(
                    axis_block,
                    u64::from(bins),
                    spectrum::Limits { samples: 4096, work: *work },
                    &NoResponse,
                )
                .map_err(|se| Error::Profile(profiles::Error::Spectrum(se)))?;
                ExtendedKind::Spectrum3(axis, bins, axis_id)
            }
            0x0104 => ExtendedKind::Mueller4,
            0x0105 => {
                let (calib_scale, calib_offset) = calib_params(view, ext);
                let fallback_carrier_hz = iq_fallback_carrier(view, ext);
                ExtendedKind::Iq5 { calib_scale, calib_offset, fallback_carrier_hz }
            }
            0x0106 => {
                let (calib_scale, _) = calib_params(view, ext);
                ExtendedKind::Ionizing6 { calib_scale }
            }
            0x0107 => {
                let (calib_scale, _) = calib_params(view, ext);
                ExtendedKind::Bio7 { calib_scale }
            }
            _ => return Err(Error::Profile(profiles::Error::UnsupportedProfile)),
        };

        for ri in 0..rc {
            charge(work, 1)?;
            let valid = validity_mask.is_none_or(|m| m[ri as usize] == 1);
            let off = (ri as usize) * stride;
            let slice = &payload[off..off + stride];
            match &kind {
                ExtendedKind::Field0(_) => { let _ = parse_mode0_field(slice, valid)?; }
                ExtendedKind::Ray1 => { let _ = parse_mode1_ray(slice, valid)?; }
                ExtendedKind::Yee2(_) => { let _ = parse_mode2_yee(slice, valid)?; }
                ExtendedKind::Spectrum3(_, bins, axis_id) => {
                    let _ = parse_mode3_spectrum(slice, valid, *bins, *axis_id)?;
                }
                ExtendedKind::Mueller4 => { let _ = parse_mode4_mueller(slice, valid)?; }
                ExtendedKind::Iq5 { .. } => { let _ = parse_mode5_iq(slice, valid)?; }
                ExtendedKind::Ionizing6 { .. } => { let _ = parse_mode6_ionizing(slice, valid)?; }
                ExtendedKind::Bio7 { .. } => { let _ = parse_mode7_bio(slice, valid)?; }
                ExtendedKind::Auxiliary => {}
            }
        }
        out.push(ExtendedChunk {
            entry_index: index,
            canonical_mode: mode,
            descriptor_id: e.descriptor_id(),
            frame_id: e.frame_id(),
            clock_id,
            stream_id: e.stream_id(),
            start_tick,
            temporal_step_ticks,
            time_count: e.time_count(),
            payload,
            stride,
            record_count: rc,
            validity_mask,
            kind,
        });
    }
    Ok(out)
}
