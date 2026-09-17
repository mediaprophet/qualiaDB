//! Visual logic FLOW for a semantic instrument (SI-09).
//!
//! Not a second Logic Workbench. Dispatch is entry-point names
//! (`assess` / `recognise`) only — never Host.*.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element};

const CONNECTING_COPY: &str = "entry → input shape → logic → output shape";
const REGION_LABEL: &str = "instrument logic flow";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowKind {
    Entry,
    InputShape,
    Logic,
    OutputShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowNode {
    pub id: &'static str,
    pub kind: FlowKind,
    pub label: &'static str,
}

const ASSESS_FLOW: &[FlowNode] = &[
    FlowNode {
        id: "assess",
        kind: FlowKind::Entry,
        label: "assess",
    },
    FlowNode {
        id: "assess-input",
        kind: FlowKind::InputShape,
        label: "input shape",
    },
    FlowNode {
        id: "assess-logic",
        kind: FlowKind::Logic,
        label: "N3/CML logic",
    },
    FlowNode {
        id: "assess-output",
        kind: FlowKind::OutputShape,
        label: "output shape",
    },
];

const RECOGNISE_FLOW: &[FlowNode] = &[
    FlowNode {
        id: "recognise",
        kind: FlowKind::Entry,
        label: "recognise",
    },
    FlowNode {
        id: "recognise-input",
        kind: FlowKind::InputShape,
        label: "input shape",
    },
    FlowNode {
        id: "recognise-logic",
        kind: FlowKind::Logic,
        label: "N3/CML logic",
    },
    FlowNode {
        id: "recognise-output",
        kind: FlowKind::OutputShape,
        label: "output shape",
    },
];

/// Four-node pipeline for `assess` / `recognise`. Unknown and Host.* are empty.
pub fn flow_for_entry(entry: &str) -> &'static [FlowNode] {
    if entry.contains("Host.") {
        return &[];
    }
    match entry {
        "assess" => ASSESS_FLOW,
        "recognise" => RECOGNISE_FLOW,
        _ => &[],
    }
}

/// Instrument logic-flow region. Node clicks write the node label into status.
pub fn build_flow_view(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("instrument-flow");
    root.set_attribute("data-instrument-flow", "1").ok();
    root.set_attribute("role", "region").ok();
    root.set_attribute("aria-label", REGION_LABEL).ok();

    let title = document.create_element("div").unwrap();
    title.set_class_name("instrument-flow-title");
    title.set_text_content(Some("Instrument logic flow"));
    root.append_child(&title).unwrap();

    let copy = document.create_element("p").unwrap();
    copy.set_class_name("instrument-flow-copy");
    copy.set_text_content(Some(CONNECTING_COPY));
    root.append_child(&copy).unwrap();

    let list = document.create_element("div").unwrap();
    list.set_class_name("instrument-flow-nodes");
    list.set_attribute("role", "list").ok();
    list.set_attribute("aria-label", REGION_LABEL).ok();
    for node in ASSESS_FLOW {
        let btn = document.create_element("button").unwrap();
        btn.set_attribute("type", "button").ok();
        btn.set_class_name("instrument-flow-node");
        btn.set_attribute("data-flow-node", node.id).ok();
        btn.set_attribute("data-flow-kind", kind_attr(node.kind)).ok();
        btn.set_attribute("aria-label", node.label).ok();
        btn.set_attribute("role", "listitem").ok();
        btn.set_text_content(Some(node.label));
        list.append_child(&btn).unwrap();
    }
    root.append_child(&list).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_class_name("instrument-flow-status");
    status.set_attribute("data-flow-status", "1").ok();
    status.set_attribute("role", "status").ok();
    root.append_child(&status).unwrap();

    wire_nodes(&root);
    root
}

fn kind_attr(kind: FlowKind) -> &'static str {
    match kind {
        FlowKind::Entry => "entry",
        FlowKind::InputShape => "input-shape",
        FlowKind::Logic => "logic",
        FlowKind::OutputShape => "output-shape",
    }
}

fn set_status(root: &Element, msg: &str) {
    if let Some(status) = root.query_selector("[data-flow-status]").ok().flatten() {
        status.set_text_content(Some(msg));
    }
}

fn wire_nodes(root: &Element) {
    let root_c = root.clone();
    let nodes = root.query_selector_all("[data-flow-node]").unwrap();
    for i in 0..nodes.length() {
        let btn = nodes.get(i).unwrap().dyn_into::<Element>().unwrap();
        let id = btn.get_attribute("data-flow-node").unwrap_or_default();
        let root_c = root_c.clone();
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let label = ASSESS_FLOW
                .iter()
                .chain(RECOGNISE_FLOW.iter())
                .find(|n| n.id == id)
                .map(|n| n.label)
                .unwrap_or("");
            set_status(&root_c, label);
        }) as Box<dyn FnMut(_)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds_of(nodes: &[FlowNode]) -> [FlowKind; 4] {
        [
            nodes[0].kind,
            nodes[1].kind,
            nodes[2].kind,
            nodes[3].kind,
        ]
    }

    #[test]
    fn assess_and_recognise_have_four_ordered_kinds() {
        let expected = [
            FlowKind::Entry,
            FlowKind::InputShape,
            FlowKind::Logic,
            FlowKind::OutputShape,
        ];
        let assess = flow_for_entry("assess");
        let recognise = flow_for_entry("recognise");
        assert_eq!(assess.len(), 4);
        assert_eq!(recognise.len(), 4);
        assert_eq!(kinds_of(assess), expected);
        assert_eq!(kinds_of(recognise), expected);
    }

    #[test]
    fn host_clinical_risk_framingham_yields_empty() {
        assert!(flow_for_entry("Host.ClinicalRisk.framingham").is_empty());
        assert!(flow_for_entry("Host.Anything").is_empty());
        assert!(flow_for_entry("run").is_empty());
        assert!(flow_for_entry("").is_empty());
    }

    #[test]
    fn node_labels_are_non_empty_and_contain_no_host_ids() {
        for entry in ["assess", "recognise"] {
            for node in flow_for_entry(entry) {
                assert!(!node.id.is_empty());
                assert!(!node.label.is_empty());
                assert!(!node.id.contains("Host."));
                assert!(!node.label.contains("Host."));
            }
        }
        assert!(!CONNECTING_COPY.contains("Host."));
        assert!(!REGION_LABEL.contains("Host."));
    }
}
