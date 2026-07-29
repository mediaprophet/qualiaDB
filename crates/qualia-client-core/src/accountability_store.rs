//! **Persistence for the accountability fabric** — the on-disk home for the tamper-evident
//! [`AccountabilityLedger`], the revocable [`ConsentCredential`]s, and the durable [`ConductRecord`]s
//! (ADR 0011 D2/D4/D5). It turns the tested domain models into something the desktop can actually use.
//!
//! Design: one small JSON sidecar file (`wellfair/accountability.json`), loaded/saved whole, matching the
//! sibling-store convention (`personal_profile::EmergencyContactStore`). A person's own accountability set is
//! small, and whole-file rewrite is the correct shape for a store whose records **mutate** (a credential is
//! *revoked* — the wrapped key destroyed in place — so append-only JSONL would not do).
//!
//! **Every accountability-relevant act is written into the signed hash-chained ledger**, not only the
//! convenience indexes: granting a credential logs a `"consent_granted"` entry, revoking logs
//! `"consent_revoked"`, and recording conduct logs a `"conduct"` entry carrying the record. So the ledger is
//! the authoritative tamper-evident spine (a betrayer cannot quietly drop the inconvenient act — [`verify`]
//! catches it), and [`AccountabilityState::credentials`] / [`AccountabilityState::conduct`] are fast views
//! over it.
//!
//! Scope: this is *tamper-evidence + local durability*. Anti-deletion **durability across parties** (so no one
//! can destroy the only copy) is the commons-replication layer (swarm/WebTorrent; coordinate), and the two
//! compose — replicate this file, and any pruned/rewritten copy is provably tampered. Real envelope
//! encryption of the wrapped key is the vault's job (deferred); here the wrapped key is carried as opaque
//! bytes, exactly as the model intends.
//!
//! [`verify`]: AccountabilityLedger::verify

use std::fs;
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::accountability_ledger::{AccountabilityLedger, LedgerEntry, LedgerTamper};
use crate::consent_credential::{
    audit_trail_for_credential, Attestation, ConductRecord, ConsentCredential,
    EncryptedCommonsPayload, PayloadCommitment,
};
use crate::dead_mans_switch::{DeadMansSwitch, Disposition, PartyAttestation};
use crate::disclosure_trace::{
    actors_with_access, disclosure_chain, trace_leak, DisclosureEvent, DisclosureFingerprint,
    TransparencyCc,
};
use crate::incapacity_switch::IncapacitySwitch;

/// Sidecar file, under the same `wellfair/` prefix as the other host stores.
pub const STORE_FILE: &str = "wellfair/accountability.json";

/// Ledger record kinds this store writes (the tamper-evident spine's vocabulary).
pub const KIND_CONSENT_GRANTED: &str = "consent_granted";
pub const KIND_CONSENT_REVOKED: &str = "consent_revoked";
pub const KIND_CONDUCT: &str = "conduct";
pub const KIND_DEAD_MANS_ARMED: &str = "dead_mans_armed";
pub const KIND_DEAD_MANS_ALIVE: &str = "dead_mans_alive";
pub const KIND_DEAD_MANS_ATTESTED: &str = "dead_mans_attested";
pub const KIND_DEAD_MANS_ENACTED: &str = "dead_mans_enacted";
pub const KIND_INCAPACITY_ARMED: &str = "incapacity_armed";
pub const KIND_INCAPACITY_ACTIVATED: &str = "incapacity_activated";
pub const KIND_INCAPACITY_REVERSED: &str = "incapacity_reversed";
pub const KIND_TRANSPARENCY_CC: &str = "transparency_cc";
pub const KIND_DISCLOSURE: &str = "disclosure";

/// A persisted dead-man switch together with the party attestations accumulated toward its trigger. The
/// [`DeadMansSwitch`] domain type carries no attestations (they are passed to `enact`); the store holds them
/// so the gamified trigger can accumulate across sessions (in the real model, on the friends' devices).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeadMansSwitchRecord {
    pub switch: DeadMansSwitch,
    #[serde(default)]
    pub attestations: Vec<PartyAttestation>,
}

/// The whole persisted accountability set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountabilityState {
    /// The authoritative, signed, hash-chained record of every act (the tamper-evidence spine).
    pub ledger: AccountabilityLedger,
    /// The consent credentials granted (mutated in place on revoke — the wrapped key is destroyed).
    pub credentials: Vec<ConsentCredential>,
    /// The durable conduct trail (survives credential revocation; also mirrored into the ledger).
    pub conduct: Vec<ConductRecord>,
    /// The **envelope-encrypted** commons payloads (ciphertext + content-address commitment), keyed by
    /// commitment via a credential's `payload_commitment`. Only ciphertext lives here — the DEK is sealed
    /// inside each credential's `wrapped_key`, never stored in the clear.
    #[serde(default)]
    pub payloads: Vec<EncryptedCommonsPayload>,
    /// Armed **dead-man switches** (post-death disposition), each with its accumulated attestations.
    #[serde(default)]
    pub dead_mans_switches: Vec<DeadMansSwitchRecord>,
    /// Armed **incapacity switches** (advocate activation on validated, reversible incapacity).
    #[serde(default)]
    pub incapacity_switches: Vec<IncapacitySwitch>,
    /// **Transparency cc's** — durable "I informed authority X on date Y" protective records.
    #[serde(default)]
    pub disclosure_ccs: Vec<TransparencyCc>,
    /// **Disclosure events** — the durable, attributable access/onward-share trail (who saw what, via whom).
    #[serde(default)]
    pub disclosure_events: Vec<DisclosureEvent>,
}

/// On-disk store for the accountability fabric.
pub struct AccountabilityStore {
    path: PathBuf,
}

impl AccountabilityStore {
    /// Open (or prepare to create) the store under `storage_root`.
    pub fn open(storage_root: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = storage_root.as_ref().join(STORE_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self { path })
    }

    /// Load the whole set (empty default if the file does not yet exist).
    pub fn load(&self) -> std::io::Result<AccountabilityState> {
        match fs::read(&self.path) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(|e| std::io::Error::other(e.to_string()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(AccountabilityState::default())
            }
            Err(e) => Err(e),
        }
    }

    /// Persist the whole set (write-to-temp then rename, so a crash can't leave a half-written chain).
    pub fn save(&self, state: &AccountabilityState) -> std::io::Result<()> {
        let bytes =
            serde_json::to_vec_pretty(state).map_err(|e| std::io::Error::other(e.to_string()))?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, &bytes)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Append a raw record to the tamper-evident ledger, signed by `signer`, and persist. Returns the entry.
    pub fn append_ledger(
        &self,
        kind: &str,
        payload_json: &str,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<LedgerEntry> {
        let mut state = self.load()?;
        let entry = state
            .ledger
            .append(kind, payload_json, signer, time_unix)
            .clone();
        self.save(&state)?;
        Ok(entry)
    }

    /// Verify the whole ledger chain. `Ok(None)` = intact; `Ok(Some(tamper))` = a detected, named tamper.
    pub fn verify_ledger(&self) -> std::io::Result<Result<(), LedgerTamper>> {
        Ok(self.load()?.ledger.verify())
    }

    /// The most-recent ledger entries (newest first), capped to `limit`.
    pub fn ledger_entries(&self, limit: usize) -> std::io::Result<Vec<LedgerEntry>> {
        let mut entries = self.load()?.ledger.entries().to_vec();
        entries.reverse();
        entries.truncate(limit);
        Ok(entries)
    }

    /// **Grant a consent credential** and log it into the ledger. The credential is stored; a
    /// `"consent_granted"` entry (subject → agent, scope, purpose) enters the signed chain.
    pub fn grant_credential(
        &self,
        cred: ConsentCredential,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<ConsentCredential> {
        let mut state = self.load()?;
        let note = serde_json::json!({
            "credential_id": cred.id,
            "subject": cred.subject_did,
            "agent": cred.agent_did,
            "scope": cred.scope,
            "purpose": cred.purpose,
            "commitment": hex::encode(cred.payload_commitment),
        })
        .to_string();
        state
            .ledger
            .append(KIND_CONSENT_GRANTED, &note, signer, time_unix);
        state.credentials.push(cred.clone());
        self.save(&state)?;
        Ok(cred)
    }

    /// **Revoke a consent credential** — crypto-enforced (the wrapped key is destroyed in
    /// [`ConsentCredential::revoke`]) — and log a `"consent_revoked"` entry. Returns `true` if a live
    /// credential was revoked. The credential row and every conduct record under it **persist**: revoking
    /// consent removes access, never accountability.
    pub fn revoke_credential(
        &self,
        credential_id: &str,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<bool> {
        let mut state = self.load()?;
        let Some(cred) = state.credentials.iter_mut().find(|c| c.id == credential_id) else {
            return Ok(false);
        };
        let was_active = cred.is_active(time_unix);
        cred.revoke(time_unix);
        let note = serde_json::json!({
            "credential_id": credential_id,
            "revoked_unix": time_unix,
        })
        .to_string();
        state
            .ledger
            .append(KIND_CONSENT_REVOKED, &note, signer, time_unix);
        self.save(&state)?;
        Ok(was_active)
    }

    /// All stored credentials (active and revoked — the revoked ones remain as the audit anchor).
    pub fn list_credentials(&self) -> std::io::Result<Vec<ConsentCredential>> {
        Ok(self.load()?.credentials)
    }

    /// The still-live **wrapped DEK** held by `agent_did`'s credential for `commitment` (e.g. the owner's own
    /// credential), so the DEK can be recovered and re-sealed on enactment. `None` if no such active credential.
    pub fn wrapped_key_for(
        &self,
        commitment: &PayloadCommitment,
        agent_did: &str,
        now_unix: u64,
    ) -> std::io::Result<Option<Vec<u8>>> {
        let state = self.load()?;
        Ok(state
            .credentials
            .iter()
            .find(|c| &c.payload_commitment == commitment && c.agent_did == agent_did)
            .and_then(|c| c.payload_key(now_unix).map(|k| k.to_vec())))
    }

    /// **Record an agent's conduct** under a credential — signed by `signer` (an
    /// [`Attestation::Signature`]) — into both the durable conduct trail and the tamper-evident ledger. The
    /// record binds to the payload **commitment** (not the payload), so it proves *what* was acted on without
    /// holding the datum, and it **survives** the credential's revocation.
    pub fn record_conduct(
        &self,
        agent_did: impl Into<String>,
        credential_id: impl Into<String>,
        action: impl Into<String>,
        reason: impl Into<String>,
        commitment: PayloadCommitment,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<ConductRecord> {
        let agent_did = agent_did.into();
        let credential_id = credential_id.into();
        let action = action.into();
        let reason = reason.into();

        // The signature is over the bound content — attributable + court-auditable.
        let bound = content_signing_bytes(
            &agent_did,
            &credential_id,
            &action,
            &reason,
            &commitment,
            time_unix,
        );
        let sig = signer.sign(&bound);
        let id = conduct_id(&agent_did, &credential_id, &action, time_unix);

        let record = ConductRecord {
            id,
            agent_did,
            credential_id,
            action,
            reason,
            time_unix,
            payload_commitment: commitment,
            attestation: Attestation::Signature {
                alg: "ed25519".into(),
                sig_hex: hex::encode(sig.to_bytes()),
            },
        };

        let mut state = self.load()?;
        let payload =
            serde_json::to_string(&record).map_err(|e| std::io::Error::other(e.to_string()))?;
        state
            .ledger
            .append(KIND_CONDUCT, &payload, signer, time_unix);
        state.conduct.push(record.clone());
        self.save(&state)?;
        Ok(record)
    }

    /// The **audit view** — every conduct record taken under one credential, in order. Exactly the records
    /// that survive that credential's revocation (the accountability the person cannot erase and the agent
    /// cannot withhold).
    pub fn audit_trail(&self, credential_id: &str) -> std::io::Result<Vec<ConductRecord>> {
        let state = self.load()?;
        Ok(audit_trail_for_credential(&state.conduct, credential_id)
            .into_iter()
            .cloned()
            .collect())
    }

    /// **Seal a plaintext payload and grant a consent credential over it** — the *real envelope-encryption*
    /// path (as opposed to [`grant_credential`], which takes an already-wrapped key). Generates a random DEK,
    /// AEAD-encrypts the plaintext, content-addresses the ciphertext, seals the DEK to `recipient_public`
    /// (the credential's real `wrapped_key`), stores the ciphertext in the commons, grants the credential,
    /// and logs `consent_granted`. Returns the granted credential. Nothing is stored in the clear: the
    /// plaintext becomes ciphertext, and the DEK survives only sealed inside the credential.
    ///
    /// [`grant_credential`]: AccountabilityStore::grant_credential
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::too_many_arguments)]
    pub fn seal_and_grant_credential(
        &self,
        credential_id: impl Into<String>,
        subject_did: impl Into<String>,
        agent_did: impl Into<String>,
        scope: impl Into<String>,
        purpose: impl Into<String>,
        plaintext: &[u8],
        recipient_public: &[u8; 32],
        storers: Vec<String>,
        expiry_unix: Option<u64>,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<ConsentCredential> {
        use crate::envelope_encryption::{seal_payload, wrap_dek_to};
        let (payload, dek) = seal_payload(plaintext, storers).map_err(std::io::Error::other)?;
        let wrapped = wrap_dek_to(recipient_public, &dek).map_err(std::io::Error::other)?;
        let commitment = payload.commitment;
        let cred = ConsentCredential::grant(
            credential_id,
            subject_did,
            agent_did,
            scope,
            purpose,
            commitment,
            wrapped,
            time_unix,
            expiry_unix,
        );

        let mut state = self.load()?;
        state.payloads.push(payload);
        let note = serde_json::json!({
            "credential_id": cred.id,
            "subject": cred.subject_did,
            "agent": cred.agent_did,
            "scope": cred.scope,
            "purpose": cred.purpose,
            "commitment": hex::encode(commitment),
            "sealed": true,
        })
        .to_string();
        state
            .ledger
            .append(KIND_CONSENT_GRANTED, &note, signer, time_unix);
        state.credentials.push(cred.clone());
        self.save(&state)?;
        Ok(cred)
    }

    /// **Open a sealed payload through a credential** — the end-to-end decrypt path. Reads the credential's
    /// `wrapped_key` (present only while active — revocation destroys it), unwraps the DEK with the
    /// recipient's X25519 secret, verifies the content-address commitment, and AEAD-decrypts. `Err` if the
    /// credential is unknown, revoked/expired (no key ⇒ payload unavailable), or its ciphertext is missing.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_payload_via_credential(
        &self,
        credential_id: &str,
        recipient_secret: &[u8; 32],
        now_unix: u64,
    ) -> std::io::Result<Vec<u8>> {
        use crate::envelope_encryption::open_payload_with_wrapped;
        let state = self.load()?;
        let cred = state
            .credentials
            .iter()
            .find(|c| c.id == credential_id)
            .ok_or_else(|| {
                std::io::Error::other(format!("credential '{credential_id}' not found"))
            })?;
        let wrapped = cred.payload_key(now_unix).ok_or_else(|| {
            std::io::Error::other(
                "credential revoked or expired — the wrapped key is destroyed, payload unavailable",
            )
        })?;
        let payload = state
            .payloads
            .iter()
            .find(|p| p.commitment == cred.payload_commitment)
            .ok_or_else(|| {
                std::io::Error::other("sealed payload for this credential not found in the commons")
            })?;
        open_payload_with_wrapped(payload, recipient_secret, wrapped).map_err(std::io::Error::other)
    }

    // --- Dead-man switch (post-death disposition; gamified + reversible) ---

    /// **Arm** a dead-man switch over a payload and log it. The owner sets the liveness grace + the gamified
    /// trigger (parties + threshold) + the disposition; it fires only when the heartbeat lapses AND a quorum
    /// of distinct parties attest.
    pub fn arm_dead_mans_switch(
        &self,
        switch: DeadMansSwitch,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<()> {
        let mut state = self.load()?;
        let note = serde_json::json!({
            "commitment": hex::encode(switch.payload_commitment),
            "threshold": switch.trigger.attestation_threshold,
            "parties": switch.trigger.parties,
        })
        .to_string();
        state
            .ledger
            .append(KIND_DEAD_MANS_ARMED, &note, signer, time_unix);
        // Replace any existing switch for the same commitment (re-arm) or push.
        if let Some(rec) = state
            .dead_mans_switches
            .iter_mut()
            .find(|r| r.switch.payload_commitment == switch.payload_commitment)
        {
            rec.switch = switch;
            rec.attestations.clear();
        } else {
            state.dead_mans_switches.push(DeadMansSwitchRecord {
                switch,
                attestations: Vec::new(),
            });
        }
        self.save(&state)?;
        Ok(())
    }

    /// **The principal is alive** — touch the heartbeat and un-fire a not-yet-enacted switch (the
    /// reversibility). Returns whether a switch for `commitment` was found.
    pub fn dead_mans_alive(
        &self,
        commitment: &PayloadCommitment,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<bool> {
        let mut state = self.load()?;
        let Some(rec) = state
            .dead_mans_switches
            .iter_mut()
            .find(|r| &r.switch.payload_commitment == commitment)
        else {
            return Ok(false);
        };
        rec.switch.principal_alive(time_unix);
        let note = serde_json::json!({ "commitment": hex::encode(commitment) }).to_string();
        state
            .ledger
            .append(KIND_DEAD_MANS_ALIVE, &note, signer, time_unix);
        self.save(&state)?;
        Ok(true)
    }

    /// Record a **party attestation** toward a switch's trigger (the friend-side accumulation). Returns
    /// whether the switch was found.
    pub fn attest_dead_mans(
        &self,
        commitment: &PayloadCommitment,
        attestation: PartyAttestation,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<bool> {
        let mut state = self.load()?;
        let Some(rec) = state
            .dead_mans_switches
            .iter_mut()
            .find(|r| &r.switch.payload_commitment == commitment)
        else {
            return Ok(false);
        };
        let note = serde_json::json!({
            "commitment": hex::encode(commitment),
            "party": attestation.party_did,
        })
        .to_string();
        // Keep the latest attestation per party.
        rec.attestations
            .retain(|a| a.party_did != attestation.party_did);
        rec.attestations.push(attestation);
        state
            .ledger
            .append(KIND_DEAD_MANS_ATTESTED, &note, signer, time_unix);
        self.save(&state)?;
        Ok(true)
    }

    /// **Enact** the switch if the gamified rule is satisfied (heartbeat lapsed + quorum attested). Records it
    /// fired, logs it, and returns the [`Disposition`] to carry out (key-release is a separate compose step).
    pub fn enact_dead_mans(
        &self,
        commitment: &PayloadCommitment,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<Option<Disposition>> {
        let mut state = self.load()?;
        let Some(rec) = state
            .dead_mans_switches
            .iter_mut()
            .find(|r| &r.switch.payload_commitment == commitment)
        else {
            return Ok(None);
        };
        let attestations = rec.attestations.clone();
        let disposition = rec.switch.enact(&attestations, time_unix).cloned();
        if disposition.is_some() {
            let note = serde_json::json!({ "commitment": hex::encode(commitment) }).to_string();
            state
                .ledger
                .append(KIND_DEAD_MANS_ENACTED, &note, signer, time_unix);
            self.save(&state)?;
        }
        Ok(disposition)
    }

    /// All armed dead-man switch records (with their accumulated attestations).
    pub fn list_dead_mans_switches(&self) -> std::io::Result<Vec<DeadMansSwitchRecord>> {
        Ok(self.load()?.dead_mans_switches)
    }

    /// **Enact a dead-man switch AND perform the key-release** — the composition that makes the disposition
    /// real. If the switch fires with [`Disposition::ReleaseTo`], the caller-supplied `dek` (recovered by
    /// unwrapping an owner credential) is **re-sealed to each disposition party's X25519 key** and a consent
    /// credential is granted to them, so they can now decrypt the payload they previously could not. Each grant
    /// is logged. `MakePublic` / `SelfDefinedRules` are returned but not key-released here (MakePublic's
    /// irreversibility is a deferred values decision). Returns the disposition (or `None` if not triggerable).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enact_dead_mans_release(
        &self,
        commitment: &PayloadCommitment,
        dek: &[u8; 32],
        party_keys: &[(String, [u8; 32])],
        subject_did: &str,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<Option<Disposition>> {
        let mut state = self.load()?;
        let disp = Self::enact_and_release_in_state(
            &mut state,
            commitment,
            dek,
            party_keys,
            subject_did,
            signer,
            time_unix,
        )
        .map_err(std::io::Error::other)?;
        if disp.is_some() {
            self.save(&state)?;
        }
        Ok(disp)
    }

    /// **Social-recovery enactment (no owner key):** reconstruct the payload DEK from a quorum of friends'
    /// Shamir shares, then enact + release to the disposition parties. This is the true post-death / incapacity
    /// path — a quorum of chosen trustees recovers the key **without the owner**, closing the gap
    /// [`enact_dead_mans_release`](Self::enact_dead_mans_release) left (which needed the owner's derived key).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reconstruct_and_release(
        &self,
        commitment: &PayloadCommitment,
        recovery_shares: &[crate::shamir_recovery::Share],
        party_keys: &[(String, [u8; 32])],
        subject_did: &str,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<Option<Disposition>> {
        let dek_vec =
            crate::shamir_recovery::reconstruct(recovery_shares).map_err(std::io::Error::other)?;
        let dek: [u8; 32] = dek_vec
            .as_slice()
            .try_into()
            .map_err(|_| std::io::Error::other("reconstructed DEK is not 32 bytes"))?;
        let mut state = self.load()?;
        let disp = Self::enact_and_release_in_state(
            &mut state,
            commitment,
            &dek,
            party_keys,
            subject_did,
            signer,
            time_unix,
        )
        .map_err(std::io::Error::other)?;
        if disp.is_some() {
            self.save(&state)?;
        }
        Ok(disp)
    }

    /// Shared enact-and-release logic operating on a loaded state (no IO): enacts the switch for `commitment`,
    /// logs it, and — for a `ReleaseTo` disposition — seals `dek` to each supplied party key and grants a
    /// credential. Returns the disposition, or `None` if not triggerable.
    #[cfg(not(target_arch = "wasm32"))]
    fn enact_and_release_in_state(
        state: &mut AccountabilityState,
        commitment: &PayloadCommitment,
        dek: &[u8; 32],
        party_keys: &[(String, [u8; 32])],
        subject_did: &str,
        signer: &SigningKey,
        time_unix: u64,
    ) -> Result<Option<Disposition>, String> {
        use crate::envelope_encryption::wrap_dek_to;
        let Some(rec) = state
            .dead_mans_switches
            .iter_mut()
            .find(|r| &r.switch.payload_commitment == commitment)
        else {
            return Ok(None);
        };
        let attestations = rec.attestations.clone();
        let Some(disposition) = rec.switch.enact(&attestations, time_unix).cloned() else {
            return Ok(None);
        };
        let enote = serde_json::json!({ "commitment": hex::encode(commitment), "released": true })
            .to_string();
        state
            .ledger
            .append(KIND_DEAD_MANS_ENACTED, &enote, signer, time_unix);

        if let Disposition::ReleaseTo { parties } = &disposition {
            for did in parties.clone() {
                let Some((_, pk)) = party_keys.iter().find(|(d, _)| d == &did) else {
                    continue; // no key for this party yet (needs remote-key distribution)
                };
                let wrapped = wrap_dek_to(pk, dek)?;
                let digest = Sha256::digest(format!("release:{did}:{time_unix}").as_bytes());
                let id = format!("cc-{}", hex::encode(&digest[..6]));
                let cred = ConsentCredential::grant(
                    id.clone(),
                    subject_did,
                    &did,
                    "dead-man-release",
                    "post-death disposition",
                    *commitment,
                    wrapped,
                    time_unix,
                    None,
                );
                let gnote = serde_json::json!({
                    "credential_id": id,
                    "agent": did,
                    "commitment": hex::encode(commitment),
                    "via": "dead_mans_release",
                })
                .to_string();
                state
                    .ledger
                    .append(KIND_CONSENT_GRANTED, &gnote, signer, time_unix);
                state.credentials.push(cred);
            }
        }
        Ok(Some(disposition))
    }

    // --- Incapacity switch (advocate activation on validated, reversible incapacity) ---

    /// **Arm** an incapacity switch and log it. Replaces any existing switch for the same principal.
    pub fn arm_incapacity_switch(
        &self,
        switch: IncapacitySwitch,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<()> {
        let mut state = self.load()?;
        let note = serde_json::json!({
            "principal": switch.principal_did,
            "advocate": switch.advocate_did,
            "threshold": switch.trigger.attestation_threshold,
        })
        .to_string();
        state
            .ledger
            .append(KIND_INCAPACITY_ARMED, &note, signer, time_unix);
        if let Some(existing) = state
            .incapacity_switches
            .iter_mut()
            .find(|s| s.principal_did == switch.principal_did)
        {
            *existing = switch;
        } else {
            state.incapacity_switches.push(switch);
        }
        self.save(&state)?;
        Ok(())
    }

    /// **Activate** advocacy if the corroborated trigger is satisfied (quorum + optional official instrument).
    /// Returns whether it activated.
    pub fn activate_incapacity(
        &self,
        principal_did: &str,
        attesting_parties: &[String],
        official_instrument: Option<&str>,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<bool> {
        let mut state = self.load()?;
        let Some(switch) = state
            .incapacity_switches
            .iter_mut()
            .find(|s| s.principal_did == principal_did)
        else {
            return Ok(false);
        };
        let activated = switch.activate(attesting_parties, official_instrument, time_unix);
        if activated {
            let note = serde_json::json!({
                "principal": principal_did,
                "official_instrument": official_instrument.is_some(),
            })
            .to_string();
            state
                .ledger
                .append(KIND_INCAPACITY_ACTIVATED, &note, signer, time_unix);
            self.save(&state)?;
        }
        Ok(activated)
    }

    /// **Regain capacity** — the advocate stands down, control reverts to the principal (the reversibility).
    /// Returns whether a switch for the principal was found.
    pub fn regain_capacity(
        &self,
        principal_did: &str,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<bool> {
        let mut state = self.load()?;
        let Some(switch) = state
            .incapacity_switches
            .iter_mut()
            .find(|s| s.principal_did == principal_did)
        else {
            return Ok(false);
        };
        switch.regain_capacity(time_unix);
        let note = serde_json::json!({ "principal": principal_did }).to_string();
        state
            .ledger
            .append(KIND_INCAPACITY_REVERSED, &note, signer, time_unix);
        self.save(&state)?;
        Ok(true)
    }

    /// All armed incapacity switches.
    pub fn list_incapacity_switches(&self) -> std::io::Result<Vec<IncapacitySwitch>> {
        Ok(self.load()?.incapacity_switches)
    }

    // --- Disclosure traceability (ADR 0011 D5): a betrayal is knowable + attributable ---

    /// Record a **transparency cc** — the protective "I informed authority X on date Y for purpose Z" note —
    /// and log it. Durable: if the authority later betrays or fails to act, that is knowable against this.
    pub fn record_transparency_cc(
        &self,
        cc: TransparencyCc,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<()> {
        let mut state = self.load()?;
        let note = serde_json::json!({
            "authority": cc.informed_authority_did,
            "credential_id": cc.credential_id,
            "purpose": cc.purpose,
        })
        .to_string();
        state
            .ledger
            .append(KIND_TRANSPARENCY_CC, &note, signer, time_unix);
        state.disclosure_ccs.push(cc);
        self.save(&state)?;
        Ok(())
    }

    /// Record a **disclosure event** (an access or onward-share) and log it — the attributable trail. The
    /// event's `accountable_actor` (a staffer if a delegate acted, else the recipient) is what a traced leak
    /// points to.
    pub fn record_disclosure_event(
        &self,
        event: DisclosureEvent,
        signer: &SigningKey,
        time_unix: u64,
    ) -> std::io::Result<()> {
        let mut state = self.load()?;
        let note = serde_json::json!({
            "commitment": hex::encode(event.payload_commitment),
            "recipient": event.recipient_did,
            "actor": event.accountable_actor(),
            "id": event.id,
        })
        .to_string();
        state
            .ledger
            .append(KIND_DISCLOSURE, &note, signer, time_unix);
        state.disclosure_events.push(event);
        self.save(&state)?;
        Ok(())
    }

    /// The full disclosure chain for a payload — who saw it, via which route, in order.
    pub fn disclosure_chain(
        &self,
        commitment: &PayloadCommitment,
    ) -> std::io::Result<Vec<DisclosureEvent>> {
        let state = self.load()?;
        Ok(disclosure_chain(&state.disclosure_events, commitment)
            .into_iter()
            .cloned()
            .collect())
    }

    /// The distinct actors who had access to a payload — the set a leak **must** be within.
    pub fn actors_with_access(
        &self,
        commitment: &PayloadCommitment,
    ) -> std::io::Result<Vec<String>> {
        let state = self.load()?;
        Ok(actors_with_access(&state.disclosure_events, commitment)
            .into_iter()
            .map(|s| s.to_string())
            .collect())
    }

    /// **Trace a leak** by its per-recipient fingerprint → the disclosure it came from (and thence the
    /// accountable actor). Returns the matching event, if any.
    pub fn trace_leak(
        &self,
        fingerprint: &DisclosureFingerprint,
    ) -> std::io::Result<Option<DisclosureEvent>> {
        let state = self.load()?;
        Ok(trace_leak(&state.disclosure_events, fingerprint).cloned())
    }

    /// All transparency cc records.
    pub fn list_transparency_ccs(&self) -> std::io::Result<Vec<TransparencyCc>> {
        Ok(self.load()?.disclosure_ccs)
    }
}

/// The bytes an [`Attestation::Signature`] on a conduct record signs — the record's bound content.
fn content_signing_bytes(
    agent_did: &str,
    credential_id: &str,
    action: &str,
    reason: &str,
    commitment: &PayloadCommitment,
    time_unix: u64,
) -> [u8; 32] {
    let mut h = Sha256::new();
    for part in [agent_did, credential_id, action, reason] {
        h.update(part.as_bytes());
        h.update(b"\x1f");
    }
    h.update(commitment);
    h.update(b"\x1f");
    h.update(time_unix.to_le_bytes());
    h.finalize().into()
}

fn conduct_id(agent_did: &str, credential_id: &str, action: &str, time_unix: u64) -> String {
    let digest =
        Sha256::digest(format!("{agent_did}:{credential_id}:{action}:{time_unix}").as_bytes());
    format!("cd-{}", hex::encode(&digest[..6]))
}

/// Parse a 32-byte hex commitment (helper for the host/command boundary). `Err` on wrong length / non-hex.
pub fn parse_commitment_hex(s: &str) -> Result<PayloadCommitment, String> {
    let bytes = hex::decode(s.trim()).map_err(|e| format!("commitment not hex: {e}"))?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| format!("commitment must be 32 bytes, got {}", bytes.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signer(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn cred(id: &str, commitment: PayloadCommitment) -> ConsentCredential {
        ConsentCredential::grant(
            id,
            "did:wf:person",
            "did:wf:social-worker",
            "housing-support",
            "assess and arrange support",
            commitment,
            b"wrapped-key".to_vec(),
            1_000,
            None,
        )
    }

    #[test]
    fn grant_then_list_persists_and_logs_to_ledger() {
        let dir = tempfile::tempdir().unwrap();
        let store = AccountabilityStore::open(dir.path()).unwrap();
        let sk = signer(1);
        store
            .grant_credential(cred("cc-1", [7u8; 32]), &sk, 1_000)
            .unwrap();

        // Persisted across a fresh open.
        let store2 = AccountabilityStore::open(dir.path()).unwrap();
        let creds = store2.list_credentials().unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].id, "cc-1");
        // The grant is in the signed chain, and the chain verifies.
        assert_eq!(store2.verify_ledger().unwrap(), Ok(()));
        assert_eq!(
            store2
                .load()
                .unwrap()
                .ledger
                .of_kind(KIND_CONSENT_GRANTED)
                .len(),
            1
        );
    }

    #[test]
    fn conduct_survives_revocation_and_stays_auditable() {
        let dir = tempfile::tempdir().unwrap();
        let store = AccountabilityStore::open(dir.path()).unwrap();
        let sk = signer(2);
        let commitment = [9u8; 32];
        store
            .grant_credential(cred("cc-9", commitment), &sk, 1_000)
            .unwrap();

        store
            .record_conduct(
                "did:wf:social-worker",
                "cc-9",
                "accessed housing record",
                "under consent",
                commitment,
                &sk,
                1_100,
            )
            .unwrap();
        store
            .record_conduct(
                "did:wf:social-worker",
                "cc-9",
                "requested placement",
                "under consent",
                commitment,
                &sk,
                1_150,
            )
            .unwrap();

        // The person revokes — access ends (key destroyed), but the conduct trail remains.
        assert!(store.revoke_credential("cc-9", &sk, 1_200).unwrap());
        let creds = store.list_credentials().unwrap();
        assert!(
            !creds[0].payload_accessible(1_300),
            "revoked → payload unavailable"
        );

        let trail = store.audit_trail("cc-9").unwrap();
        assert_eq!(trail.len(), 2, "both acts survive revocation");
        assert!(trail.iter().all(|r| r.concerns_commitment(&commitment)));
        assert!(trail
            .iter()
            .all(|r| matches!(r.attestation, Attestation::Signature { .. })));

        // Whole chain still verifies: grant + 2 conduct + revoke.
        assert_eq!(store.verify_ledger().unwrap(), Ok(()));
        assert_eq!(store.load().unwrap().ledger.len(), 4);
    }

    #[test]
    fn a_dropped_ledger_entry_is_detected_after_reload() {
        let dir = tempfile::tempdir().unwrap();
        let store = AccountabilityStore::open(dir.path()).unwrap();
        let sk = signer(3);
        store.append_ledger("conduct", "a", &sk, 1_000).unwrap();
        store.append_ledger("conduct", "b", &sk, 1_100).unwrap();
        store.append_ledger("conduct", "c", &sk, 1_200).unwrap();

        // A betrayer edits the file to remove the middle act, then we reload.
        let mut state = store.load().unwrap();
        state.ledger = {
            // reconstruct a ledger missing entry 1 by round-tripping through its serialized form
            let mut kept: Vec<_> = state.ledger.entries().to_vec();
            kept.remove(1);
            serde_json::from_value(serde_json::json!({ "entries": kept })).unwrap()
        };
        store.save(&state).unwrap();

        assert!(
            matches!(store.verify_ledger().unwrap(), Err(_)),
            "deletion is detectable on reload"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn seal_grant_open_then_revoke_denies_but_bytes_survive() {
        use crate::envelope_encryption::EnvelopeKeypair;
        let dir = tempfile::tempdir().unwrap();
        let store = AccountabilityStore::open(dir.path()).unwrap();
        let sk = signer(5);
        let agent = EnvelopeKeypair::generate().unwrap();

        // Seal a real record and grant the agent a credential over it.
        let cred = store
            .seal_and_grant_credential(
                "cc-seal",
                "did:wf:person",
                "did:wf:social-worker",
                "housing-support",
                "assess and arrange support",
                b"the sensitive housing record",
                &agent.public,
                vec!["did:wf:person".into(), "did:wf:archive".into()],
                None,
                &sk,
                1_000,
            )
            .unwrap();

        // The agent decrypts the real ciphertext through the credential.
        let opened = store
            .open_payload_via_credential(&cred.id, &agent.secret, 1_100)
            .unwrap();
        assert_eq!(opened, b"the sensitive housing record");

        // Nothing is stored in the clear: the persisted payload is ciphertext, not the plaintext.
        let st = store.load().unwrap();
        assert_eq!(st.payloads.len(), 1);
        assert_ne!(
            st.payloads[0].ciphertext.as_slice(),
            b"the sensitive housing record"
        );

        // Revoke — the wrapped key is destroyed; opening now fails (no key, no payload)…
        assert!(store.revoke_credential(&cred.id, &sk, 1_200).unwrap());
        assert!(store
            .open_payload_via_credential(&cred.id, &agent.secret, 1_300)
            .is_err());
        // …though the commons ciphertext survives (revocation is access, not deletion).
        let st = store.load().unwrap();
        assert_eq!(st.payloads.len(), 1);
        assert!(st.payloads[0].is_durable());
        // The whole ledger (grant + revoke) still verifies.
        assert_eq!(store.verify_ledger().unwrap(), Ok(()));
    }

    #[test]
    fn dead_mans_switch_gamified_trigger_and_reversibility() {
        use crate::dead_mans_switch::{
            AttestationKind, DeadMansSwitch, Disposition, Heartbeat, PartyAttestation, TriggerRule,
        };
        let dir = tempfile::tempdir().unwrap();
        let store = AccountabilityStore::open(dir.path()).unwrap();
        let sk = signer(6);
        let c = [3u8; 32];
        let att = |who: &str, t: u64| PartyAttestation {
            party_did: who.into(),
            kind: AttestationKind::BelievedDead,
            time_unix: t,
        };
        let sw = DeadMansSwitch {
            payload_commitment: c,
            heartbeat: Heartbeat::new(1_000, 100),
            trigger: TriggerRule {
                require_heartbeat_lapsed: true,
                attestation_threshold: 2,
                parties: vec!["a".into(), "b".into()],
            },
            disposition: Disposition::ReleaseTo {
                parties: vec!["trustee".into()],
            },
            fired_unix: None,
        };
        store.arm_dead_mans_switch(sw, &sk, 1_000).unwrap();

        // One party alone can't fire it (gamified — quorum required).
        store
            .attest_dead_mans(&c, att("a", 1_200), &sk, 1_200)
            .unwrap();
        assert!(store.enact_dead_mans(&c, &sk, 1_200).unwrap().is_none());
        // Two distinct parties + lapsed heartbeat → triggerable, but the principal showing up resets it.
        store
            .attest_dead_mans(&c, att("b", 1_200), &sk, 1_200)
            .unwrap();
        assert!(
            store.dead_mans_alive(&c, &sk, 1_200).unwrap(),
            "principal alive = reversibility"
        );
        assert!(
            store.enact_dead_mans(&c, &sk, 1_250).unwrap().is_none(),
            "alive at 1200, grace 100 → not lapsed until 1300"
        );
        // Later, no further aliveness; heartbeat lapsed again + quorum persists → it enacts its disposition.
        let disp = store.enact_dead_mans(&c, &sk, 1_400).unwrap();
        assert!(matches!(disp, Some(Disposition::ReleaseTo { .. })));
        // The whole chain (arm + attest×2 + alive + enact) verifies.
        assert_eq!(store.verify_ledger().unwrap(), Ok(()));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn dead_mans_release_hands_the_key_to_the_disposition_party() {
        use crate::dead_mans_switch::{
            AttestationKind, DeadMansSwitch, Disposition, Heartbeat, PartyAttestation, TriggerRule,
        };
        use crate::envelope_encryption::{
            open_payload_with_wrapped, seal_payload, EnvelopeKeypair,
        };
        let dir = tempfile::tempdir().unwrap();
        let store = AccountabilityStore::open(dir.path()).unwrap();
        let sk = signer(9);
        let trustee = EnvelopeKeypair::generate().unwrap();

        // Seal a payload; persist the ciphertext. The DEK is what the switch will hand over on enact.
        let (payload, dek) =
            seal_payload(b"the estate letter", vec!["did:wf:person".into()]).unwrap();
        let c = payload.commitment;
        {
            let mut st = store.load().unwrap();
            st.payloads.push(payload);
            store.save(&st).unwrap();
        }
        // Before enactment the trustee has no credential — no way in.
        assert!(store
            .wrapped_key_for(&c, "did:wf:trustee", 1_100)
            .unwrap()
            .is_none());

        // Arm a switch releasing to the trustee; a friend attests; heartbeat lapses.
        store
            .arm_dead_mans_switch(
                DeadMansSwitch {
                    payload_commitment: c,
                    heartbeat: Heartbeat::new(1_000, 100),
                    trigger: TriggerRule {
                        require_heartbeat_lapsed: true,
                        attestation_threshold: 1,
                        parties: vec!["did:wf:friend".into()],
                    },
                    disposition: Disposition::ReleaseTo {
                        parties: vec!["did:wf:trustee".into()],
                    },
                    fired_unix: None,
                },
                &sk,
                1_000,
            )
            .unwrap();
        store
            .attest_dead_mans(
                &c,
                PartyAttestation {
                    party_did: "did:wf:friend".into(),
                    kind: AttestationKind::BelievedDead,
                    time_unix: 1_200,
                },
                &sk,
                1_200,
            )
            .unwrap();

        // Enact + release the DEK to the trustee's X25519 key.
        let party_keys = vec![("did:wf:trustee".to_string(), trustee.public)];
        let disp = store
            .enact_dead_mans_release(&c, &dek, &party_keys, "did:wf:person", &sk, 1_200)
            .unwrap();
        assert!(matches!(disp, Some(Disposition::ReleaseTo { .. })));

        // The trustee now holds a credential whose wrapped key opens the payload with the trustee's secret —
        // access was genuinely handed over by the crypto, not just recorded.
        let st = store.load().unwrap();
        let cred = st
            .credentials
            .iter()
            .find(|c2| c2.agent_did == "did:wf:trustee")
            .expect("trustee credential granted on enact");
        let wrapped = cred.payload_key(1_300).expect("live wrapped key");
        let payload = st.payloads.iter().find(|p| p.commitment == c).unwrap();
        let opened = open_payload_with_wrapped(payload, &trustee.secret, wrapped).unwrap();
        assert_eq!(opened, b"the estate letter");
        assert_eq!(store.verify_ledger().unwrap(), Ok(()));
    }

    #[test]
    fn incapacity_switch_activates_with_quorum_and_reverses() {
        use crate::incapacity_switch::{IncapacityKind, IncapacitySwitch, IncapacityTrigger};
        let dir = tempfile::tempdir().unwrap();
        let store = AccountabilityStore::open(dir.path()).unwrap();
        let sk = signer(7);
        let sw = IncapacitySwitch {
            principal_did: "did:wf:person".into(),
            kind: IncapacityKind::InvoluntaryPsychiatric,
            trigger: IncapacityTrigger {
                parties: vec!["adv".into(), "friend".into()],
                attestation_threshold: 2,
                require_official_instrument: false,
            },
            advocate_did: "did:wf:advocate".into(),
            active_since_unix: None,
        };
        store.arm_incapacity_switch(sw, &sk, 1_000).unwrap();
        // One attester is not enough (corroboration required).
        assert!(!store
            .activate_incapacity("did:wf:person", &["adv".into()], None, &sk, 1_100)
            .unwrap());
        // Quorum → advocate activates.
        assert!(store
            .activate_incapacity(
                "did:wf:person",
                &["adv".into(), "friend".into()],
                None,
                &sk,
                1_100
            )
            .unwrap());
        assert!(store.list_incapacity_switches().unwrap()[0].advocate_active());
        // Recovery reverses it.
        assert!(store.regain_capacity("did:wf:person", &sk, 1_500).unwrap());
        assert!(!store.list_incapacity_switches().unwrap()[0].advocate_active());
        assert_eq!(store.verify_ledger().unwrap(), Ok(()));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn social_recovery_reconstructs_the_key_without_the_owner() {
        use crate::dead_mans_switch::{
            AttestationKind, DeadMansSwitch, Disposition, Heartbeat, PartyAttestation, TriggerRule,
        };
        use crate::envelope_encryption::{
            open_payload_with_wrapped, seal_payload, EnvelopeKeypair,
        };
        use crate::shamir_recovery::split;
        let dir = tempfile::tempdir().unwrap();
        let store = AccountabilityStore::open(dir.path()).unwrap();
        let sk = signer(10);
        let trustee = EnvelopeKeypair::generate().unwrap();

        // Owner seals a payload (while alive); the DEK is split 2-of-3 among friends and handed out.
        let (payload, dek) =
            seal_payload(b"the will and testament", vec!["did:wf:person".into()]).unwrap();
        let c = payload.commitment;
        {
            let mut st = store.load().unwrap();
            st.payloads.push(payload);
            store.save(&st).unwrap();
        }
        let shares = split(&dek, 2, 3).unwrap();

        store
            .arm_dead_mans_switch(
                DeadMansSwitch {
                    payload_commitment: c,
                    heartbeat: Heartbeat::new(1_000, 100),
                    trigger: TriggerRule {
                        require_heartbeat_lapsed: true,
                        attestation_threshold: 1,
                        parties: vec!["did:wf:friend".into()],
                    },
                    disposition: Disposition::ReleaseTo {
                        parties: vec!["did:wf:trustee".into()],
                    },
                    fired_unix: None,
                },
                &sk,
                1_000,
            )
            .unwrap();
        store
            .attest_dead_mans(
                &c,
                PartyAttestation {
                    party_did: "did:wf:friend".into(),
                    kind: AttestationKind::BelievedDead,
                    time_unix: 1_200,
                },
                &sk,
                1_200,
            )
            .unwrap();

        // Post-death: TWO friends combine their shares — **the owner key is never used** — to reconstruct the
        // DEK and release it to the trustee.
        let quorum = vec![shares[0].clone(), shares[2].clone()];
        let party_keys = vec![("did:wf:trustee".to_string(), trustee.public)];
        let disp = store
            .reconstruct_and_release(&c, &quorum, &party_keys, "did:wf:person", &sk, 1_200)
            .unwrap();
        assert!(matches!(disp, Some(Disposition::ReleaseTo { .. })));

        // The trustee opens the payload — recovered entirely from friends' shares, no owner key involved.
        let st = store.load().unwrap();
        let cred = st
            .credentials
            .iter()
            .find(|c2| c2.agent_did == "did:wf:trustee")
            .unwrap();
        let wrapped = cred.payload_key(1_300).unwrap();
        let p = st.payloads.iter().find(|p| p.commitment == c).unwrap();
        assert_eq!(
            open_payload_with_wrapped(p, &trustee.secret, wrapped).unwrap(),
            b"the will and testament"
        );
        assert_eq!(store.verify_ledger().unwrap(), Ok(()));
    }

    #[test]
    fn disclosure_trace_records_and_attributes_a_staff_leak() {
        use crate::disclosure_trace::{DisclosureEvent, DisclosureKind, TransparencyCc};
        let dir = tempfile::tempdir().unwrap();
        let store = AccountabilityStore::open(dir.path()).unwrap();
        let sk = signer(8);
        let c = [9u8; 32];
        let fp_staff = [2u8; 16];

        // The person cc's the MP (protective record), then the MP accesses, then the MP's staffer leaks onward.
        store
            .record_transparency_cc(
                TransparencyCc {
                    credential_id: "cc-t".into(),
                    informed_authority_did: "did:wf:mp".into(),
                    purpose: "protection from serious crime".into(),
                    informed_unix: 1_000,
                },
                &sk,
                1_000,
            )
            .unwrap();
        store
            .record_disclosure_event(
                DisclosureEvent {
                    id: "d1".into(),
                    payload_commitment: c,
                    credential_id: "cc-t".into(),
                    recipient_did: "did:wf:mp".into(),
                    acting_delegate_did: None,
                    time_unix: 1_100,
                    fingerprint: [1u8; 16],
                    kind: DisclosureKind::DirectAccess,
                },
                &sk,
                1_100,
            )
            .unwrap();
        store
            .record_disclosure_event(
                DisclosureEvent {
                    id: "d2".into(),
                    payload_commitment: c,
                    credential_id: "cc-t".into(),
                    recipient_did: "did:wf:mp".into(),
                    acting_delegate_did: Some("did:wf:staffer".into()),
                    time_unix: 1_200,
                    fingerprint: fp_staff,
                    kind: DisclosureKind::OnwardShare {
                        to_did: "did:wf:perpetrator".into(),
                    },
                },
                &sk,
                1_200,
            )
            .unwrap();

        assert_eq!(store.disclosure_chain(&c).unwrap().len(), 2);
        let actors = store.actors_with_access(&c).unwrap();
        assert!(actors.contains(&"did:wf:mp".to_string()));
        assert!(
            actors.contains(&"did:wf:staffer".to_string()),
            "the acting staffer is in the access set"
        );
        // The leaked fingerprint traces to the staffer as the accountable actor — the betrayal is knowable.
        let ev = store.trace_leak(&fp_staff).unwrap().unwrap();
        assert_eq!(ev.accountable_actor(), "did:wf:staffer");
        assert_eq!(store.verify_ledger().unwrap(), Ok(()));
    }

    #[test]
    fn parse_commitment_hex_roundtrips() {
        let c = [0x2au8; 32];
        let hexed = hex::encode(c);
        assert_eq!(parse_commitment_hex(&hexed).unwrap(), c);
        assert!(parse_commitment_hex("zz").is_err());
        assert!(
            parse_commitment_hex("2a2a").is_err(),
            "wrong length rejected"
        );
    }
}
