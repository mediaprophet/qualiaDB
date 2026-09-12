//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Hypermedia library browser (Poet chrome).

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Library / Lived Memory — honest held on Poet until the in-shell shelf opens
// ---------------------------------------------------------------------------

/// Poet-side library chrome. Live list/search/ingest lives in the Desktop
/// in-shell shelf (`Tools → Hypermedia Library` / `/library`, Tauri
/// `library_*` / `wellfair_*_library`). This view does not invent Host
/// `Library.*` methods and does not pretend placeholder counts are a shelf.
pub fn build_library_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display:flex;flex-direction:column;flex:1;min-height:12rem;gap:10px;padding:8px;",
    );
    wrapper
        .set_attribute("data-surface", "library")
        .ok();
    wrapper
        .set_attribute("data-honesty", "held")
        .ok();

    let eyebrow = document.create_element("div").unwrap();
    eyebrow.set_class_name("vibe-toolbar");
    let eyebrow_el: HtmlElement = eyebrow.clone().dyn_into().unwrap();
    eyebrow_el.style().set_css_text(
        "font-size:10px;font-weight:800;letter-spacing:.1em;text-transform:uppercase;color:var(--qualia-accent, #7dd3fc);",
    );
    eyebrow.set_text_content(Some("Hypermedia Library"));
    wrapper.append_child(&eyebrow).unwrap();

    let title = document.create_element("h1").unwrap();
    title.set_text_content(Some("Lived Memory"));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("margin:0;font-size:1.15rem;letter-spacing:-.03em;");
    wrapper.append_child(&title).unwrap();

    let gate = document.create_element("div").unwrap();
    gate.set_class_name("lexicon-held-gate");
    gate.set_attribute("data-gate", "held").ok();
    gate.set_attribute("data-honesty", "held").ok();
    let gate_el: HtmlElement = gate.clone().dyn_into().unwrap();
    gate_el.style().set_css_text(
        "padding:12px 14px;border:1px dashed var(--border-subtle, #334);border-radius:10px;background:rgba(127,127,127,.06);",
    );

    let label = document.create_element("div").unwrap();
    label.set_class_name("lexicon-held-label");
    label.set_text_content(Some("held / not yet"));
    let label_el: HtmlElement = label.clone().dyn_into().unwrap();
    label_el
        .style()
        .set_css_text("font-size:11px;font-weight:800;margin-bottom:6px;");
    gate.append_child(&label).unwrap();

    let why = document.create_element("p").unwrap();
    why.set_class_name("held-bind-note");
    why.set_text_content(Some(
        "held / not yet — Poet does not invent Library.* Host methods. Open Tools \u{2192} Hypermedia Library (or /library) for the live desktop shelf (library_list / library_search / wellfair_*_library).",
    ));
    let why_el: HtmlElement = why.clone().dyn_into().unwrap();
    why_el.style().set_css_text(
        "margin:0;font-size:11px;line-height:1.45;color:var(--text-muted, #8b8178);",
    );
    gate.append_child(&why).unwrap();
    wrapper.append_child(&gate).unwrap();

    wrapper
}

#[cfg(test)]
mod tests {
    #[test]
    fn poet_library_copy_avoids_unavailable_and_fake_counts() {
        let src = include_str!("library.rs");
        let impl_src = src.split("#[cfg(test)]").next().unwrap_or(src);
        let folded = impl_src.to_ascii_lowercase();
        assert!(
            !folded.contains("unavailable"),
            "Poet library chrome must not say unavailable"
        );
        assert!(
            folded.contains("held / not yet"),
            "Poet library must use held / not yet"
        );
        assert!(
            !impl_src.contains("\"142\""),
            "do not ship placeholder document counts"
        );
    }
}
