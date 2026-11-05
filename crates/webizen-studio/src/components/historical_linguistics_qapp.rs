use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `HistoricalLinguisticsQapp` component.
pub fn HistoricalLinguisticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:historicallinguistics".to_string(),
            title: "Historical Linguistics Explorer".to_string()
        }
    }
}
