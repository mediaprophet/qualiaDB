use super::mueller::MuellerUnits;
use super::registry::canonical_emf_mode;
use super::*;
pub(super) struct Binding<'a> {
    pub entry: Entry<'a>,
    pub canonical_mode: u16,
    pub axis: Option<AxisView<'a>>,
    pub axis_id: u32,
    pub bins: u32,
    pub samples_per_packet: u32,
    pub step: i64,
    pub mask: Option<u32>,
    pub clock_id: u32,
    pub ext_id: u32,
    pub constitutive_id: Option<u32>,
    pub material_id: Option<u32>,
    pub calibration_id: Option<u32>,
    pub evidence_id: Option<u32>,
    pub acquisition_id: Option<u32>,
    pub bio_context_id: Option<u32>,
    pub profile: &'a str,
    pub quantity: &'a str,
    pub units: &'a str,
    pub model_uri: &'a str,
    pub brick: Option<BrickLayout>,
}
impl Binding<'_> {
    #[inline]
    pub fn samples_per_packet(&self) -> u32 {
        self.samples_per_packet
    }

    pub fn tick(&self, index: u64) -> Result<i64, Error> {
        let _ = self.samples_per_packet();
        if self.entry.flags() & 2 == 0 {
            return Ok(0);
        }
        let tick = i128::from(self.entry.start_tick())
            .checked_add(
                i128::from(index / self.brick.map_or(1, |b| b.cell_count()))
                    .checked_mul(i128::from(self.step))
                    .ok_or(Error::Overflow)?,
            )
            .ok_or(Error::Overflow)?;
        i64::try_from(tick).map_err(|_| Error::Time)
    }
}
fn reconciled_opt_ref(a: Option<Field<'_>>, b: Option<Field<'_>>) -> Result<Option<u32>, Error> {
    match (a, b) {
        (Some(fa), Some(fb)) => {
            let ida = u32_at(fa.value(), 0);
            let idb = u32_at(fb.value(), 0);
            if ida != idb {
                Err(Error::Descriptor)
            } else {
                Ok(Some(ida))
            }
        }
        (Some(f), None) | (None, Some(f)) => Ok(Some(u32_at(f.value(), 0))),
        (None, None) => Ok(None),
    }
}
pub(super) fn bind<'a>(ctx: &Context<'_, 'a>, index: u32) -> Result<Binding<'a>, Error> {
    let entry = ctx.entry(index).map_err(Error::Shared)?;
    let canonical_mode = canonical_emf_mode(entry.mode()).ok_or(Error::UnsupportedProfile)?;
    if entry.revision() != 1 {
        return Err(Error::UnsupportedProfile);
    }
    let desc = ctx
        .require_schema(entry.descriptor_id(), 3)
        .map_err(Error::Shared)?;
    ctx.charge_work(
        (desc.raw_bytes().len() as u64)
            .checked_mul(16)
            .ok_or(Error::Overflow)?,
    )
    .map_err(Error::Shared)?;
    let ext = extension_for(ctx, desc)?;
    ctx.charge_work(
        (desc.raw_bytes().len() as u64)
            .checked_add(ext.raw_bytes().len() as u64)
            .and_then(|n| n.checked_mul(32))
            .ok_or(Error::Overflow)?,
    )
    .map_err(Error::Shared)?;
    if canonical_mode == 0x0106 && text(required(ext, 6)?)? != dose_biofield::DOSE_PROFILE {
        return Err(Error::UnsupportedProfile);
    }
    if scalar32(ext, 1)? != u32::from(entry.mode())
        || scalar32(ext, 2)? != u32::from(entry.revision())
        || scalar32(ext, 4)? != entry.frame_id()
        || scalar32(ext, 5)? != scalar32(desc, 3)?
    {
        return Err(Error::Descriptor);
    }
    let frame = ctx
        .require_schema(entry.frame_id(), 2)
        .map_err(Error::Shared)?;
    if scalar32(frame, 1)? != 0 {
        return Err(Error::UnsupportedProfile);
    }
    if [9, 10, 11].iter().any(|&k| field(desc, k).is_some()) {
        return Err(Error::Descriptor);
    }
    let brick = if entry.flags() & 4 != 0 {
        if !matches!(canonical_mode, 0x0100 | 0x0102) {
            return Err(Error::UnsupportedProfile);
        }
        Some(spatial::from_metadata(frame, ext)?.brick(entry.spatial_key())?)
    } else {
        if canonical_mode != 0x0102 && [5, 6, 7].iter().any(|&k| field(frame, k).is_some()) {
            return Err(Error::UnsupportedProfile);
        }
        None
    };
    let per_time = brick.map_or(1, |b| b.cell_count());
    let clock = ctx
        .require_schema(scalar32(ext, 3)?, 1)
        .map_err(Error::Shared)?;
    let step = scalar64(ext, 11)? as i64;
    if entry.flags() & 2 != 0 {
        if entry
            .time_count()
            .checked_mul(per_time)
            .ok_or(Error::Overflow)?
            != entry.record_count()
            || step <= 0
            || scalar32(desc, 1)? != clock.id()
        {
            return Err(Error::Time);
        }
        let n = scalar64(clock, 4)?;
        let d = scalar64(clock, 5)?;
        let g = gcd(n, step as u64);
        let expected_n = n / g;
        let expected_d = u128::from(d) * u128::from(step as u64 / g);
        if scalar64(desc, 13)? != expected_n || u128::from(scalar64(desc, 14)?) != expected_d {
            return Err(Error::Time);
        }
    } else if step != 0 || scalar32(clock, 1)? != 0 {
        return Err(Error::Time);
    }
    if entry.flags() & 2 == 0 && brick.is_some() && entry.record_count() != per_time {
        return Err(Error::Shape);
    }
    let (axis, axis_id, bins, samples_per_packet, quantity, unit, model) = match canonical_mode {
        0x0100 => (
            None,
            0,
            0,
            0,
            "electromagnetic_field_snapshot",
            "mixed:mode-registry",
            FIELD_MODEL_URI,
        ),
        0x0101 => {
            let p = ctx
                .require_schema(scalar32(ext, 16)?, 135)
                .map_err(Error::Shared)?;
            let u = text(required(p, 3)?)?;
            if !["W m^-2", "W sr^-1 m^-2"].contains(&u) {
                return Err(Error::Descriptor);
            }
            (None, 0, 0, 0, "polarimetric_ray", u, RAY_MODEL_URI)
        }
        0x0102 => (
            None,
            0,
            0,
            0,
            "yee_cartesian_cell",
            "mixed:mode-registry",
            yee::YEE_MODEL_URI,
        ),
        0x0103 => {
            let n = scalar32(ext, 19)?;
            let id = scalar32(ext, 12)?;
            let block = ctx.require_schema(id, 132).map_err(Error::Shared)?;
            let available = ctx.remaining_work();
            let response = schemas::ProfileResponseValidator { ctx };
            let axis = spectrum::admit_axis(
                block,
                u64::from(n),
                spectrum::Limits {
                    samples: 4096,
                    work: available,
                },
                &response,
            )
            .map_err(Error::Spectrum)?;
            ctx.charge_work(available - axis.remaining_work())
                .map_err(Error::Shared)?;
            let q = axis.quantity();
            (
                Some(axis),
                id,
                n,
                0,
                q.name(),
                q.unit(axis.kind()),
                SPECTRUM_MODEL_URI,
            )
        }
        0x0104 => {
            let p = ctx
                .require_schema(scalar32(ext, 16)?, 135)
                .map_err(Error::Shared)?;
            let u = text(required(p, 3)?)?;
            let _ = MuellerUnits::parse(u)?;
            (
                None,
                0,
                0,
                0,
                "mueller_scattering_matrix",
                u,
                mueller::MUELLER_MODEL_URI,
            )
        }
        0x0105 => {
            let n_samples = scalar32(ext, 18)?;
            let acq = ctx
                .require_schema(scalar32(ext, 15)?, 136)
                .map_err(Error::Shared)?;
            let u = text(required(acq, 5)?)?;
            (
                None,
                0,
                0,
                n_samples,
                "rf_iq_samples",
                u,
                iq::IQ_MODEL_URI,
            )
        }
        0x0106 => (
            None,
            0,
            0,
            0,
            "ionizing_radiation_dose",
            "mixed:mode-registry",
            dose_biofield::DOSE_MODEL_URI,
        ),
        0x0107 => (
            None,
            0,
            0,
            0,
            "bioelectric_current_dipole",
            "A m",
            dose_biofield::BIOFIELD_MODEL_URI,
        ),
        _ => return Err(Error::UnsupportedProfile),
    };
    if text(required(desc, 7)?)? != quantity
        || text(required(desc, 8)?)? != unit
        || text(required(desc, 12)?)? != model
    {
        return Err(Error::Descriptor);
    }
    let stride = match canonical_mode {
        0x0103 => 64 + ((u64::from(bins) * 8 + 63) & !63),
        0x0104 => 256,
        0x0105 => u64::try_from(iq::iq_record_stride(samples_per_packet)?).map_err(|_| Error::Shape)?,
        _ => 128,
    };
    if u64::from(entry.stride()) != stride {
        return Err(Error::Shape);
    }
    let mask = field(desc, 16).map(|f| u32_at(f.value(), 0));
    if let Some(index) = mask {
        validate_mask(ctx, index)?;
    }
    let material_id = reconciled_opt_ref(field(desc, 4), field(ext, 13))?;
    let calibration_id = reconciled_opt_ref(field(desc, 6), field(ext, 30))?;
    let binding = Binding {
        entry,
        canonical_mode,
        axis,
        axis_id,
        bins,
        samples_per_packet,
        step,
        mask,
        brick,
        clock_id: clock.id(),
        ext_id: ext.id(),
        constitutive_id: field(ext, 14).map(|f| u32_at(f.value(), 0)),
        material_id,
        calibration_id,
        evidence_id: field(ext, 17).map(|f| u32_at(f.value(), 0)),
        acquisition_id: field(ext, 15).map(|f| u32_at(f.value(), 0)),
        bio_context_id: field(ext, 22).map(|f| u32_at(f.value(), 0)),
        profile: text(required(ext, 6)?)?,
        quantity: text(required(desc, 7)?)?,
        units: text(required(desc, 8)?)?,
        model_uri: text(required(desc, 12)?)?,
    };
    binding.tick(entry.record_count() - 1)?;
    Ok(binding)
}
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

fn validate_mask(ctx: &Context<'_, '_>, index: u32) -> Result<(), Error> {
    let entry = ctx.entry(index).map_err(Error::Shared)?;
    let desc = ctx
        .require_schema(entry.descriptor_id(), 3)
        .map_err(Error::Shared)?;
    if entry.mode() != 0x106
        || entry.revision() != 1
        || entry.flags() & 6 != 0
        || entry.frame_id() != 0
        || entry.stride() != 1
    {
        return Err(Error::UnsupportedProfile);
    }
    if scalar32(desc, 9)? != 1
        || scalar32(desc, 11)? != 7
        || text(required(desc, 10)?)? != "byte"
        || text(required(desc, 7)?)? != "validity"
        || text(required(desc, 8)?)? != "1"
        || text(required(desc, 12)?)? != VALIDITY_MODEL_URI
    {
        return Err(Error::Descriptor);
    }
    let mut ext = None;
    let refs = required(desc, 18)?;
    if refs.flags() & 1 == 0 {
        return Err(Error::Descriptor);
    }
    for id in refs.value().chunks_exact(4) {
        let block = ctx.block(u32_at(id, 0)).map_err(Error::Shared)?;
        if block.schema_type() == 103 {
            if ext.is_some() {
                return Err(Error::Descriptor);
            }
            ext = Some(block);
        }
    }
    let ext = ext.ok_or(Error::Descriptor)?;
    if scalar32(ext, 1)? != 0x106
        || scalar32(ext, 2)? != 1
        || scalar32(ext, 5)? != scalar32(desc, 3)?
    {
        return Err(Error::Descriptor);
    }
    Ok(())
}

pub(super) fn extension_for<'a>(
    ctx: &Context<'_, 'a>,
    desc: Block<'a>,
) -> Result<Block<'a>, Error> {
    let refs = required(desc, 18)?;
    if refs.flags() & 1 == 0 {
        return Err(Error::Descriptor);
    }
    let mut ext = None;
    for id in refs.value().chunks_exact(4) {
        let block = ctx.block(u32_at(id, 0)).map_err(Error::Shared)?;
        if block.schema_type() == 103 {
            if ext.is_some() {
                return Err(Error::Descriptor);
            }
            ext = Some(block);
        }
    }
    ext.ok_or(Error::Descriptor)
}
