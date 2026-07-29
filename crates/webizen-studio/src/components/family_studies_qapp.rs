use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn FamilyStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:familystudies".to_string(),
            title: "Family Studies Explorer".to_string()
        }
    }
}
