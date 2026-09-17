//! Manufacture chrome (SI-09). DOM for the existing layer state machine.
//!
//! Source, graph, and executable stay distinct. Dispatch is assess/recognise
//! only — never Host.*. Empty drafts name the next action, never "broken".

use super::manufacture::{
    can_publish, next_action, Draft, Layer, LAYERS, HELD_INVALID_PUBLISH, HELD_ROUND_TRIP,
    NEXT_PUBLISH,
};
use super::manufacture_keys::{apply_key, classify, ManufactureFocus};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, KeyboardEvent};

const REGION_LABEL: &str = "manufacture layers";
const TITLE: &str = "Manufacture";
const LEDE: &str = "Source, graph, and executable stay distinct. Invalid packs cannot publish.";
const LABELLED_ARIA: &str = "Keep Demo/Reference labelled";

/// Toggle a layer. Round-trip loss is source without graph (Studio parity).
pub fn toggle_layer(draft: &mut Draft, layer: Layer) {
    match layer {
        Layer::Source => draft.source_ok = !draft.source_ok,
        Layer::Graph => draft.graph_ok = !draft.graph_ok,
        Layer::Executable => draft.executable_ok = !draft.executable_ok,
    }
    draft.round_trip_loss = draft.source_ok && !draft.graph_ok;
}

fn pressed(ok: bool) -> &'static str {
    if ok {
        "true"
    } else {
        "false"
    }
}

fn publish_label(draft: &Draft) -> &'static str {
    if can_publish(draft).is_ok() {
        NEXT_PUBLISH
    } else {
        HELD_INVALID_PUBLISH
    }
}

/// Manufacture region: layer toggles, Demo/Reference label, next-action status, publish.
pub fn build_manufacture_panel(document: &Document) -> Element {
    let draft = Rc::new(RefCell::new(Draft::default()));
    let current = *draft.borrow();

    let root = document.create_element("div").unwrap();
    root.set_class_name("instrument-manufacture-panel");
    root.set_attribute("data-manufacture-panel", "1").ok();
    root.set_attribute("role", "region").ok();
    root.set_attribute("aria-label", REGION_LABEL).ok();

    let title = document.create_element("div").unwrap();
    title.set_class_name("lexicon-bay-title");
    title.set_text_content(Some(TITLE));
    root.append_child(&title).unwrap();

    let lede = document.create_element("p").unwrap();
    lede.set_class_name("lexicon-bay-lede");
    lede.set_text_content(Some(LEDE));
    root.append_child(&lede).unwrap();

    let row = document.create_element("div").unwrap();
    row.set_class_name("instrument-layer-row");
    row.set_attribute("role", "list").ok();
    row.set_attribute("aria-label", "authoring layers").ok();
    for layer in LAYERS {
        let btn = document.create_element("button").unwrap();
        btn.set_attribute("type", "button").ok();
        btn.set_class_name("lexicon-chip");
        btn.set_attribute("role", "listitem").ok();
        btn.set_attribute("data-layer", layer.as_str()).ok();
        btn.set_attribute("aria-pressed", pressed(current.layer_ok(layer)))
            .ok();
        btn.set_text_content(Some(layer.as_str()));
        row.append_child(&btn).unwrap();
    }
    let labelled = document.create_element("button").unwrap();
    labelled.set_attribute("type", "button").ok();
    labelled.set_class_name("lexicon-chip");
    labelled
        .set_attribute("data-labelled", pressed(current.labelled))
        .ok();
    labelled
        .set_attribute("aria-pressed", pressed(current.labelled))
        .ok();
    labelled.set_attribute("aria-label", LABELLED_ARIA).ok();
    labelled.set_text_content(Some("labelled"));
    row.append_child(&labelled).unwrap();
    root.append_child(&row).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_class_name("lexicon-held-gate");
    status.set_attribute("role", "status").ok();
    status.set_attribute("data-next-action", "1").ok();
    status.set_text_content(Some(next_action(&current)));
    root.append_child(&status).unwrap();

    let loss = document.create_element("p").unwrap();
    loss.set_class_name("lexicon-bay-lede");
    loss.set_attribute("data-round-trip-loss", "1").ok();
    loss.set_attribute("hidden", "").ok();
    root.append_child(&loss).unwrap();

    let publish = document.create_element("button").unwrap();
    publish.set_attribute("type", "button").ok();
    publish.set_class_name("lexicon-open-btn");
    publish.set_attribute("data-manufacture-publish", "1").ok();
    publish.set_attribute("disabled", "").ok();
    publish
        .set_attribute("aria-label", publish_label(&current))
        .ok();
    publish.set_text_content(Some(publish_label(&current)));
    root.append_child(&publish).unwrap();

    wire_toggles(&root, draft.clone());
    wire_keys(&root, draft);
    root
}

fn refresh_panel(root: &Element, draft: &Draft) {
    for layer in LAYERS {
        let sel = format!("[data-layer=\"{}\"]", layer.as_str());
        if let Some(btn) = root.query_selector(&sel).ok().flatten() {
            btn.set_attribute("aria-pressed", pressed(draft.layer_ok(layer)))
                .ok();
        }
    }
    if let Some(lab) = root.query_selector("[data-labelled]").ok().flatten() {
        lab.set_attribute("data-labelled", pressed(draft.labelled))
            .ok();
        lab.set_attribute("aria-pressed", pressed(draft.labelled))
            .ok();
    }
    if let Some(status) = root.query_selector("[data-next-action]").ok().flatten() {
        status.set_text_content(Some(next_action(draft)));
    }
    if let Some(rt) = root
        .query_selector("[data-round-trip-loss]")
        .ok()
        .flatten()
    {
        if draft.round_trip_loss {
            rt.remove_attribute("hidden").ok();
            rt.set_text_content(Some(HELD_ROUND_TRIP));
        } else {
            rt.set_attribute("hidden", "").ok();
            rt.set_text_content(None);
        }
    }
    if let Some(pub_btn) = root
        .query_selector("[data-manufacture-publish]")
        .ok()
        .flatten()
    {
        let ready = can_publish(draft).is_ok();
        if ready {
            pub_btn.remove_attribute("disabled").ok();
        } else {
            pub_btn.set_attribute("disabled", "").ok();
        }
        let label = publish_label(draft);
        pub_btn.set_attribute("aria-label", label).ok();
        pub_btn.set_text_content(Some(label));
    }
}

fn wire_toggles(root: &Element, draft: Rc<RefCell<Draft>>) {
    let nodes = root.query_selector_all("[data-layer]").unwrap();
    for i in 0..nodes.length() {
        let btn = nodes.get(i).unwrap().dyn_into::<Element>().unwrap();
        let key = btn.get_attribute("data-layer").unwrap_or_default();
        let layer = match key.as_str() {
            "source" => Layer::Source,
            "graph" => Layer::Graph,
            "executable" => Layer::Executable,
            _ => continue,
        };
        let draft_c = draft.clone();
        let root_c = root.clone();
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let mut next = *draft_c.borrow();
            toggle_layer(&mut next, layer);
            *draft_c.borrow_mut() = next;
            refresh_panel(&root_c, &next);
        }) as Box<dyn FnMut(_)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }

    if let Some(lab) = root.query_selector("[data-labelled]").ok().flatten() {
        let draft_c = draft.clone();
        let root_c = root.clone();
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let mut next = *draft_c.borrow();
            next.labelled = !next.labelled;
            *draft_c.borrow_mut() = next;
            refresh_panel(&root_c, &next);
        }) as Box<dyn FnMut(_)>);
        lab.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }
}

fn wire_keys(root: &Element, draft: Rc<RefCell<Draft>>) {
    root.set_attribute("tabindex", "0").ok();
    let focus = Rc::new(RefCell::new(ManufactureFocus::start()));
    let draft_c = draft;
    let root_c = root.clone();
    let focus_c = focus;
    let closure = Closure::wrap(Box::new(move |e: KeyboardEvent| {
        let key = e.key();
        let chord = classify(&key, e.ctrl_key() || e.meta_key());
        let mut draft = *draft_c.borrow();
        let mut focus = *focus_c.borrow();
        match apply_key(&mut focus, chord, &mut draft) {
            Ok(_) | Err(_) => {
                *draft_c.borrow_mut() = draft;
                *focus_c.borrow_mut() = focus;
                refresh_panel(&root_c, &draft);
                if !matches!(
                    chord,
                    super::manufacture_keys::ManufactureKey::Ignore
                ) {
                    e.prevent_default();
                }
            }
        }
    }) as Box<dyn FnMut(_)>);
    root.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .ok();
    closure.forget();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready() -> Draft {
        Draft {
            source_ok: true,
            graph_ok: true,
            executable_ok: true,
            labelled: true,
            round_trip_loss: false,
        }
    }

    #[test]
    fn toggle_source_without_graph_sets_round_trip_loss() {
        let mut draft = Draft::default();
        toggle_layer(&mut draft, Layer::Source);
        assert!(draft.source_ok);
        assert!(!draft.graph_ok);
        assert!(draft.round_trip_loss);
        assert_eq!(next_action(&draft), HELD_ROUND_TRIP);
        toggle_layer(&mut draft, Layer::Graph);
        assert!(draft.graph_ok);
        assert!(!draft.round_trip_loss);
        toggle_layer(&mut draft, Layer::Graph);
        assert!(!draft.graph_ok);
        assert!(draft.round_trip_loss);
    }

    #[test]
    fn unlabelled_cannot_enable_publish() {
        let mut draft = ready();
        draft.labelled = false;
        assert!(can_publish(&draft).is_err());
        assert_eq!(publish_label(&draft), HELD_INVALID_PUBLISH);
        assert!(can_publish(&ready()).is_ok());
        assert_eq!(publish_label(&ready()), NEXT_PUBLISH);
    }

    #[test]
    fn host_dot_never_appears_in_labels() {
        for layer in LAYERS {
            assert!(!layer.as_str().contains("Host."));
        }
        for s in [
            REGION_LABEL,
            TITLE,
            LEDE,
            LABELLED_ARIA,
            HELD_INVALID_PUBLISH,
            NEXT_PUBLISH,
            HELD_ROUND_TRIP,
            next_action(&Draft::default()),
        ] {
            assert!(!s.contains("Host."));
            assert!(!s.to_ascii_lowercase().contains("broken"));
        }
    }
}
