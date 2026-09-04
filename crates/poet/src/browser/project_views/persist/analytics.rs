use web_sys::{Document, Element};

use super::super::super::cop_records::{
    build_cop_panel, build_count_panel, build_family_panel, CopField, CopPanel,
};
use super::{ledger, wrap, ANALYTICS_FAMILIES, DASHBOARD_FAMILIES, EVENT_FIELDS, RESOURCE_FAMILIES,
    TASK_COLUMNS, TASK_FIELDS};

pub fn build_dashboard_view(document: &Document) -> Element {
    let wrapper = wrap(
        document,
        build_count_panel(
            document,
            "Live project counts from the COP /records ledger. Empty families stay at 0.",
            DASHBOARD_FAMILIES,
        ),
    );
    wrapper
        .append_child(&build_family_panel(
            document,
            "project_milestone",
            "Milestones persist independently of the count cards above.",
            &[
                CopField {
                    key: "date",
                    placeholder: "Date (YYYY-MM-DD)",
                },
                CopField {
                    key: "status",
                    placeholder: "Status (on_track|at_risk|delayed|not_started)",
                },
            ],
        ))
        .unwrap();
    wrapper
}

pub fn build_analytics_view(document: &Document) -> Element {
    wrap(
        document,
        build_count_panel(
            document,
            "Analytics are live ledger counts, not fabricated velocity or burn charts.",
            ANALYTICS_FAMILIES,
        ),
    )
}

pub fn build_resource_report_view(document: &Document) -> Element {
    wrap(
        document,
        build_count_panel(
            document,
            "Resource report is a live count of time, task, budget, cost, member, and token records.",
            RESOURCE_FAMILIES,
        ),
    )
}

pub fn build_portfolio_view(document: &Document) -> Element {
    ledger(
        document,
        "project",
        "Portfolio lists live project records for this principal. Save a project to add it.",
        &[
            CopField {
                key: "type",
                placeholder: "Type (research|civic_open|humanitarian_ict|…)",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity (public|restricted|classified)",
            },
            CopField {
                key: "license",
                placeholder: "License",
            },
            CopField {
                key: "status",
                placeholder: "Status",
            },
        ],
    )
}

pub fn build_kanban_view(document: &Document) -> Element {
    wrap(
        document,
        build_cop_panel(
            document,
            &CopPanel {
                family: "project_task",
                heading: "Kanban columns group live project_task records by status. Empty columns stay empty.",
                fields: TASK_FIELDS,
                kind: Some("task"),
                group_by: Some("status"),
                columns: Some(TASK_COLUMNS),
            },
        ),
    )
}

pub fn build_task_list_view(document: &Document) -> Element {
    wrap(
        document,
        build_cop_panel(
            document,
            &CopPanel {
                family: "project_task",
                heading: "Flat list of the same project_task ledger used by Kanban and Gantt.",
                fields: TASK_FIELDS,
                kind: Some("task"),
                group_by: None,
                columns: None,
            },
        ),
    )
}

pub fn build_gantt_view(document: &Document) -> Element {
    wrap(
        document,
        build_cop_panel(
            document,
            &CopPanel {
                family: "project_task",
                heading: "Gantt is a dated projection of project_task records. Bars require start and end dates on the record; they are not sample schedules.",
                fields: TASK_FIELDS,
                kind: Some("task"),
                group_by: None,
                columns: None,
            },
        ),
    )
}

pub fn build_events_view(document: &Document) -> Element {
    ledger(
        document,
        "project_event",
        "Live project events. Calendar and timeline read this same family.",
        EVENT_FIELDS,
    )
}

pub fn build_calendar_view(document: &Document) -> Element {
    ledger(
        document,
        "project_event",
        "Calendar lists live project_event records by their date field. It is not a fabricated month grid.",
        EVENT_FIELDS,
    )
}

pub fn build_timeline_view(document: &Document) -> Element {
    ledger(
        document,
        "project_event",
        "Timeline is the chronological project_event ledger, not a mock phase chart.",
        EVENT_FIELDS,
    )
}

pub fn build_agent_console_view(document: &Document) -> Element {
    wrap(
        document,
        build_family_panel(
            document,
            "project_agent",
            "Agent queries persist on the ledger. This does not generate model answers; specialist invoke is unbound.",
            &[
                CopField {
                    key: "agent",
                    placeholder: "Agent id (local definition)",
                },
                CopField {
                    key: "scope",
                    placeholder: "Scope (wiki|tasks|governance)",
                },
                CopField {
                    key: "query",
                    placeholder: "Question to record",
                },
                CopField {
                    key: "status",
                    placeholder: "Status (queued|reviewed)",
                },
            ],
        ),
    )
}
