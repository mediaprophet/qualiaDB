//! Poet Catalog · Instruments bay (SI-09 chrome).
//! Seed cards, inspect, collect≠activate, manufacture layers. No Host IDs.

use super::inspect::{
    activate_requires_closed, collect_without_activate, record_run, InspectState, ACTIONS,
    COLLECTED_NOT_ACTIVE, LIFECYCLE_CHIPS,
};
use super::manufacture::{can_publish, next_action, Draft, Layer};
use super::{seed_cards, CHIP_DEMO, CHIP_REFERENCE, HELD_OPEN_PACK, WASM_Q42_HELD};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element};

/// Catalog peer for semantic-instrument collectables.
pub fn build_instrument_bay(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("lexicon-bay instrument-bay");
    root.set_attribute("data-instrument-bay", "1").ok();
    root.set_attribute("data-shape", "container").ok();
    root.set_attribute("data-gate", "held").ok();
    root.set_attribute("role", "region").ok();
    root.set_attribute("aria-label", "Catalog · Instruments").ok();

    let title = document.create_element("div").unwrap();
    title.set_class_name("lexicon-bay-title");
    title.set_text_content(Some("Catalog · Instruments"));
    root.append_child(&title).unwrap();

    let lede = document.create_element("p").unwrap();
    lede.set_class_name("lexicon-bay-lede");
    lede.set_text_content(Some(
        "Demo and Reference packs. Artwork is a handle, not proof. Collect is not activate.",
    ));
    root.append_child(&lede).unwrap();

    let chips = document.create_element("div").unwrap();
    chips.set_class_name("lexicon-chip-row");
    chips.set_attribute("role", "list").ok();
    chips.set_attribute("aria-label", "lifecycle").ok();
    for chip in LIFECYCLE_CHIPS {
        let el = document.create_element("span").unwrap();
        el.set_class_name("lexicon-chip");
        el.set_attribute("role", "listitem").ok();
        el.set_text_content(Some(chip));
        chips.append_child(&el).unwrap();
    }
    for label in [CHIP_DEMO, CHIP_REFERENCE] {
        let el = document.create_element("span").unwrap();
        el.set_class_name("lexicon-chip");
        el.set_text_content(Some(label));
        chips.append_child(&el).unwrap();
    }
    root.append_child(&chips).unwrap();

    let layers = document.create_element("div").unwrap();
    layers.set_class_name("instrument-layer-row");
    layers.set_attribute("aria-label", "authoring layers").ok();
    for layer in [Layer::Source, Layer::Graph, Layer::Executable] {
        let el = document.create_element("span").unwrap();
        el.set_class_name("lexicon-chip");
        el.set_attribute("data-layer", layer.as_str()).ok();
        el.set_text_content(Some(layer.as_str()));
        layers.append_child(&el).unwrap();
    }
    root.append_child(&layers).unwrap();

    let list = document.create_element("div").unwrap();
    list.set_class_name("instrument-card-list");
    list.set_attribute("role", "list").ok();
    for card in seed_cards() {
        let btn = document.create_element("button").unwrap();
        btn.set_attribute("type", "button").ok();
        btn.set_class_name("lexicon-pack-card instrument-card");
        btn.set_attribute("data-instrument-slug", card.slug).ok();
        btn.set_attribute("aria-label", card.accessible_text).ok();
        btn.set_attribute("role", "listitem").ok();
        btn.set_text_content(Some(&format!(
            "{} · {} · {}",
            card.name, card.category_chip, card.entry_point
        )));
        list.append_child(&btn).unwrap();
    }
    root.append_child(&list).unwrap();

    let actions = document.create_element("div").unwrap();
    actions.set_class_name("instrument-action-row");
    actions.set_attribute("role", "toolbar").ok();
    actions.set_attribute("aria-label", "instrument actions").ok();
    for (id, label) in ACTIONS {
        let btn = document.create_element("button").unwrap();
        btn.set_attribute("type", "button").ok();
        btn.set_class_name("lexicon-open-btn");
        btn.set_attribute("data-instrument-action", *id).ok();
        btn.set_attribute("aria-label", *label).ok();
        btn.set_text_content(Some(*label));
        actions.append_child(&btn).unwrap();
    }
    root.append_child(&actions).unwrap();

    let stage = document.create_element("div").unwrap();
    stage.set_class_name("lexicon-held-gate");
    stage.set_attribute("data-instrument-stage", "1").ok();
    stage.set_attribute("role", "status").ok();
    stage.set_text_content(Some(HELD_OPEN_PACK));
    root.append_child(&stage).unwrap();

    root.append_child(&super::build_flow_view(document)).unwrap();
    root.append_child(&super::build_graph_view(document)).unwrap();
    root.append_child(&super::build_shapes_view(document)).unwrap();
    root.append_child(&super::build_shapes_canvas(document)).unwrap();
    root.append_child(&super::build_manufacture_panel(document))
        .unwrap();
    root.append_child(&super::build_receipts_view(document))
        .unwrap();
    root.append_child(&super::build_validate_view(document))
        .unwrap();
    root.append_child(&super::build_version_view(document))
        .unwrap();
    root.append_child(&super::build_deps_view(document))
        .unwrap();
    root.append_child(&super::build_fixture_view(document))
        .unwrap();
    root.append_child(&super::build_capability_view(document))
        .unwrap();
    root.append_child(&super::build_manifest_view(document))
        .unwrap();
    if let Some(stage) = root.query_selector("[data-instrument-stage]").ok().flatten() {
        stage
            .set_attribute("aria-live", super::a11y::live_polite())
            .ok();
    }

    let wasm = document.create_element("p").unwrap();
    wasm.set_class_name("lexicon-bay-lede");
    wasm.set_text_content(Some(WASM_Q42_HELD));
    root.append_child(&wasm).unwrap();

    let selected = Rc::new(RefCell::new(Option::<String>::None));
    let inspect = Rc::new(RefCell::new(InspectState::default()));
    wire_cards(&root, selected.clone(), inspect.clone());
    wire_actions(&root, selected, inspect);
    root
}

fn stage_el(root: &Element) -> Option<Element> {
    root.query_selector("[data-instrument-stage]").ok().flatten()
}

fn set_stage(root: &Element, msg: &str) {
    if let Some(stage) = stage_el(root) {
        stage.set_text_content(Some(msg));
    }
}

fn wire_cards(
    root: &Element,
    selected: Rc<RefCell<Option<String>>>,
    inspect: Rc<RefCell<InspectState>>,
) {
    let root_c = root.clone();
    let nodes = root.query_selector_all("[data-instrument-slug]").unwrap();
    for i in 0..nodes.length() {
        let btn = nodes.get(i).unwrap().dyn_into::<Element>().unwrap();
        let slug = btn.get_attribute("data-instrument-slug").unwrap_or_default();
        let root_c = root_c.clone();
        let selected = selected.clone();
        let inspect = inspect.clone();
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            *selected.borrow_mut() = Some(slug.clone());
            *inspect.borrow_mut() = InspectState::default();
            let msg = super::seed_cards()
                .iter()
                .find(|c| c.slug == slug)
                .map(|c| {
                    super::a11y::announce_select(&format!(
                        "{} · {} · entry {} · inspect before network fetch · artwork is not proof",
                        c.name, c.category_chip, c.entry_point
                    ))
                })
                .unwrap_or_else(|| HELD_OPEN_PACK.to_string());
            set_stage(&root_c, &msg);
        }) as Box<dyn FnMut(_)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }
}

fn wire_actions(
    root: &Element,
    selected: Rc<RefCell<Option<String>>>,
    inspect: Rc<RefCell<InspectState>>,
) {
    let root_c = root.clone();
    let nodes = root.query_selector_all("[data-instrument-action]").unwrap();
    for i in 0..nodes.length() {
        let btn = nodes.get(i).unwrap().dyn_into::<Element>().unwrap();
        let action = btn
            .get_attribute("data-instrument-action")
            .unwrap_or_default();
        let root_c = root_c.clone();
        let selected = selected.clone();
        let inspect = inspect.clone();
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let has_card = selected.borrow().is_some();
            let msg = match action.as_str() {
                "inspect" => {
                    if !has_card {
                        HELD_OPEN_PACK
                    } else {
                        if inspect.borrow().collected {
                            inspect.borrow_mut().lock_closed = true;
                        }
                        "inspect before network fetch · licence and honesty visible · artwork is a handle, not proof"
                    }
                }
                "collect" => {
                    if !has_card {
                        HELD_OPEN_PACK
                    } else {
                        collect_without_activate(&mut inspect.borrow_mut());
                        COLLECTED_NOT_ACTIVE
                    }
                }
                "activate" => match activate_requires_closed(&inspect.borrow()) {
                    Ok(()) => {
                        inspect.borrow_mut().activated = true;
                        "active"
                    }
                    Err(e) => e,
                },
                "run" => match record_run(&mut inspect.borrow_mut()) {
                    Ok(()) => "run entry point assess",
                    Err(e) => e,
                },
                "publish" => can_publish(&Draft {
                    source_ok: true,
                    graph_ok: true,
                    executable_ok: true,
                    labelled: true,
                    round_trip_loss: false,
                })
                .err()
                .unwrap_or("ready to publish labelled pack"),
                "cancel" => "cancel run",
                "view-receipt" => "receipt history remains after revoke",
                _ => next_action(&Draft::default()),
            };
            set_stage(&root_c, msg);
        }) as Box<dyn FnMut(_)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }
}
