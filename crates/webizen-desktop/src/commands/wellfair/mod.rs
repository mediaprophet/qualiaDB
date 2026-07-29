//! Wellfair health records, vault, governance, library, clinical, sanctuary

#![allow(non_snake_case)]

use super::*;

pub mod agency;
pub mod anatomy;
pub mod assessment;
pub mod assessment_instruments;
pub mod chora;
pub mod clinical;
pub mod consent_creds;
pub mod credentials;
pub mod crypto;
pub mod disclosure;
pub mod finance;
pub mod guardianship;
pub mod health;
pub mod ledger;
pub mod library;
pub mod life_records;
pub mod med_reminder;
pub mod medication;
pub mod policy;
pub mod projects;
pub mod safeguard;
pub mod sanctuary_basic;
pub mod sanctuary_vault;
pub mod sync;
pub mod welfare_support;
pub mod wellbeing;
pub mod work_items;

// ── Shared helpers ──────────────────────────────────────────────────────────

pub(super) fn wellfair_now_unix() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

pub(super) fn parse_urgency(s: &str) -> wellfare_core::welfare_support::Urgency {
    use wellfare_core::welfare_support::Urgency::*;
    match s.to_ascii_lowercase().as_str() {
        "low" => Low,
        "high" => High,
        "critical" => Critical,
        _ => Moderate,
    }
}

pub(super) fn parse_stream_status(s: &str) -> wellfare_core::welfare_support::StreamStatus {
    use wellfare_core::welfare_support::StreamStatus::*;
    match s.to_ascii_lowercase().as_str() {
        "active" => Active,
        "suspended" => Suspended,
        "ceased" => Ceased,
        "rejected" => Rejected,
        _ => Applied,
    }
}
