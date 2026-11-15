use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `HistoryOfScienceAndMedicineQapp` component.
pub fn HistoryOfScienceAndMedicineQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:historyofscienceandmedicine".to_string(),
            title: "History Of Science And Medicine Explorer".to_string()
        }
    }
}
