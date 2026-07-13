use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn RomanceLanguagesAndLiteraturesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:romancelanguagesandliteratures".to_string(),
            title: "Romance Languages And Literatures Explorer".to_string()
        }
    }
}
