//! Wave-12 Host binds: quantum-chemistry integral primitives.
//!
//! Wraps `specialized_libs::chemistry_modeling::integrals::{IntegralEngine, GtoPrimitive}`
//! free-surface methods that had no exact `Chemistry.*` Host twin in waves 1–11.

use super::super::args;
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

/// `Chemistry.boys_function` — Boys F_n(t). Args: `{ n, t }`. Out: `{ value }`.
pub fn boys_function(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args_v, "n")
        .ok_or_else(|| args::bad(span, "Chemistry.boys_function needs n"))? as u8;
    let t = args::rec_f64(args_v, "t")
        .ok_or_else(|| args::bad(span, "Chemistry.boys_function needs t"))?;
    Ok(args::record([(
        "value",
        Value::F64(IntegralEngine::boys_function(n, t)),
    )]))
}

/// `Chemistry.overlap_s` — s-type overlap (a|b). Args: `{ a, b }`. Out: `{ value }`.
pub fn overlap_s(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = gto_at(args_v, "a", span)?;
    let b = gto_at(args_v, "b", span)?;
    Ok(args::record([(
        "value",
        Value::F64(IntegralEngine::overlap_s(&a, &b)),
    )]))
}

/// `Chemistry.kinetic_s` — s-type kinetic (a|−½∇²|b). Args: `{ a, b }`. Out: `{ value }`.
pub fn kinetic_s(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = gto_at(args_v, "a", span)?;
    let b = gto_at(args_v, "b", span)?;
    Ok(args::record([(
        "value",
        Value::F64(IntegralEngine::kinetic_s(&a, &b)),
    )]))
}

/// `Chemistry.nuclear_s` — s-type nuclear attraction. Args: `{ a, b, center, z }`.
/// Out: `{ value }`.
pub fn nuclear_s(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = gto_at(args_v, "a", span)?;
    let b = gto_at(args_v, "b", span)?;
    let center = args::rec_f64_list(args_v, "center")
        .ok_or_else(|| args::bad(span, "Chemistry.nuclear_s needs center: [x,y,z]"))?;
    if center.len() != 3 {
        return Err(args::bad(span, "center must have length 3"));
    }
    let z = args::rec_f64(args_v, "z")
        .ok_or_else(|| args::bad(span, "Chemistry.nuclear_s needs z (nuclear charge)"))?;
    Ok(args::record([(
        "value",
        Value::F64(IntegralEngine::nuclear_s(
            &a,
            &b,
            [center[0], center[1], center[2]],
            z,
        )),
    )]))
}

/// `Chemistry.dipole_s` — s-type dipole moments. Args: `{ a, b }`. Out: `{ dipole }`.
pub fn dipole_s(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = gto_at(args_v, "a", span)?;
    let b = gto_at(args_v, "b", span)?;
    let d = IntegralEngine::dipole_s(&a, &b);
    Ok(args::record([("dipole", args::f64_list_value(d))]))
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
    fn wave12_boys_f0_at_zero() {
        let mut m = BTreeMap::new();
        m.insert("n".into(), Value::U64(0));
        m.insert("t".into(), Value::F64(0.0));
        let out = boys_function(&Value::Record(m), span()).unwrap();
        let v = args::rec_f64(&out, "value").unwrap();
        assert!((v - 1.0).abs() < 1e-12, "F0(0)=1 got {v}");
    }

    #[test]
    fn wave12_overlap_s_identical_centers() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), s_gto([0.0, 0.0, 0.0], 1.0));
        m.insert("b".into(), s_gto([0.0, 0.0, 0.0], 1.0));
        let out = overlap_s(&Value::Record(m), span()).unwrap();
        let v = args::rec_f64(&out, "value").unwrap();
        assert!(v > 0.0, "overlap should be positive, got {v}");
    }

    #[test]
    fn wave12_kinetic_and_nuclear_and_dipole() {
        let a = s_gto([0.0, 0.0, 0.0], 1.0);
        let b = s_gto([0.0, 0.0, 0.0], 1.0);
        let mut kin = BTreeMap::new();
        kin.insert("a".into(), a.clone());
        kin.insert("b".into(), b.clone());
        let k = args::rec_f64(
            &kinetic_s(&Value::Record(kin), span()).unwrap(),
            "value",
        )
        .unwrap();
        assert!(k > 0.0, "kinetic {k}");

        let mut nuc = BTreeMap::new();
        nuc.insert("a".into(), a.clone());
        nuc.insert("b".into(), b.clone());
        nuc.insert("center".into(), args::f64_list_value([0.0, 0.0, 0.0]));
        nuc.insert("z".into(), Value::F64(1.0));
        let n = args::rec_f64(&nuclear_s(&Value::Record(nuc), span()).unwrap(), "value").unwrap();
        assert!(n < 0.0, "nuclear attraction should be negative, got {n}");

        let mut dip = BTreeMap::new();
        dip.insert("a".into(), a);
        dip.insert("b".into(), b);
        let out = dipole_s(&Value::Record(dip), span()).unwrap();
        let d = args::rec_f64_list(&out, "dipole").unwrap();
        assert_eq!(d.len(), 3);
        assert!(d.iter().all(|x| x.abs() < 1e-12), "on-origin dipole ~0: {d:?}");
    }
}
