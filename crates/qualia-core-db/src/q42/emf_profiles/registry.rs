use super::*;
const fn r(key: u16, type_tag: u8, count: Option<u32>, required: bool) -> FieldRule {
    FieldRule {
        key,
        type_tag,
        count,
        required,
    }
}
const EXT: &[FieldRule] = &[
    r(1, 1, Some(1), true),
    r(2, 1, Some(1), true),
    r(3, 8, Some(1), true),
    r(4, 8, Some(1), false),
    r(5, 8, Some(1), true),
    r(6, 6, None, true),
    r(7, 6, None, true),
    r(8, 2, Some(3), true),
    r(9, 1, Some(3), true),
    r(10, 1, Some(1), true),
    r(11, 3, Some(1), true),
    r(12, 8, Some(1), false),
    r(13, 8, Some(1), false),
    r(14, 8, Some(1), false),
    r(15, 8, Some(1), false),
    r(16, 8, Some(1), false),
    r(17, 8, Some(1), false),
    r(18, 1, Some(1), false),
    r(19, 1, Some(1), false),
    r(20, 1, Some(3), false),
    r(21, 6, None, false),
    r(22, 8, Some(1), false),
    r(23, 2, None, false),
    r(24, 6, None, false),
    r(25, 8, None, false),
    r(26, 9, None, false),
    r(27, 3, None, false),
    r(28, 2, None, false),
    r(29, 9, None, false),
    r(30, 8, Some(1), false),
];
const POL: &[FieldRule] = &[
    r(1, 6, None, true),
    r(2, 6, None, true),
    r(3, 6, None, true),
    r(4, 6, None, true),
];
pub fn extension_rules() -> &'static [FieldRule] {
    EXT
}
pub fn polarization_rules() -> &'static [FieldRule] {
    POL
}
/// Explicit point-stream subset only. No constitutive/material/calibration,
pub const fn canonical_emf_mode(mode: u16) -> Option<u16> {
    match mode {
        0 | 0x0100 => Some(0x0100),
        1 | 0x0101 => Some(0x0101),
        2 | 0x0102 => Some(0x0102),
        3 | 0x0103 => Some(0x0103),
        4 | 0x0104 => Some(0x0104),
        5 | 0x0105 => Some(0x0105),
        6 | 0x0106 => Some(0x0106),
        7 | 0x0107 => Some(0x0107),
        _ => None,
    }
}

/// Admits all EMF v4 profiles and companion schemas (103, 105..=107, 132..=137).
#[derive(Clone, Copy, Debug, Default)]
pub struct ProfileRegistry;
impl DomainRegistry for ProfileRegistry {
    fn rules(&self, schema: u16, revision: u16) -> Option<&'static [FieldRule]> {
        if revision != 1 {
            return None;
        }
        match schema {
            103 => Some(EXT),
            105 => Some(schemas::material_binding_rules()),
            106 => Some(schemas::coordinate_transform_rules()),
            107 => Some(schemas::calibration_rules()),
            132 => Some(spectrum::schema_rules()),
            133 => Some(schemas::constitutive_model_rules()),
            134 => Some(schemas::model_evidence_rules()),
            135 => Some(POL),
            136 => Some(schemas::acquisition_rules()),
            137 => Some(schemas::bio_context_rules()),
            _ => None,
        }
    }
    fn descriptor_target_schema(&self, key: u16) -> Option<u16> {
        match key {
            4 => Some(105),
            5 => Some(106),
            6 => Some(107),
            _ => None,
        }
    }
    fn validate_block(
        &self,
        block: Block<'_>,
        ctx: &Context<'_, '_>,
    ) -> Result<(), semantics::Error> {
        let result = (|| -> Result<(), Error> {
            match block.schema_type() {
                103 => extension(block, ctx),
                105 => schemas::validate_material_binding(block, ctx),
                106 => schemas::validate_coordinate_transform(block, ctx),
                107 => schemas::validate_calibration(block, ctx),
                133 => schemas::validate_constitutive_model(block, ctx),
                134 => schemas::validate_model_evidence(block, ctx),
                135 => polarization(block),
                136 => schemas::validate_acquisition(block, ctx),
                137 => schemas::validate_bio_context(block, ctx),
                132 => {
                    let positions = field(block, 5).ok_or(Error::Descriptor)?.element_count();
                    let semantics = text(required(block, 2)?)?;
                    let samples = if semantics == "point_density" {
                        u64::from(positions)
                    } else {
                        u64::from(positions).checked_sub(1).ok_or(Error::Shape)?
                    };
                    let available = ctx.remaining_work();
                    let response = schemas::ProfileResponseValidator { ctx };
                    let axis = spectrum::admit_axis(
                        block,
                        samples,
                        spectrum::Limits {
                            samples: 4096,
                            work: available,
                        },
                        &response,
                    )
                    .map_err(Error::Spectrum)?;
                    ctx.charge_work(available - axis.remaining_work())
                        .map_err(Error::Shared)?;
                    match axis.sigma() {
                        SigmaInterpretation::LegacyDisplay400700nm
                        | SigmaInterpretation::LegacyEtalonDisplay380750nm => {
                            Err(Error::UnsupportedProfile)
                        }
                        _ => Ok(()),
                    }
                }
                _ => Err(Error::UnsupportedProfile),
            }
        })();
        result.map_err(profile_error)
    }
    fn validate_grid_frame(
        &self,
        frame: Block<'_>,
        ctx: &Context<'_, '_>,
    ) -> Result<(), semantics::Error> {
        (|| -> Result<(), Error> {
            let mut found = false;
            for block in ctx.blocks() {
                let block = block.map_err(Error::Shared)?;
                ctx.charge_work(block.fields().count() as u64)
                    .map_err(Error::Shared)?;
                if block.schema_type() != 103 || block.revision() != 1 {
                    continue;
                }
                let Some(reference) = field(block, 4) else {
                    continue;
                };
                if reference.type_tag() != 8
                    || reference.element_count() != 1
                    || u32_at(reference.value(), 0) != frame.id()
                {
                    continue;
                }
                semantics::validate_fields(block, extension_rules()).map_err(Error::Shared)?;
                let mode_u32 = scalar32(block, 1)?;
                let mode_u16 = u16::try_from(mode_u32).map_err(|_| Error::UnsupportedProfile)?;
                let Some(cmode) = canonical_emf_mode(mode_u16) else {
                    return Err(Error::UnsupportedProfile);
                };
                let prof = text(required(block, 6)?)?;
                if !matches!(cmode, 0x0100 | 0x0102)
                    || (prof != FIELD_PROFILE && prof != yee::YEE_PROFILE)
                {
                    return Err(Error::UnsupportedProfile);
                }
                let layout = spatial::from_metadata(frame, block)?;
                if !(matches!(layout.spec().indexing, GridIndexing::Hilbert { .. })
                    || (matches!(layout.spec().indexing, GridIndexing::Morton { .. })
                        && layout.spec().dimensions.iter().any(|&n| n > 1 << 21)))
                {
                    return Err(Error::Frame);
                }
                found = true;
            }
            if !found {
                return Err(Error::UnsupportedProfile);
            }
            Ok(())
        })()
        .map_err(profile_error)
    }
    fn validate_spatial_entry(
        &self,
        entry: Entry<'_>,
        frame: Block<'_>,
        ctx: &Context<'_, '_>,
    ) -> Result<(), semantics::Error> {
        (|| -> Result<(), Error> {
            if !matches!(canonical_emf_mode(entry.mode()), Some(0x0100 | 0x0102)) {
                return Err(Error::UnsupportedProfile);
            }
            let desc = ctx
                .require_schema(entry.descriptor_id(), 3)
                .map_err(Error::Shared)?;
            let ext = bindings::extension_for(ctx, desc)?;
            if scalar32(ext, 4)? != frame.id() || scalar32(ext, 1)? != u32::from(entry.mode()) {
                return Err(Error::Frame);
            }
            let brick = spatial::from_metadata(frame, ext)?.brick(entry.spatial_key())?;
            let times = if entry.flags() & 2 != 0 {
                entry.time_count()
            } else {
                1
            };
            if brick
                .cell_count()
                .checked_mul(times)
                .ok_or(Error::Overflow)?
                != entry.record_count()
            {
                return Err(Error::Shape);
            }
            Ok(())
        })()
        .map_err(profile_error)
    }
    fn validate_units(&self, units: &str, _ctx: &Context<'_, '_>) -> Result<(), semantics::Error> {
        if matches!(
            units,
            "W m^-2"
                | "W sr^-1 m^-2"
                | "W m^-2 Hz^-1"
                | "W m^-2 m^-1"
                | "W sr^-1 m^-2 Hz^-1"
                | "W sr^-1 m^-2 m^-1"
                | "V/m"
                | "T"
                | "A/m"
                | "V"
                | "Gy"
                | "Sv"
                | "photons m^-2"
                | "J"
                | "A m"
                | "A*m"
                | "sr^-1"
                | "1"
                | "counts"
                | "mixed:mode-registry"
                | "V/W"
                | "A/W"
        ) {
            Ok(())
        } else {
            Err(semantics::Error::UnsupportedDomain)
        }
    }
}
fn req_critical_ref<'a>(
    block: Block<'a>,
    key: u16,
    expected_schema: u16,
    ctx: &Context<'_, 'a>,
) -> Result<Block<'a>, Error> {
    let f = required(block, key)?;
    if f.flags() & 1 == 0 {
        return Err(Error::Descriptor);
    }
    ctx.require_schema(u32_at(f.value(), 0), expected_schema)
        .map_err(Error::Shared)
}
fn opt_ref<'a>(
    block: Block<'a>,
    key: u16,
    expected_schema: u16,
    ctx: &Context<'_, 'a>,
) -> Result<Option<Block<'a>>, Error> {
    if let Some(f) = field(block, key) {
        let b = ctx
            .require_schema(u32_at(f.value(), 0), expected_schema)
            .map_err(Error::Shared)?;
        Ok(Some(b))
    } else {
        Ok(None)
    }
}
fn extension(block: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), Error> {
    let raw_mode = u16::try_from(scalar32(block, 1)?).map_err(|_| Error::UnsupportedProfile)?;
    let profile = text(required(block, 6)?)?;
    if scalar32(block, 2)? != 1 {
        return Err(Error::UnsupportedProfile);
    }
    let clock = ctx
        .require_schema(scalar32(block, 3)?, 1)
        .map_err(Error::Shared)?;
    if raw_mode == 0x0106 && profile == VALIDITY_PROFILE {
        if field(block, 4).is_some() || scalar64(block, 11)? != 0 || scalar32(clock, 1)? != 0 {
            return Err(Error::UnsupportedProfile);
        }
        ctx.require_schema(scalar32(block, 5)?, 4)
            .map_err(Error::Shared)?;
        for f in block.fields() {
            if f.key() >= 12 {
                return Err(Error::UnsupportedProfile);
            }
        }
        return Ok(());
    }
    let canonical = canonical_emf_mode(raw_mode).ok_or(Error::UnsupportedProfile)?;
    let expected = match canonical {
        0x0100 => FIELD_PROFILE,
        0x0101 => RAY_PROFILE,
        0x0102 => yee::YEE_PROFILE,
        0x0103 => SPECTRUM_PROFILE,
        0x0104 => mueller::MUELLER_PROFILE,
        0x0105 => iq::IQ_PROFILE,
        0x0106 => dose_biofield::DOSE_PROFILE,
        0x0107 => dose_biofield::BIOFIELD_PROFILE,
        _ => return Err(Error::UnsupportedProfile),
    };
    if profile != expected {
        return Err(Error::UnsupportedProfile);
    }
    let frame_f = required(block, 4)?;
    if frame_f.flags() & 1 == 0 {
        return Err(Error::Descriptor);
    }
    let frame = ctx
        .require_schema(u32_at(frame_f.value(), 0), 2)
        .map_err(Error::Shared)?;
    ctx.require_schema(scalar32(block, 5)?, 4)
        .map_err(Error::Shared)?;

    if canonical == 0x0102 {
        yee::validate_yee_extension(block, frame, ctx)?;
    } else if canonical == 0x0100 && [5, 6, 7].iter().any(|&key| field(frame, key).is_some()) {
        spatial::from_metadata(frame, block)?;
    } else {
        if text(required(block, 7)?)? != "linear" || scalar32(block, 10)? != 0 {
            return Err(Error::UnsupportedProfile);
        }
        for v in required(block, 8)?.value().chunks_exact(8) {
            if u64_at(v, 0) != 1 {
                return Err(Error::UnsupportedProfile);
            }
        }
        for v in required(block, 9)?.value().chunks_exact(4) {
            if u32_at(v, 0) != 1 {
                return Err(Error::UnsupportedProfile);
            }
        }
    }
    if (scalar64(block, 11)? as i64) < 0 {
        return Err(Error::Time);
    }
    // Verify optional cross-referenced schemas whenever attached and reject unimplemented keys.
    let _ = opt_ref(block, 13, 105, ctx)?;
    let _ = opt_ref(block, 14, 133, ctx)?;
    let _ = opt_ref(block, 15, 136, ctx)?;
    let _ = opt_ref(block, 16, 135, ctx)?;
    let _ = opt_ref(block, 17, 134, ctx)?;
    let _ = opt_ref(block, 22, 137, ctx)?;
    let _ = opt_ref(block, 30, 107, ctx)?;
    for f in block.fields() {
        let k = f.key();
        if matches!(k, 20 | 21 | 23 | 24 | 25)
            || (k == 18 && canonical != 0x0105)
            || (matches!(k, 26..=29) && canonical != 0x0102)
            || ((k == 12 || k == 19) && canonical != 0x0103)
            || (k == 14 && canonical != 0x0102)
            || (k == 15 && canonical != 0x0105)
            || (k == 16 && !matches!(canonical, 0x0101 | 0x0104))
            || (k == 22 && canonical != 0x0107)
        {
            return Err(Error::UnsupportedProfile);
        }
    }
    match canonical {
        0x0100 => {}
        0x0101 => {
            let _ = req_critical_ref(block, 16, 135, ctx)?;
        }
        0x0102 => {
            let _ = req_critical_ref(block, 14, 133, ctx)?;
        }
        0x0103 => {
            let _ = req_critical_ref(block, 12, 132, ctx)?;
            let f19 = required(block, 19)?;
            if f19.flags() & 1 == 0 {
                return Err(Error::Descriptor);
            }
            if !(1..=4096).contains(&u32_at(f19.value(), 0)) {
                return Err(Error::Shape);
            }
        }
        0x0104 => {
            let _ = req_critical_ref(block, 16, 135, ctx)?;
            let _ = req_critical_ref(block, 17, 134, ctx)?;
        }
        0x0105 => {
            let acq = req_critical_ref(block, 15, 136, ctx)?;
            let cal = req_critical_ref(block, 30, 107, ctx)?;
            if scalar32(acq, 6)? != cal.id() {
                return Err(Error::Descriptor);
            }
            let n_f = required(block, 18)?;
            if !(1..=iq::MAX_IQ_SAMPLES).contains(&u32_at(n_f.value(), 0)) {
                return Err(Error::Shape);
            }
        }
        0x0106 => {
            let ev = req_critical_ref(block, 17, 134, ctx)?;
            let _ = required(ev, 9)?;
            let _ = required(ev, 10)?;
            let _ = required(ev, 12)?;
            let _ = req_critical_ref(block, 30, 107, ctx)?;
        }
        0x0107 => {
            let ev = req_critical_ref(block, 17, 134, ctx)?;
            let bio = req_critical_ref(block, 22, 137, ctx)?;
            let cal = req_critical_ref(block, 30, 107, ctx)?;
            if let Some(bc) = field(bio, 5) {
                if u32_at(bc.value(), 0) != cal.id() {
                    return Err(Error::Descriptor);
                }
            }
            if let Some(be) = field(bio, 6) {
                if u32_at(be.value(), 0) != ev.id() {
                    return Err(Error::Descriptor);
                }
            }
        }
        _ => return Err(Error::UnsupportedProfile),
    }
    Ok(())
}
fn polarization(block: Block<'_>) -> Result<(), Error> {
    if text(required(block, 1)?)? != "deterministic_transverse_v1"
        || text(required(block, 2)?)? != "I_Q_U_V_v1"
        || text(required(block, 4)?)? != "time_averaged"
    {
        return Err(Error::UnsupportedProfile);
    }
    if !["W m^-2", "W sr^-1 m^-2", "1", "sr^-1"].contains(&text(required(block, 3)?)?) {
        return Err(Error::Descriptor);
    }
    Ok(())
}

fn profile_error(error: Error) -> semantics::Error {
    match error {
        Error::Shared(e) => e,
        Error::UnsupportedProfile => semantics::Error::UnsupportedDomain,
        Error::Frame => semantics::Error::Frame,
        Error::Time => semantics::Error::Time,
        Error::Overflow => semantics::Error::Overflow,
        Error::Limit => semantics::Error::Limit,
        _ => semantics::Error::Value,
    }
}
