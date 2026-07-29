use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn GermanicLanguagesAndLiteraturesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:germaniclanguagesandliteratures".to_string(),
            title: "Germanic Languages And Literatures Explorer".to_string()
        }
    }
}
