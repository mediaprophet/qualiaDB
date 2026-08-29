//! Obligation Tracker — persistent per-asset obligation records.

use web_sys::{Document, Element};

use super::super::cop_records::{build_family_panel, CopField};

pub fn build_obligation_tracker_view(document: &Document) -> Element {
    build_family_panel(
        document,
        "obligation",
        "Per-asset obligation recovery. Totals are the stored recovered/outstanding values.",
        &[
            CopField {
                key: "asset",
                placeholder: "Asset name",
            },
            CopField {
                key: "license",
                placeholder: "License (COP-Permissive, CC-BY, \u{2026})",
            },
            CopField {
                key: "total",
                placeholder: "Total obligation",
            },
            CopField {
                key: "recovered",
                placeholder: "Recovered",
            },
            CopField {
                key: "outstanding",
                placeholder: "Outstanding",
            },
        ],
    )
}
