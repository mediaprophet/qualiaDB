use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn FrancophonieStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:francophoniestudies".to_string(),
            title: "Francophonie Studies Explorer".to_string()
        }
    }
}
