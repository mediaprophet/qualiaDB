//! Governance specialist persistence surfaces.

use super::{ledger, CopField};
use web_sys::{Document, Element};

pub fn build_meetings_view(document: &Document) -> Element {
    ledger(
        document,
        "gov_meeting",
        "Governance meetings persist as COP records. DeonticLogic.evaluate scans the live graph.",
        &[
            CopField {
                key: "when",
                placeholder: "When",
            },
            CopField {
                key: "quorum",
                placeholder: "Quorum",
            },
        ],
        &[(
            "DeonticLogic.evaluate",
            "DeonticLogic.evaluate",
            serde_json::json!({ "party": "local", "now": 0 }),
        )],
    )
}

pub fn build_disputes_view(document: &Document) -> Element {
    ledger(
        document,
        "gov_dispute",
        "Disputes persist as COP records. LegalLogic.compute(jural) inspects Hohfeld correlatives.",
        &[
            CopField {
                key: "parties",
                placeholder: "Parties",
            },
            CopField {
                key: "status",
                placeholder: "Status (open|resolved)",
            },
        ],
        &[(
            "LegalLogic.compute",
            "LegalLogic.compute",
            serde_json::json!({ "mode": "jural", "role": "principal" }),
        )],
    )
}

pub fn build_complaints_view(document: &Document) -> Element {
    ledger(
        document,
        "gov_complaint",
        "Complaints persist as COP records. LegalLogic.compute(responsibility) is the live scan.",
        &[
            CopField {
                key: "against",
                placeholder: "Against DID",
            },
            CopField {
                key: "status",
                placeholder: "Status",
            },
        ],
        &[(
            "LegalLogic.compute",
            "LegalLogic.compute",
            serde_json::json!({ "mode": "responsibility" }),
        )],
    )
}

pub fn build_coi_view(document: &Document) -> Element {
    ledger(
        document,
        "gov_coi",
        "Conflict-of-interest declarations persist as COP records. Capacity/duress is LegalLogic.compute.",
        &[
            CopField {
                key: "declarant",
                placeholder: "Declarant DID",
            },
            CopField {
                key: "status",
                placeholder: "Status (declared|mitigated)",
            },
        ],
        &[(
            "LegalLogic.compute",
            "LegalLogic.compute",
            serde_json::json!({ "mode": "capacity" }),
        )],
    )
}

pub fn build_corrections_view(document: &Document) -> Element {
    ledger(
        document,
        "gov_correction",
        "Corrections are append-only COP records. Originals are superseded, not deleted.",
        &[
            CopField {
                key: "supersedes",
                placeholder: "Supersedes id",
            },
            CopField {
                key: "body",
                placeholder: "Correction",
            },
        ],
        &[(
            "DeonticLogic.evaluate",
            "DeonticLogic.evaluate",
            serde_json::json!({ "party": "local", "now": 0 }),
        )],
    )
}
