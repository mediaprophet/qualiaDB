//! Source ↔ visual / graph round-trip loss (SI-09).
//!
//! Declares concrete unsupported constructs before save. Layout coordinate
//! round-trips live in [`super::persist`]; this module owns content fidelity.

use super::manufacture::Draft;

/// Concrete loss reasons shown before save. Never a Host ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LossReason {
    pub id: &'static str,
    pub label: &'static str,
}

pub const LOSS_SOURCE_WITHOUT_GRAPH: LossReason = LossReason {
    id: "source-without-graph",
    label: "source N3 present without compiled graph",
};
pub const LOSS_GRAPH_WITHOUT_EXECUTABLE: LossReason = LossReason {
    id: "graph-without-executable",
    label: "compiled graph present without executable form",
};
pub const LOSS_VISUAL_STALE: LossReason = LossReason {
    id: "visual-stale",
    label: "visual .10d handle stale relative to source",
};

/// Detect round-trip loss from layer flags. Empty when layers are consistent.
pub fn loss_reasons(draft: &Draft) -> Vec<LossReason> {
    let mut out = Vec::new();
    if draft.source_ok && !draft.graph_ok {
        out.push(LOSS_SOURCE_WITHOUT_GRAPH);
    }
    if draft.graph_ok && !draft.executable_ok {
        out.push(LOSS_GRAPH_WITHOUT_EXECUTABLE);
    }
    if draft.round_trip_loss && !out.iter().any(|r| r.id == LOSS_SOURCE_WITHOUT_GRAPH.id) {
        out.push(LOSS_SOURCE_WITHOUT_GRAPH);
    }
    // Visual stale only when authoring flagged loss after source change.
    if draft.round_trip_loss && draft.source_ok {
        out.push(LOSS_VISUAL_STALE);
    }
    out
}

/// Sync the `round_trip_loss` flag from layer consistency.
pub fn sync_round_trip_flag(draft: &mut Draft) {
    draft.round_trip_loss = draft.source_ok && !draft.graph_ok;
}

/// Source → graph → executable with no loss; returns empty reasons.
pub fn source_visual_round_trip_ok(draft: &Draft) -> bool {
    draft.source_ok
        && draft.graph_ok
        && draft.executable_ok
        && !draft.round_trip_loss
        && loss_reasons(draft).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_without_graph_lists_concrete_loss() {
        let draft = Draft {
            source_ok: true,
            graph_ok: false,
            executable_ok: false,
            labelled: true,
            round_trip_loss: true,
        };
        let reasons = loss_reasons(&draft);
        assert!(reasons.iter().any(|r| r.id == "source-without-graph"));
        assert!(reasons.iter().any(|r| r.id == "visual-stale"));
        assert!(!source_visual_round_trip_ok(&draft));
        for r in &reasons {
            assert!(!r.id.contains("Host."));
            assert!(!r.label.contains("Host."));
        }
    }

    #[test]
    fn complete_layers_have_no_loss() {
        let draft = Draft {
            source_ok: true,
            graph_ok: true,
            executable_ok: true,
            labelled: true,
            round_trip_loss: false,
        };
        assert!(loss_reasons(&draft).is_empty());
        assert!(source_visual_round_trip_ok(&draft));
    }

    #[test]
    fn sync_sets_flag_when_source_lacks_graph() {
        let mut draft = Draft {
            source_ok: true,
            graph_ok: false,
            ..Draft::default()
        };
        sync_round_trip_flag(&mut draft);
        assert!(draft.round_trip_loss);
        draft.graph_ok = true;
        sync_round_trip_flag(&mut draft);
        assert!(!draft.round_trip_loss);
    }
}
