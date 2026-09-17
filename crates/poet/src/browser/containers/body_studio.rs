//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Studio, dataset, and media-pipeline container bodies.
use crate::tool_chest::core::registry::SeedContainer;
use web_sys::{Document, Element};

pub(super) fn try_fill(document: &Document, container: &SeedContainer, body: &Element) -> bool {
    match container.container_type.as_str() {
        "audio_session" => {
            body.append_child(
                &crate::browser::studio_views::persist::build_audio_session_view(document),
            )
            .unwrap();
            true
        }
        "scene_view" => {
            body.append_child(&crate::browser::studio_views::scene_view::build_scene_view(
                document,
            ))
            .unwrap();
            true
        }
        "animation_timeline" => {
            body.append_child(
                &crate::browser::studio_views::animation_timeline::build_animation_timeline_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "desk_surface" => {
            body.append_child(
                &crate::browser::studio_views::desk_surface::build_desk_surface_view(document),
            )
            .unwrap();
            true
        }
        "transport" => {
            body.append_child(
                &crate::browser::studio_views::transport::build_transport_view(document),
            )
            .unwrap();
            true
        }
        "routing_matrix" => {
            body.append_child(
                &crate::browser::studio_views::routing_matrix::build_routing_matrix_view(document),
            )
            .unwrap();
            true
        }
        "spatial_audio" => {
            body.append_child(
                &crate::browser::studio_views::spatial_audio::build_spatial_audio_view(document),
            )
            .unwrap();
            true
        }
        "dataset_registry" => {
            body.append_child(
                &crate::browser::dataset_views::dataset_registry::build_dataset_registry_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "dataset_importer" => {
            body.append_child(
                &crate::browser::dataset_views::dataset_importer::build_dataset_importer_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "presentation_editor" => {
            body.append_child(
                &crate::browser::dataset_views::presentation_editor::build_presentation_editor_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "view_canvas" => {
            body.append_child(
                &crate::browser::dataset_views::view_canvas::build_view_canvas_view(document),
            )
            .unwrap();
            true
        }
        "scene_graph" => {
            body.append_child(
                &crate::browser::studio_views::scene_graph::build_scene_graph_view(document),
            )
            .unwrap();
            true
        }
        "material_editor" => {
            body.append_child(
                &crate::browser::studio_views::material_editor::build_material_editor_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "lighting_editor" => {
            body.append_child(
                &crate::browser::studio_views::lighting_editor::build_lighting_editor_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "tensor_inspector" => {
            body.append_child(
                &crate::browser::studio_views::tensor_inspector::build_tensor_inspector_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "asset_library" => {
            body.append_child(
                &crate::browser::studio_views::asset_library::build_asset_library_view(document),
            )
            .unwrap();
            true
        }
        "channel_strip" => {
            body.append_child(
                &crate::browser::studio_views::channel_strip::build_channel_strip_view(document),
            )
            .unwrap();
            true
        }
        "meter_bridge" => {
            body.append_child(
                &crate::browser::studio_views::meter_bridge::build_meter_bridge_view(document),
            )
            .unwrap();
            true
        }
        "automation_lanes" => {
            body.append_child(
                &crate::browser::studio_views::automation_lanes::build_automation_lanes_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "annotation_panel" => {
            body.append_child(
                &crate::browser::dataset_views::annotation_panel::build_annotation_panel_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "lineage_graph" => {
            body.append_child(
                &crate::browser::dataset_views::lineage_graph::build_lineage_graph_view(document),
            )
            .unwrap();
            true
        }
        "lod_chain" => {
            body.append_child(
                &crate::browser::studio_views::lod_chain::build_lod_chain_view(document),
            )
            .unwrap();
            true
        }
        "shadow_settings" => {
            body.append_child(
                &crate::browser::studio_views::shadow_settings::build_shadow_settings_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "gis_maps" => {
            body.append_child(
                &crate::browser::studio_views::gis_maps::build_gis_maps_view(document),
            )
            .unwrap();
            true
        }
        "ragdoll_skin" => {
            body.append_child(
                &crate::browser::studio_views::ragdoll_skin::build_ragdoll_skin_view(document),
            )
            .unwrap();
            true
        }
        "animation_export" => {
            body.append_child(
                &crate::browser::studio_views::animation_export::build_animation_export_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "desk_persistence" => {
            body.append_child(
                &crate::browser::studio_views::desk_persistence::build_desk_persistence_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "hrtf_personalization" => {
            body.append_child(
                &crate::browser::studio_views::hrtf_personalization::build_hrtf_personalization_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "manifold_transition_audio" => {
            body.append_child(
                &crate::browser::studio_views::manifold_transition_audio::build_manifold_transition_audio_view(document),
            )
            .unwrap();
            true
        }
        "video_view" => {
            body.append_child(
                &crate::browser::dataset_views::video_view::build_video_view_view(document),
            )
            .unwrap();
            true
        }
        "presentation_publish" => {
            body.append_child(
                &crate::browser::dataset_views::presentation_publish::build_presentation_publish_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "super_resolve" => {
            body.append_child(
                &crate::browser::dataset_views::super_resolve::build_super_resolve_view(document),
            )
            .unwrap();
            true
        }
        "cad_curation" => {
            body.append_child(
                &crate::browser::dataset_views::cad_curation::build_cad_curation_view(document),
            )
            .unwrap();
            true
        }
        _ => false,
    }
}
