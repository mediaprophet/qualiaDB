use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn SpinozaStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:spinozastudies".to_string(),
            title: "Spinoza Studies Explorer".to_string()
        }
    }
}
