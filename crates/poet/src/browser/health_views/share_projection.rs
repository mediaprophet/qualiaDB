//! Fail-closed projection of consent grants (`health_share`) and revocation receipts.

use super::model::HealthRecord;

/// Status of a consent disclosure grant.
#[derive(Debug, Clone, PartialEq)]
pub enum ShareStatus {
    Active,
    Expired {
        expires_at: String,
    },
    Revoked {
        receipt_id: String,
        reason: String,
        revoked_at: String,
    },
}

/// A projected consent disclosure pairing the underlying health_share record with its status.
#[derive(Debug, Clone, PartialEq)]
pub struct ShareItem {
    pub record: HealthRecord,
    pub share_to: String,
    pub purpose: String,
    pub scope: String,
    pub status: ShareStatus,
}

/// Project health_share records and identify active vs expired vs revoked status.
///
/// Missing expiry fails closed (Expired). Missing scope never widens to "all
/// categories".
pub fn project_shares(records: &[HealthRecord], now_timestamp: i64) -> Vec<ShareItem> {
    let mut revocations: std::collections::HashMap<String, &HealthRecord> =
        std::collections::HashMap::new();

    for record in records {
        if record.family == "health_safeguard" || record.family == "health_revocation" {
            if let Some(target) = record.field_text("targets_id") {
                revocations.insert(target, record);
            }
        }
    }

    records
        .iter()
        .filter(|record| record.family == "health_share")
        .map(|record| {
            let share_to = record
                .field_text("share_to")
                .unwrap_or_else(|| "Unspecified recipient".into());
            let purpose = record
                .field_text("purpose")
                .unwrap_or_else(|| "General care".into());
            let scope = record
                .field_text("scope")
                .unwrap_or_else(|| "unspecified (fail closed)".into());

            let status = if let Some(receipt) = revocations.get(&record.id) {
                let receipt_id = receipt.id.clone();
                let reason = receipt
                    .field_text("reason")
                    .unwrap_or_else(|| "Revoked by patient".into());
                let revoked_at = receipt.occurred_label();
                ShareStatus::Revoked {
                    receipt_id,
                    reason,
                    revoked_at,
                }
            } else if let Some(expires_raw) = record.field_text("expires_at") {
                let expiry_ts = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&expires_raw) {
                    dt.timestamp()
                } else if let Ok(ts) = expires_raw.parse::<i64>() {
                    ts
                } else {
                    i64::MAX
                };
                if now_timestamp >= expiry_ts {
                    ShareStatus::Expired {
                        expires_at: expires_raw,
                    }
                } else {
                    ShareStatus::Active
                }
            } else {
                ShareStatus::Expired {
                    expires_at: "missing — fail closed".into(),
                }
            };

            ShareItem {
                record: record.clone(),
                share_to,
                purpose,
                scope,
                status,
            }
        })
        .collect()
}

/// Construct the field payload for an append-only consent revocation receipt.
pub fn build_consent_revocation_payload(
    share: &HealthRecord,
    reason: &str,
    sensitivity: &str,
) -> (String, String, serde_json::Map<String, serde_json::Value>) {
    let now = chrono::Utc::now();
    let receipt_title = format!("Revocation receipt · {}", share.title);
    let mut fields = serde_json::Map::new();
    fields.insert(
        "targets_id".into(),
        serde_json::Value::String(share.id.clone()),
    );
    fields.insert(
        "revoked_grant_family".into(),
        serde_json::Value::String("health_share".into()),
    );
    fields.insert(
        "reason".into(),
        serde_json::Value::String(reason.trim().to_string()),
    );
    fields.insert(
        "share_to".into(),
        serde_json::Value::String(share.field_text("share_to").unwrap_or_default()),
    );
    fields.insert(
        "occurred_at".into(),
        serde_json::Value::String(now.to_rfc3339()),
    );
    fields.insert(
        "sensitivity".into(),
        serde_json::Value::String(sensitivity.to_string()),
    );
    ("health_safeguard".into(), receipt_title, fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::health_views::model::{records_from_payload, HealthRecord};
    use serde_json::json;

    #[test]
    fn project_shares_identifies_active_expired_and_revoked_states() {
        let records = records_from_payload(
            "health_share",
            &json!({"records": [
                {
                    "id": "share-active",
                    "family": "health_share",
                    "title": "Dr. Smith vitals share",
                    "fields": {
                        "share_to": "did:example:dr-smith",
                        "purpose": "cardiology-review",
                        "scope": "health_vital",
                        "expires_at": "2026-12-31T23:59:59Z"
                    },
                    "created_at": 100, "updated_at": 100
                },
                {
                    "id": "share-expired",
                    "family": "health_share",
                    "title": "Emergency intake share",
                    "fields": {
                        "share_to": "did:example:clinic",
                        "purpose": "intake",
                        "expires_at": "2026-01-01T00:00:00Z"
                    },
                    "created_at": 50, "updated_at": 50
                },
                {
                    "id": "share-revoked",
                    "family": "health_share",
                    "title": "Old consultation share",
                    "fields": {
                        "share_to": "did:example:consultant",
                        "purpose": "second-opinion"
                    },
                    "created_at": 80, "updated_at": 80
                },
                {
                    "id": "receipt-rev-1",
                    "family": "health_safeguard",
                    "title": "Revocation receipt · Old consultation share",
                    "fields": {
                        "targets_id": "share-revoked",
                        "reason": "Consultation concluded",
                        "occurred_at": "2026-06-01T10:00:00Z"
                    },
                    "created_at": 90, "updated_at": 90
                }
            ]}),
        );

        let now = 1_788_000_000; // mid 2026
        let shares = project_shares(&records, now);
        assert_eq!(shares.len(), 3);

        let active_item = shares
            .iter()
            .find(|s| s.record.id == "share-active")
            .unwrap();
        assert_eq!(active_item.status, ShareStatus::Active);

        let expired_item = shares
            .iter()
            .find(|s| s.record.id == "share-expired")
            .unwrap();
        assert!(matches!(expired_item.status, ShareStatus::Expired { .. }));

        let revoked_item = shares
            .iter()
            .find(|s| s.record.id == "share-revoked")
            .unwrap();
        match &revoked_item.status {
            ShareStatus::Revoked {
                receipt_id, reason, ..
            } => {
                assert_eq!(receipt_id, "receipt-rev-1");
                assert_eq!(reason, "Consultation concluded");
            }
            other => panic!("Expected Revoked, got {:?}", other),
        }

        let expired_scope = shares
            .iter()
            .find(|s| s.record.id == "share-expired")
            .unwrap();
        assert_ne!(expired_scope.scope, "All categories");
        assert_eq!(expired_scope.scope, "unspecified (fail closed)");
    }

    #[test]
    fn project_shares_missing_expiry_fails_closed_and_never_claims_all_categories() {
        let records = records_from_payload(
            "health_share",
            &json!({"records": [{
                "id": "share-no-expiry",
                "family": "health_share",
                "title": "Open-ended share",
                "fields": {
                    "share_to": "did:example:clinic",
                    "purpose": "review"
                },
                "created_at": 10, "updated_at": 10
            }]}),
        );
        let shares = project_shares(&records, 1_788_000_000);
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].scope, "unspecified (fail closed)");
        match &shares[0].status {
            ShareStatus::Expired { expires_at } => {
                assert!(expires_at.contains("fail closed"));
            }
            other => panic!("Expected Expired fail-closed, got {:?}", other),
        }
    }

    #[test]
    fn build_consent_revocation_payload_records_target_and_reason() {
        let share = HealthRecord {
            id: "share-42".into(),
            family: "health_share".into(),
            title: "Cardio share".into(),
            fields: serde_json::Map::new(),
            created_at: 100,
            updated_at: 100,
        };
        let (family, title, fields) =
            build_consent_revocation_payload(&share, "Revoked by patient request", "classified");
        assert_eq!(family, "health_safeguard");
        assert_eq!(title, "Revocation receipt · Cardio share");
        assert_eq!(
            fields.get("targets_id").unwrap().as_str().unwrap(),
            "share-42"
        );
        assert_eq!(
            fields
                .get("revoked_grant_family")
                .unwrap()
                .as_str()
                .unwrap(),
            "health_share"
        );
        assert_eq!(
            fields.get("reason").unwrap().as_str().unwrap(),
            "Revoked by patient request"
        );
        assert_eq!(
            fields.get("sensitivity").unwrap().as_str().unwrap(),
            "classified"
        );
        assert!(fields.contains_key("occurred_at"));
    }
}
