#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

fn parse_sensitivity(s: &str) -> wellfare_core::record::SensitivityClass {
    match s.to_ascii_lowercase().as_str() {
        "classified" => wellfare_core::record::SensitivityClass::Classified,
        "public" => wellfare_core::record::SensitivityClass::Public,
        _ => wellfare_core::record::SensitivityClass::Restricted,
    }
}

fn parse_epistemic(s: &str) -> wellfare_core::record::EpistemicStatus {
    match s.to_ascii_lowercase().as_str() {
        "hypothesis" => wellfare_core::record::EpistemicStatus::Hypothesis,
        "disputed" => wellfare_core::record::EpistemicStatus::Disputed,
        "refuted" => wellfare_core::record::EpistemicStatus::Refuted,
        _ => wellfare_core::record::EpistemicStatus::Asserted,
    }
}

#[command]
pub fn wellfair_evaluate_policy(
    app: AppHandle,
    qapp_id: String,
    scope: String,
    sensitivity: String,
    epistemic: String,
) -> Result<String, String> {
    let sens = parse_sensitivity(&sensitivity);
    let ep = parse_epistemic(&epistemic);
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let decision = if let Some(host) = guard.as_ref() {
            host.evaluate_policy(&qapp_id, &scope, sens, ep)?
        } else {
            let svc = qualia_client_core::wellfair::policy::PolicyDecisionService::new();
            svc.evaluate_access(&qapp_id, &scope, sens, ep, &[], 0, false).to_dto()
        };
        serde_json::to_string(&decision).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_grant_consent(
    app: AppHandle,
    draft_json: String,
    scope: String,
) -> Result<String, String> {
    let draft: qualia_client_core::wellfair::host_state::ConsentGrantDraft =
        serde_json::from_str(&draft_json).map_err(|e| format!("invalid consent draft: {e}"))?;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let grant = host.grant_consent(&draft, &scope)?;
        serde_json::to_string(&grant).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_revoke_consent(app: AppHandle, grant_id: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let revoked = host.revoke_consent(&grant_id)?;
        serde_json::to_string(&serde_json::json!({ "revoked": revoked })).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_list_consents(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let grants = host.list_consents()?;
        serde_json::to_string(&grants).map_err(|e| e.to_string())
    })?
}

#[derive(Debug, serde::Deserialize)]
struct ConditionReportInput {
    label: String,
    icd10_code: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct AllergyReportInput {
    substance: String,
    reaction: Option<String>,
    severity: Option<String>,
    notes: Option<String>,
}

#[command]
pub fn wellfair_add_condition(app: AppHandle, report_json: String) -> Result<String, String> {
    let input: ConditionReportInput =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid condition JSON: {e}"))?;
    let mut report = wellfare_core::conditions::ConditionReport::new(input.label);
    report.icd10_code = input
        .icd10_code
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());
    report.notes = input
        .notes
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry = host.add_condition(&report)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_add_allergy(app: AppHandle, report_json: String) -> Result<String, String> {
    let input: AllergyReportInput =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid allergy JSON: {e}"))?;
    let mut report = wellfare_core::conditions::AllergyReport::new(input.substance);
    report.reaction = input
        .reaction
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());
    report.severity = input
        .severity
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());
    report.notes = input
        .notes
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry = host.add_allergy(&report)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[derive(Debug, serde::Deserialize)]
struct DisputedDiagnosisInput {
    label: String,
    attributed_by: Option<String>,
    dispute_reason: Option<String>,
    supporting_notes: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct HousingSafetyInput {
    dwelling_type: Option<String>,
    homelessness: Option<bool>,
    violence_concern: Option<bool>,
    hazards: Option<String>,
    location_notes: Option<String>,
    notes: Option<String>,
}

#[command]
pub fn wellfair_add_disputed_diagnosis(
    app: AppHandle,
    report_json: String,
) -> Result<String, String> {
    let input: DisputedDiagnosisInput =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid disputed JSON: {e}"))?;
    let mut report = wellfare_core::personal_records::DisputedDiagnosisReport::new(input.label);
    report.attributed_by = input.attributed_by.filter(|s| !s.trim().is_empty());
    report.dispute_reason = input.dispute_reason.filter(|s| !s.trim().is_empty());
    report.supporting_notes = input.supporting_notes.filter(|s| !s.trim().is_empty());
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry = host.add_disputed_diagnosis(&report)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_add_housing_safety(
    app: AppHandle,
    report_json: String,
) -> Result<String, String> {
    let input: HousingSafetyInput =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid housing JSON: {e}"))?;
    let mut report = wellfare_core::personal_records::HousingSafetyReport::new();
    if let Some(dt) = input.dwelling_type.as_deref() {
        report.dwelling_type = match dt.to_ascii_lowercase().as_str() {
            "fixed" => wellfare_core::personal_records::DwellingType::Fixed,
            "temporary" => wellfare_core::personal_records::DwellingType::Temporary,
            "mobile_shelter" | "mobileshelter" => {
                wellfare_core::personal_records::DwellingType::MobileShelter
            }
            "homeless" => wellfare_core::personal_records::DwellingType::Homeless,
            _ => wellfare_core::personal_records::DwellingType::Unknown,
        };
    }
    report.homelessness = input.homelessness.unwrap_or(false);
    report.violence_concern = input.violence_concern.unwrap_or(false);
    report.hazards = input.hazards.filter(|s| !s.trim().is_empty());
    report.location_notes = input.location_notes.filter(|s| !s.trim().is_empty());
    report.notes = input.notes.filter(|s| !s.trim().is_empty());
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry = host.add_housing_safety(&report)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

