//! News — announcements feed with public/private toggle (§2.7.2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("feed", "News Feed"),
    ("drafts", "Drafts"),
    ("archive", "Archive"),
];

const FEED: &[(&str, &str, &str, &str, &str)] = &[
    (
        "Ontology Spec v3 Approved",
        "2026-08-03",
        "milestone",
        "public",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Contributor Joined: contributor_03",
        "2026-08-01",
        "member",
        "public",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Funding Proposal Submitted",
        "2026-08-15",
        "funding",
        "restricted",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Alpha Release Date Set",
        "2026-08-18",
        "milestone",
        "public",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Dispute DSP-001 Resolved",
        "2026-08-17",
        "governance",
        "restricted",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "SHACL Shapes Library Published",
        "2026-08-10",
        "release",
        "public",
        "did:qualia:contributor_02",
    ),
];

const DRAFTS: &[(&str, &str, &str)] = &[
    (
        "Beta Release Preparation",
        "milestone",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "New Contributor Onboarding",
        "member",
        "did:qualia:timothy_charles_holborn",
    ),
];

pub fn build_news_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let tab_bar = build_tab_bar(document);
    wrapper.append_child(&tab_bar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    content.append_child(&build_feed_tab(document)).unwrap();

    for (i, (tab_id, _)) in TABS.iter().enumerate().skip(1) {
        let panel = build_tab_panel(document, tab_id);
        if i > 0 {
            let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
            p_el.style().set_css_text("display: none;");
        }
        content.append_child(&panel).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} news requires COP-X4 notifications + RSS/magnet export for open projects.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_tab_bar(document: &Document) -> Element {
    let tab_bar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = tab_bar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); \
         overflow-x: auto;",
    );
    for (i, (tab_id, tab_label)) in TABS.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute("data-news-tab", tab_id).unwrap();
        tab.set_text_content(Some(tab_label));
        let t_el: HtmlElement = tab.clone().dyn_into().unwrap();
        t_el.style().set_css_text(&format!(
            "padding: 4px 10px; border: none; border-bottom: 2px solid {}; \
             background: transparent; color: {}; font-size: 10px; \
             font-family: var(--font-mono); cursor: pointer; white-space: nowrap;",
            if i == 0 {
                "var(--accent-cyan)"
            } else {
                "transparent"
            },
            if i == 0 {
                "var(--text-primary)"
            } else {
                "var(--text-muted)"
            },
        ));
        tab_bar.append_child(&tab).unwrap();
    }
    tab_bar
}

fn build_feed_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-news-panel", "feed").unwrap();

    for (title, date, kind, visibility, author) in FEED {
        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        let border = match *visibility {
            "public" => "var(--border-subtle)",
            "restricted" => "rgba(255, 165, 0, 0.3)",
            _ => "var(--border-subtle)",
        };
        c_el.style().set_css_text(&format!(
            "border: 1px solid {}; border-radius: 4px; padding: 6px 8px; \
             margin-bottom: 4px; background: var(--surface-panel);",
            border,
        ));

        let hdr = document.create_element("div").unwrap();
        let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
        h_el.style()
            .set_css_text("display: flex; align-items: center; gap: 6px; margin-bottom: 2px;");

        let badge = document.create_element("span").unwrap();
        badge.set_text_content(Some(kind));
        let b_el: HtmlElement = badge.clone().dyn_into().unwrap();
        let badge_color = match *kind {
            "milestone" => "rgba(100, 200, 100, 0.8)",
            "release" => "rgba(0, 200, 255, 0.8)",
            "funding" => "rgba(255, 165, 0, 0.8)",
            "governance" => "rgba(200, 150, 255, 0.8)",
            "member" => "rgba(150, 200, 255, 0.8)",
            _ => "var(--text-muted)",
        };
        b_el.style().set_css_text(&format!(
            "font-size: 8px; color: {}; font-family: var(--font-mono); \
             text-transform: uppercase; font-weight: 600;",
            badge_color,
        ));
        hdr.append_child(&badge).unwrap();

        let t = document.create_element("span").unwrap();
        t.set_text_content(Some(title));
        let t_el: HtmlElement = t.clone().dyn_into().unwrap();
        t_el.style().set_css_text(
            "font-size: 10px; font-weight: 600; color: var(--text-primary); \
             font-family: var(--font-mono);",
        );
        hdr.append_child(&t).unwrap();
        card.append_child(&hdr).unwrap();

        let meta = document.create_element("div").unwrap();
        let vis_color = match *visibility {
            "public" => "rgba(100, 200, 100, 0.8)",
            "restricted" => "rgba(255, 165, 0, 0.8)",
            _ => "var(--text-muted)",
        };
        meta.set_text_content(Some(
            &format!("{}  |  {}  |  {}", date, author, visibility,),
        ));
        let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
        m_el.style().set_css_text(&format!(
            "font-size: 8px; color: {}; font-family: var(--font-mono);",
            vis_color,
        ));
        card.append_child(&meta).unwrap();

        panel.append_child(&card).unwrap();
    }

    let btn = document.create_element("button").unwrap();
    btn.set_text_content(Some("+ New Announcement"));
    let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
    b_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px; margin-top: 4px;",
    );
    panel.append_child(&btn).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-news-panel", tab_id).unwrap();

    match tab_id {
        "drafts" => {
            for (title, kind, author) in DRAFTS {
                let card = document.create_element("div").unwrap();
                let c_el: HtmlElement = card.clone().dyn_into().unwrap();
                c_el.style().set_css_text(
                    "border: 1px dashed var(--border-medium); border-radius: 4px; \
                     padding: 6px 8px; margin-bottom: 4px; background: var(--surface-panel);",
                );

                let hdr = document.create_element("div").unwrap();
                hdr.set_text_content(Some(&format!("[DRAFT] {} ({})", title, kind)));
                let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
                h_el.style().set_css_text(
                    "font-size: 10px; color: var(--text-secondary); \
                     font-family: var(--font-mono); margin-bottom: 2px;",
                );
                card.append_child(&hdr).unwrap();

                let meta = document.create_element("div").unwrap();
                meta.set_text_content(Some(author));
                let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
                m_el.style().set_css_text(
                    "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);",
                );
                card.append_child(&meta).unwrap();

                panel.append_child(&card).unwrap();
            }
        }
        "archive" => {
            let info = document.create_element("div").unwrap();
            info.set_text_content(Some(
                "Archived announcements are retained per project policy. \
                 Public announcements may be exported to RSS/magnet.",
            ));
            let i_el: HtmlElement = info.clone().dyn_into().unwrap();
            i_el.style().set_css_text(
                "padding: 8px; font-size: 9px; color: var(--text-muted); \
                 font-family: var(--font-mono); background: var(--surface-panel); \
                 border-radius: 4px;",
            );
            panel.append_child(&info).unwrap();
        }
        _ => {}
    }

    panel
}
