use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `BotanyQapp` component.
pub fn BotanyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:botany".to_string(),
            title: "Botany Explorer".to_string()
        }
    }
}
