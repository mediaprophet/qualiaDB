//! Poet manufacturing surface for semantic instruments (SI-09).
//!
//! Full visual editors are later. This module owns **held/not-yet copy**,
//! Demo labelling, layer-distinct manufacture, inspect-without-activate,
//! and WASM Q42 degrade. Dispatch remains entry-point names, never new Host IDs.

pub mod a11y;
pub mod bay;
pub mod capability;
pub mod degrade;
pub mod deps;
pub mod drag;
pub mod fixture;
pub mod flow;
pub mod graph;
pub mod graph_keys;
pub mod inspect;
pub mod manufacture;
pub mod manufacture_keys;
pub mod manufacture_panel;
pub mod manifest_edit;
pub mod persist;
pub mod receipts;
pub mod round_trip;
pub mod shapes;
pub mod shapes_canvas;
pub mod undo;
pub mod undo_keys;
pub mod validate;
pub mod version;
pub mod walkthrough;

pub use bay::build_instrument_bay;
pub use capability::build_capability_view;
pub use degrade::{q42_v3_status, wasm_profile, WasmProfile, WASM_Q42_HELD};
pub use deps::build_deps_view;
pub use fixture::build_fixture_view;
pub use flow::{build_flow_view, flow_for_entry, FlowKind, FlowNode};
pub use graph::build_graph_view;
pub use graph_keys::GraphFocus;
pub use inspect::{
    activate_requires_closed, collect_without_activate, record_run, revoke_keeps_receipts,
    InspectState, ACTIONS, LIFECYCLE_CHIPS,
};
pub use manufacture::{can_publish, next_action, Draft, Layer, LAYERS, HELD_INVALID_PUBLISH};
pub use manufacture_keys::{keyboard_only_author_to_publish, ManufactureFocus, ManufactureKey};
pub use manufacture_panel::build_manufacture_panel;
pub use manifest_edit::build_manifest_view;
pub use receipts::build_receipts_view;
pub use round_trip::{loss_reasons, source_visual_round_trip_ok};
pub use walkthrough::{author_to_publish, collect_to_receipt_walkthrough};
pub use shapes::{build_shapes_view, input_constraints, living_safe_guard, output_constraints};
pub use shapes_canvas::build_shapes_canvas;
pub use validate::build_validate_view;
pub use version::build_version_view;

pub const HELD_OPEN_PACK: &str = "held / not yet — open a pack or seed demos";
pub const CHIP_DEMO: &str = "Demo";
pub const CHIP_REFERENCE: &str = "Reference";

/// One catalogue card. Artwork is a handle, not proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstrumentCard {
    pub slug: &'static str,
    pub name: &'static str,
    pub category_chip: &'static str,
    pub entry_point: &'static str,
    pub accessible_text: &'static str,
}

pub fn seed_cards() -> &'static [InstrumentCard] {
    &[
        InstrumentCard {
            slug: "unit-convert",
            name: "Unit conversion (demo)",
            category_chip: CHIP_DEMO,
            entry_point: "assess",
            accessible_text: "Unit conversion demo instrument",
        },
        InstrumentCard {
            slug: "community-water",
            name: "Community water assessment (demo)",
            category_chip: CHIP_DEMO,
            entry_point: "assess",
            accessible_text: "Community water assessment demo instrument",
        },
        InstrumentCard {
            slug: "community-infra-rpl",
            name: "Community infrastructure role (demo)",
            category_chip: CHIP_DEMO,
            entry_point: "recognise",
            accessible_text: "Community infrastructure role demo instrument",
        },
    ]
}

/// Two-flag wrapper used by catalogue chrome: invalid or unlabelled cannot publish.
pub fn can_publish_pack(valid: bool, labelled: bool) -> Result<(), &'static str> {
    can_publish(&Draft {
        source_ok: valid,
        graph_ok: valid,
        executable_ok: valid,
        labelled,
        round_trip_loss: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_cards_are_labelled_demo_and_use_entry_points() {
        for card in seed_cards() {
            assert_eq!(card.category_chip, CHIP_DEMO);
            assert!(card.entry_point == "assess" || card.entry_point == "recognise");
            assert!(!card.accessible_text.is_empty());
            assert!(!card.slug.contains("Host."));
        }
    }

    #[test]
    fn invalid_pack_cannot_publish() {
        assert_eq!(can_publish_pack(false, true), Err(HELD_INVALID_PUBLISH));
        assert!(can_publish_pack(true, true).is_ok());
        assert!(can_publish_pack(true, false).is_err());
    }

    #[test]
    fn chips_include_demo_reference_and_lifecycle() {
        assert_eq!(CHIP_DEMO, "Demo");
        assert_eq!(CHIP_REFERENCE, "Reference");
        assert_eq!(LIFECYCLE_CHIPS.len(), 6);
    }
}
