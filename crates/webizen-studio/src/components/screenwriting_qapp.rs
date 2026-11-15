use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `ScreenwritingQapp` component.
pub fn ScreenwritingQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:screenwriting".to_string(),
            title: "Screenwriting Explorer".to_string()
        }
    }
}
