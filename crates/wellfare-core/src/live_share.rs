//! JSON wire protocol for companion live section share (usage agreement + request/decision).

use serde::{Deserialize, Serialize};

pub const MSG_USAGE_AGREEMENT: &str = "USAGE_AGREEMENT";
pub const MSG_LIVE_SECTION_REQUEST: &str = "LIVE_SECTION_REQUEST";
pub const MSG_LIVE_SECTION_DECISION: &str = "LIVE_SECTION_DECISION";
pub const COMPANION_LIVE_SHARE_CONTEXT: &str = "wellfair:live_share";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageAgreement {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub device_id: String,
    pub purpose: String,
    pub allowed_kinds: Vec<String>,
    pub expires_at_unix: u64,
    pub accepted_at_unix: u64,
}

impl UsageAgreement {
    pub fn new(
        device_id: impl Into<String>,
        purpose: impl Into<String>,
        allowed_kinds: Vec<String>,
        expires_at_unix: u64,
        accepted_at_unix: u64,
    ) -> Self {
        Self {
            msg_type: MSG_USAGE_AGREEMENT.into(),
            device_id: device_id.into(),
            purpose: purpose.into(),
            allowed_kinds,
            expires_at_unix,
            accepted_at_unix,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LiveSectionRequest {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub id: String,
    pub device_id: String,
    pub recipient_label: String,
    pub purpose: String,
    pub requested_kinds: Vec<String>,
    pub requested_fields: Vec<String>,
    pub ttl_seconds: u32,
}

impl LiveSectionRequest {
    pub fn new(
        id: impl Into<String>,
        device_id: impl Into<String>,
        recipient_label: impl Into<String>,
        purpose: impl Into<String>,
        requested_kinds: Vec<String>,
        requested_fields: Vec<String>,
        ttl_seconds: u32,
    ) -> Self {
        Self {
            msg_type: MSG_LIVE_SECTION_REQUEST.into(),
            id: id.into(),
            device_id: device_id.into(),
            recipient_label: recipient_label.into(),
            purpose: purpose.into(),
            requested_kinds,
            requested_fields,
            ttl_seconds,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LiveSectionDecision {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub request_id: String,
    pub approved: bool,
    pub projection_kinds: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub decided_at_unix: u64,
}

impl LiveSectionDecision {
    pub fn approved(
        request_id: impl Into<String>,
        projection_kinds: Vec<String>,
        decided_at_unix: u64,
    ) -> Self {
        Self {
            msg_type: MSG_LIVE_SECTION_DECISION.into(),
            request_id: request_id.into(),
            approved: true,
            projection_kinds,
            reason: None,
            decided_at_unix,
        }
    }

    pub fn denied(
        request_id: impl Into<String>,
        reason: impl Into<String>,
        decided_at_unix: u64,
    ) -> Self {
        Self {
            msg_type: MSG_LIVE_SECTION_DECISION.into(),
            request_id: request_id.into(),
            approved: false,
            projection_kinds: vec![],
            reason: Some(reason.into()),
            decided_at_unix,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_agreement_round_trips_json() {
        let msg = UsageAgreement::new(
            "phone-abc",
            "caregiver vitals view",
            vec!["vitals".into(), "sleep".into()],
            1_800_000_000,
            1_700_000_000,
        );
        let json = serde_json::to_string(&msg).unwrap();
        let back: UsageAgreement = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, back);
        assert!(json.contains("\"type\":\"USAGE_AGREEMENT\""));
    }

    #[test]
    fn live_section_request_round_trips_json() {
        let msg = LiveSectionRequest::new(
            "req-001",
            "phone-abc",
            "Desktop WellFair",
            "live section preview",
            vec!["conditions".into()],
            vec!["label".into(), "severity".into()],
            300,
        );
        let json = serde_json::to_string(&msg).unwrap();
        let back: LiveSectionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, back);
        assert!(json.contains("\"type\":\"LIVE_SECTION_REQUEST\""));
    }

    #[test]
    fn live_section_decision_round_trips_json() {
        let approved = LiveSectionDecision::approved("req-001", vec!["conditions".into()], 1_700_000_100);
        let json = serde_json::to_string(&approved).unwrap();
        let back: LiveSectionDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(approved, back);
        assert!(json.contains("\"type\":\"LIVE_SECTION_DECISION\""));
        assert!(!json.contains("reason"));

        let denied = LiveSectionDecision::denied("req-002", "user declined", 1_700_000_200);
        let json = serde_json::to_string(&denied).unwrap();
        let back: LiveSectionDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(denied, back);
        assert!(json.contains("\"reason\":\"user declined\""));
    }

    #[test]
    fn message_types_are_distinct() {
        let usage = UsageAgreement::new("d", "p", vec![], 0, 0);
        let request = LiveSectionRequest::new("i", "d", "r", "p", vec![], vec![], 60);
        let decision = LiveSectionDecision::approved("i", vec![], 0);

        assert_ne!(usage.msg_type, request.msg_type);
        assert_ne!(usage.msg_type, decision.msg_type);
        assert_ne!(request.msg_type, decision.msg_type);
        assert_eq!(usage.msg_type, MSG_USAGE_AGREEMENT);
        assert_eq!(request.msg_type, MSG_LIVE_SECTION_REQUEST);
        assert_eq!(decision.msg_type, MSG_LIVE_SECTION_DECISION);
    }
}