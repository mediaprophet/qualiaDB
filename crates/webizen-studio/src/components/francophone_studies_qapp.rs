use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn FrancophonieStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:francophoniestudies".to_string(),
            title: "Francophonie Studies Explorer".to_string()
        }
    }
}
