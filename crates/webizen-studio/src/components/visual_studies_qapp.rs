use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `VisualStudiesQapp` component.
pub fn VisualStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:visualstudies".to_string(),
            title: "Visual Studies Explorer".to_string()
        }
    }
}
