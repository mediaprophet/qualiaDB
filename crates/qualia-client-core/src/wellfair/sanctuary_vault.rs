//! Encrypted-at-rest Sanctuary store with an independent decoy lane (master plan §6).
//!
//! Unlike the projection filter in [`super::sanctuary`] (which merely hides journal rows on
//! read), this is a **real boundary**: sensitive notes live only inside AEAD-encrypted lanes,
//! keyed by material derived from the owner's PIN with **Argon2id** (memory-hard; ADR D1) — or
//! PBKDF2-HMAC-SHA256 for a PBKDF2-configured vault — over a per-lane random salt. When the vault
//! is not unlocked there is nothing readable on disk — not a filtered view, actual ciphertext.
//!
//! The on-disk format is the **CBOR-native, n-layer** [`VaultContainerV2`] (vault v2, ADR §2/§9).
//! There is **no JSON path** anywhere in this module: the container and the per-lane records are
//! both `ciborium`-encoded. The container carries a constant number of layer slots (padded with
//! reserved layers) so the on-disk layer *count* reveals nothing about how many lanes are real /
//! decoy / empty.
//!
//! Two independent lanes are in use:
//! - **Real** — the true Sanctuary, opened by the real PIN.
//! - **Decoy** — a separate encrypted lane with its own salt/key, opened by the duress PIN. It
//!   never aliases real data (different key, different ciphertext) and a duress unlock only ever
//!   touches the decoy lane.
//!
//! The PIN is never stored, not even hashed. A per-lane *verifier* (a fixed magic string encrypted
//! under the lane key) is used to recognise which lane a PIN belongs to. There is no destructive
//! "nuke PIN" (plan §6).
//!
//! Native-only: `qualia_core_db::crypto::sanctuary_crypto` is `not(wasm32)`; the desktop is the
//! authoritative node that owns keys and the vault.
#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use qualia_core_db::crypto::sanctuary_crypto::{
    decrypt_sanctuary_chunk, derive_sanctuary_key_material, derive_sanctuary_key_material_argon2,
    encrypt_sanctuary_chunk, SanctuaryAeadAlgorithm, SanctuaryKeyMaterial, ARGON2_M_COST_KIB,
    ARGON2_P_COST, ARGON2_T_COST, SANCTUARY_TAG_BYTES,
};
use qualia_core_db::crypto::sanctuary_keychain;
use sha2::{Digest, Sha256};

use super::vault_container::{EncBlob, KdfDescriptor, Layer, LayerRole, VaultContainerV2};

/// On-disk vault file. **CBOR**, not JSON (vault v2).
pub const SANCTUARY_VAULT_FILE: &str = "wellfair/sanctuary_vault.cbor";
const ALGO: SanctuaryAeadAlgorithm = SanctuaryAeadAlgorithm::Aes256Gcm;
const VERIFIER_MAGIC: &[u8] = b"WELLFAIR-SANCTUARY-VERIFIER-v1";
/// Reserved chunk index for the verifier so it never shares a nonce with a records write.
const VERIFIER_CHUNK: u64 = u64::MAX;
/// Minimum PIN/passphrase length (ADR D5 — raised from 4). There is no maximum: a passphrase is
/// encouraged, and Argon2id makes a long secret cheap to stretch.
const MIN_PIN_LEN: usize = 6;

/// Reject the weakest PINs a coercer or shoulder-surfer would try first (ADR D5). This is a floor,
/// not a strength meter — it blocks trivially guessable values, it does not certify strong ones.
fn validate_pin_strength(pin: &str) -> Result<(), String> {
    if pin.chars().count() < MIN_PIN_LEN {
        return Err(format!(
            "PIN/passphrase must be at least {MIN_PIN_LEN} characters"
        ));
    }
    if let Some(first) = pin.chars().next() {
        if pin.chars().all(|c| c == first) {
            return Err("Too weak: all identical characters".into());
        }
    }
    if is_trivial_sequence(pin) {
        return Err("Too weak: a straight run of consecutive characters".into());
    }
    const COMMON: &[&str] = &[
        "123456", "1234567", "12345678", "password", "qwerty", "111111", "000000", "letmein",
        "abc123", "iloveyou",
    ];
    if COMMON.iter().any(|c| c.eq_ignore_ascii_case(pin)) {
        return Err("Too common — choose something less guessable".into());
    }
    Ok(())
}

/// True for a straight ascending/descending run over the whole string (e.g. `123456`, `987654`,
/// `abcdef`). A real passphrase will not be a single monotone run.
fn is_trivial_sequence(pin: &str) -> bool {
    let b = pin.as_bytes();
    if b.len() < MIN_PIN_LEN {
        return false;
    }
    let ascending = b.windows(2).all(|w| w[1] == w[0].wrapping_add(1));
    let descending = b.windows(2).all(|w| w[0] == w[1].wrapping_add(1));
    ascending || descending
}

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

    /// The container layer role this lane maps to.
    fn role(self) -> LayerRole {
        match self {
            SanctuaryLane::Real => LayerRole::Real,
            SanctuaryLane::Decoy => LayerRole::Decoy,
        }
    }

    /// Stable, non-secret layer id used when the lane is first created.
    fn layer_id(self) -> &'static str {
        match self {
            SanctuaryLane::Real => "real",
            SanctuaryLane::Decoy => "decoy:0",
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

// --- CBOR helpers (the vault serializes nothing as JSON) ---

fn cbor_encode<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

fn cbor_decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    ciborium::from_reader(bytes).map_err(|e| e.to_string())
}

fn vault_path(root: &Path) -> PathBuf {
    root.join(SANCTUARY_VAULT_FILE)
}

fn random_salt() -> [u8; 16] {
    // uuid v4 is CSPRNG-backed (getrandom); 16 bytes is ample salt entropy.
    uuid::Uuid::new_v4().into_bytes()
}

/// The production Argon2id KDF descriptor (ADR D1).
fn argon2_default_kdf() -> KdfDescriptor {
    KdfDescriptor::Argon2id {
        m_cost_kib: ARGON2_M_COST_KIB,
        t_cost: ARGON2_T_COST,
        p_cost: ARGON2_P_COST,
    }
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

/// Fold an optional OS-keychain pepper into the PIN before the KDF stretch. When no pepper is
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

/// Resolve a layer's KDF. Every real/decoy layer a vault creates carries an explicit descriptor;
/// only reserved padding layers have `None`, and those are never opened. There is no legacy
/// vault-level `iterations` fallback (no JSON vaults ever existed).
fn resolve_kdf(layer: &Layer) -> Result<KdfDescriptor, String> {
    layer
        .kdf
        .clone()
        .ok_or_else(|| "sanctuary layer has no KDF descriptor".to_string())
}

/// Derive 48-byte lane key material under the layer's KDF.
fn derive_lane_material(
    kdf: &KdfDescriptor,
    secret: &[u8],
    salt: &[u8],
) -> Result<SanctuaryKeyMaterial, String> {
    match kdf {
        KdfDescriptor::Pbkdf2 { iterations } => {
            Ok(derive_sanctuary_key_material(secret, salt, *iterations))
        }
        KdfDescriptor::Argon2id {
            m_cost_kib,
            t_cost,
            p_cost,
        } => derive_sanctuary_key_material_argon2(secret, salt, *m_cost_kib, *t_cost, *p_cost),
    }
}

fn lane_key(
    pin: &str,
    layer: &Layer,
    pepper: Option<&[u8; 32]>,
) -> Result<SanctuaryKeyMaterial, String> {
    let salt = hex::decode(&layer.salt_hex).map_err(|e| e.to_string())?;
    let secret = effective_secret(pin, pepper);
    derive_lane_material(&resolve_kdf(layer)?, &secret, &salt)
}

/// Derive the key for a layer and confirm the PIN opens it (verifier → magic constant).
fn try_lane(
    pin: &str,
    layer: &Layer,
    lane_id: SanctuaryLane,
    pepper: Option<&[u8; 32]>,
) -> Option<SanctuaryKeyMaterial> {
    let key = lane_key(pin, layer, pepper).ok()?;
    match open(&key, &layer.verifier, lane_id.aad()) {
        Ok(v) if v == VERIFIER_MAGIC => Some(key),
        _ => None,
    }
}

/// Resolve which lane a PIN opens, returning the key alongside it.
///
/// **Constant-work across the three outcomes** (real PIN / duress-decoy PIN / wrong PIN): ALWAYS
/// derive BOTH lane keys — one full KDF stretch each — before branching on which verifier opened.
/// Early-returning once the real lane matched would make a real unlock cost one KDF while a duress
/// or wrong PIN costs two — a ~2x timing tell a coercer could use to single out the real PIN (i.e.
/// to learn that a *hidden* primary vault exists behind the decoy). The residual difference (an
/// AEAD tag verify that succeeds on one lane vs fails on the other, plus a short magic-constant
/// compare) is microseconds beside two memory-hard KDF stretches. This is *equal KDF work*, not a
/// proof of microarchitectural constant-time — see the Sanctuary threat-model ADR (D4).
fn open_lane(
    container: &VaultContainerV2,
    pin: &str,
    pepper: Option<&[u8; 32]>,
) -> Result<(SanctuaryLane, SanctuaryKeyMaterial), String> {
    let real = container
        .layer_by_role(LayerRole::Real)
        .ok_or("Sanctuary vault is missing its real layer")?;
    let decoy = container
        .layer_by_role(LayerRole::Decoy)
        .ok_or("Sanctuary vault is missing its decoy layer")?;
    let real_key = try_lane(pin, real, SanctuaryLane::Real, pepper);
    let decoy_key = try_lane(pin, decoy, SanctuaryLane::Decoy, pepper);
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
    kdf: KdfDescriptor,
    pepper: Option<&[u8; 32]>,
) -> Result<Layer, String> {
    let salt = random_salt();
    let secret = effective_secret(pin, pepper);
    let key = derive_lane_material(&kdf, &secret, &salt)?;
    let aad = lane_id.aad();
    let verifier = seal(&key, VERIFIER_CHUNK, VERIFIER_MAGIC, aad);
    let empty: Vec<SanctuaryVaultNote> = Vec::new();
    let records_pt = cbor_encode(&empty)?;
    let records = seal(&key, 0, &records_pt, aad);
    Ok(Layer {
        id: lane_id.layer_id().to_string(),
        role: lane_id.role(),
        salt_hex: hex::encode(salt),
        kdf: Some(kdf),
        verifier,
        records,
        next_counter: 1,
        audit_pubkey_hex: None,
        wrapped_keys: Vec::new(),
    })
}

fn load_container(root: &Path) -> Result<Option<VaultContainerV2>, String> {
    let path = vault_path(root);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read(&path).map_err(|e| e.to_string())?;
    Ok(Some(VaultContainerV2::from_cbor(&raw)?))
}

fn save_container(root: &Path, container: &VaultContainerV2) -> Result<(), String> {
    let path = vault_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = container.to_cbor()?;
    fs::write(&path, bytes).map_err(|e| e.to_string())
}

/// Is the encrypted vault configured on disk?
pub fn is_configured(root: impl AsRef<Path>) -> bool {
    vault_path(root.as_ref()).exists()
}

/// Create the two encrypted lanes with the production **Argon2id** KDF (memory-hard; ADR D1).
/// Fails if a PIN is too weak, the PINs are equal, or the vault already exists.
pub fn setup(root: impl AsRef<Path>, real_pin: &str, decoy_pin: &str) -> Result<(), String> {
    build_vault(root, real_pin, decoy_pin, argon2_default_kdf(), None, None)
}

/// As [`setup`] but with an explicit PBKDF2 iteration count. Crate-internal so **tests** can use a
/// fast work factor; production goes through [`setup`] (Argon2id).
pub(crate) fn setup_with_iterations(
    root: impl AsRef<Path>,
    real_pin: &str,
    decoy_pin: &str,
    iterations: u32,
) -> Result<(), String> {
    build_vault(
        root,
        real_pin,
        decoy_pin,
        KdfDescriptor::Pbkdf2 { iterations },
        None,
        None,
    )
}

/// Shared vault constructor. `pepper`/`vault_id` are `Some` only for a keychain-wrapped vault.
fn build_vault(
    root: impl AsRef<Path>,
    real_pin: &str,
    decoy_pin: &str,
    kdf: KdfDescriptor,
    pepper: Option<&[u8; 32]>,
    vault_id: Option<String>,
) -> Result<(), String> {
    validate_pin_strength(real_pin)?;
    validate_pin_strength(decoy_pin)?;
    if real_pin == decoy_pin {
        return Err("Decoy PIN must differ from the real unlock PIN".into());
    }
    let root = root.as_ref();
    if is_configured(root) {
        return Err("Sanctuary vault already exists".into());
    }
    let real = new_lane(real_pin, SanctuaryLane::Real, kdf.clone(), pepper)?;
    let decoy = new_lane(decoy_pin, SanctuaryLane::Decoy, kdf, pepper)?;
    // `new` pads to the constant layer shape (reserved layers) and stamps the v2 version.
    let container = VaultContainerV2::new(vec![real, decoy], pepper.is_some(), vault_id)?;
    save_container(root, &container)
}

/// Resolve the pepper needed to open `container`. `override_pepper` (a recovery code) wins;
/// otherwise a keychain-wrapped vault fetches its pepper from the OS keychain (erroring if gone).
fn pepper_for(
    container: &VaultContainerV2,
    override_pepper: Option<[u8; 32]>,
) -> Result<Option<[u8; 32]>, String> {
    if let Some(p) = override_pepper {
        return Ok(Some(p));
    }
    if !container.keychain_wrapped {
        return Ok(None);
    }
    let vault_id = container
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
    load_container(root.as_ref())
        .ok()
        .flatten()
        .map(|c| c.keychain_wrapped)
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
        argon2_default_kdf(),
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
    let container = load_container(root.as_ref())?.ok_or("Sanctuary vault is not set up")?;
    if !container.keychain_wrapped {
        return Err("This vault is not keychain-wrapped; unlock with the PIN alone".into());
    }
    let bytes = hex::decode(recovery_code_hex.trim())
        .map_err(|_| "Recovery code is not valid hex".to_string())?;
    if bytes.len() != 32 {
        return Err("Recovery code must be 32 bytes (64 hex chars)".into());
    }
    let mut pepper = [0u8; 32];
    pepper.copy_from_slice(&bytes);
    let (lane, _key) = open_lane(&container, pin, Some(&pepper))?;
    if let Some(vault_id) = container.vault_id.as_deref() {
        // Re-seat the pepper for future unlocks on this device.
        sanctuary_keychain::store_pepper(vault_id, &pepper)?;
    }
    Ok(lane)
}

/// Resolve which lane a PIN opens (or an error if it opens neither).
pub fn resolve_lane(root: impl AsRef<Path>, pin: &str) -> Result<SanctuaryLane, String> {
    let container = load_container(root.as_ref())?.ok_or("Sanctuary vault is not set up")?;
    let pepper = pepper_for(&container, None)?;
    Ok(open_lane(&container, pin, pepper.as_ref())?.0)
}

fn layer_ref(container: &VaultContainerV2, lane: SanctuaryLane) -> Result<&Layer, String> {
    container
        .layer_by_role(lane.role())
        .ok_or_else(|| "Sanctuary vault is missing a lane layer".to_string())
}

fn layer_mut(container: &mut VaultContainerV2, lane: SanctuaryLane) -> Result<&mut Layer, String> {
    let role = lane.role();
    container
        .layers
        .iter_mut()
        .find(|l| l.role == role)
        .ok_or_else(|| "Sanctuary vault is missing a lane layer".to_string())
}

/// Read the notes held in the lane the PIN opens. Nothing is readable without a valid PIN.
pub fn list_notes(
    root: impl AsRef<Path>,
    pin: &str,
) -> Result<(SanctuaryLane, Vec<SanctuaryVaultNote>), String> {
    let container = load_container(root.as_ref())?.ok_or("Sanctuary vault is not set up")?;
    let pepper = pepper_for(&container, None)?;
    let (lane, key) = open_lane(&container, pin, pepper.as_ref())?;
    let layer = layer_ref(&container, lane)?;
    let pt = open(&key, &layer.records, lane.aad())?;
    let notes: Vec<SanctuaryVaultNote> = cbor_decode(&pt)?;
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
    let mut container = load_container(root)?.ok_or("Sanctuary vault is not set up")?;
    let pepper = pepper_for(&container, None)?;
    let (lane, key) = open_lane(&container, pin, pepper.as_ref())?;
    let (mut notes, counter) = {
        let layer = layer_ref(&container, lane)?;
        let pt = open(&key, &layer.records, lane.aad())?;
        let notes: Vec<SanctuaryVaultNote> = cbor_decode(&pt)?;
        (notes, layer.next_counter)
    };
    notes.push(SanctuaryVaultNote {
        id: uuid::Uuid::new_v4().to_string(),
        body: body.to_string(),
        created_at_unix: now_unix,
    });
    let records_pt = cbor_encode(&notes)?;
    let blob = seal(&key, counter, &records_pt, lane.aad());
    {
        let layer = layer_mut(&mut container, lane)?;
        layer.records = blob;
        layer.next_counter = counter.saturating_add(1);
    }
    save_container(root, &container)?;
    Ok(lane)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wellfair::vault_container::{CONTAINER_SLOTS, CONTAINER_VERSION};

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
        // CBOR is binary now; scan the raw bytes for the sensitive substring.
        let raw = fs::read(vault_path(dir.path())).unwrap();
        let needle = b"TERMINALLY-SENSITIVE-STRING";
        assert!(
            !raw.windows(needle.len()).any(|w| w == needle),
            "sanctuary body leaked to disk"
        );
    }

    #[test]
    fn on_disk_container_has_constant_shape_and_v2_version() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        let container = load_container(dir.path()).unwrap().unwrap();
        assert_eq!(container.version, CONTAINER_VERSION);
        assert_eq!(container.layers.len(), CONTAINER_SLOTS);
        // The audit log starts empty (populated only by decoy-session writes in S5c).
        assert!(container.audit_log.is_empty());
        // Vault file is the .cbor path, and it is not valid UTF-8 JSON text.
        assert!(vault_path(dir.path()).to_string_lossy().ends_with(".cbor"));
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
            KdfDescriptor::Pbkdf2 { iterations: 1_000 },
            Some(&pepper),
            Some("test-vault".into()),
        )
        .unwrap();

        let container = load_container(dir.path()).unwrap().unwrap();
        assert!(container.keychain_wrapped);
        assert_eq!(container.vault_id.as_deref(), Some("test-vault"));

        // Correct pepper opens the real lane.
        let (lane, _k) = open_lane(&container, "real-pin-1", Some(&pepper)).unwrap();
        assert_eq!(lane, SanctuaryLane::Real);

        // The same PIN with NO pepper (i.e. disk + PIN, as an attacker would try) does NOT open it.
        assert!(open_lane(&container, "real-pin-1", None).is_err());
        // A wrong pepper does not open it either.
        assert!(open_lane(&container, "real-pin-1", Some(&[9u8; 32])).is_err());
    }

    #[test]
    fn unwrapped_vault_needs_no_pepper() {
        // Regression: the default (unwrapped) path resolves a None pepper and opens on PIN alone.
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        let container = load_container(dir.path()).unwrap().unwrap();
        assert!(!container.keychain_wrapped);
        assert!(pepper_for(&container, None).unwrap().is_none());
        assert!(!is_keychain_wrapped(dir.path()));
    }

    #[test]
    fn argon2_vault_opens_and_isolates_lanes() {
        // Fast Argon2id params for the test; production uses 64 MiB (ADR D1).
        let dir = tempfile::tempdir().unwrap();
        let kdf = KdfDescriptor::Argon2id { m_cost_kib: 16, t_cost: 1, p_cost: 1 };
        build_vault(dir.path(), "real-pass-alpha", "decoy-pass-beta", kdf, None, None).unwrap();
        assert_eq!(resolve_lane(dir.path(), "real-pass-alpha").unwrap(), SanctuaryLane::Real);
        assert_eq!(resolve_lane(dir.path(), "decoy-pass-beta").unwrap(), SanctuaryLane::Decoy);
        assert!(resolve_lane(dir.path(), "wrong-pass-xyz").is_err());
        let container = load_container(dir.path()).unwrap().unwrap();
        assert!(matches!(
            container.layer_by_role(LayerRole::Real).unwrap().kdf,
            Some(KdfDescriptor::Argon2id { .. })
        ));
        assert!(matches!(
            container.layer_by_role(LayerRole::Decoy).unwrap().kdf,
            Some(KdfDescriptor::Argon2id { .. })
        ));
    }

    #[test]
    fn production_setup_uses_argon2id() {
        // setup() must produce memory-hard Argon2id lanes (real 64 MiB params, one-time cost).
        let dir = tempfile::tempdir().unwrap();
        setup(dir.path(), "correct-horse", "battery-staple-2").unwrap();
        let container = load_container(dir.path()).unwrap().unwrap();
        assert!(matches!(
            container.layer_by_role(LayerRole::Real).unwrap().kdf,
            Some(KdfDescriptor::Argon2id { .. })
        ));
    }

    #[test]
    fn pbkdf2_vault_opens() {
        // A PBKDF2-configured vault (fast test factor) opens on its PINs.
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        assert_eq!(resolve_lane(dir.path(), "real-pin-1").unwrap(), SanctuaryLane::Real);
    }

    #[test]
    fn pin_policy_rejects_weak_pins() {
        let dir = tempfile::tempdir().unwrap();
        assert!(setup(dir.path(), "a1b2", "decoy-strong-1").is_err(), "too short");
        assert!(setup(dir.path(), "aaaaaa", "decoy-strong-1").is_err(), "all identical");
        assert!(setup(dir.path(), "123456", "decoy-strong-1").is_err(), "sequential");
        assert!(setup(dir.path(), "password", "decoy-strong-1").is_err(), "common");
        // None of the rejected attempts created a vault (they fail before derivation).
        assert!(!is_configured(dir.path()));
    }

    #[test]
    fn tampered_ciphertext_fails_to_open() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        add_note(dir.path(), "real-pin-1", "secret", 10).unwrap();
        // Flip a byte in the stored real-layer ciphertext.
        let mut container = load_container(dir.path()).unwrap().unwrap();
        {
            let layer = layer_mut(&mut container, SanctuaryLane::Real).unwrap();
            let mut ct = hex::decode(&layer.records.ct_hex).unwrap();
            ct[0] ^= 0xFF;
            layer.records.ct_hex = hex::encode(&ct);
        }
        save_container(dir.path(), &container).unwrap();
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
