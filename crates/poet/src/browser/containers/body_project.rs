//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Project, agreement, and governance container bodies.
use crate::tool_chest::core::registry::SeedContainer;
use web_sys::{Document, Element};

pub(super) fn try_fill(document: &Document, container: &SeedContainer, body: &Element) -> bool {
    match container.container_type.as_str() {
        "kanban" => {
            body.append_child(&crate::browser::project_views::kanban::build_kanban_view(
                document,
            ))
            .unwrap();
            true
        }
        "project_sheet" => {
            body.append_child(
                &crate::browser::project_views::project_sheet::build_project_sheet_view(document),
            )
            .unwrap();
            true
        }
        "budget" => {
            body.append_child(&crate::browser::project_views::budget::build_budget_view(
                document,
            ))
            .unwrap();
            true
        }
        "cost_base" => {
            body.append_child(
                &crate::browser::project_views::cost_base::build_cost_base_view(document),
            )
            .unwrap();
            true
        }
        "deliverable" => {
            body.append_child(
                &crate::browser::project_views::deliverable::build_deliverable_view(document),
            )
            .unwrap();
            true
        }
        "review" => {
            body.append_child(&crate::browser::project_views::review::build_review_view(
                document,
            ))
            .unwrap();
            true
        }
        "discussion" => {
            body.append_child(
                &crate::browser::project_views::discussion::build_discussion_view(document),
            )
            .unwrap();
            true
        }
        "roadmap" => {
            body.append_child(&crate::browser::project_views::roadmap::build_roadmap_view(
                document,
            ))
            .unwrap();
            true
        }
        "commons" => {
            body.append_child(&crate::browser::project_views::commons::build_commons_view(
                document,
            ))
            .unwrap();
            true
        }
        "agreement_builder" => {
            body.append_child(
                &crate::browser::agreement_views::agreement_builder::build_agreement_builder_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "compensation_model" => {
            body.append_child(
                &crate::browser::agreement_views::compensation_model::build_compensation_model_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "contribution_ledger" => {
            body.append_child(
                &crate::browser::agreement_views::contribution_ledger::build_contribution_ledger_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "license_builder" => {
            body.append_child(
                &crate::browser::agreement_views::license_builder::build_license_builder_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "obligation_tracker" => {
            body.append_child(
                &crate::browser::agreement_views::obligation_tracker::build_obligation_tracker_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "ip_registry" => {
            body.append_child(
                &crate::browser::project_views::ip_registry::build_ip_registry_view(document),
            )
            .unwrap();
            true
        }
        "data_sources" => {
            body.append_child(
                &crate::browser::project_views::data_sources::build_data_sources_view(document),
            )
            .unwrap();
            true
        }
        "disputes" => {
            body.append_child(
                &crate::browser::governance_views::disputes::build_disputes_view(document),
            )
            .unwrap();
            true
        }
        "complaints" => {
            body.append_child(
                &crate::browser::governance_views::complaints::build_complaints_view(document),
            )
            .unwrap();
            true
        }
        "corrections" => {
            body.append_child(
                &crate::browser::governance_views::corrections::build_corrections_view(document),
            )
            .unwrap();
            true
        }
        "governance_meetings" => {
            body.append_child(
                &crate::browser::governance_views::governance_meetings::build_governance_meetings_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "conflict_of_interest" => {
            body.append_child(
                &crate::browser::governance_views::conflict_of_interest::build_conflict_of_interest_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "onboarding" => {
            body.append_child(
                &crate::browser::project_views::onboarding::build_onboarding_view(document),
            )
            .unwrap();
            true
        }
        "bulk_import" => {
            body.append_child(
                &crate::browser::project_views::bulk_import::build_bulk_import_view(document),
            )
            .unwrap();
            true
        }
        "knowledge_base" => {
            body.append_child(
                &crate::browser::project_views::knowledge_base::build_knowledge_base_view(document),
            )
            .unwrap();
            true
        }
        "agent_console" => {
            body.append_child(
                &crate::browser::project_views::agent_console::build_agent_console_view(document),
            )
            .unwrap();
            true
        }
        "awards" => {
            body.append_child(&crate::browser::project_views::awards::build_awards_view(
                document,
            ))
            .unwrap();
            true
        }
        "token_mgr" => {
            body.append_child(
                &crate::browser::project_views::token_mgr::build_token_mgr_view(document),
            )
            .unwrap();
            true
        }
        "dashboard" => {
            body.append_child(
                &crate::browser::project_views::dashboard::build_dashboard_view(document),
            )
            .unwrap();
            true
        }
        "wiki" => {
            body.append_child(&crate::browser::project_views::wiki::build_wiki_view(
                document,
            ))
            .unwrap();
            true
        }
        "governance" => {
            body.append_child(
                &crate::browser::project_views::governance::build_governance_view(document),
            )
            .unwrap();
            true
        }
        "credentials" => {
            body.append_child(
                &crate::browser::project_views::credentials::build_credentials_view(document),
            )
            .unwrap();
            true
        }
        "gantt" => {
            body.append_child(&crate::browser::project_views::gantt::build_gantt_view(
                document,
            ))
            .unwrap();
            true
        }
        "timeline" => {
            body.append_child(
                &crate::browser::project_views::timeline::build_timeline_view(document),
            )
            .unwrap();
            true
        }
        "calendar" => {
            body.append_child(
                &crate::browser::project_views::calendar::build_calendar_view(document),
            )
            .unwrap();
            true
        }
        "doc_mgmt" => {
            body.append_child(
                &crate::browser::project_views::doc_mgmt::build_doc_mgmt_view(document),
            )
            .unwrap();
            true
        }
        "resource_report" => {
            body.append_child(
                &crate::browser::project_views::resource_report::build_resource_report_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "time_tracking" => {
            body.append_child(
                &crate::browser::project_views::time_tracking::build_time_tracking_view(document),
            )
            .unwrap();
            true
        }
        "voting" => {
            body.append_child(&crate::browser::project_views::voting::build_voting_view(
                document,
            ))
            .unwrap();
            true
        }
        "risk" => {
            body.append_child(&crate::browser::project_views::risk::build_risk_view(
                document,
            ))
            .unwrap();
            true
        }
        "task_list" => {
            body.append_child(
                &crate::browser::project_views::task_list::build_task_list_view(document),
            )
            .unwrap();
            true
        }
        "issues" => {
            body.append_child(&crate::browser::project_views::issues::build_issues_view(
                document,
            ))
            .unwrap();
            true
        }
        "asset_mgr" => {
            body.append_child(
                &crate::browser::project_views::asset_mgr::build_asset_mgr_view(document),
            )
            .unwrap();
            true
        }
        "bounties" => {
            body.append_child(
                &crate::browser::project_views::bounties::build_bounties_view(document),
            )
            .unwrap();
            true
        }
        "automation" => {
            body.append_child(
                &crate::browser::project_views::automation::build_automation_view(document),
            )
            .unwrap();
            true
        }
        "analytics" => {
            body.append_child(
                &crate::browser::project_views::analytics::build_analytics_view(document),
            )
            .unwrap();
            true
        }
        "events" => {
            body.append_child(&crate::browser::project_views::events::build_events_view(
                document,
            ))
            .unwrap();
            true
        }
        "news" => {
            body.append_child(&crate::browser::project_views::news::build_news_view(
                document,
            ))
            .unwrap();
            true
        }
        "portfolio" => {
            body.append_child(
                &crate::browser::project_views::portfolio::build_portfolio_view(document),
            )
            .unwrap();
            true
        }
        "integrations" => {
            body.append_child(
                &crate::browser::project_views::integrations::build_integrations_view(document),
            )
            .unwrap();
            true
        }
        "retrospective" => {
            body.append_child(
                &crate::browser::project_views::retrospective::build_retrospective_view(document),
            )
            .unwrap();
            true
        }
        _ => false,
    }
}
