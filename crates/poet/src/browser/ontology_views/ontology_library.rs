//! Ontology Library — searchable library of all N3 ontologies (P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const ONTOLOGIES: &[(&str, &str, &str, &str, u32, u32, bool)] = &[
    (
        "social.n3",
        "Social relations",
        "soc",
        "CC BY-NC-ND",
        28,
        12,
        true,
    ),
    (
        "epistemics.n3",
        "Epistemic modalities",
        "epi",
        "CC BY-NC-ND",
        22,
        8,
        true,
    ),
    (
        "agency.n3",
        "Agent identity",
        "agn",
        "CC BY-NC-ND",
        15,
        6,
        true,
    ),
    (
        "personhood.n3",
        "Personhood status",
        "per",
        "CC BY-NC-ND",
        12,
        5,
        true,
    ),
    (
        "selfhood.n3",
        "Selfhood sensitivity",
        "self",
        "CC BY-NC-ND",
        10,
        4,
        true,
    ),
    (
        "provenance.n3",
        "Provenance tracking",
        "prov",
        "CC BY-NC-ND",
        18,
        7,
        true,
    ),
    (
        "values.n3",
        "Values anchor",
        "val",
        "CC BY-NC-ND",
        20,
        9,
        true,
    ),
    (
        "obligations.n3",
        "Obligations",
        "obl",
        "CC BY-NC-ND",
        25,
        10,
        true,
    ),
    (
        "duty-of-care.n3",
        "Duty of care",
        "doc",
        "CC BY-NC-ND",
        30,
        12,
        true,
    ),
    (
        "care-scope.n3",
        "Care scope",
        "cs",
        "CC BY-NC-ND",
        24,
        8,
        true,
    ),
    (
        "research.n3",
        "Research methodology",
        "res",
        "CC BY-NC-ND",
        26,
        11,
        true,
    ),
    (
        "learning-core.n3",
        "Learning",
        "learn",
        "CC BY-NC-ND",
        22,
        9,
        true,
    ),
    (
        "communications.n3",
        "Communications",
        "comm",
        "CC BY-NC-ND",
        18,
        7,
        true,
    ),
    (
        "social-connections.n3",
        "Social connections",
        "sc",
        "CC BY-NC-ND",
        20,
        8,
        true,
    ),
    (
        "spatial-3d.n3",
        "3D spatial",
        "sp3d",
        "CC BY-NC-ND",
        12,
        5,
        true,
    ),
    (
        "audio-production.n3",
        "Audio production",
        "aud",
        "CC BY-NC-ND",
        10,
        4,
        true,
    ),
    (
        "hypermedia.n3",
        "HCF spec",
        "hm",
        "CC BY-NC-ND",
        16,
        6,
        true,
    ),
    (
        "container.n3",
        "Container spec",
        "cont",
        "CC BY-NC-ND",
        20,
        8,
        true,
    ),
    (
        "document.n3",
        "Document model",
        "docm",
        "CC BY-NC-ND",
        18,
        7,
        true,
    ),
    (
        "code.n3",
        "Code artefacts",
        "code",
        "CC BY-NC-ND",
        15,
        6,
        true,
    ),
    ("settings.n3", "Settings", "set", "CC BY-NC-ND", 14, 5, true),
    (
        "investigation.n3",
        "Investigation",
        "inv",
        "CC BY-NC-ND",
        22,
        9,
        true,
    ),
    (
        "guardianship.n3",
        "Guardianship",
        "grd",
        "CC BY-NC-ND",
        16,
        6,
        true,
    ),
    (
        "agent-nomenclature.n3",
        "Agent nomenclature",
        "anm",
        "CC BY-NC-ND",
        14,
        5,
        true,
    ),
    (
        "ungrounded-generation.n3",
        "Ungrounded generation",
        "ug",
        "CC BY-NC-ND",
        10,
        4,
        true,
    ),
    (
        "adversarial-conduct.n3",
        "Adversarial conduct",
        "adv",
        "CC BY-NC-ND",
        28,
        12,
        true,
    ),
    (
        "adversarial-relational.n3",
        "Adversarial relations",
        "advr",
        "CC BY-NC-ND",
        24,
        10,
        true,
    ),
    (
        "faith-systems.n3",
        "Faith systems",
        "faith",
        "CC BY-NC-ND",
        20,
        8,
        true,
    ),
    (
        "game-design.n3",
        "Game design",
        "gd",
        "CC BY-NC-ND",
        18,
        7,
        true,
    ),
    (
        "game-world.n3",
        "Game world",
        "gw",
        "CC BY-NC-ND",
        12,
        5,
        true,
    ),
    (
        "image-editing.n3",
        "Image editing",
        "ie",
        "CC BY-NC-ND",
        8,
        3,
        true,
    ),
    (
        "interactive-hypermedia.n3",
        "Interactive hypermedia",
        "ih",
        "CC BY-NC-ND",
        7,
        3,
        true,
    ),
    (
        "learning-experience.n3",
        "Learning experience",
        "le",
        "CC BY-NC-ND",
        16,
        6,
        true,
    ),
    (
        "learning-experience-modality.n3",
        "Learning modality",
        "lem",
        "CC BY-NC-ND",
        20,
        8,
        true,
    ),
    (
        "portal-worlds.n3",
        "Portal worlds",
        "pw",
        "CC BY-NC-ND",
        6,
        2,
        true,
    ),
    (
        "presentation.n3",
        "Presentation",
        "pres",
        "CC BY-NC-ND",
        18,
        7,
        true,
    ),
    (
        "production-document.n3",
        "Production docs",
        "pdoc",
        "CC BY-NC-ND",
        22,
        9,
        true,
    ),
    (
        "production-events.n3",
        "Production events",
        "pev",
        "CC BY-NC-ND",
        8,
        3,
        true,
    ),
    (
        "video-production.n3",
        "Video production",
        "vp",
        "CC BY-NC-ND",
        6,
        2,
        true,
    ),
];

pub fn build_ontology_library_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    // Search bar
    let search_bar = document.create_element("div").unwrap();
    let sb_el: HtmlElement = search_bar.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle);",
    );

    let search_input = document.create_element("input").unwrap();
    search_input.set_attribute("type", "text").unwrap();
    search_input
        .set_attribute("placeholder", "Search 39 ontologies...")
        .unwrap();
    let si_el: HtmlElement = search_input.clone().dyn_into().unwrap();
    si_el.style().set_css_text(
        "flex: 1; padding: 4px 8px; background: var(--surface-bg); \
         border: 1px solid var(--border-medium); border-radius: 3px; \
         font-size: 9px; font-family: var(--font-mono); color: var(--text-primary);",
    );
    search_bar.append_child(&search_input).unwrap();

    let filter_btn = document.create_element("button").unwrap();
    filter_btn.set_text_content(Some("Filter"));
    let fb_el: HtmlElement = filter_btn.clone().dyn_into().unwrap();
    fb_el.style().set_css_text(
        "padding: 2px 6px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 8px; font-family: var(--font-mono);",
    );
    search_bar.append_child(&filter_btn).unwrap();
    wrapper.append_child(&search_bar).unwrap();

    // Stats bar
    let stats = document.create_element("div").unwrap();
    stats.set_text_content(Some(
        "39 ontologies | 612 classes | 248 properties | All CC BY-NC-ND",
    ));
    let st_el: HtmlElement = stats.clone().dyn_into().unwrap();
    st_el.style().set_css_text(
        "padding: 3px 8px; font-size: 8px; color: var(--text-muted); \
         font-family: var(--font-mono); border-bottom: 1px solid var(--border-subtle);",
    );
    wrapper.append_child(&stats).unwrap();

    // Ontology list
    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 4px;");

    for (file, domain, prefix, license, classes, props, loaded) in ONTOLOGIES {
        let card = document.create_element("div").unwrap();
        let cd_el: HtmlElement = card.clone().dyn_into().unwrap();
        let border = if *loaded {
            "var(--accent-cyan)"
        } else {
            "var(--border-subtle)"
        };
        cd_el.style().set_css_text(&format!(
            "padding: 6px 8px; background: var(--surface-panel); border-radius: 6px; \
             margin-bottom: 4px; border: 1px solid {}; cursor: pointer;",
            border,
        ));

        // Header row
        let hdr = document.create_element("div").unwrap();
        let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
        h_el.style()
            .set_css_text("display: flex; align-items: center; gap: 6px; margin-bottom: 2px;");

        let name_div = document.create_element("div").unwrap();
        name_div.set_text_content(Some(file));
        let n_el: HtmlElement = name_div.clone().dyn_into().unwrap();
        n_el.style().set_css_text(
            "font-size: 9px; font-weight: 600; color: var(--text-primary); \
             font-family: var(--font-mono);",
        );
        hdr.append_child(&name_div).unwrap();

        let prefix_badge = document.create_element("span").unwrap();
        prefix_badge.set_text_content(Some(prefix));
        let pb_el: HtmlElement = prefix_badge.clone().dyn_into().unwrap();
        pb_el.style().set_css_text(
            "font-size: 7px; color: var(--accent-cyan); font-family: var(--font-mono); \
             font-weight: 600; background: rgba(0, 200, 255, 0.1); padding: 1px 4px; \
             border-radius: 2px;",
        );
        hdr.append_child(&prefix_badge).unwrap();

        // Loaded indicator
        let loaded_badge = document.create_element("span").unwrap();
        loaded_badge.set_text_content(Some(if *loaded { "Loaded" } else { "Available" }));
        let lb_el: HtmlElement = loaded_badge.clone().dyn_into().unwrap();
        let lb_color = if *loaded {
            "rgba(100, 200, 100, 0.8)"
        } else {
            "var(--text-muted)"
        };
        lb_el.style().set_css_text(&format!(
            "margin-left: auto; font-size: 7px; color: {}; font-family: var(--font-mono); \
             font-weight: 600; text-transform: uppercase;",
            lb_color,
        ));
        hdr.append_child(&loaded_badge).unwrap();
        card.append_child(&hdr).unwrap();

        // Domain + stats
        let desc = document.create_element("div").unwrap();
        desc.set_text_content(Some(domain));
        let d_el: HtmlElement = desc.clone().dyn_into().unwrap();
        d_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        card.append_child(&desc).unwrap();

        let meta = document.create_element("div").unwrap();
        meta.set_text_content(Some(&format!(
            "{} classes | {} properties | {}",
            classes, props, license,
        )));
        let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
        m_el.style().set_css_text(
            "font-size: 7px; color: var(--text-secondary); font-family: var(--font-mono); \
             margin-top: 2px;",
        );
        card.append_child(&meta).unwrap();

        content.append_child(&card).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} ontology library indexes qualia-ui/ontologies/ (39 N3 files).",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
