//! Wave-13 Host binds: remaining chemistry integral / GTO angular helpers.
//!
//! Wraps `IntegralEngine::evaluate_eri`, `GtoPrimitive::total_angular_momentum`,
//! and `AngularMomentum::{letter,n_cartesian,n_spherical,from_letter}` — Host-missing
//! pure surfaces not bound in waves 1–12.

use super::super::args;
use crate::specialized_libs::chemistry_modeling::basis_set::AngularMomentum;
use crate::specialized_libs::chemistry_modeling::integrals::{GtoPrimitive, IntegralEngine};
use vibe::{Diagnostic, Span, Value};

fn gto_at(args_v: &Value, key: &str, span: Span) -> Result<GtoPrimitive, Diagnostic> {
    let v = args::rec(args_v, key)
        .ok_or_else(|| args::bad(span, format!("Chemistry needs {key}: gto record")))?;
    let origin = args::rec_f64_list(v, "origin")
        .ok_or_else(|| args::bad(span, format!("{key}.origin: [x,y,z]")))?;
    if origin.len() != 3 {
        return Err(args::bad(span, format!("{key}.origin must have length 3")));
    }
    let exponent = args::rec_f64(v, "exponent")
        .ok_or_else(|| args::bad(span, format!("{key}.exponent required")))?;
    let lx = args::rec_u64(v, "lx").unwrap_or(0) as u8;
    let ly = args::rec_u64(v, "ly").unwrap_or(0) as u8;
    let lz = args::rec_u64(v, "lz").unwrap_or(0) as u8;
    let coefficient = args::rec_f64(v, "coefficient").unwrap_or(1.0);
    Ok(GtoPrimitive {
        origin: [origin[0], origin[1], origin[2]],
        exponent,
        l: [lx, ly, lz],
        coefficient,
    })
}

fn am_from_args(args_v: &Value, span: Span) -> Result<AngularMomentum, Diagnostic> {
    if let Some(l) = args::rec_u64(args_v, "l") {
        return Ok(AngularMomentum(l as u8));
    }
    if let Some(s) = args::rec_str(args_v, "letter") {
        let c = s.chars().next().ok_or_else(|| {
            args::bad(span, "Chemistry angular helpers need letter: non-empty string")
        })?;
        return AngularMomentum::from_letter(c)
            .ok_or_else(|| args::bad(span, format!("unknown spectroscopic letter '{c}'")));
    }
    Err(args::bad(
        span,
        "Chemistry angular helpers need l: u64 or letter: string",
    ))
}

/// `Chemistry.evaluate_eri` — two-electron repulsion (ab|cd). Args: `{ a,b,c,d }`.
/// Out: `{ value }`.
pub fn evaluate_eri(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = gto_at(args_v, "a", span)?;
    let b = gto_at(args_v, "b", span)?;
    let c = gto_at(args_v, "c", span)?;
    let d = gto_at(args_v, "d", span)?;
    Ok(args::record([(
        "value",
        Value::F64(IntegralEngine::evaluate_eri(&a, &b, &c, &d)),
    )]))
}

fn gto_flat(args_v: &Value, span: Span) -> Result<GtoPrimitive, Diagnostic> {
    let origin = args::rec_f64_list(args_v, "origin")
        .ok_or_else(|| args::bad(span, "total_angular_momentum needs origin"))?;
    if origin.len() != 3 {
        return Err(args::bad(span, "origin must have length 3"));
    }
    let exponent = args::rec_f64(args_v, "exponent")
        .ok_or_else(|| args::bad(span, "total_angular_momentum needs exponent"))?;
    Ok(GtoPrimitive {
        origin: [origin[0], origin[1], origin[2]],
        exponent,
        l: [
            args::rec_u64(args_v, "lx").unwrap_or(0) as u8,
            args::rec_u64(args_v, "ly").unwrap_or(0) as u8,
            args::rec_u64(args_v, "lz").unwrap_or(0) as u8,
        ],
        coefficient: args::rec_f64(args_v, "coefficient").unwrap_or(1.0),
    })
}

/// `Chemistry.total_angular_momentum` — lx+ly+lz for a GTO. Args: `{ gto }` or bare
/// gto fields. Out: `{ value }`.
pub fn total_angular_momentum(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let gto = if args::rec(args_v, "gto").is_some() {
        gto_at(args_v, "gto", span)?
    } else {
        gto_flat(args_v, span)?
    };
    Ok(args::record([(
        "value",
        Value::U64(gto.total_angular_momentum() as u64),
    )]))
}

/// `Chemistry.letter` — spectroscopic letter for angular momentum. Args: `{ l }`.
/// Out: `{ letter }`.
pub fn letter(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let am = am_from_args(args_v, span)?;
    Ok(args::record([(
        "letter",
        Value::String(am.letter().to_string()),
    )]))
}

/// `Chemistry.n_cartesian` — (l+1)(l+2)/2. Args: `{ l }` or `{ letter }`. Out: `{ value }`.
pub fn n_cartesian(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let am = am_from_args(args_v, span)?;
    Ok(args::record([("value", Value::U64(am.n_cartesian() as u64))]))
}

/// `Chemistry.n_spherical` — 2l+1. Args: `{ l }` or `{ letter }`. Out: `{ value }`.
pub fn n_spherical(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let am = am_from_args(args_v, span)?;
    Ok(args::record([("value", Value::U64(am.n_spherical() as u64))]))
}

/// `Chemistry.from_letter` — letter → l. Args: `{ letter }`. Out: `{ l }`.
pub fn from_letter(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let s = args::rec_str(args_v, "letter")
        .ok_or_else(|| args::bad(span, "Chemistry.from_letter needs letter"))?;
    let c = s
        .chars()
        .next()
        .ok_or_else(|| args::bad(span, "letter must be non-empty"))?;
    let am = AngularMomentum::from_letter(c)
        .ok_or_else(|| args::bad(span, format!("unknown spectroscopic letter '{c}'")))?;
    Ok(args::record([("l", Value::U64(am.0 as u64))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn s_gto(origin: [f64; 3], exponent: f64) -> Value {
        let mut m = BTreeMap::new();
        m.insert(
            "origin".into(),
            args::f64_list_value([origin[0], origin[1], origin[2]]),
        );
        m.insert("exponent".into(), Value::F64(exponent));
        m.insert("coefficient".into(), Value::F64(1.0));
        Value::Record(m)
    }

    #[test]
    fn wave13_evaluate_eri_same_center_positive() {
        let g = s_gto([0.0, 0.0, 0.0], 1.0);
        let mut m = BTreeMap::new();
        m.insert("a".into(), g.clone());
        m.insert("b".into(), g.clone());
        m.insert("c".into(), g.clone());
        m.insert("d".into(), g);
        let out = evaluate_eri(&Value::Record(m), span()).unwrap();
        let v = args::rec_f64(&out, "value").unwrap();
        assert!(v > 0.0, "ERI (ss|ss) on-center should be positive, got {v}");
    }

    #[test]
    fn wave13_total_angular_momentum_p_orbital() {
        let mut m = BTreeMap::new();
        m.insert("origin".into(), args::f64_list_value([0.0, 0.0, 0.0]));
        m.insert("exponent".into(), Value::F64(1.0));
        m.insert("lx".into(), Value::U64(1));
        m.insert("ly".into(), Value::U64(0));
        m.insert("lz".into(), Value::U64(0));
        let out = total_angular_momentum(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_u64(&out, "value"), Some(1));
    }

    #[test]
    fn wave13_angular_momentum_letter_cartesian_spherical() {
        let mut m = BTreeMap::new();
        m.insert("l".into(), Value::U64(2));
        assert_eq!(
            args::rec_str(&letter(&Value::Record(m.clone()), span()).unwrap(), "letter"),
            Some("d")
        );
        assert_eq!(
            args::rec_u64(&n_cartesian(&Value::Record(m.clone()), span()).unwrap(), "value"),
            Some(6)
        );
        assert_eq!(
            args::rec_u64(&n_spherical(&Value::Record(m), span()).unwrap(), "value"),
            Some(5)
        );

        let mut fl = BTreeMap::new();
        fl.insert("letter".into(), Value::String("p".into()));
        assert_eq!(
            args::rec_u64(&from_letter(&Value::Record(fl), span()).unwrap(), "l"),
            Some(1)
        );
    }
}
