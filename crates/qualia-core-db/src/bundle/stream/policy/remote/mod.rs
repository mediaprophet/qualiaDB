//! Purpose-scoped remote controller recognition, whole-source disclosure policy,
//! RecipientRecord reuse, decision-ticket codec, and R2 issuance/current grant.
//! B2 wrap_for / private M decode / PacketProtection delivery remain separate.
mod codec;
mod decision;
mod policy;
mod recognition;
mod issuer;
mod grant;
mod current;

pub use codec::{
    decode_decision_ticket, encode_decision_ticket_into, ticket_package_len, DecisionTicketFields,
    DisclosureAction, DECISION_TICKET_BYTES, MAX_TICKET_PACKAGE_BYTES,
};
pub use decision::{check_recipient_against_independent, DisclosureDecision, IndependentAdmission};
pub use policy::{
    bind_policy_to_closure, decode_disclosure_policy, encode_disclosure_policy_into, ClosureBudget,
    DisclosurePolicyFields, DisclosurePolicyView, SourceCoverage, MAX_CLOSURE_LABELS,
    MAX_DISCLOSURE_POLICY_BYTES,
};
pub use recognition::{
    AllowedOps, ControllerRecognition, RecognitionAdmin, RecognitionEnrollment, RecognitionHandle,
    RecognizedController, MAX_RECOGNIZED,
};
pub use issuer::fresh_challenge;
#[cfg(not(target_arch = "wasm32"))]
pub use issuer::{issue_disclosure, IssueBuffers, IssueRequest, IssuedLengths, ManagedSigners};
pub use grant::{
    install_remote_disclosure, InstallRequest, RemoteDisclosureGrant, VerifierChallengeSlot,
};

#[cfg(test)]
mod lifecycle_tests;

use super::PolicyError;
use crate::crypto::network::errors::CryptoError;
use crate::governance::consent_contract::ConsentLedger;
use crate::net::qdnf::authority::PolicyOutcome;
use crate::net::qdnf::policy_labels::VerifiedLabel;
use crate::sensing::signed_scope::SignedSensingScope;

/// Existing 1856-byte RecipientRecord is the remote authorization payload.
/// Do not extend Delivery/RecordHeader or smuggle a ticket into A.
pub use crate::crypto::network::object_stream::{
    RecipientRecord, RECIPIENT_BYTES, RECIPIENT_PURPOSE,
};

/// Paired control-message purpose. Defined here; object_stream/types.rs stays frozen.
pub const DISCLOSURE_DECISION_PURPOSE: &[u8] = b"qbdl-stream-protected-v1/disclosure-decision";
pub const MAX_DECISION_LEASE_SECS: u64 = 30;
pub const REMOTE_AUTHORIZATION_BYTES: usize = RECIPIENT_BYTES;

/// Wire role flags never install recognition. Local admin is instance-bound.
pub fn refuse_wire_role_install(_role: u8) -> PolicyError {
    PolicyError::Crypto(CryptoError::Unauthorized)
}

/// SignedControllerChain-style delegation is unsupported in this profile.
pub fn refuse_delegation_chain(_chain: &[u8]) -> PolicyError {
    PolicyError::Crypto(CryptoError::Unsupported)
}

/// Do not accept an all-subjects-consented boolean as co-consent evidence.
pub fn refuse_all_subjects_consented_flag(_flag: bool) -> PolicyError {
    PolicyError::Reference
}

/// VerifiedLabel checks encoding/digest, not recognized issuer authority.
pub fn refuse_verified_label_as_issuer(_label: &VerifiedLabel) -> PolicyError {
    PolicyError::Reference
}

/// ConsentLedger is classical health-category consent, not PQ source authority.
pub fn refuse_consent_ledger_as_pq_source_auth(_ledger: &ConsentLedger) -> PolicyError {
    PolicyError::Crypto(CryptoError::Unsupported)
}

/// SignedSensingScope is a source-specific Ed25519 profile, not whole-source disclosure.
pub fn refuse_signed_sensing_scope_as_whole_source(_scope: &SignedSensingScope) -> PolicyError {
    PolicyError::Crypto(CryptoError::Unsupported)
}

/// PolicyOutcome::Allow is not a remote disclosure decision.
pub fn refuse_policy_outcome(outcome: PolicyOutcome) -> PolicyError {
    let _ = outcome;
    PolicyError::Reference
}

/// A bool trait is not remote authority.
pub fn refuse_bool_authority(_allow: bool) -> PolicyError {
    PolicyError::Reference
}

#[cfg(test)]
mod tests;
