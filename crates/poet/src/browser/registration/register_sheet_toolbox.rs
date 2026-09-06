//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_sheet_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "sheet:place_sheet".into(),
                label: "+ Spreadsheet".into(),
                icon: "sheet".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "hm".into(),
                description: "Place a spreadsheet container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "sheet:import".into(),
                label: "Import Data".into(),
                icon: "import".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "hm".into(),
                description: "Import CSV/HCF data into active sheet.".into(),
            },
            ActionType::Mutate,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "sheet".into(),
            label: "Spreadsheet & Tensors".into(),
            icon: "sheet".into(),
            ontology_prefix: "hm".into(),
            description: "Spreadsheets, tensor arrays, formulas, and data import.".into(),
            enabled_by_default: true,
            family: "sheet".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "sheet:grid".into(),
                    label: "Tensor Dimensions & Formats".into(),
                    icon: "sheet".into(),
                    description: "Configure 1D/2D/3D/10D tensor dimensions and cell formatting."
                        .into(),
                },
                vec![Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "sheet:stats_mean".into(),
                        label: "Mean".into(),
                        icon: "sheet".into(),
                        kind: ToolKind::Query,
                        capability_scope: Some("Statistics.mean".into()),
                        ontology_prefix: "hm".into(),
                        description: "Mean of numbers on the selected sheet. Local compute; daemon upgrades to Statistics.mean.".into(),
                    },
                    ActionType::Query,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "sheet:stats_median".into(),
                        label: "Median".into(),
                        icon: "sheet".into(),
                        kind: ToolKind::Query,
                        capability_scope: Some("Statistics.median".into()),
                        ontology_prefix: "hm".into(),
                        description: "Median of numbers on the selected sheet. Local compute; daemon upgrades to Statistics.median.".into(),
                    },
                    ActionType::Query,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "sheet:stats_variance".into(),
                        label: "Variance".into(),
                        icon: "sheet".into(),
                        kind: ToolKind::Query,
                        capability_scope: Some("Statistics.variance".into()),
                        ontology_prefix: "hm".into(),
                        description: "Sample variance of numbers on the selected sheet. Local compute; daemon upgrades to Statistics.variance.".into(),
                    },
                    ActionType::Query,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "sheet:stats_std_dev".into(),
                        label: "Std Dev".into(),
                        icon: "sheet".into(),
                        kind: ToolKind::Query,
                        capability_scope: Some("Statistics.std_dev".into()),
                        ontology_prefix: "hm".into(),
                        description: "Sample standard deviation on the selected sheet. Local compute; daemon upgrades to Statistics.std_dev.".into(),
                    },
                    ActionType::Query,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "sheet:stats_min".into(),
                        label: "Min".into(),
                        icon: "sheet".into(),
                        kind: ToolKind::Query,
                        capability_scope: Some("Statistics.min".into()),
                        ontology_prefix: "hm".into(),
                        description: "Minimum of numbers on the selected sheet. Local compute; daemon upgrades to Statistics.min.".into(),
                    },
                    ActionType::Query,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "sheet:stats_max".into(),
                        label: "Max".into(),
                        icon: "sheet".into(),
                        kind: ToolKind::Query,
                        capability_scope: Some("Statistics.max".into()),
                        ontology_prefix: "hm".into(),
                        description: "Maximum of numbers on the selected sheet. Local compute; daemon upgrades to Statistics.max.".into(),
                    },
                    ActionType::Query,
                ))],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "sheet:tools".into(),
                    label: "Spreadsheet Tools".into(),
                    icon: "tools".into(),
                    description: "Place spreadsheets and import external tabular data.".into(),
                },
                tools,
            ),
        ],
    ));
}
