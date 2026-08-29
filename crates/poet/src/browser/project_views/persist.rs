//! Shared COP ledger helpers for Project container views.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement, HtmlSelectElement, HtmlTextAreaElement};

use super::super::cop_records::{
    build_cop_panel, build_count_panel, build_family_panel, CopField, CopPanel,
};
use super::super::native_daemon::{
    daemon_records_query, daemon_records_upsert, is_daemon_connected, NativeRecordQueryRequest,
    NativeRecordUpsertRequest,
};

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

pub fn build_bulk_import_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; overflow: auto; padding: 8px; gap: 8px;",
    );

    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    let status_el: HtmlElement = status.clone().dyn_into().unwrap();
    status_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);");
    status.set_text_content(Some(
        "Paste JSON Lines ({title, fields}) and import into a project family. Invalid lines fail closed.",
    ));
    wrapper.append_child(&status).unwrap();

    let select = document.create_element("select").unwrap();
    select.set_attribute("data-import-family", "true").ok();
    for (family, label) in IMPORT_FAMILIES {
        let option = document.create_element("option").unwrap();
        option.set_attribute("value", family).ok();
        option.set_text_content(Some(&format!("{label} ({family})")));
        select.append_child(&option).unwrap();
    }
    wrapper.append_child(&select).unwrap();

    let area = document.create_element("textarea").unwrap();
    area.set_attribute("data-import-body", "true").ok();
    area.set_attribute(
        "placeholder",
        "{\"title\":\"Review ontology\",\"fields\":{\"status\":\"open\",\"assignee\":\"did:…\"}}",
    )
    .ok();
    let area_el: HtmlElement = area.clone().dyn_into().unwrap();
    area_el.style().set_css_text(
        "min-height: 160px; font-family: var(--font-mono); font-size: 10px; padding: 8px; \
         background: var(--canvas-bg); color: var(--text-primary); border: 1px solid var(--border-subtle);",
    );
    wrapper.append_child(&area).unwrap();

    let save = document.create_element("button").unwrap();
    save.set_text_content(Some("Import JSON Lines"));
    save.set_attribute("type", "button").ok();
    save.set_attribute("data-requires-daemon", "true").ok();
    let export = document.create_element("button").unwrap();
    export.set_text_content(Some("Export selected family as JSON"));
    export.set_attribute("type", "button").ok();
    export.set_attribute("data-requires-daemon", "true").ok();
    if !is_daemon_connected() {
        save.set_attribute("disabled", "").ok();
        save.set_attribute("title", "Requires a running local QualiaDB daemon.")
            .ok();
        export.set_attribute("disabled", "").ok();
        export
            .set_attribute("title", "Requires a running local QualiaDB daemon.")
            .ok();
    }
    wrapper.append_child(&save).unwrap();
    wrapper.append_child(&export).unwrap();

    let wrapper_clone = wrapper.clone();
    let status_clone = status.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let family = wrapper_clone
            .query_selector("[data-import-family]")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<HtmlSelectElement>().ok())
            .map(|select| select.value())
            .unwrap_or_default();
        let body = wrapper_clone
            .query_selector("[data-import-body]")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<HtmlTextAreaElement>().ok())
            .map(|area| area.value())
            .unwrap_or_default();
        if family.is_empty() || body.trim().is_empty() {
            status_clone
                .set_text_content(Some("Select a family and paste at least one JSON line."));
            return;
        }
        status_clone.set_text_content(Some("Importing…"));
        let status_async = status_clone.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let mut saved = 0usize;
            let mut failed = 0usize;
            for (index, line) in body.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let parsed: serde_json::Value = match serde_json::from_str(line) {
                    Ok(value) => value,
                    Err(error) => {
                        failed += 1;
                        status_async.set_text_content(Some(&format!(
                            "Line {} is not JSON: {error}",
                            index + 1
                        )));
                        continue;
                    }
                };
                let title = parsed
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if title.trim().is_empty() {
                    failed += 1;
                    continue;
                }
                let fields = parsed
                    .get("fields")
                    .and_then(serde_json::Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                match daemon_records_upsert(NativeRecordUpsertRequest {
                    family: family.clone(),
                    title,
                    id: None,
                    fields,
                })
                .await
                {
                    Ok(response) if response.ok => saved += 1,
                    _ => failed += 1,
                }
            }
            let _ = daemon_records_upsert(NativeRecordUpsertRequest {
                family: "project_import".to_string(),
                title: format!("import into {family}"),
                id: None,
                fields: serde_json::Map::from_iter([
                    (
                        "target".to_string(),
                        serde_json::Value::String(family.clone()),
                    ),
                    (
                        "saved".to_string(),
                        serde_json::Value::String(saved.to_string()),
                    ),
                    (
                        "failed".to_string(),
                        serde_json::Value::String(failed.to_string()),
                    ),
                ]),
            })
            .await;
            status_async.set_text_content(Some(&format!(
                "Imported {saved} record(s); {failed} line(s) rejected."
            )));
        });
    }) as Box<dyn FnMut(_)>);
    save.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    let wrapper_export = wrapper.clone();
    let status_export = status.clone();
    let export_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let family = wrapper_export
            .query_selector("[data-import-family]")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<HtmlSelectElement>().ok())
            .map(|select| select.value())
            .unwrap_or_default();
        if family.is_empty() {
            status_export.set_text_content(Some("Select a family to export."));
            return;
        }
        status_export.set_text_content(Some("Exporting…"));
        let status_async = status_export.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match daemon_records_query(NativeRecordQueryRequest {
                family: family.clone(),
                query: String::new(),
                kind: String::new(),
            })
            .await
            {
                Ok(response) if response.ok => {
                    let encoded = js_sys::encode_uri_component(
                        &serde_json::to_string_pretty(&response.data).unwrap_or_default(),
                    );
                    let Some(document) = web_sys::window().and_then(|window| window.document())
                    else {
                        status_async.set_text_content(Some("Window unavailable for export."));
                        return;
                    };
                    match document
                        .create_element("a")
                        .ok()
                        .and_then(|element| element.dyn_into::<web_sys::HtmlAnchorElement>().ok())
                    {
                        Some(anchor) => {
                            anchor.set_href(&format!(
                                "data:application/json;charset=utf-8,{encoded}"
                            ));
                            anchor.set_download(&format!("{family}.json"));
                            anchor.click();
                            status_async.set_text_content(Some(&format!(
                                "Exported live `{family}` records as JSON."
                            )));
                        }
                        None => {
                            status_async
                                .set_text_content(Some("Export download could not be created."));
                        }
                    }
                }
                Ok(response) => status_async.set_text_content(Some(
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Export query failed."),
                )),
                Err(error) => status_async.set_text_content(Some(&error)),
            }
        });
    }) as Box<dyn FnMut(_)>);
    export
        .add_event_listener_with_callback("click", export_closure.as_ref().unchecked_ref())
        .unwrap();
    export_closure.forget();

    wrapper
        .append_child(&build_family_panel(
            document,
            "project_import",
            "Import receipts from previous JSON Line runs.",
            &[
                CopField {
                    key: "target",
                    placeholder: "Target family",
                },
                CopField {
                    key: "saved",
                    placeholder: "Saved count",
                },
                CopField {
                    key: "failed",
                    placeholder: "Failed count",
                },
            ],
        ))
        .unwrap();
    wrapper
}

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
