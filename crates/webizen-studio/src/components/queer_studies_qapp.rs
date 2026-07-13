use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn QueerStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:queerstudies".to_string(),
            title: "Queer Studies Explorer".to_string()
        }
    }
}
