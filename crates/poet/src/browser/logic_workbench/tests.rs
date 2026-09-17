//! Tests for the logic workbench module.

use super::*;

#[test]
fn test_p0_tools_not_empty() {
    assert!(!P0_TOOLS.is_empty());
    assert!(P0_TOOLS.len() >= 7);
}

#[test]
fn test_all_modalities_not_empty() {
    assert!(!ALL_MODALITIES.is_empty());
    assert!(ALL_MODALITIES.len() >= 30);
}

#[test]
fn test_modality_descriptions_cover_all() {
    for (key, _) in ALL_MODALITIES {
        let desc = descriptions::get(key);
        assert!(
            !desc.starts_with("Select a modality"),
            "missing description for: {}",
            key
        );
    }
}

#[test]
fn test_deontic_operators_present() {
    assert!(DEONTIC_OPERATORS.iter().any(|(k, _)| *k == "OBLIGATE"));
    assert!(DEONTIC_OPERATORS.iter().any(|(k, _)| *k == "FORBID"));
}

#[test]
fn test_shacl_constraints_present() {
    assert!(SHACL_CONSTRAINTS
        .iter()
        .any(|(k, _)| k.contains("minCount")));
    assert!(SHACL_CONSTRAINTS
        .iter()
        .any(|(k, _)| k.contains("datatype")));
}

#[test]
fn test_hohfeld_positions_complete() {
    let positions = [
        "Right",
        "Duty",
        "Privilege",
        "No-Right",
        "Power",
        "Liability",
        "Immunity",
        "Disability",
    ];
    for p in &positions {
        assert!(format!("{:?}", p).len() > 0);
    }
    assert_eq!(positions.len(), 8);
}

#[test]
fn test_p1_legal_tools_not_empty() {
    assert!(!P1_LEGAL_TOOLS.is_empty());
    assert!(P1_LEGAL_TOOLS.len() >= 8);
}

#[test]
fn test_p1_gov_tools_not_empty() {
    assert!(!P1_GOV_TOOLS.is_empty());
    assert!(P1_GOV_TOOLS.len() >= 6);
}

#[test]
fn test_p1_logic_tools_not_empty() {
    assert!(!P1_LOGIC_TOOLS.is_empty());
    assert!(P1_LOGIC_TOOLS.len() >= 9);
}

#[test]
fn test_p1_advanced_tools_not_empty() {
    assert!(!P1_ADVANCED_TOOLS.is_empty());
    assert!(P1_ADVANCED_TOOLS.len() >= 8);
}

#[test]
fn test_p2_domain_tools_not_empty() {
    assert!(!P2_DOMAIN_TOOLS.is_empty());
    assert!(P2_DOMAIN_TOOLS.len() >= 9);
}

#[test]
fn test_p2_infra_tools_not_empty() {
    assert!(!P2_INFRA_TOOLS.is_empty());
    assert!(P2_INFRA_TOOLS.len() >= 9);
}

#[test]
fn test_p2_infra_ext_tools_not_empty() {
    assert!(!P2_INFRA_EXT_TOOLS.is_empty());
    assert!(P2_INFRA_EXT_TOOLS.len() >= 10);
}

#[test]
fn test_p2_extras_tools_not_empty() {
    assert!(!P2_EXTRAS_TOOLS.is_empty());
    assert!(P2_EXTRAS_TOOLS.len() >= 3);
}

#[test]
fn test_all_tool_ids_unique() {
    let all_tools: Vec<&str> = P0_TOOLS
        .iter()
        .chain(P1_LEGAL_TOOLS.iter())
        .chain(P1_GOV_TOOLS.iter())
        .chain(P1_LOGIC_TOOLS.iter())
        .chain(P1_ADVANCED_TOOLS.iter())
        .chain(P2_DOMAIN_TOOLS.iter())
        .chain(P2_INFRA_TOOLS.iter())
        .chain(P2_INFRA_EXT_TOOLS.iter())
        .chain(P2_EXTRAS_TOOLS.iter())
        .map(|(id, _, _)| *id)
        .collect();
    let mut seen = std::collections::HashSet::new();
    for id in &all_tools {
        assert!(seen.insert(*id), "duplicate tool id: {}", id);
    }
}

#[test]
fn test_all_tool_ids_have_modality_descriptions() {
    let all_tool_ids: Vec<&str> = P1_LEGAL_TOOLS
        .iter()
        .chain(P1_GOV_TOOLS.iter())
        .chain(P1_LOGIC_TOOLS.iter())
        .chain(P1_ADVANCED_TOOLS.iter())
        .chain(P2_DOMAIN_TOOLS.iter())
        .chain(P2_INFRA_TOOLS.iter())
        .chain(P2_INFRA_EXT_TOOLS.iter())
        .chain(P2_EXTRAS_TOOLS.iter())
        .map(|(id, _, _)| *id)
        .collect();
    for id in &all_tool_ids {
        let desc = descriptions::get(id);
        assert!(
            !desc.starts_with("Select a modality"),
            "missing description for tool: {}",
            id
        );
    }
}
