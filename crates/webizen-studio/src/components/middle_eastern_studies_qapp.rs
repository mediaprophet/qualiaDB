use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MiddleEasternStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:middleeasternstudies".to_string(),
            title: "Middle Eastern Studies Explorer".to_string()
        }
    }
}
