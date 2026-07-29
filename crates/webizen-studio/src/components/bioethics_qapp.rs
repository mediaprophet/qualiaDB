use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn BioethicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:bioethics".to_string(),
            title: "Bioethics Explorer".to_string()
        }
    }
}
