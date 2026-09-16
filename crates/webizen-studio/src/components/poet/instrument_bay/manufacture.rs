//! Studio manufacture panel (SI-09 chrome).
//!
//! Source, graph, and executable stay distinct layers. Empty drafts name the
//! next action. Unlabelled or incomplete packs cannot publish.

use dioxus::prelude::*;

/// Authoring layers. Chrome must not collapse these into one blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Source,
    Graph,
    Executable,
}

impl Layer {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Graph => "graph",
            Self::Executable => "executable",
        }
    }
}

pub const LAYERS: [Layer; 3] = [Layer::Source, Layer::Graph, Layer::Executable];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Draft {
    pub source_ok: bool,
    pub graph_ok: bool,
    pub executable_ok: bool,
    pub labelled: bool,
    pub round_trip_loss: bool,
}

pub const HELD_ADD_N3: &str = "held / not yet — add definition N3";
pub const HELD_COMPILE_GRAPH: &str = "held / not yet — compile graph from source";
pub const HELD_BUILD_EXECUTABLE: &str = "held / not yet — build executable form";
pub const HELD_LABEL: &str = "held / not yet — Demo/Reference must stay labelled";
pub const HELD_ROUND_TRIP: &str =
    "held / not yet — round-trip loss shown; restore graph from source";
pub const HELD_INVALID_PUBLISH: &str = "held / not yet — invalid pack cannot publish";
pub const NEXT_PUBLISH: &str = "sign and publish";

impl Draft {
    pub fn layer_ok(self, layer: Layer) -> bool {
        match layer {
            Layer::Source => self.source_ok,
            Layer::Graph => self.graph_ok,
            Layer::Executable => self.executable_ok,
        }
    }

    pub fn layers_ready(self) -> bool {
        self.source_ok && self.graph_ok && self.executable_ok
    }
}

pub fn toggle_layer(draft: &mut Draft, layer: Layer) {
    match layer {
        Layer::Source => draft.source_ok = !draft.source_ok,
        Layer::Graph => draft.graph_ok = !draft.graph_ok,
        Layer::Executable => draft.executable_ok = !draft.executable_ok,
    }
    draft.round_trip_loss = draft.source_ok && !draft.graph_ok;
}

/// Next named action for an incomplete or ready draft.
pub fn next_action(draft: &Draft) -> &'static str {
    if !draft.source_ok {
        return HELD_ADD_N3;
    }
    if draft.round_trip_loss {
        return HELD_ROUND_TRIP;
    }
    if !draft.graph_ok {
        return HELD_COMPILE_GRAPH;
    }
    if !draft.executable_ok {
        return HELD_BUILD_EXECUTABLE;
    }
    if !draft.labelled {
        return HELD_LABEL;
    }
    NEXT_PUBLISH
}

/// Publish requires every layer plus a Demo/Reference label.
pub fn can_publish(draft: &Draft) -> Result<(), &'static str> {
    if !draft.layers_ready() || !draft.labelled {
        return Err(HELD_INVALID_PUBLISH);
    }
    Ok(())
}

#[component]
pub fn ManufacturePanel() -> Element {
    let mut draft = use_signal(Draft::default);
    let current = draft();
    let ready = can_publish(&current).is_ok();

    rsx! {
        div {
            class: "instrument-manufacture-panel",
            "data-manufacture-panel": "1",
            role: "region",
            "aria-label": "manufacture layers",

            div { class: "lexicon-bay-title", "Manufacture" }
            p { class: "lexicon-bay-lede",
                "Source, graph, and executable stay distinct. Invalid packs cannot publish."
            }

            div {
                class: "instrument-layer-row",
                role: "list",
                "aria-label": "authoring layers",
                for layer in LAYERS {
                    button {
                        r#type: "button",
                        class: "lexicon-chip",
                        role: "listitem",
                        "data-layer": "{layer.as_str()}",
                        "aria-pressed": "{current.layer_ok(layer)}",
                        onclick: move |_| {
                            let mut next = draft();
                            toggle_layer(&mut next, layer);
                            draft.set(next);
                        },
                        "{layer.as_str()}"
                    }
                }
                button {
                    r#type: "button",
                    class: "lexicon-chip",
                    "data-labelled": "{current.labelled}",
                    "aria-pressed": "{current.labelled}",
                    "aria-label": "Keep Demo/Reference labelled",
                    onclick: move |_| {
                        let mut next = draft();
                        next.labelled = !next.labelled;
                        draft.set(next);
                    },
                    "labelled"
                }
            }

            div {
                class: "lexicon-held-gate",
                role: "status",
                "data-next-action": "1",
                "{next_action(&current)}"
            }

            if current.round_trip_loss {
                p {
                    class: "lexicon-bay-lede",
                    "data-round-trip-loss": "1",
                    "{HELD_ROUND_TRIP}"
                }
            }

            button {
                r#type: "button",
                class: "lexicon-open-btn",
                "data-manufacture-publish": "1",
                disabled: !ready,
                "aria-label": if ready { NEXT_PUBLISH } else { HELD_INVALID_PUBLISH },
                if ready { "{NEXT_PUBLISH}" } else { "{HELD_INVALID_PUBLISH}" }
            }
        }
    }
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
    fn layers_stay_distinct() {
        assert_eq!(LAYERS.len(), 3);
        assert_ne!(Layer::Source, Layer::Graph);
        assert_ne!(Layer::Graph, Layer::Executable);
        assert_ne!(Layer::Source, Layer::Executable);
        assert_eq!(Layer::Source.as_str(), "source");
        assert_eq!(Layer::Graph.as_str(), "graph");
        assert_eq!(Layer::Executable.as_str(), "executable");
        let names: [&str; 3] = [
            LAYERS[0].as_str(),
            LAYERS[1].as_str(),
            LAYERS[2].as_str(),
        ];
        assert_ne!(names[0], names[1]);
        assert_ne!(names[1], names[2]);
        assert_ne!(names[0], names[2]);
    }

    #[test]
    fn cannot_publish_unlabelled() {
        let mut draft = ready();
        draft.labelled = false;
        assert_eq!(can_publish(&draft), Err(HELD_INVALID_PUBLISH));
        assert_eq!(next_action(&draft), HELD_LABEL);
        assert!(can_publish(&ready()).is_ok());
        let missing = Draft::default();
        assert_eq!(next_action(&missing), HELD_ADD_N3);
        assert_eq!(can_publish(&missing), Err(HELD_INVALID_PUBLISH));
    }
}
