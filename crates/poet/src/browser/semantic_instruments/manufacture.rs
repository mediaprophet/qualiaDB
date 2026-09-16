//! Manufacture state machine (SI-09).
//!
//! Source text, generated graph, and executable form stay distinct layers.
//! Empty / incomplete states name the next action. Invalid or unlabelled
//! packs cannot publish. No Host IDs.

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

/// Keyboard-named manufacture actions (a11y). Icon-only is not a handle.
pub const ACTIONS: &[(&str, &str)] = &[
    ("add-definition-n3", "Add definition N3"),
    ("compile-graph", "Compile graph from source"),
    ("build-executable", "Build executable form"),
    ("label-category", "Keep Demo/Reference labelled"),
    ("show-round-trip-loss", "Show round-trip loss before save"),
    ("validate", "Validate pack"),
    ("sign", "Sign pack"),
    ("publish", "Publish pack"),
];

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
    pub fn layer_ok(&self, layer: Layer) -> bool {
        match layer {
            Layer::Source => self.source_ok,
            Layer::Graph => self.graph_ok,
            Layer::Executable => self.executable_ok,
        }
    }

    pub fn layers_ready(&self) -> bool {
        self.source_ok && self.graph_ok && self.executable_ok
    }
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

/// Invalid (missing layer) or unlabelled packs cannot publish.
pub fn can_publish(draft: &Draft) -> Result<(), &'static str> {
    if !draft.layers_ready() {
        return Err(HELD_INVALID_PUBLISH);
    }
    if !draft.labelled {
        return Err(HELD_LABEL);
    }
    Ok(())
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
    }

    #[test]
    fn next_action_names_missing_definition_n3() {
        let draft = Draft::default();
        assert_eq!(next_action(&draft), HELD_ADD_N3);
        assert_eq!(can_publish(&draft), Err(HELD_INVALID_PUBLISH));
    }

    #[test]
    fn next_action_names_graph_then_executable() {
        let mut draft = Draft {
            source_ok: true,
            ..Draft::default()
        };
        assert_eq!(next_action(&draft), HELD_COMPILE_GRAPH);
        draft.graph_ok = true;
        assert_eq!(next_action(&draft), HELD_BUILD_EXECUTABLE);
    }

    #[test]
    fn round_trip_loss_is_shown_before_save() {
        let draft = Draft {
            source_ok: true,
            graph_ok: false,
            executable_ok: false,
            labelled: true,
            round_trip_loss: true,
        };
        assert_eq!(next_action(&draft), HELD_ROUND_TRIP);
        assert_eq!(can_publish(&draft), Err(HELD_INVALID_PUBLISH));
    }

    #[test]
    fn unlabelled_cannot_publish() {
        let mut draft = ready();
        draft.labelled = false;
        assert_eq!(next_action(&draft), HELD_LABEL);
        assert_eq!(can_publish(&draft), Err(HELD_LABEL));
    }

    #[test]
    fn labelled_complete_draft_may_publish() {
        let draft = ready();
        assert_eq!(next_action(&draft), NEXT_PUBLISH);
        assert!(can_publish(&draft).is_ok());
    }

    #[test]
    fn every_manufacture_action_has_accessible_name() {
        for (id, name) in ACTIONS {
            assert!(!id.is_empty());
            assert!(!name.is_empty());
            assert!(!id.contains("Host."));
        }
        let ids: Vec<&str> = ACTIONS.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&"publish"));
        assert!(ids.contains(&"validate"));
        assert!(ids.contains(&"sign"));
    }
}
