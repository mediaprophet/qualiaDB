use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn AncientNearEasternStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ancientneareasternstudies".to_string(),
            title: "Ancient Near Eastern Studies Explorer".to_string()
        }
    }
}
