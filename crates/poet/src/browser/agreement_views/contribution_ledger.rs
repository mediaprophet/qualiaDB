//! Contribution Ledger — persistent COP-C1 contribution records.

use web_sys::{Document, Element};

use super::super::cop_records::{build_family_panel, CopField};

pub fn build_contribution_ledger_view(document: &Document) -> Element {
    build_family_panel(
        document,
        "contribution",
        "Append-only contribution ledger. Fair value and obligation are stored fields, not generated sample hours.",
        &[
            CopField {
                key: "did",
                placeholder: "Contributor DID",
            },
            CopField {
                key: "kind",
                placeholder: "Kind (time|skill|expertise)",
            },
            CopField {
                key: "quantity",
                placeholder: "Quantity",
            },
            CopField {
                key: "fair_value",
                placeholder: "Fair value",
            },
            CopField {
                key: "obligation",
                placeholder: "Obligation cost",
            },
        ],
    )
}
