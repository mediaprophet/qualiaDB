//! Catalog of host `Family.method` invoke ids.
//!
//! Vibe is the language; Poet (and other hosts) supply capabilities.
//! Dotted calls such as `Animation.orbit_spin(t)` resolve against this
//! catalog at compile time and lower to `Host::capability_invoke`.
//! Unknown `Family.method` names fail closed.

mod cosmic;
mod ids;
mod local;
mod suggest;

pub use ids::{ALL_INVOKE_IDS, INVOKE_ID_COUNT};
pub use local::{invoke_local, payload_from_args, sample_to_value};
pub use suggest::did_you_mean;

use crate::animation::presets::list_all_presets;
use crate::animation::AnimationFamily;
use crate::value::Value;

/// Resolve a dotted path to a canonical invoke id.
///
/// `Animation.evaluate_preset` is already canonical.
/// `Animation.orbit_spin` is a preset alias of `Animation.evaluate_preset`.
pub fn canonical_id(path: &str) -> Option<&'static str> {
    if let Some(id) = lookup_static(path) {
        return Some(id);
    }
    if animation_preset(path).is_some() {
        return Some("Animation.evaluate_preset");
    }
    None
}

pub fn is_known(path: &str) -> bool {
    canonical_id(path).is_some()
}

/// `Family` grant: `using Animation;` covers every `Animation.*` catalog id
/// and every animation preset alias.
fn utf8_eq_ignore_case(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

pub fn family_of(path: &str) -> Option<&str> {
    let dot = path.find('.')?;
    Some(&path[..dot])
}

pub fn granted_covers(granted: &[&str], path: &str) -> bool {
    if let Some(id) = canonical_id(path) {
        if granted.iter().any(|g| *g == id || *g == path) {
            return true;
        }
        if let Some(fam) = family_of(id) {
            if granted.iter().any(|g| utf8_eq_ignore_case(g, fam)) {
                return true;
            }
        }
        if let Some(fam) = family_of(path) {
            if granted.iter().any(|g| utf8_eq_ignore_case(g, fam)) {
                return true;
            }
        }
        return false;
    }
    if looks_like_catalog_path(path) {
        return false;
    }
    true
}

/// Known catalog family + dotted method, e.g. `HID.poll`.
/// User enums such as `Shape.Circle` are **not** catalog paths.
pub fn looks_like_catalog_path(path: &str) -> bool {
    if animation_preset(path).is_some() {
        return true;
    }
    let Some(fam) = family_of(path) else {
        return false;
    };
    let prefix = format!("{fam}.");
    ALL_INVOKE_IDS.iter().any(|id| id.starts_with(&prefix))
}

pub fn animation_preset(path: &str) -> Option<(&'static str, &'static str)> {
    let name = path.strip_prefix("Animation.")?;
    for info in list_all_presets() {
        if info.preset == name {
            return Some((info.family, info.preset));
        }
    }
    None
}

/// When `Animation.orbit_spin(t)` lowers to `Animation.evaluate_preset`,
/// fill `family`/`preset` on the payload so every Host sees the alias.
/// Named fields already present win.
pub fn apply_preset_alias(path: &str, payload: &mut Value) {
    let Some((family, preset)) = animation_preset(path) else {
        return;
    };
    let Value::Record(map) = payload else {
        return;
    };
    map.entry("family".into())
        .or_insert_with(|| Value::String(family.into()));
    map.entry("preset".into())
        .or_insert_with(|| Value::String(preset.into()));
}

pub fn parse_family_from_preset(family: &str) -> Option<AnimationFamily> {
    AnimationFamily::from_name(family)
}

fn lookup_static(path: &str) -> Option<&'static str> {
    ALL_INVOKE_IDS.iter().copied().find(|id| *id == path)
}

/// Expand `using Animation;` into the family grant id.
pub fn family_grant(family: &str) -> String {
    family.to_string()
}

pub fn methods_for_family(family: &str) -> Vec<&'static str> {
    let prefix = format!("{family}.");
    ALL_INVOKE_IDS
        .iter()
        .copied()
        .filter(|id| id.starts_with(&prefix))
        .collect()
}

/// Unique catalog family names, in catalog order.
pub fn families() -> Vec<&'static str> {
    let mut out = Vec::new();
    for id in ALL_INVOKE_IDS {
        if let Some(fam) = family_of(id) {
            if !out.contains(&fam) {
                out.push(fam);
            }
        }
    }
    out
}

/// One-line hover text for a catalog path or identifier.
pub fn describe(path: &str) -> String {
    if let Some((family, preset)) = animation_preset(path) {
        return format!(
            "Animation preset `{preset}` (family {family}) → `Animation.evaluate_preset`"
        );
    }
    if let Some(id) = canonical_id(path) {
        if id == "GraphAuthoring.process" || id == "N3Logic.evaluate" {
            return format!(
                "catalog `{id}` — graph host invoke (fail-closed; lease with `using {}`). Natural persons are rdfs:Class + SHACL/ShEx; owl:Thing is forbidden for persons.",
                family_of(id).unwrap_or("graph")
            );
        }
        if let Some(fam) = family_of(id) {
            return format!(
                "catalog `{id}` — {fam} host invoke (fail-closed; lease with `using {fam};`)"
            );
        }
        return format!("catalog `{id}` — host invoke (fail-closed)");
    }
    if looks_like_catalog_path(path) {
        return match did_you_mean(path) {
            Some(hint) => format!("unknown catalog path `{path}`; did you mean `{hint}`?"),
            None => format!("unknown catalog path `{path}`"),
        };
    }
    format!("ident `{path}`")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_animation_and_hid() {
        assert!(is_known("Animation.evaluate_preset"));
        assert!(is_known("HID.poll"));
        assert!(is_known("GraphDatabase.sparql"));
        assert!(is_known("GraphDatabase.volume_open"));
        assert!(is_known("GraphDatabase.volume_commit"));
        assert!(is_known("DeonticLogic.evaluate"));
        assert!(is_known("N3Logic.evaluate"));
        assert!(is_known("GraphAuthoring.process"));
        assert!(is_known("LegalLogic.compute"));
        assert!(describe("GraphAuthoring.process").contains("owl:Thing"));
    }

    #[test]
    fn preset_alias_resolves() {
        assert_eq!(
            canonical_id("Animation.orbit_spin"),
            Some("Animation.evaluate_preset")
        );
        assert!(animation_preset("Animation.orbit_spin").is_some());
    }

    #[test]
    fn preset_alias_fills_family_and_preset() {
        let mut payload = payload_from_args(&[Value::F64(0.2)], &[]);
        apply_preset_alias("Animation.glass_reveal", &mut payload);
        let Value::Record(map) = payload else {
            panic!("expected record");
        };
        assert_eq!(
            map.get("family"),
            Some(&Value::String("hud_glass_ui".into()))
        );
        assert_eq!(
            map.get("preset"),
            Some(&Value::String("glass_reveal".into()))
        );
        assert_eq!(map.get("t"), Some(&Value::F64(0.2)));
    }

    #[test]
    fn using_family_covers_methods() {
        let g = ["Animation"];
        assert!(granted_covers(&g, "Animation.evaluate_preset"));
        assert!(granted_covers(&g, "Animation.orbit_spin"));
        assert!(!granted_covers(&g, "HID.poll"));
    }

    #[test]
    fn unknown_family_method_is_catalog_shaped() {
        assert!(looks_like_catalog_path("Animation.not_a_real_preset"));
        assert!(!is_known("Animation.not_a_real_preset"));
        assert!(!looks_like_catalog_path("math.sin"));
    }

    #[test]
    fn families_include_render_and_deontic() {
        let fams = families();
        assert!(fams.contains(&"Render"));
        assert!(fams.contains(&"DeonticLogic"));
        assert!(fams.contains(&"Animation"));
    }

    #[test]
    fn describe_mentions_lease() {
        let d = describe("DeonticLogic.evaluate");
        assert!(d.contains("DeonticLogic.evaluate"));
        assert!(d.contains("using DeonticLogic"));
    }

    #[test]
    fn suggest_nearby_id() {
        let s = did_you_mean("HID.pol");
        assert!(s.is_some());
        assert!(s.unwrap().contains("HID.poll"));
    }
}
