//! Instrument I/O SHACL authoring chrome (SI-09).
//!
//! Living-safe: a NaturalPerson is never `owl:Thing`. Reuses the idea of the
//! logic-workbench SHACL panel (target class, constraint type, list) but stays
//! instrument-scoped — not a visual SHACL canvas. Dispatch is by entry point
//! (`assess` / `recognise`); `Host.*` has no constraints.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlInputElement, HtmlSelectElement};

/// One property constraint on an instrument entry-point shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeConstraint {
    pub path: &'static str,
    pub min_count: u8,
    pub message: &'static str,
}

const ASSESS_RECOGNISE_INPUT: &[ShapeConstraint] = &[
    ShapeConstraint {
        path: "si:inputShape",
        min_count: 1,
        message: "Input SHACL/shape reference is required.",
    },
    ShapeConstraint {
        path: "si:incompleteInputBehaviour",
        min_count: 1,
        message: "Incomplete input behaviour is Held.",
    },
];

const ASSESS_RECOGNISE_OUTPUT: &[ShapeConstraint] = &[
    ShapeConstraint {
        path: "si:resultKind",
        min_count: 1,
        message: "Result kind is a claim, not a silent fact.",
    },
    ShapeConstraint {
        path: "si:signatureIsNotTruth",
        min_count: 1,
        message: "A signature proves origin/integrity, not substantive truth.",
    },
];

/// Class suggestions for the target-class field. `owl:Thing` is never offered.
const CLASS_SUGGESTIONS: &[&str] = &["si:InstrumentRelease", "values:NaturalPerson", "rdfs:Class"];

const CONSTRAINT_TYPES: &[&str] = &["minCount", "class", "datatype"];

pub fn input_constraints(entry: &str) -> &'static [ShapeConstraint] {
    if is_assess_or_recognise(entry) {
        ASSESS_RECOGNISE_INPUT
    } else {
        &[]
    }
}

pub fn output_constraints(entry: &str) -> &'static [ShapeConstraint] {
    if is_assess_or_recognise(entry) {
        ASSESS_RECOGNISE_OUTPUT
    } else {
        &[]
    }
}

pub fn living_safe_guard() -> &'static str {
    "Living-safe: a NaturalPerson is never owl:Thing; persons are rdfs:Class, not the OWL universal superclass."
}

/// Instrument-scoped I/O shape chrome. Client-side authoring only.
pub fn build_shapes_view(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("instrument-shapes");
    root.set_attribute("role", "region").ok();
    root.set_attribute("aria-label", "instrument input and output shapes")
        .ok();

    let status = document.create_element("p").unwrap();
    status.set_class_name("instrument-shapes-status");
    status.set_attribute("role", "status").ok();
    status.set_attribute("data-si-shapes-status", "1").ok();
    status.set_text_content(Some(living_safe_guard()));
    root.append_child(&status).unwrap();

    append_group(document, &root, "input", input_constraints("assess"));
    append_group(document, &root, "output", output_constraints("assess"));

    let row = document.create_element("div").unwrap();
    row.set_class_name("instrument-shapes-author");
    row.set_attribute("role", "group").ok();
    row.set_attribute("aria-label", "author a constraint").ok();

    let target = document.create_element("input").unwrap();
    target.set_attribute("type", "text").ok();
    target.set_attribute("data-si-shapes-target", "1").ok();
    target
        .set_attribute("placeholder", "si:InstrumentRelease")
        .ok();
    target.set_attribute("aria-label", "target class").ok();
    target
        .set_attribute("list", "si-shapes-class-suggestions")
        .ok();
    row.append_child(&target).unwrap();

    let list = document.create_element("datalist").unwrap();
    list.set_id("si-shapes-class-suggestions");
    for class in CLASS_SUGGESTIONS {
        let opt = document.create_element("option").unwrap();
        opt.set_attribute("value", class).ok();
        list.append_child(&opt).unwrap();
    }
    row.append_child(&list).unwrap();

    let select = document.create_element("select").unwrap();
    select.set_attribute("data-si-shapes-type", "1").ok();
    select.set_attribute("aria-label", "constraint type").ok();
    for ty in CONSTRAINT_TYPES {
        let opt = document.create_element("option").unwrap();
        opt.set_attribute("value", ty).ok();
        opt.set_text_content(Some(ty));
        select.append_child(&opt).unwrap();
    }
    row.append_child(&select).unwrap();

    let add = document.create_element("button").unwrap();
    add.set_attribute("type", "button").ok();
    add.set_attribute("data-si-shapes-add", "1").ok();
    add.set_attribute("aria-label", "add constraint").ok();
    add.set_text_content(Some("Add"));
    row.append_child(&add).unwrap();
    root.append_child(&row).unwrap();

    let authored = document.create_element("div").unwrap();
    authored.set_class_name("instrument-shapes-authored");
    authored.set_attribute("data-si-shapes-authored", "1").ok();
    authored.set_attribute("role", "list").ok();
    authored
        .set_attribute("aria-label", "authored constraints")
        .ok();
    root.append_child(&authored).unwrap();

    wire_add(&root);
    root
}

fn is_assess_or_recognise(entry: &str) -> bool {
    let e = entry.trim();
    !e.starts_with("Host.") && matches!(e, "assess" | "recognise")
}

fn append_group(
    document: &Document,
    parent: &Element,
    label: &str,
    constraints: &[ShapeConstraint],
) {
    let heading = document.create_element("div").unwrap();
    heading.set_class_name("instrument-shapes-group");
    heading.set_text_content(Some(label));
    parent.append_child(&heading).unwrap();
    for c in constraints {
        let row = document.create_element("div").unwrap();
        row.set_class_name("instrument-shapes-row");
        row.set_attribute("role", "listitem").ok();
        row.set_attribute("data-shape-path", c.path).ok();
        row.set_text_content(Some(&format!(
            "{} · minCount {} · {}",
            c.path, c.min_count, c.message
        )));
        parent.append_child(&row).unwrap();
    }
}

fn is_owl_thing(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower == "owl:thing" || lower.ends_with("#thing") || lower.contains("owl:thing")
}

/// `owl:Thing` is never a class for persons and is not offered as a class.
fn reject_owl_thing_class(target: &str) -> bool {
    is_owl_thing(target)
}

fn wire_add(root: &Element) {
    let add = match root.query_selector("[data-si-shapes-add]").ok().flatten() {
        Some(el) => el,
        None => return,
    };
    let root_c = root.clone();
    let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        let target = root_c
            .query_selector("[data-si-shapes-target]")
            .ok()
            .flatten()
            .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
            .map(|i| i.value())
            .unwrap_or_default();
        let ctype = root_c
            .query_selector("[data-si-shapes-type]")
            .ok()
            .flatten()
            .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
            .map(|s| s.value())
            .unwrap_or_default();
        let status = root_c
            .query_selector("[data-si-shapes-status]")
            .ok()
            .flatten();
        if target.trim().is_empty() {
            return;
        }
        if reject_owl_thing_class(&target) {
            if let Some(status) = status {
                status.set_text_content(Some(living_safe_guard()));
            }
            return;
        }
        let list = match root_c
            .query_selector("[data-si-shapes-authored]")
            .ok()
            .flatten()
        {
            Some(el) => el,
            None => return,
        };
        let doc = match list.owner_document() {
            Some(d) => d,
            None => return,
        };
        let row = doc.create_element("div").unwrap();
        row.set_class_name("instrument-shapes-row");
        row.set_attribute("role", "listitem").ok();
        row.set_text_content(Some(&format!(
            "{} · minCount 1 · {}",
            ctype.trim(),
            target.trim()
        )));
        list.append_child(&row).ok();
    }) as Box<dyn FnMut(_)>);
    add.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .ok();
    closure.forget();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn living_safe_guard_mentions_owl_thing_and_natural_person() {
        let g = living_safe_guard();
        assert!(g.contains("owl:Thing"));
        assert!(g.contains("NaturalPerson"));
        assert!(g.to_ascii_lowercase().contains("person"));
        assert!(!CLASS_SUGGESTIONS.iter().any(|c| c.contains("owl:Thing")));
    }

    #[test]
    fn assess_has_non_empty_input_and_output() {
        assert!(!input_constraints("assess").is_empty());
        assert!(!output_constraints("assess").is_empty());
        assert!(!input_constraints("recognise").is_empty());
        assert!(!output_constraints("recognise").is_empty());
        assert_eq!(input_constraints("assess")[0].path, "si:inputShape");
        assert_eq!(input_constraints("assess")[0].min_count, 1);
        assert!(input_constraints("assess")[1].message.contains("Held"));
        assert_eq!(output_constraints("assess")[0].path, "si:resultKind");
        assert_eq!(
            output_constraints("assess")[1].path,
            "si:signatureIsNotTruth"
        );
    }

    #[test]
    fn host_star_is_empty() {
        assert!(input_constraints("Host.").is_empty());
        assert!(output_constraints("Host.").is_empty());
        assert!(input_constraints("Host.assess").is_empty());
        assert!(output_constraints("Host.recognise").is_empty());
        assert!(input_constraints("Host.Instrument.assess").is_empty());
    }

    #[test]
    fn no_constraint_path_contains_host() {
        for entry in ["assess", "recognise"] {
            for c in input_constraints(entry)
                .iter()
                .chain(output_constraints(entry))
            {
                assert!(!c.path.contains("Host."));
                assert!(!c.message.contains("Host."));
            }
        }
    }
}
