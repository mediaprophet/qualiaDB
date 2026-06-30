use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn HistoryOfScienceAndMedicineQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:historyofscienceandmedicine".to_string(),
            title: "History Of Science And Medicine Explorer".to_string()
        }
    }
}
