use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn FamilyStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:familystudies".to_string(),
            title: "Family Studies Explorer".to_string()
        }
    }
}
