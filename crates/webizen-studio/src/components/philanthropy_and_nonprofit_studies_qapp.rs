use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PhilanthropyAndNonprofitStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:philanthropyandnonprofitstudies".to_string(),
            title: "Philanthropy And Nonprofit Studies Explorer".to_string()
        }
    }
}
