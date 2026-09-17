use web_sys::{Document, Element};

use super::super::super::cop_records::{build_family_panel, CopField};
use super::wrap;

pub fn build_conditions_view(document: &Document) -> Element {
    wrap(
        document,
        build_family_panel(
            document,
            "health_condition",
            "Conditions the Principal HAS (q42:hasCondition). Not the identity of the Principal.",
            &[
                CopField {
                    key: "code",
                    placeholder: "Code (SNOMED/ICD if known)",
                },
                CopField {
                    key: "status",
                    placeholder: "Status (active|resolved)",
                },
                CopField {
                    key: "sensitivity",
                    placeholder: "Sensitivity (classified|restricted)",
                },
            ],
        ),
    )
}

pub fn build_medications_view(document: &Document) -> Element {
    wrap(
        document,
        build_family_panel(
            document,
            "health_medication",
            "Medications persist as records. No sample prescriptions.",
            &[
                CopField {
                    key: "dose",
                    placeholder: "Dose",
                },
                CopField {
                    key: "status",
                    placeholder: "Status (active|stopped)",
                },
                CopField {
                    key: "sensitivity",
                    placeholder: "Sensitivity",
                },
            ],
        ),
    )
}

pub fn build_lab_results_view(document: &Document) -> Element {
    wrap(
        document,
        build_family_panel(
            document,
            "health_lab",
            "Lab results you enter. NLP extract from documents lives in Health Documents. Values are not invented.",
            &[
                CopField {
                    key: "analyte",
                    placeholder: "Analyte",
                },
                CopField {
                    key: "value",
                    placeholder: "Value (as reported)",
                },
                CopField {
                    key: "unit",
                    placeholder: "Unit",
                },
                CopField {
                    key: "sensitivity",
                    placeholder: "Sensitivity",
                },
            ],
        ),
    )
}

pub fn build_vitals_view(document: &Document) -> Element {
    wrap(
        document,
        build_family_panel(
            document,
            "health_vital",
            "Vitals you measure or transcribe. ClinicalRisk uses these fields only.",
            &[
                CopField {
                    key: "sys_bp",
                    placeholder: "Systolic BP",
                },
                CopField {
                    key: "dia_bp",
                    placeholder: "Diastolic BP",
                },
                CopField {
                    key: "hr",
                    placeholder: "Heart rate",
                },
                CopField {
                    key: "sensitivity",
                    placeholder: "Sensitivity",
                },
            ],
        ),
    )
}

pub fn build_safeguards_view(document: &Document) -> Element {
    wrap(
        document,
        build_family_panel(
            document,
            "health_safeguard",
            "Safeguard / consent gates. Fail closed until a record exists.",
            &[
                CopField {
                    key: "gate",
                    placeholder: "Gate (consent|sanctuary|disclosure)",
                },
                CopField {
                    key: "status",
                    placeholder: "Status (in_force|revoked)",
                },
                CopField {
                    key: "sensitivity",
                    placeholder: "Sensitivity",
                },
            ],
        ),
    )
}

pub fn build_clinical_reports_view(document: &Document) -> Element {
    wrap(
        document,
        build_family_panel(
            document,
            "health_report",
            "Clinical report metadata. Body text goes through Health Documents (NLP + library).",
            &[
                CopField {
                    key: "author",
                    placeholder: "Author DID",
                },
                CopField {
                    key: "date",
                    placeholder: "Date",
                },
                CopField {
                    key: "sensitivity",
                    placeholder: "Sensitivity",
                },
            ],
        ),
    )
}
