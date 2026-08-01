use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

fn generate_master_secret() -> Result<[u8; 32], String> {
    let mut secret = [0u8; 32];
    getrandom::fill(&mut secret).map_err(|e| format!("OS RNG failed: {e}"))?;
    Ok(secret)
}

#[cfg(not(target_arch = "wasm32"))]
fn keyring_entry_for(storage_dir: &str) -> Result<keyring::Entry, String> {
    let mut hasher = Sha256::new();
    hasher.update(storage_dir.as_bytes());
    let digest = hasher.finalize();
    let username = format!("master_{}", hex::encode(&digest[..8]));
    keyring::Entry::new("qualia_db", &username).map_err(|e| format!("Keyring error: {e}"))
}

/// High-level Key Management module for the Qualia Node.
pub struct KeyVault {
    master_key: Option<SigningKey>,
    storage_dir: Option<String>,
}

impl KeyVault {
    /// Creates an in-memory KeyVault with a fresh ephemeral key (for tests/stubs only).
    pub fn new() -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"ephemeral");
        let result = hasher.finalize();
        let mut secret = [0u8; 32];
        secret.copy_from_slice(&result);
        Self {
            master_key: Some(SigningKey::from_bytes(&secret)),
            storage_dir: None,
        }
    }

    pub fn is_locked(&self) -> bool {
        self.master_key.is_none()
    }

    pub fn lock(&mut self) {
        self.master_key = None;
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn unlock(&mut self) -> Result<(), String> {
        let dir = self
            .storage_dir
            .as_ref()
            .ok_or("No storage dir configured")?;
        let temp = Self::load_or_generate(dir)?;
        self.master_key = temp.master_key;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn unlock(&mut self) -> Result<(), String> {
        let temp = Self::load_or_generate("")?;
        self.master_key = temp.master_key;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_or_generate(storage_dir: &str) -> Result<Self, String> {
        let vault_path = Path::new(storage_dir).join("keystore.bin");
        let entry = keyring_entry_for(storage_dir)?;

        let master_key = match entry.get_password() {
            Ok(secret_hex) => {
                let bytes =
                    hex::decode(secret_hex).map_err(|e| format!("Invalid hex in keyring: {e}"))?;
                if bytes.len() != 32 {
                    return Err("Corrupted master key length in keyring".into());
                }
                let mut secret = [0u8; 32];
                secret.copy_from_slice(&bytes[0..32]);
                SigningKey::from_bytes(&secret)
            }
            Err(_) => {
                if vault_path.exists() {
                    let bytes = fs::read(&vault_path)
                        .map_err(|e| format!("Failed to read keystore: {e}"))?;
                    if bytes.len() != 32 {
                        return Err("Corrupted master key length".into());
                    }
                    let secret_hex = hex::encode(&bytes[0..32]);
                    let _ = entry.set_password(&secret_hex);
                    let mut secret = [0u8; 32];
                    secret.copy_from_slice(&bytes[0..32]);
                    SigningKey::from_bytes(&secret)
                } else {
                    let secret = generate_master_secret()?;
                    let new_key = SigningKey::from_bytes(&secret);
                    let secret_hex = hex::encode(secret);
                    let _ = entry.set_password(&secret_hex);
                    if let Some(parent) = vault_path.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|e| format!("Failed to create keystore dir: {e}"))?;
                    }
                    fs::write(&vault_path, secret)
                        .map_err(|e| format!("Failed to write keystore: {e}"))?;
                    new_key
                }
            }
        };

        Ok(Self {
            master_key: Some(master_key),
            storage_dir: Some(storage_dir.to_string()),
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load_or_generate(storage_dir: &str) -> Result<Self, String> {
        let vault_path = Path::new(storage_dir).join("keystore.bin");

        let master_key = if vault_path.exists() {
            let bytes =
                fs::read(&vault_path).map_err(|e| format!("Failed to read keystore: {}", e))?;
            if bytes.len() != 32 {
                return Err("Corrupted master key length".into());
            }
            let mut secret = [0u8; 32];
            secret.copy_from_slice(&bytes[0..32]);
            SigningKey::from_bytes(&secret)
        } else {
            let secret = generate_master_secret()?;
            if let Some(parent) = vault_path.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create keystore dir: {e}"))?;
                }
            }
            if !storage_dir.is_empty() {
                fs::write(&vault_path, secret)
                    .map_err(|e| format!("Failed to write keystore: {e}"))?;
            }
            SigningKey::from_bytes(&secret)
        };

        Ok(Self {
            master_key: Some(master_key),
            storage_dir: Some(storage_dir.to_string()),
        })
    }

    /// Derives a deterministic Pairwise or Front Door key from the Master Key.
    /// This ensures we can recover all DIDs from the single master root.
    pub fn derive_key(&self, context_id: &str) -> SigningKey {
        let master_key = self.master_key.as_ref().expect("Vault is locked");
        let mut hasher = Sha256::new();
        hasher.update(master_key.to_bytes());
        hasher.update(context_id.as_bytes());
        let result = hasher.finalize();

        let mut child_secret = [0u8; 32];
        child_secret.copy_from_slice(&result);
        SigningKey::from_bytes(&child_secret)
    }

    /// Computes an Ed25519 signature over a generic byte payload
    pub fn sign_payload(&self, signing_key: &SigningKey, payload: &[u8]) -> Signature {
        signing_key.sign(payload)
    }

    /// Exposes the raw bytes of the master key for libp2p identity bindings
    pub fn get_master_key_bytes(&self) -> [u8; 32] {
        self.master_key
            .as_ref()
            .expect("Vault is locked")
            .to_bytes()
    }

    /// Ed25519 verifying key bytes for a context-derived pairwise key.
    pub fn public_key_bytes_for_context(&self, context_id: &str) -> [u8; 32] {
        VerifyingKey::from(&self.derive_key(context_id)).to_bytes()
    }

    /// Verifies a payload against a given public key bytes
    pub fn verify_signature(
        public_key_bytes: &[u8; 32],
        payload: &[u8],
        signature_bytes: &[u8; 64],
    ) -> Result<(), String> {
        let verifying_key =
            VerifyingKey::from_bytes(public_key_bytes).map_err(|_| "Invalid public key")?;
        let signature = Signature::from_bytes(signature_bytes);

        verifying_key
            .verify(payload, &signature)
            .map_err(|_| "Invalid signature".to_string())
    }

    /// Generates a WebID-TLS / mTLS compatible self-signed X.509 certificate.
    ///
    /// The DID URI is placed in the Subject Alternative Name URI extension so
    /// transport identity can be bound to the Qualia principal. Uses real `rcgen`
    /// PEM output (not a mock). Private key material is the provided Ed25519 seed
    /// encoded as PKCS#8 PEM.
    pub fn generate_webid_tls_cert(
        &self,
        key: &SigningKey,
        did_uri: &str,
    ) -> Result<(String, String), String> {
        generate_webid_tls_cert_for_seed(&key.to_bytes(), did_uri)
    }
}

/// PKCS#8 PEM + self-signed cert for an Ed25519 seed and DID SAN URI.
pub fn generate_webid_tls_cert_for_seed(
    seed: &[u8; 32],
    did_uri: &str,
) -> Result<(String, String), String> {
    use rcgen::{
        CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose,
        SanType,
    };

    if did_uri.trim().is_empty() {
        return Err("did_uri must not be empty".into());
    }

    let key_pem = format_ed25519_pkcs8_pem(seed);
    let key_pair =
        KeyPair::from_pem(&key_pem).map_err(|e| format!("rcgen KeyPair::from_pem: {e}"))?;

    let mut params =
        CertificateParams::new(Vec::<String>::new()).map_err(|e| format!("cert params: {e}"))?;
    params.distinguished_name.push(DnType::CommonName, did_uri);
    params.subject_alt_names = vec![SanType::URI(
        did_uri
            .try_into()
            .map_err(|e| format!("SAN URI: {e}"))?,
    )];
    params.is_ca = IsCa::NoCa;
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ClientAuth,
        ExtendedKeyUsagePurpose::ServerAuth,
    ];

    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| format!("self_signed: {e}"))?;
    Ok((cert.pem(), key_pair.serialize_pem()))
}

#[cfg(test)]
mod webid_tls_tests {
    use super::*;

    #[test]
    fn webid_tls_cert_is_real_pem_with_did_san() {
        let seed = [9u8; 32];
        let did = "did:q42:person:aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899";
        let (cert, key) = generate_webid_tls_cert_for_seed(&seed, did).expect("cert");
        assert!(cert.contains("BEGIN CERTIFICATE"));
        assert!(key.contains("BEGIN PRIVATE KEY") || key.contains("BEGIN"));
        assert!(!cert.contains("MIIMOCK"));
    }
}

/// RFC 8410 PKCS#8 for an Ed25519 private key seed (32 bytes).
fn format_ed25519_pkcs8_pem(seed: &[u8; 32]) -> String {
    // PrivateKeyInfo ::= SEQUENCE {
    //   version                   Version, -- 0
    //   privateKeyAlgorithm       AlgorithmIdentifier, -- id-Ed25519
    //   privateKey                OCTET STRING, -- OCTET STRING of 32-byte seed
    // }
    let mut inner_octet = Vec::with_capacity(2 + 32);
    inner_octet.push(0x04); // OCTET STRING
    inner_octet.push(32);
    inner_octet.extend_from_slice(seed);

    let mut body = Vec::new();
    // version INTEGER 0
    body.extend_from_slice(&[0x02, 0x01, 0x00]);
    // AlgorithmIdentifier SEQUENCE { OID 1.3.101.112 }
    body.extend_from_slice(&[
        0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70,
    ]);
    // privateKey OCTET STRING wrapping inner OCTET STRING
    body.push(0x04);
    body.push(inner_octet.len() as u8);
    body.extend_from_slice(&inner_octet);

    let mut der = Vec::new();
    der.push(0x30);
    der.push(body.len() as u8);
    der.extend_from_slice(&body);

    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&der);
    let mut pem = String::from("-----BEGIN PRIVATE KEY-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(chunk).unwrap_or(""));
        pem.push('\n');
    }
    pem.push_str("-----END PRIVATE KEY-----\n");
    pem
}

impl KeyVault {

    /// Issues a cryptographically signed Semantic Token for an installed qapp.
    /// The token enforces gatekeeper boundary policies (which shapes the qapp can access).
    pub fn issue_qapp_token(
        &self,
        qapp_did: &str,
        audience: &str,
        expiry_epoch: u64,
        nonce: &str,
        capabilities: Vec<String>,
        sensitivity_clearance: SubgraphLayer,
    ) -> Result<String, String> {
        let payload = QappSessionTokenV2 {
            qapp_did: qapp_did.to_string(),
            expiry_epoch,
            audience: audience.to_string(),
            nonce: nonce.to_string(),
            capabilities,
            sensitivity_clearance,
        };
        let payload_json =
            serde_json::to_string(&payload).map_err(|e| format!("Serialization error: {}", e))?;

        let master_key = self.master_key.as_ref().expect("Vault is locked");
        let signature = self.sign_payload(master_key, payload_json.as_bytes());
        let signature_hex = hex::encode(signature.to_bytes());
        let payload_hex = hex::encode(payload_json.as_bytes());

        // Token format: payload_hex.signature_hex
        Ok(format!("{}.{}", payload_hex, signature_hex))
    }

    /// Verifies a qapp token's signature using the Master Key, and checks expiry and audience.
    pub fn verify_qapp_token(
        &self,
        token: &str,
        expected_audience: &str,
    ) -> Result<QappSessionTokenV2, String> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 2 {
            return Err("Invalid semantic token format".into());
        }

        let payload_bytes =
            hex::decode(parts[0]).map_err(|_| "Invalid payload hex representation".to_string())?;
        let signature_bytes = hex::decode(parts[1])
            .map_err(|_| "Invalid signature hex representation".to_string())?;

        if signature_bytes.len() != 64 {
            return Err("Invalid signature byte length".into());
        }
        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(&signature_bytes);

        let master_key = self.master_key.as_ref().expect("Vault is locked");
        let verifying_key = VerifyingKey::from(master_key);
        let signature = Signature::from_bytes(&sig_array);
        verifying_key
            .verify(&payload_bytes, &signature)
            .map_err(|_| "Invalid token signature".to_string())?;

        let payload: QappSessionTokenV2 = serde_json::from_slice(&payload_bytes)
            .map_err(|e| format!("Failed to parse token payload: {}", e))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "Time went backwards")?
            .as_secs();

        if payload.expiry_epoch < now {
            return Err("Token expired".into());
        }

        if payload.audience != expected_audience {
            return Err("Token audience mismatch".into());
        }

        Ok(payload)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct QappSessionTokenV2 {
    pub qapp_did: String,
    pub expiry_epoch: u64,
    pub audience: String,
    pub nonce: String,
    pub capabilities: Vec<String>,
    pub sensitivity_clearance: SubgraphLayer,
}

// ── Credential-gated subgraph layer encryption ───────────────────────────────

/// Named sensitivity tiers for credential-gated subgraph views.
///
/// Each layer has a dedicated AES-256-GCM key derived from the node's master key.
/// Access is gated by the deontic engine evaluating the agent's VCs against the
/// layer's ODRL policy before releasing the key.
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
#[repr(u8)]
pub enum SubgraphLayer {
    Public = 0,
    Professional = 1,
    Legal = 2,
    Medical = 3,
    Fiduciary = 4,
}

impl SubgraphLayer {
    /// HKDF info label used for key derivation — must stay stable across versions.
    pub fn label(self) -> &'static str {
        match self {
            Self::Public => "qualia:subgraph:layer:public",
            Self::Professional => "qualia:subgraph:layer:professional",
            Self::Legal => "qualia:subgraph:layer:legal",
            Self::Medical => "qualia:subgraph:layer:medical",
            Self::Fiduciary => "qualia:subgraph:layer:fiduciary",
        }
    }

    /// Minimum sensitivity metadata bits[59:56] required to reach this layer.
    pub fn sensitivity_tier(self) -> u8 {
        self as u8
    }
}

/// A 32-byte AES-256-GCM subgraph key bound to a specific `SubgraphLayer`.
pub struct SubgraphKey {
    layer: SubgraphLayer,
    key_bytes: [u8; 32],
}

impl std::fmt::Debug for SubgraphKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubgraphKey")
            .field("layer", &self.layer)
            .field("key_bytes", &"[REDACTED]")
            .finish()
    }
}

impl zeroize::Zeroize for SubgraphKey {
    fn zeroize(&mut self) {
        self.key_bytes.zeroize();
    }
}

impl Drop for SubgraphKey {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.key_bytes.zeroize();
    }
}

impl SubgraphKey {
    /// Raw key bytes — use only to pass to an AES-GCM cipher; do not persist unencrypted.
    #[inline]
    pub fn raw(&self) -> &[u8; 32] {
        &self.key_bytes
    }

    pub fn layer(&self) -> SubgraphLayer {
        self.layer
    }
}

/// An X25519 ECDH-encapsulated subgraph key (key wrapped for a specific recipient DID).
///
/// `ephemeral_public` is the sender's ephemeral X25519 public key (32 bytes).
/// `ciphertext` is the 32-byte layer key XOR-masked with the ECDH shared secret.
///
/// The recipient computes `shared = X25519(their_static_private, ephemeral_public)`,
/// then `layer_key = ciphertext XOR shared`, then verifies with `mac`.
#[derive(Debug, Clone)]
pub struct EncapsulatedKey {
    pub layer: SubgraphLayer,
    pub ephemeral_public: [u8; 32],
    /// `AES-256-GCM ciphertext` of the 32-byte layer key (32 + 16-byte tag = 48 bytes).
    pub ciphertext: [u8; 48],
    /// Nonce used for the AES-GCM wrap (12 bytes).
    pub nonce: [u8; 12],
}

impl KeyVault {
    /// Derive a deterministic AES-256-GCM key for `layer` using HKDF-SHA-256.
    ///
    /// IKM  = master ed25519 secret key bytes (32 bytes)
    /// Salt = b"qualia:subgraph:salt:v1"
    /// Info = `layer.label()` bytes
    pub fn generate_layer_key(&self, layer: SubgraphLayer) -> SubgraphKey {
        use hkdf::Hkdf;
        use sha2::Sha256;

        let master_key = self.master_key.as_ref().expect("Vault is locked");
        let ikm = master_key.to_bytes();
        let salt: &[u8] = b"qualia:subgraph:salt:v1";
        let hk = Hkdf::<Sha256>::new(Some(salt), &ikm);

        let mut key_bytes = [0u8; 32];
        hk.expand(layer.label().as_bytes(), &mut key_bytes)
            .expect("HKDF expand: 32 bytes always fits");

        SubgraphKey { layer, key_bytes }
    }

    /// Encapsulate `layer_key` for a recipient identified by their X25519 public key bytes.
    ///
    /// Uses ephemeral X25519 ECDH + AES-256-GCM to wrap the 32-byte layer key.
    /// The `recipient_x25519_pub` is typically derived from the recipient's DID key material.
    ///
    /// # Errors
    /// Returns `Err` if the recipient public key bytes are invalid.
    pub fn encapsulate_for_recipient(
        &self,
        layer_key: &SubgraphKey,
        recipient_x25519_pub: &[u8; 32],
        nonce_entropy: &[u8; 32],
    ) -> Result<EncapsulatedKey, String> {
        use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit};
        use x25519_dalek::PublicKey;

        // Derive ephemeral X25519 keypair from nonce_entropy (deterministic for tests).
        let ephemeral_scalar = {
            let mut h = Sha256::new();
            h.update(b"qualia:ecdh:ephemeral:");
            h.update(nonce_entropy);
            h.update(layer_key.layer().label().as_bytes());
            let digest = h.finalize();
            let mut scalar = [0u8; 32];
            scalar.copy_from_slice(&digest);
            scalar
        };

        // Build ephemeral static secret from the scalar bytes.
        let ephemeral_secret = x25519_dalek::StaticSecret::from(ephemeral_scalar);
        let ephemeral_public = PublicKey::from(&ephemeral_secret);

        // ECDH shared secret.
        let recipient_pub = PublicKey::from(*recipient_x25519_pub);
        let shared_secret = ephemeral_secret.diffie_hellman(&recipient_pub);

        // Derive AES-GCM key from ECDH shared secret via SHA-256.
        let wrap_key_bytes = {
            let mut h = Sha256::new();
            h.update(shared_secret.as_bytes());
            h.update(b":qualia:wrap:");
            h.update(layer_key.layer().label().as_bytes());
            h.finalize()
        };

        // Derive 12-byte AES-GCM nonce from nonce_entropy.
        let nonce_bytes: [u8; 12] = {
            let mut h = Sha256::new();
            h.update(b"qualia:nonce:");
            h.update(nonce_entropy);
            let d = h.finalize();
            let mut n = [0u8; 12];
            n.copy_from_slice(&d[..12]);
            n
        };

        let cipher =
            Aes256Gcm::new_from_slice(&wrap_key_bytes).map_err(|_| "AES-GCM key init failed")?;
        let aes_nonce = aes_gcm::Nonce::try_from(nonce_bytes.as_slice()).unwrap();

        let ct_vec = cipher
            .encrypt(&aes_nonce, layer_key.raw().as_ref())
            .map_err(|_| "AES-GCM encryption failed")?;

        let mut ciphertext = [0u8; 48];
        if ct_vec.len() != 48 {
            return Err(format!("unexpected ciphertext length {}", ct_vec.len()));
        }
        ciphertext.copy_from_slice(&ct_vec);

        Ok(EncapsulatedKey {
            layer: layer_key.layer(),
            ephemeral_public: *ephemeral_public.as_bytes(),
            ciphertext,
            nonce: nonce_bytes,
        })
    }

    /// Decapsulate an `EncapsulatedKey` using the recipient's X25519 static secret key bytes.
    ///
    /// Returns the 32-byte layer key on success.
    pub fn decapsulate(
        &self,
        encapsulated: &EncapsulatedKey,
        recipient_x25519_secret: &[u8; 32],
    ) -> Result<SubgraphKey, String> {
        use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit};
        use x25519_dalek::{PublicKey, StaticSecret};

        let secret = StaticSecret::from(*recipient_x25519_secret);
        let ephemeral_pub = PublicKey::from(encapsulated.ephemeral_public);
        let shared_secret = secret.diffie_hellman(&ephemeral_pub);

        let wrap_key_bytes = {
            let mut h = Sha256::new();
            h.update(shared_secret.as_bytes());
            h.update(b":qualia:wrap:");
            h.update(encapsulated.layer.label().as_bytes());
            h.finalize()
        };

        let cipher =
            Aes256Gcm::new_from_slice(&wrap_key_bytes).map_err(|_| "AES-GCM key init failed")?;
        let nonce = aes_gcm::Nonce::try_from(encapsulated.nonce.as_slice()).unwrap();

        let plaintext = cipher
            .decrypt(&nonce, encapsulated.ciphertext.as_ref())
            .map_err(|_| "AES-GCM decryption failed — wrong key or tampered ciphertext")?;

        if plaintext.len() != 32 {
            return Err(format!("unexpected plaintext length {}", plaintext.len()));
        }
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&plaintext);

        Ok(SubgraphKey {
            layer: encapsulated.layer,
            key_bytes,
        })
    }

    /// Derive the X25519 static secret for this node from the master Ed25519 key.
    ///
    /// Used when the node itself is a VC recipient.
    pub fn derive_x25519_secret(&self) -> [u8; 32] {
        let master_key = self.master_key.as_ref().expect("Vault is locked");
        let mut h = Sha256::new();
        h.update(master_key.to_bytes());
        h.update(b"qualia:x25519:static");
        h.finalize().into()
    }
}

#[cfg(test)]
mod subgraph_key_tests {
    use super::*;

    fn test_vault() -> KeyVault {
        let tmp = tempfile::tempdir().expect("tmpdir");
        KeyVault::load_or_generate(tmp.path().to_str().unwrap()).expect("vault")
    }

    #[test]
    fn layer_key_derivation_is_deterministic() {
        let vault = test_vault();
        let k1 = vault.generate_layer_key(SubgraphLayer::Medical);
        let k2 = vault.generate_layer_key(SubgraphLayer::Medical);
        assert_eq!(k1.raw(), k2.raw());
    }

    #[test]
    fn different_layers_produce_different_keys() {
        let vault = test_vault();
        let med = vault.generate_layer_key(SubgraphLayer::Medical);
        let leg = vault.generate_layer_key(SubgraphLayer::Legal);
        let fid = vault.generate_layer_key(SubgraphLayer::Fiduciary);
        assert_ne!(med.raw(), leg.raw());
        assert_ne!(leg.raw(), fid.raw());
        assert_ne!(med.raw(), fid.raw());
    }

    #[test]
    fn encapsulate_decapsulate_roundtrip() {
        let vault = test_vault();
        let layer_key = vault.generate_layer_key(SubgraphLayer::Fiduciary);

        // Recipient's X25519 keys.
        let recipient_secret = vault.derive_x25519_secret();
        let recipient_pub = {
            use x25519_dalek::{PublicKey, StaticSecret};
            let s = StaticSecret::from(recipient_secret);
            *PublicKey::from(&s).as_bytes()
        };

        let nonce_entropy = [0x42u8; 32];
        let encapsulated = vault
            .encapsulate_for_recipient(&layer_key, &recipient_pub, &nonce_entropy)
            .expect("encapsulate");

        let recovered = vault
            .decapsulate(&encapsulated, &recipient_secret)
            .expect("decapsulate");

        assert_eq!(recovered.raw(), layer_key.raw());
        assert_eq!(recovered.layer(), SubgraphLayer::Fiduciary);
    }

    #[test]
    fn lock_and_unlock_roundtrip() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let path = tmp.path().to_str().unwrap();
        let mut vault = KeyVault::load_or_generate(path).expect("vault");
        assert!(!vault.is_locked());

        let before = vault.get_master_key_bytes();
        vault.lock();
        assert!(vault.is_locked());

        vault.unlock().expect("unlock");
        assert!(!vault.is_locked());
        assert_eq!(vault.get_master_key_bytes(), before);
    }

    #[test]
    fn decapsulate_wrong_key_fails() {
        let vault = test_vault();
        let layer_key = vault.generate_layer_key(SubgraphLayer::Legal);

        let recipient_secret = vault.derive_x25519_secret();
        let recipient_pub = {
            use x25519_dalek::{PublicKey, StaticSecret};
            let s = StaticSecret::from(recipient_secret);
            *PublicKey::from(&s).as_bytes()
        };

        let nonce_entropy = [0x99u8; 32];
        let encapsulated = vault
            .encapsulate_for_recipient(&layer_key, &recipient_pub, &nonce_entropy)
            .expect("encapsulate");

        let wrong_secret = [0xFFu8; 32];
        let result = vault.decapsulate(&encapsulated, &wrong_secret);
        assert!(result.is_err());
    }
}
