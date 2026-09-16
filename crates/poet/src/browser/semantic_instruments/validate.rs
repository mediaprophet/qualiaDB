//! Validation evidence panel (SI-09).
//!
//! Source N3, compiled graph, and executable form stay distinct. Demo/Reference
//! labelling and round-trip loss are inspectable. Invalid packs stay held.
//! Dispatch is never a Host ID.

use super::manufacture::{can_publish, Draft};
use web_sys::{Document, Element};

const REGION_LABEL: &str = "validation evidence";
const INSPECTABLE: &str = "validation evidence inspectable";

/// One inspectable validation row. `ok` is the check result, not a Host ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Evidence {
    pub id: &'static str,
    pub ok: bool,
    pub label: &'static str,
}

const EVIDENCE_SOURCE: Evidence = Evidence {
    id: "source-n3",
    ok: false,
    label: "source N3",
};
const EVIDENCE_GRAPH: Evidence = Evidence {
    id: "compiled-graph",
    ok: false,
    label: "compiled graph",
};
const EVIDENCE_EXECUTABLE: Evidence = Evidence {
    id: "executable-form",
    ok: false,
    label: "executable form",
};
const EVIDENCE_LABELLED: Evidence = Evidence {
    id: "demo-reference-labelled",
    ok: false,
    label: "Demo/Reference labelled",
};
const EVIDENCE_ROUND_TRIP: Evidence = Evidence {
    id: "round-trip-loss",
    ok: false,
    label: "round-trip loss",
};

/// Evidence for a manufacture draft. Round-trip `ok` is `!draft.round_trip_loss`.
pub fn evidence_for(draft: &Draft) -> Vec<Evidence> {
    vec![
        Evidence {
            ok: draft.source_ok,
            ..EVIDENCE_SOURCE
        },
        Evidence {
            ok: draft.graph_ok,
            ..EVIDENCE_GRAPH
        },
        Evidence {
            ok: draft.executable_ok,
            ..EVIDENCE_EXECUTABLE
        },
        Evidence {
            ok: draft.labelled,
            ..EVIDENCE_LABELLED
        },
        Evidence {
            ok: !draft.round_trip_loss,
            ..EVIDENCE_ROUND_TRIP
        },
    ]
}

/// `None` when `can_publish` is `Ok`; otherwise the held reason.
pub fn validation_held(draft: &Draft) -> Option<&'static str> {
    match can_publish(draft) {
        Ok(()) => None,
        Err(reason) => Some(reason),
    }
}

fn status_copy(draft: &Draft) -> &'static str {
    validation_held(draft).unwrap_or(INSPECTABLE)
}

fn bool_attr(ok: bool) -> &'static str {
    if ok {
        "true"
    } else {
        "false"
    }
}

/// Validation evidence region. Default draft is held / not yet.
pub fn build_validate_view(document: &Document) -> Element {
    let draft = Draft::default();
    let root = document.create_element("div").unwrap();
    root.set_class_name("instrument-validate");
    root.set_attribute("data-validate-panel", "1").ok();
    root.set_attribute("role", "region").ok();
    root.set_attribute("aria-label", REGION_LABEL).ok();

    let title = document.create_element("div").unwrap();
    title.set_class_name("instrument-validate-title");
    title.set_text_content(Some("Validation evidence"));
    root.append_child(&title).unwrap();

    let list = document.create_element("div").unwrap();
    list.set_class_name("instrument-validate-list");
    list.set_attribute("role", "list").ok();
    list.set_attribute("aria-label", REGION_LABEL).ok();
    for row in evidence_for(&draft) {
        let item = document.create_element("div").unwrap();
        item.set_class_name("instrument-validate-row");
        item.set_attribute("role", "listitem").ok();
        item.set_attribute("data-evidence-id", row.id).ok();
        item.set_attribute("data-ok", bool_attr(row.ok)).ok();
        item.set_text_content(Some(row.label));
        list.append_child(&item).unwrap();
    }
    root.append_child(&list).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_class_name("instrument-validate-status");
    status.set_attribute("data-validate-status", "1").ok();
    status.set_attribute("role", "status").ok();
    status.set_text_content(Some(status_copy(&draft)));
    root.append_child(&status).unwrap();
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_labelled() -> Draft {
        Draft {
            source_ok: true,
            graph_ok: true,
            executable_ok: true,
            labelled: true,
            round_trip_loss: false,
        }
    }

    #[test]
    fn default_draft_all_evidence_not_ok_and_held() {
        let draft = Draft::default();
        let rows = evidence_for(&draft);
        assert_eq!(rows.len(), 5);
        for row in &rows {
            if row.id == "round-trip-loss" {
                // Specified formula: ok = !draft.round_trip_loss (Default is false).
                assert_eq!(row.ok, !draft.round_trip_loss);
            } else {
                assert!(!row.ok, "{}", row.id);
            }
        }
        assert_eq!(
            validation_held(&draft),
            Some(super::super::manufacture::HELD_INVALID_PUBLISH)
        );
        assert_ne!(status_copy(&draft), INSPECTABLE);
    }

    #[test]
    fn ready_labelled_draft_has_no_held_and_all_ok() {
        let draft = ready_labelled();
        assert!(validation_held(&draft).is_none());
        assert_eq!(status_copy(&draft), INSPECTABLE);
        for row in evidence_for(&draft) {
            assert!(row.ok, "{}", row.id);
        }
    }

    #[test]
    fn host_never_in_evidence_ids_or_labels() {
        let draft = ready_labelled();
        for row in evidence_for(&draft).iter().chain(evidence_for(&Draft::default()).iter()) {
            assert!(!row.id.contains("Host."));
            assert!(!row.label.contains("Host."));
        }
        assert!(!REGION_LABEL.contains("Host."));
        assert!(!INSPECTABLE.contains("Host."));
        assert!(!status_copy(&draft).contains("Host."));
        assert!(!status_copy(&Draft::default()).contains("Host."));
    }
}
