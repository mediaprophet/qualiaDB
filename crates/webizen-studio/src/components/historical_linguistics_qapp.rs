use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn HistoricalLinguisticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:historicallinguistics".to_string(),
            title: "Historical Linguistics Explorer".to_string()
        }
    }
}
