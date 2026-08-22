//! Discussion — threaded comments on work items / projects.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const THREADS: &[(&str, &str, &str, &str, &[(&str, &str, &str)])] = &[
    // (author, timestamp, body, parent_id, &[(reply_author, reply_timestamp, reply_body)])
    (
        "did:qualia:timothy_charles_holborn",
        "2026-08-01T10:00:00Z",
        "Should we use N3 or Turtle for the ontology definitions?",
        "none",
        &[
            (
                "did:qualia:researcher_01",
                "2026-08-01T11:00:00Z",
                "N3 \u{2014} it supports rules which we need for deontic evaluation.",
            ),
            (
                "did:qualia:reviewer_02",
                "2026-08-01T12:00:00Z",
                "Agreed. N3 also has built-in reasoning support in QualiaDB.",
            ),
        ],
    ),
    (
        "did:qualia:researcher_01",
        "2026-08-05T09:00:00Z",
        "The SHACL shapes need revision \u{2014} missing sh:minCount on some properties.",
        "none",
        &[(
            "did:qualia:timothy_charles_holborn",
            "2026-08-05T10:00:00Z",
            "Can you file a work item for this?",
        )],
    ),
    (
        "did:qualia:reviewer_02",
        "2026-08-10T14:00:00Z",
        "Benchmark results look promising. Suggest publishing to Commons.",
        "none",
        &[],
    ),
];

pub fn build_discussion_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let scroll = document.create_element("div").unwrap();
    let s_el: HtmlElement = scroll.clone().dyn_into().unwrap();
    s_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 4px 8px;");

    for (author, timestamp, body, _parent, replies) in THREADS {
        let comment = build_comment(document, author, timestamp, body, 0);
        scroll.append_child(&comment).unwrap();

        for (r_author, r_timestamp, r_body) in *replies {
            let reply = build_comment(document, r_author, r_timestamp, r_body, 1);
            scroll.append_child(&reply).unwrap();
        }
    }

    wrapper.append_child(&scroll).unwrap();

    // Compose bar
    let compose = document.create_element("div").unwrap();
    let c_el: HtmlElement = compose.clone().dyn_into().unwrap();
    c_el.style().set_css_text(
        "display: flex; gap: 6px; padding: 6px 8px; \
         border-top: 1px solid var(--border-subtle);",
    );

    let input = document.create_element("textarea").unwrap();
    let i_el: HtmlElement = input.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "flex: 1; padding: 4px 6px; background: var(--surface-panel); \
         border: 1px solid var(--border-medium); border-radius: 3px; \
         color: var(--text-primary); font-size: 11px; resize: none; height: 32px;",
    );
    input
        .clone()
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .unwrap()
        .set_placeholder("Add a comment...");
    compose.append_child(&input).unwrap();

    let send_btn = document.create_element("button").unwrap();
    send_btn.set_text_content(Some("Post"));
    let sb_el: HtmlElement = send_btn.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
             background: var(--accent-cyan); color: var(--background-primary); \
             border-radius: 3px; cursor: pointer; font-size: 10px; font-weight: 600;",
    );
    compose.append_child(&send_btn).unwrap();

    wrapper.append_child(&compose).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} discussion requires COP-X1 Comment/DiscussionThread engine command.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_comment(
    document: &Document,
    author: &str,
    timestamp: &str,
    body: &str,
    depth: usize,
) -> Element {
    let comment = document.create_element("div").unwrap();
    let c_el: HtmlElement = comment.clone().dyn_into().unwrap();
    let indent = depth * 24;
    c_el.style().set_css_text(&format!(
        "padding: 6px 8px; margin-left: {}px; margin-bottom: 4px; \
         border-left: 2px solid var(--border-subtle); \
         background: var(--surface-panel); border-radius: 0 4px 4px 0;",
        indent
    ));

    let header = document.create_element("div").unwrap();
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style()
        .set_css_text("display: flex; gap: 6px; margin-bottom: 3px;");

    let who = document.create_element("span").unwrap();
    who.set_text_content(Some(author));
    let w_el: HtmlElement = who.clone().dyn_into().unwrap();
    w_el.style().set_css_text(
        "font-size: 10px; color: var(--accent-cyan); \
             font-family: var(--font-mono);",
    );
    header.append_child(&who).unwrap();

    let when = document.create_element("span").unwrap();
    when.set_text_content(Some(timestamp));
    let wh_el: HtmlElement = when.clone().dyn_into().unwrap();
    wh_el
        .style()
        .set_css_text("font-size: 9px; color: var(--text-muted);");
    header.append_child(&when).unwrap();
    comment.append_child(&header).unwrap();

    let text = document.create_element("div").unwrap();
    text.set_text_content(Some(body));
    let t_el: HtmlElement = text.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("font-size: 11px; color: var(--text-primary);");
    comment.append_child(&text).unwrap();

    comment
}
