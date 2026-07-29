use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn RomanceLanguagesAndLiteraturesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:romancelanguagesandliteratures".to_string(),
            title: "Romance Languages And Literatures Explorer".to_string()
        }
    }
}
