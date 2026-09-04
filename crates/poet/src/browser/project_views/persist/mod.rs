//! Shared COP ledger helpers for Project container views.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use super::super::cop_records::{build_family_panel, CopField};

pub const TASK_COLUMNS: &[(&str, &str)] = &[
    ("proposed", "Proposed"),
    ("todo", "Todo"),
    ("in_progress", "In Progress"),
    ("blocked", "Blocked"),
    ("in_review", "In Review"),
    ("done", "Done"),
    ("cancelled", "Cancelled"),
];

pub const TASK_FIELDS: &[CopField] = &[
    CopField {
        key: "status",
        placeholder: "Status (proposed|todo|in_progress|blocked|in_review|done|cancelled)",
    },
    CopField {
        key: "assignee",
        placeholder: "Assignee DID",
    },
    CopField {
        key: "priority",
        placeholder: "Priority (P0|P1|P2)",
    },
    CopField {
        key: "phase",
        placeholder: "Phase",
    },
    CopField {
        key: "start",
        placeholder: "Start (YYYY-MM-DD)",
    },
    CopField {
        key: "end",
        placeholder: "End (YYYY-MM-DD)",
    },
    CopField {
        key: "percent",
        placeholder: "Percent complete",
    },
];

pub const EVENT_FIELDS: &[CopField] = &[
    CopField {
        key: "date",
        placeholder: "Date (YYYY-MM-DD)",
    },
    CopField {
        key: "kind",
        placeholder: "Kind (meeting|milestone|deadline)",
    },
    CopField {
        key: "actor",
        placeholder: "Actor DID",
    },
    CopField {
        key: "location",
        placeholder: "Location",
    },
];

pub const DASHBOARD_FAMILIES: &[(&str, &str)] = &[
    ("project", "Projects"),
    ("project_task", "Tasks"),
    ("project_issue", "Issues"),
    ("project_wiki", "Wiki pages"),
    ("project_deliverable", "Deliverables"),
    ("project_milestone", "Milestones"),
    ("project_risk", "Risks"),
    ("project_event", "Events"),
    ("project_budget", "Budget lines"),
];

pub const ANALYTICS_FAMILIES: &[(&str, &str)] = &[
    ("project_task", "Tasks"),
    ("project_issue", "Issues"),
    ("project_time", "Time entries"),
    ("project_budget", "Budget lines"),
    ("project_asset", "Assets"),
    ("project_vote", "Votes"),
    ("project_review", "Reviews"),
    ("project_bounty", "Bounties"),
    ("project_member", "Members"),
];

pub const RESOURCE_FAMILIES: &[(&str, &str)] = &[
    ("project_time", "Time entries"),
    ("project_task", "Tasks"),
    ("project_budget", "Budget lines"),
    ("project_cost", "Cost-base rows"),
    ("project_member", "Members"),
    ("project_token", "Tokens"),
];

pub const IMPORT_FAMILIES: &[(&str, &str)] = &[
    ("project_member", "Members"),
    ("project_task", "Tasks"),
    ("project_issue", "Issues"),
    ("project_asset", "Assets"),
    ("project_event", "Events"),
    ("contribution", "Contributions"),
];

pub fn ledger(
    document: &Document,
    family: &'static str,
    heading: &str,
    fields: &'static [CopField],
) -> Element {
    wrap(
        document,
        build_family_panel(document, family, heading, fields),
    )
}

pub fn wrap(document: &Document, child: Element) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; overflow: auto; padding: 8px; gap: 8px;",
    );
    wrapper.append_child(&child).unwrap();
    wrapper
}



mod analytics;
mod imports;

pub use analytics::*;
pub use imports::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_families_are_nonempty_and_unique() {
        assert_eq!(DASHBOARD_FAMILIES.len(), 9);
        let mut names: Vec<_> = DASHBOARD_FAMILIES
            .iter()
            .map(|(family, _)| *family)
            .collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), DASHBOARD_FAMILIES.len());
    }

    #[test]
    fn task_columns_cover_kanban_statuses() {
        assert_eq!(TASK_COLUMNS.len(), 7);
        assert_eq!(TASK_COLUMNS[0].0, "proposed");
        assert_eq!(TASK_COLUMNS[6].0, "cancelled");
    }

    #[test]
    fn import_families_include_tasks_and_members() {
        assert!(IMPORT_FAMILIES
            .iter()
            .any(|(family, _)| *family == "project_task"));
        assert!(IMPORT_FAMILIES
            .iter()
            .any(|(family, _)| *family == "project_member"));
    }
}
