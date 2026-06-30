use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn QueerCinemaStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:queercinemastudies".to_string(),
            title: "Queer Cinema Studies Explorer".to_string()
        }
    }
}
