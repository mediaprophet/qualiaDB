//! Required clinical-risk arguments. Missing fields fail closed; they are
//! never treated as patient values.

use super::super::args;
use vibe::{Diagnostic, Span, Value};

pub fn need_u64(args_v: &Value, span: Span, key: &str, unit: &str) -> Result<u64, Diagnostic> {
    args::rec_u64(args_v, key).ok_or_else(|| {
        args::bad(
            span,
            format!("{key} ({unit}) is required; missing is not a patient value"),
        )
    })
}

pub fn need_f64(args_v: &Value, span: Span, key: &str, unit: &str) -> Result<f64, Diagnostic> {
    let value = args::rec_f64(args_v, key).ok_or_else(|| {
        args::bad(
            span,
            format!("{key} ({unit}) is required; missing is not a patient value"),
        )
    })?;
    if !value.is_finite() {
        return Err(args::bad(
            span,
            format!("{key} ({unit}) must be a finite number"),
        ));
    }
    Ok(value)
}

pub fn need_bool(args_v: &Value, span: Span, key: &str) -> Result<bool, Diagnostic> {
    args::rec_bool(args_v, key).ok_or_else(|| {
        args::bad(
            span,
            format!("{key} must be true or false; omitting it is not false"),
        )
    })
}

pub fn need_str<'a>(args_v: &'a Value, span: Span, key: &str) -> Result<&'a str, Diagnostic> {
    let value = args::rec_str(args_v, key)
        .ok_or_else(|| args::bad(span, format!("{key} is required; missing is not a default")))?;
    if value.trim().is_empty() {
        return Err(args::bad(
            span,
            format!("{key} is required; empty is not a default"),
        ));
    }
    Ok(value)
}

pub fn need_age_years(
    args_v: &Value,
    span: Span,
    min_inclusive: u8,
    max_inclusive: u8,
    applicability: &str,
) -> Result<u8, Diagnostic> {
    let age = need_u64(args_v, span, "age", "years")?;
    if age < u64::from(min_inclusive) || age > u64::from(max_inclusive) {
        return Err(args::bad(
            span,
            format!(
                "age {age} years is outside this algorithm's applicability ({min_inclusive}–{max_inclusive}; {applicability}); inapplicable input cannot calculate"
            ),
        ));
    }
    u8::try_from(age).map_err(|_| args::bad(span, "age (years) is out of range"))
}

pub fn need_positive_f64(
    args_v: &Value,
    span: Span,
    key: &str,
    unit: &str,
    min_exclusive: f64,
    max_inclusive: f64,
) -> Result<f64, Diagnostic> {
    let value = need_f64(args_v, span, key, unit)?;
    if value <= min_exclusive || value > max_inclusive {
        return Err(args::bad(
            span,
            format!("{key} ({unit}) must be in ({min_exclusive}, {max_inclusive}]"),
        ));
    }
    Ok(value)
}

pub fn provenance(
    algorithm: &'static str,
    version: &'static str,
    citation: &'static str,
    applicability: &'static str,
    units: &'static str,
) -> [(&'static str, Value); 7] {
    [
        ("algorithm", Value::String(algorithm.into())),
        ("version", Value::String(version.into())),
        ("citation", Value::String(citation.into())),
        ("applicability", Value::String(applicability.into())),
        ("units", Value::String(units.into())),
        ("not_diagnosis", Value::Bool(true)),
        (
            "not_advice",
            Value::String(
                "This number is not a diagnosis, treatment recommendation, or clinical advice."
                    .into(),
            ),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn missing_bool_is_not_false() {
        let empty = Value::Record(BTreeMap::new());
        assert!(need_bool(&empty, span(), "diabetic").is_err());
    }

    #[test]
    fn missing_age_is_not_a_default() {
        let empty = Value::Record(BTreeMap::new());
        assert!(need_age_years(&empty, span(), 30, 74, "Framingham").is_err());
    }
}
