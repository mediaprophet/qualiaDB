//! Encrypted-at-rest Sanctuary store with an independent decoy lane (master plan §6).
//!
//! Unlike the projection filter in [`super::sanctuary`] (which merely hides journal rows on
//! read), this is a **real boundary**: sensitive notes live only inside AEAD-encrypted lane
//! files, keyed by material derived from the owner's PIN with PBKDF2-HMAC-SHA256 (310k
//! iterations) over a per-lane random salt. When the vault is not unlocked there is nothing
//! readable on disk — not a filtered view, actual ciphertext.
//!
//! Two independent lanes exist:
//! - **Real** — the true Sanctuary, opened by the real PIN.
//! - **Decoy** — a separate encrypted store with its own salt/key, opened by the duress PIN.
//!   It never aliases real data (different key, different ciphertext) and a duress unlock only
//!   ever touches the decoy lane.
//!
//! The PIN is never stored, not even hashed. A per-lane *verifier* (a fixed magic string
//! encrypted under the lane key) is used to recognise which lane a PIN belongs to. There is no
//! destructive "nuke PIN" (plan §6).
//!
//! Native-only: `qualia_core_db::crypto::sanctuary_crypto` is `not(wasm32)`; the desktop is the
//! authoritative node that owns keys and the vault.
#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use qualia_core_db::crypto::sanctuary_crypto::{
    decrypt_sanctuary_chunk, derive_sanctuary_key_material, encrypt_sanctuary_chunk,
    SanctuaryAeadAlgorithm, SanctuaryKeyMaterial, DEFAULT_PBKDF2_ITERATIONS, SANCTUARY_TAG_BYTES,
};
use qualia_core_db::crypto::sanctuary_keychain;
use sha2::{Digest, Sha256};

pub const SANCTUARY_VAULT_FILE: &str = "wellfair/sanctuary_vault.json";
const ALGO: SanctuaryAeadAlgorithm = SanctuaryAeadAlgorithm::Aes256Gcm;
const VERIFIER_MAGIC: &[u8] = b"WELLFAIR-SANCTUARY-VERIFIER-v1";
/// Reserved chunk index for the verifier so it never shares a nonce with a records write.
const VERIFIER_CHUNK: u64 = u64::MAX;
const MIN_PIN_LEN: usize = 4;

/// Which lane a PIN opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SanctuaryLane {
    Real,
    Decoy,
}

impl SanctuaryLane {
    fn aad(self) -> &'static [u8] {
        match self {
            SanctuaryLane::Real => b"wellfair:sanctuary:real",
            SanctuaryLane::Decoy => b"wellfair:sanctuary:decoy",
        }
    }
}

/// A sensitive note held only inside the encrypted vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanctuaryVaultNote {
    pub id: String,
    pub body: String,
    pub created_at_unix: u32,
}

/// An AEAD ciphertext blob (hex-encoded) plus the chunk index used to derive its nonce.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct EncBlob {
    chunk_index: u64,
    ct_hex: String,
    tag_hex: String,
}

/// Per-lane persisted state: salt, PIN verifier, encrypted records, and the next nonce counter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LaneState {
    salt_hex: String,
    verifier: EncBlob,
    records: EncBlob,
    next_counter: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct VaultMeta {
    version: u16,
    iterations: u32,
    real: LaneState,
    decoy: LaneState,
    /// T1.2: when true, the KDF input is peppered with an OS-keychain-held secret keyed by
    /// `vault_id`. Defaults false so existing (unwrapped) vaults deserialize unchanged.
    #[serde(default)]
    keychain_wrapped: bool,
    /// Stable, non-secret keychain lookup id (path-independent). `Some` only when wrapped.
    #[serde(default)]
    vault_id: Option<String>,
}

fn vault_path(root: &Path) -> PathBuf {
    root.join(SANCTUARY_VAULT_FILE)
}

fn random_salt() -> [u8; 16] {
    // uuid v4 is CSPRNG-backed (getrandom); 16 bytes is ample salt entropy.
    uuid::Uuid::new_v4().into_bytes()
}

fn seal(key: &SanctuaryKeyMaterial, chunk: u64, plaintext: &[u8], aad: &[u8]) -> EncBlob {
    let mut ct = vec![0u8; plaintext.len()];
    let mut tag = [0u8; SANCTUARY_TAG_BYTES];
    // AES-256-GCM over caller buffers; deterministic nonce from (tweak, chunk) — chunk is a
    // monotonic counter here, so nonces never repeat under one key.
    encrypt_sanctuary_chunk(ALGO, key, chunk, plaintext, &mut ct, &mut tag, aad)
        .expect("aead encrypt");
    EncBlob {
        chunk_index: chunk,
        ct_hex: hex::encode(&ct),
        tag_hex: hex::encode(tag),
    }
}

fn open(key: &SanctuaryKeyMaterial, blob: &EncBlob, aad: &[u8]) -> Result<Vec<u8>, String> {
    let ct = hex::decode(&blob.ct_hex).map_err(|e| e.to_string())?;
    let tag_bytes = hex::decode(&blob.tag_hex).map_err(|e| e.to_string())?;
    if tag_bytes.len() != SANCTUARY_TAG_BYTES {
        return Err("corrupt sanctuary tag".into());
    }
    let mut tag = [0u8; SANCTUARY_TAG_BYTES];
    tag.copy_from_slice(&tag_bytes);
    let mut pt = vec![0u8; ct.len()];
    decrypt_sanctuary_chunk(ALGO, key, blob.chunk_index, &ct, &tag, &mut pt, aad)
        .map_err(|_| "sanctuary decryption failed (wrong PIN or tampered vault)".to_string())?;
    Ok(pt)
}

/// Fold an optional OS-keychain pepper into the PIN before the PBKDF2 stretch. When no pepper is
/// present (the default, unwrapped vault) the PIN bytes are used verbatim — so unwrapped vaults
/// derive exactly as before. A pepper domain-separates and binds the derivation to the keychain
/// secret: disk + PIN alone cannot reproduce the key.
fn effective_secret(pin: &str, pepper: Option<&[u8; 32]>) -> Vec<u8> {
    match pepper {
        None => pin.as_bytes().to_vec(),
        Some(p) => {
            let mut h = Sha256::new();
            h.update(b"q42:sanctuary:pepper:v1");
            h.update(p);
            h.update(pin.as_bytes());
            h.finalize().to_vec()
        }
    }
}

fn lane_key(
    pin: &str,
    lane: &LaneState,
    iterations: u32,
    pepper: Option<&[u8; 32]>,
) -> Result<SanctuaryKeyMaterial, String> {
    let salt = hex::decode(&lane.salt_hex).map_err(|e| e.to_string())?;
    let secret = effective_secret(pin, pepper);
    Ok(derive_sanctuary_key_material(&secret, &salt, iterations))
}

/// Derive the key for `lane_id` and confirm the PIN opens it (verifier → magic constant).
fn try_lane(
    pin: &str,
    lane: &LaneState,
    lane_id: SanctuaryLane,
    iterations: u32,
    pepper: Option<&[u8; 32]>,
) -> Option<SanctuaryKeyMaterial> {
    let key = lane_key(pin, lane, iterations, pepper).ok()?;
    match open(&key, &lane.verifier, lane_id.aad()) {
        Ok(v) if v == VERIFIER_MAGIC => Some(key),
        _ => None,
    }
}

/// Resolve which lane a PIN opens, returning the key alongside it.
///
/// **Constant-work across the three outcomes** (real PIN / duress-decoy PIN / wrong PIN): ALWAYS
/// derive BOTH lane keys — one full PBKDF2-310k stretch each — before branching on which verifier
/// opened. Early-returning once the real lane matched would make a real unlock cost one KDF while a
/// duress or wrong PIN costs two — a ~2x timing tell a coercer could use to single out the real PIN
/// (i.e. to learn that a *hidden* primary vault exists behind the decoy). The residual difference
/// (an AEAD tag verify that succeeds on one lane vs fails on the other, plus a short magic-constant
/// compare) is microseconds beside two 310k-iteration PBKDF2 stretches. This is *equal KDF work*,
/// not a proof of microarchitectural constant-time — see the Sanctuary threat-model ADR (D4).
fn open_lane(
    meta: &VaultMeta,
    pin: &str,
    pepper: Option<&[u8; 32]>,
) -> Result<(SanctuaryLane, SanctuaryKeyMaterial), String> {
    let real_key = try_lane(pin, &meta.real, SanctuaryLane::Real, meta.iterations, pepper);
    let decoy_key = try_lane(pin, &meta.decoy, SanctuaryLane::Decoy, meta.iterations, pepper);
    if let Some(key) = real_key {
        return Ok((SanctuaryLane::Real, key));
    }
    if let Some(key) = decoy_key {
        return Ok((SanctuaryLane::Decoy, key));
    }
    Err("Incorrect PIN".into())
}

fn new_lane(
    pin: &str,
    lane_id: SanctuaryLane,
    iterations: u32,
    pepper: Option<&[u8; 32]>,
) -> LaneState {
    let salt = random_salt();
    let secret = effective_secret(pin, pepper);
    let key = derive_sanctuary_key_material(&secret, &salt, iterations);
    let aad = lane_id.aad();
    let verifier = seal(&key, VERIFIER_CHUNK, VERIFIER_MAGIC, aad);
    let empty: Vec<SanctuaryVaultNote> = Vec::new();
    let records_pt = serde_json::to_vec(&empty).expect("serialize empty records");
    let records = seal(&key, 0, &records_pt, aad);
    LaneState {
        salt_hex: hex::encode(salt),
        verifier,
        records,
        next_counter: 1,
    }
}

fn load_meta(root: &Path) -> Result<Option<VaultMeta>, String> {
    let path = vault_path(root);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let meta: VaultMeta = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    Ok(Some(meta))
}

fn save_meta(root: &Path, meta: &VaultMeta) -> Result<(), String> {
    let path = vault_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

/// Is the encrypted vault configured on disk?
pub fn is_configured(root: impl AsRef<Path>) -> bool {
    vault_path(root.as_ref()).exists()
}

/// Create the two encrypted lanes at the production PBKDF2 work factor.
/// Fails if PINs are too short, equal, or the vault already exists.
pub fn setup(root: impl AsRef<Path>, real_pin: &str, decoy_pin: &str) -> Result<(), String> {
    setup_with_iterations(root, real_pin, decoy_pin, DEFAULT_PBKDF2_ITERATIONS)
}

/// As [`setup`] but with an explicit PBKDF2 iteration count. Public within the crate so tests
/// can use a low work factor; production always goes through [`setup`] (310k iterations).
pub(crate) fn setup_with_iterations(
    root: impl AsRef<Path>,
    real_pin: &str,
    decoy_pin: &str,
    iterations: u32,
) -> Result<(), String> {
    build_vault(root, real_pin, decoy_pin, iterations, None, None)
}

/// Shared vault constructor. `pepper`/`vault_id` are `Some` only for a keychain-wrapped vault.
fn build_vault(
    root: impl AsRef<Path>,
    real_pin: &str,
    decoy_pin: &str,
    iterations: u32,
    pepper: Option<&[u8; 32]>,
    vault_id: Option<String>,
) -> Result<(), String> {
    if real_pin.len() < MIN_PIN_LEN || decoy_pin.len() < MIN_PIN_LEN {
        return Err(format!("PIN must be at least {MIN_PIN_LEN} characters"));
    }
    if real_pin == decoy_pin {
        return Err("Decoy PIN must differ from the real unlock PIN".into());
    }
    let root = root.as_ref();
    if is_configured(root) {
        return Err("Sanctuary vault already exists".into());
    }
    let meta = VaultMeta {
        version: 1,
        iterations,
        real: new_lane(real_pin, SanctuaryLane::Real, iterations, pepper),
        decoy: new_lane(decoy_pin, SanctuaryLane::Decoy, iterations, pepper),
        keychain_wrapped: pepper.is_some(),
        vault_id,
    };
    save_meta(root, &meta)
}

/// Resolve the pepper needed to open `meta`. `override_pepper` (a recovery code) wins; otherwise a
/// keychain-wrapped vault fetches its pepper from the OS keychain (erroring if the entry is gone).
fn pepper_for(
    meta: &VaultMeta,
    override_pepper: Option<[u8; 32]>,
) -> Result<Option<[u8; 32]>, String> {
    if let Some(p) = override_pepper {
        return Ok(Some(p));
    }
    if !meta.keychain_wrapped {
        return Ok(None);
    }
    let vault_id = meta
        .vault_id
        .as_deref()
        .ok_or("Keychain-wrapped vault is missing its vault_id")?;
    sanctuary_keychain::get_pepper(vault_id)?
        .map(Some)
        .ok_or_else(|| {
            "Sanctuary keychain secret not found on this device — supply the recovery code".into()
        })
}

/// Is the on-disk vault keychain-wrapped (T1.2)?
pub fn is_keychain_wrapped(root: impl AsRef<Path>) -> bool {
    load_meta(root.as_ref())
        .ok()
        .flatten()
        .map(|m| m.keychain_wrapped)
        .unwrap_or(false)
}

/// **Opt-in (off by default).** Create the two encrypted lanes with an OS-keychain-held pepper mixed
/// into the KDF, so disk + PIN alone cannot open the vault. Returns the pepper as a hex **recovery
/// code**: the caller MUST have the user record it out-of-band — if the keychain entry is later lost
/// (reinstall / new machine), this code is the only way back in (see [`unlock_with_recovery`]).
///
/// Enabling this is a deliberate, recovery-aware choice; the ordinary [`setup`] path stays unwrapped.
pub fn setup_wrapped(
    root: impl AsRef<Path>,
    real_pin: &str,
    decoy_pin: &str,
) -> Result<String, String> {
    let pepper = sanctuary_keychain::generate_pepper()?;
    let vault_id = uuid::Uuid::new_v4().to_string();
    // Store the pepper first; if vault construction then fails, roll the keychain entry back.
    sanctuary_keychain::store_pepper(&vault_id, &pepper)?;
    match build_vault(
        root,
        real_pin,
        decoy_pin,
        DEFAULT_PBKDF2_ITERATIONS,
        Some(&pepper),
        Some(vault_id.clone()),
    ) {
        Ok(()) => Ok(hex::encode(pepper)),
        Err(e) => {
            let _ = sanctuary_keychain::delete_pepper(&vault_id);
            Err(e)
        }
    }
}

/// Recover access to a keychain-wrapped vault whose keychain entry is missing (new device, OS
/// reinstall) by supplying the hex recovery code from [`setup_wrapped`]. On success the pepper is
/// re-stored into this device's keychain so subsequent unlocks are seamless again.
pub fn unlock_with_recovery(
    root: impl AsRef<Path>,
    pin: &str,
    recovery_code_hex: &str,
) -> Result<SanctuaryLane, String> {
    let meta = load_meta(root.as_ref())?.ok_or("Sanctuary vault is not set up")?;
    if !meta.keychain_wrapped {
        return Err("This vault is not keychain-wrapped; unlock with the PIN alone".into());
    }
    let bytes = hex::decode(recovery_code_hex.trim())
        .map_err(|_| "Recovery code is not valid hex".to_string())?;
    if bytes.len() != 32 {
        return Err("Recovery code must be 32 bytes (64 hex chars)".into());
    }
    let mut pepper = [0u8; 32];
    pepper.copy_from_slice(&bytes);
    let (lane, _key) = open_lane(&meta, pin, Some(&pepper))?;
    if let Some(vault_id) = meta.vault_id.as_deref() {
        // Re-seat the pepper for future unlocks on this device.
        sanctuary_keychain::store_pepper(vault_id, &pepper)?;
    }
    Ok(lane)
}

/// Resolve which lane a PIN opens (or an error if it opens neither).
pub fn resolve_lane(root: impl AsRef<Path>, pin: &str) -> Result<SanctuaryLane, String> {
    let meta = load_meta(root.as_ref())?.ok_or("Sanctuary vault is not set up")?;
    let pepper = pepper_for(&meta, None)?;
    Ok(open_lane(&meta, pin, pepper.as_ref())?.0)
}

fn lane_ref<'a>(meta: &'a VaultMeta, lane: SanctuaryLane) -> &'a LaneState {
    match lane {
        SanctuaryLane::Real => &meta.real,
        SanctuaryLane::Decoy => &meta.decoy,
    }
}

fn lane_mut(meta: &mut VaultMeta, lane: SanctuaryLane) -> &mut LaneState {
    match lane {
        SanctuaryLane::Real => &mut meta.real,
        SanctuaryLane::Decoy => &mut meta.decoy,
    }
}

/// Read the notes held in the lane the PIN opens. Nothing is readable without a valid PIN.
pub fn list_notes(root: impl AsRef<Path>, pin: &str) -> Result<(SanctuaryLane, Vec<SanctuaryVaultNote>), String> {
    let meta = load_meta(root.as_ref())?.ok_or("Sanctuary vault is not set up")?;
    let pepper = pepper_for(&meta, None)?;
    let (lane, key) = open_lane(&meta, pin, pepper.as_ref())?;
    let state = lane_ref(&meta, lane);
    let pt = open(&key, &state.records, lane.aad())?;
    let notes: Vec<SanctuaryVaultNote> = serde_json::from_slice(&pt).map_err(|e| e.to_string())?;
    Ok((lane, notes))
}

/// Append a note to the lane the PIN opens (real PIN → real lane; duress PIN → decoy only).
pub fn add_note(
    root: impl AsRef<Path>,
    pin: &str,
    body: &str,
    now_unix: u32,
) -> Result<SanctuaryLane, String> {
    let root = root.as_ref();
    let mut meta = load_meta(root)?.ok_or("Sanctuary vault is not set up")?;
    let pepper = pepper_for(&meta, None)?;
    let (lane, key) = open_lane(&meta, pin, pepper.as_ref())?;
    let (mut notes, counter) = {
        let state = lane_ref(&meta, lane);
        let pt = open(&key, &state.records, lane.aad())?;
        let notes: Vec<SanctuaryVaultNote> =
            serde_json::from_slice(&pt).map_err(|e| e.to_string())?;
        (notes, state.next_counter)
    };
    notes.push(SanctuaryVaultNote {
        id: uuid::Uuid::new_v4().to_string(),
        body: body.to_string(),
        created_at_unix: now_unix,
    });
    let records_pt = serde_json::to_vec(&notes).map_err(|e| e.to_string())?;
    let blob = seal(&key, counter, &records_pt, lane.aad());
    {
        let state = lane_mut(&mut meta, lane);
        state.records = blob;
        state.next_counter = counter.saturating_add(1);
    }
    save_meta(root, &meta)?;
    Ok(lane)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_rejects_short_or_equal_pins() {
        let dir = tempfile::tempdir().unwrap();
        assert!(setup(dir.path(), "abc", "decoy-pin").is_err());
        assert!(setup(dir.path(), "same-pin", "same-pin").is_err());
    }

    #[test]
    fn real_and_decoy_pins_open_distinct_lanes() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        assert_eq!(resolve_lane(dir.path(), "real-pin-1").unwrap(), SanctuaryLane::Real);
        assert_eq!(resolve_lane(dir.path(), "decoy-pin-2").unwrap(), SanctuaryLane::Decoy);
        assert!(resolve_lane(dir.path(), "wrong-pin").is_err());
    }

    #[test]
    fn notes_are_lane_isolated_and_decoy_never_sees_real() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();

        assert_eq!(add_note(dir.path(), "real-pin-1", "real secret", 10).unwrap(), SanctuaryLane::Real);
        assert_eq!(add_note(dir.path(), "decoy-pin-2", "decoy filler", 11).unwrap(), SanctuaryLane::Decoy);
        // A duress note must never land in the real lane.
        assert_eq!(add_note(dir.path(), "decoy-pin-2", "more decoy", 12).unwrap(), SanctuaryLane::Decoy);

        let (lane, real_notes) = list_notes(dir.path(), "real-pin-1").unwrap();
        assert_eq!(lane, SanctuaryLane::Real);
        assert_eq!(real_notes.len(), 1);
        assert_eq!(real_notes[0].body, "real secret");

        let (lane, decoy_notes) = list_notes(dir.path(), "decoy-pin-2").unwrap();
        assert_eq!(lane, SanctuaryLane::Decoy);
        assert_eq!(decoy_notes.len(), 2);
        assert!(decoy_notes.iter().all(|n| n.body != "real secret"));
    }

    #[test]
    fn plaintext_never_appears_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        add_note(dir.path(), "real-pin-1", "TERMINALLY-SENSITIVE-STRING", 10).unwrap();
        let raw = fs::read_to_string(vault_path(dir.path())).unwrap();
        assert!(!raw.contains("TERMINALLY-SENSITIVE-STRING"), "sanctuary body leaked to disk");
    }

    #[test]
    fn effective_secret_folds_pepper_deterministically() {
        // No pepper → PIN verbatim (unwrapped vaults derive exactly as before).
        assert_eq!(effective_secret("pin", None), b"pin".to_vec());
        // A pepper changes the derived secret, deterministically.
        let peppered = effective_secret("pin", Some(&[1u8; 32]));
        assert_ne!(peppered, b"pin".to_vec());
        assert_eq!(peppered, effective_secret("pin", Some(&[1u8; 32])));
        assert_ne!(peppered, effective_secret("pin", Some(&[2u8; 32])));
    }

    #[test]
    fn keychain_pepper_binds_the_vault_key() {
        // Hermetic: exercises the pepper-mixing directly via build_vault/open_lane, with NO real
        // OS-keychain I/O (setup_wrapped/unlock_with_recovery own that thin wrapper).
        let dir = tempfile::tempdir().unwrap();
        let pepper = [7u8; 32];
        build_vault(
            dir.path(),
            "real-pin-1",
            "decoy-pin-2",
            1_000,
            Some(&pepper),
            Some("test-vault".into()),
        )
        .unwrap();

        let meta = load_meta(dir.path()).unwrap().unwrap();
        assert!(meta.keychain_wrapped);
        assert_eq!(meta.vault_id.as_deref(), Some("test-vault"));

        // Correct pepper opens the real lane.
        let (lane, _k) = open_lane(&meta, "real-pin-1", Some(&pepper)).unwrap();
        assert_eq!(lane, SanctuaryLane::Real);

        // The same PIN with NO pepper (i.e. disk + PIN, as an attacker would try) does NOT open it.
        assert!(open_lane(&meta, "real-pin-1", None).is_err());
        // A wrong pepper does not open it either.
        assert!(open_lane(&meta, "real-pin-1", Some(&[9u8; 32])).is_err());
    }

    #[test]
    fn unwrapped_vault_needs_no_pepper() {
        // Regression: the default (unwrapped) path resolves a None pepper and opens on PIN alone.
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        let meta = load_meta(dir.path()).unwrap().unwrap();
        assert!(!meta.keychain_wrapped);
        assert!(pepper_for(&meta, None).unwrap().is_none());
        assert!(!is_keychain_wrapped(dir.path()));
    }

    #[test]
    fn tampered_ciphertext_fails_to_open() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        add_note(dir.path(), "real-pin-1", "secret", 10).unwrap();
        // Flip a byte in the stored ciphertext.
        let mut meta = load_meta(dir.path()).unwrap().unwrap();
        let mut ct = hex::decode(&meta.real.records.ct_hex).unwrap();
        ct[0] ^= 0xFF;
        meta.real.records.ct_hex = hex::encode(&ct);
        save_meta(dir.path(), &meta).unwrap();
        assert!(list_notes(dir.path(), "real-pin-1").is_err());
    }

    #[test]
    fn survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        add_note(dir.path(), "real-pin-1", "persisted secret", 10).unwrap();
        // Fresh calls re-read the file from disk (no in-memory session).
        let (_, notes) = list_notes(dir.path(), "real-pin-1").unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].body, "persisted secret");
    }
}
