//! Dual-path Tool Chest actions for curated `Cosmic.*` ALL_BOUND ids (waves 19–20).
//!
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.

use serde_json::json;
use web_sys::{Document, Element};

const C_LIGHT: f64 = 299_792_458.0;
const H0_PER_S: f64 = 2.2685455e-18; // ≈ 70 km/s/Mpc
const MPC_M: f64 = 3.085_677_581e22;
const EARTH_R_M: f64 = 6_371_000.0;
const H_PLANCK: f64 = 6.626_070_15e-34;
const M_ELECTRON: f64 = 9.109_383_701_5e-31;
const M_PROTON: f64 = 1.672_621_923_69e-27;
const ELECTRON_COMPTON_M: f64 = 2.426_310_238_67e-12;
const BOHR_RADIUS_M: f64 = 5.291_772_109_03e-11;
const WGS84_A: f64 = 6_378_137.0;
const WGS84_F: f64 = 1.0 / 298.257_223_563;
const WGS84_B: f64 = WGS84_A * (1.0 - WGS84_F);
const WGS84_E2: f64 = WGS84_F * (2.0 - WGS84_F);

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn selected_source(document: &Document) -> Option<String> {
    let container = selected_container(document)?;
    let text = container
        .query_selector(".vibe-editor, .vibe-editor-textarea, .doc-editor, .sheet-grid")
        .ok()
        .flatten()
        .and_then(|editor| editor.text_content())
        .or_else(|| container.text_content())?;
    let bounded: String = text.chars().take(16_384).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn parse_numbers(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|' | '\n' | '\r'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(64)
        .collect()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
}

fn string_attr(el: Option<&Element>, name: &str) -> Option<String> {
    el.and_then(|e| e.get_attribute(name))
        .filter(|s| !s.trim().is_empty())
}

fn first_token(source: &str) -> Option<String> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|'))
        .map(str::trim)
        .find(|t| !t.is_empty() && t.parse::<f64>().is_err())
        .map(|t| t.chars().take(64).collect())
}

fn nth_non_numeric_token(source: &str, n: usize) -> Option<String> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|'))
        .map(str::trim)
        .filter(|t| !t.is_empty() && t.parse::<f64>().is_err())
        .nth(n)
        .map(|t| t.chars().take(96).collect())
}

fn usri_candidate(container: Option<&Element>, source: &str) -> Option<String> {
    string_attr(container, "data-uri")
        .or_else(|| string_attr(container, "data-usri"))
        .or_else(|| {
            source
                .lines()
                .map(str::trim)
                .find(|l| l.starts_with("urn:omni:"))
                .map(|l| l.chars().take(256).collect())
        })
        .or_else(|| {
            let trimmed = source.trim();
            trimmed
                .starts_with("urn:omni:")
                .then(|| trimmed.chars().take(256).collect())
        })
}

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: serde_json::Value,
) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(cap_id, &local_message);
        super::interactions::show_tool_status(
            document,
            &label,
            &report.message,
            report.status_kind,
        );
        return;
    }
    super::interactions::show_tool_status(
        document,
        &label,
        &format!("Running {cap_id}…"),
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_invoke(cap_id, args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(cap_id, &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("capability invoke failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(cap_id, &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

// ── Local sketches (offline; Host path is authoritative) ─────────────

fn deg2rad(d: f64) -> f64 {
    d * std::f64::consts::PI / 180.0
}

fn local_geodetic_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let phi1 = deg2rad(lat1);
    let phi2 = deg2rad(lat2);
    let dphi = deg2rad(lat2 - lat1);
    let dlam = deg2rad(lon2 - lon1);
    let a = (dphi / 2.0).sin().powi(2) + phi1.cos() * phi2.cos() * (dlam / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    EARTH_R_M * c
}

fn local_surface_gravity(name: &str) -> Option<f64> {
    Some(match name.trim().to_ascii_lowercase().as_str() {
        "earth" => 9.80665,
        "mars" => 3.72076,
        "moon" | "luna" => 1.62,
        "venus" => 8.87,
        "jupiter" => 24.79,
        _ => return None,
    })
}

fn local_flrw_distance_m(z: f64) -> Option<f64> {
    if !(z >= 0.0 && z.is_finite()) {
        return None;
    }
    Some(C_LIGHT * z / H0_PER_S)
}

fn local_flrw_redshift(a_emit: f64) -> Option<f64> {
    if !(a_emit > 0.0 && a_emit.is_finite()) {
        return None;
    }
    Some(1.0 / a_emit - 1.0)
}

fn local_hubble_velocity(distance_m: f64) -> Option<f64> {
    if !distance_m.is_finite() {
        return None;
    }
    Some(H0_PER_S * distance_m)
}

fn local_warp_factor_c(w: f64, tos: bool) -> Option<f64> {
    if !(w >= 0.0 && w.is_finite()) {
        return None;
    }
    if tos {
        Some(w * w * w)
    } else if w >= 10.0 {
        Some(f64::INFINITY)
    } else if w <= 9.0 {
        Some(w.powf(10.0 / 3.0))
    } else {
        let base = w.powf(10.0 / 3.0);
        let tan_arg = std::f64::consts::PI * (w - 9.0) / (2.0 * (1.0 + 1e-6));
        Some(base * (1.0 + 0.1 * tan_arg.tan()))
    }
}

fn local_typical_length(level: &str) -> Option<f64> {
    Some(match level.trim().to_ascii_lowercase().as_str() {
        "l-2" => 1.616_255e-35,
        "l-1" => 1e-34,
        "l0" => ELECTRON_COMPTON_M,
        "l1" => 1e-15,
        "l2" => BOHR_RADIUS_M,
        "l3" => 1e-9,
        "l4" => 1e-5,
        "l5" | "l6" => 1e3,
        "l7" => 1e11,
        "l8" => 1e16,
        "l9" => 1e21,
        "l10" => 1e23,
        "l11" => 1e24,
        "l12" => 8.8e26,
        _ => return None,
    })
}

fn local_observe_redshift(z: f64) -> Option<(f64, f64, f64)> {
    let d_m = local_flrw_distance_m(z)?;
    let d_mpc = d_m / MPC_M;
    let v_km_s = local_hubble_velocity(d_m)? / 1000.0;
    Some((d_mpc, d_mpc, v_km_s))
}

fn local_compton(particle: &str) -> Option<f64> {
    match particle.trim().to_ascii_lowercase().as_str() {
        "electron" | "e" => Some(ELECTRON_COMPTON_M),
        "proton" | "p" => Some(H_PLANCK / (M_PROTON * C_LIGHT)),
        _ => None,
    }
}

fn local_de_broglie(particle: &str, v: f64) -> Option<f64> {
    if !(v > 0.0 && v.is_finite()) {
        return None;
    }
    let m = match particle.trim().to_ascii_lowercase().as_str() {
        "electron" | "e" => M_ELECTRON,
        "proton" | "p" => M_PROTON,
        _ => return None,
    };
    let lambda = H_PLANCK / (m * v);
    lambda.is_finite().then_some(lambda)
}

fn local_atmosphere_pressure(body: &str, altitude_m: f64) -> Option<f64> {
    if !altitude_m.is_finite() {
        return None;
    }
    let (p0, h) = match body.trim().to_ascii_lowercase().as_str() {
        "earth" => (101_325.0, 8500.0),
        "mars" => (610.0, 11_100.0),
        "venus" => (9.2e6, 15_900.0),
        _ => return None,
    };
    Some(p0 * (-altitude_m / h).exp())
}

fn local_geodetic_to_ecef(lat_deg: f64, lon_deg: f64, alt_m: f64) -> Option<(f64, f64, f64)> {
    if !(lat_deg.is_finite() && lon_deg.is_finite() && alt_m.is_finite()) {
        return None;
    }
    let lat = deg2rad(lat_deg);
    let lon = deg2rad(lon_deg);
    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let n = WGS84_A / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();
    Some((
        (n + alt_m) * cos_lat * lon.cos(),
        (n + alt_m) * cos_lat * lon.sin(),
        (n * (1.0 - WGS84_E2) + alt_m) * sin_lat,
    ))
}

fn local_ecef_to_geodetic(x: f64, y: f64, z: f64) -> Option<(f64, f64, f64)> {
    if !(x.is_finite() && y.is_finite() && z.is_finite()) {
        return None;
    }
    let lon = y.atan2(x);
    let p = (x * x + y * y).sqrt();
    if p < 1e-12 {
        return Some((
            if z > 0.0 { 90.0 } else { -90.0 },
            0.0,
            z.abs() - WGS84_B,
        ));
    }
    let e_prime_sq = (WGS84_A * WGS84_A - WGS84_B * WGS84_B) / (WGS84_B * WGS84_B);
    let theta = (z * WGS84_A).atan2(p * WGS84_B);
    let sin_theta = theta.sin();
    let cos_theta = theta.cos();
    let lat = (z + e_prime_sq * WGS84_B * sin_theta * sin_theta * sin_theta)
        .atan2(p - WGS84_E2 * WGS84_A * cos_theta * cos_theta * cos_theta);
    let sin_lat = lat.sin();
    let n = WGS84_A / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();
    let alt = p / lat.cos() - n;
    Some((lat.to_degrees(), lon.to_degrees(), alt))
}

fn local_ecef_to_enu(
    x: f64,
    y: f64,
    z: f64,
    ref_lat: f64,
    ref_lon: f64,
    ref_alt: f64,
) -> Option<(f64, f64, f64)> {
    let (rx, ry, rz) = local_geodetic_to_ecef(ref_lat, ref_lon, ref_alt)?;
    let dx = x - rx;
    let dy = y - ry;
    let dz = z - rz;
    let lat = deg2rad(ref_lat);
    let lon = deg2rad(ref_lon);
    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let sin_lon = lon.sin();
    let cos_lon = lon.cos();
    Some((
        -sin_lon * dx + cos_lon * dy,
        -sin_lat * cos_lon * dx - sin_lat * sin_lon * dy + cos_lat * dz,
        cos_lat * cos_lon * dx + cos_lat * sin_lon * dy + sin_lat * dz,
    ))
}

fn local_enu_to_ecef(
    east: f64,
    north: f64,
    up: f64,
    ref_lat: f64,
    ref_lon: f64,
    ref_alt: f64,
) -> Option<(f64, f64, f64)> {
    let (rx, ry, rz) = local_geodetic_to_ecef(ref_lat, ref_lon, ref_alt)?;
    let lat = deg2rad(ref_lat);
    let lon = deg2rad(ref_lon);
    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let sin_lon = lon.sin();
    let cos_lon = lon.cos();
    Some((
        rx - sin_lon * east - sin_lat * cos_lon * north + cos_lat * cos_lon * up,
        ry + cos_lon * east - sin_lat * sin_lon * north + cos_lat * sin_lon * up,
        rz + cos_lat * north + sin_lat * up,
    ))
}

fn local_body_profile(name: &str) -> Option<(f64, f64)> {
    // (equatorial_radius_m, mass_kg) sketch table — Host is authoritative.
    Some(match name.trim().to_ascii_lowercase().as_str() {
        "earth" => (6_378_137.0, 5.972e24),
        "mars" => (3_396_200.0, 6.417e23),
        "jupiter" => (71_492_000.0, 1.898e27),
        "saturn" => (60_268_000.0, 5.683e26),
        "moon" | "luna" => (1_737_400.0, 7.342e22),
        "europa" => (1_560_800.0, 4.8e22),
        "sun" => (696_340_000.0, 1.989e30),
        "sgr-a" => (1.227e10, 8.54e36),
        "neutron-star" => (12_000.0, 2.8e30),
        _ => return None,
    })
}

fn local_stardate_year(stardate: f64) -> Option<f64> {
    if !stardate.is_finite() {
        return None;
    }
    Some(if stardate >= 860_000.0 {
        3188.0 + (stardate - 860_000.0) / 1000.0
    } else if stardate >= 41_000.0 {
        2364.0 + (stardate - 41_000.0) / 1000.0
    } else {
        2265.0 + 0.1 * stardate
    })
}

fn local_atmosphere_temperature(body: &str, altitude_m: f64) -> Option<f64> {
    if !altitude_m.is_finite() {
        return None;
    }
    let t0 = match body.trim().to_ascii_lowercase().as_str() {
        "earth" => 288.15,
        "mars" => 210.0,
        "venus" => 737.0,
        _ => return None,
    };
    let lapse = 0.0065;
    let tropo = 12_000.0;
    if altitude_m < tropo {
        Some(t0 - lapse * altitude_m)
    } else {
        Some(t0 - lapse * tropo)
    }
}

fn local_magnetosphere_field(body: &str, distance_m: f64, body_radius_m: f64) -> Option<f64> {
    if !(distance_m > 0.0 && body_radius_m > 0.0 && distance_m.is_finite() && body_radius_m.is_finite())
    {
        return None;
    }
    let b0 = match body.trim().to_ascii_lowercase().as_str() {
        "earth" => 3.12e-5,
        "jupiter" => 4.28e-4,
        "mars" => 0.0,
        _ => return None,
    };
    Some(b0 * (body_radius_m / distance_m).powi(3))
}

fn local_scale_factor(from_level: &str, to_level: &str) -> Option<f64> {
    let from_l = local_typical_length(from_level)?;
    let to_l = local_typical_length(to_level)?;
    if to_l == 0.0 {
        return None;
    }
    let sf = from_l / to_l;
    sf.is_finite().then_some(sf)
}

fn local_usri_sketch(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("urn:omni:")?;
    let main = rest.split('#').next().unwrap_or(rest);
    let main_path = main.split('/').next().unwrap_or(main);
    let segs: Vec<&str> = main_path.split(':').collect();
    if segs.len() < 5 {
        return None;
    }
    Some(format!(
        "v={} realm={} path={}",
        segs[0],
        segs[1],
        segs[4].chars().take(48).collect::<String>()
    ))
}

fn resolve_scale(container: Option<&Element>, source: &str) -> String {
    string_attr(container, "data-scale")
        .or_else(|| {
            first_token(source)
                .filter(|t| matches!(t.to_ascii_lowercase().as_str(), "tos" | "tng"))
        })
        .unwrap_or_else(|| "tng".into())
        .to_ascii_lowercase()
}

fn is_tos(scale: &str) -> bool {
    scale.eq_ignore_ascii_case("tos")
}

/// `Cosmic.geodetic_distance` — `{ lat_deg, lon_deg, lat2_deg, lon2_deg }`.
pub(super) fn run_geodetic_distance(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let lat = numeric_attr(container.as_ref(), "data-lat-deg")
        .or_else(|| nums.first().copied())
        .unwrap_or(37.7749);
    let lon = numeric_attr(container.as_ref(), "data-lon-deg")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(-122.4194);
    let lat2 = numeric_attr(container.as_ref(), "data-lat2-deg")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(34.0522);
    let lon2 = numeric_attr(container.as_ref(), "data-lon2-deg")
        .or_else(|| nums.get(3).copied())
        .unwrap_or(-118.2437);
    let sketch = local_geodetic_distance(lat, lon, lat2, lon2);
    invoke_dual(
        document,
        label,
        "Cosmic.geodetic_distance",
        format!("Geodetic distance sketch ≈ {sketch:.0} m"),
        json!({
            "lat_deg": lat,
            "lon_deg": lon,
            "lat2_deg": lat2,
            "lon2_deg": lon2,
        }),
    );
}

/// `Cosmic.surface_gravity` — `{ name }`.
pub(super) fn run_surface_gravity(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let name = string_attr(container.as_ref(), "data-name")
        .or_else(|| string_attr(container.as_ref(), "data-body"))
        .or_else(|| first_token(&source))
        .unwrap_or_else(|| "earth".into())
        .to_ascii_lowercase();
    let Some(g) = local_surface_gravity(&name) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a known body name (earth/mars/moon/venus/jupiter via data-name).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.surface_gravity",
        format!("Surface gravity sketch {name} ≈ {g:.4} m/s²"),
        json!({ "name": name }),
    );
}

/// `Cosmic.flrw_distance` — `{ z }`.
pub(super) fn run_flrw_distance(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let z = numeric_attr(container.as_ref(), "data-z")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.1);
    let Some(d) = local_flrw_distance_m(z) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need z ≥ 0 (data-z or one surface number).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.flrw_distance",
        format!("FLRW distance sketch z={z} → d≈{d:.3e} m"),
        json!({ "z": z }),
    );
}

/// `Cosmic.flrw_redshift` — `{ a_emit }`.
pub(super) fn run_flrw_redshift(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let a_emit = numeric_attr(container.as_ref(), "data-a-emit")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.5);
    let Some(z) = local_flrw_redshift(a_emit) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a_emit > 0 (data-a-emit or one surface number).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.flrw_redshift",
        format!("FLRW redshift sketch a={a_emit} → z≈{z:.6}"),
        json!({ "a_emit": a_emit }),
    );
}

/// `Cosmic.flrw_hubble_velocity` — `{ distance_m }`.
pub(super) fn run_flrw_hubble_velocity(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let distance_m = numeric_attr(container.as_ref(), "data-distance-m")
        .or_else(|| nums.first().copied())
        .unwrap_or(MPC_M);
    let Some(v) = local_hubble_velocity(distance_m) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite distance_m (data-distance-m or one surface number).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.flrw_hubble_velocity",
        format!("Hubble velocity sketch d={distance_m:.3e} m → v≈{v:.3e} m/s"),
        json!({ "distance_m": distance_m }),
    );
}

/// `Cosmic.warp_velocity` — `{ warp, scale? }`.
pub(super) fn run_warp_velocity(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let warp = numeric_attr(container.as_ref(), "data-warp")
        .or_else(|| nums.first().copied())
        .unwrap_or(1.0);
    let scale = resolve_scale(container.as_ref(), &source);
    let Some(fc) = local_warp_factor_c(warp, is_tos(&scale)) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need warp ≥ 0 (data-warp or one surface number).",
            "error",
        );
        return;
    };
    let v = fc * C_LIGHT;
    invoke_dual(
        document,
        label,
        "Cosmic.warp_velocity",
        format!("Warp velocity sketch warp={warp} ({scale}) → v≈{v:.3e} m/s"),
        json!({ "warp": warp, "scale": scale }),
    );
}

/// `Cosmic.warp_factor_c` — `{ warp, scale? }` (wave-18 Host).
pub(super) fn run_warp_factor_c(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let warp = numeric_attr(container.as_ref(), "data-warp")
        .or_else(|| nums.first().copied())
        .unwrap_or(2.0);
    let scale = resolve_scale(container.as_ref(), &source);
    let Some(fc) = local_warp_factor_c(warp, is_tos(&scale)) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need warp ≥ 0 (data-warp or one surface number).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.warp_factor_c",
        format!("Warp factor-c sketch warp={warp} ({scale}) → v/c≈{fc:.6}"),
        json!({ "warp": warp, "scale": scale }),
    );
}

/// `Cosmic.typical_length` — `{ level }` (wave-18 Host).
pub(super) fn run_typical_length(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let level = string_attr(container.as_ref(), "data-level")
        .or_else(|| first_token(&source))
        .unwrap_or_else(|| "L2".into());
    let Some(len) = local_typical_length(&level) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need hierarchy level L-2…L12 (data-level or token).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.typical_length",
        format!("Typical length sketch {level} ≈ {len:.3e} m"),
        json!({ "level": level }),
    );
}

/// `Cosmic.observe_redshift` — `{ z }` (wave-18 Host).
pub(super) fn run_observe_redshift(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let z = numeric_attr(container.as_ref(), "data-z")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.0);
    let Some((comoving, _physical, v_km_s)) = local_observe_redshift(z) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need z ≥ 0 (data-z or one surface number).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.observe_redshift",
        format!("Observe-redshift sketch z={z} → d≈{comoving:.4} Mpc, v≈{v_km_s:.2} km/s"),
        json!({ "z": z }),
    );
}

/// `Cosmic.compton_wavelength` — `{ particle? }`.
pub(super) fn run_compton_wavelength(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let particle = string_attr(container.as_ref(), "data-particle")
        .or_else(|| first_token(&source))
        .unwrap_or_else(|| "electron".into())
        .to_ascii_lowercase();
    let Some(lambda) = local_compton(&particle) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need particle electron|proton (data-particle).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.compton_wavelength",
        format!("Compton wavelength sketch {particle} ≈ {lambda:.3e} m"),
        json!({ "particle": particle }),
    );
}

/// `Cosmic.de_broglie_wavelength` — `{ particle?, velocity_m_s }`.
pub(super) fn run_de_broglie_wavelength(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let particle = string_attr(container.as_ref(), "data-particle")
        .or_else(|| first_token(&source))
        .unwrap_or_else(|| "electron".into())
        .to_ascii_lowercase();
    let velocity_m_s = numeric_attr(container.as_ref(), "data-velocity-m-s")
        .or_else(|| nums.first().copied())
        .unwrap_or(1.0e6);
    let Some(lambda) = local_de_broglie(&particle, velocity_m_s) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need particle electron|proton and velocity_m_s > 0.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.de_broglie_wavelength",
        format!("de Broglie sketch {particle} @ {velocity_m_s:.3e} m/s → λ≈{lambda:.3e} m"),
        json!({ "particle": particle, "velocity_m_s": velocity_m_s }),
    );
}

/// `Cosmic.atmosphere_pressure` — `{ body?, altitude_m }`.
pub(super) fn run_atmosphere_pressure(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let body = string_attr(container.as_ref(), "data-body")
        .or_else(|| first_token(&source))
        .unwrap_or_else(|| "earth".into())
        .to_ascii_lowercase();
    let altitude_m = numeric_attr(container.as_ref(), "data-altitude-m")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.0);
    let Some(p) = local_atmosphere_pressure(&body, altitude_m) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need body earth|mars|venus and finite altitude_m.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.atmosphere_pressure",
        format!("Atmosphere pressure sketch {body} @ {altitude_m:.0} m → P≈{p:.3e} Pa"),
        json!({ "body": body, "altitude_m": altitude_m }),
    );
}

/// `Cosmic.geodetic_to_ecef` — `{ lat_deg, lon_deg, alt_m }`.
pub(super) fn run_geodetic_to_ecef(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let lat = numeric_attr(container.as_ref(), "data-lat-deg")
        .or_else(|| nums.first().copied())
        .unwrap_or(37.7749);
    let lon = numeric_attr(container.as_ref(), "data-lon-deg")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(-122.4194);
    let alt_m = numeric_attr(container.as_ref(), "data-alt-m")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(0.0);
    let Some((x, y, z)) = local_geodetic_to_ecef(lat, lon, alt_m) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite lat_deg, lon_deg, alt_m.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.geodetic_to_ecef",
        format!("Geodetic→ECEF sketch → ({x:.3e}, {y:.3e}, {z:.3e}) m"),
        json!({ "lat_deg": lat, "lon_deg": lon, "alt_m": alt_m }),
    );
}

/// `Cosmic.ecef_to_geodetic` — `{ x, y, z }`.
pub(super) fn run_ecef_to_geodetic(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let x = numeric_attr(container.as_ref(), "data-x")
        .or_else(|| nums.first().copied())
        .unwrap_or(-2.710_057e6);
    let y = numeric_attr(container.as_ref(), "data-y")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(-4.278_266e6);
    let z = numeric_attr(container.as_ref(), "data-z")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(3.855_140e6);
    let Some((lat, lon, alt)) = local_ecef_to_geodetic(x, y, z) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite ECEF x, y, z (metres).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.ecef_to_geodetic",
        format!("ECEF→geodetic sketch → lat≈{lat:.4}° lon≈{lon:.4}° alt≈{alt:.1} m"),
        json!({ "x": x, "y": y, "z": z }),
    );
}

/// `Cosmic.ecef_to_enu` — `{ x, y, z, ref_lat_deg, ref_lon_deg, ref_alt_m }`.
pub(super) fn run_ecef_to_enu(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let x = numeric_attr(container.as_ref(), "data-x")
        .or_else(|| nums.first().copied())
        .unwrap_or(-2.710_057e6);
    let y = numeric_attr(container.as_ref(), "data-y")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(-4.278_266e6);
    let z = numeric_attr(container.as_ref(), "data-z")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(3.855_140e6);
    let ref_lat = numeric_attr(container.as_ref(), "data-ref-lat-deg")
        .or_else(|| nums.get(3).copied())
        .unwrap_or(37.7749);
    let ref_lon = numeric_attr(container.as_ref(), "data-ref-lon-deg")
        .or_else(|| nums.get(4).copied())
        .unwrap_or(-122.4194);
    let ref_alt = numeric_attr(container.as_ref(), "data-ref-alt-m")
        .or_else(|| nums.get(5).copied())
        .unwrap_or(0.0);
    let Some((east, north, up)) = local_ecef_to_enu(x, y, z, ref_lat, ref_lon, ref_alt) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need ECEF x,y,z plus ref_lat_deg, ref_lon_deg, ref_alt_m.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.ecef_to_enu",
        format!("ECEF→ENU sketch → E≈{east:.1} N≈{north:.1} U≈{up:.1} m"),
        json!({
            "x": x, "y": y, "z": z,
            "ref_lat_deg": ref_lat, "ref_lon_deg": ref_lon, "ref_alt_m": ref_alt,
        }),
    );
}

/// `Cosmic.enu_to_ecef` — `{ east, north, up, ref_lat_deg, ref_lon_deg, ref_alt_m }`.
pub(super) fn run_enu_to_ecef(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let east = numeric_attr(container.as_ref(), "data-east")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.0);
    let north = numeric_attr(container.as_ref(), "data-north")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(0.0);
    let up = numeric_attr(container.as_ref(), "data-up")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(0.0);
    let ref_lat = numeric_attr(container.as_ref(), "data-ref-lat-deg")
        .or_else(|| nums.get(3).copied())
        .unwrap_or(37.7749);
    let ref_lon = numeric_attr(container.as_ref(), "data-ref-lon-deg")
        .or_else(|| nums.get(4).copied())
        .unwrap_or(-122.4194);
    let ref_alt = numeric_attr(container.as_ref(), "data-ref-alt-m")
        .or_else(|| nums.get(5).copied())
        .unwrap_or(0.0);
    let Some((x, y, z)) = local_enu_to_ecef(east, north, up, ref_lat, ref_lon, ref_alt) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need east,north,up plus ref_lat_deg, ref_lon_deg, ref_alt_m.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.enu_to_ecef",
        format!("ENU→ECEF sketch → ({x:.3e}, {y:.3e}, {z:.3e}) m"),
        json!({
            "east": east, "north": north, "up": up,
            "ref_lat_deg": ref_lat, "ref_lon_deg": ref_lon, "ref_alt_m": ref_alt,
        }),
    );
}

/// `Cosmic.body_profile` — `{ name }`.
pub(super) fn run_body_profile(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let name = string_attr(container.as_ref(), "data-name")
        .or_else(|| string_attr(container.as_ref(), "data-body"))
        .or_else(|| first_token(&source))
        .unwrap_or_else(|| "earth".into())
        .to_ascii_lowercase();
    let Some((radius, mass)) = local_body_profile(&name) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need known body name (earth/mars/jupiter/moon/… via data-name).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.body_profile",
        format!("Body profile sketch {name} R≈{radius:.3e} m M≈{mass:.3e} kg"),
        json!({ "name": name }),
    );
}

/// `Cosmic.stardate_to_gregorian` — `{ stardate }`.
pub(super) fn run_stardate_to_gregorian(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let stardate = numeric_attr(container.as_ref(), "data-stardate")
        .or_else(|| nums.first().copied())
        .unwrap_or(41_000.0);
    let Some(year) = local_stardate_year(stardate) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite stardate (data-stardate or one surface number).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.stardate_to_gregorian",
        format!("Stardate sketch {stardate} → ≈{year:.2} CE"),
        json!({ "stardate": stardate }),
    );
}

/// `Cosmic.cochrane_units` — `{ warp, scale? }` (equals warp factor-c).
pub(super) fn run_cochrane_units(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let warp = numeric_attr(container.as_ref(), "data-warp")
        .or_else(|| nums.first().copied())
        .unwrap_or(1.0);
    let scale = resolve_scale(container.as_ref(), &source);
    let Some(c) = local_warp_factor_c(warp, is_tos(&scale)) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need warp ≥ 0 (data-warp or one surface number).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.cochrane_units",
        format!("Cochrane units sketch warp={warp} ({scale}) → ≈{c:.6}"),
        json!({ "warp": warp, "scale": scale }),
    );
}

/// `Cosmic.atmosphere_temperature` — `{ body?, altitude_m }`.
pub(super) fn run_atmosphere_temperature(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let body = string_attr(container.as_ref(), "data-body")
        .or_else(|| first_token(&source))
        .unwrap_or_else(|| "earth".into())
        .to_ascii_lowercase();
    let altitude_m = numeric_attr(container.as_ref(), "data-altitude-m")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.0);
    let Some(t) = local_atmosphere_temperature(&body, altitude_m) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need body earth|mars|venus and finite altitude_m.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.atmosphere_temperature",
        format!("Atmosphere temperature sketch {body} @ {altitude_m:.0} m → T≈{t:.2} K"),
        json!({ "body": body, "altitude_m": altitude_m }),
    );
}

/// `Cosmic.magnetosphere_field` — `{ body?, distance_m, body_radius_m? }`.
pub(super) fn run_magnetosphere_field(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let body = string_attr(container.as_ref(), "data-body")
        .or_else(|| first_token(&source))
        .unwrap_or_else(|| "earth".into())
        .to_ascii_lowercase();
    let distance_m = numeric_attr(container.as_ref(), "data-distance-m")
        .or_else(|| nums.first().copied())
        .unwrap_or(EARTH_R_M * 2.0);
    let body_radius_m = numeric_attr(container.as_ref(), "data-body-radius-m")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(EARTH_R_M);
    let Some(b) = local_magnetosphere_field(&body, distance_m, body_radius_m) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need body earth|jupiter|mars, distance_m > 0, body_radius_m > 0.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.magnetosphere_field",
        format!("Magnetosphere field sketch {body} @ {distance_m:.3e} m → B≈{b:.3e} T"),
        json!({
            "body": body,
            "distance_m": distance_m,
            "body_radius_m": body_radius_m,
        }),
    );
}

/// `Cosmic.scale_factor` — `{ from_level, to_level }`.
pub(super) fn run_scale_factor(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let from_level = string_attr(container.as_ref(), "data-from-level")
        .or_else(|| nth_non_numeric_token(&source, 0))
        .unwrap_or_else(|| "L0".into());
    let to_level = string_attr(container.as_ref(), "data-to-level")
        .or_else(|| nth_non_numeric_token(&source, 1))
        .unwrap_or_else(|| "L5".into());
    let Some(sf) = local_scale_factor(&from_level, &to_level) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need hierarchy levels L-2…L12 (data-from-level / data-to-level).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.scale_factor",
        format!("Scale factor sketch {from_level}→{to_level} ≈ {sf:.3e}"),
        json!({ "from_level": from_level, "to_level": to_level }),
    );
}

/// `Cosmic.usri_parse` — `{ uri }`.
pub(super) fn run_usri_parse(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let uri = usri_candidate(container.as_ref(), &source).unwrap_or_else(|| {
        "urn:omni:v1:physical:observable:standard:earth".into()
    });
    let Some(sketch) = local_usri_sketch(&uri) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need urn:omni:… USRI (data-uri or surface text).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Cosmic.usri_parse",
        format!("USRI parse sketch {sketch}"),
        json!({ "uri": uri }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_cosmic_wave19_match_known() {
        let d = local_geodetic_distance(37.7749, -122.4194, 34.0522, -118.2437);
        assert!(d > 500_000.0 && d < 600_000.0, "SF–LA sketch got {d}");

        assert!((local_surface_gravity("earth").unwrap() - 9.80665).abs() < 1e-9);
        assert!((local_surface_gravity("mars").unwrap() - 3.72076).abs() < 1e-9);

        assert!(local_flrw_distance_m(0.0).unwrap().abs() < 1e-9);
        assert!((local_flrw_redshift(0.5).unwrap() - 1.0).abs() < 1e-12);
        let v = local_hubble_velocity(MPC_M).unwrap() / 1000.0;
        assert!((v - 70.0).abs() < 1.0, "H0≈70 km/s/Mpc got {v}");

        assert!((local_warp_factor_c(2.0, true).unwrap() - 8.0).abs() < 1e-12);
        assert!((local_warp_factor_c(1.0, false).unwrap() - 1.0).abs() < 1e-12);

        let l2 = local_typical_length("L2").unwrap();
        assert!(l2 > 5.0e-11 && l2 < 6.0e-11, "Bohr sketch got {l2}");

        let (c_mpc, _, v0) = local_observe_redshift(0.0).unwrap();
        assert!(c_mpc.abs() < 1e-12);
        assert!(v0.abs() < 1e-9);

        let e = local_compton("electron").unwrap();
        assert!(e > 2.0e-12 && e < 3.0e-12);

        let db = local_de_broglie("electron", 1.0e6).unwrap();
        assert!(db.is_finite() && db > 0.0);

        assert!((local_atmosphere_pressure("earth", 0.0).unwrap() - 101_325.0).abs() < 1.0);
    }

    #[test]
    fn local_cosmic_wave20_remainder_match_known() {
        let (x, y, z) = local_geodetic_to_ecef(0.0, 0.0, 0.0).unwrap();
        assert!((x - WGS84_A).abs() < 1.0, "equator ECEF x got {x}");
        assert!(y.abs() < 1.0);
        assert!(z.abs() < 1.0);

        let (lat, lon, alt) = local_ecef_to_geodetic(x, y, z).unwrap();
        assert!(lat.abs() < 1e-6);
        assert!(lon.abs() < 1e-6);
        assert!(alt.abs() < 1.0);

        let (e, n, u) = local_ecef_to_enu(x, y, z, 0.0, 0.0, 0.0).unwrap();
        assert!(e.abs() < 1.0 && n.abs() < 1.0 && u.abs() < 1.0);
        let back = local_enu_to_ecef(0.0, 0.0, 0.0, 0.0, 0.0, 0.0).unwrap();
        assert!((back.0 - WGS84_A).abs() < 1.0);

        let (r, m) = local_body_profile("earth").unwrap();
        assert!((r - 6_378_137.0).abs() < 1.0);
        assert!(m > 5.0e24);

        assert!((local_stardate_year(41_000.0).unwrap() - 2364.0).abs() < 1e-9);
        assert!((local_warp_factor_c(1.0, false).unwrap() - 1.0).abs() < 1e-12); // cochrane ≡ factor-c

        assert!((local_atmosphere_temperature("earth", 0.0).unwrap() - 288.15).abs() < 1e-9);
        let b = local_magnetosphere_field("earth", EARTH_R_M * 2.0, EARTH_R_M).unwrap();
        assert!((b - 3.12e-5 / 8.0).abs() < 1e-9);

        let sf = local_scale_factor("L0", "L5").unwrap();
        assert!(sf > 0.0 && sf < 1e-10, "L0→L5 sketch got {sf}");

        let usri = local_usri_sketch("urn:omni:v1:physical:observable:standard:earth").unwrap();
        assert!(usri.contains("physical"));
        assert!(usri.contains("earth"));
    }
}
