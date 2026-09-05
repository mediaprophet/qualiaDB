//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_health_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "health:place_health_overview".into(),
                label: "+ Health overview".into(),
                icon: "health".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "health".into(),
                description: "Place the Health overview (live COP counts).".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "health:place_health_documents".into(),
                label: "+ Health documents".into(),
                icon: "health".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "health".into(),
                description: "Place NLP + Semantic Library document ingest (classified/secret)."
                    .into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "health:place_disclosure_log".into(),
                label: "+ Share / disclosure".into(),
                icon: "health".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "health".into(),
                description: "Place private/permissive share to a clinician DID.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "health:place_conditions".into(),
                label: "+ Conditions".into(),
                icon: "health".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "health".into(),
                description: "Place condition records (possessions of the Principal).".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "health:place_health".into(),
                label: "+ Health Vault".into(),
                icon: "health".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "health".into(),
                description: "Place a health vault container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "health:place_health_calculators".into(),
                label: "+ Clinical calculators".into(),
                icon: "health".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "health".into(),
                description: "Place Framingham, CHA₂DS₂-VASc, and SCORE2 forms. Empty until you enter values."
                    .into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "health:pathology".into(),
                label: "🔬 Pathology Assay".into(),
                icon: "pathology".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "health".into(),
                description: "Run pathology and diagnostic assay.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "health:anatomy_10d".into(),
                label: "+ 10D Anatomy".into(),
                icon: "anatomy".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "health".into(),
                description: "Place a 10D anatomy container.".into(),
            },
            ActionType::Query,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "health".into(),
            label: "Scientific & Clinical Lab".into(),
            icon: "health".into(),
            ontology_prefix: "health".into(),
            description: "Health overview, NLP document ingest, clinician share, conditions. Clinical risk engines stay on entered vitals.".into(),
            enabled_by_default: false,
            family: "lab".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "health:clinical".into(),
                    label: "Clinical Engines & Biomarkers".into(),
                    icon: "health".into(),
                    description: "Framingham, CHA₂DS₂-VASc, and SCORE2. Required inputs only; incomplete cannot calculate."
                        .into(),
                },
                vec![
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "health:framingham".into(),
                            label: "Framingham".into(),
                            icon: "health".into(),
                            kind: ToolKind::RunAction,
                            capability_scope: Some("ClinicalRisk.framingham".into()),
                            ontology_prefix: "health".into(),
                            description: "Open the Framingham form. ClinicalRisk.framingham runs only after every required input is entered.".into(),
                        },
                        ActionType::Invoke,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "health:cha2ds2".into(),
                            label: "CHA₂DS₂-VASc".into(),
                            icon: "health".into(),
                            kind: ToolKind::RunAction,
                            capability_scope: Some("ClinicalRisk.cha2ds2_vasc".into()),
                            ontology_prefix: "health".into(),
                            description: "Open the CHA₂DS₂-VASc form. ClinicalRisk.cha2ds2_vasc applies only with atrial fibrillation.".into(),
                        },
                        ActionType::Invoke,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "health:score2".into(),
                            label: "SCORE2".into(),
                            icon: "health".into(),
                            kind: ToolKind::RunAction,
                            capability_scope: Some("ClinicalRisk.score2".into()),
                            ontology_prefix: "health".into(),
                            description: "Open the SCORE2 form. ClinicalRisk.score2 requires a named European risk region.".into(),
                        },
                        ActionType::Invoke,
                    )),
                ],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "health:tools".into(),
                    label: "Lab Viewports & Assays".into(),
                    icon: "tools".into(),
                    description: "Place health vaults and run pathology assays.".into(),
                },
                tools,
            ),
        ],
    ));
}
