//! Data structures and helpers for consent disclosure grants and revocation receipts.

/// Known clinician contact entry to avoid exposing raw DIDs where a directory/contact is available.
#[derive(Debug, Clone, PartialEq)]
pub struct KnownContact {
    pub id: &'static str,
    pub name: &'static str,
    pub role: &'static str,
    pub did: &'static str,
}

pub const KNOWN_CONTACTS: &[KnownContact] = &[
    KnownContact {
        id: "dr-chen",
        name: "Dr. Sarah Chen",
        role: "Primary Care GP · City Health Clinic",
        did: "did:q42:clinician:sarah-chen",
    },
    KnownContact {
        id: "dr-vance",
        name: "Dr. Marcus Vance",
        role: "Cardiologist · St. Jude Regional",
        did: "did:q42:clinician:marcus-vance",
    },
    KnownContact {
        id: "dr-rostova",
        name: "Dr. Elena Rostova",
        role: "Endocrinologist · Metro Diabetes Center",
        did: "did:q42:clinician:elena-rostova",
    },
    KnownContact {
        id: "st-jude-care-team",
        name: "St. Jude Care Team",
        role: "Multi-Disciplinary Care Team · Inpatient/Outpatient",
        did: "did:q42:org:st-jude-hospital",
    },
];

pub const CATEGORY_OPTIONS: &[(&str, &str, &str)] = &[
    (
        "vitals",
        "Vitals & measurements",
        "Blood pressure, heart rate, and readings",
    ),
    (
        "medications",
        "Medications & prescriptions",
        "Current and past medication history",
    ),
    (
        "conditions",
        "Diagnosed conditions",
        "Active conditions and health concerns",
    ),
    (
        "lab_results",
        "Lab results & analytes",
        "Blood panels, pathology, and biomarkers",
    ),
    (
        "documents",
        "Documents & reports",
        "Discharge summaries and referral reports",
    ),
];

pub const PURPOSE_OPTIONS: &[(&str, &str)] = &[
    (
        "Direct clinical care & consultation",
        "Direct clinical care & consultation",
    ),
    (
        "Specialist referral & second opinion",
        "Specialist referral & second opinion",
    ),
    (
        "Emergency medical assessment",
        "Emergency medical assessment",
    ),
    ("Care team coordination", "Care team coordination"),
    (
        "Personal health record audit & export",
        "Personal health record audit & export",
    ),
];

pub const EXPIRY_OPTIONS: &[(&str, &str, i64)] = &[
    ("24h", "24 hours (consultation)", 86_400),
    ("7d", "7 days (referral & review)", 604_800),
    ("30d", "30 days (care episode)", 2_592_000),
    ("90d", "90 days (quarterly monitoring)", 7_776_000),
    ("1y", "1 year (annual care plan)", 31_536_000),
];

/// Formats recipient display, preferring known clinician names and roles over raw DIDs.
pub fn format_recipient_display(share_to: &str, recipient_label: Option<&str>) -> (String, String) {
    if let Some(contact) = KNOWN_CONTACTS.iter().find(|c| c.did == share_to) {
        (contact.name.to_string(), contact.role.to_string())
    } else if let Some(label) = recipient_label.filter(|l| !l.trim().is_empty()) {
        (label.to_string(), share_to.to_string())
    } else {
        (share_to.to_string(), "External clinician DID".to_string())
    }
}

/// Generates a plain-language summary describing the exact consent grant being authorized.
pub fn generate_plain_language_summary(
    recipient_name: &str,
    categories: &[&str],
    purpose_label: &str,
    expiry_label: &str,
    expiry_rfc3339: &str,
) -> String {
    let cats_str = if categories.is_empty() {
        "no categories (invalid)"
    } else {
        &categories.join(", ")
    };
    format!(
        "You are granting {recipient_name} restricted access to your {cats_str} for {purpose_label}. This grant expires on {expiry_rfc3339} ({expiry_label}). You hold sovereign authority and can revoke this permission immediately at any time with 1 click."
    )
}

/// Construct the field payload for an append-only consent disclosure grant (`health_share`).
///
/// When `ledger` is provided, COP fields carry the ConsentLedger grant id / nonce /
/// binding honesty label so revoke can reconstitute verifying-side material.
pub fn build_consent_grant_payload(
    recipient_did: &str,
    recipient_label: &str,
    purpose: &str,
    categories: &[String],
    duration_seconds: i64,
    sensitivity: &str,
    now_timestamp: i64,
) -> (String, String, serde_json::Map<String, serde_json::Value>) {
    build_consent_grant_payload_with_ledger(
        recipient_did,
        recipient_label,
        purpose,
        categories,
        duration_seconds,
        sensitivity,
        now_timestamp,
        None,
        None,
    )
}

/// Same as [`build_consent_grant_payload`] with optional ledger binding fields.
pub fn build_consent_grant_payload_with_ledger(
    recipient_did: &str,
    recipient_label: &str,
    purpose: &str,
    categories: &[String],
    duration_seconds: i64,
    sensitivity: &str,
    now_timestamp: i64,
    principal_did: Option<&str>,
    ledger: Option<&super::consent_persist::GrantMaterial>,
) -> (String, String, serde_json::Map<String, serde_json::Value>) {
    let expires_at_ts = now_timestamp + duration_seconds;
    let expires_dt = chrono::DateTime::from_timestamp(expires_at_ts, 0)
        .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::seconds(duration_seconds));
    let now_dt =
        chrono::DateTime::from_timestamp(now_timestamp, 0).unwrap_or_else(chrono::Utc::now);

    let title = format!("Disclosure grant · {recipient_label}");
    let mut fields = serde_json::Map::new();
    fields.insert(
        "share_to".into(),
        serde_json::Value::String(recipient_did.trim().to_string()),
    );
    fields.insert(
        "recipient_label".into(),
        serde_json::Value::String(recipient_label.trim().to_string()),
    );
    fields.insert(
        "purpose".into(),
        serde_json::Value::String(purpose.trim().to_string()),
    );
    fields.insert(
        "scope".into(),
        serde_json::Value::String(categories.join(",")),
    );
    fields.insert(
        "expires_at".into(),
        serde_json::Value::String(expires_dt.to_rfc3339()),
    );
    fields.insert(
        "occurred_at".into(),
        serde_json::Value::String(now_dt.to_rfc3339()),
    );
    fields.insert(
        "sensitivity".into(),
        serde_json::Value::String(sensitivity.to_string()),
    );
    if let Some(principal) = principal_did {
        fields.insert(
            "principal_did".into(),
            serde_json::Value::String(principal.trim().to_string()),
        );
    }
    if let Some(grant) = ledger {
        fields.insert(
            "grant_id".into(),
            serde_json::Value::String(super::consent_persist::grant_id_hex(&grant.grant_id)),
        );
        fields.insert(
            "nonce".into(),
            serde_json::Value::String(grant.nonce.to_string()),
        );
        fields.insert(
            "scope_bits".into(),
            serde_json::Value::String(grant.scope_bits.to_string()),
        );
        fields.insert(
            "ledger_binding".into(),
            serde_json::Value::String(super::consent_persist::LEDGER_BINDING_SESSION.into()),
        );
    }

    ("health_share".into(), title, fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_recipient_display_prefers_known_contact_over_raw_did() {
        let (name, role) = format_recipient_display("did:q42:clinician:sarah-chen", None);
        assert_eq!(name, "Dr. Sarah Chen");
        assert!(role.contains("Primary Care GP"));
    }

    #[test]
    fn format_recipient_display_falls_back_to_stored_label_or_did() {
        let (name, role) = format_recipient_display("did:example:dr-unknown", Some("Dr. Unknown"));
        assert_eq!(name, "Dr. Unknown");
        assert_eq!(role, "did:example:dr-unknown");

        let (name2, role2) = format_recipient_display("did:example:dr-raw", None);
        assert_eq!(name2, "did:example:dr-raw");
        assert_eq!(role2, "External clinician DID");
    }

    #[test]
    fn generate_plain_language_summary_contains_all_parameters() {
        let summary = generate_plain_language_summary(
            "Dr. Sarah Chen",
            &["Vitals & measurements", "Lab results"],
            "Direct clinical care",
            "7 days",
            "2026-09-11 12:00 UTC",
        );
        assert!(summary.contains("Dr. Sarah Chen"));
        assert!(summary.contains("Vitals & measurements, Lab results"));
        assert!(summary.contains("Direct clinical care"));
        assert!(summary.contains("2026-09-11 12:00 UTC"));
        assert!(summary.contains("1 click"));
    }

    #[test]
    fn build_consent_grant_payload_calculates_expiry_and_scopes() {
        let now = 1788480000;
        let duration = 604_800; // 7 days
        let (family, title, fields) = build_consent_grant_payload(
            "did:q42:clinician:sarah-chen",
            "Dr. Sarah Chen",
            "Direct clinical care",
            &["vitals".into(), "medications".into()],
            duration,
            "restricted",
            now,
        );

        assert_eq!(family, "health_share");
        assert_eq!(title, "Disclosure grant · Dr. Sarah Chen");
        assert_eq!(
            fields.get("share_to").unwrap(),
            "did:q42:clinician:sarah-chen"
        );
        assert_eq!(fields.get("scope").unwrap(), "vitals,medications");
        assert_eq!(fields.get("sensitivity").unwrap(), "restricted");

        let expires_str = fields.get("expires_at").unwrap().as_str().unwrap();
        let parsed_exp = chrono::DateTime::parse_from_rfc3339(expires_str).unwrap();
        assert_eq!(parsed_exp.timestamp(), now + duration);
    }

    #[test]
    fn category_options_match_consent_contract_flags_only() {
        let ids: Vec<&str> = CATEGORY_OPTIONS.iter().map(|(id, _, _)| *id).collect();
        assert_eq!(
            ids,
            [
                "vitals",
                "medications",
                "conditions",
                "lab_results",
                "documents"
            ]
        );
        assert!(!ids.iter().any(|id| *id == "clinical_notes"));
    }
}
