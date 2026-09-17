//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Credential inspector panel for capability and access-control state.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Credential Inspector — shows capabilities, access control policies, conditions
// ---------------------------------------------------------------------------

/// Build the credential inspector panel — shows the current viewer's
/// capabilities, access control policies, and conditions.
///
/// See `ontologies/settings.n3` §5 (Capability Management) and §6 (Access Control).
pub fn build_credential_inspector_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-panel credential-inspector");
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 8px; \
         padding: 8px; overflow-y: auto; font-family: var(--font-mono);",
    );

    // Header
    let header = document.create_element("div").unwrap();
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "font-size: 11px; font-weight: 700; color: var(--text-primary); \
         padding-bottom: 6px; border-bottom: 1px solid var(--border-subtle);",
    );
    header.set_text_content(Some("\u{1F511} Credential Inspector"));
    wrapper.append_child(&header).unwrap();

    // Actor identity
    let actor = document.create_element("div").unwrap();
    let a_el: HtmlElement = actor.clone().dyn_into().unwrap();
    a_el.style().set_css_text(
        "font-size: 10px; color: var(--text-secondary); padding: 6px 10px; \
         background: var(--surface-panel); border-radius: var(--radius-xs);",
    );
    actor.set_text_content(Some(
        "Actor: did:qualia:timothy_charles_holborn\nType: NaturalPerson\nCircumstance: owner, non-delegable"
    ));
    let a_html: HtmlElement = actor.clone().dyn_into().unwrap();
    a_html
        .style()
        .set_property("white-space", "pre-line")
        .unwrap();
    wrapper.append_child(&actor).unwrap();

    // Capabilities section
    let cap_label = document.create_element("div").unwrap();
    let cl_el: HtmlElement = cap_label.clone().dyn_into().unwrap();
    cl_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 4px;");
    cap_label.set_text_content(Some("Capabilities:"));
    wrapper.append_child(&cap_label).unwrap();

    let capabilities = [
        (
            "selfhood:access",
            "active",
            "owner-only, non-delegable",
            "var(--accent-emerald)",
        ),
        (
            "provenance:read",
            "active",
            "full provenance graph",
            "var(--accent-emerald)",
        ),
        (
            "context-markup:read",
            "active",
            "all append scopes",
            "var(--accent-emerald)",
        ),
        (
            "context-markup:append",
            "active",
            "audience scope",
            "var(--accent-emerald)",
        ),
        (
            "checkpoint:create",
            "active",
            "all modes",
            "var(--accent-emerald)",
        ),
        (
            "checkpoint:restore",
            "active",
            "any branch",
            "var(--accent-emerald)",
        ),
        (
            "publication:distribute",
            "active",
            "with consent check",
            "var(--accent-cyan)",
        ),
        (
            "metadata:strip",
            "pending",
            "requires fiduciary",
            "var(--accent-amber)",
        ),
        (
            "crypto:key:read",
            "suspended",
            "sandbox-only",
            "var(--accent-amber)",
        ),
        (
            "agent:delegate",
            "revoked",
            "non-delegable",
            "var(--accent-red)",
        ),
    ];

    let cap_list = document.create_element("div").unwrap();
    let cl_html: HtmlElement = cap_list.clone().dyn_into().unwrap();
    cl_html.style().set_css_text(
        "display: flex; flex-direction: column; gap: 3px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 10px;",
    );

    for (name, status, constraint, color) in &capabilities {
        let cap = document.create_element("div").unwrap();
        let c_el: HtmlElement = cap.clone().dyn_into().unwrap();
        c_el.style().set_css_text(&format!(
            "padding: 4px 8px; border-radius: var(--radius-xs); \
             background: var(--surface-panel); border-left: 3px solid {}; \
             display: flex; justify-content: space-between; align-items: center; gap: 8px;",
            color
        ));

        let name_el = document.create_element("span").unwrap();
        name_el
            .set_attribute("style", "font-size: 10px; color: var(--text-primary);")
            .unwrap();
        name_el.set_text_content(Some(name));
        cap.append_child(&name_el).unwrap();

        let status_el = document.create_element("span").unwrap();
        status_el
            .set_attribute(
                "style",
                &format!(
                    "font-size: 9px; color: {}; font-weight: 700; text-transform: uppercase;",
                    color
                ),
            )
            .unwrap();
        status_el.set_text_content(Some(status));
        cap.append_child(&status_el).unwrap();

        let constraint_el = document.create_element("div").unwrap();
        constraint_el
            .set_attribute(
                "style",
                "font-size: 8px; color: var(--text-muted); flex-basis: 100%;",
            )
            .unwrap();
        constraint_el.set_text_content(Some(constraint));
        cap.append_child(&constraint_el).unwrap();

        cap_list.append_child(&cap).unwrap();
    }
    wrapper.append_child(&cap_list).unwrap();

    // Access control policies section
    let pol_label = document.create_element("div").unwrap();
    let p_el: HtmlElement = pol_label.clone().dyn_into().unwrap();
    p_el.style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 8px;");
    pol_label.set_text_content(Some("Access Control Policies:"));
    wrapper.append_child(&pol_label).unwrap();

    let policies = [
        (
            "selfhood-protection",
            "selfhood:access",
            "owner-only, non-delegable, non-transferable",
        ),
        ("provenance-read", "provenance:read", "non-anonymous"),
        (
            "context-markup-append",
            "context-markup:append",
            "from-trusted-device",
        ),
        (
            "publication-distribute",
            "publication:distribute",
            "with-mfa, consent-check",
        ),
    ];

    let pol_list = document.create_element("div").unwrap();
    let pl_html: HtmlElement = pol_list.clone().dyn_into().unwrap();
    pl_html.style().set_css_text(
        "display: flex; flex-direction: column; gap: 3px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 10px;",
    );

    for (name, required_cap, condition) in &policies {
        let pol = document.create_element("div").unwrap();
        let p_el: HtmlElement = pol.clone().dyn_into().unwrap();
        p_el.style().set_css_text(
            "padding: 4px 8px; border-radius: var(--radius-xs); \
             background: var(--surface-panel); \
             display: flex; flex-direction: column; gap: 1px;",
        );

        let name_el = document.create_element("span").unwrap();
        name_el
            .set_attribute(
                "style",
                "font-size: 10px; color: var(--text-primary); font-weight: 600;",
            )
            .unwrap();
        name_el.set_text_content(Some(name));
        pol.append_child(&name_el).unwrap();

        let req_el = document.create_element("span").unwrap();
        req_el
            .set_attribute("style", "font-size: 8px; color: var(--text-muted);")
            .unwrap();
        req_el.set_text_content(Some(&format!("requires: {}", required_cap)));
        pol.append_child(&req_el).unwrap();

        let cond_el = document.create_element("span").unwrap();
        cond_el
            .set_attribute("style", "font-size: 8px; color: var(--text-muted);")
            .unwrap();
        cond_el.set_text_content(Some(&format!("condition: {}", condition)));
        pol.append_child(&cond_el).unwrap();

        pol_list.append_child(&pol).unwrap();
    }
    wrapper.append_child(&pol_list).unwrap();

    // Honesty note
    let note = document.create_element("div").unwrap();
    let n_el: HtmlElement = note.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 6px 8px; \
         background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--accent-cyan); margin-top: 4px;",
    );
    note.set_text_content(Some(
        "\u{1F4A1} Capabilities and policies are structural mocks based on \
         ontologies/settings.n3 \u{00A7}5\u{2013}6. Live capability resolution \
         and Sentinel VM enforcement are engine wiring pending.",
    ));
    wrapper.append_child(&note).unwrap();

    wrapper
}
