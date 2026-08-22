//! ShEx Editor — Shape Expressions grammar-based validation (P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! ShEx provides grammar-based validation without OWL's owl:Thing root
//! commitment. Unlike SHACL (which is SPARQL-based and targets existing
//! graphs), ShEx defines shapes as grammars — making it suitable for
//! natural-person ontologies where the OWL "everything is a Thing"
//! abstraction is inappropriate.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const SHAPES: &[(&str, &str, &str, &str)] = &[
    ("PersonShape", "person:Person", "3 constraints", "Valid"),
    (
        "NaturalPersonShape",
        "person:NaturalPerson",
        "5 constraints",
        "Valid",
    ),
    ("SelfhoodShape", "self:Selfhood", "4 constraints", "Valid"),
    ("GuardianShape", "grd:Guardian", "6 constraints", "Valid"),
    (
        "CareRecipientShape",
        "cs:CareRecipient",
        "7 constraints",
        "Warning",
    ),
    (
        "SocialRelationShape",
        "person:SocialRelation",
        "4 constraints",
        "Valid",
    ),
];

const CONSTRAINTS: &[(&str, &str, &str, &str)] = &[
    ("PersonShape", "person:hasName", "1..1", "xsd:string"),
    (
        "PersonShape",
        "person:hasPersonhood",
        "1..1",
        "person:Personhood",
    ),
    (
        "PersonShape",
        "person:hasSocialRelation",
        "0..*",
        "person:SocialRelation",
    ),
    (
        "NaturalPersonShape",
        "person:hasSelfhood",
        "1..1",
        "self:Selfhood",
    ),
    ("NaturalPersonShape", "person:hasName", "1..1", "xsd:string"),
    (
        "NaturalPersonShape",
        "person:dateOfBirth",
        "1..1",
        "xsd:date",
    ),
    (
        "NaturalPersonShape",
        "person:hasGuardian",
        "0..2",
        "grd:Guardian",
    ),
    (
        "NaturalPersonShape",
        "agency:hasAgentType",
        "1..1",
        "agency:NaturalAgent",
    ),
    (
        "SelfhoodShape",
        "self:hasInteriorState",
        "0..*",
        "self:InteriorState",
    ),
    ("SelfhoodShape", "self:hasQualia", "0..*", "self:Qualia"),
    (
        "SelfhoodShape",
        "self:hasSensitivity",
        "1..1",
        "self:SensitivityClass",
    ),
    ("SelfhoodShape", "self:hasConsent", "0..*", "self:Consent"),
    ("GuardianShape", "grd:hasWard", "1..*", "cs:CareRecipient"),
    (
        "GuardianShape",
        "grd:hasAuthority",
        "1..1",
        "grd:AuthorityType",
    ),
    ("GuardianShape", "grd:hasDuration", "1..1", "grd:Duration"),
    ("GuardianShape", "grd:hasScope", "1..*", "cs:CareScope"),
    ("GuardianShape", "grd:hasOversight", "0..1", "grd:Oversight"),
    (
        "GuardianShape",
        "grd:hasReportingObligation",
        "0..*",
        "obl:Obligation",
    ),
];

const SAMPLE_SHEX: &str = r#"PREFIX:     <https://qualiadb.org/schema/ui/personhood#>
PREFIX self:  <https://qualiadb.org/schema/ui/selfhood#>
PREFIX agency: <https://qualiadb.org/schema/ui/agency#>
PREFIX grd:   <https://qualiadb.org/schema/ui/guardianship#>
PREFIX xsd:   <http://www.w3.org/2001/XMLSchema#>

# PersonShape — does NOT inherit from owl:Thing.
# Shape-based: validates structure without ontological commitment.
:PersonShape {
    :hasName         xsd:string   ;   # 1..1
    :hasPersonhood   @:PersonhoodShape ;  # 1..1
    :hasSocialRelation @:SocialRelationShape ;  # 0..*
}

# NaturalPersonShape — extends PersonShape with selfhood.
# Only natural persons have selfhood (interior, non-social, non-legal).
# Legal persons do NOT match this shape.
:NaturalPersonShape @:PersonShape EXTRA :hasSelfhood {
    :hasSelfhood     @self:SelfhoodShape ;  # 1..1 — required
    :dateOfBirth     xsd:date     ;   # 1..1
    :hasGuardian     @grd:GuardianShape ? ;  # 0..2
    agency:hasAgentType  [ agency:NaturalAgent ] ;  # 1..1
}

# SelfhoodShape — protected from external claims.
# Selfhood is interior, non-social, non-legal. Law does not apply.
self:SelfhoodShape CLOSED {
    :hasInteriorState  +  ;   # 0..* (one or more)
    :hasQualia         *  ;   # 0..*
    :hasSensitivity    .  ;   # 1..1 (exactly one)
    :hasConsent        *  ;   # 0..*
}
"#;

pub fn build_shex_editor_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); \
         flex-wrap: wrap; align-items: center;",
    );

    // Paradigm note
    let note = document.create_element("span").unwrap();
    note.set_text_content(Some(
        "\u{2139} ShEx: grammar-based, no owl:Thing root \u{2014} suitable for natural-person ontologies",
    ));
    let n_el: HtmlElement = note.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "font-size: 7px; color: rgba(200, 150, 255, 0.7); font-family: var(--font-mono); \
         margin-right: 4px;",
    );
    toolbar.append_child(&note).unwrap();

    let sep1 = document.create_element("div").unwrap();
    let s1_el: HtmlElement = sep1.clone().dyn_into().unwrap();
    s1_el
        .style()
        .set_css_text("width: 1px; height: 16px; background: var(--border-subtle); margin: 0 2px;");
    toolbar.append_child(&sep1).unwrap();

    for label in &[
        "New",
        "Import ShExC",
        "Import ShExJ",
        "Validate",
        "Compile",
        "Format",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 6px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 8px; font-family: var(--font-mono);",
        );
        toolbar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&toolbar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow: hidden; display: flex;");

    // Left: shapes list + constraints
    let left = document.create_element("div").unwrap();
    let l_el: HtmlElement = left.clone().dyn_into().unwrap();
    l_el.style().set_css_text(
        "width: 280px; border-right: 1px solid var(--border-subtle); \
         overflow-y: auto; padding: 4px; flex-shrink: 0;",
    );

    // Shapes summary
    let shapes_header = document.create_element("div").unwrap();
    shapes_header.set_text_content(Some("Shape Definitions (6)"));
    let sh_el: HtmlElement = shapes_header.clone().dyn_into().unwrap();
    sh_el.style().set_css_text(
        "font-size: 8px; font-weight: 700; color: var(--text-muted); \
         font-family: var(--font-mono); text-transform: uppercase; \
         margin-bottom: 4px; padding: 0 2px;",
    );
    left.append_child(&shapes_header).unwrap();

    for (name, target, count, status) in SHAPES {
        let card = document.create_element("div").unwrap();
        let cd_el: HtmlElement = card.clone().dyn_into().unwrap();
        cd_el.style().set_css_text(
            "padding: 4px 6px; background: var(--surface-panel); border-radius: 4px; \
             margin-bottom: 3px; border: 1px solid var(--border-subtle); cursor: pointer;",
        );

        let hdr = document.create_element("div").unwrap();
        let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
        h_el.style()
            .set_css_text("display: flex; align-items: center; gap: 4px;");

        let name_div = document.create_element("div").unwrap();
        name_div.set_text_content(Some(name));
        let n_el: HtmlElement = name_div.clone().dyn_into().unwrap();
        n_el.style().set_css_text(
            "font-size: 9px; font-weight: 600; color: var(--accent-cyan); \
             font-family: var(--font-mono);",
        );
        hdr.append_child(&name_div).unwrap();

        let status_badge = document.create_element("span").unwrap();
        status_badge.set_text_content(Some(status));
        let sb_el: HtmlElement = status_badge.clone().dyn_into().unwrap();
        let sb_color = if *status == "Valid" {
            "rgba(100, 200, 100, 0.8)"
        } else {
            "rgba(255, 165, 0, 0.8)"
        };
        sb_el.style().set_css_text(&format!(
            "margin-left: auto; font-size: 7px; color: {}; font-family: var(--font-mono); \
             font-weight: 600;",
            sb_color,
        ));
        hdr.append_child(&status_badge).unwrap();
        card.append_child(&hdr).unwrap();

        let target_div = document.create_element("div").unwrap();
        target_div.set_text_content(Some(target));
        let t_el: HtmlElement = target_div.clone().dyn_into().unwrap();
        t_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
             margin-top: 1px;",
        );
        card.append_child(&target_div).unwrap();

        let count_div = document.create_element("div").unwrap();
        count_div.set_text_content(Some(count));
        let cn_el: HtmlElement = count_div.clone().dyn_into().unwrap();
        cn_el.style().set_css_text(
            "font-size: 7px; color: var(--text-secondary); font-family: var(--font-mono);",
        );
        card.append_child(&count_div).unwrap();

        left.append_child(&card).unwrap();
    }

    // Constraints table
    let const_header = document.create_element("div").unwrap();
    const_header.set_text_content(Some("Triple Constraints (18)"));
    let ch_el: HtmlElement = const_header.clone().dyn_into().unwrap();
    ch_el.style().set_css_text(
        "font-size: 8px; font-weight: 700; color: var(--text-muted); \
         font-family: var(--font-mono); text-transform: uppercase; \
         margin-top: 8px; margin-bottom: 4px; padding: 0 2px;",
    );
    left.append_child(&const_header).unwrap();

    let const_table = make_table(document, &["Shape", "Predicate", "Card.", "Type"]);
    let const_tbody = document.create_element("tbody").unwrap();
    for (shape, predicate, cardinality, dtype) in CONSTRAINTS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            shape.to_string(),
            predicate.to_string(),
            cardinality.to_string(),
            dtype.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                td_el.style().set_css_text(
                    "padding: 2px 4px; border-bottom: 1px solid var(--border-subtle); \
                     color: rgba(200, 150, 255, 0.8); font-size: 7px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else if i == 1 {
                td_el.style().set_css_text(
                    "padding: 2px 4px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 7px; font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 2px 4px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 7px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        const_tbody.append_child(&tr).unwrap();
    }
    const_table.append_child(&const_tbody).unwrap();
    left.append_child(&const_table).unwrap();

    content.append_child(&left).unwrap();

    // Right: ShExC text editor
    let editor_area = document.create_element("div").unwrap();
    let ea_el: HtmlElement = editor_area.clone().dyn_into().unwrap();
    ea_el
        .style()
        .set_css_text("flex: 1; display: flex; flex-direction: column; overflow: hidden;");

    // File tab bar
    let tab_bar = document.create_element("div").unwrap();
    let tb2_el: HtmlElement = tab_bar.clone().dyn_into().unwrap();
    tb2_el.style().set_css_text(
        "display: flex; gap: 2px; padding: 2px 4px; border-bottom: 1px solid var(--border-subtle); \
         background: var(--surface-bg);",
    );

    for (name, active) in &[
        ("personhood.shex", true),
        ("selfhood.shex", false),
        ("guardianship.shex", false),
    ] {
        let tab = document.create_element("div").unwrap();
        tab.set_text_content(Some(name));
        let t_el: HtmlElement = tab.clone().dyn_into().unwrap();
        let bg = if *active {
            "var(--surface-panel)"
        } else {
            "var(--surface-bg)"
        };
        let border = if *active {
            "rgba(200, 150, 255, 0.5)"
        } else {
            "transparent"
        };
        t_el.style().set_css_text(&format!(
            "padding: 2px 8px; font-size: 8px; font-family: var(--font-mono); \
             color: {}; background: {}; border-radius: 3px 3px 0 0; \
             border-bottom: 2px solid {}; cursor: pointer;",
            if *active {
                "var(--text-primary)"
            } else {
                "var(--text-muted)"
            },
            bg,
            border,
        ));
        tab_bar.append_child(&tab).unwrap();
    }
    ea_el.append_child(&tab_bar).unwrap();

    // Code editor
    let textarea = document.create_element("textarea").unwrap();
    let ta_el: HtmlElement = textarea.clone().dyn_into().unwrap();
    ta_el.style().set_css_text(
        "flex: 1; padding: 8px; background: var(--surface-panel); border: none; \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-primary); \
         resize: none; outline: none; line-height: 1.6; white-space: pre; \
         overflow: auto; tab-size: 2;",
    );
    textarea.set_attribute("spellcheck", "false").unwrap();
    let ta_input: web_sys::HtmlTextAreaElement = textarea.clone().dyn_into().unwrap();
    ta_input.set_value(SAMPLE_SHEX);
    ea_el.append_child(&textarea).unwrap();

    // Status bar
    let status = document.create_element("div").unwrap();
    status.set_text_content(Some(
        "Ln 30, Col 3  |  ShExC  |  3 shapes, 18 constraints  |  No owl:Thing commitment  |  UTF-8",
    ));
    let st_el: HtmlElement = status.clone().dyn_into().unwrap();
    st_el.style().set_css_text(
        "padding: 2px 8px; font-size: 7px; color: var(--text-muted); \
         font-family: var(--font-mono); border-top: 1px solid var(--border-subtle); \
         background: var(--surface-bg);",
    );
    ea_el.append_child(&status).unwrap();

    content.append_child(&editor_area).unwrap();
    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} ShEx editor requires ShEx parser + qualia_core_db shape engine.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn make_table(document: &Document, headers: &[&str]) -> Element {
    let table = document.create_element("table").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 9px;");
    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in headers {
        let th = document.create_element("th").unwrap();
        th.set_text_content(Some(h));
        let th_el: HtmlElement = th.clone().dyn_into().unwrap();
        th_el.style().set_css_text(
            "text-align: left; padding: 2px 4px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
