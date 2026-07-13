//! OS-keychain-held pepper for the Sanctuary vault (T1.2 — optional second factor).
//!
//! When a vault is **keychain-wrapped**, its PBKDF2 input is peppered with a 32-byte secret held
//! in the platform keychain (Windows Credential Manager / macOS Keychain / Linux Secret Service).
//! Disk + a guessed or weak PIN alone can then no longer open the vault — the machine's keychain is
//! also required.
//!
//! **Recovery model (why this is opt-in / off by default).** Losing the keychain entry — OS
//! reinstall, moving to a new machine, credential-store reset — makes the vault unopenable *unless*
//! the one-time **recovery code** (the hex pepper handed back when wrapping is enabled) is supplied.
//! Enabling wrapping is therefore a deliberate, recovery-aware choice; the default vault is
//! unwrapped and unchanged.
//!
//! This module owns only the keychain I/O. The pepper-mixing itself lives in the vault layer
//! (`qualia-client-core::wellfair::sanctuary_vault`) and is hermetically testable without touching
//! the real OS keychain.

const SERVICE: &str = "qualia_db_sanctuary";

fn entry(vault_id: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, &format!("pepper_{vault_id}"))
        .map_err(|e| format!("Keyring error: {e}"))
}

/// Generate a fresh 32-byte pepper from the OS CSPRNG.
pub fn generate_pepper() -> Result<[u8; 32], String> {
    let mut pepper = [0u8; 32];
    getrandom::fill(&mut pepper).map_err(|e| format!("OS RNG failed: {e}"))?;
    Ok(pepper)
}

/// Store (or overwrite) the pepper for `vault_id` in the OS keychain.
pub fn store_pepper(vault_id: &str, pepper: &[u8; 32]) -> Result<(), String> {
    entry(vault_id)?
        .set_password(&hex::encode(pepper))
        .map_err(|e| format!("Keyring store failed: {e}"))
}

/// Read the pepper for `vault_id`. `Ok(None)` means no entry exists on this device (the caller
/// then falls back to the recovery code).
pub fn get_pepper(vault_id: &str) -> Result<Option<[u8; 32]>, String> {
    match entry(vault_id)?.get_password() {
        Ok(hex_str) => {
            let bytes = hex::decode(hex_str).map_err(|e| format!("Invalid pepper hex: {e}"))?;
            if bytes.len() != 32 {
                return Err("Corrupted pepper length in keychain".into());
            }
            let mut pepper = [0u8; 32];
            pepper.copy_from_slice(&bytes);
            Ok(Some(pepper))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Keyring read failed: {e}")),
    }
}

/// Remove the pepper for `vault_id` (idempotent — a missing entry is not an error).
pub fn delete_pepper(vault_id: &str) -> Result<(), String> {
    match entry(vault_id)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Keyring delete failed: {e}")),
    }
}
