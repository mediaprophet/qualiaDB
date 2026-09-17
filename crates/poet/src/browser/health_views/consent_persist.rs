//! Poet-side ConsentLedger persist seam (HLT-CL).
//!
//! Grant/revoke COP upserts must pass through this helper so replay and
//! revocation are fail-closed before JSON is committed. Cryptographic
//! signing keys never live in Poet UI structs — only verifying DID hashes,
//! scope bits, and opaque grant/receipt identifiers.
//!
//! The in-memory ledger mirrors `qualia_core_db::governance::consent_contract::ConsentLedger`
//! slot / replay / revoke semantics so unit tests can prove issue + revoke
//! hit a real ledger seam without linking the full core-db WASM surface.

use std::sync::Mutex;

/// Same bound as core-db `MAX_CONSENT_LEDGER`.
pub const MAX_CONSENT_LEDGER: usize = 32;

/// Fail-closed persist errors. `Unavailable` is honesty (not a soft success).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsentPersistError {
    Expired { expires_at: u64, now: u64 },
    ReplayDetected,
    Revoked { revoked_at: u64 },
    UnauthorizedRevocation,
    ScopeViolation,
    LedgerFull,
    Unavailable { reason: &'static str },
    Rejected { reason: String },
}

impl ConsentPersistError {
    pub fn user_message(&self) -> String {
        match self {
            Self::Expired { .. } => "Consent grant is already expired (fail closed).".into(),
            Self::ReplayDetected => "Consent grant replay detected; issue rejected.".into(),
            Self::Revoked { .. } => "Consent grant was already revoked.".into(),
            Self::UnauthorizedRevocation => {
                "Only the granting principal may revoke this consent.".into()
            }
            Self::ScopeViolation => {
                "Consent scope is empty or includes unknown categories (fail closed).".into()
            }
            Self::LedgerFull => "Consent ledger is full; cannot record further grants.".into(),
            Self::Unavailable { reason } => format!("Consent ledger unavailable: {reason}"),
            Self::Rejected { reason } => reason.clone(),
        }
    }
}

/// Verifying-side grant material only — no signing private key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantMaterial {
    pub grant_id: [u8; 32],
    pub principal_did: [u8; 32],
    pub recipient_did: [u8; 32],
    pub scope_bits: u32,
    pub purpose_hash: u64,
    pub created_at: u64,
    pub expires_at: u64,
    pub nonce: u64,
}

/// Principal-attested revocation receipt view — no signing private key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevocationMaterial {
    pub receipt_id: [u8; 32],
    pub grant_id: [u8; 32],
    pub revoked_by: [u8; 32],
    pub revoked_at: u64,
    pub reason_hash: u64,
}

/// Port mirroring `ConsentLedger::{issue,revoke}`.
pub trait ConsentLedgerPort {
    fn issue(&mut self, grant: &GrantMaterial, now: u64) -> Result<(), ConsentPersistError>;
    fn revoke(
        &mut self,
        grant: &GrantMaterial,
        receipt: &RevocationMaterial,
    ) -> Result<(), ConsentPersistError>;
}

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

/// Bounded in-memory ConsentLedger used by Poet persist + unit tests.
#[derive(Debug, Clone)]
pub struct InMemoryConsentLedger {
    seen: [Option<SeenGrant>; MAX_CONSENT_LEDGER],
    n_seen: usize,
    revoked: [Option<RevokedGrant>; MAX_CONSENT_LEDGER],
    n_revoked: usize,
}

impl Default for InMemoryConsentLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryConsentLedger {
    pub const fn new() -> Self {
        Self {
            seen: [None; MAX_CONSENT_LEDGER],
            n_seen: 0,
            revoked: [None; MAX_CONSENT_LEDGER],
            n_revoked: 0,
        }
    }

    pub fn seen_count(&self) -> usize {
        self.n_seen
    }

    pub fn revoked_count(&self) -> usize {
        self.n_revoked
    }

    fn is_replay(&self, grant: &GrantMaterial) -> bool {
        self.seen.iter().take(self.n_seen).any(|slot| {
            slot.is_some_and(|seen| seen.grant_id == grant.grant_id && seen.nonce == grant.nonce)
        })
    }

    fn revoked_at(&self, grant_id: &[u8; 32]) -> Option<u64> {
        self.revoked.iter().take(self.n_revoked).find_map(|slot| {
            slot.and_then(|rev| (rev.grant_id == *grant_id).then_some(rev.revoked_at))
        })
    }
}

impl ConsentLedgerPort for InMemoryConsentLedger {
    fn issue(&mut self, grant: &GrantMaterial, now: u64) -> Result<(), ConsentPersistError> {
        if grant.scope_bits == 0 {
            return Err(ConsentPersistError::ScopeViolation);
        }
        if now >= grant.expires_at {
            return Err(ConsentPersistError::Expired {
                expires_at: grant.expires_at,
                now,
            });
        }
        if let Some(revoked_at) = self.revoked_at(&grant.grant_id) {
            return Err(ConsentPersistError::Revoked { revoked_at });
        }
        if self.is_replay(grant) {
            return Err(ConsentPersistError::ReplayDetected);
        }
        if self.n_seen >= MAX_CONSENT_LEDGER {
            return Err(ConsentPersistError::LedgerFull);
        }
        self.seen[self.n_seen] = Some(SeenGrant {
            grant_id: grant.grant_id,
            nonce: grant.nonce,
        });
        self.n_seen += 1;
        Ok(())
    }

    fn revoke(
        &mut self,
        grant: &GrantMaterial,
        receipt: &RevocationMaterial,
    ) -> Result<(), ConsentPersistError> {
        if receipt.grant_id != grant.grant_id {
            return Err(ConsentPersistError::Rejected {
                reason: "Revocation receipt does not target this grant.".into(),
            });
        }
        if receipt.revoked_by != grant.principal_did {
            return Err(ConsentPersistError::UnauthorizedRevocation);
        }
        if self.revoked_at(&grant.grant_id).is_some() {
            return Ok(());
        }
        if self.n_revoked >= MAX_CONSENT_LEDGER {
            return Err(ConsentPersistError::LedgerFull);
        }
        self.revoked[self.n_revoked] = Some(RevokedGrant {
            grant_id: grant.grant_id,
            revoked_at: receipt.revoked_at,
        });
        self.n_revoked += 1;
        Ok(())
    }
}

/// Issue a grant on the ledger before any COP JSON upsert.
pub fn persist_issue(
    ledger: &mut dyn ConsentLedgerPort,
    grant: &GrantMaterial,
    now: u64,
) -> Result<(), ConsentPersistError> {
    ledger.issue(grant, now)
}

/// Revoke a grant on the ledger before any COP JSON upsert.
pub fn persist_revoke(
    ledger: &mut dyn ConsentLedgerPort,
    grant: &GrantMaterial,
    receipt: &RevocationMaterial,
) -> Result<(), ConsentPersistError> {
    ledger.revoke(grant, receipt)
}

/// ConsentScope bitflags (aligned with core-db `ConsentScope`).
pub mod scope {
    pub const VITALS: u32 = 1 << 0;
    pub const LABS: u32 = 1 << 1;
    pub const CONDITIONS: u32 = 1 << 2;
    pub const MEDICATIONS: u32 = 1 << 3;
    pub const DOCUMENTS: u32 = 1 << 4;

    pub fn flag_for_label(label: &str) -> Option<u32> {
        match label.trim() {
            "health_vital" | "vitals" => Some(VITALS),
            "health_lab" | "labs" | "lab_results" => Some(LABS),
            "health_condition" | "conditions" => Some(CONDITIONS),
            "health_medication" | "medications" => Some(MEDICATIONS),
            "health_document" | "documents" => Some(DOCUMENTS),
            _ => None,
        }
    }

    pub fn from_labels(labels: &[&str]) -> Result<u32, super::ConsentPersistError> {
        if labels.is_empty() {
            return Err(super::ConsentPersistError::ScopeViolation);
        }
        let mut bits = 0u32;
        for label in labels {
            let flag = flag_for_label(label).ok_or(super::ConsentPersistError::ScopeViolation)?;
            bits |= flag;
        }
        Ok(bits)
    }
}

/// Stable 32-byte fingerprint of a DID / purpose string (no private key material).
pub fn fingerprint32(input: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    let bytes = input.as_bytes();
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for (i, b) in bytes.iter().enumerate() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
        out[i % 32] ^= *b;
        out[(i + 7) % 32] ^= ((hash >> 8) & 0xff) as u8;
        out[(i + 13) % 32] ^= ((hash >> 24) & 0xff) as u8;
        out[(i + 19) % 32] ^= ((hash >> 40) & 0xff) as u8;
    }
    out[0] ^= (bytes.len() as u8).wrapping_mul(31);
    out
}

pub fn grant_id_hex(grant_id: &[u8; 32]) -> String {
    grant_id.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn parse_grant_id_hex(hex: &str) -> Option<[u8; 32]> {
    let hex = hex.trim();
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Build verifying-side grant material from disclosure workspace fields.
pub fn grant_material_from_disclosure(
    principal_did: &str,
    recipient_did: &str,
    purpose: &str,
    categories: &[String],
    created_at: u64,
    expires_at: u64,
    nonce: u64,
) -> Result<GrantMaterial, ConsentPersistError> {
    let labels: Vec<&str> = categories.iter().map(|s| s.as_str()).collect();
    let scope_bits = scope::from_labels(&labels)?;
    let principal = fingerprint32(principal_did.trim());
    let recipient = fingerprint32(recipient_did.trim());
    let purpose_hash = {
        let fp = fingerprint32(purpose.trim());
        u64::from_le_bytes(fp[0..8].try_into().unwrap())
    };
    let mut id_src = String::new();
    id_src.push_str(principal_did.trim());
    id_src.push('|');
    id_src.push_str(recipient_did.trim());
    id_src.push('|');
    id_src.push_str(&scope_bits.to_string());
    id_src.push('|');
    id_src.push_str(&nonce.to_string());
    id_src.push('|');
    id_src.push_str(&created_at.to_string());
    let grant_id = fingerprint32(&id_src);

    Ok(GrantMaterial {
        grant_id,
        principal_did: principal,
        recipient_did: recipient,
        scope_bits,
        purpose_hash,
        created_at,
        expires_at,
        nonce,
    })
}

/// Build revocation material for an active share (principal-only).
pub fn revocation_material_from_grant(
    grant: &GrantMaterial,
    reason: &str,
    revoked_at: u64,
) -> RevocationMaterial {
    let mut id_src = String::new();
    id_src.push_str(&grant_id_hex(&grant.grant_id));
    id_src.push('|');
    id_src.push_str(&revoked_at.to_string());
    id_src.push('|');
    id_src.push_str(reason.trim());
    RevocationMaterial {
        receipt_id: fingerprint32(&id_src),
        grant_id: grant.grant_id,
        revoked_by: grant.principal_did,
        revoked_at,
        reason_hash: u64::from_le_bytes(fingerprint32(reason.trim())[0..8].try_into().unwrap()),
    }
}

static SESSION_LEDGER: Mutex<Option<InMemoryConsentLedger>> = Mutex::new(None);

/// Session-local ConsentLedger for the Poet browser process.
pub fn with_session_ledger<R>(f: impl FnOnce(&mut InMemoryConsentLedger) -> R) -> R {
    let mut guard = SESSION_LEDGER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.is_none() {
        *guard = Some(InMemoryConsentLedger::new());
    }
    f(guard.as_mut().expect("session ledger initialized"))
}

/// Honesty label when the daemon/crypto ledger path is not reachable.
pub const LEDGER_UNAVAILABLE_REASON: &str =
    "cryptographic ConsentLedger not reachable from Poet UI; session ledger gate only";

/// Label written into COP fields so projection stays honest about binding strength.
pub const LEDGER_BINDING_SESSION: &str = "poet_session_consent_ledger";

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_grant(nonce: u64) -> GrantMaterial {
        grant_material_from_disclosure(
            "did:q42:principal:alice",
            "did:q42:clinician:bob",
            "Direct clinical care",
            &["vitals".into(), "labs".into()],
            1_000_000,
            1_003_600,
            nonce,
        )
        .unwrap()
    }

    #[test]
    fn persist_issue_and_revoke_hit_in_memory_consent_ledger() {
        let mut ledger = InMemoryConsentLedger::new();
        let grant = sample_grant(7);
        persist_issue(&mut ledger, &grant, 1_000_100).unwrap();
        assert_eq!(ledger.seen_count(), 1);

        assert_eq!(
            persist_issue(&mut ledger, &grant, 1_000_200).unwrap_err(),
            ConsentPersistError::ReplayDetected
        );

        let receipt = revocation_material_from_grant(&grant, "patient revoke", 1_000_500);
        persist_revoke(&mut ledger, &grant, &receipt).unwrap();
        assert_eq!(ledger.revoked_count(), 1);

        let err = persist_issue(&mut ledger, &grant, 1_000_600).unwrap_err();
        assert_eq!(
            err,
            ConsentPersistError::Revoked {
                revoked_at: 1_000_500
            }
        );
    }

    #[test]
    fn unknown_scope_labels_fail_closed() {
        let err = grant_material_from_disclosure(
            "did:q42:alice",
            "did:q42:bob",
            "care",
            &["clinical_notes".into()],
            1,
            100,
            1,
        )
        .unwrap_err();
        assert_eq!(err, ConsentPersistError::ScopeViolation);
    }

    #[test]
    fn expired_grant_fails_closed_on_issue() {
        let mut ledger = InMemoryConsentLedger::new();
        let grant = sample_grant(1);
        let err = persist_issue(&mut ledger, &grant, grant.expires_at).unwrap_err();
        assert!(matches!(err, ConsentPersistError::Expired { .. }));
        assert_eq!(ledger.seen_count(), 0);
    }

    #[test]
    fn non_principal_cannot_revoke() {
        let mut ledger = InMemoryConsentLedger::new();
        let grant = sample_grant(3);
        persist_issue(&mut ledger, &grant, 1_000_100).unwrap();
        let mut receipt = revocation_material_from_grant(&grant, "forged", 1_000_200);
        receipt.revoked_by = fingerprint32("did:q42:mallory");
        assert_eq!(
            persist_revoke(&mut ledger, &grant, &receipt).unwrap_err(),
            ConsentPersistError::UnauthorizedRevocation
        );
        assert_eq!(ledger.revoked_count(), 0);
    }

    #[test]
    fn grant_id_hex_round_trips() {
        let grant = sample_grant(9);
        let hex = grant_id_hex(&grant.grant_id);
        assert_eq!(parse_grant_id_hex(&hex), Some(grant.grant_id));
    }
}
