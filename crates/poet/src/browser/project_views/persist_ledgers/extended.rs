//! Simple Project views that persist one COP family each.

use web_sys::{Document, Element};

use super::super::super::cop_records::{build_family_panel, CopField};
use super::super::persist::{ledger, wrap};

/// Builds credentials-view.
pub fn build_credentials_view(document: &Document) -> Element {
    ledger(
        document,
        "project_credential",
        "Credential records. DID signing is not performed by this panel.",
        &[
            CopField {
                key: "subject",
                placeholder: "Subject DID",
            },
            CopField {
                key: "kind",
                placeholder: "Kind",
            },
            CopField {
                key: "status",
                placeholder: "Status (draft|issued|revoked)",
            },
        ],
    )
}

/// Builds integrations-view.
pub fn build_integrations_view(document: &Document) -> Element {
    ledger(
        document,
        "project_integration",
        "Integration endpoints persist as records. External transport is not opened from this panel.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind",
            },
            CopField {
                key: "endpoint",
                placeholder: "Endpoint",
            },
            CopField {
                key: "status",
                placeholder: "Status (configured|disabled)",
            },
        ],
    )
}

/// Builds data-sources-view.
pub fn build_data_sources_view(document: &Document) -> Element {
    ledger(
        document,
        "project_datasource",
        "Data source registry. Fetch/ingest is not run from this panel.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind",
            },
            CopField {
                key: "uri",
                placeholder: "URI",
            },
            CopField {
                key: "format",
                placeholder: "Format",
            },
        ],
    )
}

/// Builds automation-view.
pub fn build_automation_view(document: &Document) -> Element {
    ledger(
        document,
        "project_automation",
        "Automation rules persist as records. Vibe/IntentBus execution is unbound until scheduled.",
        &[
            CopField {
                key: "trigger",
                placeholder: "Trigger",
            },
            CopField {
                key: "action",
                placeholder: "Action",
            },
            CopField {
                key: "status",
                placeholder: "Status (draft|armed|disabled)",
            },
        ],
    )
}

/// Builds token-mgr-view.
pub fn build_token_mgr_view(document: &Document) -> Element {
    ledger(
        document,
        "project_token",
        "Token records. Wallet mint/transfer is not performed here.",
        &[
            CopField {
                key: "symbol",
                placeholder: "Symbol",
            },
            CopField {
                key: "supply",
                placeholder: "Supply",
            },
            CopField {
                key: "status",
                placeholder: "Status",
            },
        ],
    )
}

/// Builds time-tracking-view.
pub fn build_time_tracking_view(document: &Document) -> Element {
    ledger(
        document,
        "project_time",
        "Time entries persist as records. Totals are not fabricated.",
        &[
            CopField {
                key: "actor",
                placeholder: "Actor DID",
            },
            CopField {
                key: "hours",
                placeholder: "Hours",
            },
            CopField {
                key: "task",
                placeholder: "Task id",
            },
            CopField {
                key: "date",
                placeholder: "Date (YYYY-MM-DD)",
            },
        ],
    )
}

/// Builds news-view.
pub fn build_news_view(document: &Document) -> Element {
    ledger(
        document,
        "project_news",
        "News items persist as records. RSS/magnet export is unbound.",
        &[
            CopField {
                key: "date",
                placeholder: "Date (YYYY-MM-DD)",
            },
            CopField {
                key: "summary",
                placeholder: "Summary",
            },
            CopField {
                key: "visibility",
                placeholder: "Visibility (public|restricted)",
            },
        ],
    )
}

/// Builds onboarding-view.
pub fn build_onboarding_view(document: &Document) -> Element {
    ledger(
        document,
        "project_onboarding",
        "Onboarding checklist items persist as records.",
        &[
            CopField {
                key: "assignee",
                placeholder: "Assignee DID",
            },
            CopField {
                key: "status",
                placeholder: "Status (todo|done)",
            },
            CopField {
                key: "step",
                placeholder: "Step number",
            },
        ],
    )
}

/// Builds review-view.
pub fn build_review_view(document: &Document) -> Element {
    ledger(
        document,
        "project_review",
        "Review assignments persist as records.",
        &[
            CopField {
                key: "reviewer",
                placeholder: "Reviewer DID",
            },
            CopField {
                key: "target",
                placeholder: "Target id",
            },
            CopField {
                key: "status",
                placeholder: "Status (assigned|accepted|rejected)",
            },
        ],
    )
}

/// Builds retrospective-view.
pub fn build_retrospective_view(document: &Document) -> Element {
    ledger(
        document,
        "project_retrospective",
        "Retrospective notes are append-only COP records.",
        &[
            CopField {
                key: "date",
                placeholder: "Date (YYYY-MM-DD)",
            },
            CopField {
                key: "went_well",
                placeholder: "Went well",
            },
            CopField {
                key: "improve",
                placeholder: "Improve",
            },
        ],
    )
}

/// Builds ip-registry-view.
pub fn build_ip_registry_view(document: &Document) -> Element {
    ledger(
        document,
        "project_ip",
        "Intellectual-property records. Filing with a registry is not performed here.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (copyright|patent|mark)",
            },
            CopField {
                key: "jurisdiction",
                placeholder: "Jurisdiction",
            },
            CopField {
                key: "status",
                placeholder: "Status",
            },
        ],
    )
}

/// Builds commons-view.
pub fn build_commons_view(document: &Document) -> Element {
    ledger(
        document,
        "project_commons",
        "Commons publication records. Publication transport is unbound.",
        &[
            CopField {
                key: "license",
                placeholder: "License",
            },
            CopField {
                key: "uri",
                placeholder: "URI",
            },
            CopField {
                key: "status",
                placeholder: "Status (draft|published)",
            },
        ],
    )
}

/// Builds project-sheet-view.
pub fn build_project_sheet_view(document: &Document) -> Element {
    let wrapper = wrap(
        document,
        build_family_panel(
            document,
            "project",
            "Project metadata persists on the COP ledger. Badges above used to be fabricated; live records replace them.",
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
                    key: "values",
                    placeholder: "Values / instrument",
                },
                CopField {
                    key: "status",
                    placeholder: "Status",
                },
            ],
        ),
    );
    wrapper
        .append_child(&build_family_panel(
            document,
            "project_member",
            "Members and roles persist as their own family.",
            &[
                CopField {
                    key: "did",
                    placeholder: "Member DID",
                },
                CopField {
                    key: "role",
                    placeholder: "Role",
                },
                CopField {
                    key: "status",
                    placeholder: "Status (active|invited)",
                },
            ],
        ))
        .unwrap();
    wrapper
}
