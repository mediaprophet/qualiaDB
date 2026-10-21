use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `LiberationStudiesQapp` component.
pub fn LiberationStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:liberationstudies".to_string(),
            title: "Liberation Studies Explorer".to_string()
        }
    }
}
