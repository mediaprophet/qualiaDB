//! G-LEXICON-0 studio-bay chrome — Catalog peer (davinci / monet).
//!
//! Held-gate + living/artifact/machine chips on live
//! `GraphDatabase.lexicon_manifest`. No Host widen. No in-binary WordNet.
//! Missing / unknown / E300 looks **held / not yet**, never "broken".
//! Living copy stays person/living/country — never Thing-washed.

mod model;

pub use model::{
    catalog_filter_chips, chips_for_framing, copy_avoids_broken, copy_avoids_thing_wash,
    held_outcome, interpret_invoke, parse_pack_card, recipe_beat, sanitize_held_why, Framing,
    FramingChip, ManifestOutcome, PackCard, RecipeBeat, RecipeEvent, ARTIFACT_SAYABLE, HELD_WHY,
    INVOKE_ID, LIVING_SAYABLE, MACHINE_SAYABLE,
};

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, HtmlInputElement};

use super::native_daemon::{daemon_invoke, is_daemon_connected};
use model::{
    catalog_filter_chips as filter_chips, chips_for_framing as framing_chips, framing_copy,
    FramingChip as Chip, RecipeBeat as Beat, RecipeEvent as Event, HELD_WHY as WHY,
    INVOKE_ID as BIND,
};

/// Studio bay Catalog peer — path field, chips, held-gate / arrive pack card.
pub fn build_lexicon_bay(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("lexicon-bay");
    root.set_attribute("data-lexicon-bay", "1").ok();
    root.set_attribute("data-shape", "container").ok();
    super::surface_aspects::mark(&root, "entrance");
    paint_held(&root);

    let title = document.create_element("div").unwrap();
    title.set_class_name("lexicon-bay-title");
    title.set_text_content(Some("Catalog · Lexicon packs"));
    title
        .set_attribute(
            "title",
            "Open pack path → GraphDatabase.lexicon_manifest. Daemon down → held / not yet — open lexicon pack.",
        )
        .ok();
    root.append_child(&title).unwrap();

    let path_row = document.create_element("div").unwrap();
    path_row.set_class_name("lexicon-path-row");

    let path = document.create_element("input").unwrap();
    path.set_attribute("type", "text").ok();
    path.set_attribute("data-lexicon-path", "1").ok();
    path.set_attribute(
        "placeholder",
        "lexicon pack path (.lexicon.json or .q42 + sidecar)",
    )
    .ok();
    path.set_attribute("aria-label", "Lexicon pack path").ok();
    path_row.append_child(&path).unwrap();

    let open_btn = document.create_element("button").unwrap();
    open_btn.set_attribute("type", "button").ok();
    open_btn.set_class_name("lexicon-open-btn");
    open_btn.set_text_content(Some("Open pack"));
    path_row.append_child(&open_btn).unwrap();
    root.append_child(&path_row).unwrap();

    let chips = document.create_element("div").unwrap();
    chips.set_class_name("lexicon-chip-row");
    chips.set_attribute("data-lexicon-chips", "1").ok();
    chips.set_attribute("role", "tablist").ok();
    chips
        .set_attribute("aria-label", "living artifact machine")
        .ok();
    root.append_child(&chips).unwrap();

    let stage = document.create_element("div").unwrap();
    stage.set_class_name("lexicon-stage");
    stage.set_attribute("data-lexicon-stage", "1").ok();
    root.append_child(&stage).unwrap();

    render_chips(&root, filter_chips(), None);
    render_held_stage(&root, WHY);
    wire_open(&root, &open_btn);
    wire_dismiss(&root);

    root
}

fn paint_held(root: &Element) {
    root.set_attribute("data-gate", "held").ok();
    root.set_attribute("data-honesty", "unavailable").ok();
    root.set_attribute("data-recipe", Beat::Hold.as_str()).ok();
    root.set_attribute("data-beat", Beat::Hold.named_beat())
        .ok();
    root.set_attribute("data-lexicon-framing", "").ok();
}

fn render_chips(root: &Element, chips: &[Chip], active: Option<Chip>) {
    let Some(row) = root.query_selector("[data-lexicon-chips]").ok().flatten() else {
        return;
    };
    row.set_inner_html("");
    let Some(doc) = row.owner_document() else {
        return;
    };
    for chip in chips {
        let el = doc.create_element("button").unwrap();
        el.set_attribute("type", "button").ok();
        el.set_class_name("lexicon-chip");
        el.set_attribute("data-lexicon-chip", chip.token()).ok();
        el.set_attribute("data-tone", chip.tone()).ok();
        el.set_attribute("title", chip.sayable()).ok();
        let pressed = active == Some(*chip);
        el.set_attribute("aria-pressed", if pressed { "true" } else { "false" })
            .ok();
        el.set_text_content(Some(chip.token()));
        let secondary = doc.create_element("span").unwrap();
        secondary.set_class_name("lexicon-chip-ns");
        let ns = if *chip == Chip::Machine {
            BIND
        } else {
            chip.sayable()
        };
        secondary.set_text_content(Some(ns));
        el.append_child(&secondary).unwrap();
        row.append_child(&el).unwrap();
    }
}

fn render_held_stage(root: &Element, why: &str) {
    let Some(stage) = root.query_selector("[data-lexicon-stage]").ok().flatten() else {
        return;
    };
    stage.set_inner_html("");
    let Some(doc) = stage.owner_document() else {
        return;
    };
    let gate = doc.create_element("div").unwrap();
    gate.set_class_name("lexicon-held-gate");
    gate.set_attribute("data-gate", "held").ok();
    gate.set_attribute("data-honesty", "unavailable").ok();
    gate.set_attribute("data-recipe", Beat::Hold.as_str()).ok();
    let label = doc.create_element("div").unwrap();
    label.set_class_name("lexicon-held-label");
    label.set_text_content(Some("held / not yet"));
    gate.append_child(&label).unwrap();
    let reason = doc.create_element("div").unwrap();
    reason.set_class_name("gated-reason");
    reason.set_text_content(Some(why));
    gate.append_child(&reason).unwrap();
    stage.append_child(&gate).unwrap();
}

fn render_open_stage(root: &Element, card: &PackCard) {
    let Some(stage) = root.query_selector("[data-lexicon-stage]").ok().flatten() else {
        return;
    };
    stage.set_inner_html("");
    let Some(doc) = stage.owner_document() else {
        return;
    };
    let pack = doc.create_element("div").unwrap();
    pack.set_class_name("lexicon-pack-card");
    pack.set_attribute("data-lexicon-pack", "1").ok();
    pack.set_attribute("data-recipe", Beat::Arrive.as_str())
        .ok();
    pack.set_attribute("data-beat", Beat::Arrive.named_beat())
        .ok();
    pack.set_attribute("data-honesty", "live").ok();
    let title = if card.pack_id.is_empty() {
        format!("pack {}", card.pack_semver)
    } else {
        format!("{} · {}", card.pack_id, card.pack_semver)
    };
    let heading = doc.create_element("div").unwrap();
    heading.set_class_name("lexicon-pack-semver");
    heading.set_text_content(Some(&title));
    pack.append_child(&heading).unwrap();

    let frame = doc.create_element("div").unwrap();
    frame.set_class_name("lexicon-pack-framing");
    frame.set_text_content(Some(&format!(
        "{} · {}",
        card.framing.as_str(),
        framing_copy(card.framing)
    )));
    pack.append_child(&frame).unwrap();

    if !card.uplift_from.is_empty() || !card.concept_ids.is_empty() {
        let recipe = doc.create_element("div").unwrap();
        recipe.set_class_name("lexicon-upgrade");
        recipe.set_attribute("data-lexicon-upgrade", "1").ok();
        recipe
            .set_attribute("data-recipe", Beat::Hold.as_str())
            .ok();
        recipe
            .set_attribute("data-beat", Beat::Hold.named_beat())
            .ok();
        let note = doc.create_element("div").unwrap();
        note.set_text_content(Some(if card.uplift_from.is_empty() {
            "Upgrade recipe · listen — hold on concept ids (no pack write)."
        } else {
            "Upgrade recipe · hold on breaking-id list (listen only)."
        }));
        recipe.append_child(&note).unwrap();
        if !card.uplift_from.is_empty() {
            let from = doc.create_element("div").unwrap();
            from.set_class_name("lexicon-uplift");
            from.set_text_content(Some(&format!("uplift from {}", card.uplift_from)));
            recipe.append_child(&from).unwrap();
        }
        for id in &card.concept_ids {
            let row = doc.create_element("div").unwrap();
            row.set_class_name("lexicon-concept-id");
            row.set_text_content(Some(id));
            recipe.append_child(&row).unwrap();
        }
        let dismiss = doc.create_element("button").unwrap();
        dismiss.set_attribute("type", "button").ok();
        dismiss.set_attribute("data-lexicon-dismiss", "1").ok();
        dismiss.set_class_name("lexicon-dismiss-btn");
        dismiss.set_text_content(Some("Leave recipe"));
        recipe.append_child(&dismiss).unwrap();
        pack.append_child(&recipe).unwrap();
    }
    stage.append_child(&pack).unwrap();
}

fn apply_outcome(root: &Element, outcome: ManifestOutcome) {
    match outcome {
        ManifestOutcome::Held { why } => {
            paint_held(root);
            render_chips(root, filter_chips(), None);
            render_held_stage(root, &why);
        }
        ManifestOutcome::Open(card) => {
            let beat = recipe_beat(Event::PackOpen);
            root.set_attribute("data-gate", "open").ok();
            root.set_attribute("data-honesty", "live").ok();
            root.set_attribute("data-recipe", beat.as_str()).ok();
            root.set_attribute("data-beat", beat.named_beat()).ok();
            root.set_attribute("data-lexicon-framing", card.framing.as_str())
                .ok();
            render_chips(root, framing_chips(card.framing), None);
            render_open_stage(root, &card);
            if !card.concept_ids.is_empty() || !card.uplift_from.is_empty() {
                let hold = recipe_beat(Event::BreakingIdsShown);
                if let Some(upgrade) = root.query_selector("[data-lexicon-upgrade]").ok().flatten()
                {
                    upgrade.set_attribute("data-recipe", hold.as_str()).ok();
                    upgrade.set_attribute("data-beat", hold.named_beat()).ok();
                }
            }
        }
    }
}

fn wire_open(root: &Element, button: &Element) {
    let root = root.clone();
    let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        let path = root
            .query_selector("[data-lexicon-path]")
            .ok()
            .flatten()
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            .map(|input| input.value())
            .unwrap_or_default();
        let path = path.trim().to_string();
        if path.is_empty() {
            web_sys::console::log_1(&"[Lexicon Bay] Open pack: empty path → held".into());
            paint_outcome_all_bays(&root, held_outcome(WHY));
            return;
        }
        if !is_daemon_connected() {
            web_sys::console::log_1(
                &"[Lexicon Bay] Open pack: daemon not connected → held".into(),
            );
            paint_outcome_all_bays(&root, held_outcome(WHY));
            return;
        }
        let root_async = root.clone();
        let path_log = path.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match daemon_invoke(BIND, serde_json::json!({ "path": path })).await {
                Ok(response) => {
                    let outcome = interpret_invoke(
                        response.ok,
                        &response.value,
                        response.diagnostic.as_deref(),
                    );
                    web_sys::console::log_1(
                        &format!(
                            "[Lexicon Bay] invoke path={path_log:?} ok={} value_prefix={:?} outcome={outcome:?}",
                            response.ok,
                            response.value.chars().take(120).collect::<String>(),
                        )
                        .into(),
                    );
                    paint_outcome_all_bays(&root_async, outcome);
                }
                Err(err) => {
                    web_sys::console::log_1(
                        &format!("[Lexicon Bay] invoke error → held: {err}").into(),
                    );
                    paint_outcome_all_bays(&root_async, held_outcome(WHY));
                }
            }
        });
    }) as Box<dyn FnMut(web_sys::Event)>);
    button
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

/// Zone D IDE and vibe-console each mount a Catalog bay — paint every
/// `[data-lexicon-bay]` so arrive does not look held on the sibling surface.
fn paint_outcome_all_bays(hint: &Element, outcome: ManifestOutcome) {
    let Some(doc) = hint.owner_document() else {
        apply_outcome(hint, outcome);
        return;
    };
    let Ok(nodes) = doc.query_selector_all("[data-lexicon-bay]") else {
        apply_outcome(hint, outcome);
        return;
    };
    if nodes.length() == 0 {
        apply_outcome(hint, outcome);
        return;
    }
    for i in 0..nodes.length() {
        if let Some(node) = nodes.get(i) {
            if let Ok(el) = node.dyn_into::<Element>() {
                // Keep path fields in sync with the bay that was clicked.
                if let (Some(src), Some(dst)) = (
                    hint.query_selector("[data-lexicon-path]")
                        .ok()
                        .flatten()
                        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok()),
                    el.query_selector("[data-lexicon-path]")
                        .ok()
                        .flatten()
                        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok()),
                ) {
                    dst.set_value(&src.value());
                }
                apply_outcome(&el, outcome.clone());
            }
        }
    }
}

fn wire_dismiss(root: &Element) {
    let root = root.clone();
    let listen = root.clone();
    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let Some(target) = event.target() else {
            return;
        };
        let Ok(el) = target.dyn_into::<Element>() else {
            return;
        };
        if el
            .closest("[data-lexicon-dismiss]")
            .ok()
            .flatten()
            .is_none()
            && el.get_attribute("data-lexicon-dismiss").is_none()
        {
            return;
        }
        let leave = recipe_beat(Event::Dismiss);
        if let Some(upgrade) = root.query_selector("[data-lexicon-upgrade]").ok().flatten() {
            upgrade.set_attribute("data-recipe", leave.as_str()).ok();
            upgrade.set_attribute("data-beat", leave.named_beat()).ok();
        }
        apply_outcome(&root, held_outcome(WHY));
        root.set_attribute("data-recipe", leave.as_str()).ok();
        root.set_attribute("data-beat", leave.named_beat()).ok();
    }) as Box<dyn FnMut(web_sys::Event)>);
    listen
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

/// Thin Catalog / REPL / Problems peer switch for the studio bay drawer.
pub fn wire_bay_tabs(tabs: &Element, panes: &[(&str, &Element)]) {
    let pane_els: Vec<(String, Element)> = panes
        .iter()
        .map(|(id, el)| ((*id).to_string(), (*el).clone()))
        .collect();
    let tab_nodes = tabs.query_selector_all("[data-bay-tab]").unwrap();
    for i in 0..tab_nodes.length() {
        let tab = tab_nodes.get(i).unwrap().dyn_into::<Element>().unwrap();
        let tabs = tabs.clone();
        let pane_els = pane_els.clone();
        let tab_listen = tab.clone();
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let Some(want) = tab.get_attribute("data-bay-tab") else {
                return;
            };
            let all = tabs.query_selector_all("[data-bay-tab]").unwrap();
            for j in 0..all.length() {
                let other = all.get(j).unwrap().dyn_into::<Element>().unwrap();
                let on = other.get_attribute("data-bay-tab").as_deref() == Some(want.as_str());
                other
                    .set_attribute("aria-selected", if on { "true" } else { "false" })
                    .ok();
                let _ = other.class_list().toggle_with_force("is-active", on);
            }
            for (id, pane) in &pane_els {
                let show = id == &want;
                if show {
                    pane.remove_attribute("hidden").ok();
                    if let Ok(html) = pane.clone().dyn_into::<HtmlElement>() {
                        html.style().set_property("display", "").ok();
                    }
                } else {
                    pane.set_attribute("hidden", "").ok();
                    if let Ok(html) = pane.clone().dyn_into::<HtmlElement>() {
                        html.style().set_property("display", "none").ok();
                    }
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        tab_listen
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
