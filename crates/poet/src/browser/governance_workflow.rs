//! COP-backed credential, provenance, context-markup, constituency, and
//! consent workflow surfaces.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use super::cop_records::{build_family_panel, CopField};

pub fn build_credential_view(document: &Document) -> Element {
    let root = wrapper(document, "Credential Inspector");
    root.append_child(&build_family_panel(
        document,
        "capability_grant",
        "Capability grants are live COP records. Native availability is negotiated separately with CapabilityDiscovery.list.",
        &[
            CopField { key: "capability", placeholder: "Capability id" },
            CopField { key: "principal", placeholder: "Principal / agent DID" },
            CopField { key: "status", placeholder: "granted|suspended|revoked|pending" },
            CopField { key: "condition", placeholder: "Condition / scope" },
        ],
    )).unwrap();
    root.append_child(&super::live_invoke::action_bar(
        document,
        &[(
            "CapabilityDiscovery.list",
            "Refresh native capabilities",
            serde_json::json!({}),
        )],
    ))
    .unwrap();
    root
}

pub fn build_context_markup_view(document: &Document) -> Element {
    let root = wrapper(document, "Context Markup Editor");
    root.append_child(&build_family_panel(
        document,
        "context_markup",
        "Context markup persists with byte span, semantic link, scope, and provenance. It does not rewrite source text.",
        &[
            CopField { key: "document", placeholder: "Document / container id" },
            CopField { key: "markup_type", placeholder: "term|entity|claimedFact|annotation" },
            CopField { key: "byte_span", placeholder: "Byte span start:end" },
            CopField { key: "links_to", placeholder: "Linked IRI" },
            CopField { key: "append_scope", placeholder: "private|audience|public" },
            CopField { key: "provenance", placeholder: "Actor / receipt id" },
        ],
    )).unwrap();
    root
}

pub fn build_provenance_view(document: &Document) -> Element {
    let root = wrapper(document, "Provenance & Derivative Chain");
    root.append_child(&build_family_panel(
        document,
        "provenance_entry",
        "Provenance entries are persisted facts. Credits and derivative chains are derived only from records entered or produced by a capability.",
        &[
            CopField { key: "artifact", placeholder: "Artifact / checkpoint id" },
            CopField { key: "actor", placeholder: "Actor DID" },
            CopField { key: "role", placeholder: "author|editor|extractor|fiduciary" },
            CopField { key: "source", placeholder: "Source URI / artifact id" },
            CopField { key: "transformation", placeholder: "Transformation" },
            CopField { key: "derived_from", placeholder: "Parent artifact id(s)" },
            CopField { key: "confidence", placeholder: "Confidence 0..1" },
        ],
    )).unwrap();
    root
}

pub fn build_constituency_view(document: &Document) -> Element {
    let root = wrapper(document, "Constituency Manager");
    root.append_child(&build_family_panel(
        document,
        "constituency",
        "Constituencies persist as explicit parties; no people or consent states are inferred.",
        &[
            CopField {
                key: "artifact",
                placeholder: "Artifact / container id",
            },
            CopField {
                key: "iri",
                placeholder: "Party DID / constituency IRI",
            },
            CopField {
                key: "constituency_type",
                placeholder: "dataSubject|rightsHolder|stakeholder|audience|community",
            },
            CopField {
                key: "consent_required",
                placeholder: "true|false",
            },
        ],
    ))
    .unwrap();
    root.append_child(&consent_records(document)).unwrap();
    root
}

pub fn build_consent_view(document: &Document) -> Element {
    let root = wrapper(document, "Consent State");
    root.append_child(&consent_records(document)).unwrap();
    root
}

pub fn build_capability_badge_view(document: &Document) -> Element {
    build_credential_view(document)
}

fn consent_records(document: &Document) -> Element {
    build_family_panel(
        document,
        "constituency_consent",
        "Consent is live only when an explicit record exists. Missing consent remains missing, never implicitly granted.",
        &[
            CopField {
                key: "artifact",
                placeholder: "Artifact / container id",
            },
            CopField {
                key: "constituency",
                placeholder: "Constituency IRI / DID",
            },
            CopField {
                key: "status",
                placeholder: "pending|granted|denied|revoked",
            },
            CopField {
                key: "granted_by",
                placeholder: "Authorizing DID",
            },
            CopField {
                key: "expires",
                placeholder: "Expiry unix seconds (optional)",
            },
        ],
    )
}

fn wrapper(document: &Document, title: &str) -> Element {
    let root = document.create_element("div").unwrap();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display:flex;flex-direction:column;flex:1;gap:8px;padding:8px;overflow:auto;",
    );
    let heading = document.create_element("div").unwrap();
    heading.set_text_content(Some(title));
    root.append_child(&heading).unwrap();
    root
}
