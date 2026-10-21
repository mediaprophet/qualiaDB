use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `SportsStudiesQapp` component.
pub fn SportsStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:sportsstudies".to_string(),
            title: "Sports Studies Explorer".to_string()
        }
    }
}
