use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn AncientNearEasternStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ancientneareasternstudies".to_string(),
            title: "Ancient Near Eastern Studies Explorer".to_string()
        }
    }
}
