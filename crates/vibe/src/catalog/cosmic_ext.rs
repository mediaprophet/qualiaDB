//! Remaining catalogued Cosmic.* kernels (G-COORD LocalHost slice).
//! UTF-8 names; no ASCII-only folding. No new Host methods.

use crate::cosmic;
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

use super::cosmic::{bad, rec_f64, rec_str, record, utf8_fold};

pub fn invoke(id: &str, args: &Value, span: Span) -> Result<Value, Diagnostic> {
    match id {
        "Cosmic.ecef_to_enu" => ecef_to_enu(args, span),
        "Cosmic.enu_to_ecef" => enu_to_ecef(args, span),
        "Cosmic.flrw_hubble_velocity" => flrw_hubble(args, span),
        "Cosmic.warp_velocity" => warp_velocity(args, span),
        "Cosmic.cochrane_units" => cochrane_units(args, span),
        "Cosmic.atmosphere_pressure" => atmosphere_pressure(args, span),
        "Cosmic.atmosphere_temperature" => atmosphere_temperature(args, span),
        "Cosmic.magnetosphere_field" => magnetosphere_field(args, span),
        "Cosmic.scale_factor" => scale_factor(args, span),
        "Cosmic.compton_wavelength" => compton_wavelength(args, span),
        "Cosmic.de_broglie_wavelength" => de_broglie_wavelength(args, span),
        "Cosmic.usri_parse" => usri_parse(args, span),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{id} is not in the Cosmic LocalHost slice"),
        )),
    }
}

fn warp_scale(s: &str) -> cosmic::warp::WarpScale {
    match utf8_fold(s).as_str() {
        "tos" => cosmic::warp::WarpScale::Tos,
        _ => cosmic::warp::WarpScale::Tng,
    }
}

fn atm_profile(body: &str) -> cosmic::atmosphere::AtmosphericProfile {
    match utf8_fold(body).as_str() {
        "mars" | "火星" => cosmic::atmosphere::AtmosphericProfile::mars(),
        "venus" | "金星" => cosmic::atmosphere::AtmosphericProfile::venus(),
        _ => cosmic::atmosphere::AtmosphericProfile::earth(),
    }
}

fn mag_profile(body: &str) -> cosmic::atmosphere::MagnetosphereProfile {
    match utf8_fold(body).as_str() {
        "jupiter" | "木星" => cosmic::atmosphere::MagnetosphereProfile::jupiter(),
        _ => cosmic::atmosphere::MagnetosphereProfile::earth(),
    }
}

fn particle(name: &str) -> cosmic::microverse::ParticleProfile {
    match utf8_fold(name).as_str() {
        "proton" | "质子" => cosmic::microverse::ParticleProfile::proton(),
        _ => cosmic::microverse::ParticleProfile::electron(),
    }
}

fn ecef_to_enu(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = rec_f64(args, "x").ok_or_else(|| {
        bad(
            span,
            "ecef_to_enu needs { x, y, z, ref_lat_deg, ref_lon_deg, ref_alt_m }",
        )
    })?;
    let y = rec_f64(args, "y").unwrap_or(0.0);
    let z = rec_f64(args, "z").unwrap_or(0.0);
    let ref_lat = rec_f64(args, "ref_lat_deg").unwrap_or(0.0);
    let ref_lon = rec_f64(args, "ref_lon_deg").unwrap_or(0.0);
    let ref_alt = rec_f64(args, "ref_alt_m").unwrap_or(0.0);
    let enu = cosmic::transforms::ecef_to_enu(
        cosmic::transforms::Ecef { x, y, z },
        cosmic::transforms::Geodetic {
            lat_deg: ref_lat,
            lon_deg: ref_lon,
            alt_m: ref_alt,
        },
    );
    Ok(record(&[
        ("east", Value::F64(enu.east)),
        ("north", Value::F64(enu.north)),
        ("up", Value::F64(enu.up)),
        ("realm", Value::String("earth".into())),
    ]))
}

fn enu_to_ecef(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let east = rec_f64(args, "east").ok_or_else(|| {
        bad(
            span,
            "enu_to_ecef needs { east, north, up, ref_lat_deg, ref_lon_deg, ref_alt_m }",
        )
    })?;
    let north = rec_f64(args, "north").unwrap_or(0.0);
    let up = rec_f64(args, "up").unwrap_or(0.0);
    let ref_lat = rec_f64(args, "ref_lat_deg").unwrap_or(0.0);
    let ref_lon = rec_f64(args, "ref_lon_deg").unwrap_or(0.0);
    let ref_alt = rec_f64(args, "ref_alt_m").unwrap_or(0.0);
    let ecef = cosmic::transforms::enu_to_ecef(
        cosmic::transforms::Enu { east, north, up },
        cosmic::transforms::Geodetic {
            lat_deg: ref_lat,
            lon_deg: ref_lon,
            alt_m: ref_alt,
        },
    );
    Ok(record(&[
        ("x", Value::F64(ecef.x)),
        ("y", Value::F64(ecef.y)),
        ("z", Value::F64(ecef.z)),
        ("realm", Value::String("earth".into())),
    ]))
}

fn flrw_hubble(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let d = rec_f64(args, "distance_m")
        .ok_or_else(|| bad(span, "flrw_hubble_velocity needs { distance_m: f64 }"))?;
    let metric = cosmic::flrw::FlrwMetric::flat_present_epoch();
    Ok(Value::F64(metric.hubble_velocity(d)))
}

fn warp_velocity(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let w = rec_f64(args, "warp")
        .ok_or_else(|| bad(span, "warp_velocity needs { warp: f64, scale: string }"))?;
    let scale = warp_scale(rec_str(args, "scale").unwrap_or("tng"));
    Ok(record(&[
        (
            "metres_per_second",
            Value::F64(cosmic::warp::warp_velocity(w, scale)),
        ),
        ("realm", Value::String("fictional".into())),
        ("system", Value::String("warp".into())),
    ]))
}

fn cochrane_units(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let w = rec_f64(args, "warp")
        .ok_or_else(|| bad(span, "cochrane_units needs { warp: f64, scale: string }"))?;
    let scale = warp_scale(rec_str(args, "scale").unwrap_or("tng"));
    Ok(Value::F64(cosmic::warp::cochrane_units(w, scale)))
}

fn atmosphere_pressure(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let body = rec_str(args, "body").unwrap_or("earth");
    let alt = rec_f64(args, "altitude_m").ok_or_else(|| {
        bad(
            span,
            "atmosphere_pressure needs { body: string, altitude_m: f64 }",
        )
    })?;
    Ok(Value::F64(atm_profile(body).pressure_at_altitude(alt)))
}

fn atmosphere_temperature(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let body = rec_str(args, "body").unwrap_or("earth");
    let alt = rec_f64(args, "altitude_m").ok_or_else(|| {
        bad(
            span,
            "atmosphere_temperature needs { body: string, altitude_m: f64 }",
        )
    })?;
    Ok(Value::F64(atm_profile(body).temperature_at_altitude(alt)))
}

fn magnetosphere_field(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let body = rec_str(args, "body").unwrap_or("earth");
    let r = rec_f64(args, "distance_m").ok_or_else(|| {
        bad(
            span,
            "magnetosphere_field needs { body: string, distance_m: f64, body_radius_m: f64 }",
        )
    })?;
    let body_r = rec_f64(args, "body_radius_m").unwrap_or(6_371_000.0);
    Ok(Value::F64(mag_profile(body).field_at_distance(r, body_r)))
}

fn scale_factor(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let from_str = rec_str(args, "from_level").ok_or_else(|| {
        bad(
            span,
            "scale_factor needs { from_level: string, to_level: string }",
        )
    })?;
    let to_str = rec_str(args, "to_level").ok_or_else(|| {
        bad(
            span,
            "scale_factor needs { from_level: string, to_level: string }",
        )
    })?;
    let from = parse_hierarchy_level(from_str, span)?;
    let to = parse_hierarchy_level(to_str, span)?;
    Ok(Value::F64(cosmic::microverse::scale_factor_between(
        from, to,
    )))
}

fn compton_wavelength(args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let name = rec_str(args, "particle").unwrap_or("electron");
    Ok(Value::F64(particle(name).compton_wavelength()))
}

fn de_broglie_wavelength(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let name = rec_str(args, "particle").unwrap_or("electron");
    let v = rec_f64(args, "velocity_m_s").ok_or_else(|| {
        bad(
            span,
            "de_broglie_wavelength needs { particle: string, velocity_m_s: f64 }",
        )
    })?;
    Ok(Value::F64(particle(name).de_broglie_wavelength(v)))
}

fn usri_parse(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let uri = rec_str(args, "uri").ok_or_else(|| bad(span, "usri_parse needs { uri: string }"))?;
    let usri = cosmic::usri::Usri::parse(uri).map_err(|e| bad(span, e))?;
    Ok(usri.to_value())
}

fn parse_hierarchy_level(
    s: &str,
    span: Span,
) -> Result<cosmic::cb_usri::HierarchyLevel, Diagnostic> {
    use cosmic::cb_usri::HierarchyLevel;
    Ok(match utf8_fold(s).as_str() {
        "l-2" => HierarchyLevel::LNeg2,
        "l-1" => HierarchyLevel::LNeg1,
        "l0" => HierarchyLevel::L0,
        "l1" => HierarchyLevel::L1,
        "l2" => HierarchyLevel::L2,
        "l3" => HierarchyLevel::L3,
        "l4" => HierarchyLevel::L4,
        "l5" => HierarchyLevel::L5,
        "l6" => HierarchyLevel::L6,
        "l7" => HierarchyLevel::L7,
        "l8" => HierarchyLevel::L8,
        "l9" => HierarchyLevel::L9,
        "l10" => HierarchyLevel::L10,
        "l11" => HierarchyLevel::L11,
        "l12" => HierarchyLevel::L12,
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("unknown hierarchy level: {s}"),
            ))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert((*k).into(), v.clone());
        }
        Value::Record(m)
    }

    #[test]
    fn warp_and_mars_atmosphere_are_live() {
        let w = invoke(
            "Cosmic.warp_velocity",
            &rec(&[
                ("warp", Value::F64(5.0)),
                ("scale", Value::String("tng".into())),
            ]),
            Span::point(0),
        )
        .unwrap();
        match w {
            Value::Record(map) => {
                assert_eq!(map.get("realm"), Some(&Value::String("fictional".into())));
            }
            other => panic!("{other:?}"),
        }
        let p = invoke(
            "Cosmic.atmosphere_pressure",
            &rec(&[
                ("body", Value::String("火星".into())),
                ("altitude_m", Value::F64(0.0)),
            ]),
            Span::point(0),
        )
        .unwrap();
        assert!(matches!(p, Value::F64(_)));
    }
}
