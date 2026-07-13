use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn LibraryAndInformationScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:libraryandinformationscience".to_string(),
            title: "Library And Information Science Explorer".to_string()
        }
    }
}
