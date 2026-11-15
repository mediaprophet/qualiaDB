//! Signal metadata tables with CONTROL-approved schema210 allocation.
mod registry;
mod transform;
use crate::q42::{
    field_envelope::{Block, Field},
    field_semantics::{self as sem, FieldRule},
};
pub use registry::{SignalRegistry, SHAPE_SCHEMA, SIGNAL_SCHEMA, TRANSFORM_SCHEMA};
pub use transform::{FrequencyOffset, OriginBasis, Transform, TransformKind, WindowKind};
pub(crate) const fn r(key: u16, tag: u8, count: Option<u32>, required: bool) -> FieldRule {
    FieldRule {
        key,
        type_tag: tag,
        count,
        required,
    }
}
pub(crate) fn get<'a>(b: Block<'a>, key: u16) -> Result<Field<'a>, sem::Error> {
    sem::field(b, key).ok_or(sem::Error::MissingField)
}
pub(crate) fn u32v(b: Block<'_>, key: u16) -> Result<u32, sem::Error> {
    let f = get(b, key)?;
    if ![1, 8, 9].contains(&f.type_tag()) || f.value().len() != 4 {
        return Err(sem::Error::FieldType);
    }
    Ok(u32::from_le_bytes(f.value().try_into().unwrap()))
}
pub(crate) fn u64v(b: Block<'_>, key: u16) -> Result<u64, sem::Error> {
    let f = get(b, key)?;
    if ![2, 3].contains(&f.type_tag()) || f.value().len() != 8 {
        return Err(sem::Error::FieldType);
    }
    Ok(u64::from_le_bytes(f.value().try_into().unwrap()))
}
pub(crate) fn f64v(b: Block<'_>, key: u16) -> Result<f64, sem::Error> {
    let f = get(b, key)?;
    if f.type_tag() != 5 || f.value().len() != 8 {
        return Err(sem::Error::FieldType);
    }
    Ok(f64::from_le_bytes(f.value().try_into().unwrap()))
}
pub(crate) fn text<'a>(b: Block<'a>, key: u16) -> Result<&'a str, sem::Error> {
    let f = get(b, key)?;
    if f.type_tag() != 6 {
        return Err(sem::Error::FieldType);
    }
    let s = std::str::from_utf8(f.value()).map_err(|_| sem::Error::Value)?;
    if s.is_empty() || s.chars().any(char::is_control) {
        return Err(sem::Error::Value);
    }
    Ok(s)
}
