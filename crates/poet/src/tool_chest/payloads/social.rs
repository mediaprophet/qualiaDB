//! Social payload types: connection requests, risk assessment, edge queries.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)
//!
//! These structs are the `P` parameter in `VibeScriptPayload<P>`.
//! They serialise to CBOR-LD for wire transmission.

// ---------------------------------------------------------------------------
// Connection Request payloads
// ---------------------------------------------------------------------------

/// Parameters for submitting a connection request.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SubmitConnectionRequestParams {
    /// DID of the requester.
    pub requester_did: String,
    /// DID of the target.
    pub target_did: String,
    /// Requested edge type — e.g. `soc:friendship`, `soc:professional`.
    pub requested_edge_type: String,
    /// ZKP claims to request from the requester.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zkp_claims: Vec<String>,
}

/// Parameters for querying connection request status.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct QueryConnectionRequestParams {
    /// Connection request IRI or DID.
    pub request_id: String,
}

/// Parameters for acting on a connection request (accept, decline, block).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ActOnConnectionRequestParams {
    /// Connection request IRI.
    pub request_id: String,
    /// Action to take.
    pub action: ConnectionRequestAction,
    /// Optional reason (for decline/block).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Actions that can be taken on a connection request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionRequestAction {
    Accept,
    Decline,
    Block,
    Withdraw,
}

// ---------------------------------------------------------------------------
// Risk Assessment payloads
// ---------------------------------------------------------------------------

/// Parameters for querying risk assessment results.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct QueryRiskAssessmentParams {
    /// Connection request IRI.
    pub request_id: String,
}

/// A risk assessment result returned from the backend.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RiskAssessmentResult {
    /// Risk level — `none`, `low`, `moderate`, `high`, `critical`.
    pub risk_level: String,
    /// Risk indicators detected.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indicators: Vec<String>,
    /// Whether the request was blocked automatically.
    pub blocked: bool,
    /// Whether guardian/protector approval is required.
    pub protector_approval_required: bool,
}

// ---------------------------------------------------------------------------
// Social Edge payloads
// ---------------------------------------------------------------------------

/// Parameters for querying social edges.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct QuerySocialEdgesParams {
    /// DID of the entity whose edges to query.
    pub entity_did: String,
    /// Optional edge type filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_type: Option<String>,
    /// Optional disclosure level filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disclosure_level: Option<String>,
}

/// Parameters for updating a social edge's enumerable characteristics.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UpdateEdgeCharacteristicsParams {
    /// Edge IRI.
    pub edge_iri: String,
    /// Duration category — `new`, `recent`, `established`, `long-term`, `enduring`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    /// Interaction frequency — `daily`, `weekly`, `monthly`, `quarterly`, `rare`, `none`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction_frequency: Option<String>,
    /// Reciprocity — `mutual`, `asymmetric`, `one-sided`, `transactional`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reciprocity: Option<String>,
    /// Power dynamic — `peer`, `hierarchical`, `asymmetric`, `dependent`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_dynamic: Option<String>,
    /// Disclosure level — `public`, `acquaintance`, `personal`, `intimate`, `selfhood`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disclosure_level: Option<String>,
    /// Tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

// ---------------------------------------------------------------------------
// Vulnerable Person Protection payloads
// ---------------------------------------------------------------------------

/// Parameters for querying vulnerable person protection policies.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct QueryProtectionPoliciesParams {
    /// DID of the protected person.
    pub protected_person_did: String,
}

/// Parameters for creating or updating a protection policy.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UpdateProtectionPolicyParams {
    /// DID of the protected person.
    pub protected_person_did: String,
    /// Vulnerability category — e.g. `minor`, `dv-survivor`, `whistleblower`.
    pub category: String,
    /// Whether this protection is mandatory.
    pub mandatory: bool,
    /// Connection approval mode — `always-required`, `age-based`, `risk-based`, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_approval: Option<String>,
    /// Max disclosure level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_disclosure_level: Option<String>,
    /// Monitoring mode — `passive`, `active`, `off`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitoring_mode: Option<String>,
    /// Alert triggers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alert_triggers: Vec<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_request_params_serialise() {
        let params = SubmitConnectionRequestParams {
            requester_did: "did:qualia:alice".into(),
            target_did: "did:qualia:bob".into(),
            requested_edge_type: "soc:friendship".into(),
            zkp_claims: vec!["age > 18".into(), "identity uniqueness".into()],
        };

        let mut cbor = Vec::new();
        ciborium::ser::into_writer(&params, &mut cbor).expect("cbor encode");
        let decoded: SubmitConnectionRequestParams =
            ciborium::from_reader(&cbor[..]).expect("cbor decode");
        assert_eq!(decoded.requester_did, "did:qualia:alice");
        assert_eq!(decoded.zkp_claims.len(), 2);
    }

    #[test]
    fn risk_assessment_result_serialise() {
        let result = RiskAssessmentResult {
            risk_level: "moderate".into(),
            indicators: vec!["new-account".into(), "no-shared-contacts".into()],
            blocked: false,
            protector_approval_required: true,
        };

        let mut cbor = Vec::new();
        ciborium::ser::into_writer(&result, &mut cbor).expect("cbor encode");
        let decoded: RiskAssessmentResult = ciborium::from_reader(&cbor[..]).expect("cbor decode");
        assert_eq!(decoded.risk_level, "moderate");
        assert!(!decoded.blocked);
        assert!(decoded.protector_approval_required);
    }
}
