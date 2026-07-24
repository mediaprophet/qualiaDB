//! Accountability fabric: ledger + consent credentials


use sha2::{Digest, Sha256};


use super::*;

impl WebizenHostApi {
    // --- Accountability fabric (ADR 0011) — tamper-evident ledger + revocable consent credentials ---
    //
    // Turns the tested domain models (`crate::accountability_ledger`, `crate::consent_credential`) into a
    // usable loop: grant a worker scoped access, record how/why they acted (attributable, court-auditable),
    // let the person revoke (crypto-enforced — the key is destroyed, access ends), and keep the conduct trail
    // un-erasable. All acts are written into a signed, hash-chained ledger the person's own key signs; a
    // betrayer cannot quietly drop the inconvenient act without `verify()` naming it. Anti-deletion durability
    // across parties (commons replication) and real envelope encryption of the wrapped key are the deferred
    // composition steps (coordinate) — the wrapped key is carried as opaque bytes here, as the model intends.

    pub(crate) fn accountability_store(&self) -> Result<crate::accountability_store::AccountabilityStore, String> {
        crate::accountability_store::AccountabilityStore::open(&self.storage_root)
            .map_err(|e| e.to_string())
    }

    /// Append a raw record to the person's tamper-evident accountability ledger, signed by the owner key.
    pub fn ledger_append(
        &self,
        kind: &str,
        payload_json: &str,
    ) -> Result<crate::accountability_ledger::LedgerEntry, String> {
        self.accountability_store()?
            .append_ledger(kind, payload_json, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// Verify the whole ledger chain. `Ok(None)` = intact; `Ok(Some(tamper))` = a detected, named tamper.
    pub fn ledger_verify(
        &self,
    ) -> Result<Option<crate::accountability_ledger::LedgerTamper>, String> {
        let verdict = self.accountability_store()?.verify_ledger().map_err(|e| e.to_string())?;
        Ok(verdict.err())
    }

    /// The most-recent ledger entries (newest first), capped to `limit`.
    pub fn ledger_entries(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::accountability_ledger::LedgerEntry>, String> {
        self.accountability_store()?.ledger_entries(limit).map_err(|e| e.to_string())
    }

    /// **Grant a consent credential** to an agent (e.g. a social worker) over a committed payload. The
    /// subject is the vault owner. `commitment_hex` is the 32-byte payload commitment; `wrapped_key_hex` is
    /// the (opaque) wrapped data key that revocation destroys; `expiry_unix` optionally auto-expires access.
    pub fn grant_consent_credential(
        &self,
        agent_did: &str,
        scope: &str,
        purpose: &str,
        commitment_hex: &str,
        wrapped_key_hex: &str,
        expiry_unix: Option<u64>,
    ) -> Result<crate::consent_credential::ConsentCredential, String> {
        let commitment = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        let wrapped_key = hex::decode(wrapped_key_hex.trim())
            .map_err(|e| format!("wrapped key not hex: {e}"))?;
        let now = Self::now_unix();
        let id = {
            let digest = Sha256::digest(format!("{agent_did}:{scope}:{now}").as_bytes());
            format!("cc-{}", hex::encode(&digest[..6]))
        };
        let cred = crate::consent_credential::ConsentCredential::grant(
            id,
            &self.owner_did,
            agent_did,
            scope,
            purpose,
            commitment,
            wrapped_key,
            now,
            expiry_unix,
        );
        self.accountability_store()?
            .grant_credential(cred, &self.signing_key, now)
            .map_err(|e| e.to_string())
    }

    /// **Revoke a consent credential** — crypto-enforced (the wrapped key is destroyed). Returns whether a
    /// live credential was revoked. The conduct trail under it persists.
    pub fn revoke_consent_credential(&self, credential_id: &str) -> Result<bool, String> {
        self.accountability_store()?
            .revoke_credential(credential_id, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// All stored consent credentials (active and revoked — revoked rows remain as the audit anchor).
    pub fn list_consent_credentials(
        &self,
    ) -> Result<Vec<crate::consent_credential::ConsentCredential>, String> {
        self.accountability_store()?.list_credentials().map_err(|e| e.to_string())
    }

    /// **Record an agent's conduct** under a credential — signed (attributable + court-auditable) — into the
    /// durable trail and the tamper-evident ledger. Binds to the payload commitment, not the payload.
    pub fn record_conduct(
        &self,
        agent_did: &str,
        credential_id: &str,
        action: &str,
        reason: &str,
        commitment_hex: &str,
    ) -> Result<crate::consent_credential::ConductRecord, String> {
        let commitment = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        self.accountability_store()?
            .record_conduct(
                agent_did,
                credential_id,
                action,
                reason,
                commitment,
                &self.signing_key,
                Self::now_unix(),
            )
            .map_err(|e| e.to_string())
    }

    /// The **audit view** — every conduct record taken under one credential (survives its revocation).
    pub fn conduct_audit_trail(
        &self,
        credential_id: &str,
    ) -> Result<Vec<crate::consent_credential::ConductRecord>, String> {
        self.accountability_store()?.audit_trail(credential_id).map_err(|e| e.to_string())
    }

    /// **Record guardian notifications** from a flagged ingest into the tamper-evident ledger — so a flagged
    /// ingest under a guardianship relation is both a notification to the guardian AND an auditable,
    /// un-erasable event (who was notified, about what, when). Composes the hypermedia flags → guardian layer
    /// (`super::super::ingest_guardian`) with the accountability ledger.
    pub fn record_guardian_notifications(
        &self,
        notifications: &[super::super::ingest_guardian::GuardianNotification],
    ) -> Result<(), String> {
        let store = self.accountability_store()?;
        for n in notifications {
            let payload = serde_json::to_string(n).map_err(|e| e.to_string())?;
            store
                .append_ledger("guardian_notified", &payload, &self.signing_key, Self::now_unix())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

}