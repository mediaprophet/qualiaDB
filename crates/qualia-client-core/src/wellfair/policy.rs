use super::host_state::{ConsentGrantDraft, PolicyDecisionDto};
use wellfare_core::record::{EpistemicStatus, SensitivityClass};

/// The outcome of a policy decision (maps to `PolicyDecisionDto` for UI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionResult {
    Permit { obligations: Vec<String> },
    Deny { reasons: Vec<String> },
    Prompt { requested_consent: ConsentGrantDraft },
    Suspend { required_approvals: u8 },
}

/// The receipt bound to a policy decision for auditability.
#[derive(Debug, Clone)]
pub struct DecisionReceipt {
    pub id: String,
    pub timestamp_unix: u32,
    pub result: DecisionResult,
}

impl DecisionResult {
    pub fn to_dto(&self) -> PolicyDecisionDto {
        match self {
            Self::Permit { obligations } => PolicyDecisionDto::Permit {
                obligations: obligations.clone(),
            },
            Self::Deny { reasons } => PolicyDecisionDto::Deny {
                reasons: reasons.clone(),
            },
            Self::Prompt { requested_consent } => PolicyDecisionDto::Prompt {
                requested_consent: requested_consent.clone(),
            },
            Self::Suspend { required_approvals } => PolicyDecisionDto::Suspend {
                required_approvals: *required_approvals,
            },
        }
    }
}

pub struct PolicyDecisionService {
    /// qApp IDs permitted to write health observations without extra prompt.
    health_writers: &'static [&'static str],
}

impl PolicyDecisionService {
    pub fn new() -> Self {
        Self {
            health_writers: &["wellfair-health", "wellfair-shell", "wellfair"],
        }
    }

    /// Evaluates if a qApp capability is permitted to act on a record with a given sensitivity.
    pub fn evaluate_access(
        &self,
        qapp_id: &str,
        requested_scope: &str,
        sensitivity: SensitivityClass,
        epistemic: EpistemicStatus,
    ) -> DecisionResult {
        if sensitivity == SensitivityClass::Classified {
            return DecisionResult::Deny {
                reasons: vec!["Classified records require explicit guardian approval".into()],
            };
        }

        if epistemic == EpistemicStatus::Refuted {
            return DecisionResult::Deny {
                reasons: vec!["Refuted claims cannot be written as active records".into()],
            };
        }

        if requested_scope == "write_record" {
            if self.health_writers.iter().any(|id| *id == qapp_id) {
                return DecisionResult::Permit {
                    obligations: vec!["emit_wal_receipt".into()],
                };
            }
            return DecisionResult::Prompt {
                requested_consent: ConsentGrantDraft {
                    recipient: qapp_id.to_string(),
                    purpose: format!("write_record via {requested_scope}"),
                    fields: vec!["health.observation".into()],
                    expires_at_unix: None,
                },
            };
        }

        DecisionResult::Deny {
            reasons: vec![format!("Unknown scope: {requested_scope}")],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classified_fails_closed() {
        let svc = PolicyDecisionService::new();
        let d = svc.evaluate_access(
            "wellfair-health",
            "write_record",
            SensitivityClass::Classified,
            EpistemicStatus::Asserted,
        );
        assert!(matches!(d, DecisionResult::Deny { .. }));
    }

    #[test]
    fn health_writer_permitted() {
        let svc = PolicyDecisionService::new();
        let d = svc.evaluate_access(
            "wellfair-health",
            "write_record",
            SensitivityClass::Restricted,
            EpistemicStatus::Asserted,
        );
        assert!(matches!(d, DecisionResult::Permit { .. }));
    }
}
