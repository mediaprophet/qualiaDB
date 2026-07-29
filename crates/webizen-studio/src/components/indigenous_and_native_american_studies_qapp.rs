use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn IndigenousAndNativeAmericanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:indigenousandnativeamericanstudies".to_string(),
            title: "Indigenous And Native American Studies Explorer".to_string()
        }
    }
}
