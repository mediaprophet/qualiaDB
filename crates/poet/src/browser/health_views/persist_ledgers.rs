//! Remaining Health record kinds on the COP ledger.

use web_sys::{Document, Element};

use super::super::cop_records::{build_family_panel, CopField};

fn wrap(document: &Document, child: Element) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.append_child(&child).unwrap();
    wrapper
}

fn panel(
    document: &Document,
    family: &'static str,
    heading: &str,
    fields: &'static [CopField],
) -> Element {
    wrap(
        document,
        build_family_panel(document, family, heading, fields),
    )
}

pub fn build_mental_wellbeing_view(document: &Document) -> Element {
    panel(
        document,
        "health_note",
        "Mental wellbeing notes you write. Scores are not fabricated.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (wellbeing)",
            },
            CopField {
                key: "instrument",
                placeholder: "Instrument (if any)",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity (classified)",
            },
        ],
    )
}

pub fn build_therapy_notes_view(document: &Document) -> Element {
    panel(
        document,
        "health_note",
        "Therapy notes are classified. Sanctuary-respecting; no sample notes.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (therapy)",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity (classified)",
            },
        ],
    )
}

pub fn build_sleep_view(document: &Document) -> Element {
    panel(
        document,
        "health_activity",
        "Sleep entries you record.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (sleep)",
            },
            CopField {
                key: "hours",
                placeholder: "Hours",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity",
            },
        ],
    )
}

pub fn build_diet_view(document: &Document) -> Element {
    panel(
        document,
        "health_activity",
        "Diet entries you record.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (diet)",
            },
            CopField {
                key: "note",
                placeholder: "Note",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity",
            },
        ],
    )
}

pub fn build_physical_activity_view(document: &Document) -> Element {
    panel(
        document,
        "health_activity",
        "Activity entries you record.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (activity)",
            },
            CopField {
                key: "minutes",
                placeholder: "Minutes",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity",
            },
        ],
    )
}

pub fn build_immunizations_view(document: &Document) -> Element {
    panel(
        document,
        "health_report",
        "Immunization records you transcribe.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (immunization)",
            },
            CopField {
                key: "date",
                placeholder: "Date",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity",
            },
        ],
    )
}

pub fn build_procedures_view(document: &Document) -> Element {
    panel(
        document,
        "health_report",
        "Procedure records you transcribe.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (procedure)",
            },
            CopField {
                key: "date",
                placeholder: "Date",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity",
            },
        ],
    )
}

pub fn build_family_history_view(document: &Document) -> Element {
    panel(
        document,
        "health_note",
        "Family history notes. Relatives are Principals, not owl:Thing.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (family_history)",
            },
            CopField {
                key: "relation",
                placeholder: "Relation",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity",
            },
        ],
    )
}

pub fn build_hypotheses_view(document: &Document) -> Element {
    panel(
        document,
        "health_note",
        "Hypotheses you author. DIAG engines are unbound until invoked with entered evidence.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (hypothesis)",
            },
            CopField {
                key: "status",
                placeholder: "Status (draft|disclosed)",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity (classified)",
            },
        ],
    )
}

pub fn build_biometrics_view(document: &Document) -> Element {
    panel(
        document,
        "health_vital",
        "Biometric readings you record. ZK proofs are unbound until a session is registered.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (biometric)",
            },
            CopField {
                key: "metric",
                placeholder: "Metric",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity (classified)",
            },
        ],
    )
}

pub fn build_welfare_support_view(document: &Document) -> Element {
    panel(
        document,
        "health_note",
        "Welfare support records.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (welfare)",
            },
            CopField {
                key: "status",
                placeholder: "Status",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity",
            },
        ],
    )
}

pub fn build_life_records_view(document: &Document) -> Element {
    panel(
        document,
        "health_note",
        "Life records you author.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (life)",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity",
            },
        ],
    )
}

pub fn build_authority_attestations_view(document: &Document) -> Element {
    panel(
        document,
        "health_attestation",
        "Authority attestations. Signing is unbound until a key vault session is unlocked.",
        &[
            CopField {
                key: "authority",
                placeholder: "Authority DID",
            },
            CopField {
                key: "status",
                placeholder: "Status (draft|attested)",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity",
            },
        ],
    )
}
