//! Studio instrument I/O SHACL chrome (SI-09).
//!
//! Living-safe: a NaturalPerson is never `owl:Thing`. Lists `assess` /
//! `recognise` constraints; `Host.*` is empty. Not a visual SHACL canvas.

use super::select::{filter_constraints, rail_for_flow_label, Rail};
use dioxus::prelude::*;

/// One property constraint on an instrument entry-point shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeConstraint {
    pub path: &'static str,
    pub min_count: u8,
    pub message: &'static str,
}

/// Author-added row stored in the panel signal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredConstraint {
    pub path: String,
    pub min_count: u8,
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

fn is_assess_or_recognise(entry: &str) -> bool {
    let e = entry.trim();
    !e.starts_with("Host.") && matches!(e, "assess" | "recognise")
}

fn is_owl_thing(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower == "owl:thing" || lower.ends_with("#thing") || lower.contains("owl:thing")
}

pub fn parse_min_count(raw: &str) -> u8 {
    raw.trim().parse::<u8>().unwrap_or(1).max(1)
}

/// Append a path/minCount row. `owl:Thing` and Host IDs are refused.
pub fn add_authored(
    path: &str,
    min_count: u8,
    out: &mut Vec<AuthoredConstraint>,
) -> Result<(), &'static str> {
    let path = path.trim();
    if path.is_empty() {
        return Err("held / not yet — path is required");
    }
    if is_owl_thing(path) {
        return Err(living_safe_guard());
    }
    if path.contains("Host.") {
        return Err("unknown entry point (not a Host ID)");
    }
    out.push(AuthoredConstraint {
        path: path.to_string(),
        min_count: min_count.max(1),
    });
    Ok(())
}

#[component]
pub fn InstrumentShapes(
    #[props(default = "assess".to_string())] entry: String,
    #[props(default = "pipeline".to_string())] rail: String,
) -> Element {
    let mut path = use_signal(String::new);
    let mut min_count = use_signal(|| "1".to_string());
    let mut authored = use_signal(Vec::<AuthoredConstraint>::new);
    let mut status = use_signal(|| living_safe_guard().to_string());
    let inputs = input_constraints(&entry);
    let outputs = output_constraints(&entry);
    let selected: Rail = rail_for_flow_label(&rail);
    let (show_input, show_output) = filter_constraints(selected, inputs.len(), outputs.len());

    rsx! {
        div {
            class: "instrument-shapes",
            "data-instrument-shapes": "1",
            role: "region",
            "aria-label": "instrument input and output shapes",

            p {
                class: "instrument-shapes-status lexicon-bay-lede",
                role: "status",
                "data-si-shapes-status": "1",
                "{status}"
            }

            if show_input {
                div { class: "instrument-shapes-group", "input" }
                for c in inputs {
                    div {
                        class: "instrument-shapes-row",
                        role: "listitem",
                        "data-shape-path": "{c.path}",
                        "{c.path} · minCount {c.min_count} · {c.message}"
                    }
                }
            }

            if show_output {
                div { class: "instrument-shapes-group", "output" }
                for c in outputs {
                    div {
                        class: "instrument-shapes-row",
                        role: "listitem",
                        "data-shape-path": "{c.path}",
                        "{c.path} · minCount {c.min_count} · {c.message}"
                    }
                }
            }

            div {
                class: "instrument-shapes-author",
                role: "group",
                "aria-label": "author a constraint",
                input {
                    r#type: "text",
                    "data-si-shapes-path": "1",
                    "aria-label": "constraint path",
                    placeholder: "si:inputShape",
                    value: "{path}",
                    oninput: move |e| path.set(e.value()),
                }
                input {
                    r#type: "number",
                    min: "1",
                    "data-si-shapes-min": "1",
                    "aria-label": "minCount",
                    value: "{min_count}",
                    oninput: move |e| min_count.set(e.value()),
                }
                button {
                    r#type: "button",
                    class: "lexicon-open-btn",
                    "data-si-shapes-add": "1",
                    "aria-label": "add constraint",
                    onclick: move |_| {
                        let mut rows = authored();
                        match add_authored(&path(), parse_min_count(&min_count()), &mut rows) {
                            Ok(()) => {
                                authored.set(rows);
                                path.set(String::new());
                                status.set(living_safe_guard().to_string());
                            }
                            Err(msg) => status.set(msg.to_string()),
                        }
                    },
                    "Add"
                }
            }

            div {
                class: "instrument-shapes-authored",
                "data-si-shapes-authored": "1",
                role: "list",
                "aria-label": "authored constraints",
                for row in authored() {
                    div {
                        class: "instrument-shapes-row",
                        role: "listitem",
                        "{row.path} · minCount {row.min_count}"
                    }
                }
            }
        }
    }
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
        let mut rows = Vec::new();
        assert_eq!(add_authored("owl:Thing", 1, &mut rows), Err(g));
        assert!(rows.is_empty());
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
        assert_eq!(
            input_constraints("assess")[1].path,
            "si:incompleteInputBehaviour"
        );
        assert_eq!(output_constraints("assess")[0].path, "si:resultKind");
        assert_eq!(
            output_constraints("assess")[1].path,
            "si:signatureIsNotTruth"
        );
        let mut rows = Vec::new();
        assert!(add_authored("si:inputShape", 1, &mut rows).is_ok());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].min_count, 1);
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
        let mut rows = Vec::new();
        assert!(add_authored("Host.assess", 1, &mut rows).is_err());
        assert!(rows.is_empty());
    }

    #[test]
    fn rail_for_flow_label_filters_constraints() {
        let inputs = input_constraints("assess");
        let outputs = output_constraints("assess");
        let rail = rail_for_flow_label("input shape");
        assert_eq!(rail, Rail::InputShape);
        assert_eq!(filter_constraints(rail, inputs.len(), outputs.len()), (true, false));
        assert_eq!(
            filter_constraints(rail_for_flow_label("output shape"), inputs.len(), outputs.len()),
            (false, true)
        );
        assert_eq!(
            filter_constraints(rail_for_flow_label("pipeline"), inputs.len(), outputs.len()),
            (true, true)
        );
        assert_eq!(
            filter_constraints(
                rail_for_flow_label("pipeline"),
                input_constraints("Host.").len(),
                output_constraints("Host.").len()
            ),
            (false, false)
        );
    }
}
