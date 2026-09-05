//! Container rendering: glassmorphism containers with type tags, badges, ports.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use crate::tool_chest::core::registry::SeedContainer;
use web_sys::{Document, Element};

use super::container_inline_views::{
    build_gis_map_view, build_media_3d_view, build_vibescript_console,
};
use super::specialist_persist;

/// Build a single container node on the canvas.
pub fn build_container(document: &Document, container: &SeedContainer) -> Element {
    let el = document.create_element("div").unwrap();
    el.set_class_name(&format!(
        "canvas-container-node container-card container-kind-{}",
        container.kind.class_suffix()
    ));
    let container_id = if container.id.is_empty() {
        super::canvas_state::next_container_id(&container.container_type)
    } else {
        container.id.clone()
    };
    el.set_attribute("data-id", &container_id).unwrap();
    el.set_attribute("data-shape", "container").unwrap();
    super::surface_aspects::mark(&el, "entrance");
    el.set_attribute(
        "data-media-surface",
        media_surface_for(&container.container_type),
    )
    .unwrap();
    el.set_attribute("data-container-type", &container.container_type)
        .unwrap();
    el.set_attribute("data-semantic-type", &container.semantic_type)
        .unwrap();
    el.set_attribute("data-semantic-uri", &container.semantic_uri)
        .unwrap();
    if !container.target_manifold.is_empty() {
        el.set_attribute("data-target-manifold", &container.target_manifold)
            .unwrap();
    }
    if !container.target_construct.is_empty() {
        el.set_attribute("data-target-construct", &container.target_construct)
            .unwrap();
    }
    el.set_attribute("role", "group").unwrap();
    el.set_attribute("tabindex", "0").unwrap();
    el.set_attribute(
        "aria-label",
        &format!("{} {} container", container.title, container.container_type),
    )
    .unwrap();

    // Set strata and epistemic data attributes for filtering
    let (strata, epistemic) = container_type_filter_attrs(&container.container_type);
    el.set_attribute("data-strata", strata).unwrap();
    el.set_attribute("data-epistemic", epistemic).unwrap();

    let style = format!(
        "left: {}px; top: {}px; width: {}px; height: {}px; z-index: {};",
        container.x.round() as i32,
        container.y.round() as i32,
        container.width.round() as i32,
        container.height.round() as i32,
        container.z.round() as i32
    );
    el.set_attribute("style", &style).unwrap();

    // Header
    let header = document.create_element("div").unwrap();
    header.set_class_name("container-header");

    let title_group = document.create_element("div").unwrap();
    title_group.set_class_name("container-title-group");

    // Type tag
    let tag = document.create_element("span").unwrap();
    let (tag_class, tag_label) = container_type_tag(&container.container_type);
    tag.set_class_name(&format!("container-type-tag {}", tag_class));
    tag.set_text_content(Some(tag_label));
    title_group.append_child(&tag).unwrap();

    // Title
    let title = document.create_element("span").unwrap();
    title.set_class_name("container-title");
    title.set_text_content(Some(&container.title));
    title_group.append_child(&title).unwrap();

    // Honesty badge
    let badge = document.create_element("span").unwrap();
    badge.set_class_name(&format!("honesty-badge honesty-{}", container.honesty));
    badge.set_text_content(Some(&container.honesty));
    title_group.append_child(&badge).unwrap();

    title_group
        .append_child(&super::surface_aspects::chip_row(document))
        .unwrap();

    header.append_child(&title_group).unwrap();

    // Shared lifecycle chrome (settings, minimise, close).
    let actions = super::container_chrome::build_header_actions(document);
    header.append_child(&actions).unwrap();
    el.append_child(&header).unwrap();

    // Body
    let body = document.create_element("div").unwrap();
    body.set_class_name("container-body");

    match container.container_type.as_str() {
        "social" => {
            body.append_child(&super::social_workspace::build_social_view(document))
                .unwrap();
        }
        "connection-requests" => {
            body.append_child(&specialist_persist::build_connection_requests_view(
                document,
            ))
            .unwrap();
        }
        // --- Project container types (Workstream A) ---
        "kanban" => {
            body.append_child(&super::project_views::kanban::build_kanban_view(document))
                .unwrap();
        }
        "project_sheet" => {
            body.append_child(
                &super::project_views::project_sheet::build_project_sheet_view(document),
            )
            .unwrap();
        }
        "budget" => {
            body.append_child(&super::project_views::budget::build_budget_view(document))
                .unwrap();
        }
        "cost_base" => {
            body.append_child(&super::project_views::cost_base::build_cost_base_view(
                document,
            ))
            .unwrap();
        }
        "deliverable" => {
            body.append_child(&super::project_views::deliverable::build_deliverable_view(
                document,
            ))
            .unwrap();
        }
        "review" => {
            body.append_child(&super::project_views::review::build_review_view(document))
                .unwrap();
        }
        "discussion" => {
            body.append_child(&super::project_views::discussion::build_discussion_view(
                document,
            ))
            .unwrap();
        }
        "roadmap" => {
            body.append_child(&super::project_views::roadmap::build_roadmap_view(document))
                .unwrap();
        }
        "commons" => {
            body.append_child(&super::project_views::commons::build_commons_view(document))
                .unwrap();
        }
        "agreement_builder" => {
            body.append_child(
                &super::agreement_views::agreement_builder::build_agreement_builder_view(document),
            )
            .unwrap();
        }
        "compensation_model" => {
            body.append_child(
                &super::agreement_views::compensation_model::build_compensation_model_view(
                    document,
                ),
            )
            .unwrap();
        }
        "contribution_ledger" => {
            body.append_child(
                &super::agreement_views::contribution_ledger::build_contribution_ledger_view(
                    document,
                ),
            )
            .unwrap();
        }
        "license_builder" => {
            body.append_child(
                &super::agreement_views::license_builder::build_license_builder_view(document),
            )
            .unwrap();
        }
        "obligation_tracker" => {
            body.append_child(
                &super::agreement_views::obligation_tracker::build_obligation_tracker_view(
                    document,
                ),
            )
            .unwrap();
        }
        "ip_registry" => {
            body.append_child(&super::project_views::ip_registry::build_ip_registry_view(
                document,
            ))
            .unwrap();
        }
        "data_sources" => {
            body.append_child(
                &super::project_views::data_sources::build_data_sources_view(document),
            )
            .unwrap();
        }
        "disputes" => {
            body.append_child(&super::governance_views::disputes::build_disputes_view(
                document,
            ))
            .unwrap();
        }
        "complaints" => {
            body.append_child(&super::governance_views::complaints::build_complaints_view(
                document,
            ))
            .unwrap();
        }
        "corrections" => {
            body.append_child(
                &super::governance_views::corrections::build_corrections_view(document),
            )
            .unwrap();
        }
        "governance_meetings" => {
            body.append_child(
                &super::governance_views::governance_meetings::build_governance_meetings_view(
                    document,
                ),
            )
            .unwrap();
        }
        "conflict_of_interest" => {
            body.append_child(
                &super::governance_views::conflict_of_interest::build_conflict_of_interest_view(
                    document,
                ),
            )
            .unwrap();
        }
        "onboarding" => {
            body.append_child(&super::project_views::onboarding::build_onboarding_view(
                document,
            ))
            .unwrap();
        }
        "bulk_import" => {
            body.append_child(&super::project_views::bulk_import::build_bulk_import_view(
                document,
            ))
            .unwrap();
        }
        "knowledge_base" => {
            body.append_child(
                &super::project_views::knowledge_base::build_knowledge_base_view(document),
            )
            .unwrap();
        }
        "agent_console" => {
            body.append_child(
                &super::project_views::agent_console::build_agent_console_view(document),
            )
            .unwrap();
        }
        "awards" => {
            body.append_child(&super::project_views::awards::build_awards_view(document))
                .unwrap();
        }
        "token_mgr" => {
            body.append_child(&super::project_views::token_mgr::build_token_mgr_view(
                document,
            ))
            .unwrap();
        }
        "dashboard" => {
            body.append_child(&super::project_views::dashboard::build_dashboard_view(
                document,
            ))
            .unwrap();
        }
        "wiki" => {
            body.append_child(&super::project_views::wiki::build_wiki_view(document))
                .unwrap();
        }
        "governance" => {
            body.append_child(&super::project_views::governance::build_governance_view(
                document,
            ))
            .unwrap();
        }
        "credentials" => {
            body.append_child(&super::project_views::credentials::build_credentials_view(
                document,
            ))
            .unwrap();
        }
        "gantt" => {
            body.append_child(&super::project_views::gantt::build_gantt_view(document))
                .unwrap();
        }
        "timeline" => {
            body.append_child(&super::project_views::timeline::build_timeline_view(
                document,
            ))
            .unwrap();
        }
        "calendar" => {
            body.append_child(&super::project_views::calendar::build_calendar_view(
                document,
            ))
            .unwrap();
        }
        "doc_mgmt" => {
            body.append_child(&super::project_views::doc_mgmt::build_doc_mgmt_view(
                document,
            ))
            .unwrap();
        }
        "resource_report" => {
            body.append_child(
                &super::project_views::resource_report::build_resource_report_view(document),
            )
            .unwrap();
        }
        "time_tracking" => {
            body.append_child(
                &super::project_views::time_tracking::build_time_tracking_view(document),
            )
            .unwrap();
        }
        "voting" => {
            body.append_child(&super::project_views::voting::build_voting_view(document))
                .unwrap();
        }
        "risk" => {
            body.append_child(&super::project_views::risk::build_risk_view(document))
                .unwrap();
        }
        "task_list" => {
            body.append_child(&super::project_views::task_list::build_task_list_view(
                document,
            ))
            .unwrap();
        }
        "issues" => {
            body.append_child(&super::project_views::issues::build_issues_view(document))
                .unwrap();
        }
        "asset_mgr" => {
            body.append_child(&super::project_views::asset_mgr::build_asset_mgr_view(
                document,
            ))
            .unwrap();
        }
        "bounties" => {
            body.append_child(&super::project_views::bounties::build_bounties_view(
                document,
            ))
            .unwrap();
        }
        "automation" => {
            body.append_child(&super::project_views::automation::build_automation_view(
                document,
            ))
            .unwrap();
        }
        "analytics" => {
            body.append_child(&super::project_views::analytics::build_analytics_view(
                document,
            ))
            .unwrap();
        }
        "events" => {
            body.append_child(&super::project_views::events::build_events_view(document))
                .unwrap();
        }
        "news" => {
            body.append_child(&super::project_views::news::build_news_view(document))
                .unwrap();
        }
        "portfolio" => {
            body.append_child(&super::project_views::portfolio::build_portfolio_view(
                document,
            ))
            .unwrap();
        }
        "integrations" => {
            body.append_child(
                &super::project_views::integrations::build_integrations_view(document),
            )
            .unwrap();
        }
        "retrospective" => {
            body.append_child(
                &super::project_views::retrospective::build_retrospective_view(document),
            )
            .unwrap();
        }
        "health_overview" => {
            body.append_child(
                &super::health_views::health_overview::build_health_overview_view(document),
            )
            .unwrap();
        }
        "conditions" => {
            body.append_child(&super::health_views::conditions::build_conditions_view(
                document,
            ))
            .unwrap();
        }
        "clinical_reports" => {
            body.append_child(
                &super::health_views::clinical_reports::build_clinical_reports_view(document),
            )
            .unwrap();
        }
        "lab_results" => {
            body.append_child(&super::health_views::lab_results::build_lab_results_view(
                document,
            ))
            .unwrap();
        }
        "medications" => {
            body.append_child(&super::health_views::medications::build_medications_view(
                document,
            ))
            .unwrap();
        }
        "vitals" => {
            body.append_child(&super::health_views::vitals::build_vitals_view(document))
                .unwrap();
        }
        "mental_wellbeing" => {
            body.append_child(
                &super::health_views::mental_wellbeing::build_mental_wellbeing_view(document),
            )
            .unwrap();
        }
        "therapy_notes" => {
            body.append_child(
                &super::health_views::therapy_notes::build_therapy_notes_view(document),
            )
            .unwrap();
        }
        "sleep" => {
            body.append_child(&super::health_views::sleep::build_sleep_view(document))
                .unwrap();
        }
        "diet" => {
            body.append_child(&super::health_views::diet::build_diet_view(document))
                .unwrap();
        }
        "physical_activity" => {
            body.append_child(
                &super::health_views::physical_activity::build_physical_activity_view(document),
            )
            .unwrap();
        }
        "immunizations" => {
            body.append_child(
                &super::health_views::immunizations::build_immunizations_view(document),
            )
            .unwrap();
        }
        "procedures" => {
            body.append_child(&super::health_views::procedures::build_procedures_view(
                document,
            ))
            .unwrap();
        }
        "family_history" => {
            body.append_child(
                &super::health_views::family_history::build_family_history_view(document),
            )
            .unwrap();
        }
        "hypotheses" => {
            body.append_child(&super::health_views::hypotheses::build_hypotheses_view(
                document,
            ))
            .unwrap();
        }
        "biometrics" => {
            body.append_child(&super::health_views::biometrics::build_biometrics_view(
                document,
            ))
            .unwrap();
        }
        "health_documents" => {
            body.append_child(&super::health_views::documents::build_documents_view(
                document,
            ))
            .unwrap();
        }
        "welfare_support" => {
            body.append_child(
                &super::health_views::welfare_support::build_welfare_support_view(document),
            )
            .unwrap();
        }
        "life_records" => {
            body.append_child(&super::health_views::life_records::build_life_records_view(
                document,
            ))
            .unwrap();
        }
        "authority_attestations" => {
            body.append_child(
                &super::health_views::authority_attestations::build_authority_attestations_view(
                    document,
                ),
            )
            .unwrap();
        }
        "safeguards" => {
            body.append_child(&super::health_views::safeguards::build_safeguards_view(
                document,
            ))
            .unwrap();
        }
        "disclosure_log" => {
            body.append_child(
                &super::health_views::disclosure_log::build_disclosure_log_view(document),
            )
            .unwrap();
        }
        "audio_session" => {
            body.append_child(&super::studio_views::persist::build_audio_session_view(
                document,
            ))
            .unwrap();
        }
        "scene_view" => {
            body.append_child(&super::studio_views::scene_view::build_scene_view(document))
                .unwrap();
        }
        "animation_timeline" => {
            body.append_child(
                &super::studio_views::animation_timeline::build_animation_timeline_view(document),
            )
            .unwrap();
        }
        "desk_surface" => {
            body.append_child(&super::studio_views::desk_surface::build_desk_surface_view(
                document,
            ))
            .unwrap();
        }
        "transport" => {
            body.append_child(&super::studio_views::transport::build_transport_view(
                document,
            ))
            .unwrap();
        }
        "routing_matrix" => {
            body.append_child(
                &super::studio_views::routing_matrix::build_routing_matrix_view(document),
            )
            .unwrap();
        }
        "spatial_audio" => {
            body.append_child(
                &super::studio_views::spatial_audio::build_spatial_audio_view(document),
            )
            .unwrap();
        }
        "dataset_registry" => {
            body.append_child(
                &super::dataset_views::dataset_registry::build_dataset_registry_view(document),
            )
            .unwrap();
        }
        "dataset_importer" => {
            body.append_child(
                &super::dataset_views::dataset_importer::build_dataset_importer_view(document),
            )
            .unwrap();
        }
        "presentation_editor" => {
            body.append_child(
                &super::dataset_views::presentation_editor::build_presentation_editor_view(
                    document,
                ),
            )
            .unwrap();
        }
        "view_canvas" => {
            body.append_child(&super::dataset_views::view_canvas::build_view_canvas_view(
                document,
            ))
            .unwrap();
        }
        "scene_graph" => {
            body.append_child(&super::studio_views::scene_graph::build_scene_graph_view(
                document,
            ))
            .unwrap();
        }
        "material_editor" => {
            body.append_child(
                &super::studio_views::material_editor::build_material_editor_view(document),
            )
            .unwrap();
        }
        "lighting_editor" => {
            body.append_child(
                &super::studio_views::lighting_editor::build_lighting_editor_view(document),
            )
            .unwrap();
        }
        "tensor_inspector" => {
            body.append_child(
                &super::studio_views::tensor_inspector::build_tensor_inspector_view(document),
            )
            .unwrap();
        }
        "asset_library" => {
            body.append_child(
                &super::studio_views::asset_library::build_asset_library_view(document),
            )
            .unwrap();
        }
        "channel_strip" => {
            body.append_child(
                &super::studio_views::channel_strip::build_channel_strip_view(document),
            )
            .unwrap();
        }
        "meter_bridge" => {
            body.append_child(&super::studio_views::meter_bridge::build_meter_bridge_view(
                document,
            ))
            .unwrap();
        }
        "automation_lanes" => {
            body.append_child(
                &super::studio_views::automation_lanes::build_automation_lanes_view(document),
            )
            .unwrap();
        }
        "annotation_panel" => {
            body.append_child(
                &super::dataset_views::annotation_panel::build_annotation_panel_view(document),
            )
            .unwrap();
        }
        "lineage_graph" => {
            body.append_child(
                &super::dataset_views::lineage_graph::build_lineage_graph_view(document),
            )
            .unwrap();
        }
        "lod_chain" => {
            body.append_child(&super::studio_views::lod_chain::build_lod_chain_view(
                document,
            ))
            .unwrap();
        }
        "shadow_settings" => {
            body.append_child(
                &super::studio_views::shadow_settings::build_shadow_settings_view(document),
            )
            .unwrap();
        }
        "gis_maps" => {
            body.append_child(&super::studio_views::gis_maps::build_gis_maps_view(
                document,
            ))
            .unwrap();
        }
        "ragdoll_skin" => {
            body.append_child(&super::studio_views::ragdoll_skin::build_ragdoll_skin_view(
                document,
            ))
            .unwrap();
        }
        "animation_export" => {
            body.append_child(
                &super::studio_views::animation_export::build_animation_export_view(document),
            )
            .unwrap();
        }
        "desk_persistence" => {
            body.append_child(
                &super::studio_views::desk_persistence::build_desk_persistence_view(document),
            )
            .unwrap();
        }
        "hrtf_personalization" => {
            body.append_child(
                &super::studio_views::hrtf_personalization::build_hrtf_personalization_view(
                    document,
                ),
            )
            .unwrap();
        }
        "manifold_transition_audio" => {
            body.append_child(
                &super::studio_views::manifold_transition_audio::build_manifold_transition_audio_view(document),
            )
            .unwrap();
        }
        "video_view" => {
            body.append_child(&super::dataset_views::video_view::build_video_view_view(
                document,
            ))
            .unwrap();
        }
        "presentation_publish" => {
            body.append_child(
                &super::dataset_views::presentation_publish::build_presentation_publish_view(
                    document,
                ),
            )
            .unwrap();
        }
        "super_resolve" => {
            body.append_child(
                &super::dataset_views::super_resolve::build_super_resolve_view(document),
            )
            .unwrap();
        }
        "cad_curation" => {
            body.append_child(
                &super::dataset_views::cad_curation::build_cad_curation_view(document),
            )
            .unwrap();
        }
        // Ontology Workbench P0
        "ontology_graph_canvas" => {
            body.append_child(
                &super::ontology_views::graph_canvas::build_ontology_graph_canvas_view(document),
            )
            .unwrap();
        }
        "ontology_library" => {
            body.append_child(
                &super::ontology_views::ontology_library::build_ontology_library_view(document),
            )
            .unwrap();
        }
        "vocabulary_mapper" => {
            body.append_child(
                &super::ontology_views::vocabulary_mapper::build_vocabulary_mapper_view(document),
            )
            .unwrap();
        }
        "relation_builder" => {
            body.append_child(
                &super::ontology_views::relation_builder::build_relation_builder_view(document),
            )
            .unwrap();
        }
        "shacl_shapes" => {
            body.append_child(
                &super::ontology_views::shacl_shapes::build_shacl_shapes_view(document),
            )
            .unwrap();
        }
        "n3_editor" => {
            body.append_child(&super::ontology_views::n3_editor::build_n3_editor_view(
                document,
            ))
            .unwrap();
        }
        "shex_editor" => {
            body.append_child(&super::ontology_views::shex_editor::build_shex_editor_view(
                document,
            ))
            .unwrap();
        }
        "ontology_compare" => {
            body.append_child(
                &super::ontology_views::ontology_compare::build_ontology_compare_view(document),
            )
            .unwrap();
        }
        "project_ontology_selector" => {
            body.append_child(
                &super::ontology_views::project_ontology_selector::build_project_ontology_selector_view(document),
            )
            .unwrap();
        }
        // Device Workbench P0
        "device_manager" => {
            body.append_child(
                &super::device_views::device_manager::build_device_manager_view(document),
            )
            .unwrap();
        }
        "display_layout" => {
            body.append_child(
                &super::device_views::display_layout::build_display_layout_view(document),
            )
            .unwrap();
        }
        "workspace_sync" => {
            body.append_child(
                &super::device_views::workspace_sync::build_workspace_sync_view(document),
            )
            .unwrap();
        }
        "device_role_assigner" => {
            body.append_child(
                &super::device_views::device_role_assigner::build_device_role_assigner_view(
                    document,
                ),
            )
            .unwrap();
        }
        "remote_control" => {
            body.append_child(
                &super::device_views::remote_control::build_remote_control_view(document),
            )
            .unwrap();
        }
        "reputation" => {
            body.append_child(&specialist_persist::build_reputation_view(document))
                .unwrap();
        }
        "protection-policies" => {
            body.append_child(&specialist_persist::build_protection_policies_view(
                document,
            ))
            .unwrap();
        }
        "capabilities" => {
            body.append_child(&specialist_persist::build_capabilities_view(document))
                .unwrap();
        }
        "settings" => {
            body.append_child(&specialist_persist::build_settings_view(document))
                .unwrap();
        }
        "conversations" => {
            body.append_child(&specialist_persist::build_conversations_view(document))
                .unwrap();
        }
        "channels" => {
            body.append_child(&specialist_persist::build_channels_view(document))
                .unwrap();
        }
        "presence" => {
            body.append_child(&specialist_persist::build_presence_view(document))
                .unwrap();
        }
        "dual_studio" | "dual-studio" => {
            body.append_child(&super::studio_views::dual_studio::build_dual_studio_view(
                document,
            ))
            .unwrap();
        }
        "webrtc_sync" | "webrtc-sync" => {
            body.append_child(&super::webrtc_sync::build_webrtc_sync_view(document))
                .unwrap();
        }
        "admin_launcher" | "admin-launcher" | "app_launcher" => {
            body.append_child(&super::app_launcher::build_admin_launcher_view(document))
                .unwrap();
        }
        "solid_interop" | "solid-interop" => {
            let bundle = super::solid_interop::SolidPodBundle::new(
                "https://solid.webizen.id/profile/card#me",
            );
            body.append_child(&super::solid_interop::build_solid_pod_hub_view(
                document, &bundle,
            ))
            .unwrap();
        }
        "shader_pipelines" | "shader-pipelines" => {
            body.append_child(&super::shader_pipelines::build_shader_pipeline_view(
                document,
            ))
            .unwrap();
        }
        "cooperative_economics" | "cooperative-economics" => {
            body.append_child(
                &super::cooperative_economics::build_cooperative_economics_view(
                    document,
                    &super::cooperative_economics::TrueCostModel::default(),
                ),
            )
            .unwrap();
        }
        "map" => {
            body.append_child(&build_gis_map_view(document)).unwrap();
        }
        "media" => {
            body.append_child(&build_media_3d_view(document)).unwrap();
        }
        "code" => {
            body.append_child(&build_vibescript_console(document))
                .unwrap();
        }
        "doc" => {
            body.append_child(&super::container_views::build_doc_view(document))
                .unwrap();
        }
        "sheet" => {
            body.append_child(&super::sheet::build_sheet_view(
                document,
                &container.tool_settings,
            ))
            .unwrap();
        }
        "graph" => {
            body.append_child(&super::container_views::build_graph_view(document))
                .unwrap();
        }
        "ontology" => {
            body.append_child(&super::container_views::build_ontology_view(document))
                .unwrap();
        }
        "pulse" => {
            body.append_child(&super::container_views::build_pulse_view(document))
                .unwrap();
        }
        "rights" => {
            body.append_child(&super::rights_views::build_rights_view(document))
                .unwrap();
        }
        "wallet" => {
            body.append_child(&specialist_persist::build_wallet_view(document))
                .unwrap();
        }
        "library" => {
            body.append_child(&super::semantic_library_view::build(document))
                .unwrap();
        }
        "aura" => {
            body.append_child(&specialist_persist::build_aura_view(document))
                .unwrap();
        }
        "latex" => {
            body.append_child(&super::local_container_views::build_latex_view(document))
                .unwrap();
        }
        "health" => {
            body.append_child(&specialist_persist::build_health_vault_view(document))
                .unwrap();
        }
        "mail" => {
            let mailbox = super::mail_composer::MailboxManager::new("personal.example");
            body.append_child(&super::mail_composer::build_mailbox_view(
                document, &mailbox,
            ))
            .unwrap();
        }
        "anatomy" => {
            body.append_child(&specialist_persist::build_anatomy_view(document))
                .unwrap();
        }
        "webview" => {
            body.append_child(&specialist_persist::build_webview_view(document))
                .unwrap();
        }
        "webrtc" => {
            body.append_child(&specialist_persist::build_webrtc_view(document))
                .unwrap();
        }
        "finance" => {
            body.append_child(&specialist_persist::build_finance_view(document))
                .unwrap();
        }
        "vision" => {
            body.append_child(&specialist_persist::build_vision_view(document))
                .unwrap();
        }
        "listen" => {
            body.append_child(&specialist_persist::build_listen_view(document))
                .unwrap();
        }
        "triad" => {
            body.append_child(&specialist_persist::build_triad_view(document))
                .unwrap();
        }
        "portal" => {
            body.append_child(&specialist_persist::build_portal_view(document))
                .unwrap();
        }
        "slide" => {
            body.append_child(&super::local_container_views::build_slide_view(document))
                .unwrap();
        }
        "3d" => {
            body.append_child(&specialist_persist::build_3d_view(document))
                .unwrap();
        }
        "subcanvas" | "nested_manifold" => {
            let target = if container.target_manifold.is_empty() {
                "research"
            } else {
                container.target_manifold.as_str()
            };
            body.append_child(&super::construct_shelf::build_nested_manifold_view(
                document, target,
            ))
            .unwrap();
        }
        "construct_shelf" => {
            body.append_child(&super::construct_shelf::build_construct_shelf_view(
                document,
            ))
            .unwrap();
        }
        "construct_portal" => {
            body.append_child(&super::construct_shelf::build_construct_portal_view(
                document,
                &container.target_construct,
                &container.target_manifold,
            ))
            .unwrap();
        }
        "domain_lab" => {
            body.append_child(&super::logic_workbench::build_domain_lab_view(document))
                .unwrap();
        }
        "subject" => {
            body.append_child(&super::construct_shelf::build_subject_view(
                document, container,
            ))
            .unwrap();
        }
        "participants" => {
            body.append_child(&super::manifold_social::build_participants_view(document))
                .unwrap();
        }
        // --- Workflow panel containers (see SAVE_ARCHITECTURE.md) ---
        "checkpoint-tray" => {
            body.append_child(&super::checkpoint_panel::build_checkpoint_tray_view(
                document,
            ))
            .unwrap();
        }
        "credential-inspector" => {
            body.append_child(&super::governance_workflow::build_credential_view(document))
                .unwrap();
        }
        "context-markup-editor" => {
            body.append_child(&super::governance_workflow::build_context_markup_view(
                document,
            ))
            .unwrap();
        }
        "provenance-panel" => {
            body.append_child(&super::governance_workflow::build_provenance_view(document))
                .unwrap();
        }
        "publication-workflow" => {
            body.append_child(&super::publication_panel::build_publication_workflow_view(
                document,
            ))
            .unwrap();
        }
        "constituency-manager" => {
            body.append_child(&super::governance_workflow::build_constituency_view(
                document,
            ))
            .unwrap();
        }
        // --- Widget containers ---
        "capability-badge" => {
            body.append_child(&super::governance_workflow::build_capability_badge_view(
                document,
            ))
            .unwrap();
        }
        "checkpoint-indicator" => {
            body.append_child(&super::checkpoint_panel::build_checkpoint_indicator_view(
                document,
            ))
            .unwrap();
        }
        "consent-indicator" => {
            body.append_child(&super::governance_workflow::build_consent_view(document))
                .unwrap();
        }
        _ => {
            let ph = document.create_element("div").unwrap();
            ph.set_class_name("container-placeholder");
            ph.set_text_content(Some(&format!(
                "Unavailable: no standalone renderer is registered for container type `{}` ({}).",
                container.container_type, container.title
            )));
            ph.set_attribute("role", "status").unwrap();
            ph.set_attribute("data-honesty", "unavailable").unwrap();
            body.append_child(&ph).unwrap();
        }
    }
    if !container.content_html.is_empty() {
        if let Ok(Some(editor)) = body.query_selector(".doc-editor") {
            editor.set_inner_html(&container.content_html);
        }
    }
    el.append_child(&body).unwrap();
    super::tool_widgets::restore_container_settings(&el, &container.tool_settings);
    super::view_state::restore(&el, &container.view_state);
    super::surface_honesty::enforce(document, &body, &container.container_type);

    // Connection ports
    let port_in = document.create_element("button").unwrap();
    port_in.set_class_name("container-port port-in");
    port_in.set_attribute("type", "button").unwrap();
    port_in
        .set_attribute(
            "aria-label",
            "Input port: connect an incoming semantic wire",
        )
        .unwrap();
    port_in.set_attribute("data-port", "in").unwrap();
    port_in
        .set_attribute("title", "Input Port: drop incoming reactive wire here")
        .unwrap();
    el.append_child(&port_in).unwrap();

    let port_out = document.create_element("button").unwrap();
    port_out.set_class_name("container-port port-out");
    port_out.set_attribute("type", "button").unwrap();
    port_out
        .set_attribute("aria-label", "Output port: start a semantic wire")
        .unwrap();
    port_out.set_attribute("data-port", "out").unwrap();
    port_out
        .set_attribute(
            "title",
            "Output Port: drag to connect reactive wire to another container",
        )
        .unwrap();
    el.append_child(&port_out).unwrap();

    // Resize handle
    let resizer = document.create_element("div").unwrap();
    resizer.set_class_name("container-resizer resize-handle");
    resizer
        .set_attribute("title", "Drag to resize container")
        .unwrap();
    resizer.set_attribute("role", "separator").unwrap();
    resizer
        .set_attribute("aria-label", "Resize container")
        .unwrap();
    el.append_child(&resizer).unwrap();

    super::container_chrome::restore_chrome_state(&el, container);

    el
}

/// Map container type to CSS tag class and label.
/// Map container type to strata and epistemic categories for filtering.
/// Used by the Strata and Epistemic Lens pods in the top control bar.
fn container_type_filter_attrs(container_type: &str) -> (&'static str, &'static str) {
    match container_type {
        // Social strata
        "social"
        | "connection-requests"
        | "reputation"
        | "conversations"
        | "channels"
        | "presence"
        | "webrtc"
        | "webview" => ("social", "intersubjective"),

        // Environmental strata
        "map"
        | "media"
        | "3d"
        | "dual_studio"
        | "audio_session"
        | "vision"
        | "listen"
        | "triad"
        | "portal"
        | "anatomy"
        | "health"
        | "health_overview"
        | "conditions"
        | "clinical_reports"
        | "lab_results"
        | "medications"
        | "vitals"
        | "mental_wellbeing"
        | "therapy_notes"
        | "sleep"
        | "diet"
        | "physical_activity"
        | "immunizations"
        | "procedures"
        | "family_history"
        | "hypotheses"
        | "biometrics"
        | "health_documents"
        | "welfare_support"
        | "life_records"
        | "authority_attestations"
        | "safeguards"
        | "disclosure_log"
        | "scene_view"
        | "animation_timeline"
        | "desk_surface"
        | "transport"
        | "routing_matrix"
        | "spatial_audio"
        | "dataset_registry"
        | "dataset_importer"
        | "presentation_editor"
        | "view_canvas"
        | "scene_graph"
        | "material_editor"
        | "lighting_editor"
        | "tensor_inspector"
        | "asset_library"
        | "channel_strip"
        | "meter_bridge"
        | "automation_lanes"
        | "annotation_panel"
        | "lineage_graph"
        | "lod_chain"
        | "shadow_settings"
        | "gis_maps"
        | "ragdoll_skin"
        | "animation_export"
        | "desk_persistence"
        | "hrtf_personalization"
        | "manifold_transition_audio"
        | "video_view"
        | "presentation_publish"
        | "super_resolve"
        | "cad_curation"
        | "ontology_graph_canvas"
        | "ontology_library"
        | "vocabulary_mapper"
        | "relation_builder"
        | "shacl_shapes"
        | "n3_editor"
        | "shex_editor"
        | "ontology_compare"
        | "project_ontology_selector"
        | "device_manager"
        | "display_layout"
        | "workspace_sync"
        | "device_role_assigner"
        | "remote_control" => ("environmental", "objective"),

        // Legal strata
        "rights" | "protection-policies" | "capabilities" => ("legal", "normative"),

        // Financial strata
        "finance" | "wallet" => ("financial", "objective"),

        // Technical strata
        "code" | "ontology" | "graph" | "doc" | "sheet" | "latex" | "slide" | "subcanvas"
        | "pulse" | "aura" | "library" => ("technical", "objective"),

        // Workflow panels — technical
        "checkpoint-tray"
        | "credential-inspector"
        | "context-markup-editor"
        | "provenance-panel"
        | "publication-workflow"
        | "constituency-manager"
        | "capability-badge"
        | "checkpoint-indicator"
        | "consent-indicator" => ("technical", "normative"),

        // Project/ERP containers — governance strata
        "kanban"
        | "project_sheet"
        | "budget"
        | "cost_base"
        | "deliverable"
        | "review"
        | "discussion"
        | "roadmap"
        | "commons"
        | "agreement_builder"
        | "compensation_model"
        | "contribution_ledger"
        | "license_builder"
        | "obligation_tracker"
        | "ip_registry"
        | "data_sources"
        | "disputes"
        | "complaints"
        | "corrections"
        | "governance_meetings"
        | "conflict_of_interest"
        | "onboarding"
        | "bulk_import"
        | "knowledge_base"
        | "agent_console"
        | "awards"
        | "token_mgr"
        | "dashboard"
        | "wiki"
        | "governance"
        | "credentials"
        | "gantt"
        | "timeline"
        | "calendar"
        | "doc_mgmt"
        | "resource_report"
        | "time_tracking"
        | "voting"
        | "risk"
        | "task_list"
        | "issues"
        | "asset_mgr"
        | "bounties"
        | "automation"
        | "analytics"
        | "events"
        | "news"
        | "portfolio"
        | "integrations"
        | "retrospective" => ("governance", "normative"),

        // Settings — technical/normative
        "settings" => ("technical", "normative"),

        _ => ("technical", "objective"),
    }
}

fn media_surface_for(container_type: &str) -> &'static str {
    match container_type {
        "media" | "video_view" | "animation_timeline" | "animation_export" | "transport"
        | "channel_strip" | "meter_bridge" => "film",
        "dual_studio" | "dual-studio" | "lighting_editor" | "material_editor"
        | "shader_pipelines" | "scene_graph" | "scene_view" | "lod_chain" | "ragdoll_skin"
        | "shadow_settings" | "spatial_10d" | "tensor_inspector" => "cg",
        "map" | "gis_maps" => "map",
        "media_3d" | "spatial" => "3d",
        _ => "2d",
    }
}

fn container_type_tag(container_type: &str) -> (&'static str, &'static str) {
    match container_type {
        "social" => ("tag-social", "SOCIAL"),
        "connection-requests" => ("tag-social", "SOCIAL"),
        "reputation" => ("tag-social", "SOCIAL"),
        "protection-policies" => ("tag-ontology", "ONTOLOGY"),
        "settings" => ("tag-default", "CONFIG"),
        "capabilities" => ("tag-ontology", "CAPS"),
        "conversations" => ("tag-social", "CHAT"),
        "channels" => ("tag-webrtc", "RTC"),
        "presence" => ("tag-social", "PRESENCE"),
        "map" => ("tag-map", "GIS"),
        "media" => ("tag-media", "MEDIA"),
        "code" => ("tag-code", "VIBE"),
        "doc" => ("tag-doc", "DOC"),
        // Workflow panel containers
        "checkpoint-tray" => ("tag-workflow", "WORKFLOW"),
        "credential-inspector" => ("tag-workflow", "CREDENTIAL"),
        "context-markup-editor" => ("tag-workflow", "MARKUP"),
        "provenance-panel" => ("tag-workflow", "PROVENANCE"),
        "publication-workflow" => ("tag-workflow", "PUBLISH"),
        "constituency-manager" => ("tag-workflow", "CONSTITUENCY"),
        // Widget containers
        "capability-badge" => ("tag-widget", "BADGE"),
        "checkpoint-indicator" => ("tag-widget", "INDICATOR"),
        "consent-indicator" => ("tag-widget", "INDICATOR"),
        "kanban" => ("tag-workflow", "KANBAN"),
        "project_sheet" => ("tag-workflow", "PROJECT"),
        "budget" => ("tag-workflow", "BUDGET"),
        "cost_base" => ("tag-workflow", "COST"),
        "deliverable" => ("tag-workflow", "DELIVER"),
        "review" => ("tag-workflow", "REVIEW"),
        "discussion" => ("tag-social", "DISCUSS"),
        "roadmap" => ("tag-workflow", "ROADMAP"),
        "commons" => ("tag-governance", "COMMONS"),
        "agreement_builder" => ("tag-governance", "AGREE"),
        "compensation_model" => ("tag-governance", "COMP"),
        "contribution_ledger" => ("tag-governance", "LEDGER"),
        "license_builder" => ("tag-governance", "LICENSE"),
        "obligation_tracker" => ("tag-governance", "OBLIG"),
        "ip_registry" => ("tag-governance", "IP"),
        "data_sources" => ("tag-governance", "DATA"),
        "disputes" => ("tag-governance", "DISPUTE"),
        "complaints" => ("tag-governance", "COMPLAIN"),
        "corrections" => ("tag-governance", "CORRECT"),
        "governance_meetings" => ("tag-governance", "MEET"),
        "conflict_of_interest" => ("tag-governance", "COI"),
        "onboarding" => ("tag-governance", "ONBOARD"),
        "bulk_import" => ("tag-governance", "BULK"),
        "knowledge_base" => ("tag-governance", "KB"),
        "agent_console" => ("tag-governance", "AGENT"),
        "awards" => ("tag-governance", "AWARD"),
        "token_mgr" => ("tag-governance", "TOKEN"),
        "dashboard" => ("tag-governance", "DASH"),
        "wiki" => ("tag-governance", "WIKI"),
        "governance" => ("tag-governance", "GOV"),
        "credentials" => ("tag-governance", "CRED"),
        "gantt" => ("tag-governance", "GANTT"),
        "timeline" => ("tag-governance", "TL"),
        "calendar" => ("tag-governance", "CAL"),
        "doc_mgmt" => ("tag-governance", "DOC"),
        "resource_report" => ("tag-governance", "RES"),
        "time_tracking" => ("tag-governance", "TIME"),
        "voting" => ("tag-governance", "VOTE"),
        "risk" => ("tag-governance", "RISK"),
        "task_list" => ("tag-governance", "TASK"),
        "issues" => ("tag-governance", "ISS"),
        "asset_mgr" => ("tag-governance", "ASSET"),
        "bounties" => ("tag-governance", "BOUNTY"),
        "automation" => ("tag-governance", "AUTO"),
        "analytics" => ("tag-governance", "ANALYT"),
        "events" => ("tag-governance", "EVT"),
        "news" => ("tag-governance", "NEWS"),
        "portfolio" => ("tag-governance", "PORT"),
        "integrations" => ("tag-governance", "INTG"),
        "retrospective" => ("tag-governance", "RETRO"),
        "health_overview" => ("tag-health", "HOVR"),
        "conditions" => ("tag-health", "COND"),
        "clinical_reports" => ("tag-health", "CLIN"),
        "lab_results" => ("tag-health", "LAB"),
        "medications" => ("tag-health", "MED"),
        "vitals" => ("tag-health", "VIT"),
        "mental_wellbeing" => ("tag-health", "MWB"),
        "therapy_notes" => ("tag-health", "TNOTES"),
        "sleep" => ("tag-health", "SLEEP"),
        "diet" => ("tag-health", "DIET"),
        "physical_activity" => ("tag-health", "ACT"),
        "immunizations" => ("tag-health", "IMM"),
        "procedures" => ("tag-health", "PROC"),
        "family_history" => ("tag-health", "FAM"),
        "hypotheses" => ("tag-health", "HYP"),
        "biometrics" => ("tag-health", "BIO"),
        "health_documents" => ("tag-health", "HDOC"),
        "welfare_support" => ("tag-health", "WELF"),
        "life_records" => ("tag-health", "LIFE"),
        "authority_attestations" => ("tag-health", "ATT"),
        "safeguards" => ("tag-health", "SAFE"),
        "disclosure_log" => ("tag-health", "DCL"),
        "audio_session" => ("tag-studio", "AUD"),
        "dual_studio" => ("tag-studio", "DUAL"),
        "scene_view" => ("tag-studio", "3D"),
        "animation_timeline" => ("tag-studio", "ANI"),
        "desk_surface" => ("tag-studio", "DESK"),
        "transport" => ("tag-studio", "TRP"),
        "routing_matrix" => ("tag-studio", "ROUTE"),
        "spatial_audio" => ("tag-studio", "SPAT"),
        "dataset_registry" => ("tag-dataset", "DSREG"),
        "dataset_importer" => ("tag-dataset", "DSIMP"),
        "presentation_editor" => ("tag-dataset", "PRES"),
        "view_canvas" => ("tag-dataset", "VCAN"),
        "scene_graph" => ("tag-studio", "SGRP"),
        "material_editor" => ("tag-studio", "MAT"),
        "lighting_editor" => ("tag-studio", "LITE"),
        "tensor_inspector" => ("tag-studio", "TNSP"),
        "asset_library" => ("tag-studio", "LIB"),
        "channel_strip" => ("tag-studio", "CHST"),
        "meter_bridge" => ("tag-studio", "MTR"),
        "automation_lanes" => ("tag-studio", "AUTO"),
        "annotation_panel" => ("tag-dataset", "ANN"),
        "lineage_graph" => ("tag-dataset", "LIN"),
        "lod_chain" => ("tag-studio", "LOD"),
        "shadow_settings" => ("tag-studio", "SHDW"),
        "gis_maps" => ("tag-studio", "GIS"),
        "ragdoll_skin" => ("tag-studio", "RAGD"),
        "animation_export" => ("tag-studio", "AEXP"),
        "desk_persistence" => ("tag-studio", "DSKP"),
        "hrtf_personalization" => ("tag-studio", "HRTF"),
        "manifold_transition_audio" => ("tag-studio", "MTA"),
        "video_view" => ("tag-dataset", "VID"),
        "presentation_publish" => ("tag-dataset", "PUB"),
        "super_resolve" => ("tag-dataset", "SR"),
        "cad_curation" => ("tag-dataset", "CAD"),
        "ontology_graph_canvas" => ("tag-ontology", "GCAN"),
        "ontology_library" => ("tag-ontology", "OLIB"),
        "vocabulary_mapper" => ("tag-ontology", "VMAP"),
        "relation_builder" => ("tag-ontology", "REL"),
        "shacl_shapes" => ("tag-ontology", "SHAC"),
        "n3_editor" => ("tag-ontology", "N3"),
        "shex_editor" => ("tag-ontology", "SHEX"),
        "ontology_compare" => ("tag-ontology", "CMP"),
        "project_ontology_selector" => ("tag-ontology", "PSEL"),
        "device_manager" => ("tag-device", "DEVM"),
        "display_layout" => ("tag-device", "DISP"),
        "workspace_sync" => ("tag-device", "SYNC"),
        "device_role_assigner" => ("tag-device", "ROLE"),
        "remote_control" => ("tag-device", "RMOT"),
        "subject" => ("tag-knowledge", "SUBJ"),
        "participants" => ("tag-social", "PEOPLE"),
        "nested_manifold" | "subcanvas" => ("tag-default", "NEST"),
        "construct_portal" | "construct_shelf" => ("tag-default", "CSTR"),
        _ => ("tag-default", "CONTAINER"),
    }
}
