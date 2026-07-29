use super::consent_store::ConsentGrantRecord;
use super::host_state::{ConsentGrantDraft, PolicyDecisionDto};
use wellfare_core::record::{EpistemicStatus, SensitivityClass};

/// The outcome of a policy decision (maps to `PolicyDecisionDto` for UI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionResult {
    Permit {
        obligations: Vec<String>,
    },
    Deny {
        reasons: Vec<String>,
    },
    Prompt {
        requested_consent: ConsentGrantDraft,
    },
    Suspend {
        required_approvals: u8,
    },
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
    /// qApps permitted to write Classified sanctuary/wellbeing records (Phase 3).
    classified_writers: &'static [&'static str],
}

impl PolicyDecisionService {
    pub fn new() -> Self {
        Self {
            health_writers: &[
                "wellfair-health",
                "wellfair-medication",
                "wellfair-shell",
                "wellfair-life",
                "wellfair-wellbeing",
                "wellfair-finance",
                "wellfair-projects",
                "wellfair-credentials",
                "wellfair-clinical",
                "wellfair-welfare",
                "qualia-cooperative",
                "wellfair-guardianship",
                "wellfair",
            ],
            classified_writers: &[
                "wellfair-shell",
                "wellfair-sanctuary",
                "wellfair-wellbeing",
                "wellfair-life",
            ],
        }
    }

    fn has_active_grant(
        grants: &[ConsentGrantRecord],
        qapp_id: &str,
        scope: &str,
        now_unix: u64,
    ) -> bool {
        grants
            .iter()
            .any(|g| g.is_active(now_unix) && g.recipient == qapp_id && g.scope == scope)
    }

    /// Evaluates if a qApp capability is permitted to act on a record with a given sensitivity.
    ///
    /// `is_proxy_action` marks a write made by an agent acting *on behalf of* the principal
    /// (the envelope carries a `proxy_did` distinct from the owner). Supported-agency
    /// accountability holds such a write in escrow for M-of-N guardian co-signature rather than
    /// committing it silently — see [`super::guardianship`]. Non-proxy writes (the principal
    /// acting for themselves) are unaffected.
    pub fn evaluate_access(
        &self,
        qapp_id: &str,
        requested_scope: &str,
        sensitivity: SensitivityClass,
        epistemic: EpistemicStatus,
        active_grants: &[ConsentGrantRecord],
        now_unix: u64,
        is_proxy_action: bool,
    ) -> DecisionResult {
        if sensitivity == SensitivityClass::Classified {
            if requested_scope == "write_record"
                && self.classified_writers.iter().any(|id| *id == qapp_id)
            {
                return DecisionResult::Permit {
                    obligations: vec![
                        "emit_wal_receipt".into(),
                        "sanctuary_projection_required".into(),
                    ],
                };
            }
            return DecisionResult::Deny {
                reasons: vec!["Classified records require explicit guardian approval".into()],
            };
        }

        if epistemic == EpistemicStatus::Refuted {
            return DecisionResult::Deny {
                reasons: vec!["Refuted claims cannot be written as active records".into()],
            };
        }

        // Supported-agency escrow: a proxy writing a protected record on the principal's behalf
        // does not auto-commit — it suspends pending M-of-N guardian co-signature. (Classified is
        // handled above by the fail-closed writer allowlist; Public needs no escrow.)
        if is_proxy_action
            && requested_scope == "write_record"
            && sensitivity == SensitivityClass::Restricted
        {
            return DecisionResult::Suspend {
                required_approvals: 2,
            };
        }

        match requested_scope {
            "write_record" | "read_record" => {
                if self.health_writers.iter().any(|id| *id == qapp_id) {
                    return DecisionResult::Permit {
                        obligations: vec!["emit_wal_receipt".into()],
                    };
                }
                if Self::has_active_grant(active_grants, qapp_id, requested_scope, now_unix) {
                    return DecisionResult::Permit {
                        obligations: vec![
                            "emit_wal_receipt".into(),
                            "honour_consent_expiry".into(),
                        ],
                    };
                }
                let fields = if requested_scope == "write_record" {
                    vec!["health.observation".into()]
                } else {
                    vec!["health.observation".into(), "profile.display_name".into()]
                };
                DecisionResult::Prompt {
                    requested_consent: ConsentGrantDraft {
                        recipient: qapp_id.to_string(),
                        purpose: format!("{requested_scope} via WellFair host"),
                        fields,
                        expires_at_unix: None,
                    },
                }
            }
            _ => DecisionResult::Deny {
                reasons: vec![format!("Unknown scope: {requested_scope}")],
            },
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
            &[],
            0,
            false,
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
            &[],
            0,
            false,
        );
        assert!(matches!(d, DecisionResult::Permit { .. }));
    }

    #[test]
    fn medication_writer_permitted() {
        let svc = PolicyDecisionService::new();
        let d = svc.evaluate_access(
            "wellfair-medication",
            "write_record",
            SensitivityClass::Restricted,
            EpistemicStatus::Asserted,
            &[],
            0,
            false,
        );
        assert!(matches!(d, DecisionResult::Permit { .. }));
    }

    #[test]
    fn active_grant_permits_third_party_qapp() {
        use super::super::consent_store::ConsentGrantRecord;
        let svc = PolicyDecisionService::new();
        let grant = ConsentGrantRecord {
            id: "g1".into(),
            recipient: "wellfair-care".into(),
            purpose: "care team write".into(),
            fields: vec!["health.observation".into()],
            scope: "write_record".into(),
            granted_at_unix: 1,
            expires_at_unix: None,
            revoked: false,
        };
        let d = svc.evaluate_access(
            "wellfair-care",
            "write_record",
            SensitivityClass::Restricted,
            EpistemicStatus::Asserted,
            &[grant],
            100,
            false,
        );
        assert!(matches!(d, DecisionResult::Permit { .. }));
    }

    #[test]
    fn proxy_restricted_write_suspends_for_guardian_cosignature() {
        let svc = PolicyDecisionService::new();
        // Even a trusted health-writer qapp: a *proxy* write on protected data escrows.
        let d = svc.evaluate_access(
            "wellfair-health",
            "write_record",
            SensitivityClass::Restricted,
            EpistemicStatus::Asserted,
            &[],
            0,
            true,
        );
        assert!(matches!(
            d,
            DecisionResult::Suspend {
                required_approvals: 2
            }
        ));
    }

    #[test]
    fn proxy_public_write_is_not_escrowed() {
        let svc = PolicyDecisionService::new();
        let d = svc.evaluate_access(
            "wellfair-health",
            "write_record",
            SensitivityClass::Public,
            EpistemicStatus::Asserted,
            &[],
            0,
            true,
        );
        assert!(matches!(d, DecisionResult::Permit { .. }));
    }

    #[test]
    fn non_proxy_restricted_write_still_permits() {
        let svc = PolicyDecisionService::new();
        let d = svc.evaluate_access(
            "wellfair-health",
            "write_record",
            SensitivityClass::Restricted,
            EpistemicStatus::Asserted,
            &[],
            0,
            false,
        );
        assert!(matches!(d, DecisionResult::Permit { .. }));
    }
}
