use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn MedicalHumanitiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:medicalhumanities".to_string(),
            title: "Medical Humanities Explorer".to_string()
        }
    }
}
