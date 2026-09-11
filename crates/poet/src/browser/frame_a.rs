//! Frame A first-arrive chrome — lexicon §7 voice, sayables-first.
//!
//! Cold-load must show **Ask graph · Keep volume · Play cell** without an
//! agent and without hunting Tool Chest categories. Capability.method stays
//! on `data-*` / muted copy. No new Host ids. No dotted `qualia.*`.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use crate::tool_chest::core::intent_bus::ActionType;

/// Lexicon §7 Frame A — first arrive (verbatim lines).
pub const FRAME_A_WHERE: &str = "You're in the studio bay.";
pub const FRAME_A_TRIO: &str = "Ask a graph, Keep a volume, Play a cell.";
pub const FRAME_A_ADVANCED: &str =
    "Advanced method names stay out of the way until you want them.";

pub const SAYABLE_ASK: &str = "Ask graph";
pub const SAYABLE_KEEP: &str = "Keep volume";
pub const SAYABLE_PLAY: &str = "Play cell";

/// Live catalog id already bound as office:graph. Not a new Capability.
pub const ASK_TOOL_ID: &str = "graph:sparql_query";

/// Always-visible cold-load banner (Research first paint included).
pub fn mount_banner(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("frame-a-banner");
    root.set_attribute("data-frame-a", "arrive").ok();
    root.set_attribute("data-recipe", "arrive").ok();
    super::surface_aspects::mark(&root, "entrance");
    paint_copy(&root, true);
    wire_sayables(&root);
    root
}

/// Compact Frame A voice for studio-bay / Catalog peer.
pub fn mount_compact(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("frame-a-compact");
    root.set_attribute("data-frame-a", "arrive").ok();
    root.set_attribute("data-recipe", "arrive").ok();
    super::surface_aspects::mark(&root, "entrance");
    paint_copy(&root, false);
    wire_sayables(&root);
    root
}

fn paint_copy(root: &Element, include_where: bool) {
    let Some(doc) = root.owner_document() else {
        return;
    };
    if include_where {
        let where_el = doc.create_element("div").unwrap();
        where_el.set_class_name("frame-a-where");
        where_el.set_text_content(Some(FRAME_A_WHERE));
        root.append_child(&where_el).ok();
    }

    let trio = doc.create_element("div").unwrap();
    trio.set_class_name("frame-a-trio");
    trio.set_text_content(Some(FRAME_A_TRIO));
    root.append_child(&trio).ok();

    let actions = doc.create_element("div").unwrap();
    actions.set_class_name("frame-a-sayables");
    actions.set_attribute("role", "group").ok();
    actions
        .set_attribute("aria-label", "Ask graph, Keep volume, Play cell")
        .ok();
    for (id, label) in [
        ("ask", SAYABLE_ASK),
        ("keep", SAYABLE_KEEP),
        ("play", SAYABLE_PLAY),
    ] {
        let btn = doc.create_element("button").unwrap();
        btn.set_attribute("type", "button").ok();
        btn.set_class_name("frame-a-sayable");
        btn.set_attribute("data-frame-a-sayable", id).ok();
        btn.set_text_content(Some(label));
        btn.set_attribute("aria-label", label).ok();
        if id == "ask" {
            btn.set_attribute("data-tool-id", ASK_TOOL_ID).ok();
            btn.set_attribute("data-capability", "GraphDatabase.sparql")
                .ok();
        }
        if id == "keep" {
            btn.set_attribute("data-capability", "GraphDatabase.volume_open")
                .ok();
        }
        actions.append_child(&btn).ok();
    }
    root.append_child(&actions).ok();

    let advanced = doc.create_element("div").unwrap();
    advanced.set_class_name("frame-a-advanced");
    advanced.set_text_content(Some(FRAME_A_ADVANCED));
    root.append_child(&advanced).ok();
}

fn wire_sayables(root: &Element) {
    let Some(doc) = root.owner_document() else {
        return;
    };
    if let Ok(buttons) = root.query_selector_all("[data-frame-a-sayable]") {
        for i in 0..buttons.length() {
            let Some(node) = buttons.get(i) else {
                continue;
            };
            let Ok(btn) = node.dyn_into::<Element>() else {
                continue;
            };
            let Some(kind) = btn.get_attribute("data-frame-a-sayable") else {
                continue;
            };
            let listen = btn.clone();
            let document = doc.clone();
            let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
                match kind.as_str() {
                    "ask" => dispatch_ask(&document),
                    "keep" => dispatch_keep(&document),
                    "play" => dispatch_play(&document),
                    _ => {}
                }
            }) as Box<dyn FnMut(web_sys::Event)>);
            listen
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .ok();
            closure.forget();
        }
    }
}

fn dispatch_ask(document: &Document) {
    super::tool_actions::dispatch(document, ASK_TOOL_ID, SAYABLE_ASK, ActionType::Query);
}

fn dispatch_keep(document: &Document) {
    super::topbar::open_save_mode_dialog(document);
}

fn dispatch_play(document: &Document) {
    if let Some(btn) = document
        .query_selector("[data-instrument-action=\"code:run\"]")
        .ok()
        .flatten()
        .or_else(|| {
            document
                .query_selector(".vibe-run-btn[data-instrument-action=\"code:run\"]")
                .ok()
                .flatten()
        })
        .or_else(|| document.query_selector(".vibe-run-btn").ok().flatten())
    {
        if let Ok(el) = btn.dyn_into::<HtmlElement>() {
            el.click();
            return;
        }
    }
    if let Some(existing) = document.get_element_by_id("frame-a-play-held") {
        existing.remove();
    }
    let note = document.create_element("div").unwrap();
    note.set_id("frame-a-play-held");
    note.set_class_name("frame-a-held-note");
    note.set_attribute("data-honesty", "unavailable").ok();
    note.set_text_content(Some(
        "held / not yet — place or open a cell, then Play cell.",
    ));
    if let Some(banner) = document.query_selector("[data-frame-a]").ok().flatten() {
        banner.append_child(&note).ok();
    }
}

pub fn primary_copy_is_sayable(label: &str) -> bool {
    matches!(label, SAYABLE_ASK | SAYABLE_KEEP | SAYABLE_PLAY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_a_copy_matches_lexicon_section_7() {
        assert_eq!(FRAME_A_WHERE, "You're in the studio bay.");
        assert_eq!(FRAME_A_TRIO, "Ask a graph, Keep a volume, Play a cell.");
        assert_eq!(
            FRAME_A_ADVANCED,
            "Advanced method names stay out of the way until you want them."
        );
        assert!(FRAME_A_TRIO.contains("Ask"));
        assert!(FRAME_A_TRIO.contains("Keep"));
        assert!(FRAME_A_TRIO.contains("Play"));
    }

    #[test]
    fn sayables_are_human_not_methods() {
        assert_eq!(SAYABLE_ASK, "Ask graph");
        assert_eq!(SAYABLE_KEEP, "Keep volume");
        assert_eq!(SAYABLE_PLAY, "Play cell");
        for s in [SAYABLE_ASK, SAYABLE_KEEP, SAYABLE_PLAY, FRAME_A_TRIO] {
            let lower = s.to_ascii_lowercase();
            assert!(!lower.contains("graphdatabase"));
            assert!(!lower.contains("capability"));
            assert!(!lower.contains("sparql"));
            assert!(!lower.contains("qualia."));
            assert!(!lower.contains("all_bound"));
        }
        assert_eq!(ASK_TOOL_ID, "graph:sparql_query");
        assert!(primary_copy_is_sayable(SAYABLE_ASK));
    }
}
