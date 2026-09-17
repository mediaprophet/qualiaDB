//! Rights & Agreements tab views backed by persistent COP records.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use super::super::cop_records::{build_family_panel, CopField};
use super::super::live_invoke;

pub fn build_agreements_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-rights-panel", "agreements")
        .unwrap();
    let ledger = build_family_panel(
        document,
        "rights_agreement",
        "Live COP agreements. New records persist on the local daemon.",
        &[
            CopField {
                key: "parties",
                placeholder: "Parties (DID \u{2194} DID)",
            },
            CopField {
                key: "threshold",
                placeholder: "Threshold (e.g. 2-of-3)",
            },
            CopField {
                key: "status",
                placeholder: "Status (draft|active|pending)",
            },
        ],
    );
    ledger
        .append_child(&live_invoke::action_bar(
            document,
            &[(
                "DeonticLogic.evaluate",
                "DeonticLogic.evaluate",
                serde_json::json!({ "modality": "obligate", "body": "agreement" }),
            )],
        ))
        .unwrap();
    panel.append_child(&ledger).unwrap();
    panel
}

pub fn build_deontic_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-rights-panel", "deontic").unwrap();
    let ledger = build_family_panel(
        document,
        "rights_deontic",
        "Persistent deontic norms (OBLIGATE/PERMIT/FORBID) compiled from COP-R4.",
        &[
            CopField {
                key: "modality",
                placeholder: "OBLIGATE|PERMIT|FORBID",
            },
            CopField {
                key: "norm",
                placeholder: "Norm (role:action)",
            },
            CopField {
                key: "source",
                placeholder: "Instrument (COP-R4, COP-M1, \u{2026})",
            },
        ],
    );
    ledger
        .append_child(&live_invoke::action_bar(
            document,
            &[(
                "DeonticLogic.evaluate",
                "DeonticLogic.evaluate",
                serde_json::json!({ "modality": "obligate", "body": "role:action" }),
            )],
        ))
        .unwrap();
    panel.append_child(&ledger).unwrap();
    panel
}

pub fn build_jural_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-rights-panel", "jural").unwrap();

    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Hohfeld correlatives are structural: Right/Duty, Privilege/No-Right, Power/Liability, Immunity/Disability.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "margin-bottom: 8px; padding: 6px 8px; font-size: 10px; color: var(--text-muted); \
         font-family: var(--font-mono); background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();
    let ledger = build_family_panel(
        document,
        "rights_jural",
        "Persisted role inspections against Hohfeld positions.",
        &[
            CopField {
                key: "role",
                placeholder: "Role (principal, agent, fiduciary)",
            },
            CopField {
                key: "position",
                placeholder: "Position (Right, Duty, Power, \u{2026})",
            },
            CopField {
                key: "correlative",
                placeholder: "Unmet correlative, if any",
            },
        ],
    );
    ledger
        .append_child(&live_invoke::action_bar(
            document,
            &[(
                "LegalLogic.compute",
                "LegalLogic.compute",
                serde_json::json!({ "mode": "jural", "role": "principal" }),
            )],
        ))
        .unwrap();
    panel.append_child(&ledger).unwrap();
    panel
}

pub fn build_breach_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-rights-panel", "breach").unwrap();
    let ledger = build_family_panel(
        document,
        "rights_breach",
        "Provenance-anchored breach/defeat records. External signatures remain a field, not a fabricated proof.",
        &[
            CopField {
                key: "norm",
                placeholder: "Norm id",
            },
            CopField {
                key: "status",
                placeholder: "breached|defeated|active",
            },
            CopField {
                key: "opcode",
                placeholder: "OBLIGATE|PERMIT|FORBID",
            },
            CopField {
                key: "signature",
                placeholder: "External signature requirement / receipt",
            },
        ],
    );
    ledger
        .append_child(&live_invoke::action_bar(
            document,
            &[(
                "LegalLogic.compute",
                "LegalLogic.compute",
                serde_json::json!({ "mode": "meta_deontic", "actor": "principal", "action": "breach" }),
            )],
        ))
        .unwrap();
    panel.append_child(&ledger).unwrap();
    panel
}

pub fn build_consents_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-rights-panel", "consents")
        .unwrap();
    panel.append_child(&build_family_panel(
        document,
        "rights_consent",
        "Scoped consent grants. In-force state is the stored expiry/revocation, not a sample roster.",
        &[
            CopField {
                key: "subject",
                placeholder: "Subject DID",
            },
            CopField {
                key: "scope",
                placeholder: "Scope (e.g. health:records)",
            },
            CopField {
                key: "status",
                placeholder: "granted|revoked|expired",
            },
            CopField {
                key: "expiry",
                placeholder: "Expiry unix seconds",
            },
        ],
    ))
    .unwrap();
    panel
}
