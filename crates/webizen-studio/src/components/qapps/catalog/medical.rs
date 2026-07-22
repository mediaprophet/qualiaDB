use super::{Cat, QApp, Stat};

pub(super) fn apps() -> Vec<QApp> {
    vec![
        // â”€â”€ Medical & Life Sciences â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        QApp {
            id: "health-vitals",
            name: "Health Vital Monitor",
            tagline: "Biosignals",
            desc: "Real-time biosignal monitoring via the biosciences SHACL engine. IoT sensor \
                   ingestion, standardised ontology mapping (HL7 FHIR, SNOMED CT), and anomaly alerting.",
            icon: "heart-pulse",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Medical,
        },
        QApp {
            id: "clinical-risk",
            name: "Clinical Risk Scorer",
            tagline: "Decision Support",
            desc: "Framingham, APACHE-II, SOFA, and custom prognosis models via clinical_engine. \
                   Gene expression evaluation, guideline cross-referencing, and a signed audit trail.",
            icon: "clipboard2-pulse",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Medical,
        },
        QApp {
            id: "dicom-viewer",
            name: "DICOM Viewer",
            tagline: "Medical Imaging",
            desc: "DICOM file ingestion and rendering via dicom_ingest. Slice navigation, \
                   window/level adjustment, annotation overlay, and export to NQuin-tagged graph nodes.",
            icon: "image-alt",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Medical,
        },
        QApp {
            id: "anatomy-browser",
            name: "Anatomy Context Browser",
            tagline: "Reference Atlas",
            desc: "Interactive anatomical reference powered by anatomy_context. Link structures to \
                   clinical risk scores, DICOM regions of interest, and bioinformatics datasets.",
            icon: "person-bounding-box",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Medical,
        },
        QApp {
            id: "comorbidity",
            name: "Comorbidity Analyzer",
            tagline: "Multi-condition Risk",
            desc: "Multi-condition risk assessment via comorbidity_eval. Surfaces drug-interaction \
                   risks, contraindication flags, and population-level co-occurrence patterns.",
            icon: "shield-plus",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Medical,
        },
    ]
}
