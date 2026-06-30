use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn EducationStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:educationstudies".to_string(),
            title: "Education Studies Explorer".to_string()
        }
    }
}
