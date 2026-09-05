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
                    description: "Select CVD risk models and adjust blood pressure biomarkers."
                        .into(),
                },
                vec![],
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
