//! Envelope encryption + safeguard switches (dead-man, incapacity)


use sha2::{Digest, Sha256};


use super::*;

impl WebizenHostApi {
    // --- Real envelope encryption over the consent credential (ADR 0011 D1/D2) ---
    //
    // Makes "revoke destroys the wrapped key â‡’ no key, no payload" a *fact*: the payload is AEAD-encrypted
    // under a random DEK; the DEK is sealed (X25519 sealed box) to the recipient's public key â€” that sealed
    // DEK is the credential's real `wrapped_key`; revoke destroys it. The owner's envelope keypair is
    // **derived** from the owner signing-key seed (nothing secret stored at rest). Native-only (the sealed-box
    // primitives are `not(wasm32)`; the desktop owns keys).

    /// The owner's envelope **public** key (hex) â€” publishable so others can seal payloads *to* the owner.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn owner_envelope_public_hex(&self) -> String {
        use crate::envelope_encryption::{EnvelopeKeypair, OWNER_ENVELOPE_DOMAIN};
        EnvelopeKeypair::derive(&self.signing_key.to_bytes(), OWNER_ENVELOPE_DOMAIN).public_hex()
    }

    /// **Seal a plaintext payload and grant a consent credential over it** â€” real envelope encryption. If
    /// `agent_public_hex` is empty, the payload is sealed to the OWNER's derived envelope key (self-custody,
    /// so the owner can [`open_owner_payload`]); supply an agent's X25519 public key to grant *that* agent
    /// access (they open it on their own device with their secret â€” the owner cannot).
    ///
    /// [`open_owner_payload`]: WebizenHostApi::open_owner_payload
    #[cfg(not(target_arch = "wasm32"))]
    pub fn seal_and_grant_consent_credential(
        &self,
        agent_did: &str,
        agent_public_hex: &str,
        scope: &str,
        purpose: &str,
        plaintext: &str,
        expiry_unix: Option<u64>,
    ) -> Result<crate::consent_credential::ConsentCredential, String> {
        use crate::envelope_encryption::{EnvelopeKeypair, OWNER_ENVELOPE_DOMAIN};
        let owner = EnvelopeKeypair::derive(&self.signing_key.to_bytes(), OWNER_ENVELOPE_DOMAIN);
        let recipient_public: [u8; 32] = if agent_public_hex.trim().is_empty() {
            owner.public
        } else {
            let bytes = hex::decode(agent_public_hex.trim())
                .map_err(|e| format!("agent public key not hex: {e}"))?;
            bytes
                .as_slice()
                .try_into()
                .map_err(|_| "agent public key must be 32 bytes".to_string())?
        };
        let now = Self::now_unix();
        let id = {
            let d = Sha256::digest(format!("{agent_did}:{scope}:{now}").as_bytes());
            format!("cc-{}", hex::encode(&d[..6]))
        };
        self.accountability_store()?
            .seal_and_grant_credential(
                id,
                &self.owner_did,
                agent_did,
                scope,
                purpose,
                plaintext.as_bytes(),
                &recipient_public,
                vec![self.owner_did.clone()],
                expiry_unix,
                &self.signing_key,
                now,
            )
            .map_err(|e| e.to_string())
    }

    /// **Open an owner-sealed payload** through a credential â€” proves the crypto-revoke property end-to-end:
    /// works while the credential is live, fails once revoked (the wrapped key is gone), though the commons
    /// ciphertext survives. Only opens payloads sealed to the owner (an agent-sealed payload opens on the
    /// agent's device).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_owner_payload(&self, credential_id: &str) -> Result<String, String> {
        use crate::envelope_encryption::{EnvelopeKeypair, OWNER_ENVELOPE_DOMAIN};
        let owner = EnvelopeKeypair::derive(&self.signing_key.to_bytes(), OWNER_ENVELOPE_DOMAIN);
        let bytes = self
            .accountability_store()?
            .open_payload_via_credential(credential_id, &owner.secret, Self::now_unix())
            .map_err(|e| e.to_string())?;
        String::from_utf8(bytes).map_err(|e| format!("payload not valid utf-8: {e}"))
    }

    // --- Safeguard switches (ADR 0011 D6/D7): dead-man + incapacity, owner-signed into the ledger ---

    /// Arm a **dead-man switch** over a payload (post-death disposition; gamified + reversible).
    pub fn arm_dead_mans_switch(
        &self,
        switch: crate::dead_mans_switch::DeadMansSwitch,
    ) -> Result<(), String> {
        self.accountability_store()?
            .arm_dead_mans_switch(switch, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// **I'm alive** â€” touch the heartbeat + un-fire a not-yet-enacted switch (reversibility). The routine
    /// owner-side action that keeps a dead-man switch from firing.
    pub fn dead_mans_alive(&self, commitment_hex: &str) -> Result<bool, String> {
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        self.accountability_store()?
            .dead_mans_alive(&c, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// Record a **party attestation** toward a dead-man switch's gamified trigger.
    pub fn attest_dead_mans(
        &self,
        commitment_hex: &str,
        attestation: crate::dead_mans_switch::PartyAttestation,
    ) -> Result<bool, String> {
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        self.accountability_store()?
            .attest_dead_mans(&c, attestation, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// **Enact** a dead-man switch if the gamified rule holds â€” returns the [`Disposition`] to carry out.
    ///
    /// [`Disposition`]: crate::dead_mans_switch::Disposition
    pub fn enact_dead_mans(
        &self,
        commitment_hex: &str,
    ) -> Result<Option<crate::dead_mans_switch::Disposition>, String> {
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        self.accountability_store()?
            .enact_dead_mans(&c, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// List armed dead-man switches (with accumulated attestations).
    pub fn list_dead_mans_switches(
        &self,
    ) -> Result<Vec<crate::accountability_store::DeadMansSwitchRecord>, String> {
        self.accountability_store()?.list_dead_mans_switches().map_err(|e| e.to_string())
    }

    /// **Enact a dead-man switch AND release the keys** (ADR 0011 D6, key-release-on-enact). Recovers the
    /// payload DEK by unwrapping the owner's own credential, then â€” for a `ReleaseTo` disposition â€” re-seals
    /// the DEK to each supplied party X25519 pubkey and grants them a credential, so the disposition actually
    /// hands over access. `party_keys` = `(did, pubkey_hex)` pairs. (The owner key is derivable here; the true
    /// post-death friend-side release without the owner needs Shamir pre-positioning â€” separate.)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enact_dead_mans_release(
        &self,
        commitment_hex: &str,
        party_keys_hex: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        use crate::envelope_encryption::{unwrap_dek, EnvelopeKeypair, OWNER_ENVELOPE_DOMAIN};
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        let now = Self::now_unix();
        let store = self.accountability_store()?;
        let owner = EnvelopeKeypair::derive(&self.signing_key.to_bytes(), OWNER_ENVELOPE_DOMAIN);
        // Recover the DEK by unwrapping the owner's own credential for this payload.
        let wrapped = store
            .wrapped_key_for(&c, &self.owner_did, now)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| {
                "no owner credential holds the DEK for this payload (seal it to yourself first)".to_string()
            })?;
        let dek = unwrap_dek(&owner.secret, &wrapped)?;
        let mut party_keys: Vec<(String, [u8; 32])> = Vec::new();
        for (did, pk_hex) in party_keys_hex {
            let bytes = hex::decode(pk_hex.trim()).map_err(|e| format!("party key not hex: {e}"))?;
            let pk: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| "party key must be 32 bytes".to_string())?;
            party_keys.push((did, pk));
        }
        let disposition = store
            .enact_dead_mans_release(&c, &dek, &party_keys, &self.owner_did, &self.signing_key, now)
            .map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "enacted": disposition.is_some(), "disposition": disposition }))
    }

    /// **Split a payload's DEK into Shamir social-recovery shares** (`threshold`-of-`parties.len()`), so a
    /// quorum of friends can later reconstruct the key **without the owner**. Recovers the DEK from the owner's
    /// own credential, splits it, and returns the shares paired with the parties they should be handed to
    /// (the caller distributes them off-device â€” they are NOT stored here). Owner-side, done while alive.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn split_dek_recovery(
        &self,
        commitment_hex: &str,
        threshold: usize,
        parties: Vec<String>,
    ) -> Result<serde_json::Value, String> {
        use crate::envelope_encryption::{unwrap_dek, EnvelopeKeypair, OWNER_ENVELOPE_DOMAIN};
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        let now = Self::now_unix();
        let store = self.accountability_store()?;
        let owner = EnvelopeKeypair::derive(&self.signing_key.to_bytes(), OWNER_ENVELOPE_DOMAIN);
        let wrapped = store
            .wrapped_key_for(&c, &self.owner_did, now)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "no owner credential holds the DEK for this payload".to_string())?;
        let dek = unwrap_dek(&owner.secret, &wrapped)?;
        let shares = crate::shamir_recovery::split(&dek, threshold, parties.len())?;
        let tagged: Vec<serde_json::Value> = parties
            .iter()
            .zip(shares.iter())
            .map(|(party, share)| serde_json::json!({ "party": party, "share": share }))
            .collect();
        Ok(serde_json::json!({ "threshold": threshold, "shares": tagged }))
    }

    /// **Social-recovery enactment (no owner key):** given a quorum of friends' Shamir shares, reconstruct the
    /// DEK, enact the dead-man switch, and release to the disposition parties. `party_keys` = `(did, pubkey_hex)`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reconstruct_and_release(
        &self,
        commitment_hex: &str,
        shares: Vec<crate::shamir_recovery::Share>,
        party_keys_hex: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        let now = Self::now_unix();
        let mut party_keys: Vec<(String, [u8; 32])> = Vec::new();
        for (did, pk_hex) in party_keys_hex {
            let bytes = hex::decode(pk_hex.trim()).map_err(|e| format!("party key not hex: {e}"))?;
            let pk: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| "party key must be 32 bytes".to_string())?;
            party_keys.push((did, pk));
        }
        let disposition = self
            .accountability_store()?
            .reconstruct_and_release(&c, &shares, &party_keys, &self.owner_did, &self.signing_key, now)
            .map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "enacted": disposition.is_some(), "disposition": disposition }))
    }

    /// Publish a **peer's envelope (X25519) public key** into their peer record, so releases to that party
    /// can resolve the key automatically (remote-key distribution). The owner's own publishable key is
    /// [`owner_envelope_public_hex`](Self::owner_envelope_public_hex).
    pub fn set_peer_envelope_key(&self, did: &str, pubkey_hex: &str) -> Result<(), String> {
        crate::social_peers::set_peer_envelope_key(did, pubkey_hex)
    }

    /// **Enact + release resolving the disposition parties' keys from the peer store** (remote-key
    /// distribution). Reads the switch's `ReleaseTo` parties, looks up each one's published envelope key from
    /// `social_peers`, and releases to those with a known key â€” reporting any parties whose key is still
    /// missing (so the owner knows to obtain it). No keys pasted by hand.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enact_dead_mans_release_via_peers(
        &self,
        commitment_hex: &str,
    ) -> Result<serde_json::Value, String> {
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        let switches = self
            .accountability_store()?
            .list_dead_mans_switches()
            .map_err(|e| e.to_string())?;
        let rec = switches
            .iter()
            .find(|r| r.switch.payload_commitment == c)
            .ok_or_else(|| "no dead-man switch for that commitment".to_string())?;
        let parties = match &rec.switch.disposition {
            crate::dead_mans_switch::Disposition::ReleaseTo { parties } => parties.clone(),
            _ => Vec::new(),
        };
        let peers = crate::social_peers::list_peers();
        let resolved = crate::social_peers::resolve_envelope_keys(&peers, &parties);
        let have: std::collections::BTreeSet<&str> = resolved.iter().map(|(d, _)| d.as_str()).collect();
        let missing: Vec<String> =
            parties.iter().filter(|d| !have.contains(d.as_str())).cloned().collect();
        let result = self.enact_dead_mans_release(commitment_hex, resolved)?;
        Ok(serde_json::json!({ "result": result, "missing_keys_for": missing }))
    }

    /// Arm an **incapacity switch** (advocate activation on validated, reversible incapacity).
    pub fn arm_incapacity_switch(
        &self,
        switch: crate::incapacity_switch::IncapacitySwitch,
    ) -> Result<(), String> {
        self.accountability_store()?
            .arm_incapacity_switch(switch, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// **Activate** advocacy if the corroborated trigger holds (quorum + optional official instrument).
    pub fn activate_incapacity(
        &self,
        principal_did: &str,
        attesting_parties: Vec<String>,
        official_instrument: Option<String>,
    ) -> Result<bool, String> {
        self.accountability_store()?
            .activate_incapacity(
                principal_did,
                &attesting_parties,
                official_instrument.as_deref(),
                &self.signing_key,
                Self::now_unix(),
            )
            .map_err(|e| e.to_string())
    }

    /// **Regain capacity** â€” the advocate stands down (reversibility).
    pub fn regain_capacity(&self, principal_did: &str) -> Result<bool, String> {
        self.accountability_store()?
            .regain_capacity(principal_did, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// List armed incapacity switches.
    pub fn list_incapacity_switches(
        &self,
    ) -> Result<Vec<crate::incapacity_switch::IncapacitySwitch>, String> {
        self.accountability_store()?.list_incapacity_switches().map_err(|e| e.to_string())
    }

}