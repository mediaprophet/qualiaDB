//! Cosmic coordinate system (OCS) capability.invoke bindings.
//!
//! Exposes the `poet_vibe::cosmic` library to VibeScript via stable
//! capability.invoke IDs. Each function takes a `Value` record (or list)
//! and returns a `Value` record (or scalar).

#![allow(dead_code)]

use poet_vibe::cosmic;
use poet_vibe::{DiagCode, Diagnostic, Span, Value};

use super::args;

// ── Coordinate transforms ──────────────────────────────────────────────────

/// `Cosmic.geodetic_to_ecef` — { lat_deg, lon_deg, alt_m } → { x, y, z }
pub fn geodetic_to_ecef(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let lat = args::rec_f64(args_v, "lat_deg")
        .ok_or_else(|| args::bad(span, "geodetic_to_ecef needs { lat_deg, lon_deg, alt_m }"))?;
    let lon = args::rec_f64(args_v, "lon_deg").unwrap_or(0.0);
    let alt = args::rec_f64(args_v, "alt_m").unwrap_or(0.0);
    let ecef = cosmic::transforms::geodetic_to_ecef(cosmic::transforms::Geodetic {
        lat_deg: lat,
        lon_deg: lon,
        alt_m: alt,
    });
    Ok(args::record([
        ("x", Value::F64(ecef.x)),
        ("y", Value::F64(ecef.y)),
        ("z", Value::F64(ecef.z)),
    ]))
}

/// `Cosmic.ecef_to_geodetic` — { x, y, z } → { lat_deg, lon_deg, alt_m }
pub fn ecef_to_geodetic(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args_v, "x")
        .ok_or_else(|| args::bad(span, "ecef_to_geodetic needs { x, y, z }"))?;
    let y = args::rec_f64(args_v, "y").unwrap_or(0.0);
    let z = args::rec_f64(args_v, "z").unwrap_or(0.0);
    let g = cosmic::transforms::ecef_to_geodetic(cosmic::transforms::Ecef { x, y, z });
    Ok(args::record([
        ("lat_deg", Value::F64(g.lat_deg)),
        ("lon_deg", Value::F64(g.lon_deg)),
        ("alt_m", Value::F64(g.alt_m)),
    ]))
}

/// `Cosmic.ecef_to_enu` — { x, y, z, ref_lat_deg, ref_lon_deg, ref_alt_m } → { east, north, up }
pub fn ecef_to_enu(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args_v, "x").ok_or_else(|| {
        args::bad(
            span,
            "ecef_to_enu needs { x, y, z, ref_lat_deg, ref_lon_deg, ref_alt_m }",
        )
    })?;
    let y = args::rec_f64(args_v, "y").unwrap_or(0.0);
    let z = args::rec_f64(args_v, "z").unwrap_or(0.0);
    let ref_lat = args::rec_f64(args_v, "ref_lat_deg").unwrap_or(0.0);
    let ref_lon = args::rec_f64(args_v, "ref_lon_deg").unwrap_or(0.0);
    let ref_alt = args::rec_f64(args_v, "ref_alt_m").unwrap_or(0.0);
    let enu = cosmic::transforms::ecef_to_enu(
        cosmic::transforms::Ecef { x, y, z },
        cosmic::transforms::Geodetic {
            lat_deg: ref_lat,
            lon_deg: ref_lon,
            alt_m: ref_alt,
        },
    );
    Ok(args::record([
        ("east", Value::F64(enu.east)),
        ("north", Value::F64(enu.north)),
        ("up", Value::F64(enu.up)),
    ]))
}

/// `Cosmic.enu_to_ecef` — { east, north, up, ref_lat_deg, ref_lon_deg, ref_alt_m } → { x, y, z }
pub fn enu_to_ecef(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let east = args::rec_f64(args_v, "east").ok_or_else(|| {
        args::bad(
            span,
            "enu_to_ecef needs { east, north, up, ref_lat_deg, ref_lon_deg, ref_alt_m }",
        )
    })?;
    let north = args::rec_f64(args_v, "north").unwrap_or(0.0);
    let up = args::rec_f64(args_v, "up").unwrap_or(0.0);
    let ref_lat = args::rec_f64(args_v, "ref_lat_deg").unwrap_or(0.0);
    let ref_lon = args::rec_f64(args_v, "ref_lon_deg").unwrap_or(0.0);
    let ref_alt = args::rec_f64(args_v, "ref_alt_m").unwrap_or(0.0);
    let ecef = cosmic::transforms::enu_to_ecef(
        cosmic::transforms::Enu { east, north, up },
        cosmic::transforms::Geodetic {
            lat_deg: ref_lat,
            lon_deg: ref_lon,
            alt_m: ref_alt,
        },
    );
    Ok(args::record([
        ("x", Value::F64(ecef.x)),
        ("y", Value::F64(ecef.y)),
        ("z", Value::F64(ecef.z)),
    ]))
}

/// `Cosmic.geodetic_distance` — { lat_deg, lon_deg, lat2_deg, lon2_deg } → meters
pub fn geodetic_distance(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let lat1 = args::rec_f64(args_v, "lat_deg").ok_or_else(|| {
        args::bad(
            span,
            "geodetic_distance needs { lat_deg, lon_deg, lat2_deg, lon2_deg }",
        )
    })?;
    let lon1 = args::rec_f64(args_v, "lon_deg").unwrap_or(0.0);
    let lat2 = args::rec_f64(args_v, "lat2_deg").unwrap_or(0.0);
    let lon2 = args::rec_f64(args_v, "lon2_deg").unwrap_or(0.0);
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

// ── Celestial body profiles ────────────────────────────────────────────────

/// `Cosmic.body_profile` — { name: string } → { name, class, radius_m, mass_kg, ... }
pub fn body_profile(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let name = args::rec_str(args_v, "name")
        .ok_or_else(|| args::bad(span, "body_profile needs { name: string }"))?;
    let body = cosmic::celestial::body_profile_by_name(name)
        .ok_or_else(|| args::bad(span, format!("unknown celestial body: {name}")))?;
    Ok(args::record([
        ("name", Value::String(body.name.clone())),
        ("class", Value::String(body.class.as_str().into())),
        ("equatorial_radius_m", Value::F64(body.equatorial_radius_m)),
        ("mass_kg", Value::F64(body.mass_kg)),
        ("rotation_period_s", Value::F64(body.rotation_period_s)),
    ]))
}

/// `Cosmic.surface_gravity` — { name: string } → m/s²
pub fn surface_gravity(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let name = args::rec_str(args_v, "name")
        .ok_or_else(|| args::bad(span, "surface_gravity needs { name: string }"))?;
    let body = cosmic::celestial::body_profile_by_name(name)
        .ok_or_else(|| args::bad(span, format!("unknown celestial body: {name}")))?;
    Ok(Value::F64(body.surface_gravity()))
}

// ── Cosmological metric (FLRW) ─────────────────────────────────────────────

/// `Cosmic.flrw_distance` — { z: f64 } → comoving distance in meters
pub fn flrw_distance(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let z = args::rec_f64(args_v, "z")
        .ok_or_else(|| args::bad(span, "flrw_distance needs { z: f64 }"))?;
    let metric = cosmic::flrw::FlrwMetric::flat_present_epoch();
    let d = metric.redshift_to_distance(z);
    Ok(Value::F64(d))
}

/// `Cosmic.flrw_redshift` — { a_emit: f64 } → redshift z
pub fn flrw_redshift(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a_emit = args::rec_f64(args_v, "a_emit")
        .ok_or_else(|| args::bad(span, "flrw_redshift needs { a_emit: f64 }"))?;
    let metric = cosmic::flrw::FlrwMetric::flat_present_epoch();
    Ok(Value::F64(metric.redshift(a_emit)))
}

/// `Cosmic.flrw_hubble_velocity` — { distance_m: f64 } → velocity m/s
pub fn flrw_hubble_velocity(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let d = args::rec_f64(args_v, "distance_m")
        .ok_or_else(|| args::bad(span, "flrw_hubble_velocity needs { distance_m: f64 }"))?;
    let metric = cosmic::flrw::FlrwMetric::flat_present_epoch();
    Ok(Value::F64(metric.hubble_velocity(d)))
}

// ── Stardate / warp (fictional profile) ────────────────────────────────────

/// `Cosmic.stardate_to_gregorian` — { stardate: f64 } → gregorian year
pub fn stardate_to_gregorian(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let s = args::rec_f64(args_v, "stardate")
        .ok_or_else(|| args::bad(span, "stardate_to_gregorian needs { stardate: f64 }"))?;
    let sd = cosmic::stardate::Stardate::new(s);
    Ok(Value::F64(sd.to_gregorian_year()))
}

/// `Cosmic.warp_velocity` — { warp: f64, scale: string } → velocity m/s
pub fn warp_velocity(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let w = args::rec_f64(args_v, "warp")
        .ok_or_else(|| args::bad(span, "warp_velocity needs { warp: f64, scale: string }"))?;
    let scale_str = args::rec_str(args_v, "scale").unwrap_or("tng");
    let scale = match scale_str {
        "tos" => cosmic::warp::WarpScale::Tos,
        _ => cosmic::warp::WarpScale::Tng,
    };
    Ok(Value::F64(cosmic::warp::warp_velocity(w, scale)))
}

/// `Cosmic.cochrane_units` — { warp: f64, scale: string } → cochrane units
pub fn cochrane_units(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let w = args::rec_f64(args_v, "warp")
        .ok_or_else(|| args::bad(span, "cochrane_units needs { warp: f64, scale: string }"))?;
    let scale_str = args::rec_str(args_v, "scale").unwrap_or("tng");
    let scale = match scale_str {
        "tos" => cosmic::warp::WarpScale::Tos,
        _ => cosmic::warp::WarpScale::Tng,
    };
    Ok(Value::F64(cosmic::warp::cochrane_units(w, scale)))
}

// ── Atmospheric models ─────────────────────────────────────────────────────

/// `Cosmic.atmosphere_pressure` — { body: string, altitude_m: f64 } → pressure Pa
pub fn atmosphere_pressure(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let body = args::rec_str(args_v, "body").unwrap_or("earth");
    let alt = args::rec_f64(args_v, "altitude_m").ok_or_else(|| {
        args::bad(
            span,
            "atmosphere_pressure needs { body: string, altitude_m: f64 }",
        )
    })?;
    let profile = match body {
        "mars" => cosmic::atmosphere::AtmosphericProfile::mars(),
        "venus" => cosmic::atmosphere::AtmosphericProfile::venus(),
        _ => cosmic::atmosphere::AtmosphericProfile::earth(),
    };
    Ok(Value::F64(profile.pressure_at_altitude(alt)))
}

/// `Cosmic.atmosphere_temperature` — { body: string, altitude_m: f64 } → temperature K
pub fn atmosphere_temperature(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let body = args::rec_str(args_v, "body").unwrap_or("earth");
    let alt = args::rec_f64(args_v, "altitude_m").ok_or_else(|| {
        args::bad(
            span,
            "atmosphere_temperature needs { body: string, altitude_m: f64 }",
        )
    })?;
    let profile = match body {
        "mars" => cosmic::atmosphere::AtmosphericProfile::mars(),
        "venus" => cosmic::atmosphere::AtmosphericProfile::venus(),
        _ => cosmic::atmosphere::AtmosphericProfile::earth(),
    };
    Ok(Value::F64(profile.temperature_at_altitude(alt)))
}

/// `Cosmic.magnetosphere_field` — { body: string, distance_m: f64, body_radius_m: f64 } → field T
pub fn magnetosphere_field(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let body = args::rec_str(args_v, "body").unwrap_or("earth");
    let r = args::rec_f64(args_v, "distance_m").ok_or_else(|| {
        args::bad(
            span,
            "magnetosphere_field needs { body: string, distance_m: f64, body_radius_m: f64 }",
        )
    })?;
    let body_r = args::rec_f64(args_v, "body_radius_m").unwrap_or(6_371_000.0);
    let profile = match body {
        "jupiter" => cosmic::atmosphere::MagnetosphereProfile::jupiter(),
        _ => cosmic::atmosphere::MagnetosphereProfile::earth(),
    };
    Ok(Value::F64(profile.field_at_distance(r, body_r)))
}

// ── Microverse / quantum scale ─────────────────────────────────────────────

/// `Cosmic.scale_factor` — { from_level: string, to_level: string } → scale factor
pub fn scale_factor(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let from_str = args::rec_str(args_v, "from_level").ok_or_else(|| {
        args::bad(
            span,
            "scale_factor needs { from_level: string, to_level: string }",
        )
    })?;
    let to_str = args::rec_str(args_v, "to_level").ok_or_else(|| {
        args::bad(
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

/// `Cosmic.compton_wavelength` — { particle: string } → wavelength in meters
pub fn compton_wavelength(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let name = args::rec_str(args_v, "particle").unwrap_or("electron");
    let particle = match name {
        "proton" => cosmic::microverse::ParticleProfile::proton(),
        _ => cosmic::microverse::ParticleProfile::electron(),
    };
    Ok(Value::F64(particle.compton_wavelength()))
}

/// `Cosmic.de_broglie_wavelength` — { particle: string, velocity_m_s: f64 } → wavelength m
pub fn de_broglie_wavelength(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let name = args::rec_str(args_v, "particle").unwrap_or("electron");
    let v = args::rec_f64(args_v, "velocity_m_s").ok_or_else(|| {
        args::bad(
            span,
            "de_broglie_wavelength needs { particle: string, velocity_m_s: f64 }",
        )
    })?;
    let particle = match name {
        "proton" => cosmic::microverse::ParticleProfile::proton(),
        _ => cosmic::microverse::ParticleProfile::electron(),
    };
    Ok(Value::F64(particle.de_broglie_wavelength(v)))
}

// ── USRI parsing ───────────────────────────────────────────────────────────

/// `Cosmic.usri_parse` — { uri: string } → parsed USRI record
pub fn usri_parse(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let uri = args::rec_str(args_v, "uri")
        .ok_or_else(|| args::bad(span, "usri_parse needs { uri: string }"))?;
    let usri = cosmic::usri::Usri::parse(uri).map_err(|e| args::bad(span, e))?;
    Ok(usri.to_value())
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn parse_hierarchy_level(
    s: &str,
    span: Span,
) -> Result<cosmic::cb_usri::HierarchyLevel, Diagnostic> {
    use cosmic::cb_usri::HierarchyLevel;
    Ok(match s {
        "L-2" | "l-2" => HierarchyLevel::LNeg2,
        "L-1" | "l-1" => HierarchyLevel::LNeg1,
        "L0" | "l0" => HierarchyLevel::L0,
        "L1" | "l1" => HierarchyLevel::L1,
        "L2" | "l2" => HierarchyLevel::L2,
        "L3" | "l3" => HierarchyLevel::L3,
        "L4" | "l4" => HierarchyLevel::L4,
        "L5" | "l5" => HierarchyLevel::L5,
        "L6" | "l6" => HierarchyLevel::L6,
        "L7" | "l7" => HierarchyLevel::L7,
        "L8" | "l8" => HierarchyLevel::L8,
        "L9" | "l9" => HierarchyLevel::L9,
        "L10" | "l10" => HierarchyLevel::L10,
        "L11" | "l11" => HierarchyLevel::L11,
        "L12" | "l12" => HierarchyLevel::L12,
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
    use poet_vibe::Value;
    use std::collections::BTreeMap;

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert((*k).into(), v.clone());
        }
        Value::Record(m)
    }

    const S: Span = Span::new(0, 0);

    #[test]
    fn cosmic_geodetic_to_ecef_roundtrip() {
        let input = rec(&[
            ("lat_deg", Value::F64(37.7749)),
            ("lon_deg", Value::F64(-122.4194)),
            ("alt_m", Value::F64(0.0)),
        ]);
        let ecef = geodetic_to_ecef(&input, S).unwrap();
        // Now convert back
        let back = ecef_to_geodetic(&ecef, S).unwrap();
        let lat = args::rec_f64(&back, "lat_deg").unwrap();
        let lon = args::rec_f64(&back, "lon_deg").unwrap();
        assert!((lat - 37.7749).abs() < 1e-6);
        assert!((lon - (-122.4194)).abs() < 1e-6);
    }

    #[test]
    fn cosmic_geodetic_distance_sf_to_la() {
        let input = rec(&[
            ("lat_deg", Value::F64(37.7749)),
            ("lon_deg", Value::F64(-122.4194)),
            ("lat2_deg", Value::F64(34.0522)),
            ("lon2_deg", Value::F64(-118.2437)),
        ]);
        let d = geodetic_distance(&input, S).unwrap();
        let dv = d.as_f64().unwrap();
        // SF to LA is ~559 km
        assert!(dv > 500_000.0 && dv < 600_000.0, "got {dv} m");
    }

    #[test]
    fn cosmic_surface_gravity_earth() {
        let input = rec(&[("name", Value::String("earth".into()))]);
        let g = surface_gravity(&input, S).unwrap();
        let gv = g.as_f64().unwrap();
        assert!((gv - 9.81).abs() < 0.1, "got {gv}");
    }

    #[test]
    fn cosmic_surface_gravity_mars() {
        let input = rec(&[("name", Value::String("mars".into()))]);
        let g = surface_gravity(&input, S).unwrap();
        let gv = g.as_f64().unwrap();
        assert!((gv - 3.71).abs() < 0.1, "got {gv}");
    }

    #[test]
    fn cosmic_flrw_distance_zero_redshift() {
        let input = rec(&[("z", Value::F64(0.0))]);
        let d = flrw_distance(&input, S).unwrap();
        let dv = d.as_f64().unwrap();
        assert!(dv.abs() < 1e-6, "zero redshift should give zero distance");
    }

    #[test]
    fn cosmic_stardate_tng() {
        // TNG stardate 41000 → year 2364
        let input = rec(&[("stardate", Value::F64(41000.0))]);
        let year = stardate_to_gregorian(&input, S).unwrap();
        let y = year.as_f64().unwrap();
        assert!((y - 2364.0).abs() < 1.0, "got {y}");
    }

    #[test]
    fn cosmic_warp_velocity_tng_warp1() {
        let input = rec(&[
            ("warp", Value::F64(1.0)),
            ("scale", Value::String("tng".into())),
        ]);
        let v = warp_velocity(&input, S).unwrap();
        let c = 299_792_458.0;
        assert!((v.as_f64().unwrap() - c).abs() < 1.0, "warp 1 = c");
    }

    #[test]
    fn cosmic_atmosphere_pressure_earth_surface() {
        let input = rec(&[
            ("body", Value::String("earth".into())),
            ("altitude_m", Value::F64(0.0)),
        ]);
        let p = atmosphere_pressure(&input, S).unwrap();
        assert!((p.as_f64().unwrap() - 101_325.0).abs() < 1.0);
    }

    #[test]
    fn cosmic_compton_wavelength_electron() {
        let input = rec(&[("particle", Value::String("electron".into()))]);
        let w = compton_wavelength(&input, S).unwrap();
        // Electron Compton wavelength ≈ 2.426e-12 m
        let wv = w.as_f64().unwrap();
        assert!(wv > 2.0e-12 && wv < 3.0e-12, "got {wv}");
    }

    #[test]
    fn cosmic_scale_factor_l0_to_l5() {
        let input = rec(&[
            ("from_level", Value::String("L0".into())),
            ("to_level", Value::String("L5".into())),
        ]);
        let sf = scale_factor(&input, S).unwrap();
        // L0 (quantum ~2.4e-12 m) to L5 (geodetic ~1 km) — 1 unit at L0
        // is a tiny fraction of 1 unit at L5
        let sfv = sf.as_f64().unwrap();
        assert!(sfv > 0.0 && sfv < 1e-10, "got {sfv}");
    }
}
