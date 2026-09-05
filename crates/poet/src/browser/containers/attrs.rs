//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Container type tags, strata/epistemic filters, and media-surface labels.

/// Map container type to strata and epistemic categories for filtering.
/// Used by the Strata and Epistemic Lens pods in the top control bar.
pub(super) fn container_type_filter_attrs(container_type: &str) -> (&'static str, &'static str) {
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
        | "health_calculators"
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

pub(super) fn media_surface_for(container_type: &str) -> &'static str {
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

pub(super) fn container_type_tag(container_type: &str) -> (&'static str, &'static str) {
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
        "health_calculators" => ("tag-health", "CALC"),
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

#[cfg(test)]
mod tests {
    use super::{container_type_filter_attrs, container_type_tag, media_surface_for};

    #[test]
    fn health_calculators_stay_environmental_objective() {
        assert_eq!(
            container_type_filter_attrs("health_calculators"),
            ("environmental", "objective")
        );
        assert_eq!(
            container_type_tag("health_calculators"),
            ("tag-health", "CALC")
        );
    }

    #[test]
    fn code_habitat_keeps_vibe_tag() {
        assert_eq!(container_type_tag("code"), ("tag-code", "VIBE"));
        assert_eq!(media_surface_for("code"), "2d");
    }

    #[test]
    fn unknown_type_stays_technical_objective() {
        assert_eq!(
            container_type_filter_attrs("not-a-registered-type"),
            ("technical", "objective")
        );
        assert_eq!(
            container_type_tag("not-a-registered-type"),
            ("tag-default", "CONTAINER")
        );
    }

    #[test]
    fn map_and_film_surfaces_remain_distinct() {
        assert_eq!(media_surface_for("map"), "map");
        assert_eq!(media_surface_for("video_view"), "film");
        assert_eq!(media_surface_for("scene_graph"), "cg");
    }
}
