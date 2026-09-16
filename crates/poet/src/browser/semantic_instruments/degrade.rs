//! WASM capability-aware degradation (SI-09/SI-10).
//!
//! Native Q42 v3 volumes are not mmapable on the WASM / lite profile.
//! N3 definition graphs and HCF documents remain inspectable.

/// WASM / lite hosts cannot mmap native Q42 v3 volumes. Inspect N3/HCF; hold the volume.
pub const WASM_Q42_HELD: &str =
    "held / not yet — native Q42 v3 volumes are not loadable on this WASM profile; N3/HCF remain inspectable";

/// Keyboard-named degrade actions (a11y).
pub const ACTIONS: &[(&str, &str)] = &[
    ("inspect-n3", "Inspect definition N3"),
    ("inspect-hcf", "Inspect HCF document"),
    ("hold-q42-v3", "Hold native Q42 v3 volume"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WasmProfile {
    pub n3_inspectable: bool,
    pub hcf_inspectable: bool,
    pub q42_v3_loadable: bool,
}

pub fn wasm_profile() -> WasmProfile {
    WasmProfile {
        n3_inspectable: true,
        hcf_inspectable: true,
        q42_v3_loadable: false,
    }
}

pub fn q42_v3_status() -> &'static str {
    WASM_Q42_HELD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_q42_held_copy_matches() {
        assert!(WASM_Q42_HELD.contains("native Q42 v3"));
        assert!(WASM_Q42_HELD.contains("not loadable"));
        assert!(WASM_Q42_HELD.contains("N3/HCF"));
        assert!(WASM_Q42_HELD.contains("inspectable"));
        assert!(WASM_Q42_HELD.starts_with("held / not yet"));
        assert_eq!(q42_v3_status(), WASM_Q42_HELD);
    }

    #[test]
    fn n3_and_hcf_remain_inspectable_on_wasm() {
        let profile = wasm_profile();
        assert!(profile.n3_inspectable);
        assert!(profile.hcf_inspectable);
        assert!(!profile.q42_v3_loadable);
    }

    #[test]
    fn degrade_actions_are_named() {
        let ids: Vec<&str> = ACTIONS.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&"inspect-n3"));
        assert!(ids.contains(&"inspect-hcf"));
        assert!(ids.contains(&"hold-q42-v3"));
        for (id, name) in ACTIONS {
            assert!(!id.contains("Host."));
            assert!(!name.is_empty());
        }
    }
}
