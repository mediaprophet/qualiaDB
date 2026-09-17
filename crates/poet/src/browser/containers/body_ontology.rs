//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Ontology workbench and device container bodies.
use crate::tool_chest::core::registry::SeedContainer;
use web_sys::{Document, Element};

pub(super) fn try_fill(document: &Document, container: &SeedContainer, body: &Element) -> bool {
    match container.container_type.as_str() {
        "ontology_graph_canvas" => {
            body.append_child(
                &crate::browser::ontology_views::graph_canvas::build_ontology_graph_canvas_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "ontology_library" => {
            body.append_child(
                &crate::browser::ontology_views::ontology_library::build_ontology_library_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "vocabulary_mapper" => {
            body.append_child(
                &crate::browser::ontology_views::vocabulary_mapper::build_vocabulary_mapper_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "relation_builder" => {
            body.append_child(
                &crate::browser::ontology_views::relation_builder::build_relation_builder_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "shacl_shapes" => {
            body.append_child(
                &crate::browser::ontology_views::shacl_shapes::build_shacl_shapes_view(document),
            )
            .unwrap();
            true
        }
        "n3_editor" => {
            body.append_child(
                &crate::browser::ontology_views::n3_editor::build_n3_editor_view(document),
            )
            .unwrap();
            true
        }
        "shex_editor" => {
            body.append_child(
                &crate::browser::ontology_views::shex_editor::build_shex_editor_view(document),
            )
            .unwrap();
            true
        }
        "ontology_compare" => {
            body.append_child(
                &crate::browser::ontology_views::ontology_compare::build_ontology_compare_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "project_ontology_selector" => {
            body.append_child(
                &crate::browser::ontology_views::project_ontology_selector::build_project_ontology_selector_view(document),
            )
            .unwrap();
            true
        }
        "device_manager" => {
            body.append_child(
                &crate::browser::device_views::device_manager::build_device_manager_view(document),
            )
            .unwrap();
            true
        }
        "display_layout" => {
            body.append_child(
                &crate::browser::device_views::display_layout::build_display_layout_view(document),
            )
            .unwrap();
            true
        }
        "workspace_sync" => {
            body.append_child(
                &crate::browser::device_views::workspace_sync::build_workspace_sync_view(document),
            )
            .unwrap();
            true
        }
        "device_role_assigner" => {
            body.append_child(
                &crate::browser::device_views::device_role_assigner::build_device_role_assigner_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "remote_control" => {
            body.append_child(
                &crate::browser::device_views::remote_control::build_remote_control_view(document),
            )
            .unwrap();
            true
        }
        _ => false,
    }
}
