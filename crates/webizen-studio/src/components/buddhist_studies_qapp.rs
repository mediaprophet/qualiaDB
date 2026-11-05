use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `BuddhistStudiesQapp` component.
pub fn BuddhistStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:buddhiststudies".to_string(),
            title: "Buddhist Studies Explorer".to_string()
        }
    }
}
