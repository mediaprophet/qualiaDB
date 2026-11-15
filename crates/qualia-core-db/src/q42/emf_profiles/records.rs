use super::*;
#[derive(Clone, Copy, Debug)]
pub struct FieldRecord<'a> {
    pub(super) bytes: &'a [u8],
}
impl FieldRecord<'_> {
    pub fn electric_v_per_m(self) -> [f64; 3] {
        triple(self.bytes, 0)
    }
    pub fn magnetic_tesla(self) -> [f64; 3] {
        triple(self.bytes, 24)
    }
    /// Local metres in ProfileChunk::frame_id(); no transform is applied.
    pub fn position_m(self) -> [f64; 3] {
        triple(self.bytes, 48)
    }
    pub fn tick(self) -> i64 {
        u64_at(self.bytes, 72) as i64
    }
    pub fn carrier_hz(self) -> Option<f64> {
        if u32_at(self.bytes, 120) & 1 != 0 {
            Some(f64_at(self.bytes, 80))
        } else {
            None
        }
    }
    pub fn coherence_phase_rad(self) -> Option<f64> {
        if u32_at(self.bytes, 120) & 2 != 0 {
            Some(f64_at(self.bytes, 88))
        } else {
            None
        }
    }
    /// Stored observation only; no constitutive formula is inferred.
    pub fn stored_poynting_w_per_m2(self) -> Option<[f64; 3]> {
        if u32_at(self.bytes, 120) & 4 != 0 {
            Some(triple(self.bytes, 96))
        } else {
            None
        }
    }
    pub fn confidence(self) -> Option<f32> {
        if u32_at(self.bytes, 120) & 8 != 0 {
            Some(f32::from_bits(u32_at(self.bytes, 124)))
        } else {
            None
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct RayRecord<'a> {
    pub(super) bytes: &'a [u8],
}
impl RayRecord<'_> {
    /// Local metres in ProfileChunk::frame_id(); no transform is applied.
    pub fn position_m(self) -> [f64; 3] {
        triple(self.bytes, 0)
    }
    pub fn direction(self) -> [f64; 3] {
        triple(self.bytes, 24)
    }
    /// Time-averaged I/Q/U/V in ProfileChunk::units(), schema135 convention.
    pub fn stokes(self) -> [f64; 4] {
        [
            f64_at(self.bytes, 48),
            f64_at(self.bytes, 56),
            f64_at(self.bytes, 64),
            f64_at(self.bytes, 72),
        ]
    }
    pub fn carrier_hz(self) -> f64 {
        f64_at(self.bytes, 80)
    }
    pub fn bandwidth_hz(self) -> f64 {
        f64_at(self.bytes, 88)
    }
    pub fn geometric_path_m(self) -> f64 {
        f64_at(self.bytes, 96)
    }
    pub fn divergence_rad(self) -> f64 {
        f64_at(self.bytes, 112)
    }
    pub fn phase_rad(self) -> Option<f64> {
        if u32_at(self.bytes, 124) & 1 != 0 {
            Some(f64_at(self.bytes, 104))
        } else {
            None
        }
    }
    pub fn material_entry(self) -> u32 {
        u32_at(self.bytes, 120)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct SpectrumRecord<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) bins: u32,
}
impl<'a> SpectrumRecord<'a> {
    /// Local metres in ProfileChunk::frame_id(); no transform is applied.
    pub fn position_m(self) -> [f64; 3] {
        triple(self.bytes, 0)
    }
    pub fn tick(self) -> i64 {
        u64_at(self.bytes, 24) as i64
    }
    pub fn axis_metadata_id(self) -> u32 {
        u32_at(self.bytes, 36)
    }
    pub fn material_entry(self) -> u32 {
        u32_at(self.bytes, 32)
    }
    pub fn temperature_kelvin(self) -> Option<f64> {
        if u32_at(self.bytes, 48) & 1 != 0 {
            Some(f64_at(self.bytes, 40))
        } else {
            None
        }
    }
    pub fn values(self) -> SpectrumValues<'a> {
        SpectrumValues {
            bytes: &self.bytes[64..64 + self.bins as usize * 8],
        }
    }
}
#[derive(Clone, Debug)]
pub struct SpectrumValues<'a> {
    bytes: &'a [u8],
}
impl Iterator for SpectrumValues<'_> {
    type Item = f64;
    fn next(&mut self) -> Option<f64> {
        if self.bytes.is_empty() {
            return None;
        }
        let v = f64_at(self.bytes, 0);
        self.bytes = &self.bytes[8..];
        Some(v)
    }
}
fn check_material_entry(
    context: &Context<'_, '_>,
    binding: &bindings::Binding<'_>,
    material_entry: u32,
) -> Result<(), Error> {
    if material_entry == 0 {
        return Ok(());
    }
    let Some(mat_id) = binding.material_id else {
        return Err(Error::UnsupportedProfile);
    };
    let mat_block = context.require_schema(mat_id, 105).map_err(Error::Shared)?;
    if !schemas::material_binding_contains(mat_block, u64::from(material_entry))? {
        return Err(Error::Descriptor);
    }
    Ok(())
}

pub(super) fn validate(
    bytes: &[u8],
    context: &Context<'_, '_>,
    binding: &bindings::Binding<'_>,
    ext: Block<'_>,
    yee_policy: yee::YeeParsePolicy,
    valid: bool,
    tick: i64,
) -> Result<(), Error> {
    match binding.canonical_mode {
        0x0100 => field_record(bytes, valid, tick),
        0x0101 => ray_record(bytes, context, binding, valid),
        0x0102 => yee_record(bytes, context, binding, yee_policy, valid),
        0x0103 => spectrum_record(
            bytes,
            context,
            binding,
            valid,
            tick,
            binding.axis_id,
            binding.bins,
            binding
                .axis
                .map(|axis| axis.quantity())
                .ok_or(Error::Descriptor)?,
        ),
        0x0104 => mueller_record(bytes, context, binding, valid),
        0x0105 => iq_record(bytes, context, binding, ext, valid, tick),
        0x0106 => dose_record(bytes, context, binding, valid, tick),
        0x0107 => biofield_record(bytes, context, binding, valid, tick),
        _ => Err(Error::UnsupportedProfile),
    }
}
pub(super) fn field_record(bytes: &[u8], valid: bool, tick: i64) -> Result<(), Error> {
    if u64_at(bytes, 72) as i64 != tick {
        return Err(Error::Time);
    }
    if !valid {
        zeros(&bytes[..72])?;
        zeros(&bytes[80..])?;
        return Ok(());
    }
    finite(bytes, 0, 72)?;
    finite(bytes, 80, 120)?;
    let flags = u32_at(bytes, 120);
    if flags & !15 != 0 {
        return Err(Error::Flags);
    }
    if flags & 1 != 0 {
        if f64_at(bytes, 80) <= 0.0 {
            return Err(Error::Value);
        }
    } else {
        zeros(&bytes[80..88])?;
    }
    if flags & 2 != 0 {
        if flags & 1 == 0 {
            return Err(Error::UnsupportedProfile);
        }
    } else {
        zeros(&bytes[88..96])?;
    }
    if flags & 4 == 0 {
        zeros(&bytes[96..120])?;
    }
    if flags & 8 != 0 {
        let c = f32::from_bits(u32_at(bytes, 124));
        if !c.is_finite() {
            return Err(Error::NonFinite);
        }
        if !(0.0..=1.0).contains(&c) {
            return Err(Error::Value);
        }
    } else {
        zeros(&bytes[124..128])?;
    }
    Ok(())
}
fn ray_record(
    bytes: &[u8],
    context: &Context<'_, '_>,
    binding: &bindings::Binding<'_>,
    valid: bool,
) -> Result<(), Error> {
    if !valid {
        zeros(bytes)?;
        return Ok(());
    }
    let material_entry = u32_at(bytes, 120);
    check_material_entry(context, binding, material_entry)?;
    let flags = u32_at(bytes, 124);
    if flags & !1 != 0 {
        return Err(Error::Flags);
    }
    finite(bytes, 0, 120)?;
    if flags & 1 != 0 {
        if binding.calibration_id.is_none() && binding.evidence_id.is_none() {
            return Err(Error::UnsupportedProfile);
        }
    } else {
        zeros(&bytes[104..112])?;
    }
    let d = triple(bytes, 24);
    let norm = (d[0] * d[0] + d[1] * d[1]) + d[2] * d[2];
    if (norm - 1.0).abs() > 1e-12 {
        return Err(Error::Value);
    }
    let i = f64_at(bytes, 48);
    let magnitude = f64_at(bytes, 56)
        .hypot(f64_at(bytes, 64))
        .hypot(f64_at(bytes, 72));
    if i < 0.0
        || !magnitude.is_finite()
        || (magnitude > i && (magnitude - i) / magnitude > 64.0 * f64::EPSILON)
    {
        return Err(Error::Value);
    }
    if f64_at(bytes, 80) <= 0.0
        || f64_at(bytes, 88) < 0.0
        || f64_at(bytes, 96) < 0.0
        || !(0.0..std::f64::consts::FRAC_PI_2).contains(&f64_at(bytes, 112))
    {
        return Err(Error::Value);
    }
    Ok(())
}
fn spectrum_record(
    bytes: &[u8],
    context: &Context<'_, '_>,
    binding: &bindings::Binding<'_>,
    valid: bool,
    tick: i64,
    axis_id: u32,
    bins: u32,
    quantity: spectrum::Quantity,
) -> Result<(), Error> {
    if u64_at(bytes, 24) as i64 != tick {
        return Err(Error::Time);
    }
    if u32_at(bytes, 36) != axis_id {
        return Err(Error::Descriptor);
    }
    let flags = u32_at(bytes, 48);
    if flags & !1 != 0 {
        return Err(Error::Flags);
    }
    zeros(&bytes[52..64])?;
    let end = 64 + bins as usize * 8;
    zeros(&bytes[end..])?;
    if !valid {
        zeros(&bytes[..24])?;
        zeros(&bytes[32..36])?;
        zeros(&bytes[40..52])?;
        zeros(&bytes[64..end])?;
        return Ok(());
    }
    check_material_entry(context, binding, u32_at(bytes, 32))?;
    finite(bytes, 0, 24)?;
    finite(bytes, 40, 48)?;
    finite(bytes, 64, end)?;
    if flags & 1 != 0 {
        if f64_at(bytes, 40) < 0.0 {
            return Err(Error::Value);
        }
    } else {
        zeros(&bytes[40..48])?;
    }
    for at in (64..end).step_by(8) {
        let v = f64_at(bytes, at);
        if v < 0.0 || (quantity == spectrum::Quantity::DimensionlessReflectance && v > 1.0) {
            return Err(Error::Value);
        }
    }
    Ok(())
}
fn yee_record(
    bytes: &[u8],
    context: &Context<'_, '_>,
    binding: &bindings::Binding<'_>,
    policy: yee::YeeParsePolicy,
    valid: bool,
) -> Result<(), Error> {
    let Some(rec) = yee::YeeRecord::parse(bytes, valid, policy)? else {
        return Ok(());
    };
    check_material_entry(context, binding, rec.material_entry)?;
    if let Some(cid) = binding.constitutive_id {
        let cb = context.require_schema(cid, 133).map_err(Error::Shared)?;
        if let Ok(schemas::ConstitutiveKind::Vacuum) =
            schemas::ConstitutiveKind::parse(text(required(cb, 1)?)?)
        {
            if (rec.epsilon_relative - 1.0).abs() > 1.0e-12
                || (rec.mu_relative - 1.0).abs() > 1.0e-12
                || rec.conductivity_s_per_m != 0.0
            {
                return Err(Error::Value);
            }
        }
    }
    Ok(())
}
fn mueller_record(
    bytes: &[u8],
    context: &Context<'_, '_>,
    binding: &bindings::Binding<'_>,
    valid: bool,
) -> Result<(), Error> {
    let Some(rec) = mueller::MuellerRecord::parse(bytes, valid, true)? else {
        return Ok(());
    };
    check_material_entry(context, binding, rec.material_entry)
}
fn iq_record(
    bytes: &[u8],
    context: &Context<'_, '_>,
    binding: &bindings::Binding<'_>,
    ext: Block<'_>,
    valid: bool,
    tick: i64,
) -> Result<(), Error> {
    let Some(pkt) = iq::IqPacketView::parse(bytes, valid)? else {
        return Ok(());
    };
    if binding.entry.flags() & 2 != 0 && pkt.header.first_sample_tick != tick {
        return Err(Error::Time);
    }
    let acq_id = binding.acquisition_id.ok_or(Error::Descriptor)?;
    let cal_id = binding.calibration_id.ok_or(Error::Descriptor)?;
    let acq = context.require_schema(acq_id, 136).map_err(Error::Shared)?;
    let cal = context.require_schema(cal_id, 107).map_err(Error::Shared)?;
    let raw_count_mode = text(required(acq, 5)?)? == "counts";
    pkt.validate_against_blocks(ext, acq, cal, raw_count_mode)
}
fn dose_record(
    bytes: &[u8],
    context: &Context<'_, '_>,
    binding: &bindings::Binding<'_>,
    valid: bool,
    tick: i64,
) -> Result<(), Error> {
    let Some(rec) = dose_biofield::DoseTransportRecord::parse(bytes, valid)? else {
        return Ok(());
    };
    if binding.entry.flags() & 2 != 0 && rec.tick != tick {
        return Err(Error::Time);
    }
    let cal_id = binding.calibration_id.ok_or(Error::Descriptor)?;
    let ev_id = binding.evidence_id.ok_or(Error::Descriptor)?;
    let cal = context.require_schema(cal_id, 107).map_err(Error::Shared)?;
    let ev = context.require_schema(ev_id, 134).map_err(Error::Shared)?;
    let (_species, _epistemic) = rec.validate_against_blocks(cal, ev)?;
    check_material_entry(context, binding, rec.material_entry)
}
fn biofield_record(
    bytes: &[u8],
    context: &Context<'_, '_>,
    binding: &bindings::Binding<'_>,
    valid: bool,
    tick: i64,
) -> Result<(), Error> {
    let Some(rec) = dose_biofield::BiofieldRecord::parse(bytes, valid)? else {
        return Ok(());
    };
    if binding.entry.flags() & 2 != 0 && rec.tick != tick {
        return Err(Error::Time);
    }
    let bio_id = binding.bio_context_id.ok_or(Error::Descriptor)?;
    let ev_id = binding.evidence_id.ok_or(Error::Descriptor)?;
    let cal_id = binding.calibration_id.ok_or(Error::Descriptor)?;
    let bio = context.require_schema(bio_id, 137).map_err(Error::Shared)?;
    let ev = context.require_schema(ev_id, 134).map_err(Error::Shared)?;
    let cal = context.require_schema(cal_id, 107).map_err(Error::Shared)?;
    let consent = context
        .require_schema(rec.consent_provenance_ref, 4)
        .map_err(Error::Shared)?;
    let _epistemic = rec.validate_against_blocks(bio, ev, cal, consent, true)?;
    Ok(())
}
