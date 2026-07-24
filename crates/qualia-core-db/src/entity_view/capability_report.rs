//! Machine-readable capability report for design process (not marketing).

use serde_json::{json, Value};

/// Static capability inventory for the entity-view kernel + host expectations.
/// Used by designers and desktop `view_capability_report`.
pub fn entity_view_capability_report() -> Value {
    json!({
        "version": 1,
        "product_frame": {
            "name": "Qualia / Webizen mindware (whole product)",
            "not": ["portal product", "single mindware module"],
            "intent": "prosthetic extension of self and human life; rights-fail-closed",
            "primary_host": "webizen-desktop (shell + studio habitat + browser + native)"
        },
        "module": {
            "crate": "qualia-core-db",
            "path": "entity_view",
            "role": "multi-observer entity projection kernel (pure, no I/O)"
        },
        "capabilities": [
            {
                "id": "entity_id",
                "status": "ready",
                "summary": "Stable EntityId from URI/DID/fragment (fnv60 / q_hash)",
                "design_use": "Same identity across flat cards, scene nodes, web locus"
            },
            {
                "id": "observer_status",
                "status": "ready",
                "summary": "Principal|Peer|Guardian|Steward|Public|Instrument|Auditor",
                "design_use": "Different people / roles see different wings of the same entities"
            },
            {
                "id": "rights_filter",
                "status": "ready",
                "summary": "Pure decide_view / filter_visible — secret fail-closed, peer offered-only",
                "design_use": "HUD must show hidden_count; never imply public=private"
            },
            {
                "id": "affordance_bits",
                "status": "ready",
                "summary": "open/share/enter/edit packed u8 on projections",
                "design_use": "Disable share on secret; dim enter when denied"
            },
            {
                "id": "bifurcated_package",
                "status": "ready",
                "summary": "Private / offered / commons wings with stable digests",
                "design_use": "Dual-layer commons vs personal representation packs"
            },
            {
                "id": "fragment_attribution",
                "status": "ready",
                "summary": "Typed edges on fragments (illustrates, narrative, groundsIn, social…)",
                "design_use": "STEM-in-story; not whole-corpus fiction/non-fiction only"
            },
            {
                "id": "projection_flat_scene",
                "status": "ready",
                "summary": "FlatCard list + SceneNodeProj layout (geo pins or grid)",
                "design_use": "Flatten / spatialize morph of same selection"
            },
            {
                "id": "presentation_level",
                "status": "ready",
                "summary": "P0–P6 morphology level enum",
                "design_use": "Theme packs bias flat vs immersive"
            },
            {
                "id": "view_host_session",
                "status": "ready",
                "summary": "App-global session (observer, selection, last projection, attention URL)",
                "design_use": "Shell, Library, Browser share one session"
            },
            {
                "id": "view_project_library",
                "status": "ready",
                "summary": "Library disk → filter → flat+scene JSON",
                "design_use": "Lived Memory entity view strip"
            },
            {
                "id": "view_project_web_locus",
                "status": "ready",
                "summary": "URL → web_locus entity card",
                "design_use": "Browser remember / select current page as entity"
            },
            {
                "id": "view_morph",
                "status": "ready",
                "summary": "flatten | spatialize | both on last projection",
                "design_use": "Morph controls on Library and eventually immersive stage"
            },
            {
                "id": "scene_entity_id",
                "status": "ready",
                "summary": "webizen-render SceneNode.entity_id (+ affordance_bits)",
                "design_use": "Pick node → select same entity as flat card"
            },
            {
                "id": "circumstance_tuple",
                "status": "partial",
                "summary": "Circumstance struct + view_set_circumstance; session field present; path-steering not fully productised",
                "design_use": "D6 job vs private chrome; environment presets (sanctuary/workplace/cafe)"
            },
            {
                "id": "fragment_store_persist",
                "status": "partial",
                "summary": "Edge model + tests; not yet WAL-backed attribution store",
                "design_use": "Show edges in mockups; persistence next"
            },
            {
                "id": "learning_morph_film_mmo",
                "status": "partial",
                "summary": "PresentationLevel + dual projection; full film/MMO stage not productised",
                "design_use": "Concept boards for learning morphs; capability Present only for P0-P2 path"
            },
            {
                "id": "library_entity_view_ui",
                "status": "ready",
                "summary": "Library panel entity-view strip: observer, P0-P6, flatten/spatialize, hidden_count, select",
                "design_use": "D3/D5/D11 live surface for polish"
            },
            {
                "id": "browser_web_locus",
                "status": "ready",
                "summary": "Browser chrome projects URL as web_locus into shared session; remember button; entity chip",
                "design_use": "D10 shared session across browser + Library"
            },
            {
                "id": "zk_enumerated_states",
                "status": "planned",
                "summary": "Privacy engine HE/DP present elsewhere; ZK not wired to entity_view states",
                "design_use": "Do not claim ZK UI yet"
            }
        ],
        "desktop_commands": [
            "view_session",
            "view_set_observer",
            "view_set_presentation_level",
            "view_project_library",
            "view_project_web_locus",
            "view_morph",
            "view_select",
            "view_clear_selection",
            "view_bifurcate_package",
            "view_capability_report",
            "view_set_circumstance"
        ],
        "surfaces": {
            "library": "entity-view strip wired (observer, morph, project, select)",
            "browser": "web_locus project on navigate + remember; entity chip",
            "shell": "shares process-wide ViewSession via view_* commands"
        },
        "design_boards_enabled": ["D1", "D2", "D3", "D4", "D5", "D10", "D11", "D12"],
        "design_boards_partial": ["D6", "D7", "D8", "D9"],
        "honesty": {
            "bifurcation_is_not_encryption": true,
            "instruments_are_not_persons": true,
            "commons_not_private_twin": true,
            "morph_is_not_full_immersive_stage": true
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_lists_ready_and_partial() {
        let v = entity_view_capability_report();
        let caps = v["capabilities"].as_array().unwrap();
        assert!(caps.len() >= 10);
        assert_eq!(v["module"]["path"], "entity_view");
        let cmds = v["desktop_commands"].as_array().unwrap();
        assert!(cmds.iter().any(|c| c == "view_capability_report"));
    }
}
