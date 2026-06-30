use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn ForeignLanguagesAndLiteraturesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:foreignlanguagesandliteratures".to_string(),
            title: "Foreign Languages And Literatures Explorer".to_string()
        }
    }
}
