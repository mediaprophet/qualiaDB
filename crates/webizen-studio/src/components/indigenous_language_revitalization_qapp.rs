use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn IndigenousLanguageRevitalizationQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:indigenouslanguagerevitalization".to_string(),
            title: "Indigenous Language Revitalization Explorer".to_string()
        }
    }
}
