use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `EnglishLanguageAndLiteratureQapp` component.
pub fn EnglishLanguageAndLiteratureQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:englishlanguageandliterature".to_string(),
            title: "English Language And Literature Explorer".to_string()
        }
    }
}
