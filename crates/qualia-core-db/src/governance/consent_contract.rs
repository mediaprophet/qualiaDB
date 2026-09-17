//! Consent grant and revocation service contract (HLT-03).
//!
//! Provides time-bounded, category-scoped consent grants and immutable revocation
//! receipts. Enforces cryptographic authorization, strict fail-closed expiry,
//! and permanent, non-reactivatable revocation.
//!
//! Maps directly to Deontic Super-Quins (OP_PERMIT / DEFEATER_BIT | OP_FORBID)
//! in `qualia-core-db::modalities::logic::deontic`.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::modalities::logic::deontic::{DEFEATER_BIT, OP_FORBID, OP_PERMIT};
use crate::NQuin;

/// Bitflag representations for health record categories that can be consented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentScope(pub u32);

impl ConsentScope {
    pub const VITALS: u32 = 1 << 0;
    pub const LABS: u32 = 1 << 1;
    pub const CONDITIONS: u32 = 1 << 2;
    pub const MEDICATIONS: u32 = 1 << 3;
    pub const DOCUMENTS: u32 = 1 << 4;

    pub const ALL: u32 =
        Self::VITALS | Self::LABS | Self::CONDITIONS | Self::MEDICATIONS | Self::DOCUMENTS;

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn contains(&self, flag: u32) -> bool {
        (self.0 & flag) == flag
    }

    pub fn category_from_family(family: &str) -> Option<u32> {
        match family {
            "health_vital" | "vitals" => Some(Self::VITALS),
            "health_lab" | "labs" | "lab_results" => Some(Self::LABS),
            "health_condition" | "conditions" => Some(Self::CONDITIONS),
            "health_medication" | "medications" => Some(Self::MEDICATIONS),
            "health_document" | "documents" => Some(Self::DOCUMENTS),
            _ => None,
        }
    }

    /// Parse UI/payload labels into a contract scope. Unknown labels fail closed
    /// (they must not widen authority). Empty input is a scope violation.
    pub fn from_labels(labels: &[&str]) -> Result<Self, ConsentError> {
        if labels.is_empty() {
            return Err(ConsentError::ScopeViolation { requested_flag: 0 });
        }
        let mut bits = 0u32;
        for label in labels {
            let flag = Self::category_from_family(label.trim())
                .ok_or(ConsentError::ScopeViolation { requested_flag: 0 })?;
            bits |= flag;
        }
        Ok(Self(bits))
    }
}

/// Errors occurring during consent authorization or revocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsentError {
    InvalidSignature,
    Expired { expires_at: u64, now: u64 },
    Revoked { revoked_at: u64 },
    UnauthorizedRevocation { actor: [u8; 32] },
    TargetMismatch,
    RequesterMismatch,
    ScopeViolation { requested_flag: u32 },
    ReplayDetected,
    LedgerFull,
    MalformedKey,
}

/// A time-bounded, category-scoped cryptographic consent grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentGrant {
    #[serde(with = "serde_bytes")]
    pub grant_id: [u8; 32],
    #[serde(with = "serde_bytes")]
    pub principal_did: [u8; 32],
    #[serde(with = "serde_bytes")]
    pub recipient_did: [u8; 32],
    pub scope: ConsentScope,
    pub purpose_hash: u64,
    pub created_at: u64,
    pub expires_at: u64,
    pub nonce: u64,
    #[serde(with = "serde_bytes")]
    pub signature: [u8; 64],
}

impl ConsentGrant {
    /// Compute the immutable canonical digest of the grant payload.
    pub fn compute_digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.grant_id);
        hasher.update(&self.principal_did);
        hasher.update(&self.recipient_did);
        hasher.update(&self.scope.0.to_be_bytes());
        hasher.update(&self.purpose_hash.to_be_bytes());
        hasher.update(&self.created_at.to_be_bytes());
        hasher.update(&self.expires_at.to_be_bytes());
        hasher.update(&self.nonce.to_be_bytes());
        let result = hasher.finalize();
        let mut digest = [0u8; 32];
        digest.copy_from_slice(&result);
        digest
    }

    /// Sign and construct a new consent grant.
    pub fn new_signed(
        grant_id: [u8; 32],
        principal_signing_key: &SigningKey,
        recipient_did: [u8; 32],
        scope: ConsentScope,
        purpose_hash: u64,
        created_at: u64,
        expires_at: u64,
        nonce: u64,
    ) -> Self {
        let mut grant = Self {
            grant_id,
            principal_did: principal_signing_key.verifying_key().to_bytes(),
            recipient_did,
            scope,
            purpose_hash,
            created_at,
            expires_at,
            nonce,
            signature: [0u8; 64],
        };
        let digest = grant.compute_digest();
        grant.signature = principal_signing_key.sign(&digest).to_bytes();
        grant
    }

    /// Verify signature and temporal validity. Expiry fails closed.
    pub fn verify(&self, now: u64) -> Result<(), ConsentError> {
        let verifying_key = VerifyingKey::from_bytes(&self.principal_did)
            .map_err(|_| ConsentError::MalformedKey)?;
        let signature = Signature::from_bytes(&self.signature);
        let digest = self.compute_digest();
        verifying_key
            .verify(&digest, &signature)
            .map_err(|_| ConsentError::InvalidSignature)?;

        if now >= self.expires_at {
            return Err(ConsentError::Expired {
                expires_at: self.expires_at,
                now,
            });
        }
        Ok(())
    }

    /// Convert grant into an `OP_PERMIT` Deontic Super-Quin.
    pub fn to_deontic_quin(&self, category_flag: u32) -> NQuin {
        let subject = u64::from_le_bytes(self.recipient_did[0..8].try_into().unwrap())
            & 0x7FFF_FFFF_FFFF_FFFF;
        let predicate = ((category_flag as u64) << 8) | (OP_PERMIT as u64);
        let object = u64::from_le_bytes(self.principal_did[0..8].try_into().unwrap())
            & 0x7FFF_FFFF_FFFF_FFFF;
        let context =
            u64::from_le_bytes(self.grant_id[0..8].try_into().unwrap()) & 0x7FFF_FFFF_FFFF_FFFF;
        let metadata = self.expires_at.min(0xFFFF_FFFF);
        let parity = subject ^ predicate ^ object ^ context;
        NQuin {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity,
        }
    }
}

/// An immutable revocation receipt permanently terminating a consent grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevocationReceipt {
    #[serde(with = "serde_bytes")]
    pub receipt_id: [u8; 32],
    #[serde(with = "serde_bytes")]
    pub grant_id: [u8; 32],
    #[serde(with = "serde_bytes")]
    pub revoked_by: [u8; 32],
    pub revoked_at: u64,
    pub reason_hash: u64,
    #[serde(with = "serde_bytes")]
    pub signature: [u8; 64],
}

impl RevocationReceipt {
    pub fn compute_digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.receipt_id);
        hasher.update(&self.grant_id);
        hasher.update(&self.revoked_by);
        hasher.update(&self.revoked_at.to_be_bytes());
        hasher.update(&self.reason_hash.to_be_bytes());
        let result = hasher.finalize();
        let mut digest = [0u8; 32];
        digest.copy_from_slice(&result);
        digest
    }

    /// Sign and construct a revocation receipt. Must be signed by the grant's principal.
    pub fn new_signed(
        receipt_id: [u8; 32],
        grant: &ConsentGrant,
        revoking_key: &SigningKey,
        revoked_at: u64,
        reason_hash: u64,
    ) -> Self {
        let mut receipt = Self {
            receipt_id,
            grant_id: grant.grant_id,
            revoked_by: revoking_key.verifying_key().to_bytes(),
            revoked_at,
            reason_hash,
            signature: [0u8; 64],
        };
        let digest = receipt.compute_digest();
        receipt.signature = revoking_key.sign(&digest).to_bytes();
        receipt
    }

    /// Verify signature and verify that the revoking actor matches the grant principal.
    pub fn verify_against_grant(&self, grant: &ConsentGrant) -> Result<(), ConsentError> {
        if self.grant_id != grant.grant_id {
            return Err(ConsentError::TargetMismatch);
        }
        if self.revoked_by != grant.principal_did {
            return Err(ConsentError::UnauthorizedRevocation {
                actor: self.revoked_by,
            });
        }
        let verifying_key =
            VerifyingKey::from_bytes(&self.revoked_by).map_err(|_| ConsentError::MalformedKey)?;
        let signature = Signature::from_bytes(&self.signature);
        let digest = self.compute_digest();
        verifying_key
            .verify(&digest, &signature)
            .map_err(|_| ConsentError::InvalidSignature)?;
        Ok(())
    }

    /// Convert revocation receipt into a `DEFEATER_BIT | OP_FORBID` Deontic Super-Quin.
    pub fn to_defeater_quin(&self, grant: &ConsentGrant, category_flag: u32) -> NQuin {
        let subject = u64::from_le_bytes(grant.recipient_did[0..8].try_into().unwrap())
            & 0x7FFF_FFFF_FFFF_FFFF;
        let predicate = DEFEATER_BIT | ((category_flag as u64) << 8) | (OP_FORBID as u64);
        let object = u64::from_le_bytes(grant.principal_did[0..8].try_into().unwrap())
            & 0x7FFF_FFFF_FFFF_FFFF;
        let context =
            u64::from_le_bytes(self.grant_id[0..8].try_into().unwrap()) & 0x7FFF_FFFF_FFFF_FFFF;
        let metadata = self.revoked_at.min(0xFFFF_FFFF);
        let parity = subject ^ predicate ^ object ^ context;
        NQuin {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity,
        }
    }
}

/// Maximum grant/revocation slots in the fail-closed in-memory ledger.
pub const MAX_CONSENT_LEDGER: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SeenGrant {
    grant_id: [u8; 32],
    nonce: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RevokedGrant {
    grant_id: [u8; 32],
    revoked_at: u64,
}

/// Bounded, zero-heap registry: replay of `(grant_id, nonce)` and remembered
/// revocations so omitting a receipt cannot reactivate a grant.
#[derive(Debug, Clone)]
pub struct ConsentLedger {
    seen: [Option<SeenGrant>; MAX_CONSENT_LEDGER],
    n_seen: usize,
    revoked: [Option<RevokedGrant>; MAX_CONSENT_LEDGER],
    n_revoked: usize,
}

impl Default for ConsentLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsentLedger {
    pub const fn new() -> Self {
        Self {
            seen: [None; MAX_CONSENT_LEDGER],
            n_seen: 0,
            revoked: [None; MAX_CONSENT_LEDGER],
            n_revoked: 0,
        }
    }

    fn is_replay(&self, grant: &ConsentGrant) -> bool {
        self.seen.iter().take(self.n_seen).any(|slot| {
            slot.is_some_and(|seen| seen.grant_id == grant.grant_id && seen.nonce == grant.nonce)
        })
    }

    fn revoked_at(&self, grant_id: &[u8; 32]) -> Option<u64> {
        self.revoked.iter().take(self.n_revoked).find_map(|slot| {
            slot.and_then(|rev| (rev.grant_id == *grant_id).then_some(rev.revoked_at))
        })
    }

    /// Register a verified grant. Duplicate `(grant_id, nonce)` is replay.
    pub fn issue(&mut self, grant: &ConsentGrant, now: u64) -> Result<(), ConsentError> {
        grant.verify(now)?;
        if self.is_replay(grant) {
            return Err(ConsentError::ReplayDetected);
        }
        if self.n_seen >= MAX_CONSENT_LEDGER {
            return Err(ConsentError::LedgerFull);
        }
        self.seen[self.n_seen] = Some(SeenGrant {
            grant_id: grant.grant_id,
            nonce: grant.nonce,
        });
        self.n_seen += 1;
        Ok(())
    }

    /// Record a principal-signed revocation. Later authorize calls fail even
    /// if the caller omits the receipt.
    pub fn revoke(
        &mut self,
        grant: &ConsentGrant,
        receipt: &RevocationReceipt,
    ) -> Result<(), ConsentError> {
        receipt.verify_against_grant(grant)?;
        if self.revoked_at(&grant.grant_id).is_some() {
            return Ok(());
        }
        if self.n_revoked >= MAX_CONSENT_LEDGER {
            return Err(ConsentError::LedgerFull);
        }
        self.revoked[self.n_revoked] = Some(RevokedGrant {
            grant_id: grant.grant_id,
            revoked_at: receipt.revoked_at,
        });
        self.n_revoked += 1;
        Ok(())
    }

    pub fn authorize_category_access(
        &self,
        grant: &ConsentGrant,
        revocations: &[RevocationReceipt],
        requested_category: u32,
        actor_did: &[u8; 32],
        current_timestamp: u64,
    ) -> Result<(), ConsentError> {
        if let Some(revoked_at) = self.revoked_at(&grant.grant_id) {
            return Err(ConsentError::Revoked { revoked_at });
        }
        authorize_category_access(
            grant,
            revocations,
            requested_category,
            actor_did,
            current_timestamp,
        )
    }
}

/// Authorize access to a scoped category under a grant and a receipt slice.
///
/// Any verified receipt targeting this grant permanently denies access.
/// Callers that can omit receipts must use [`ConsentLedger`] so revocation
/// cannot be replayed away.
pub fn authorize_category_access(
    grant: &ConsentGrant,
    revocations: &[RevocationReceipt],
    requested_category: u32,
    actor_did: &[u8; 32],
    current_timestamp: u64,
) -> Result<(), ConsentError> {
    for receipt in revocations {
        if receipt.grant_id != grant.grant_id {
            continue;
        }
        receipt.verify_against_grant(grant)?;
        return Err(ConsentError::Revoked {
            revoked_at: receipt.revoked_at,
        });
    }

    grant.verify(current_timestamp)?;

    if &grant.recipient_did != actor_did {
        return Err(ConsentError::RequesterMismatch);
    }

    if !grant.scope.contains(requested_category) {
        return Err(ConsentError::ScopeViolation {
            requested_flag: requested_category,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modalities::logic::deontic::{
        evaluate_deontic_contract, DeonticStatus, DeonticVerdict,
    };
    use ed25519_dalek::SigningKey;

    fn test_key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    #[test]
    fn valid_grant_authorizes_scoped_category() {
        let alice_key = test_key(1);
        let dr_bob_did = test_key(2).verifying_key().to_bytes();
        let now = 1_000_000;
        let grant = ConsentGrant::new_signed(
            [42u8; 32],
            &alice_key,
            dr_bob_did,
            ConsentScope(ConsentScope::VITALS | ConsentScope::LABS),
            100,
            now,
            now + 3600,
            1,
        );

        assert!(authorize_category_access(
            &grant,
            &[],
            ConsentScope::VITALS,
            &dr_bob_did,
            now + 100
        )
        .is_ok());
        assert!(
            authorize_category_access(&grant, &[], ConsentScope::LABS, &dr_bob_did, now + 100)
                .is_ok()
        );
    }

    #[test]
    fn expired_grant_fails_closed() {
        let alice_key = test_key(1);
        let dr_bob_did = test_key(2).verifying_key().to_bytes();
        let now = 1_000_000;
        let grant = ConsentGrant::new_signed(
            [42u8; 32],
            &alice_key,
            dr_bob_did,
            ConsentScope(ConsentScope::VITALS),
            100,
            now,
            now + 3600,
            1,
        );

        let err =
            authorize_category_access(&grant, &[], ConsentScope::VITALS, &dr_bob_did, now + 3601)
                .unwrap_err();
        assert_eq!(
            err,
            ConsentError::Expired {
                expires_at: now + 3600,
                now: now + 3601
            }
        );
    }

    #[test]
    fn revoked_grant_fails_closed_and_cannot_reactivate() {
        let alice_key = test_key(1);
        let dr_bob_did = test_key(2).verifying_key().to_bytes();
        let now = 1_000_000;
        let grant = ConsentGrant::new_signed(
            [42u8; 32],
            &alice_key,
            dr_bob_did,
            ConsentScope(ConsentScope::VITALS),
            100,
            now,
            now + 3600,
            1,
        );
        let receipt = RevocationReceipt::new_signed([99u8; 32], &grant, &alice_key, now + 500, 200);

        let err = authorize_category_access(
            &grant,
            &[receipt],
            ConsentScope::VITALS,
            &dr_bob_did,
            now + 600,
        )
        .unwrap_err();
        assert_eq!(
            err,
            ConsentError::Revoked {
                revoked_at: now + 500
            }
        );
    }

    #[test]
    fn unauthorized_actor_cannot_revoke() {
        let alice_key = test_key(1);
        let mallory_key = test_key(9);
        let dr_bob_did = test_key(2).verifying_key().to_bytes();
        let now = 1_000_000;
        let grant = ConsentGrant::new_signed(
            [42u8; 32],
            &alice_key,
            dr_bob_did,
            ConsentScope(ConsentScope::VITALS),
            100,
            now,
            now + 3600,
            1,
        );
        // Mallory attempts to forge or revoke Alice's grant
        let receipt =
            RevocationReceipt::new_signed([99u8; 32], &grant, &mallory_key, now + 500, 200);

        let err = authorize_category_access(
            &grant,
            &[receipt],
            ConsentScope::VITALS,
            &dr_bob_did,
            now + 600,
        )
        .unwrap_err();
        assert!(matches!(err, ConsentError::UnauthorizedRevocation { .. }));
    }

    #[test]
    fn out_of_scope_category_is_denied() {
        let alice_key = test_key(1);
        let dr_bob_did = test_key(2).verifying_key().to_bytes();
        let now = 1_000_000;
        let grant = ConsentGrant::new_signed(
            [42u8; 32],
            &alice_key,
            dr_bob_did,
            ConsentScope(ConsentScope::VITALS), // Vitals only!
            100,
            now,
            now + 3600,
            1,
        );

        let err =
            authorize_category_access(&grant, &[], ConsentScope::DOCUMENTS, &dr_bob_did, now + 100)
                .unwrap_err();
        assert_eq!(
            err,
            ConsentError::ScopeViolation {
                requested_flag: ConsentScope::DOCUMENTS
            }
        );
    }

    #[test]
    fn tampered_grant_fails_signature_check() {
        let alice_key = test_key(1);
        let dr_bob_did = test_key(2).verifying_key().to_bytes();
        let now = 1_000_000;
        let mut grant = ConsentGrant::new_signed(
            [42u8; 32],
            &alice_key,
            dr_bob_did,
            ConsentScope(ConsentScope::VITALS),
            100,
            now,
            now + 3600,
            1,
        );
        // Tamper with scope post-signing
        grant.scope = ConsentScope(ConsentScope::ALL);
        assert_eq!(
            grant.verify(now).unwrap_err(),
            ConsentError::InvalidSignature
        );
    }

    #[test]
    fn deontic_frame_integration_evaluates_active_and_defeated_states() {
        let alice_key = test_key(1);
        let dr_bob_did = test_key(2).verifying_key().to_bytes();
        let now = 1_000_000;
        let grant = ConsentGrant::new_signed(
            [42u8; 32],
            &alice_key,
            dr_bob_did,
            ConsentScope(ConsentScope::VITALS),
            100,
            now,
            now + 3600,
            1,
        );
        let grant_quin = grant.to_deontic_quin(ConsentScope::VITALS);

        // 1. Frame with grant only -> Active
        let frame_active = [grant_quin.clone()];
        let mut verdicts = [DeonticVerdict::default()];
        let count =
            evaluate_deontic_contract(&frame_active, (now + 100) as u32, &mut verdicts).unwrap();
        assert_eq!(count, 1);
        assert_eq!(verdicts[0].status, DeonticStatus::Active);

        // 2. Frame with grant + revocation defeater -> Defeated
        let receipt = RevocationReceipt::new_signed([99u8; 32], &grant, &alice_key, now + 500, 200);
        let defeater_quin = receipt.to_defeater_quin(&grant, ConsentScope::VITALS);
        let frame_revoked = [grant_quin, defeater_quin];
        let mut verdicts_revoked = [DeonticVerdict::default()];
        let count =
            evaluate_deontic_contract(&frame_revoked, (now + 600) as u32, &mut verdicts_revoked)
                .unwrap();
        assert_eq!(count, 1);
        assert_eq!(verdicts_revoked[0].status, DeonticStatus::Defeated);
    }

    fn vitals_grant(alice: &SigningKey, recipient: [u8; 32], now: u64, nonce: u64) -> ConsentGrant {
        ConsentGrant::new_signed(
            [nonce as u8; 32],
            alice,
            recipient,
            ConsentScope(ConsentScope::VITALS),
            100,
            now,
            now + 3600,
            nonce,
        )
    }

    #[test]
    fn exact_expiry_instant_fails_closed() {
        let alice_key = test_key(1);
        let dr_bob_did = test_key(2).verifying_key().to_bytes();
        let now = 1_000_000;
        let grant = vitals_grant(&alice_key, dr_bob_did, now, 1);
        let err =
            authorize_category_access(&grant, &[], ConsentScope::VITALS, &dr_bob_did, now + 3600)
                .unwrap_err();
        assert!(matches!(err, ConsentError::Expired { .. }));
    }

    #[test]
    fn non_recipient_actor_is_denied() {
        let alice_key = test_key(1);
        let dr_bob_did = test_key(2).verifying_key().to_bytes();
        let mallory_did = test_key(9).verifying_key().to_bytes();
        let now = 1_000_000;
        let grant = vitals_grant(&alice_key, dr_bob_did, now, 1);
        let err =
            authorize_category_access(&grant, &[], ConsentScope::VITALS, &mallory_did, now + 100)
                .unwrap_err();
        assert_eq!(err, ConsentError::RequesterMismatch);
    }

    #[test]
    fn labels_reject_unknown_and_empty_without_widening() {
        assert!(ConsentScope::from_labels(&[]).is_err());
        assert!(ConsentScope::from_labels(&["clinical_notes"]).is_err());
        assert!(ConsentScope::from_labels(&["vitals", "owl:Thing"]).is_err());
        let scope = ConsentScope::from_labels(&["vitals", "lab_results"]).unwrap();
        assert!(scope.contains(ConsentScope::VITALS));
        assert!(scope.contains(ConsentScope::LABS));
        assert!(!scope.contains(ConsentScope::DOCUMENTS));
    }

    #[test]
    fn ledger_detects_replay_and_blocks_reactivation_without_receipt() {
        let alice_key = test_key(1);
        let dr_bob_did = test_key(2).verifying_key().to_bytes();
        let now = 1_000_000;
        let grant = vitals_grant(&alice_key, dr_bob_did, now, 7);
        let mut ledger = ConsentLedger::new();
        ledger.issue(&grant, now + 1).unwrap();
        assert_eq!(
            ledger.issue(&grant, now + 2).unwrap_err(),
            ConsentError::ReplayDetected
        );

        let receipt = RevocationReceipt::new_signed([99u8; 32], &grant, &alice_key, now + 500, 200);
        ledger.revoke(&grant, &receipt).unwrap();

        let err = ledger
            .authorize_category_access(&grant, &[], ConsentScope::VITALS, &dr_bob_did, now + 600)
            .unwrap_err();
        assert_eq!(
            err,
            ConsentError::Revoked {
                revoked_at: now + 500
            }
        );
    }

    #[test]
    fn grant_struct_holds_no_private_key_material() {
        let alice_key = test_key(1);
        let dr_bob_did = test_key(2).verifying_key().to_bytes();
        let grant = vitals_grant(&alice_key, dr_bob_did, 1_000_000, 1);
        assert_eq!(grant.principal_did, alice_key.verifying_key().to_bytes());
        assert_ne!(grant.principal_did, alice_key.to_bytes());
        assert_eq!(grant.signature.len(), 64);
    }
}
