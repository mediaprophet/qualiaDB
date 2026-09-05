//! In-process Cosmic.* kernels (G-COORD remap). Same math as poet_host cosmic_bind.
//! No new Host methods. Unknown Cosmic ids fail closed.

use std::collections::BTreeMap;

use crate::cosmic;
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

pub fn invoke(id: &str, args: &Value, span: Span) -> Result<Value, Diagnostic> {
    match id {
        "Cosmic.geodetic_to_ecef" => geodetic_to_ecef(args, span),
        "Cosmic.ecef_to_geodetic" => ecef_to_geodetic(args, span),
        "Cosmic.geodetic_distance" => geodetic_distance(args, span),
        "Cosmic.body_profile" => body_profile(args, span),
        "Cosmic.surface_gravity" => surface_gravity(args, span),
        "Cosmic.flrw_distance" => flrw_distance(args, span),
        "Cosmic.flrw_redshift" => flrw_redshift(args, span),
        "Cosmic.stardate_to_gregorian" => stardate_to_gregorian(args, span),
        other if other.starts_with("Cosmic.") => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!(
                "{other} is catalogued; this LocalHost slice implements the G-COORD remap set only"
            ),
        )),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("not a Cosmic capability: {id}"),
        )),
    }
}

fn bad(span: Span, msg: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagCode::E100, span, msg)
}

fn rec_f64(args: &Value, key: &str) -> Option<f64> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::F64(n)) => Some(*n),
            Some(Value::I64(n)) => Some(*n as f64),
            Some(Value::U64(n)) => Some(*n as f64),
            Some(Value::Quantity(q)) => Some(q.value),
            _ => None,
        },
        _ => None,
    }
}

/// Unicode case-fold for catalog keys. Do not use ASCII-only lowering —
/// labels and names are UTF-8 and may be any language.
fn utf8_fold(s: &str) -> String {
    s.trim().to_lowercase()
}

fn rec_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::String(s)) => Some(s.as_str()),
            _ => None,
        },
        _ => None,
    }
}

fn record(pairs: &[(&str, Value)]) -> Value {
    let mut map = BTreeMap::new();
    for (k, v) in pairs {
        map.insert((*k).into(), v.clone());
    }
    Value::Record(map)
}

fn geodetic_to_ecef(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let lat = rec_f64(args, "lat_deg")
        .ok_or_else(|| bad(span, "geodetic_to_ecef needs { lat_deg, lon_deg, alt_m }"))?;
    let lon = rec_f64(args, "lon_deg").unwrap_or(0.0);
    let alt = rec_f64(args, "alt_m").unwrap_or(0.0);
    let ecef = cosmic::transforms::geodetic_to_ecef(cosmic::transforms::Geodetic {
        lat_deg: lat,
        lon_deg: lon,
        alt_m: alt,
    });
    Ok(record(&[
        ("x", Value::F64(ecef.x)),
        ("y", Value::F64(ecef.y)),
        ("z", Value::F64(ecef.z)),
        ("realm", Value::String("earth".into())),
        ("system", Value::String("wgs84".into())),
    ]))
}

fn ecef_to_geodetic(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = rec_f64(args, "x").ok_or_else(|| bad(span, "ecef_to_geodetic needs { x, y, z }"))?;
    let y = rec_f64(args, "y").unwrap_or(0.0);
    let z = rec_f64(args, "z").unwrap_or(0.0);
    let g = cosmic::transforms::ecef_to_geodetic(cosmic::transforms::Ecef { x, y, z });
    Ok(record(&[
        ("lat_deg", Value::F64(g.lat_deg)),
        ("lon_deg", Value::F64(g.lon_deg)),
        ("alt_m", Value::F64(g.alt_m)),
        ("realm", Value::String("earth".into())),
        ("system", Value::String("wgs84".into())),
    ]))
}

fn geodetic_distance(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let lat1 = rec_f64(args, "lat_deg").ok_or_else(|| {
        bad(
            span,
            "geodetic_distance needs { lat_deg, lon_deg, lat2_deg, lon2_deg }",
        )
    })?;
    let lon1 = rec_f64(args, "lon_deg").unwrap_or(0.0);
    let lat2 = rec_f64(args, "lat2_deg").unwrap_or(0.0);
    let lon2 = rec_f64(args, "lon2_deg").unwrap_or(0.0);
    let d = cosmic::transforms::geodetic_distance(
        cosmic::transforms::Geodetic {
            lat_deg: lat1,
            lon_deg: lon1,
            alt_m: 0.0,
        },
        cosmic::transforms::Geodetic {
            lat_deg: lat2,
            lon_deg: lon2,
            alt_m: 0.0,
        },
    );
    Ok(Value::F64(d))
}

fn body_profile(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let name =
        rec_str(args, "name").ok_or_else(|| bad(span, "body_profile needs { name: string }"))?;
    let body = cosmic::celestial::body_profile_by_name(&utf8_fold(name))
        .ok_or_else(|| bad(span, format!("unknown celestial body: {name}")))?;
    Ok(record(&[
        ("name", Value::String(body.name.clone())),
        ("class", Value::String(body.class.as_str().into())),
        ("equatorial_radius_m", Value::F64(body.equatorial_radius_m)),
        ("mass_kg", Value::F64(body.mass_kg)),
        ("realm", Value::String("cosmos".into())),
        ("system", Value::String("ocs".into())),
    ]))
}

fn surface_gravity(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let name =
        rec_str(args, "name").ok_or_else(|| bad(span, "surface_gravity needs { name: string }"))?;
    let body = cosmic::celestial::body_profile_by_name(&utf8_fold(name))
        .ok_or_else(|| bad(span, format!("unknown celestial body: {name}")))?;
    Ok(Value::F64(body.surface_gravity()))
}

fn flrw_distance(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let z = rec_f64(args, "z").ok_or_else(|| bad(span, "flrw_distance needs { z: f64 }"))?;
    let metric = cosmic::flrw::FlrwMetric::flat_present_epoch();
    Ok(record(&[
        ("metres", Value::F64(metric.redshift_to_distance(z))),
        ("z", Value::F64(z)),
        ("realm", Value::String("cosmos".into())),
        ("system", Value::String("flrw".into())),
    ]))
}

fn flrw_redshift(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a_emit =
        rec_f64(args, "a_emit").ok_or_else(|| bad(span, "flrw_redshift needs { a_emit: f64 }"))?;
    let metric = cosmic::flrw::FlrwMetric::flat_present_epoch();
    Ok(Value::F64(metric.redshift(a_emit)))
}

fn stardate_to_gregorian(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let s = rec_f64(args, "stardate")
        .ok_or_else(|| bad(span, "stardate_to_gregorian needs { stardate: f64 }"))?;
    let sd = cosmic::stardate::Stardate::new(s);
    Ok(record(&[
        ("year", Value::F64(sd.to_gregorian_year())),
        ("stardate", Value::F64(s)),
        ("realm", Value::String("fictional".into())),
        ("system", Value::String("stardate".into())),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(pairs: &[(&str, Value)]) -> Value {
        record(pairs)
    }

    #[test]
    fn body_name_folds_unicode_not_ascii_only() {
        let out = invoke(
            "Cosmic.body_profile",
            &rec(&[("name", Value::String("Earth".into()))]),
            Span::point(0),
        )
        .expect("Earth");
        match out {
            Value::Record(map) => {
                assert_eq!(map.get("name"), Some(&Value::String("Earth".into())));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn earth_geodetic_is_wgs84_not_dns() {
        let out = invoke(
            "Cosmic.geodetic_to_ecef",
            &rec(&[
                ("lat_deg", Value::F64(-37.8)),
                ("lon_deg", Value::F64(144.9)),
                ("alt_m", Value::F64(0.0)),
            ]),
            Span::point(0),
        )
        .expect("earth remap");
        match out {
            Value::Record(map) => {
                assert_eq!(map.get("system"), Some(&Value::String("wgs84".into())));
                assert_eq!(map.get("realm"), Some(&Value::String("earth".into())));
                assert!(matches!(map.get("x"), Some(Value::F64(_))));
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn cosmos_body_and_fiction_stardate() {
        let body = invoke(
            "Cosmic.body_profile",
            &rec(&[("name", Value::String("earth".into()))]),
            Span::point(0),
        )
        .unwrap();
        match body {
            Value::Record(map) => {
                assert_eq!(map.get("realm"), Some(&Value::String("cosmos".into())));
            }
            other => panic!("{other:?}"),
        }
        let year = invoke(
            "Cosmic.stardate_to_gregorian",
            &rec(&[("stardate", Value::F64(41000.0))]),
            Span::point(0),
        )
        .unwrap();
        match year {
            Value::Record(map) => {
                assert_eq!(map.get("realm"), Some(&Value::String("fictional".into())));
                if let Some(Value::F64(y)) = map.get("year") {
                    assert!((y - 2364.0).abs() < 0.01);
                } else {
                    panic!("year");
                }
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn unimplemented_cosmic_id_fails_closed() {
        let err = invoke("Cosmic.warp_velocity", &Value::Null, Span::point(0)).unwrap_err();
        assert_eq!(err.code, DiagCode::E100);
    }
}
