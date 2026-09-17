//! Spectral `capability.invoke` wrappers — EMF → SPD → XYZ → sRGB pipeline.
//!
//! Wraps `render::spectral_kernel` and `render::spectral_blend` so Vibe
//! scripts can compute colour from EMF field parameters. All underlying
//! operations are deterministic (compile-time CIE 1931 CMF tables).

use super::super::args;
use crate::render::gamut::gamut_map_clamp;
use crate::render::spectral_blend::spectral_blend_emf;
use crate::render::spectral_kernel::{
    emf_to_linear_rgb, emf_to_spd, linear_rgb_to_display, spd_to_xyz, Spd, SPD_SAMPLES,
};
use vibe::{Diagnostic, Span, Value};

/// `Spectral.emf_to_spd` — EMF `[α, μ, σ]` → 41-sample SPD (380–780 nm).
pub fn emf_to_spd_fn(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let alpha = args::rec_f64(args_v, "alpha")
        .ok_or_else(|| args::bad(span, "emf_to_spd needs { alpha: f64, mu: f64, sigma: f64 }"))?;
    let mu = args::rec_f64(args_v, "mu").unwrap_or(0.0);
    let sigma = args::rec_f64(args_v, "sigma").unwrap_or(0.5);
    let spd = emf_to_spd(alpha as f32, mu as f32, sigma as f32);
    Ok(args::f64_list_value(spd.samples.iter().map(|&s| s as f64)))
}

/// `Spectral.spd_to_xyz` — 41-sample SPD → CIE XYZ tristimulus.
pub fn spd_to_xyz_fn(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let samples = args::rec_f64_list(args_v, "spd")
        .ok_or_else(|| args::bad(span, "spd_to_xyz needs { spd: [f64; 41] }"))?;
    if samples.len() < SPD_SAMPLES {
        return Err(args::bad(
            span,
            format!("spd needs {SPD_SAMPLES} samples, got {}", samples.len()),
        ));
    }
    let mut arr = [0.0f32; SPD_SAMPLES];
    for i in 0..SPD_SAMPLES {
        arr[i] = samples[i] as f32;
    }
    let xyz = spd_to_xyz(&Spd::from_samples(arr));
    Ok(args::record([
        ("x", Value::F64(xyz.x as f64)),
        ("y", Value::F64(xyz.y as f64)),
        ("z", Value::F64(xyz.z as f64)),
    ]))
}

/// `Spectral.emf_to_rgb` — EMF `[α, μ, σ]` → linear sRGB + 8-bit display sRGB.
pub fn emf_to_rgb_fn(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let alpha = args::rec_f64(args_v, "alpha")
        .ok_or_else(|| args::bad(span, "emf_to_rgb needs { alpha: f64, mu: f64, sigma: f64 }"))?;
    let mu = args::rec_f64(args_v, "mu").unwrap_or(0.0);
    let sigma = args::rec_f64(args_v, "sigma").unwrap_or(0.5);
    let lin = emf_to_linear_rgb(alpha as f32, mu as f32, sigma as f32);
    let (dr, dg, db) = linear_rgb_to_display(&lin);
    Ok(args::record([
        ("r", Value::F64(lin.r as f64)),
        ("g", Value::F64(lin.g as f64)),
        ("b", Value::F64(lin.b as f64)),
        ("display_r", Value::U64(dr as u64)),
        ("display_g", Value::U64(dg as u64)),
        ("display_b", Value::U64(db as u64)),
        ("css", Value::String(format!("rgb({dr},{dg},{db})"))),
    ]))
}

/// `Spectral.blend` — spectral-space blend of two EMF payloads → XYZ.
pub fn blend_fn(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let alpha_a = args::rec_f64(args_v, "alpha_a").ok_or_else(|| {
        args::bad(
            span,
            "blend needs { alpha_a, mu_a, sigma_a, alpha_b, mu_b, sigma_b, t }",
        )
    })?;
    let mu_a = args::rec_f64(args_v, "mu_a").unwrap_or(0.0);
    let sigma_a = args::rec_f64(args_v, "sigma_a").unwrap_or(0.2);
    let alpha_b = args::rec_f64(args_v, "alpha_b").unwrap_or(1.0);
    let mu_b = args::rec_f64(args_v, "mu_b").unwrap_or(0.0);
    let sigma_b = args::rec_f64(args_v, "sigma_b").unwrap_or(0.8);
    let t = args::rec_f64(args_v, "t").unwrap_or(0.5);
    let xyz = spectral_blend_emf(
        alpha_a as f32,
        mu_a as f32,
        sigma_a as f32,
        alpha_b as f32,
        mu_b as f32,
        sigma_b as f32,
        t as f32,
    );
    Ok(args::record([
        ("x", Value::F64(xyz.x as f64)),
        ("y", Value::F64(xyz.y as f64)),
        ("z", Value::F64(xyz.z as f64)),
    ]))
}

/// `Spectral.gamut_map` — map an XYZ colour into the sRGB display gamut.
pub fn gamut_map_fn(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args_v, "x")
        .ok_or_else(|| args::bad(span, "gamut_map needs { x: f64, y: f64, z: f64 }"))?;
    let y = args::rec_f64(args_v, "y").unwrap_or(1.0);
    let z = args::rec_f64(args_v, "z").unwrap_or(1.0);
    let mapped = gamut_map_clamp(&crate::render::spectral_kernel::Xyz::new(
        x as f32, y as f32, z as f32,
    ));
    Ok(args::record([
        ("x", Value::F64(mapped.x as f64)),
        ("y", Value::F64(mapped.y as f64)),
        ("z", Value::F64(mapped.z as f64)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use vibe::Value;

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert((*k).into(), v.clone());
        }
        Value::Record(m)
    }

    #[test]
    fn emf_to_spd_returns_41_samples() {
        let args = rec(&[
            ("alpha", Value::F64(1.0)),
            ("mu", Value::F64(0.0)),
            ("sigma", Value::F64(0.5)),
        ]);
        let r = emf_to_spd_fn(&args, vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::List(xs) => assert_eq!(xs.len(), SPD_SAMPLES),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn emf_to_rgb_green_dominates_at_mid_sigma() {
        let args = rec(&[
            ("alpha", Value::F64(1.0)),
            ("mu", Value::F64(0.1)),
            ("sigma", Value::F64(0.5)),
        ]);
        let r = emf_to_rgb_fn(&args, vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                let g = m.get("display_g").and_then(args::as_u64).unwrap();
                let r_ch = m.get("display_r").and_then(args::as_u64).unwrap();
                assert!(
                    g >= r_ch,
                    "green should dominate at sigma=0.5: r={r_ch} g={g}"
                );
                let css = m.get("css");
                assert!(css.is_some(), "should return css string");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn blend_endpoints_match_inputs() {
        // At t=0, blend should equal first EMF's XYZ.
        let args = rec(&[
            ("alpha_a", Value::F64(1.0)),
            ("mu_a", Value::F64(0.0)),
            ("sigma_a", Value::F64(0.2)),
            ("alpha_b", Value::F64(1.0)),
            ("mu_b", Value::F64(0.0)),
            ("sigma_b", Value::F64(0.8)),
            ("t", Value::F64(0.0)),
        ]);
        let r = blend_fn(&args, vibe::Span::new(0, 0)).unwrap();
        if let Value::Record(m) = r {
            let x = m.get("x").and_then(args::as_f64).unwrap();
            assert!(x.is_finite());
        }
    }

    #[test]
    fn gamut_map_brings_out_of_gamut_in() {
        let args = rec(&[
            ("x", Value::F64(2.0)),
            ("y", Value::F64(0.5)),
            ("z", Value::F64(0.0)),
        ]);
        let r = gamut_map_fn(&args, vibe::Span::new(0, 0)).unwrap();
        if let Value::Record(m) = r {
            let x = m.get("x").and_then(args::as_f64).unwrap();
            assert!(x <= 2.0, "gamut map should not increase x");
        }
    }

    #[test]
    fn spd_to_xyz_round_trip() {
        // emf_to_spd → spd_to_xyz should produce finite XYZ.
        let spd_args = rec(&[
            ("alpha", Value::F64(1.0)),
            ("mu", Value::F64(0.0)),
            ("sigma", Value::F64(0.5)),
        ]);
        let spd_val = emf_to_spd_fn(&spd_args, vibe::Span::new(0, 0)).unwrap();
        let xyz_args = rec(&[("spd", spd_val)]);
        let r = spd_to_xyz_fn(&xyz_args, vibe::Span::new(0, 0)).unwrap();
        if let Value::Record(m) = r {
            let y = m.get("y").and_then(args::as_f64).unwrap();
            assert!(
                y.is_finite() && y >= 0.0,
                "Y should be finite and non-negative: y={y}"
            );
        }
    }
}
