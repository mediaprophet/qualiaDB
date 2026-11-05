use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `MusicHistoryQapp` component.
pub fn MusicHistoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:musichistory".to_string(),
            title: "Music History Explorer".to_string()
        }
    }
}
