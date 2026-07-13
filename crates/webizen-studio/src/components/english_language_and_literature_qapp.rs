use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn EnglishLanguageAndLiteratureQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:englishlanguageandliterature".to_string(),
            title: "English Language And Literature Explorer".to_string()
        }
    }
}
