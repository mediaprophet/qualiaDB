use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn SlavicStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:slavicstudies".to_string(),
            title: "Slavic Studies Explorer".to_string()
        }
    }
}
