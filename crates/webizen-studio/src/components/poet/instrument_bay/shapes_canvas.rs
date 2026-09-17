//! Studio SVG canvas of SHACL constraints (SI-09).
//!
//! Input rail y=40, output y=110. Host.* is empty. Not `owl:Thing`.
//! Constraint lists duplicate `shapes.rs` conceptually; this file is not a Host ID.

use super::select::{filter_constraints, rail_for_flow_label};
use dioxus::prelude::*;

const INPUT_Y: f32 = 40.0;
const OUTPUT_Y: f32 = 110.0;
const CANVAS_W: f32 = 520.0;
const CANVAS_H: f32 = 160.0;
const NODE_W: f32 = 96.0;
const NODE_H: f32 = 36.0;
const PAD: f32 = 28.0;
const ARIA: &str = "instrument shape canvas";
const HELD_HOST: &str = "unknown entry point (not a Host ID)";
const LIVING_SAFE: &str = "Living-safe: a NaturalPerson is never owl:Thing; persons are rdfs:Class, not the OWL universal superclass.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShapeConstraint {
    path: &'static str,
    min_count: u8,
    message: &'static str,
}

const INPUT: &[ShapeConstraint] = &[
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

const OUTPUT: &[ShapeConstraint] = &[
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

fn is_owl_thing(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower == "owl:thing" || lower.ends_with("#thing") || lower.contains("owl:thing")
}

fn keep_constraint(c: &ShapeConstraint) -> bool {
    !is_owl_thing(c.path)
        && !c.path.contains("Host.")
        && !c.message.contains("Host.")
        && !is_owl_thing(c.message)
}

fn is_assess_or_recognise(entry: &str) -> bool {
    let e = entry.trim();
    !e.starts_with("Host.") && matches!(e, "assess" | "recognise")
}

fn living_row(constraints: &'static [ShapeConstraint]) -> Vec<ShapeConstraint> {
    constraints
        .iter()
        .copied()
        .filter(keep_constraint)
        .collect()
}

fn row_for(entry: &str, output: bool) -> Vec<ShapeConstraint> {
    if !is_assess_or_recognise(entry) {
        return Vec::new();
    }
    living_row(if output { OUTPUT } else { INPUT })
}

fn visible_rows(entry: &str, rail: &str) -> (Vec<ShapeConstraint>, Vec<ShapeConstraint>) {
    let inputs = row_for(entry, false);
    let outputs = row_for(entry, true);
    let selected = rail_for_flow_label(rail);
    let (show_input, show_output) = filter_constraints(selected, inputs.len(), outputs.len());
    (
        if show_input { inputs } else { Vec::new() },
        if show_output { outputs } else { Vec::new() },
    )
}

/// Living-safe constraint paths for an entry (input then output).
/// `Host.*` and unknown entries yield an empty lens.
pub fn canvas_nodes(entry: &str) -> Vec<&'static str> {
    row_for(entry, false)
        .into_iter()
        .chain(row_for(entry, true))
        .map(|c| c.path)
        .collect()
}

#[cfg(test)]
fn canvas_nodes_for_rail(entry: &str, rail: &str) -> Vec<&'static str> {
    let (inputs, outputs) = visible_rows(entry, rail);
    inputs
        .into_iter()
        .chain(outputs)
        .map(|c| c.path)
        .collect()
}

fn step_focus(i: usize, len: usize, key: &str) -> usize {
    match key {
        "ArrowRight" | "Right" if len > 0 && i + 1 < len => i + 1,
        "ArrowLeft" | "Left" if i > 0 => i - 1,
        _ => i,
    }
}

fn row_centres(n: usize, y: f32) -> Vec<(f32, f32)> {
    if n == 0 {
        return Vec::new();
    }
    let usable = (CANVAS_W - PAD * 2.0).max(NODE_W);
    (0..n)
        .map(|i| {
            let t = if n == 1 {
                0.5
            } else {
                i as f32 / (n - 1) as f32
            };
            let x = PAD + NODE_W * 0.5 + t * (usable - NODE_W);
            (
                x.clamp(NODE_W * 0.5, CANVAS_W - NODE_W * 0.5),
                y.clamp(NODE_H * 0.5, CANVAS_H - NODE_H * 0.5),
            )
        })
        .collect()
}

#[component]
pub fn InstrumentShapesCanvas(
    entry: String,
    #[props(default = "pipeline".to_string())] rail: String,
) -> Element {
    let (inputs, outputs) = visible_rows(&entry, &rail);
    let all_nodes = canvas_nodes(&entry);
    let in_pts = row_centres(inputs.len(), INPUT_Y);
    let out_pts = row_centres(outputs.len(), OUTPUT_Y);
    let paths: Vec<&'static str> = inputs
        .iter()
        .chain(outputs.iter())
        .map(|c| c.path)
        .collect();
    let count = paths.len();
    let input_n = inputs.len();
    let mut focus = use_signal(|| 0usize);
    let mut status = use_signal(|| {
        if entry.trim().starts_with("Host.") {
            HELD_HOST.to_string()
        } else {
            LIVING_SAFE.to_string()
        }
    });
    let key_paths = paths.clone();

    rsx! {
        div {
            class: "instrument-shapes-canvas",
            role: "application",
            tabindex: "0",
            "aria-label": ARIA,
            "data-shapes-canvas": "1",
            "data-node-count": "{count}",
            "data-total-nodes": "{all_nodes.len()}",
            "data-shapes-focus": "{focus()}",
            onkeydown: move |e| {
                let n = key_paths.len();
                if n == 0 {
                    return;
                }
                let i = step_focus(focus(), n, &e.key().to_string());
                focus.set(i);
                if let Some(path) = key_paths.get(i) {
                    status.set((*path).to_string());
                }
            },

            svg {
                width: "{CANVAS_W}",
                height: "{CANVAS_H}",
                view_box: "0 0 {CANVAS_W} {CANVAS_H}",
                "data-shapes-canvas-svg": "1",
                for (i, c) in inputs.iter().copied().enumerate() {
                    g {
                        tabindex: "0",
                        role: "button",
                        "aria-label": "{c.path}",
                        "data-shape-path": "{c.path}",
                        "data-min-count": "{c.min_count}",
                        "data-shape-message": "{c.message}",
                        onclick: move |_| {
                            focus.set(i);
                            status.set(c.path.to_string());
                        },
                        rect {
                            x: "{in_pts[i].0 - NODE_W * 0.5}",
                            y: "{in_pts[i].1 - NODE_H * 0.5}",
                            width: "{NODE_W}",
                            height: "{NODE_H}",
                            rx: "6",
                            fill: if focus() == i { "rgba(125,211,252,0.28)" } else { "rgba(125,211,252,0.12)" },
                            stroke: "currentColor",
                        }
                        text {
                            x: "{in_pts[i].0}",
                            y: "{in_pts[i].1 + 4.0}",
                            text_anchor: "middle",
                            font_size: "11",
                            fill: "currentColor",
                            "{c.path}"
                        }
                    }
                }
                for (i, c) in outputs.iter().copied().enumerate() {
                    g {
                        tabindex: "0",
                        role: "button",
                        "aria-label": "{c.path}",
                        "data-shape-path": "{c.path}",
                        "data-shape-message": "{c.message}",
                        onclick: move |_| {
                            focus.set(input_n + i);
                            status.set(c.path.to_string());
                        },
                        rect {
                            x: "{out_pts[i].0 - NODE_W * 0.5}",
                            y: "{out_pts[i].1 - NODE_H * 0.5}",
                            width: "{NODE_W}",
                            height: "{NODE_H}",
                            rx: "6",
                            fill: if focus() == input_n + i { "rgba(125,211,252,0.28)" } else { "rgba(125,211,252,0.12)" },
                            stroke: "currentColor",
                        }
                        text {
                            x: "{out_pts[i].0}",
                            y: "{out_pts[i].1 + 4.0}",
                            text_anchor: "middle",
                            font_size: "11",
                            fill: "currentColor",
                            "{c.path}"
                        }
                    }
                }
            }

            div {
                class: "instrument-shapes-canvas-status",
                role: "status",
                "data-shapes-canvas-status": "1",
                "{status}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::canvas_nodes;

    #[test]
    fn assess_has_four_nodes() {
        let nodes = canvas_nodes("assess");
        assert_eq!(nodes.len(), 4);
        assert_eq!(nodes[0], "si:inputShape");
        assert_eq!(nodes[1], "si:incompleteInputBehaviour");
        assert_eq!(nodes[2], "si:resultKind");
        assert_eq!(nodes[3], "si:signatureIsNotTruth");
        assert_eq!(canvas_nodes("recognise"), nodes);
        let ins = super::row_centres(2, super::INPUT_Y);
        let outs = super::row_centres(2, super::OUTPUT_Y);
        assert_eq!(ins[0].1, 40.0);
        assert_eq!(outs[0].1, 110.0);
        assert!(ins[0].0 < ins[1].0);
    }

    #[test]
    fn host_entries_are_empty() {
        assert!(canvas_nodes("Host.").is_empty());
        assert!(canvas_nodes("Host.assess").is_empty());
        assert!(canvas_nodes("Host.recognise").is_empty());
        assert!(canvas_nodes("Host.Instrument.assess").is_empty());
        assert!(canvas_nodes("").is_empty());
        assert!(canvas_nodes("run").is_empty());
    }

    #[test]
    fn no_host_in_paths() {
        for entry in ["assess", "recognise"] {
            for path in canvas_nodes(entry) {
                assert!(!path.contains("Host."));
                assert!(!path.contains("owl:Thing"));
            }
        }
        assert!(!super::ARIA.contains("Host."));
        assert!(super::is_owl_thing("owl:Thing"));
        assert!(!super::keep_constraint(&super::ShapeConstraint {
            path: "owl:Thing",
            min_count: 1,
            message: "no",
        }));
    }

    #[test]
    fn rail_filter_hides_input_or_output() {
        let all = canvas_nodes("assess");
        assert_eq!(all.len(), 4);
        let input = super::canvas_nodes_for_rail("assess", "input shape");
        assert_eq!(input, vec!["si:inputShape", "si:incompleteInputBehaviour"]);
        let output = super::canvas_nodes_for_rail("assess", "output shape");
        assert_eq!(output, vec!["si:resultKind", "si:signatureIsNotTruth"]);
        assert_eq!(super::canvas_nodes_for_rail("assess", "pipeline"), all);
        assert!(super::canvas_nodes_for_rail("Host.", "pipeline").is_empty());
        let mut i = 0usize;
        i = super::step_focus(i, input.len(), "ArrowRight");
        assert_eq!(input[i], "si:incompleteInputBehaviour");
        i = super::step_focus(i, input.len(), "ArrowRight");
        assert_eq!(input[i], "si:incompleteInputBehaviour");
        i = super::step_focus(i, input.len(), "ArrowLeft");
        assert_eq!(input[i], "si:inputShape");
        for path in input.iter().chain(output.iter()) {
            assert!(!path.contains("Host."));
        }
    }
}
