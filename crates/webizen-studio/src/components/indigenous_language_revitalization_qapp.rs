use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `IndigenousLanguageRevitalizationQapp` component.
pub fn IndigenousLanguageRevitalizationQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:indigenouslanguagerevitalization".to_string(),
            title: "Indigenous Language Revitalization Explorer".to_string()
        }
    }
}
