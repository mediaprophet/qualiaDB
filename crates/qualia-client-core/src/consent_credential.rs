//! **Revocable consent credentials** with *crypto-enforced* payload revocation + a **durable, attestable
//! conduct record** — the mechanism that resolves "revocable data vs durable accountability"
//! (`docs/plans/social-worker-support-and-accountability.md` §2 + §4).
//!
//! The consideration (Timothy, 2026-07-06): a person can grant an agent (a social worker — a human) a
//! **consent credential** to an **encrypted payload**, and can **take it away**. When taken away the
//! **payload becomes unavailable** — *not* by flipping a flag, but by **destroying the key** (envelope
//! encryption: the payload is `Enc(data_key, payload)`; the credential carries the *wrapped* data key;
//! revoke ⇒ the wrapped key is destroyed ⇒ no key, no payload). **But the agent's interaction records
//! persist** — how and why they acted — so revoking consent cannot erase a worker's accountability, and a
//! worker cannot hold the person's data hostage *for* accountability. The durable [`ConductRecord`] binds to
//! a **commitment** of the payload (not the payload), and carries an [`Attestation`] (a signature and/or a
//! zero-knowledge proof over the real `crypto/zk_proofs` system) so a **court can audit** that the agent
//! acted, on what basis, at what time — *without* re-exposing the revoked private data.
//!
//! **The payload lives in a permissive commons** (Timothy, 2026-07-06). The ciphertext is an
//! [`EncryptedCommonsPayload`] — **stored/replicated by many parties so it cannot be deleted** (anti-erasure:
//! not by an agent covering their tracks, not by a hostile actor destroying evidence, not by accidental
//! loss) — **yet accessible only to holders of the right credential**. Wide storage ≠ wide access: the
//! ciphertext is useless without a key. This resolves *anti-deletion vs privacy vs access-control* at once,
//! and it sharpens revocation: you cannot delete bytes others hold, so **revocation is access, not
//! deletion** — revoke a credential ⇒ that holder's key is destroyed ⇒ they lose access, while the durable
//! commons ciphertext persists for other holders and as un-erasable evidence. The person's *ultimate*
//! control is **crypto-shredding**: destroy **all** keys ⇒ the ciphertext is permanently unreadable by
//! anyone (effective erasure) even though the bytes survive. (Continuous with the permissive-commons +
//! distributed-memory-custody + erasure-prevention stances elsewhere.)
//!
//! **Credentials are not only self-consent, and need not be unilateral** (Timothy, 2026-07-06). A
//! credential's authority may derive from the **subject**, from a **court** (to support proceedings / the
//! audit case), or from another attested **authority** (a statutory body, a guardian) — see
//! [`CredentialAuthority`]. And a credential may be **multi-signature** ([`Authorization::MultiSig`]): an
//! exercise then requires (a) **instigation by a participating party** — so no outside/authority actor can
//! act alone — **and** (b) a **threshold** of party signatures. Even a valid court credential, if multi-sig,
//! cannot be exercised without a participating party setting it in motion and the threshold signing. This is
//! the check on authority: *unable to act without instigation of one of the participating parties.*
//!
//! **Scope of this module.** The pure **domain model + the invariants** — the commons payload, the
//! *revocable* per-holder access, the court/authority + multi-sig authorization, and the *durable* conduct
//! trail. It does **not** perform the actual
//! envelope encryption, the real Groth16 proof, the replication/seeding, or the `consent_store`/vault
//! wiring — those compose from `wellfair/consent_store.rs` (whose flag-`revoke` this design says should
//! become crypto-enforced), `wellfair/vault.rs`, the WebTorrent/seeder layer (the commons replication), and
//! `qualia-core-db::crypto::zk_proofs` (coordinate). This is the shape the wiring must honour.

use serde::{Deserialize, Serialize};

/// A commitment to a payload — a 32-byte hash/commitment (e.g. BLAKE3/SHA-256, computed by the crypto
/// layer). It **survives revocation** and binds a [`ConductRecord`] to *what* was acted on, without holding
/// or re-exposing the payload itself. It is also the **content address** of the [`EncryptedCommonsPayload`].
pub type PayloadCommitment = [u8; 32];

/// An **encrypted payload in a permissive commons** — content-addressed ciphertext that **many parties may
/// store (replicate)** so it **cannot be deleted** (anti-erasure), but that is **accessible only to holders
/// of the right credential** (the wrapped decryption key). Wide storage ≠ wide access: without a key the
/// ciphertext is opaque.
///
/// Revocation acts on **access** (the credential's key — see [`ConsentCredential::revoke`]), *not* on this
/// blob; the blob persists for other credential-holders and as un-erasable evidence. The person's ultimate
/// control is **crypto-shredding** — once no credential can decrypt it (all keys destroyed), it is
/// permanently unreadable by anyone, effective erasure though the bytes survive (see
/// [`is_crypto_shredded`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedCommonsPayload {
    /// Content address / commitment of the ciphertext — its durable identifier (equals the
    /// [`PayloadCommitment`] a credential and conduct record bind to).
    pub commitment: PayloadCommitment,
    /// The envelope-encrypted bytes. Opaque without a credential; safe to replicate widely.
    pub ciphertext: Vec<u8>,
    /// The parties (by identifier) who hold a copy — the commons. Replication = durability: no single
    /// party's deletion destroys the payload while another copy exists.
    pub storers: Vec<String>,
}

impl EncryptedCommonsPayload {
    pub fn new(commitment: PayloadCommitment, ciphertext: Vec<u8>, storers: Vec<String>) -> Self {
        Self { commitment, ciphertext, storers }
    }

    /// Number of independent copies held — the anti-deletion / durability measure.
    pub fn replication(&self) -> usize {
        self.storers.len()
    }

    /// Durable against unilateral deletion (held by more than one storer). A single-copy payload is *not*
    /// yet in a resilient commons.
    pub fn is_durable(&self) -> bool {
        self.storers.len() > 1
    }

    /// Add a storer (a party replicates a copy — increases durability). Idempotent.
    pub fn add_storer(&mut self, did: impl Into<String>) {
        let did = did.into();
        if !self.storers.iter().any(|s| s == &did) {
            self.storers.push(did);
        }
    }

    /// One storer drops their copy. Returns `true` if copies **remain** (the payload survives) — the point
    /// of the commons: unilateral deletion does not erase what others hold.
    pub fn drop_storer(&mut self, did: &str) -> bool {
        self.storers.retain(|s| s != did);
        !self.storers.is_empty()
    }
}

/// **Crypto-shredding check.** Is this commons payload *effectively erased* — permanently unreadable — for a
/// given set of credentials at `now`? True iff **no** credential grants a live key to it (every credential
/// for this commitment is revoked/expired, or none exists). The bytes may still be replicated across the
/// commons, but with no key anywhere they cannot be decrypted by anyone — the person's ultimate erasure
/// control, achieved by destroying keys rather than by chasing copies.
pub fn is_crypto_shredded(
    payload: &EncryptedCommonsPayload,
    credentials: &[ConsentCredential],
    now_unix: u64,
) -> bool {
    !credentials
        .iter()
        .any(|c| c.payload_commitment == payload.commitment && c.payload_key(now_unix).is_some())
}

/// Where a credential's authority derives from — the *basis* on which access is granted. Not always
/// self-consent: a **court** can hold one (to support proceedings), as can another attested **authority**.
/// Authority-issued credentials are legitimate — and, when multi-sig, still cannot be exercised unilaterally.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialAuthority {
    /// The data-subject's own consent.
    #[default]
    Subject,
    /// A court / judicial order — supports court-of-law access (audit / proceedings).
    Court { order_ref: String },
    /// Another attested authoritative agent (a statutory body, a guardian, …), by its instrument.
    Authority { authority_did: String, instrument_ref: String },
}

/// A participating party in a multi-signature authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Party {
    pub did: String,
    /// The party's role in the authorization (e.g. `"subject"`, `"guardian"`, `"advocate"`, `"court"`).
    pub role: String,
}

/// How a credential may be **exercised** (acted on the payload).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Authorization {
    /// The holder may act alone.
    #[default]
    Single,
    /// **Multi-signature.** An exercise must be (a) **instigated by one of the participating parties** — so
    /// no outside/authority actor can act unilaterally — AND (b) signed by at least `threshold` of the
    /// `parties`. The "unable to act without instigation of one of the participating parties" rule.
    MultiSig { parties: Vec<Party>, threshold: usize },
}

/// A request to exercise a credential: **who instigated it**, and the party signatures collected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExerciseRequest {
    pub instigator_did: String,
    /// DIDs of parties who have signed off on this exercise.
    pub signatures: Vec<String>,
}

impl ExerciseRequest {
    pub fn new(instigator_did: impl Into<String>, signatures: Vec<String>) -> Self {
        Self { instigator_did: instigator_did.into(), signatures }
    }
}

impl Authorization {
    /// Is this exercise authorised? `Single` → always. `MultiSig` → the instigator must be a participating
    /// party (no unilateral outside/authority action) AND at least `threshold` **distinct** participating
    /// parties must have signed.
    pub fn authorizes(&self, req: &ExerciseRequest) -> bool {
        match self {
            Authorization::Single => true,
            Authorization::MultiSig { parties, threshold } => {
                let is_party = |did: &str| parties.iter().any(|p| p.did == did);
                if !is_party(&req.instigator_did) {
                    return false; // must be instigated by a participating party
                }
                let signers: std::collections::BTreeSet<&str> = req
                    .signatures
                    .iter()
                    .map(|s| s.as_str())
                    .filter(|s| is_party(s))
                    .collect();
                signers.len() >= *threshold
            }
        }
    }
}

/// A **consent credential** — grants an agent scoped access to an encrypted payload; revocable, with the
/// revocation *crypto-enforced* (the wrapped key is destroyed). Its authority may be self-consent, a court,
/// or another authority ([`CredentialAuthority`]); its exercise may require multi-sig
/// ([`Authorization`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentCredential {
    pub id: String,
    /// The person granting consent (the data-subject).
    pub subject_did: String,
    /// The granted agent (e.g. the social worker — a human, by identifier).
    pub agent_did: String,
    /// What is granted, purpose-bound (minimal-disclosure scope).
    pub scope: String,
    pub purpose: String,
    /// A commitment to the granted payload — durable, binds the conduct trail.
    pub payload_commitment: PayloadCommitment,
    pub granted_unix: u64,
    /// Optional expiry — access ceases at/after this time even without an explicit revoke.
    pub expiry_unix: Option<u64>,
    /// Set on revoke — the moment after which the payload is unavailable.
    pub revoked_unix: Option<u64>,
    /// The **wrapped data key** the agent needs to decrypt the payload — present while access is permitted,
    /// **destroyed (`None`) on revoke/expiry**. This is the crypto-revocation: no key ⇒ no payload. Private
    /// so it can only be read through [`payload_key`](ConsentCredential::payload_key), which enforces the
    /// active check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    wrapped_key: Option<Vec<u8>>,
    /// Where this credential's authority derives from (subject / court / authority).
    #[serde(default)]
    pub authority: CredentialAuthority,
    /// How it may be exercised (alone, or multi-sig requiring party instigation + threshold).
    #[serde(default)]
    pub authorization: Authorization,
}

impl ConsentCredential {
    /// Grant a credential. `wrapped_key` is the data key wrapped for the agent (from the vault's envelope
    /// encryption) — the thing revocation destroys.
    #[allow(clippy::too_many_arguments)]
    pub fn grant(
        id: impl Into<String>,
        subject_did: impl Into<String>,
        agent_did: impl Into<String>,
        scope: impl Into<String>,
        purpose: impl Into<String>,
        payload_commitment: PayloadCommitment,
        wrapped_key: Vec<u8>,
        granted_unix: u64,
        expiry_unix: Option<u64>,
    ) -> Self {
        Self {
            id: id.into(),
            subject_did: subject_did.into(),
            agent_did: agent_did.into(),
            scope: scope.into(),
            purpose: purpose.into(),
            payload_commitment,
            granted_unix,
            expiry_unix,
            revoked_unix: None,
            wrapped_key: Some(wrapped_key),
            authority: CredentialAuthority::Subject,
            authorization: Authorization::Single,
        }
    }

    /// Set the credential's authority basis (a court order / another authority). Builder-style.
    pub fn with_authority(mut self, authority: CredentialAuthority) -> Self {
        self.authority = authority;
        self
    }

    /// Require **multi-sig** exercise: instigation by a participating party + `threshold` party signatures.
    /// Builder-style. This is what makes the credential *unable to act without a participating party*.
    pub fn requiring_multisig(mut self, parties: Vec<Party>, threshold: usize) -> Self {
        self.authorization = Authorization::MultiSig { parties, threshold };
        self
    }

    /// Whether the credential is currently active (not revoked, not past expiry).
    pub fn is_active(&self, now_unix: u64) -> bool {
        if self.revoked_unix.is_some() {
            return false;
        }
        match self.expiry_unix {
            Some(exp) => now_unix < exp,
            None => true,
        }
    }

    /// **Revoke** the credential — *crypto-enforced*: records the moment **and destroys the wrapped key**,
    /// so the payload can no longer be decrypted (it returns to the person). Idempotent.
    pub fn revoke(&mut self, now_unix: u64) {
        if self.revoked_unix.is_none() {
            self.revoked_unix = Some(now_unix);
        }
        self.wrapped_key = None; // the key is gone — no key, no payload
    }

    /// The wrapped data key **iff access is currently permitted** — `None` once revoked or expired. A `None`
    /// here *is* "the payload is unavailable to the agent": with no key there is nothing to decrypt with.
    pub fn payload_key(&self, now_unix: u64) -> Option<&[u8]> {
        if self.is_active(now_unix) {
            self.wrapped_key.as_deref()
        } else {
            None
        }
    }

    /// Whether the encrypted payload is *technically* accessible right now (active + key present) —
    /// **ignoring** any multi-sig requirement. For a multi-sig credential use [`can_exercise`] /
    /// [`exercise`], which enforce party-instigation + threshold.
    ///
    /// [`can_exercise`]: ConsentCredential::can_exercise
    /// [`exercise`]: ConsentCredential::exercise
    pub fn payload_accessible(&self, now_unix: u64) -> bool {
        self.payload_key(now_unix).is_some()
    }

    /// Whether a specific exercise is permitted: the credential is active **and** the request satisfies the
    /// [`Authorization`] (for multi-sig: instigated by a participating party + threshold signatures). This is
    /// the gate that stops an authority acting unilaterally.
    pub fn can_exercise(&self, req: &ExerciseRequest, now_unix: u64) -> bool {
        self.is_active(now_unix) && self.authorization.authorizes(req)
    }

    /// The wrapped data key **iff this exercise is authorised** (active + authorization satisfied). The
    /// authorization-aware counterpart to [`payload_key`](ConsentCredential::payload_key): a multi-sig
    /// credential yields a key only when a participating party instigated it and the threshold has signed.
    pub fn exercise(&self, req: &ExerciseRequest, now_unix: u64) -> Option<&[u8]> {
        if self.can_exercise(req, now_unix) {
            self.wrapped_key.as_deref()
        } else {
            None
        }
    }
}

/// The cryptographic **attestation** on a [`ConductRecord`] — attributable + court-auditable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Attestation {
    /// A detached signature over the record (ed25519 / ML-DSA) — binds the agent to the conduct. Hex.
    Signature { alg: String, sig_hex: String },
    /// A **zero-knowledge proof** over `crypto/zk_proofs` — proves a property (e.g. "the agent held a valid
    /// consent credential for the payload committed to by `payload_commitment` at `time`") **without
    /// revealing the payload**. Referenced by id; the proof + verifying key live in the ZK layer.
    ZkProof { proof_id: String },
}

/// A **durable** record of *how and why* an agent acted — the conduct trail. It **persists after the
/// consent credential is revoked and the payload is gone** (revoking consent does not erase accountability),
/// and it binds to the payload **commitment** (not the payload), so it proves the agent acted on a specific
/// datum **without** retaining or re-exposing that datum. Append-only in practice (the store is
/// tamper-evident — signed WAL); this type is the record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConductRecord {
    pub id: String,
    /// The agent who acted (a human worker — accountable).
    pub agent_did: String,
    /// The consent credential the action was taken under (links act → authority).
    pub credential_id: String,
    /// What the agent did (accessed / decided / referred / requested / escalated / **omitted**).
    pub action: String,
    /// Why — the stated basis/authority for the act.
    pub reason: String,
    pub time_unix: u64,
    /// Binds the record to *what* was acted on — the payload commitment (durable; the payload is not held).
    pub payload_commitment: PayloadCommitment,
    /// The attestation making this court-auditable and attributable.
    pub attestation: Attestation,
}

impl ConductRecord {
    /// Does this conduct record concern the payload committed to by `commitment`? (Audit link — verify the
    /// agent acted on a specific datum by its commitment, without needing the datum.)
    pub fn concerns_commitment(&self, commitment: &PayloadCommitment) -> bool {
        &self.payload_commitment == commitment
    }
}

/// Filter a conduct trail to the records taken under one consent credential — the **audit view**. These are
/// exactly the records that survive that credential's revocation (the accountability the person cannot erase
/// and the worker cannot withhold).
pub fn audit_trail_for_credential<'a>(
    records: &'a [ConductRecord],
    credential_id: &str,
) -> Vec<&'a ConductRecord> {
    records.iter().filter(|r| r.credential_id == credential_id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const C: PayloadCommitment = [7u8; 32];

    fn cred() -> ConsentCredential {
        ConsentCredential::grant(
            "cc-1",
            "did:wf:person",
            "did:wf:social-worker",
            "housing-support-case",
            "assess and arrange support",
            C,
            b"wrapped-data-key".to_vec(),
            1_000,
            Some(2_000),
        )
    }

    fn conduct(id: &str, cred_id: &str, action: &str) -> ConductRecord {
        ConductRecord {
            id: id.into(),
            agent_did: "did:wf:social-worker".into(),
            credential_id: cred_id.into(),
            action: action.into(),
            reason: "acting under the granted consent".into(),
            time_unix: 1_100,
            payload_commitment: C,
            attestation: Attestation::Signature {
                alg: "ed25519".into(),
                sig_hex: "deadbeef".into(),
            },
        }
    }

    #[test]
    fn granted_payload_is_accessible_until_revoked() {
        let c = cred();
        assert!(c.payload_accessible(1_100), "active grant → payload accessible");
        assert!(c.payload_key(1_100).is_some());
    }

    #[test]
    fn revocation_is_crypto_enforced_the_key_is_destroyed_and_payload_gone() {
        let mut c = cred();
        assert!(c.payload_accessible(1_100));
        c.revoke(1_200);
        // No key, no payload — not a flag, the wrapped key is gone.
        assert!(!c.payload_accessible(1_300), "revoked → payload unavailable");
        assert!(c.payload_key(1_300).is_none(), "the wrapped key is destroyed");
        assert_eq!(c.revoked_unix, Some(1_200));
        // Even serialized, the key does not travel (it's gone / skipped).
        let json = serde_json::to_string(&c).unwrap();
        assert!(!json.contains("wrapped"), "no key material persists after revoke");
    }

    #[test]
    fn expiry_also_makes_the_payload_unavailable() {
        let c = cred(); // expires at 2_000
        assert!(c.payload_accessible(1_999));
        assert!(!c.payload_accessible(2_000), "past expiry → no access");
        assert!(!c.payload_accessible(2_001));
    }

    #[test]
    fn conduct_records_persist_after_revocation_and_stay_auditable() {
        // The person revokes; the payload is gone — but the worker's conduct trail remains.
        let mut c = cred();
        let trail = vec![
            conduct("k1", "cc-1", "accessed the housing record"),
            conduct("k2", "cc-1", "requested an emergency placement"),
            conduct("k3", "other-cred", "unrelated act"),
        ];
        c.revoke(1_500);
        assert!(!c.payload_accessible(1_600), "data gone");

        // The conduct trail for this credential is untouched by revocation — the accountability survives.
        let audit = audit_trail_for_credential(&trail, "cc-1");
        assert_eq!(audit.len(), 2, "both acts under cc-1 remain auditable after revoke");
        // Each binds to the payload commitment, proving WHAT was acted on without holding the payload.
        assert!(audit.iter().all(|r| r.concerns_commitment(&C)));
        // And carries a court-auditable attestation.
        assert!(audit.iter().all(|r| matches!(r.attestation, Attestation::Signature { .. })));
    }

    #[test]
    fn a_conduct_record_can_carry_a_zk_attestation() {
        // The ZK path: prove the agent held a valid credential for the committed payload, without the payload.
        let r = ConductRecord {
            attestation: Attestation::ZkProof { proof_id: "zk:groth16:abc".into() },
            ..conduct("k", "cc-1", "acted")
        };
        assert!(matches!(r.attestation, Attestation::ZkProof { .. }));
        assert!(r.concerns_commitment(&C));
    }

    #[test]
    fn commons_payload_survives_unilateral_deletion_but_is_useless_without_a_key() {
        let mut p = EncryptedCommonsPayload::new(
            C,
            b"opaque-ciphertext".to_vec(),
            vec!["did:wf:person".into(), "did:wf:storer-a".into(), "did:wf:storer-b".into()],
        );
        assert!(p.is_durable(), "replicated across the commons");
        assert_eq!(p.replication(), 3);

        // A hostile party (or the agent) deleting THEIR copy does not erase the payload.
        assert!(p.drop_storer("did:wf:storer-a"), "copies remain after one deletion");
        assert_eq!(p.replication(), 2);
        assert!(p.drop_storer("did:wf:person"), "still survives");

        // Only when the last copy goes is the blob absent — the commons resists that.
        assert!(!p.drop_storer("did:wf:storer-b"), "no copies left");
        // The ciphertext, wherever held, is opaque — access is credential-gated, not storage-gated.
        assert!(!p.ciphertext.is_empty());
    }

    #[test]
    fn crypto_shredding_when_the_last_key_is_revoked_makes_it_unreadable_though_bytes_persist() {
        let payload = EncryptedCommonsPayload::new(
            C,
            b"ct".to_vec(),
            vec!["did:wf:person".into(), "did:wf:archive".into()],
        );
        let mut c = cred(); // the only credential granting a key to C
        // While the credential is live, the payload is not shredded (a key exists).
        assert!(!is_crypto_shredded(&payload, std::slice::from_ref(&c), 1_100));
        // Destroy the key (revoke) — no credential grants a key to C now → crypto-shredded.
        c.revoke(1_200);
        assert!(
            is_crypto_shredded(&payload, std::slice::from_ref(&c), 1_300),
            "no key anywhere → permanently unreadable, even though the commons bytes survive"
        );
        // The bytes are still replicated (not chased down) — erasure was by key-destruction.
        assert!(payload.is_durable());
    }

    fn party(did: &str, role: &str) -> Party {
        Party { did: did.into(), role: role.into() }
    }

    #[test]
    fn a_court_or_authority_can_hold_a_credential() {
        // A court credential (e.g. to support proceedings / the audit case).
        let c = cred().with_authority(CredentialAuthority::Court { order_ref: "order:2026-42".into() });
        assert!(matches!(c.authority, CredentialAuthority::Court { .. }));
        // Single authorization by default → the holder can access while active.
        assert!(c.payload_accessible(1_100));

        let c2 = cred().with_authority(CredentialAuthority::Authority {
            authority_did: "did:wf:child-protection".into(),
            instrument_ref: "mandate:7".into(),
        });
        assert!(matches!(c2.authority, CredentialAuthority::Authority { .. }));
    }

    #[test]
    fn multisig_requires_party_instigation_and_threshold_signatures() {
        let parties = vec![
            party("did:wf:person", "subject"),
            party("did:wf:advocate", "advocate"),
            party("did:wf:court", "court"),
        ];
        let c = cred().requiring_multisig(parties, 2);

        // Instigated by a participating party + 2 party signatures → authorised; exercise yields the key.
        let ok = ExerciseRequest::new("did:wf:person", vec!["did:wf:person".into(), "did:wf:advocate".into()]);
        assert!(c.can_exercise(&ok, 1_100));
        assert!(c.exercise(&ok, 1_100).is_some(), "authorised exercise yields the key");

        // Below threshold (only 1 party signature) → not authorised.
        let too_few = ExerciseRequest::new("did:wf:person", vec!["did:wf:person".into()]);
        assert!(!c.can_exercise(&too_few, 1_100));
        assert!(c.exercise(&too_few, 1_100).is_none());
    }

    #[test]
    fn an_authority_cannot_act_unilaterally_under_multisig() {
        // A court holds a multi-sig credential. It CANNOT exercise it without a participating party
        // instigating — this is "unable to act without instigation of one of the participating parties".
        let parties = vec![party("did:wf:person", "subject"), party("did:wf:guardian", "guardian")];
        let c = cred()
            .with_authority(CredentialAuthority::Court { order_ref: "order:9".into() })
            .requiring_multisig(parties, 1);

        // The court (NOT a participating party) tries to instigate alone → refused, even with a signature
        // it collected, because the instigator is not a participating party.
        let court_alone = ExerciseRequest::new("did:wf:court", vec!["did:wf:court".into()]);
        assert!(!c.can_exercise(&court_alone, 1_100), "an outside authority cannot act unilaterally");
        assert!(c.exercise(&court_alone, 1_100).is_none());

        // But once a participating party (the guardian) instigates and signs, the threshold is met.
        let party_instigated = ExerciseRequest::new("did:wf:guardian", vec!["did:wf:guardian".into()]);
        assert!(c.can_exercise(&party_instigated, 1_100), "a participating party's instigation authorises it");
    }

    #[test]
    fn revocation_defeats_even_an_authorised_multisig_exercise() {
        let parties = vec![party("did:wf:person", "subject"), party("did:wf:advocate", "advocate")];
        let mut c = cred().requiring_multisig(parties, 2);
        let req = ExerciseRequest::new("did:wf:person", vec!["did:wf:person".into(), "did:wf:advocate".into()]);
        assert!(c.exercise(&req, 1_100).is_some());
        c.revoke(1_200);
        // Even a fully-signed, party-instigated exercise gets no key once revoked — the key is gone.
        assert!(c.exercise(&req, 1_300).is_none(), "revocation (key destroyed) beats authorisation");
    }

    #[test]
    fn serde_round_trips() {
        let c = cred();
        let back: ConsentCredential = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(c, back);
        let r = conduct("k", "cc-1", "acted");
        let back_r: ConductRecord = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(r, back_r);
    }
}
