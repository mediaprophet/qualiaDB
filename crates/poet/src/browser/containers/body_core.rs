//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Social, specialist, core habitat, and workflow container bodies.
use crate::tool_chest::core::registry::SeedContainer;
use web_sys::{Document, Element};

pub(super) fn try_fill(document: &Document, container: &SeedContainer, body: &Element) -> bool {
    match container.container_type.as_str() {
        "social" => {
            body.append_child(&crate::browser::social_workspace::build_social_view(
                document,
            ))
            .unwrap();
            true
        }
        "connection-requests" => {
            body.append_child(
                &crate::browser::specialist_persist::build_connection_requests_view(document),
            )
            .unwrap();
            true
        }
        "reputation" => {
            body.append_child(&crate::browser::specialist_persist::build_reputation_view(
                document,
            ))
            .unwrap();
            true
        }
        "protection-policies" => {
            body.append_child(
                &crate::browser::specialist_persist::build_protection_policies_view(document),
            )
            .unwrap();
            true
        }
        "capabilities" => {
            body.append_child(
                &crate::browser::specialist_persist::build_capabilities_view(document),
            )
            .unwrap();
            true
        }
        "settings" => {
            body.append_child(&crate::browser::specialist_persist::build_settings_view(
                document,
            ))
            .unwrap();
            true
        }
        "conversations" => {
            body.append_child(
                &crate::browser::specialist_persist::build_conversations_view(document),
            )
            .unwrap();
            true
        }
        "channels" => {
            body.append_child(&crate::browser::specialist_persist::build_channels_view(
                document,
            ))
            .unwrap();
            true
        }
        "presence" => {
            body.append_child(&crate::browser::specialist_persist::build_presence_view(
                document,
            ))
            .unwrap();
            true
        }
        "dual_studio" | "dual-studio" => {
            body.append_child(
                &crate::browser::studio_views::dual_studio::build_dual_studio_view(document),
            )
            .unwrap();
            true
        }
        "webrtc_sync" | "webrtc-sync" => {
            body.append_child(&crate::browser::webrtc_sync::build_webrtc_sync_view(
                document,
            ))
            .unwrap();
            true
        }
        "admin_launcher" | "admin-launcher" | "app_launcher" => {
            body.append_child(&crate::browser::app_launcher::build_admin_launcher_view(
                document,
            ))
            .unwrap();
            true
        }
        "solid_interop" | "solid-interop" => {
            let bundle = crate::browser::solid_interop::SolidPodBundle::new(
                "https://solid.webizen.id/profile/card#me",
            );
            body.append_child(&crate::browser::solid_interop::build_solid_pod_hub_view(
                document, &bundle,
            ))
            .unwrap();
            true
        }
        "shader_pipelines" | "shader-pipelines" => {
            body.append_child(
                &crate::browser::shader_pipelines::build_shader_pipeline_view(document),
            )
            .unwrap();
            true
        }
        "cooperative_economics" | "cooperative-economics" => {
            body.append_child(
                &crate::browser::cooperative_economics::build_cooperative_economics_view(
                    document,
                    &crate::browser::cooperative_economics::TrueCostModel::default(),
                ),
            )
            .unwrap();
            true
        }
        "map" => {
            body.append_child(&crate::browser::container_inline_views::build_gis_map_view(
                document,
            ))
            .unwrap();
            true
        }
        "media" => {
            body.append_child(
                &crate::browser::container_inline_views::build_media_3d_view(document),
            )
            .unwrap();
            true
        }
        "code" => {
            // Full Code IDE habitat (Zone D Catalog · Lexicon) for UAT click-path.
            // Lightweight Script cells still use vibe-console via other entry points.
            let ide = crate::browser::ide::build_ide_view(
                document,
                &crate::browser::ide::IdeState::default(),
            );
            ide.set_attribute("data-code-habitat", "ide").ok();
            body.append_child(&ide).unwrap();
            true
        }
        "doc" => {
            body.append_child(&crate::browser::container_views::build_doc_view(document))
                .unwrap();
            true
        }
        "sheet" => {
            body.append_child(&crate::browser::sheet::build_sheet_view(
                document,
                &container.tool_settings,
            ))
            .unwrap();
            true
        }
        "graph" => {
            body.append_child(&crate::browser::container_views::build_graph_view(document))
                .unwrap();
            true
        }
        "ontology" => {
            body.append_child(&crate::browser::container_views::build_ontology_view(
                document,
            ))
            .unwrap();
            true
        }
        "pulse" => {
            body.append_child(&crate::browser::container_views::build_pulse_view(document))
                .unwrap();
            true
        }
        "rights" => {
            body.append_child(&crate::browser::rights_views::build_rights_view(document))
                .unwrap();
            true
        }
        "wallet" => {
            body.append_child(&crate::browser::specialist_persist::build_wallet_view(
                document,
            ))
            .unwrap();
            true
        }
        "library" => {
            body.append_child(&crate::browser::semantic_library_view::build(document))
                .unwrap();
            true
        }
        "aura" => {
            body.append_child(&crate::browser::specialist_persist::build_aura_view(
                document,
            ))
            .unwrap();
            true
        }
        "latex" => {
            body.append_child(&crate::browser::local_container_views::build_latex_view(
                document,
            ))
            .unwrap();
            true
        }
        "health" => {
            body.append_child(
                &crate::browser::specialist_persist::build_health_vault_view(document),
            )
            .unwrap();
            true
        }
        "mail" => {
            let mailbox = crate::browser::mail_composer::MailboxManager::new("personal.example");
            body.append_child(&crate::browser::mail_composer::build_mailbox_view(
                document, &mailbox,
            ))
            .unwrap();
            true
        }
        "anatomy" => {
            body.append_child(&crate::browser::specialist_persist::build_anatomy_view(
                document,
            ))
            .unwrap();
            true
        }
        "webview" => {
            body.append_child(&crate::browser::specialist_persist::build_webview_view(
                document,
            ))
            .unwrap();
            true
        }
        "webrtc" => {
            body.append_child(&crate::browser::specialist_persist::build_webrtc_view(
                document,
            ))
            .unwrap();
            true
        }
        "finance" => {
            body.append_child(&crate::browser::specialist_persist::build_finance_view(
                document,
            ))
            .unwrap();
            true
        }
        "vision" => {
            body.append_child(&crate::browser::specialist_persist::build_vision_view(
                document,
            ))
            .unwrap();
            true
        }
        "listen" => {
            body.append_child(&crate::browser::specialist_persist::build_listen_view(
                document,
            ))
            .unwrap();
            true
        }
        "triad" => {
            body.append_child(&crate::browser::specialist_persist::build_triad_view(
                document,
            ))
            .unwrap();
            true
        }
        "portal" => {
            body.append_child(&crate::browser::specialist_persist::build_portal_view(
                document,
            ))
            .unwrap();
            true
        }
        "slide" => {
            body.append_child(&crate::browser::local_container_views::build_slide_view(
                document,
            ))
            .unwrap();
            true
        }
        "3d" => {
            body.append_child(&crate::browser::specialist_persist::build_3d_view(document))
                .unwrap();
            true
        }
        "subcanvas" | "nested_manifold" => {
            let target = if container.target_manifold.is_empty() {
                "research"
            } else {
                container.target_manifold.as_str()
            };
            body.append_child(
                &crate::browser::construct_shelf::build_nested_manifold_view(document, target),
            )
            .unwrap();
            true
        }
        "construct_shelf" => {
            body.append_child(
                &crate::browser::construct_shelf::build_construct_shelf_view(document),
            )
            .unwrap();
            true
        }
        "construct_portal" => {
            body.append_child(
                &crate::browser::construct_shelf::build_construct_portal_view(
                    document,
                    &container.target_construct,
                    &container.target_manifold,
                ),
            )
            .unwrap();
            true
        }
        "domain_lab" => {
            body.append_child(&crate::browser::logic_workbench::build_domain_lab_view(
                document,
            ))
            .unwrap();
            true
        }
        "subject" => {
            body.append_child(&crate::browser::construct_shelf::build_subject_view(
                document, container,
            ))
            .unwrap();
            true
        }
        "participants" => {
            body.append_child(&crate::browser::manifold_social::build_participants_view(
                document,
            ))
            .unwrap();
            true
        }
        "checkpoint-tray" => {
            body.append_child(
                &crate::browser::checkpoint_panel::build_checkpoint_tray_view(document),
            )
            .unwrap();
            true
        }
        "credential-inspector" => {
            body.append_child(&crate::browser::governance_workflow::build_credential_view(
                document,
            ))
            .unwrap();
            true
        }
        "context-markup-editor" => {
            body.append_child(
                &crate::browser::governance_workflow::build_context_markup_view(document),
            )
            .unwrap();
            true
        }
        "provenance-panel" => {
            body.append_child(&crate::browser::governance_workflow::build_provenance_view(
                document,
            ))
            .unwrap();
            true
        }
        "publication-workflow" => {
            body.append_child(
                &crate::browser::publication_panel::build_publication_workflow_view(document),
            )
            .unwrap();
            true
        }
        "constituency-manager" => {
            body.append_child(
                &crate::browser::governance_workflow::build_constituency_view(document),
            )
            .unwrap();
            true
        }
        "capability-badge" => {
            body.append_child(
                &crate::browser::governance_workflow::build_capability_badge_view(document),
            )
            .unwrap();
            true
        }
        "checkpoint-indicator" => {
            body.append_child(
                &crate::browser::checkpoint_panel::build_checkpoint_indicator_view(document),
            )
            .unwrap();
            true
        }
        "consent-indicator" => {
            body.append_child(&crate::browser::governance_workflow::build_consent_view(
                document,
            ))
            .unwrap();
            true
        }
        _ => false,
    }
}
