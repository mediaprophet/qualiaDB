//! Sanctuary vault v2 on-disk container (S3) — **CBOR-native, additive**; not yet wired into the vault.
//!
//! The vault is an **n-layer, CBOR-serialized** container (per the vault-v2 ADR). Each [`Layer`] is
//! independently keyed; the collection generalises to any number of layers (real + decoy(s) + reserved
//! padding) so the *count* of layers on disk is a constant, revealing nothing about how many are real /
//! decoy / empty (ADR §9 constant-shape).
//!
//! **No JSON, no migration.** There is no deployed vault, so there is nothing to migrate — the format
//! is CBOR from the start, and no JSON path exists here. When S5 reconciles this with the live vault it
//! removes `serde_json` from the vault entirely (records + container both CBOR).
//!
//! **Honest scope:** CBOR is binary but self-describing — this is *consistency + not-text-editor-
//! readable*, not cryptographic hiding (a decoder still recovers the structure). The reserved padding
//! layers here carry empty blobs; making them **byte-indistinguishable** from real layers (size-matched
//! random ciphertext) is finished in S5. What S3 fixes is the *structural* shape.

use serde::{Deserialize, Serialize};

use qualia_core_db::crypto::sanctuary_audit_dag::AuditRecord;

/// Number of layer slots every container carries, so the layer *count* is constant regardless of how
/// many are actually in use (real + decoy(s) + reserved).
pub const CONTAINER_SLOTS: usize = 4;
pub const CONTAINER_VERSION: u16 = 2;

/// An AEAD ciphertext blob (hex) + the chunk index used to derive its nonce.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EncBlob {
    pub chunk_index: u64,
    pub ct_hex: String,
    pub tag_hex: String,
}

/// Per-layer KDF descriptor. Every real/decoy layer carries one (Argon2id in production); `None` only
/// on reserved padding, which is never opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "algo")]
pub enum KdfDescriptor {
    Pbkdf2 {
        iterations: u32,
    },
    Argon2id {
        m_cost_kib: u32,
        t_cost: u32,
        p_cost: u32,
    },
}

/// The role a layer plays. `Reserved` layers exist only to keep the container shape constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerRole {
    Real,
    Decoy,
    Reserved,
}

/// A key wrapped under a superior layer's key (the one-way hierarchy: the real layer wraps the decoy
/// layer key and the audit secret). `blob_hex` is the output of `sanctuary_audit::wrap_key`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrappedKey {
    /// e.g. `"decoy_lane_key"`, `"audit_secret"`.
    pub purpose: String,
    pub blob_hex: String,
}

/// One independently-keyed layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    /// Stable, non-secret layer id (addresses the manifold coordinate; e.g. `"real"`, `"decoy:0"`).
    pub id: String,
    pub role: LayerRole,
    pub salt_hex: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kdf: Option<KdfDescriptor>,
    pub verifier: EncBlob,
    pub records: EncBlob,
    pub next_counter: u64,
    /// The audit channel public key for this layer (a decoy layer's coercer-writes seal to this).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_pubkey_hex: Option<String>,
    /// Subordinate keys this layer's key can unwrap (real → decoy key + audit secret).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wrapped_keys: Vec<WrappedKey>,
}

impl Layer {
    /// Find a wrapped subordinate key by its `purpose` (e.g. `"decoy_lane_key"`, `"audit_secret"`).
    pub fn wrapped_key(&self, purpose: &str) -> Option<&WrappedKey> {
        self.wrapped_keys.iter().find(|w| w.purpose == purpose)
    }

    /// A reserved padding layer: fresh random salt, empty blobs. (S5 makes these byte-indistinguishable.)
    pub fn reserved(index: usize) -> Self {
        Layer {
            id: format!("reserved:{index}"),
            role: LayerRole::Reserved,
            salt_hex: random_salt_hex(),
            kdf: None,
            verifier: EncBlob::default(),
            records: EncBlob::default(),
            next_counter: 0,
            audit_pubkey_hex: None,
            wrapped_keys: Vec::new(),
        }
    }
}

/// The v2 vault container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultContainerV2 {
    pub version: u16,
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub keychain_wrapped: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_id: Option<String>,
    /// **Append-only** audit DAG (ADR §10): the sealed records a decoy session writes but cannot
    /// read. Only the real lane holds the audit secret that opens each record's `sealed` blob. The
    /// vault code only ever *appends* here; the per-branch head anchor (held inside the real lane's
    /// encrypted records) is what makes truncation / rewrite of the *witnessed* prefix detectable.
    #[serde(default)]
    pub audit_log: Vec<AuditRecord>,
}

impl VaultContainerV2 {
    /// Build a container from the in-use layers, padded to constant shape. Errors if there are more
    /// in-use layers than [`CONTAINER_SLOTS`].
    pub fn new(
        layers: Vec<Layer>,
        keychain_wrapped: bool,
        vault_id: Option<String>,
    ) -> Result<Self, String> {
        if layers.len() > CONTAINER_SLOTS {
            return Err("more in-use layers than container slots".into());
        }
        let mut container = VaultContainerV2 {
            version: CONTAINER_VERSION,
            layers,
            keychain_wrapped,
            vault_id,
            audit_log: Vec::new(),
        };
        container.pad_to_constant_shape();
        Ok(container)
    }

    /// Serialize to CBOR bytes.
    pub fn to_cbor(&self) -> Result<Vec<u8>, String> {
        let mut buf = Vec::new();
        ciborium::into_writer(self, &mut buf).map_err(|e| e.to_string())?;
        Ok(buf)
    }

    /// Deserialize from CBOR bytes.
    pub fn from_cbor(bytes: &[u8]) -> Result<Self, String> {
        ciborium::from_reader(bytes).map_err(|e| e.to_string())
    }

    /// Find a layer by role (first match).
    pub fn layer_by_role(&self, role: LayerRole) -> Option<&Layer> {
        self.layers.iter().find(|l| l.role == role)
    }

    /// Pad with reserved layers up to [`CONTAINER_SLOTS`] so the layer count is constant.
    pub fn pad_to_constant_shape(&mut self) {
        while self.layers.len() < CONTAINER_SLOTS {
            self.layers.push(Layer::reserved(self.layers.len()));
        }
    }
}

fn random_salt_hex() -> String {
    // uuid v4 is CSPRNG-backed; 16 bytes of salt is ample.
    hex::encode(uuid::Uuid::new_v4().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_layer(id: &str, role: LayerRole) -> Layer {
        Layer {
            id: id.into(),
            role,
            salt_hex: "00112233445566778899aabbccddeeff".into(),
            kdf: Some(KdfDescriptor::Argon2id {
                m_cost_kib: 65536,
                t_cost: 3,
                p_cost: 1,
            }),
            verifier: EncBlob {
                chunk_index: u64::MAX,
                ct_hex: "aa".into(),
                tag_hex: "bb".into(),
            },
            records: EncBlob {
                chunk_index: 0,
                ct_hex: "cc".into(),
                tag_hex: "dd".into(),
            },
            next_counter: 1,
            audit_pubkey_hex: Some("ee".repeat(32)),
            wrapped_keys: vec![WrappedKey {
                purpose: "decoy_lane_key".into(),
                blob_hex: "ff".into(),
            }],
        }
    }

    fn sample_container() -> VaultContainerV2 {
        VaultContainerV2::new(
            vec![
                sample_layer("real", LayerRole::Real),
                sample_layer("decoy:0", LayerRole::Decoy),
            ],
            false,
            None,
        )
        .unwrap()
    }

    #[test]
    fn cbor_round_trips() {
        let c = sample_container();
        let bytes = c.to_cbor().unwrap();
        let back = VaultContainerV2::from_cbor(&bytes).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn constant_shape_is_fixed_and_padded_with_reserved() {
        let c = sample_container();
        assert_eq!(c.layers.len(), CONTAINER_SLOTS);
        assert_eq!(
            c.layers
                .iter()
                .filter(|l| l.role == LayerRole::Reserved)
                .count(),
            CONTAINER_SLOTS - 2
        );
        // Each reserved layer has a distinct random salt.
        let reserved_salts: std::collections::HashSet<_> = c
            .layers
            .iter()
            .filter(|l| l.role == LayerRole::Reserved)
            .map(|l| l.salt_hex.clone())
            .collect();
        assert_eq!(reserved_salts.len(), CONTAINER_SLOTS - 2);
    }

    #[test]
    fn layer_by_role_finds_real_and_decoy() {
        let c = sample_container();
        assert_eq!(c.layer_by_role(LayerRole::Real).unwrap().id, "real");
        assert_eq!(c.layer_by_role(LayerRole::Decoy).unwrap().id, "decoy:0");
    }

    #[test]
    fn too_many_layers_is_rejected() {
        let many: Vec<Layer> = (0..CONTAINER_SLOTS + 1)
            .map(|i| sample_layer(&format!("l{i}"), LayerRole::Decoy))
            .collect();
        assert!(VaultContainerV2::new(many, false, None).is_err());
    }
}
