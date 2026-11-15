use super::*;
use crate::q42::field_semantics::{Context, DomainRegistry};
pub const SIGNAL_SCHEMA: u16 = 210;
pub const SHAPE_SCHEMA: u16 = 205;
pub const TRANSFORM_SCHEMA: u16 = 201;
const SIGNAL: &[FieldRule] = &[
    r(1, 6, None, true),
    r(2, 5, Some(1), true),
    r(3, 5, Some(1), true),
    r(4, 6, None, true),
    r(5, 6, None, true),
];
const SHAPE: &[FieldRule] = &[
    r(1, 6, None, true),
    r(2, 6, None, true),
    r(14, 2, None, true),
    r(15, 6, None, true),
];
const TRANSFORM: &[FieldRule] = &[
    r(1, 6, None, true),
    r(2, 1, Some(1), true),
    r(3, 1, Some(1), true),
    r(4, 6, None, true),
    r(5, 9, Some(1), false),
    r(6, 6, None, true),
    r(7, 1, Some(1), true),
    r(8, 9, Some(1), false),
    r(9, 6, None, true),
    r(10, 3, Some(1), true),
    r(11, 6, None, true),
    r(12, 6, None, true),
    r(13, 1, Some(1), true),
    r(14, 2, Some(1), true),
    r(15, 2, Some(1), true),
    r(16, 4, None, false),
    r(17, 6, None, false),
];
const CAL: &[FieldRule] = &[
    r(1, 6, None, true),
    r(2, 6, None, true),
    r(3, 5, Some(1), true),
    r(4, 5, Some(1), false),
    r(5, 5, Some(1), false),
    r(6, 9, Some(1), false),
    r(7, 9, Some(1), false),
    r(8, 7, Some(32), false),
    r(9, 8, Some(1), false),
    r(10, 8, Some(1), false),
];
/// Candidate preservation metadata. Calibration is retained lineage, not applied.
#[derive(Clone, Copy, Debug, Default)]
pub struct SignalRegistry;
impl DomainRegistry for SignalRegistry {
    fn rules(&self, s: u16, v: u16) -> Option<&'static [FieldRule]> {
        if v != 1 {
            return None;
        }
        match s {
            201 => Some(TRANSFORM),
            202 => Some(CAL),
            205 => Some(SHAPE),
            210 => Some(SIGNAL),
            200 | 203 | 204 | 206..=209 => {
                crate::audio::qac::profiles::QacProfileRegistry.rules(s, v)
            }
            _ => None,
        }
    }
    fn descriptor_target_schema(&self, key: u16) -> Option<u16> {
        match key {
            4 => Some(200),
            5 => Some(201),
            6 => Some(202),
            _ => None,
        }
    }
    fn validate_units(&self, units: &str, ctx: &Context<'_, '_>) -> Result<(), sem::Error> {
        crate::audio::qac::profiles::QacProfileRegistry.validate_units(units, ctx)
    }
    fn validate_block(&self, b: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), sem::Error> {
        ctx.charge_work(
            (b.fields().count() as u64)
                .checked_mul(32)
                .ok_or(sem::Error::Overflow)?,
        )?;
        match b.schema_type() {
            201 => {
                if let Some(w) = sem::field(b, 16) {
                    ctx.charge_work(w.value().len() as u64)?;
                }
                Transform::from_block(b).map_err(|_| sem::Error::Value)?;
            }
            205 => {
                if ![
                    "urn:qualia:qac:pcm-preservation:1",
                    "urn:qualia:qac:stft-preservation:1",
                    "urn:qualia:qac:opaque-original:1",
                ]
                .contains(&text(b, 1)?)
                {
                    return Err(sem::Error::UnsupportedDomain);
                }
                if text(b, 2)? != "1" {
                    return Err(sem::Error::Value);
                }
                let shape = get(b, 14)?;
                if shape.element_count() == 0 || shape.element_count() > 3 {
                    return Err(sem::Error::Value);
                }
                let mut n = 1u64;
                for raw in shape.value().chunks_exact(8) {
                    let x = u64::from_le_bytes(raw.try_into().unwrap());
                    if x == 0 {
                        return Err(sem::Error::Value);
                    }
                    n = n.checked_mul(x).ok_or(sem::Error::Overflow)?;
                }
                text(b, 15)?;
            }
            210 => {
                text(b, 1)?;
                text(b, 4)?;
                let scale = f64v(b, 2)?;
                let offset = f64v(b, 3)?;
                if !scale.is_finite() || scale <= 0.0 || !offset.is_finite() {
                    return Err(sem::Error::Value);
                }
                match text(b, 5)? {
                    "physical-base-units" | "raw-counts" => {
                        if scale != 1.0 || offset != 0.0 {
                            return Err(sem::Error::Value);
                        }
                    }
                    "digital-unrelated-to-pressure" => {}
                    _ => return Err(sem::Error::Value),
                }
            }
            202 => {
                text(b, 1)?;
                text(b, 2)?;
                if f64v(b, 3)? == 0.0 {
                    return Err(sem::Error::Value);
                }
                if sem::field(b, 5).is_some() && f64v(b, 5)? <= 0.0 {
                    return Err(sem::Error::Value);
                }
                for key in [9, 10] {
                    if sem::field(b, key).is_some() {
                        ctx.require_schema(u32v(b, key)?, 5)?;
                    }
                }
                if sem::field(b, 9).is_some() && sem::field(b, 10).is_some() {
                    let a = ctx.require_schema(u32v(b, 9)?, 5)?;
                    let z = ctx.require_schema(u32v(b, 10)?, 5)?;
                    if u32v(a, 1)? != u32v(z, 1)? {
                        return Err(sem::Error::Time);
                    }
                    let at = u64v(a, 2)? as i64;
                    let zt = u64v(z, 2)? as i64;
                    if at > zt
                        || (at == zt
                            && u128::from(u64v(a, 3)?) * u128::from(u64v(z, 4)?)
                                >= u128::from(u64v(z, 3)?) * u128::from(u64v(a, 4)?))
                    {
                        return Err(sem::Error::Time);
                    }
                }
            }
            200 | 203 | 204 | 206..=209 => {
                crate::audio::qac::profiles::QacProfileRegistry.validate_block(b, ctx)?;
            }
            _ => return Err(sem::Error::UnsupportedDomain),
        }
        Ok(())
    }
}
