use super::*;
/// Exact type/count rule; None count admits a bounded array/string length.
#[derive(Clone, Copy, Debug)]
pub struct FieldRule {
    pub key: u16,
    pub type_tag: u8,
    pub count: Option<u32>,
    pub required: bool,
}
const fn r(key: u16, type_tag: u8, count: Option<u32>, required: bool) -> FieldRule {
    FieldRule {
        key,
        type_tag,
        count,
        required,
    }
}
const CLOCK: &[FieldRule] = &[
    r(1, 1, Some(1), true),
    r(2, 3, Some(1), true),
    r(3, 1, Some(1), true),
    r(4, 2, Some(1), true),
    r(5, 2, Some(1), true),
];
const FRAME: &[FieldRule] = &[
    r(1, 1, Some(1), true),
    r(2, 8, Some(1), false),
    r(3, 5, Some(3), true),
    r(4, 5, Some(4), true),
    r(5, 5, Some(3), false),
    r(6, 1, Some(3), false),
    r(7, 1, Some(1), false),
    r(8, 6, None, true),
    r(9, 7, None, false),
];
const DESC: &[FieldRule] = &[
    r(1, 8, Some(1), false),
    r(2, 8, Some(1), false),
    r(3, 8, Some(1), true),
    r(4, 8, Some(1), false),
    r(5, 8, Some(1), false),
    r(6, 8, Some(1), false),
    r(7, 6, None, true),
    r(8, 6, None, true),
    r(9, 1, Some(1), false),
    r(10, 6, None, false),
    r(11, 1, Some(1), false),
    r(12, 6, None, true),
    r(13, 2, Some(1), false),
    r(14, 2, Some(1), false),
    r(15, 2, Some(1), false),
    r(16, 9, Some(1), false),
    r(17, 9, None, false),
    r(18, 8, None, false),
];
const PROV: &[FieldRule] = &[
    r(1, 1, Some(1), true),
    r(2, 7, None, false),
    r(3, 6, None, false),
    r(4, 6, None, false),
    r(5, 6, None, false),
    r(6, 7, Some(32), false),
    r(7, 6, None, false),
];
const INSTANT: &[FieldRule] = &[
    r(1, 8, Some(1), true),
    r(2, 3, Some(1), true),
    r(3, 2, Some(1), true),
    r(4, 2, Some(1), true),
];
pub(super) fn shared_rules(schema: u16, revision: u16) -> Option<&'static [FieldRule]> {
    if revision != 1 {
        return None;
    }
    match schema {
        1 => Some(CLOCK),
        2 => Some(FRAME),
        3 => Some(DESC),
        4 => Some(PROV),
        5 => Some(INSTANT),
        _ => None,
    }
}
/// Exact representation and required-critical checks, reusable by domain registries.
/// Unknown optional keys remain preserved. Critical unknown keys/types reject.
pub fn validate_fields(block: Block<'_>, rules: &[FieldRule]) -> Result<(), Error> {
    for rule in rules {
        if rule.required {
            let f = required(block, rule.key)?;
            if f.flags() & 1 == 0 {
                return Err(Error::RequiredCritical);
            }
        }
    }
    for f in block.fields() {
        if !(1..=11).contains(&f.type_tag()) && f.flags() & 1 != 0 {
            return Err(Error::UnknownCriticalType);
        }
        if let Some(rule) = rules.iter().find(|r| r.key == f.key()) {
            if f.type_tag() != rule.type_tag || rule.count.is_some_and(|n| n != f.element_count()) {
                return Err(Error::FieldType);
            }
        } else if f.flags() & 1 != 0 {
            return Err(Error::UnknownCriticalField);
        }
    }
    Ok(())
}
