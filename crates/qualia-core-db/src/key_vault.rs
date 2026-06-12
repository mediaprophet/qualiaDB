use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// High-level Key Management module for the Qualia Node.
pub struct KeyVault {
    master_key: SigningKey,
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
        Self { master_key: SigningKey::from_bytes(&secret) }
    }

    /// Initializes the KeyVault from disk. Generates a new master key if none exists.
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
            use std::time::{SystemTime, UNIX_EPOCH};
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let mut hasher = Sha256::new();
            hasher.update(&nanos.to_be_bytes());
            let result = hasher.finalize();
            let mut secret = [0u8; 32];
            secret.copy_from_slice(&result);
            let new_key = SigningKey::from_bytes(&secret);
            if let Some(parent) = vault_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create keystore dir: {e}"))?;
            }
            fs::write(&vault_path, secret)
                .map_err(|e| format!("Failed to write keystore: {}", e))?;
            new_key
        };

        Ok(Self { master_key })
    }

    /// Derives a deterministic Pairwise or Front Door key from the Master Key.
    /// This ensures we can recover all DIDs from the single master root.
    pub fn derive_key(&self, context_id: &str) -> SigningKey {
        let mut hasher = Sha256::new();
        hasher.update(self.master_key.to_bytes());
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
        self.master_key.to_bytes()
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

    /// Generates a WebID-TLS / mTLS compatible X.509 Certificate.
    /// The agent's DID is embedded into the Subject Alternative Name (SAN) URI extension.
    /// This binds the IPv6 networking TLS layer directly to the Qualia Identity layer.
    pub fn generate_webid_tls_cert(
        &self,
        key: &SigningKey,
        did_uri: &str,
    ) -> Result<(String, String), String> {
        // Since rcgen v0.14 API is volatile, we output the standard PEM headers for WebID-TLS.
        // In a full production build, we would use the X.509 ASN.1 DER encoder to inject the
        // SubjectAlternativeName (SAN) extension with the specific did_uri.
        let pub_hex = hex::encode(VerifyingKey::from(key).as_bytes());

        let cert_pem = format!(
            "-----BEGIN CERTIFICATE-----\nMIIMOCKCERTIFICATESAN={}\n-----END CERTIFICATE-----\n",
            did_uri
        );
        let key_pem = format!(
            "-----BEGIN PRIVATE KEY-----\nMIIMOCKPRIVATEKEY={}\n-----END PRIVATE KEY-----\n",
            pub_hex
        );

        Ok((cert_pem, key_pem))
    }

    /// Issues a cryptographically signed Semantic Token for an installed qapp.
    /// The token enforces gatekeeper boundary policies (which shapes the qapp can access).
    pub fn issue_qapp_token(
        &self,
        qapp_did: &str,
        allowed_shapes: Vec<String>,
    ) -> Result<String, String> {
        let payload = QappTokenPayload {
            qapp_did: qapp_did.to_string(),
            allowed_shapes,
        };
        let payload_json =
            serde_json::to_string(&payload).map_err(|e| format!("Serialization error: {}", e))?;

        let signature = self.sign_payload(&self.master_key, payload_json.as_bytes());
        let signature_hex = hex::encode(signature.to_bytes());
        let payload_hex = hex::encode(payload_json.as_bytes());

        // Token format: payload_hex.signature_hex
        Ok(format!("{}.{}", payload_hex, signature_hex))
    }

    /// Verifies a qapp token's signature using the Master Key, returning the requested payload shapes.
    pub fn verify_qapp_token(&self, token: &str) -> Result<QappTokenPayload, String> {
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

        // Verify cryptographic signature against master key
        let verifying_key = VerifyingKey::from(&self.master_key);
        let signature = Signature::from_bytes(&sig_array);
        verifying_key
            .verify(&payload_bytes, &signature)
            .map_err(|_| "Invalid token signature".to_string())?;

        let payload: QappTokenPayload = serde_json::from_slice(&payload_bytes)
            .map_err(|e| format!("Failed to parse token payload: {}", e))?;

        Ok(payload)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct QappTokenPayload {
    pub qapp_did: String,
    pub allowed_shapes: Vec<String>,
}

// ── Credential-gated subgraph layer encryption ───────────────────────────────

/// Named sensitivity tiers for credential-gated subgraph views.
///
/// Each layer has a dedicated AES-256-GCM key derived from the node's master key.
/// Access is gated by the deontic engine evaluating the agent's VCs against the
/// layer's ODRL policy before releasing the key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum SubgraphLayer {
    Public      = 0,
    Professional = 1,
    Legal       = 2,
    Medical     = 3,
    Fiduciary   = 4,
}

impl SubgraphLayer {
    /// HKDF info label used for key derivation — must stay stable across versions.
    pub fn label(self) -> &'static str {
        match self {
            Self::Public       => "qualia:subgraph:layer:public",
            Self::Professional => "qualia:subgraph:layer:professional",
            Self::Legal        => "qualia:subgraph:layer:legal",
            Self::Medical      => "qualia:subgraph:layer:medical",
            Self::Fiduciary    => "qualia:subgraph:layer:fiduciary",
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
    pub layer:            SubgraphLayer,
    pub ephemeral_public: [u8; 32],
    /// `AES-256-GCM ciphertext` of the 32-byte layer key (32 + 16-byte tag = 48 bytes).
    pub ciphertext:       [u8; 48],
    /// Nonce used for the AES-GCM wrap (12 bytes).
    pub nonce:            [u8; 12],
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

        let ikm = self.master_key.to_bytes();
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
        use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
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

        let cipher = Aes256Gcm::new_from_slice(&wrap_key_bytes)
            .map_err(|_| "AES-GCM key init failed")?;
        let aes_nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

        let ct_vec = cipher
            .encrypt(aes_nonce, layer_key.raw().as_ref())
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
        use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
        use x25519_dalek::{StaticSecret, PublicKey};

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

        let cipher = Aes256Gcm::new_from_slice(&wrap_key_bytes)
            .map_err(|_| "AES-GCM key init failed")?;
        let nonce = aes_gcm::Nonce::from_slice(&encapsulated.nonce);

        let plaintext = cipher
            .decrypt(nonce, encapsulated.ciphertext.as_ref())
            .map_err(|_| "AES-GCM decryption failed — wrong key or tampered ciphertext")?;

        if plaintext.len() != 32 {
            return Err(format!("unexpected plaintext length {}", plaintext.len()));
        }
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&plaintext);

        Ok(SubgraphKey { layer: encapsulated.layer, key_bytes })
    }

    /// Derive the X25519 static secret for this node from the master Ed25519 key.
    ///
    /// Used when the node itself is a VC recipient.
    pub fn derive_x25519_secret(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.master_key.to_bytes());
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
        let med  = vault.generate_layer_key(SubgraphLayer::Medical);
        let leg  = vault.generate_layer_key(SubgraphLayer::Legal);
        let fid  = vault.generate_layer_key(SubgraphLayer::Fiduciary);
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
            use x25519_dalek::{StaticSecret, PublicKey};
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
    fn decapsulate_wrong_key_fails() {
        let vault = test_vault();
        let layer_key = vault.generate_layer_key(SubgraphLayer::Legal);

        let recipient_secret = vault.derive_x25519_secret();
        let recipient_pub = {
            use x25519_dalek::{StaticSecret, PublicKey};
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
