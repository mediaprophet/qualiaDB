use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn BioethicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:bioethics".to_string(),
            title: "Bioethics Explorer".to_string()
        }
    }
}
