use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SpinozaStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:spinozastudies".to_string(),
            title: "Spinoza Studies Explorer".to_string()
        }
    }
}
