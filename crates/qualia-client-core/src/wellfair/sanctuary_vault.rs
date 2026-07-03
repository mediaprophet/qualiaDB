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
use qualia_core_db::crypto::sanctuary_audit::{
    open_sealed, seal_to, unwrap_key, wrap_key, AuditKeypair, GENESIS_PARENT,
};
use qualia_core_db::crypto::sanctuary_audit_dag::{
    derive_sessions, verify_chain, AuditAction, AuditRecord, ChainStatus,
};
use qualia_core_db::crypto::sanctuary_keychain;
use sha2::{Digest, Sha256};

use super::vault_container::{
    EncBlob, KdfDescriptor, Layer, LayerRole, VaultContainerV2, WrappedKey,
};

/// On-disk vault file. **CBOR**, not JSON (vault v2).
pub const SANCTUARY_VAULT_FILE: &str = "wellfair/sanctuary_vault.cbor";
const ALGO: SanctuaryAeadAlgorithm = SanctuaryAeadAlgorithm::Aes256Gcm;
const VERIFIER_MAGIC: &[u8] = b"WELLFAIR-SANCTUARY-VERIFIER-v1";
/// Reserved chunk index for the verifier so it never shares a nonce with a records write.
const VERIFIER_CHUNK: u64 = u64::MAX;
/// Minimum PIN/passphrase length (ADR D5 — raised from 4). There is no maximum: a passphrase is
/// encouraged, and Argon2id makes a long secret cheap to stretch.
const MIN_PIN_LEN: usize = 6;

// --- Real→decoy key hierarchy (ADR §3.2 / §7 crypto note) ---
//
// At setup the real lane's *wrapping key* wraps (a) the decoy lane's 48-byte key material and (b)
// the audit secret. A real session can therefore reach *down* into the decoy (curate it) and read
// the audit log, without re-entering the decoy PIN — but the decoy lane holds nothing that unwraps
// toward real, so the hierarchy is strictly one-way.
/// Purpose tag for the wrapped decoy lane key stored on the real layer.
const WK_DECOY_LANE_KEY: &str = "decoy_lane_key";
/// Purpose tag for the wrapped audit secret stored on the real layer.
const WK_AUDIT_SECRET: &str = "audit_secret";
/// AAD binding the decoy-lane-key wrap to its purpose.
const WRAP_AAD_DECOY: &[u8] = b"q42:sanctuary:wrap:decoy-lane-key:v1";
/// AAD binding the audit-secret wrap to its purpose.
const WRAP_AAD_AUDIT: &[u8] = b"q42:sanctuary:wrap:audit-secret:v1";
/// Serialized length of `SanctuaryKeyMaterial` (32-byte cipher key ‖ 16-byte volume tweak).
const KEY_MATERIAL_BYTES: usize = 48;

// --- Blind audit channel (ADR §3.1 / §10) ---
/// Asserted actor DID for a decoy (duress) session. Unauthenticated by design: a coercer operates
/// as a natural person under the decoy-layer DID; the real lane treats it as "decoy session".
const DECOY_ACTOR_DID: &str = "did:qualia:sanctuary:decoy-session";
/// The branch a plain (non-session-aware) decoy write lands on. The session-aware
/// [`add_note_in_session`] opens a fresh branch per duress unlock; the plain [`add_note`] fallback
/// accumulates on this single branch (S6's host layer supplies real per-session refs).
const DEFAULT_DECOY_BRANCH: &str = "decoy:default";

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

/// A witnessed head anchor for one audit branch (ADR §10 — C's finding). Held **inside the real
/// lane's encrypted records**, so the decoy session can neither read nor forge it. On each real
/// review the anchor pins the branch's head + record count; a later review that finds the witnessed
/// prefix truncated or rewritten flags forensic tampering the unkeyed hash chain alone can't prove.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct BranchAnchor {
    branch_ref: String,
    head_id_hex: String,
    record_count: u64,
}

/// The decrypted per-lane records payload. Both lanes serialize this; only the **real** lane ever
/// populates `anchors` (the decoy lane's stays empty).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct LaneData {
    #[serde(default)]
    notes: Vec<SanctuaryVaultNote>,
    #[serde(default)]
    anchors: Vec<BranchAnchor>,
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

/// Derive the 32-byte **wrapping key** for a lane from its derived key material. Domain-separated
/// with SHA-256 so it is never the same bytes as the lane's AEAD cipher key (which encrypts the
/// records), even though both descend from the same PIN stretch. Used only for the one-way key
/// hierarchy (`wrap_key`/`unwrap_key`, which is XChaCha20-Poly1305 internally).
fn wrapping_key_from(material: &SanctuaryKeyMaterial) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"q42:sanctuary:wrap-key:v1");
    h.update(material.cipher_key);
    h.update(material.volume_tweak);
    let out = h.finalize();
    let mut k = [0u8; 32];
    k.copy_from_slice(&out);
    k
}

/// Serialize lane key material to its 48-byte form (`cipher_key(32) ‖ volume_tweak(16)`) for wrapping.
fn material_to_bytes(m: &SanctuaryKeyMaterial) -> [u8; KEY_MATERIAL_BYTES] {
    let mut b = [0u8; KEY_MATERIAL_BYTES];
    b[..32].copy_from_slice(&m.cipher_key);
    b[32..].copy_from_slice(&m.volume_tweak);
    b
}

/// Reconstruct lane key material from its 48-byte form.
fn material_from_bytes(b: &[u8]) -> Result<SanctuaryKeyMaterial, String> {
    if b.len() != KEY_MATERIAL_BYTES {
        return Err("wrapped decoy key material has the wrong length".into());
    }
    let mut cipher_key = [0u8; 32];
    let mut volume_tweak = [0u8; 16];
    cipher_key.copy_from_slice(&b[..32]);
    volume_tweak.copy_from_slice(&b[32..48]);
    Ok(SanctuaryKeyMaterial {
        cipher_key,
        volume_tweak,
    })
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

/// Build a fresh lane and return it alongside its derived key material (needed by [`build_vault`]
/// to wire the real→decoy key hierarchy). The `audit_pubkey_hex` / `wrapped_keys` are filled in by
/// the caller, since they depend on the *sibling* lane's key.
fn new_lane(
    pin: &str,
    lane_id: SanctuaryLane,
    kdf: KdfDescriptor,
    pepper: Option<&[u8; 32]>,
) -> Result<(Layer, SanctuaryKeyMaterial), String> {
    let salt = random_salt();
    let secret = effective_secret(pin, pepper);
    let key = derive_lane_material(&kdf, &secret, &salt)?;
    let aad = lane_id.aad();
    let verifier = seal(&key, VERIFIER_CHUNK, VERIFIER_MAGIC, aad);
    let records_pt = cbor_encode(&LaneData::default())?;
    let records = seal(&key, 0, &records_pt, aad);
    let layer = Layer {
        id: lane_id.layer_id().to_string(),
        role: lane_id.role(),
        salt_hex: hex::encode(salt),
        kdf: Some(kdf),
        verifier,
        records,
        next_counter: 1,
        audit_pubkey_hex: None,
        wrapped_keys: Vec::new(),
    };
    Ok((layer, key))
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
    let (mut real, real_key) = new_lane(real_pin, SanctuaryLane::Real, kdf.clone(), pepper)?;
    let (mut decoy, decoy_key) = new_lane(decoy_pin, SanctuaryLane::Decoy, kdf, pepper)?;

    // --- Wire the real→decoy key hierarchy + the blind audit channel (ADR §3.1/§3.2) ---
    // 1. A fresh audit keypair. Its PUBLIC key lives (plaintext) on the decoy layer so a decoy
    //    session can seal records to it; its SECRET is wrapped under the real key so only the real
    //    lane can read them.
    let audit = AuditKeypair::generate().map_err(|e| format!("audit keypair: {e:?}"))?;
    decoy.audit_pubkey_hex = Some(hex::encode(audit.public));

    // 2. The real lane's wrapping key wraps the decoy lane key + the audit secret (one-way: the
    //    decoy holds nothing that unwraps toward real).
    let real_wrap = wrapping_key_from(&real_key);
    let wrapped_decoy = wrap_key(&real_wrap, &material_to_bytes(&decoy_key), WRAP_AAD_DECOY)
        .map_err(|e| format!("wrap decoy key: {e:?}"))?;
    let wrapped_audit = wrap_key(&real_wrap, audit.secret_bytes(), WRAP_AAD_AUDIT)
        .map_err(|e| format!("wrap audit secret: {e:?}"))?;
    real.wrapped_keys = vec![
        WrappedKey {
            purpose: WK_DECOY_LANE_KEY.into(),
            blob_hex: hex::encode(&wrapped_decoy),
        },
        WrappedKey {
            purpose: WK_AUDIT_SECRET.into(),
            blob_hex: hex::encode(&wrapped_audit),
        },
    ];

    // `new` pads to the constant layer shape (reserved layers) and stamps the v2 version.
    let container = VaultContainerV2::new(vec![real, decoy], pepper.is_some(), vault_id)?;
    save_container(root, &container)
}

/// Unwrap the decoy lane key material using the (already-unlocked) real lane key. Fails if the
/// vault has no wrapped decoy key or the wrap is tampered — a decoy or wrong key never reaches here.
fn unwrap_decoy_material(
    container: &VaultContainerV2,
    real_key: &SanctuaryKeyMaterial,
) -> Result<SanctuaryKeyMaterial, String> {
    let real_layer = container
        .layer_by_role(LayerRole::Real)
        .ok_or("Sanctuary vault is missing its real layer")?;
    let wrapped = real_layer
        .wrapped_key(WK_DECOY_LANE_KEY)
        .ok_or("Sanctuary vault has no wrapped decoy key")?;
    let blob = hex::decode(&wrapped.blob_hex).map_err(|e| e.to_string())?;
    let real_wrap = wrapping_key_from(real_key);
    let raw = unwrap_key(&real_wrap, &blob, WRAP_AAD_DECOY)
        .map_err(|_| "could not unwrap the decoy lane key (tampered vault?)".to_string())?;
    material_from_bytes(&raw)
}

/// **Real-session decoy curation (ADR §3.2).** From a real unlock, write a note into the decoy lane
/// *without* the decoy PIN — the real lane unwraps the decoy key from its one-way hierarchy. This is
/// how the victim keeps the decoy lived-in and plausible so a coercer's re-unlock shows believable
/// content. Requires the **real** PIN; supplying the decoy PIN is rejected (the decoy cannot curate
/// itself, and must not be able to detect that curation exists).
pub fn real_curate_decoy_add_note(
    root: impl AsRef<Path>,
    real_pin: &str,
    body: &str,
    now_unix: u32,
) -> Result<(), String> {
    let root = root.as_ref();
    let mut container = load_container(root)?.ok_or("Sanctuary vault is not set up")?;
    let pepper = pepper_for(&container, None)?;
    let (lane, real_key) = open_lane(&container, real_pin, pepper.as_ref())?;
    if lane != SanctuaryLane::Real {
        return Err("Curating the decoy requires the real unlock PIN".into());
    }
    let decoy_key = unwrap_decoy_material(&container, &real_key)?;

    // Read → append → re-seal the decoy records under the unwrapped decoy key (same key + monotonic
    // counter the decoy-PIN path uses, so nonces never collide).
    let aad = SanctuaryLane::Decoy.aad();
    let (mut data, counter) = {
        let layer = layer_ref(&container, SanctuaryLane::Decoy)?;
        let pt = open(&decoy_key, &layer.records, aad)?;
        let data: LaneData = cbor_decode(&pt)?;
        (data, layer.next_counter)
    };
    data.notes.push(SanctuaryVaultNote {
        id: uuid::Uuid::new_v4().to_string(),
        body: body.to_string(),
        created_at_unix: now_unix,
    });
    let records_pt = cbor_encode(&data)?;
    let blob = seal(&decoy_key, counter, &records_pt, aad);
    {
        let layer = layer_mut(&mut container, SanctuaryLane::Decoy)?;
        layer.records = blob;
        layer.next_counter = counter.saturating_add(1);
    }
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
    let data: LaneData = cbor_decode(&pt)?;
    Ok((lane, data.notes))
}

/// Append a note to the lane the PIN opens (real PIN → real lane; duress PIN → decoy only).
///
/// A **decoy** write also leaves a blind, sealed audit record on [`DEFAULT_DECOY_BRANCH`] that the
/// coercer cannot read; the real lane reviews it via [`review_decoy_activity`]. For git-like
/// per-session branches (one branch per duress unlock), use [`add_note_in_session`].
pub fn add_note(
    root: impl AsRef<Path>,
    pin: &str,
    body: &str,
    now_unix: u32,
) -> Result<SanctuaryLane, String> {
    add_note_in_session(root, pin, body, now_unix, DEFAULT_DECOY_BRANCH)
}

/// As [`add_note`], but a **decoy** write is attributed to `session_ref` — a fresh ref per duress
/// unlock yields the git-like per-session branch (ADR §10). Ignored for real-lane writes (real
/// activity is never audited).
pub fn add_note_in_session(
    root: impl AsRef<Path>,
    pin: &str,
    body: &str,
    now_unix: u32,
    session_ref: &str,
) -> Result<SanctuaryLane, String> {
    let root = root.as_ref();
    let mut container = load_container(root)?.ok_or("Sanctuary vault is not set up")?;
    let pepper = pepper_for(&container, None)?;
    let (lane, key) = open_lane(&container, pin, pepper.as_ref())?;

    // Read → append → re-seal this lane's records.
    let (mut data, counter) = {
        let layer = layer_ref(&container, lane)?;
        let pt = open(&key, &layer.records, lane.aad())?;
        let data: LaneData = cbor_decode(&pt)?;
        (data, layer.next_counter)
    };
    data.notes.push(SanctuaryVaultNote {
        id: uuid::Uuid::new_v4().to_string(),
        body: body.to_string(),
        created_at_unix: now_unix,
    });
    let records_pt = cbor_encode(&data)?;
    let blob = seal(&key, counter, &records_pt, lane.aad());
    {
        let layer = layer_mut(&mut container, lane)?;
        layer.records = blob;
        layer.next_counter = counter.saturating_add(1);
    }

    // A decoy (duress) write leaves a blind, sealed trail the coercer can neither read nor forge.
    if lane == SanctuaryLane::Decoy {
        record_decoy_action(
            &mut container,
            session_ref,
            AuditAction::AddNote,
            body.as_bytes(),
            now_unix,
        )?;
    }

    save_container(root, &container)?;
    Ok(lane)
}

// --- Blind audit append (decoy-session writes) ---

/// The decoy layer's audit **public** key (a decoy session seals its records to it).
fn decoy_audit_pubkey(container: &VaultContainerV2) -> Result<[u8; 32], String> {
    let decoy = container
        .layer_by_role(LayerRole::Decoy)
        .ok_or("Sanctuary vault is missing its decoy layer")?;
    let hex_s = decoy
        .audit_pubkey_hex
        .as_ref()
        .ok_or("decoy layer has no audit public key")?;
    let bytes = hex::decode(hex_s).map_err(|e| e.to_string())?;
    if bytes.len() != 32 {
        return Err("decoy audit public key has the wrong length".into());
    }
    let mut k = [0u8; 32];
    k.copy_from_slice(&bytes);
    Ok(k)
}

/// The id of the last record on `branch_ref` in append order (the branch head), if any.
fn last_head_for_branch(log: &[AuditRecord], branch_ref: &str) -> Option<[u8; 32]> {
    log.iter().rev().find(|r| r.branch_ref == branch_ref).map(|r| r.id)
}

/// Append one sealed audit record for a decoy action. On the first record of a branch, an
/// `OpenSession` record is appended first so every branch has a genesis entry-point.
fn record_decoy_action(
    container: &mut VaultContainerV2,
    branch_ref: &str,
    action: AuditAction,
    payload: &[u8],
    now_unix: u32,
) -> Result<(), String> {
    let audit_pub = decoy_audit_pubkey(container)?;
    if last_head_for_branch(&container.audit_log, branch_ref).is_none() {
        append_sealed(
            container,
            &audit_pub,
            branch_ref,
            AuditAction::OpenSession,
            b"session opened",
            now_unix,
        )?;
    }
    append_sealed(container, &audit_pub, branch_ref, action, payload, now_unix)
}

/// Seal `payload` to the audit public key and push a content-addressed record onto the branch. The
/// chain link is `chain_hash(parent ‖ canonical_bytes)` inside [`AuditRecord::new`]; `parent` is the
/// current branch head (or genesis).
fn append_sealed(
    container: &mut VaultContainerV2,
    audit_pub: &[u8; 32],
    branch_ref: &str,
    action: AuditAction,
    payload: &[u8],
    now_unix: u32,
) -> Result<(), String> {
    let parent = last_head_for_branch(&container.audit_log, branch_ref).unwrap_or(GENESIS_PARENT);
    let sealed = seal_to(audit_pub, payload, branch_ref.as_bytes())
        .map_err(|e| format!("seal audit record: {e:?}"))?;
    let rec = AuditRecord::new(
        parent,
        branch_ref,
        DECOY_ACTOR_DID,
        Some("decoy-session".into()),
        None,
        action,
        now_unix,
        sealed,
    );
    container.audit_log.push(rec);
    Ok(())
}

fn action_label(a: &AuditAction) -> String {
    match a {
        AuditAction::OpenSession => "open_session".into(),
        AuditAction::AddNote => "add_note".into(),
        AuditAction::EditNote => "edit_note".into(),
        AuditAction::DeleteNote => "delete_note".into(),
        AuditAction::Other(s) => format!("other:{s}"),
    }
}

// --- Real-lane audit review + head anchor (ADR §10 — C's finding) ---

/// The integrity verdict of the decoy audit log at review time.
///
/// * `Ok` — every branch's hash chain verifies and every witnessed prefix is intact.
/// * `ChainBroken` — a branch's on-disk chain is internally broken (rewrite/reorder/drop caught by
///   the content-address chain itself).
/// * `WitnessedPrefixAltered` — a prefix the real lane had **already witnessed** (anchored) is now
///   truncated or replaced. This is the forensic-tamper case the unkeyed chain alone cannot prove
///   (a coercer can forge a fresh consistent chain); the real-lane head anchor catches it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditIntegrity {
    Ok,
    ChainBroken { branch_ref: String },
    WitnessedPrefixAltered { branch_ref: String },
}

fn integrity_rank(i: &AuditIntegrity) -> u8 {
    match i {
        AuditIntegrity::Ok => 0,
        AuditIntegrity::ChainBroken { .. } => 1,
        AuditIntegrity::WitnessedPrefixAltered { .. } => 2,
    }
}

fn escalate(current: AuditIntegrity, candidate: AuditIntegrity) -> AuditIntegrity {
    if integrity_rank(&candidate) > integrity_rank(&current) {
        candidate
    } else {
        current
    }
}

/// One decrypted decoy action surfaced to the real lane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecoyActionView {
    pub branch_ref: String,
    /// `open_session` / `add_note` / `edit_note` / `delete_note` / `other:<label>`.
    pub action: String,
    pub actor_did: String,
    pub unix: u32,
    /// The decrypted payload (the coercer's note body), lossy-UTF8; empty if it could not be opened.
    pub payload: String,
}

/// The result of reviewing the decoy audit log from the real lane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecoyActivityReport {
    pub integrity: AuditIntegrity,
    /// Number of distinct entry-point sessions (branches). A **proxy / lower bound** for "number of
    /// attackers", NOT a verified head-count — shared credentials or one persistent actor across
    /// sessions both defeat a naive count (ADR §10, audit-DAG module docs). The UI must not claim a
    /// hard attacker count.
    pub session_count: usize,
    pub actions: Vec<DecoyActionView>,
}

/// **Review decoy activity from the real lane (ADR §3.1 / §10).** Opens the real lane, unwraps the
/// audit secret, decrypts every sealed decoy-session record, verifies chain integrity, and checks
/// each previously-witnessed prefix against its head anchor (detecting forensic truncation/replace).
/// Then advances the anchors for clean branches and persists them inside the real lane's encrypted
/// records. Requires the **real** PIN; the decoy PIN is rejected.
pub fn review_decoy_activity(
    root: impl AsRef<Path>,
    real_pin: &str,
) -> Result<DecoyActivityReport, String> {
    use std::collections::HashMap;

    let root = root.as_ref();
    let mut container = load_container(root)?.ok_or("Sanctuary vault is not set up")?;
    let pepper = pepper_for(&container, None)?;
    let (lane, real_key) = open_lane(&container, real_pin, pepper.as_ref())?;
    if lane != SanctuaryLane::Real {
        return Err("Reviewing decoy activity requires the real unlock PIN".into());
    }

    // Unwrap the audit secret + read the real lane's records (notes + prior anchors), then drop the
    // immutable borrow before any mutation.
    let (wrapped_audit_hex, real_records) = {
        let real_layer = container
            .layer_by_role(LayerRole::Real)
            .ok_or("Sanctuary vault is missing its real layer")?;
        let wrapped = real_layer
            .wrapped_key(WK_AUDIT_SECRET)
            .ok_or("Sanctuary vault has no wrapped audit secret")?;
        (wrapped.blob_hex.clone(), real_layer.records.clone())
    };
    let real_wrap = wrapping_key_from(&real_key);
    let secret_bytes = unwrap_key(
        &real_wrap,
        &hex::decode(&wrapped_audit_hex).map_err(|e| e.to_string())?,
        WRAP_AAD_AUDIT,
    )
    .map_err(|_| "could not unwrap the audit secret (tampered vault?)".to_string())?;
    if secret_bytes.len() != 32 {
        return Err("audit secret has the wrong length".into());
    }
    let mut audit_secret = [0u8; 32];
    audit_secret.copy_from_slice(&secret_bytes);

    let mut data: LaneData = cbor_decode(&open(&real_key, &real_records, SanctuaryLane::Real.aad())?)?;
    let prior: HashMap<String, BranchAnchor> = data
        .anchors
        .iter()
        .cloned()
        .map(|a| (a.branch_ref.clone(), a))
        .collect();

    let sessions = derive_sessions(&container.audit_log);
    let session_count = sessions.len();

    let mut integrity = AuditIntegrity::Ok;
    let mut actions: Vec<DecoyActionView> = Vec::new();
    let mut new_anchors = prior.clone();

    for session in &sessions {
        let branch = &session.branch_ref;
        let chain_ok = verify_chain(&session.records) == ChainStatus::Ok;
        let mut witnessed_altered = false;
        if !chain_ok {
            integrity = escalate(
                integrity,
                AuditIntegrity::ChainBroken {
                    branch_ref: branch.clone(),
                },
            );
        } else if let Some(anchor) = prior.get(branch) {
            let n = anchor.record_count as usize;
            if n > 0 {
                witnessed_altered = session.records.len() < n
                    || verify_chain(&session.records[..n]) != ChainStatus::Ok
                    || hex::encode(session.records[n - 1].id) != anchor.head_id_hex;
            }
            if witnessed_altered {
                integrity = escalate(
                    integrity,
                    AuditIntegrity::WitnessedPrefixAltered {
                        branch_ref: branch.clone(),
                    },
                );
            }
        }

        for rec in &session.records {
            let payload = open_sealed(&audit_secret, &rec.sealed, branch.as_bytes())
                .map(|p| String::from_utf8_lossy(&p).into_owned())
                .unwrap_or_default();
            actions.push(DecoyActionView {
                branch_ref: branch.clone(),
                action: action_label(&rec.action),
                actor_did: rec.actor_did.clone(),
                unix: rec.unix,
                payload,
            });
        }

        // Advance the anchor only for a clean branch; a tampered branch keeps its prior anchor as
        // standing evidence of what the real lane last witnessed.
        if chain_ok && !witnessed_altered {
            if let Some(last) = session.records.last() {
                new_anchors.insert(
                    branch.clone(),
                    BranchAnchor {
                        branch_ref: branch.clone(),
                        head_id_hex: hex::encode(last.id),
                        record_count: session.records.len() as u64,
                    },
                );
            }
        }
    }

    // Persist updated anchors (inside the real lane's encrypted records) only if they changed.
    let mut anchors_vec: Vec<BranchAnchor> = new_anchors.into_values().collect();
    anchors_vec.sort_by(|a, b| a.branch_ref.cmp(&b.branch_ref));
    let mut prior_vec: Vec<BranchAnchor> = prior.into_values().collect();
    prior_vec.sort_by(|a, b| a.branch_ref.cmp(&b.branch_ref));
    if anchors_vec != prior_vec {
        data.anchors = anchors_vec;
        let counter = layer_ref(&container, SanctuaryLane::Real)?.next_counter;
        let records_pt = cbor_encode(&data)?;
        let blob = seal(&real_key, counter, &records_pt, SanctuaryLane::Real.aad());
        {
            let layer = layer_mut(&mut container, SanctuaryLane::Real)?;
            layer.records = blob;
            layer.next_counter = counter.saturating_add(1);
        }
        save_container(root, &container)?;
    }

    Ok(DecoyActivityReport {
        integrity,
        session_count,
        actions,
    })
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

    // --- S5b: real→decoy key hierarchy + decoy curation ---

    #[test]
    fn setup_wires_audit_pubkey_and_wrapped_keys() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        let container = load_container(dir.path()).unwrap().unwrap();
        // Decoy layer carries the audit PUBLIC key (so a decoy session can seal to it).
        let decoy = container.layer_by_role(LayerRole::Decoy).unwrap();
        assert!(decoy.audit_pubkey_hex.is_some());
        // The audit secret / pubkey are never both readable from the decoy: only the pubkey is here.
        assert!(decoy.wrapped_keys.is_empty());
        // Real layer wraps BOTH the decoy lane key and the audit secret.
        let real = container.layer_by_role(LayerRole::Real).unwrap();
        assert!(real.wrapped_key(WK_DECOY_LANE_KEY).is_some());
        assert!(real.wrapped_key(WK_AUDIT_SECRET).is_some());
    }

    #[test]
    fn real_session_curates_decoy_without_decoy_pin() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        // The victim, in a safe real session, seeds the decoy — no decoy PIN entered.
        real_curate_decoy_add_note(dir.path(), "real-pin-1", "plausible cover note", 20).unwrap();

        // The coercer, later, opens the decoy with the duress PIN and sees the seeded note.
        let (lane, decoy_notes) = list_notes(dir.path(), "decoy-pin-2").unwrap();
        assert_eq!(lane, SanctuaryLane::Decoy);
        assert_eq!(decoy_notes.len(), 1);
        assert_eq!(decoy_notes[0].body, "plausible cover note");

        // The real lane is untouched by curation.
        let (_, real_notes) = list_notes(dir.path(), "real-pin-1").unwrap();
        assert!(real_notes.is_empty());
    }

    #[test]
    fn curation_and_decoy_pin_writes_coexist_without_nonce_collision() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        real_curate_decoy_add_note(dir.path(), "real-pin-1", "seeded-by-real", 20).unwrap();
        add_note(dir.path(), "decoy-pin-2", "written-by-coercer", 21).unwrap();
        real_curate_decoy_add_note(dir.path(), "real-pin-1", "seeded-again", 22).unwrap();

        let (_, notes) = list_notes(dir.path(), "decoy-pin-2").unwrap();
        let bodies: Vec<&str> = notes.iter().map(|n| n.body.as_str()).collect();
        assert_eq!(bodies, ["seeded-by-real", "written-by-coercer", "seeded-again"]);
    }

    #[test]
    fn curation_requires_the_real_pin() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        // The decoy PIN cannot curate the decoy (it must not even be able to detect curation).
        assert!(real_curate_decoy_add_note(dir.path(), "decoy-pin-2", "x", 20).is_err());
        // A wrong PIN cannot either.
        assert!(real_curate_decoy_add_note(dir.path(), "nope-nope", "x", 20).is_err());
    }

    #[test]
    fn audit_pubkey_on_decoy_and_secret_under_real_correspond() {
        use qualia_core_db::crypto::sanctuary_audit::{open_sealed, seal_to};

        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        let container = load_container(dir.path()).unwrap().unwrap();

        // A decoy session seals a record to the audit pubkey it can see on its own layer.
        let decoy = container.layer_by_role(LayerRole::Decoy).unwrap();
        let pub_hex = decoy.audit_pubkey_hex.as_ref().unwrap();
        let mut audit_pub = [0u8; 32];
        audit_pub.copy_from_slice(&hex::decode(pub_hex).unwrap());
        let sealed = seal_to(&audit_pub, b"coercer wrote a note under duress", b"branch:test").unwrap();

        // The real lane unwraps the audit secret and opens the record; the decoy never can.
        let (_lane, real_key) = open_lane(&container, "real-pin-1", None).unwrap();
        let real = container.layer_by_role(LayerRole::Real).unwrap();
        let wrapped = real.wrapped_key(WK_AUDIT_SECRET).unwrap();
        let real_wrap = wrapping_key_from(&real_key);
        let secret_bytes =
            unwrap_key(&real_wrap, &hex::decode(&wrapped.blob_hex).unwrap(), WRAP_AAD_AUDIT).unwrap();
        let mut audit_secret = [0u8; 32];
        audit_secret.copy_from_slice(&secret_bytes);

        let opened = open_sealed(&audit_secret, &sealed, b"branch:test").unwrap();
        assert_eq!(opened, b"coercer wrote a note under duress");
    }

    #[test]
    fn decoy_key_wrap_is_one_way() {
        // The wrapped decoy key must NOT unwrap under a wrapping key derived from the decoy lane.
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        let container = load_container(dir.path()).unwrap().unwrap();

        let (_l, real_key) = open_lane(&container, "real-pin-1", None).unwrap();
        // Sanity: it unwraps under the real key.
        assert!(unwrap_decoy_material(&container, &real_key).is_ok());

        // But not under the decoy key (the decoy cannot reach up).
        let (_l2, decoy_key) = open_lane(&container, "decoy-pin-2", None).unwrap();
        assert!(unwrap_decoy_material(&container, &decoy_key).is_err());
    }

    // --- S5c: blind audit append + real-lane review + head anchor ---

    #[test]
    fn decoy_writes_leave_readable_sealed_trail_for_real_lane() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        add_note(dir.path(), "decoy-pin-2", "coercer wrote this", 10).unwrap();

        // The coercer's plaintext note is NOT on disk (the audit payload is a sealed box).
        let raw = fs::read(vault_path(dir.path())).unwrap();
        let needle = b"coercer wrote this";
        assert!(
            !raw.windows(needle.len()).any(|w| w == needle),
            "decoy body leaked to disk"
        );

        // The real lane decrypts the trail.
        let report = review_decoy_activity(dir.path(), "real-pin-1").unwrap();
        assert_eq!(report.integrity, AuditIntegrity::Ok);
        assert_eq!(report.session_count, 1);
        assert!(report
            .actions
            .iter()
            .any(|a| a.action == "open_session"));
        assert!(report
            .actions
            .iter()
            .any(|a| a.action == "add_note" && a.payload == "coercer wrote this"));
    }

    #[test]
    fn review_requires_the_real_pin() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        add_note(dir.path(), "decoy-pin-2", "x", 10).unwrap();
        assert!(review_decoy_activity(dir.path(), "decoy-pin-2").is_err());
        assert!(review_decoy_activity(dir.path(), "wrong-pin-zz").is_err());
    }

    #[test]
    fn distinct_sessions_become_distinct_branches() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        add_note_in_session(dir.path(), "decoy-pin-2", "s1 note", 10, "unlock-1").unwrap();
        add_note_in_session(dir.path(), "decoy-pin-2", "s2 note", 11, "unlock-2").unwrap();
        let report = review_decoy_activity(dir.path(), "real-pin-1").unwrap();
        assert_eq!(report.session_count, 2);
        assert_eq!(report.integrity, AuditIntegrity::Ok);
    }

    #[test]
    fn real_notes_and_decoy_notes_stay_isolated_under_audit() {
        // Adding an audit trail must not leak into either lane's note list.
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        add_note(dir.path(), "decoy-pin-2", "decoy body", 10).unwrap();
        add_note(dir.path(), "real-pin-1", "real body", 11).unwrap();

        let (_, real_notes) = list_notes(dir.path(), "real-pin-1").unwrap();
        assert_eq!(real_notes.len(), 1);
        assert_eq!(real_notes[0].body, "real body");
        let (_, decoy_notes) = list_notes(dir.path(), "decoy-pin-2").unwrap();
        assert_eq!(decoy_notes.len(), 1);
        assert_eq!(decoy_notes[0].body, "decoy body");
    }

    #[test]
    fn anchor_flags_truncation_of_witnessed_prefix() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        add_note_in_session(dir.path(), "decoy-pin-2", "note A", 10, "s1").unwrap();
        add_note_in_session(dir.path(), "decoy-pin-2", "note B", 11, "s1").unwrap();
        // First review witnesses + anchors branch s1 (open_session + 2 add_note = 3 records).
        assert_eq!(
            review_decoy_activity(dir.path(), "real-pin-1").unwrap().integrity,
            AuditIntegrity::Ok
        );

        // A coercer with file access drops the last audit record.
        let mut container = load_container(dir.path()).unwrap().unwrap();
        container.audit_log.pop();
        save_container(dir.path(), &container).unwrap();

        let report = review_decoy_activity(dir.path(), "real-pin-1").unwrap();
        assert_eq!(
            report.integrity,
            AuditIntegrity::WitnessedPrefixAltered { branch_ref: "s1".into() }
        );
    }

    #[test]
    fn anchor_flags_forged_replacement_of_witnessed_branch() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        add_note_in_session(dir.path(), "decoy-pin-2", "coercer note A", 10, "s1").unwrap();
        add_note_in_session(dir.path(), "decoy-pin-2", "coercer note B", 11, "s1").unwrap();
        assert_eq!(
            review_decoy_activity(dir.path(), "real-pin-1").unwrap().integrity,
            AuditIntegrity::Ok
        );

        // A coercer wholesale-replaces s1 with a fresh, internally-consistent chain of the same
        // length (unkeyed BLAKE3 lets them forge a valid chain) — but the forged head no longer
        // matches the real lane's anchor. This is exactly C's finding.
        let mut container = load_container(dir.path()).unwrap().unwrap();
        let orig_len = container.audit_log.len();
        let mut forged = Vec::new();
        let mut parent = GENESIS_PARENT;
        for i in 0..orig_len {
            let rec = AuditRecord::new(
                parent,
                "s1",
                "did:forged",
                None,
                None,
                AuditAction::AddNote,
                100 + i as u32,
                vec![i as u8],
            );
            parent = rec.id;
            forged.push(rec);
        }
        container.audit_log = forged;
        save_container(dir.path(), &container).unwrap();

        let report = review_decoy_activity(dir.path(), "real-pin-1").unwrap();
        assert_eq!(
            report.integrity,
            AuditIntegrity::WitnessedPrefixAltered { branch_ref: "s1".into() }
        );
    }

    #[test]
    fn clean_review_is_idempotent_and_stable() {
        let dir = tempfile::tempdir().unwrap();
        setup_with_iterations(dir.path(), "real-pin-1", "decoy-pin-2", 1_000).unwrap();
        add_note(dir.path(), "decoy-pin-2", "one", 10).unwrap();
        let a = review_decoy_activity(dir.path(), "real-pin-1").unwrap();
        let b = review_decoy_activity(dir.path(), "real-pin-1").unwrap();
        assert_eq!(a.integrity, AuditIntegrity::Ok);
        assert_eq!(b.integrity, AuditIntegrity::Ok);
        assert_eq!(a.actions, b.actions);
        // Real lane still opens and its notes are intact after anchor writes.
        let (_, real_notes) = list_notes(dir.path(), "real-pin-1").unwrap();
        assert!(real_notes.is_empty());
    }
}
