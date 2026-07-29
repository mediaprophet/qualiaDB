use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn JewishStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:jewishstudies".to_string(),
            title: "Jewish Studies Explorer".to_string()
        }
    }
}
