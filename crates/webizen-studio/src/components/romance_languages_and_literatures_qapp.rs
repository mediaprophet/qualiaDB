use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `RomanceLanguagesAndLiteraturesQapp` component.
pub fn RomanceLanguagesAndLiteraturesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:romancelanguagesandliteratures".to_string(),
            title: "Romance Languages And Literatures Explorer".to_string()
        }
    }
}
