use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SlavicStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:slavicstudies".to_string(),
            title: "Slavic Studies Explorer".to_string()
        }
    }
}
