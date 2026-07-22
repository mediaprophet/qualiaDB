//! Vault v2 decoy audit + OS-keychain wrapping


use super::*;

impl WebizenHostApi {
    // --- Vault v2 (S6): per-session decoy audit, realâ†’decoy curation, real-lane audit review ---

    /// Add a note, attributing a **decoy** (duress) write to `session_ref` â€” a fresh ref per duress
    /// unlock yields the git-like per-session branch in the audit DAG (ADR Â§10). Real-lane writes
    /// ignore `session_ref` (real activity is never audited). The host should mint one `session_ref`
    /// per unlock (e.g. a UUID) and reuse it for every write in that session.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn add_sanctuary_vault_note_in_session(
        &self,
        pin: &str,
        body: &str,
        session_ref: &str,
    ) -> Result<super::super::sanctuary_vault::SanctuaryLane, String> {
        super::super::sanctuary_vault::add_note_in_session(
            &self.storage_root,
            pin,
            body,
            Self::now_unix() as u32,
            session_ref,
        )
    }

    /// **Curate the decoy from a real session (ADR Â§3.2).** Write a plausible note into the decoy
    /// lane *without* the decoy PIN, so a coercer's re-unlock shows fresh, believable content.
    /// Requires the **real** PIN; the decoy/wrong PIN is rejected.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn curate_sanctuary_decoy_note(&self, real_pin: &str, body: &str) -> Result<(), String> {
        super::super::sanctuary_vault::real_curate_decoy_add_note(
            &self.storage_root,
            real_pin,
            body,
            Self::now_unix() as u32,
        )
    }

    /// **Review decoy activity from the real lane (ADR Â§3.1 / Â§10).** Decrypts every sealed
    /// decoy-session record, verifies chain integrity + each witnessed-prefix head anchor, advances
    /// the anchors, and returns the decrypted actions with an integrity verdict. Requires the
    /// **real** PIN. `session_count` is a proxy for "number of attackers", never a hard head-count.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn review_sanctuary_decoy_activity(
        &self,
        real_pin: &str,
    ) -> Result<super::super::sanctuary_vault::DecoyActivityReport, String> {
        super::super::sanctuary_vault::review_decoy_activity(&self.storage_root, real_pin)
    }

    /// Read the decoy-audit retention policy (ADR Â§8). **Real-session only** â€” requires the real PIN;
    /// the setting is invisible/unreachable from a decoy session. Defaults to auto-archive.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_sanctuary_decoy_retention_mode(
        &self,
        real_pin: &str,
    ) -> Result<qualia_core_db::crypto::sanctuary_audit_dag::RetentionMode, String> {
        super::super::sanctuary_vault::get_retention_mode(&self.storage_root, real_pin)
    }

    /// Set the decoy-audit retention policy (ADR Â§8). **Real-session only** â€” requires the real PIN.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_sanctuary_decoy_retention_mode(
        &self,
        real_pin: &str,
        mode: qualia_core_db::crypto::sanctuary_audit_dag::RetentionMode,
    ) -> Result<(), String> {
        super::super::sanctuary_vault::set_retention_mode(&self.storage_root, real_pin, mode)
    }

    // --- T1.2: OS-keychain vault wrapping (opt-in, off by default; recovery-gated) ---

    /// Is the on-disk Sanctuary vault keychain-wrapped (bound to an OS-keychain pepper)?
    #[cfg(not(target_arch = "wasm32"))]
    pub fn sanctuary_vault_is_keychain_wrapped(&self) -> bool {
        super::super::sanctuary_vault::is_keychain_wrapped(&self.storage_root)
    }

    /// Opt-in: create the Sanctuary vault with an OS-keychain-held pepper so disk + PIN alone can't
    /// open it. Returns the hex **recovery code** the user MUST record â€” losing the keychain entry
    /// otherwise loses the vault. The ordinary [`Self::setup_sanctuary_vault`] path stays unwrapped.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn setup_sanctuary_vault_wrapped(
        &self,
        real_pin: &str,
        decoy_pin: &str,
    ) -> Result<String, String> {
        super::super::sanctuary_vault::setup_wrapped(&self.storage_root, real_pin, decoy_pin)
    }

    /// Recover a keychain-wrapped vault on a device whose keychain entry is missing, using the
    /// recovery code from [`Self::setup_sanctuary_vault_wrapped`]. Re-seats the pepper on success.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn sanctuary_vault_unlock_with_recovery(
        &self,
        pin: &str,
        recovery_code_hex: &str,
    ) -> Result<super::super::sanctuary_vault::SanctuaryLane, String> {
        super::super::sanctuary_vault::unlock_with_recovery(&self.storage_root, pin, recovery_code_hex)
    }

}