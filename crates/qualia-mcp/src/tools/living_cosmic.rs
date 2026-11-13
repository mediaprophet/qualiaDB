//! MCP tools for 0.0.41 Living-World Biological/Anatomical Engines, Etalon-Vibe SDK
//! Parity Dispatch, and Omniversal Cosmic Coordinate Systems (OCS v2.2.0).
//!
//! Follows `AGENTS.md` §0-B (<500 lines) and §0-A (zero-heap hot-path calls).

use crate::types::{parse_tool_args, McpSystemError};
use qualia_core_db::modalities::biological_ontology::{
    find_pin_by_id, find_pins_by_scale, query_closest_anatomical_pin_zero_heap,
    CANONICAL_ANATOMICAL_PINS,
};
use qualia_core_db::render::botanical_anatomy::{
    root_flare_radius, GOLDEN_ANGLE_RAD, PIPE_EXPONENT_EUCALYPTUS,
};
use qualia_core_db::render::skeletal::fauna_rig::{FAUNA_MAX_JOINTS, FAUNA_PARENT_INDICES};
use qualia_core_db::render::vertebrate_anatomy::{
    evaluate_anatomical_layer_peel_zero_heap, ZoologicalAnatomicalLayer,
};
use serde_json::{json, Value};
use vibe::cosmic::{
    celestial::{body_profile_by_name, pulsar_dispersion_delay_s},
    flrw::observe_redshift,
    nested::compute_healpix_index,
    usri::g_coord_realm_to_realm_class,
    warp::{cochrane_units, warp_velocity, WarpScale},
};
use vibe::extension_registry::{ExtensionRegistry, BUILTIN_EXTENSION_ALIASES};

/// `etalon_vibe_sdk` — Inspect the 11 Etalon-Vibe SDK providers, 37 extension descriptors, and 5-field `NQuin48` parity.
pub fn etalon_vibe_sdk(raw_args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(raw_args)?;
    let filter_alias = v.get("provider").and_then(Value::as_str);
    let registry = ExtensionRegistry::with_builtin_providers();

    let mut providers = Vec::new();
    for &alias in BUILTIN_EXTENSION_ALIASES {
        if let Some(target) = filter_alias {
            if !alias.eq_ignore_ascii_case(target) {
                continue;
            }
        }
        let Some((_, provider, descriptor)) = registry.find_by_namespace_or_alias(alias) else {
            return Err(McpSystemError::ExecutionFailed(format!(
                "Provider '{alias}' missing from ExtensionRegistry"
            )));
        };
        let quins = provider.static_ontology_nquins();
        let parity_ok = quins.iter().all(|q| q.verify_parity());
        let functions: Vec<Value> = descriptor
            .sub_namespaces
            .iter()
            .flat_map(|ns| {
                ns.functions.iter().map(move |f| {
                    json!({
                        "path": format!("{alias}.{}.{}", ns.name, f.name),
                        "iri": f.canonical_iri.iri,
                        "effects": f.effects.0,
                        "output_elements": f.required_output_elems,
                        "max_ticks": f.envelope.max_ticks,
                        "max_output_bytes": f.envelope.max_output_bytes,
                    })
                })
            })
            .collect();
        providers.push(json!({
            "alias": alias,
            "iri": descriptor.canonical_iri.iri,
            "targets": descriptor.supported_targets.0,
            "ontology_quins": quins.len(),
            "five_field_parity_valid": parity_ok,
            "functions": functions,
        }));
    }

    Ok(json!({
        "status": "ok",
        "schema": "qualia.etalon-vibe-sdk/v1",
        "provider_count": registry.active_count(),
        "providers": providers,
    })
    .to_string())
}

/// `living_world_anatomy` — Query botanical/vertebrate anatomical pins, zero-heap ray-sphere Loupe picking,
/// botanical root-flare allometry, and 3-layer vertebrate osteology/myology/integument peel.
pub fn living_world_anatomy(raw_args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(raw_args)?;
    let op = v.get("op").and_then(Value::as_str).unwrap_or("catalog");

    match op {
        "catalog" | "pins" => {
            let scale = v.get("scale").and_then(Value::as_u64).unwrap_or(0) as u8;
            let pins: Vec<Value> = if (1..=5).contains(&scale) {
                find_pins_by_scale(scale)
                    .map(|p| {
                        json!({
                            "pin_id": p.pin_id,
                            "scale": p.scale,
                            "curie": p.ontology_curie,
                            "scientific_term": p.scientific_term,
                            "anchor_xyz": p.anchor_local_xyz,
                        })
                    })
                    .collect()
            } else {
                CANONICAL_ANATOMICAL_PINS
                    .iter()
                    .map(|p| {
                        json!({
                            "pin_id": p.pin_id,
                            "scale": p.scale,
                            "curie": p.ontology_curie,
                            "scientific_term": p.scientific_term,
                            "anchor_xyz": p.anchor_local_xyz,
                        })
                    })
                    .collect()
            };
            Ok(json!({
                "status": "ok",
                "op": op,
                "scale_filter": scale,
                "pin_count": pins.len(),
                "pins": pins,
                "botanical_constants": {
                    "golden_angle_rad": GOLDEN_ANGLE_RAD,
                    "pipe_exponent_eucalyptus": PIPE_EXPONENT_EUCALYPTUS,
                },
                "vertebrate_rig": {
                    "max_joints": FAUNA_MAX_JOINTS,
                    "parent_indices": FAUNA_PARENT_INDICES,
                }
            })
            .to_string())
        }
        "pin_by_id" => {
            let id = v
                .get("pin_id")
                .and_then(Value::as_str)
                .ok_or(McpSystemError::InvalidParameters)?;
            let Some(pin) = find_pin_by_id(id) else {
                return Err(McpSystemError::ExecutionFailed(format!(
                    "Unknown pin_id: {id}"
                )));
            };
            Ok(json!({
                "status": "ok",
                "pin_id": pin.pin_id,
                "scale": pin.scale,
                "curie": pin.ontology_curie,
                "iri": pin.full_ontology_iri(),
                "scientific_term": pin.scientific_term,
                "anchor_xyz": pin.anchor_local_xyz,
                "normal_xyz": pin.normal_xyz,
                "biophysical_formula": pin.biophysical_formula,
                "summary": pin.educational_summary,
                "indigenous_custodial_context": pin.indigenous_custodial_context,
            })
            .to_string())
        }
        "ray_pick" => {
            let scale = v.get("scale").and_then(Value::as_u64).unwrap_or(0) as u8;
            let origin = parse_vec3(&v, "ray_origin", [0.0, 0.0, 5.0]);
            let dir = parse_vec3(&v, "ray_dir", [0.0, 0.0, -1.0]);
            let max_dist = v.get("max_dist").and_then(Value::as_f64).unwrap_or(10.0) as f32;
            let hit = query_closest_anatomical_pin_zero_heap(scale, origin, dir, max_dist);
            Ok(json!({
                "status": "ok",
                "op": "ray_pick",
                "hit": hit.map(|p| json!({
                    "pin_id": p.pin_id,
                    "scale": p.scale,
                    "curie": p.ontology_curie,
                    "scientific_term": p.scientific_term,
                    "anchor_xyz": p.anchor_local_xyz,
                })),
            })
            .to_string())
        }
        "root_flare" => {
            let base_r = v.get("base_radius").and_then(Value::as_f64).unwrap_or(0.45);
            let depth_m = v.get("depth_m").and_then(Value::as_f64).unwrap_or(0.2);
            let factor = v.get("flare_factor").and_then(Value::as_f64).unwrap_or(0.6);
            let r = root_flare_radius(base_r, depth_m, factor);
            Ok(json!({
                "status": "ok",
                "op": "root_flare",
                "radius_m": r,
            })
            .to_string())
        }
        "layer_peel" => {
            let ost = v.get("osteology_vis").and_then(Value::as_f64).unwrap_or(1.0) as f32;
            let myo = v.get("myology_vis").and_then(Value::as_f64).unwrap_or(0.5) as f32;
            let int = v.get("integument_vis").and_then(Value::as_f64).unwrap_or(0.15) as f32;
            let explode = v.get("explode").and_then(Value::as_f64).unwrap_or(0.5) as f32;
            let in_pos = [[0.0f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 2.0, 0.0]];
            let in_nrm = [[0.0f32, 1.0, 0.0]; 3];
            let tags = [
                ZoologicalAnatomicalLayer::Osteology,
                ZoologicalAnatomicalLayer::MyologyAndTendons,
                ZoologicalAnatomicalLayer::ExternalIntegument,
            ];
            let mut out_pos = [[0.0f32; 3]; 3];
            let mut out_alpha = [0.0f32; 3];
            let n = evaluate_anatomical_layer_peel_zero_heap(
                &in_pos, &in_nrm, &tags, ost, myo, int, explode, &mut out_pos, &mut out_alpha,
            );
            Ok(json!({
                "status": "ok",
                "op": "layer_peel",
                "evaluated_vertices": n,
                "displaced_positions": out_pos,
                "layer_alphas": out_alpha,
            })
            .to_string())
        }
        other => Err(McpSystemError::ExecutionFailed(format!(
            "Unsupported living_world_anatomy op: {other}"
        ))),
    }
}

fn parse_vec3(v: &Value, key: &str, default: [f32; 3]) -> [f32; 3] {
    let Some(arr) = v.get(key).and_then(Value::as_array) else {
        return default;
    };
    if arr.len() < 3 {
        return default;
    }
    [
        arr[0].as_f64().unwrap_or(default[0] as f64) as f32,
        arr[1].as_f64().unwrap_or(default[1] as f64) as f32,
        arr[2].as_f64().unwrap_or(default[2] as f64) as f32,
    ]
}

/// `cosmic_coordinates` — Omniversal Coordinate System (OCS v2.2.0): celestial profiles,
/// FLRW redshift cosmology, pulsar dispersion delay, warp velocity, HEALPix, and USRI realms.
pub fn cosmic_coordinates(raw_args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(raw_args)?;
    let op = v.get("op").and_then(Value::as_str).unwrap_or("celestial_profile");

    let allowed: &[&str] = match op {
        "celestial_profile" => &["op", "body"],
        "redshift" => &["op", "z"],
        "pulsar_dispersion" => &["op", "dm_pc_cm3", "freq_hz"],
        "warp" => &["op", "warp_factor", "scale"],
        "healpix" => &["op", "x", "y", "z", "nside"],
        "usri_realm" => &["op", "realm"],
        _ => &[],
    };
    if v.as_object().is_some_and(|map| map.keys().any(|key| !allowed.contains(&key.as_str()))) {
        return Err(McpSystemError::ExecutionFailed(format!("Unknown argument for cosmic_coordinates({op}); HEALPix uses integer z, not total_z")));
    }
    for key in ["z", "dm_pc_cm3", "freq_hz", "warp_factor", "x", "y", "nside"] {
        if v.get(key).is_some_and(|value| !value.is_number()) {
            return Err(McpSystemError::InvalidParameters);
        }
    }
    for key in ["body", "scale", "realm"] {
        if v.get(key).is_some_and(|value| !value.is_string()) {
            return Err(McpSystemError::InvalidParameters);
        }
    }
    if v.get("scale").is_some_and(|scale| !matches!(scale.as_str(), Some("tos" | "tng"))) {
        return Err(McpSystemError::InvalidParameters);
    }

    match op {
        "celestial_profile" => {
            let name = v.get("body").and_then(Value::as_str).unwrap_or("earth");
            let Some(profile) = body_profile_by_name(name) else {
                return Err(McpSystemError::ExecutionFailed(format!(
                    "Unknown celestial body: {name}"
                )));
            };
            Ok(json!({
                "status": "ok",
                "op": "celestial_profile",
                "body": profile.name,
                "usri": profile.usri,
                "equatorial_radius_m": profile.equatorial_radius_m,
                "rotation_period_s": profile.rotation_period_s,
                "mass_kg": profile.mass_kg,
                "surface_gravity_m_s2": profile.surface_gravity(),
            })
            .to_string())
        }
        "redshift" => {
            let z = v.get("z").and_then(Value::as_f64).unwrap_or(1.0);
            let obs = observe_redshift(z);
            Ok(json!({
                "status": "ok",
                "op": "redshift",
                "z": obs.z,
                "comoving_distance_mpc": obs.comoving_distance_mpc,
                "physical_distance_mpc": obs.physical_distance_mpc,
                "recession_velocity_km_s": obs.recession_velocity_km_s,
            })
            .to_string())
        }
        "pulsar_dispersion" => {
            let dm = v.get("dm_pc_cm3").and_then(Value::as_f64).unwrap_or(50.0);
            let freq = v.get("freq_hz").and_then(Value::as_f64).unwrap_or(1.4e9);
            let delay = pulsar_dispersion_delay_s(dm, freq)
                .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
            Ok(json!({
                "status": "ok",
                "op": "pulsar_dispersion",
                "dm_pc_cm3": dm,
                "freq_hz": freq,
                "delay_seconds": delay,
            })
            .to_string())
        }
        "warp" => {
            let factor = v.get("warp_factor").and_then(Value::as_f64).unwrap_or(5.0);
            let scale = match v.get("scale").and_then(Value::as_str).unwrap_or("tng") {
                "tos" => WarpScale::Tos,
                _ => WarpScale::Tng,
            };
            Ok(json!({
                "status": "ok",
                "op": "warp",
                "warp_factor": factor,
                "velocity_m_s": warp_velocity(factor, scale),
                "cochrane_multiple_c": cochrane_units(factor, scale),
            })
            .to_string())
        }
        "healpix" => {
            if v.get("nside").is_some_and(|value| value.as_u64().is_none()) {
                return Err(McpSystemError::InvalidParameters);
            }
            for key in ["x", "y", "z"] {
                if v.get(key).is_some_and(|value| value.as_i64().is_none()) {
                    return Err(McpSystemError::InvalidParameters);
                }
            }
            let x = v.get("x").and_then(Value::as_i64).unwrap_or(1);
            let y = v.get("y").and_then(Value::as_i64).unwrap_or(2);
            let z = v.get("z").and_then(Value::as_i64).unwrap_or(3);
            let nside = v.get("nside").and_then(Value::as_u64).unwrap_or(64);
            if nside == 0 || !nside.is_power_of_two() || nside > (1 << 29) {
                return Err(McpSystemError::InvalidParameters);
            }
            let idx = compute_healpix_index([x, y, z], nside);
            Ok(json!({
                "status": "ok",
                "op": "healpix",
                "nside": nside,
                "healpix_index": idx,
            })
            .to_string())
        }
        "usri_realm" => {
            let realm = v.get("realm").and_then(Value::as_str).unwrap_or("g_coord_terrestrial");
            let mapped = g_coord_realm_to_realm_class(realm).map(|r| format!("{r:?}"));
            Ok(json!({
                "status": "ok",
                "op": "usri_realm",
                "realm": realm,
                "realm_class": mapped,
            })
            .to_string())
        }
        other => Err(McpSystemError::ExecutionFailed(format!(
            "Unsupported cosmic_coordinates op: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healpix_contract_uses_integer_z_and_rejects_ignored_aliases() {
        let valid: Value = serde_json::from_str(&cosmic_coordinates(br#"{"op":"healpix","x":1,"y":2,"z":900,"nside":64}"#).unwrap()).unwrap();
        assert_eq!(valid["status"], "ok");
        assert!(cosmic_coordinates(br#"{"op":"healpix","total_z":900}"#).is_err());
        assert!(cosmic_coordinates(br#"{"op":"healpix","z":1.5}"#).is_err());
        assert!(cosmic_coordinates(br#"{"op":"healpix","nside":0}"#).is_err());
        assert!(cosmic_coordinates(br#"{"op":"healpix","nside":3}"#).is_err());
        let schema: Value = serde_json::from_str(crate::registry::stable_mcp_tools().iter().find(|t| t.name == "cosmic_coordinates").unwrap().input_schema).unwrap();
        assert!(schema["properties"].get("total_z").is_none());
        assert_eq!(schema["allOf"][0]["then"]["properties"]["z"]["type"], "integer");
    }

    #[test]
    fn test_etalon_vibe_sdk_mcp_tool() {
        let out = etalon_vibe_sdk(b"{}").unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["status"], "ok");
        assert_eq!(v["provider_count"], 11);
    }

    #[test]
    fn test_living_world_anatomy_and_cosmic_mcp_tools() {
        let out = living_world_anatomy(br#"{"op":"catalog","scale":4}"#).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["status"], "ok");
        assert!(v["pin_count"].as_u64().unwrap() > 0);

        let peel = living_world_anatomy(br#"{"op":"layer_peel","explode":0.5}"#).unwrap();
        let pv: Value = serde_json::from_str(&peel).unwrap();
        assert_eq!(pv["evaluated_vertices"], 3);

        let cos = cosmic_coordinates(br#"{"op":"redshift","z":2.0}"#).unwrap();
        let cv: Value = serde_json::from_str(&cos).unwrap();
        assert_eq!(cv["status"], "ok");
        assert!(cv["comoving_distance_mpc"].as_f64().unwrap() > 0.0);
    }
}
