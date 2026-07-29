//! Disclosure traceability + encrypted sanctuary vault

use super::super::med_reminders::{
    compute_due_reminders, load_prefs, save_prefs, DueMedReminder, MedReminderPrefs,
};
use super::super::sanctuary::{
    load_prefs as load_sanctuary_prefs, lock_sanctuary, setup_sanctuary, unlock_sanctuary,
    SanctuaryPrefs,
};
use sha2::{Digest, Sha256};
use wellfare_core::sleep_analytics::{
    self, SleepDebtReport, SleepHeatmapReport, SleepNightSample, DEFAULT_TARGET_SLEEP_MIN,
};

use super::super::personal_profile::{new_contact_id, EmergencyContact, EmergencyContactStore};

use super::*;

impl WebizenHostApi {
    // --- Disclosure traceability (ADR 0011 D5) + duty of inquiry (D8) ---

    /// Record a **transparency cc** — the protective "I informed authority X for purpose Y" note.
    pub fn record_transparency_cc(
        &self,
        credential_id: &str,
        informed_authority_did: &str,
        purpose: &str,
    ) -> Result<(), String> {
        let cc = crate::disclosure_trace::TransparencyCc {
            credential_id: credential_id.to_string(),
            informed_authority_did: informed_authority_did.to_string(),
            purpose: purpose.to_string(),
            informed_unix: Self::now_unix(),
        };
        self.accountability_store()?
            .record_transparency_cc(cc, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// Record a **disclosure event** (an access, or an onward-share if `onward_to` is set). A per-recipient
    /// fingerprint + id are generated. Returns the recorded event (its `fingerprint` is the tracing anchor).
    pub fn record_disclosure(
        &self,
        commitment_hex: &str,
        credential_id: &str,
        recipient_did: &str,
        acting_delegate_did: Option<String>,
        onward_to: Option<String>,
    ) -> Result<crate::disclosure_trace::DisclosureEvent, String> {
        let commitment = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        let now = Self::now_unix();
        let actor = acting_delegate_did.as_deref().unwrap_or(recipient_did);
        // Deterministic per-recipient/per-disclosure fingerprint (the traitor-tracing anchor).
        let digest = Sha256::digest(
            format!("{}:{recipient_did}:{actor}:{now}", hex::encode(commitment)).as_bytes(),
        );
        let mut fingerprint = [0u8; 16];
        fingerprint.copy_from_slice(&digest[..16]);
        let id = format!("d-{}", hex::encode(&digest[16..22]));
        let kind = match onward_to {
            Some(to_did) => crate::disclosure_trace::DisclosureKind::OnwardShare { to_did },
            None => crate::disclosure_trace::DisclosureKind::DirectAccess,
        };
        let event = crate::disclosure_trace::DisclosureEvent {
            id,
            payload_commitment: commitment,
            credential_id: credential_id.to_string(),
            recipient_did: recipient_did.to_string(),
            acting_delegate_did,
            time_unix: now,
            fingerprint,
            kind,
        };
        self.accountability_store()?
            .record_disclosure_event(event.clone(), &self.signing_key, now)
            .map_err(|e| e.to_string())?;
        Ok(event)
    }

    /// The disclosure chain for a payload (who saw it, via which route).
    pub fn disclosure_chain(
        &self,
        commitment_hex: &str,
    ) -> Result<Vec<crate::disclosure_trace::DisclosureEvent>, String> {
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        self.accountability_store()?
            .disclosure_chain(&c)
            .map_err(|e| e.to_string())
    }

    /// The distinct actors who had access to a payload — the set a leak must be within.
    pub fn actors_with_access(&self, commitment_hex: &str) -> Result<Vec<String>, String> {
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        self.accountability_store()?
            .actors_with_access(&c)
            .map_err(|e| e.to_string())
    }

    /// **Trace a leak** by its fingerprint (hex, 16 bytes) → the disclosure + accountable actor.
    pub fn trace_leak(
        &self,
        fingerprint_hex: &str,
    ) -> Result<Option<crate::disclosure_trace::DisclosureEvent>, String> {
        let bytes =
            hex::decode(fingerprint_hex.trim()).map_err(|e| format!("fingerprint not hex: {e}"))?;
        let fp: crate::disclosure_trace::DisclosureFingerprint = bytes
            .as_slice()
            .try_into()
            .map_err(|_| "fingerprint must be 16 bytes".to_string())?;
        self.accountability_store()?
            .trace_leak(&fp)
            .map_err(|e| e.to_string())
    }

    /// List transparency cc records.
    pub fn list_transparency_ccs(
        &self,
    ) -> Result<Vec<crate::disclosure_trace::TransparencyCc>, String> {
        self.accountability_store()?
            .list_transparency_ccs()
            .map_err(|e| e.to_string())
    }

    /// **Assess a duty of inquiry** — classify conduct against the duty (the fair negligence classifier: was
    /// an accessible means left unchecked, and did a harmful act follow?). Pure; no persistence.
    pub fn assess_duty_of_inquiry(
        &self,
        duty: crate::duty_of_inquiry::DutyOfInquiry,
        conduct: crate::duty_of_inquiry::ConductAgainstDuty,
    ) -> crate::duty_of_inquiry::InquiryVerdict {
        crate::duty_of_inquiry::assess(&duty, &conduct)
    }

    pub fn sleep_analytics(
        &self,
        target_min: f64,
    ) -> Result<(SleepDebtReport, SleepHeatmapReport), String> {
        let sleep_rows = self.list_journal_by_kind("sleep", 128)?;
        let mut samples = Vec::new();
        for row in sleep_rows {
            if let Some(ref summary) = row.summary {
                if let Some((dur, eff)) = sleep_analytics::parse_sleep_summary_json(summary) {
                    samples.push(SleepNightSample {
                        night_unix: row.asserted_time_unix,
                        duration_min: dur,
                        efficiency: eff,
                    });
                }
            }
        }
        samples.sort_by_key(|s| s.night_unix);
        let debt = sleep_analytics::compute_sleep_debt(&samples, target_min);
        let heatmap = sleep_analytics::compute_weekly_heatmap(&samples, target_min);
        Ok((debt, heatmap))
    }

    pub fn default_sleep_analytics(&self) -> Result<(SleepDebtReport, SleepHeatmapReport), String> {
        self.sleep_analytics(DEFAULT_TARGET_SLEEP_MIN)
    }

    pub fn add_emergency_contact(
        &self,
        display_name: &str,
        relationship: &str,
        phone: Option<String>,
        email: Option<String>,
        notes: Option<String>,
    ) -> Result<EmergencyContact, String> {
        let now = Self::now_unix() as u32;
        let contact = EmergencyContact {
            id: new_contact_id(display_name, now),
            display_name: display_name.to_string(),
            relationship: relationship.to_string(),
            phone,
            email,
            notes,
            created_at_unix: now,
        };
        let store = EmergencyContactStore::open(&self.storage_root).map_err(|e| e.to_string())?;
        store.append(&contact).map_err(|e| e.to_string())?;
        Ok(contact)
    }

    pub fn list_emergency_contacts(&self) -> Result<Vec<EmergencyContact>, String> {
        let store = EmergencyContactStore::open(&self.storage_root).map_err(|e| e.to_string())?;
        store.list().map_err(|e| e.to_string())
    }

    pub fn med_reminder_prefs(&self) -> MedReminderPrefs {
        load_prefs(&self.storage_root)
    }

    pub fn set_med_reminders_enabled(&self, enabled: bool) -> Result<MedReminderPrefs, String> {
        let mut prefs = load_prefs(&self.storage_root);
        if enabled && !prefs.permission_granted {
            return Err("Grant reminder permission before enabling notifications".into());
        }
        prefs.enabled = enabled;
        save_prefs(&self.storage_root, &prefs).map_err(|e| e.to_string())?;
        Ok(prefs)
    }

    pub fn grant_med_reminder_permission(&self) -> Result<MedReminderPrefs, String> {
        let mut prefs = load_prefs(&self.storage_root);
        prefs.permission_granted = true;
        prefs.permission_granted_at_unix = Some(Self::now_unix() as u32);
        save_prefs(&self.storage_root, &prefs).map_err(|e| e.to_string())?;
        Ok(prefs)
    }

    pub fn list_due_med_reminders(
        &self,
        window_minutes: i32,
    ) -> Result<Vec<DueMedReminder>, String> {
        let prefs = load_prefs(&self.storage_root);
        if !prefs.enabled || !prefs.permission_granted {
            return Ok(Vec::new());
        }
        let journal = self
            .vault
            .list_health_records(128)
            .map_err(|e| e.to_string())?;
        let now = chrono::Local::now().time();
        Ok(compute_due_reminders(&journal, now, window_minutes))
    }

    pub fn sanctuary_prefs(&self) -> SanctuaryPrefs {
        load_sanctuary_prefs(&self.storage_root)
    }

    pub fn setup_sanctuary(
        &self,
        real_pin: &str,
        decoy_pin: &str,
    ) -> Result<SanctuaryPrefs, String> {
        setup_sanctuary(
            &self.storage_root,
            real_pin,
            decoy_pin,
            Self::now_unix() as u32,
        )
    }

    pub fn lock_sanctuary(&self) -> Result<SanctuaryPrefs, String> {
        lock_sanctuary(&self.storage_root)
    }

    pub fn unlock_sanctuary(&self, pin: &str) -> Result<SanctuaryPrefs, String> {
        unlock_sanctuary(&self.storage_root, pin)
    }

    // --- Encrypted Sanctuary vault (real boundary; native-only, plan §6) ---
    //
    // Sensitive free-text notes are stored ONLY inside AEAD-encrypted lane files keyed by a
    // PBKDF2-derived key — there is no plaintext journal path for them. Nothing is readable
    // without the PIN, and the decoy PIN opens a separate lane that never aliases real data.

    #[cfg(not(target_arch = "wasm32"))]
    pub fn sanctuary_vault_configured(&self) -> bool {
        super::super::sanctuary_vault::is_configured(&self.storage_root)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn setup_sanctuary_vault(&self, real_pin: &str, decoy_pin: &str) -> Result<(), String> {
        super::super::sanctuary_vault::setup(&self.storage_root, real_pin, decoy_pin)
    }

    /// Verify a PIN and report which lane it opens (real vs duress decoy).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn sanctuary_vault_resolve_lane(
        &self,
        pin: &str,
    ) -> Result<super::super::sanctuary_vault::SanctuaryLane, String> {
        super::super::sanctuary_vault::resolve_lane(&self.storage_root, pin)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn add_sanctuary_vault_note(
        &self,
        pin: &str,
        body: &str,
    ) -> Result<super::super::sanctuary_vault::SanctuaryLane, String> {
        super::super::sanctuary_vault::add_note(
            &self.storage_root,
            pin,
            body,
            Self::now_unix() as u32,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn list_sanctuary_vault_notes(
        &self,
        pin: &str,
    ) -> Result<
        (
            super::super::sanctuary_vault::SanctuaryLane,
            Vec<super::super::sanctuary_vault::SanctuaryVaultNote>,
        ),
        String,
    > {
        super::super::sanctuary_vault::list_notes(&self.storage_root, pin)
    }
}
