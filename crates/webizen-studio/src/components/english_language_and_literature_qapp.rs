use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn EnglishLanguageAndLiteratureQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:englishlanguageandliterature".to_string(),
            title: "English Language And Literature Explorer".to_string()
        }
    }
}
