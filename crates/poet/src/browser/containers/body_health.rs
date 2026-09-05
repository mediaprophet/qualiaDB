//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Health vault and clinical workspace container bodies.
use crate::tool_chest::core::registry::SeedContainer;
use web_sys::{Document, Element};

pub(super) fn try_fill(document: &Document, container: &SeedContainer, body: &Element) -> bool {
    match container.container_type.as_str() {
        "health_overview" => {
            body.append_child(
                &crate::browser::health_views::health_overview::build_health_overview_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "health_calculators" => {
            body.append_child(
                &crate::browser::health_views::calculators::build_health_calculators_view(document),
            )
            .unwrap();
            true
        }
        "conditions" => {
            body.append_child(
                &crate::browser::health_views::conditions::build_conditions_view(document),
            )
            .unwrap();
            true
        }
        "clinical_reports" => {
            body.append_child(
                &crate::browser::health_views::clinical_reports::build_clinical_reports_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "lab_results" => {
            body.append_child(
                &crate::browser::health_views::lab_results::build_lab_results_view(document),
            )
            .unwrap();
            true
        }
        "medications" => {
            body.append_child(
                &crate::browser::health_views::medications::build_medications_view(document),
            )
            .unwrap();
            true
        }
        "vitals" => {
            body.append_child(&crate::browser::health_views::vitals::build_vitals_view(
                document,
            ))
            .unwrap();
            true
        }
        "mental_wellbeing" => {
            body.append_child(
                &crate::browser::health_views::mental_wellbeing::build_mental_wellbeing_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "therapy_notes" => {
            body.append_child(
                &crate::browser::health_views::therapy_notes::build_therapy_notes_view(document),
            )
            .unwrap();
            true
        }
        "sleep" => {
            body.append_child(&crate::browser::health_views::sleep::build_sleep_view(
                document,
            ))
            .unwrap();
            true
        }
        "diet" => {
            body.append_child(&crate::browser::health_views::diet::build_diet_view(
                document,
            ))
            .unwrap();
            true
        }
        "physical_activity" => {
            body.append_child(
                &crate::browser::health_views::physical_activity::build_physical_activity_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "immunizations" => {
            body.append_child(
                &crate::browser::health_views::immunizations::build_immunizations_view(document),
            )
            .unwrap();
            true
        }
        "procedures" => {
            body.append_child(
                &crate::browser::health_views::procedures::build_procedures_view(document),
            )
            .unwrap();
            true
        }
        "family_history" => {
            body.append_child(
                &crate::browser::health_views::family_history::build_family_history_view(document),
            )
            .unwrap();
            true
        }
        "hypotheses" => {
            body.append_child(
                &crate::browser::health_views::hypotheses::build_hypotheses_view(document),
            )
            .unwrap();
            true
        }
        "biometrics" => {
            body.append_child(
                &crate::browser::health_views::biometrics::build_biometrics_view(document),
            )
            .unwrap();
            true
        }
        "health_documents" => {
            body.append_child(
                &crate::browser::health_views::documents::build_documents_view(document),
            )
            .unwrap();
            true
        }
        "welfare_support" => {
            body.append_child(
                &crate::browser::health_views::welfare_support::build_welfare_support_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "life_records" => {
            body.append_child(
                &crate::browser::health_views::life_records::build_life_records_view(document),
            )
            .unwrap();
            true
        }
        "authority_attestations" => {
            body.append_child(
                &crate::browser::health_views::authority_attestations::build_authority_attestations_view(
                    document,
                ),
            )
            .unwrap();
            true
        }
        "safeguards" => {
            body.append_child(
                &crate::browser::health_views::safeguards::build_safeguards_view(document),
            )
            .unwrap();
            true
        }
        "disclosure_log" => {
            body.append_child(
                &crate::browser::health_views::disclosure_log::build_disclosure_log_view(document),
            )
            .unwrap();
            true
        }
        _ => false,
    }
}
