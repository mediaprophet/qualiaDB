//! Schema rules and domain registry for frozen EMF v4 wire modes (`0x0100..=0x0107` / `0..=7`).

use super::extended_records::*;
use crate::q42::{
    emf_profiles as profiles,
    field_envelope::{Block, StructuralView},
    field_semantics::{self as sem, field, Context, DomainRegistry, FieldRule},
    field_spectrum::{self as spectrum, NoResponse, SigmaInterpretation},
};
use crate::render::portal_load_emf::Error;
use profiles::{GridIndexing, GridSpec};

const fn fr(key: u16, type_tag: u8, count: Option<u32>, required: bool) -> FieldRule {
    FieldRule { key, type_tag, count, required }
}

const RULES_105: &[FieldRule] = &[
    fr(1, 7, Some(32), true), fr(2, 6, None, true), fr(3, 7, Some(32), true),
    fr(4, 2, Some(1), true), fr(5, 8, Some(1), true), fr(6, 8, Some(1), false),
    fr(7, 8, Some(1), false), fr(8, 6, None, true), fr(9, 2, None, true),
];
const RULES_106: &[FieldRule] = &[
    fr(1, 8, Some(1), true), fr(2, 8, Some(1), true), fr(3, 6, None, true),
    fr(4, 5, Some(16), true), fr(5, 8, Some(1), false), fr(6, 3, Some(2), false),
];
const RULES_107: &[FieldRule] = &[
    fr(1, 6, None, true), fr(2, 7, Some(32), true), fr(3, 6, None, true),
    fr(4, 5, Some(1), true), fr(5, 5, Some(1), true), fr(6, 5, Some(1), false),
    fr(7, 6, None, false), fr(8, 3, Some(2), false), fr(9, 8, Some(1), false),
];
const RULES_133: &[FieldRule] = &[
    fr(1, 6, None, true), fr(2, 5, Some(1), true), fr(3, 5, Some(1), true),
    fr(4, 5, Some(1), true), fr(5, 5, None, false), fr(6, 5, None, false),
    fr(7, 5, None, false), fr(8, 5, None, false), fr(9, 9, Some(1), false),
    fr(10, 8, Some(1), false), fr(11, 8, Some(1), false),
];
const RULES_134: &[FieldRule] = &[
    fr(1, 6, None, true), fr(2, 6, None, true), fr(3, 7, Some(32), true),
    fr(4, 6, None, true), fr(5, 6, None, false), fr(6, 6, None, false),
    fr(7, 6, None, false), fr(8, 6, None, true), fr(9, 6, None, false),
    fr(10, 3, Some(2), false), fr(11, 6, None, false), fr(12, 8, Some(1), false),
];
const RULES_136: &[FieldRule] = &[
    fr(1, 8, Some(1), true), fr(2, 2, Some(1), true), fr(3, 2, Some(1), true),
    fr(4, 6, None, true), fr(5, 6, None, true), fr(6, 8, Some(1), true),
    fr(7, 6, None, true), fr(8, 6, None, false), fr(9, 1, Some(1), false),
    fr(10, 1, Some(1), false), fr(11, 6, None, false), fr(12, 5, Some(1), false),
    fr(13, 1, Some(1), false),
];
const RULES_137: &[FieldRule] = &[
    fr(1, 6, None, true), fr(2, 6, None, true), fr(3, 6, None, true),
    fr(4, 8, Some(1), true), fr(5, 8, Some(1), false), fr(6, 8, Some(1), false),
    fr(7, 8, Some(1), false),
];

pub(super) fn canonical_emf_mode(mode: u16) -> Option<u16> {
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

pub(super) fn s32(b: Block<'_>, k: u16) -> Result<u32, sem::Error> {
    let f = field(b, k).ok_or(sem::Error::MissingField)?;
    Ok(u32::from_le_bytes(f.value().try_into().map_err(|_| sem::Error::FieldType)?))
}

fn s64(b: Block<'_>, k: u16) -> Result<u64, sem::Error> {
    let f = field(b, k).ok_or(sem::Error::MissingField)?;
    Ok(u64::from_le_bytes(f.value().try_into().map_err(|_| sem::Error::FieldType)?))
}

fn f64_scalar(b: Block<'_>, k: u16) -> Result<f64, sem::Error> {
    let f = field(b, k).ok_or(sem::Error::MissingField)?;
    Ok(f64::from_le_bytes(f.value().try_into().map_err(|_| sem::Error::FieldType)?))
}

fn txt<'a>(b: Block<'a>, k: u16) -> Result<&'a str, sem::Error> {
    let f = field(b, k).ok_or(sem::Error::MissingField)?;
    std::str::from_utf8(f.value()).map_err(|_| sem::Error::Value)
}

pub(super) fn find_ext_block<'v, 'a>(
    view: &'v StructuralView<'a>,
    desc: Block<'a>,
) -> Result<Option<Block<'a>>, Error> {
    let Some(ext_field) = field(desc, 18) else {
        return Ok(None);
    };
    for id_bytes in ext_field.value().chunks_exact(4) {
        let id = u32::from_le_bytes(id_bytes.try_into().unwrap());
        if let Some(b) = view.blocks().find(|b| b.id() == id) {
            if b.schema_type() == 103 && b.revision() == 1 {
                return Ok(Some(b));
            }
        }
    }
    Ok(None)
}

pub(super) fn grid_from_blocks(
    frame: Block<'_>,
    ext: Block<'_>,
) -> Result<Option<profiles::GridLayout>, Error> {
    if field(frame, 5).is_none() || field(frame, 6).is_none() {
        return Ok(None);
    }
    let spacing_bytes = field(frame, 5).unwrap().value();
    let dims_bytes = field(frame, 6).unwrap().value();
    let ext_dims = field(ext, 8).ok_or(Error::Profile(profiles::Error::Descriptor))?.value();
    let bricks = field(ext, 9).ok_or(Error::Profile(profiles::Error::Descriptor))?.value();
    let order = u32_at(field(ext, 10).ok_or(Error::Profile(profiles::Error::Descriptor))?.value(), 0);
    let idx_name = std::str::from_utf8(
        field(ext, 7).ok_or(Error::Profile(profiles::Error::Descriptor))?.value(),
    )
    .map_err(|_| Error::Profile(profiles::Error::Descriptor))?;
    let mut dimensions = [0u32; 3];
    let mut brick_dimensions = [0u32; 3];
    let mut spacing_m = [0.0f64; 3];
    for i in 0..3 {
        dimensions[i] = u32_at(dims_bytes, i * 4);
        if u64::from(dimensions[i]) != u64_at(ext_dims, i * 8) {
            return Err(Error::Profile(profiles::Error::Frame));
        }
        brick_dimensions[i] = u32_at(bricks, i * 4);
        spacing_m[i] = f64_at(spacing_bytes, i * 8);
    }
    let indexing = match idx_name {
        "linear" => {
            if order != 0 {
                return Err(Error::Profile(profiles::Error::Frame));
            }
            GridIndexing::Linear
        }
        "morton" => GridIndexing::Morton { order },
        "hilbert" => GridIndexing::Hilbert { order },
        _ => return Err(Error::Profile(profiles::Error::UnsupportedProfile)),
    };
    let layout = GridSpec { dimensions, brick_dimensions, spacing_m, indexing }.validate()?;
    Ok(Some(layout))
}

pub(super) struct ExtendedEmfRegistry;

impl DomainRegistry for ExtendedEmfRegistry {
    fn rules(&self, schema: u16, revision: u16) -> Option<&'static [FieldRule]> {
        if revision != 1 {
            return None;
        }
        match schema {
            103 => Some(profiles::extension_rules()),
            105 => Some(RULES_105),
            106 => Some(RULES_106),
            107 => Some(RULES_107),
            132 => Some(spectrum::schema_rules()),
            133 => Some(RULES_133),
            134 => Some(RULES_134),
            135 => Some(profiles::polarization_rules()),
            136 => Some(RULES_136),
            137 => Some(RULES_137),
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

    fn validate_units(&self, units: &str, _ctx: &Context<'_, '_>) -> Result<(), sem::Error> {
        const VALID: &[&str] = &[
            "W m^-2", "W sr^-1 m^-2", "W m^-2 Hz^-1", "W m^-2 m^-1",
            "W sr^-1 m^-2 Hz^-1", "W sr^-1 m^-2 m^-1", "V/m", "T", "A/m",
            "V", "Gy", "Sv", "photons m^-2", "J", "A m", "A*m", "sr^-1", "1",
            "counts", "mixed:mode-registry",
        ];
        VALID.contains(&units).then_some(()).ok_or(sem::Error::UnsupportedDomain)
    }

    fn validate_grid_frame(&self, frame: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), sem::Error> {
        for b in ctx.blocks() {
            let b = b?;
            if b.schema_type() == 103 && b.revision() == 1 {
                if field(b, 4).is_some_and(|rf| u32_at(rf.value(), 0) == frame.id())
                    && grid_from_blocks(frame, b).is_ok_and(|g| g.is_some())
                {
                    return Ok(());
                }
            }
        }
        Err(sem::Error::UnsupportedDomain)
    }

    fn validate_spatial_entry(
        &self,
        entry: crate::q42::field_envelope::Entry<'_>,
        frame: Block<'_>,
        ctx: &Context<'_, '_>,
    ) -> Result<(), sem::Error> {
        let desc = ctx.require_schema(entry.descriptor_id(), 3)?;
        let ext_ids = field(desc, 18).ok_or(sem::Error::Descriptor)?;
        let ext_id = ext_ids
            .value()
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .find(|&id| ctx.block(id).is_ok_and(|b| b.schema_type() == 103))
            .ok_or(sem::Error::Descriptor)?;
        let ext = ctx.require_schema(ext_id, 103)?;
        let layout = grid_from_blocks(frame, ext)
            .map_err(|_| sem::Error::Frame)?
            .ok_or(sem::Error::Frame)?;
        let brick = layout.brick(entry.spatial_key()).map_err(|_| sem::Error::Frame)?;
        let times = if entry.flags() & 2 != 0 { entry.time_count() } else { 1 };
        let needed = brick.cell_count().checked_mul(times).ok_or(sem::Error::Overflow)?;
        (needed == entry.record_count()).then_some(()).ok_or(sem::Error::Descriptor)
    }

    fn validate_block(&self, block: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), sem::Error> {
        match block.schema_type() {
            103 => {
                let canonical = canonical_emf_mode(s32(block, 1)? as u16)
                    .ok_or(sem::Error::UnsupportedDomain)?;
                let prof = txt(block, 6)?;
                let valid_name = match canonical {
                    0x0100 => prof == profiles::FIELD_PROFILE,
                    0x0101 => prof == profiles::RAY_PROFILE,
                    0x0102 => prof == "yee_cartesian_v1",
                    0x0103 => prof == profiles::SPECTRUM_PROFILE,
                    0x0104 => prof == "mueller_scattering_v1",
                    0x0105 => prof == "calibrated_iq_stft_v1",
                    0x0106 => prof == profiles::VALIDITY_PROFILE || prof == "ionizing_radiation_v1",
                    0x0107 => prof == "bioelectric_dipole_v1",
                    _ => false,
                };
                if !valid_name || s32(block, 2)? != 1 {
                    return Err(sem::Error::UnsupportedDomain);
                }
                ctx.require_schema(s32(block, 3)?, 1)?;
                if !(canonical == 0x0106 && prof == profiles::VALIDITY_PROFILE) {
                    ctx.require_schema(s32(block, 4)?, 2)?;
                }
                ctx.require_schema(s32(block, 5)?, 4)?;
                if canonical == 0x0103 {
                    ctx.require_schema(s32(block, 12)?, 132)?;
                    if !(1..=4096).contains(&s32(block, 19)?) {
                        return Err(sem::Error::Value);
                    }
                }
                if matches!(canonical, 0x0101 | 0x0104) {
                    ctx.require_schema(s32(block, 16)?, 135)?;
                }
                if canonical == 0x0102 {
                    ctx.require_schema(s32(block, 14)?, 133)?;
                }
                if canonical == 0x0104 || canonical == 0x0107
                    || (canonical == 0x0106 && prof == "ionizing_radiation_v1")
                {
                    let ev = ctx.require_schema(s32(block, 17)?, 134)?;
                    if canonical == 0x0106 {
                        let _ = txt(ev, 9)?;
                        let ticks = field(ev, 10).ok_or(sem::Error::MissingField)?;
                        if i64_at(ticks.value(), 8) < i64_at(ticks.value(), 0) {
                            return Err(sem::Error::Value);
                        }
                        ctx.require_schema(s32(ev, 12)?, 1)?;
                    }
                }
                if matches!(canonical, 0x0105 | 0x0107)
                    || (canonical == 0x0106 && prof == "ionizing_radiation_v1")
                {
                    ctx.require_schema(s32(block, 30)?, 107)?;
                }
                if canonical == 0x0105 {
                    ctx.require_schema(s32(block, 15)?, 136)?;
                }
                if canonical == 0x0107 {
                    ctx.require_schema(s32(block, 22)?, 137)?;
                }
                Ok(())
            }
            105 => {
                if txt(block, 2)? != "BLAKE3-256" {
                    return Err(sem::Error::Value);
                }
                ctx.require_schema(s32(block, 5)?, 2)?;
                Ok(())
            }
            106 => {
                ctx.require_schema(s32(block, 1)?, 2)?;
                ctx.require_schema(s32(block, 2)?, 2)?;
                if !matches!(txt(block, 3)?, "rigid_matrix4" | "affine_matrix4") {
                    return Err(sem::Error::Value);
                }
                let mat = field(block, 4).ok_or(sem::Error::MissingField)?.value();
                if (0..16).any(|i| !f64_at(mat, i * 8).is_finite()) {
                    return Err(sem::Error::Value);
                }
                Ok(())
            }
            107 => {
                let (scale, off) = (f64_scalar(block, 4)?, f64_scalar(block, 5)?);
                (!scale.is_finite() || scale <= 0.0 || !off.is_finite())
                    .then_some(()).map_or(Ok(()), |_| Err(sem::Error::Value))
            }
            132 => {
                let positions = field(block, 5).ok_or(sem::Error::MissingField)?.element_count();
                let samples = if txt(block, 2)? == "point_density" {
                    u64::from(positions)
                } else {
                    u64::from(positions).checked_sub(1).ok_or(sem::Error::Value)?
                };
                let avail = ctx.remaining_work();
                let axis = spectrum::admit_axis(
                    block, samples, spectrum::Limits { samples: 4096, work: avail }, &NoResponse,
                ).map_err(|_| sem::Error::Value)?;
                ctx.charge_work(avail - axis.remaining_work())?;
                match axis.sigma() {
                    SigmaInterpretation::LegacyDisplay400700nm
                    | SigmaInterpretation::LegacyEtalonDisplay380750nm => Err(sem::Error::UnsupportedDomain),
                    _ => Ok(()),
                }
            }
            133 => {
                let (eps, mu, cond) = (f64_scalar(block, 2)?, f64_scalar(block, 3)?, f64_scalar(block, 4)?);
                (eps.is_finite() && eps > 0.0 && mu.is_finite() && mu > 0.0 && cond.is_finite() && cond >= 0.0)
                    .then_some(()).ok_or(sem::Error::Value)
            }
            134 => {
                let ok = ["empirical", "model_estimate", "hypothesis", "counterfactual", "fictional"]
                    .contains(&txt(block, 8)?);
                ok.then_some(()).ok_or(sem::Error::Value)
            }
            135 => {
                if txt(block, 1)? != "deterministic_transverse_v1"
                    || txt(block, 2)? != "I_Q_U_V_v1"
                    || txt(block, 4)? != "time_averaged"
                {
                    return Err(sem::Error::UnsupportedDomain);
                }
                ["W m^-2", "W sr^-1 m^-2", "sr^-1", "1"].contains(&txt(block, 3)?)
                    .then_some(()).ok_or(sem::Error::Descriptor)
            }
            136 => {
                ctx.require_schema(s32(block, 1)?, 1)?;
                ctx.require_schema(s32(block, 6)?, 107)?;
                if txt(block, 7)? != "I_plus_iQ_exp_minus_iwt" {
                    return Err(sem::Error::Value);
                }
                if s64(block, 2)? == 0 || s64(block, 3)? == 0 {
                    return Err(sem::Error::Value);
                }
                if field(block, 12).is_some() {
                    let f = f64_scalar(block, 12)?;
                    if !f.is_finite() || f < 0.0 {
                        return Err(sem::Error::Value);
                    }
                }
                Ok(())
            }
            137 => {
                ctx.require_schema(s32(block, 4)?, 4)?;
                Ok(())
            }
            _ => Err(sem::Error::UnsupportedDomain),
        }
    }
}
