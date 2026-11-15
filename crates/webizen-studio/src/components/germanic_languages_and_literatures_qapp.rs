use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `GermanicLanguagesAndLiteraturesQapp` component.
pub fn GermanicLanguagesAndLiteraturesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:germaniclanguagesandliteratures".to_string(),
            title: "Germanic Languages And Literatures Explorer".to_string()
        }
    }
}
