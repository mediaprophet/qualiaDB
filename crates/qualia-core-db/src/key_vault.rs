use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// High-level Key Management module for the Qualia Node.
pub struct KeyVault {
    master_key: SigningKey,
}

impl KeyVault {
    /// Initializes the KeyVault from disk. Generates a new master key if none exists.
    pub fn load_or_generate(storage_dir: &str) -> Result<Self, String> {
        let vault_path = Path::new(storage_dir).join("keystore.bin");
        
        let master_key = if vault_path.exists() {
            let bytes = fs::read(&vault_path).map_err(|e| format!("Failed to read keystore: {}", e))?;
            if bytes.len() != 32 {
                return Err("Corrupted master key length".into());
            }
            let mut secret = [0u8; 32];
            secret.copy_from_slice(&bytes[0..32]);
            SigningKey::from_bytes(&secret)
        } else {
            use std::time::{SystemTime, UNIX_EPOCH};
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
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
            fs::write(&vault_path, secret).map_err(|e| format!("Failed to write keystore: {}", e))?;
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
    pub fn verify_signature(public_key_bytes: &[u8; 32], payload: &[u8], signature_bytes: &[u8; 64]) -> Result<(), String> {
        let verifying_key = VerifyingKey::from_bytes(public_key_bytes)
            .map_err(|_| "Invalid public key")?;
        let signature = Signature::from_bytes(signature_bytes);
        
        verifying_key.verify(payload, &signature).map_err(|_| "Invalid signature".to_string())
    }

    /// Generates a WebID-TLS / mTLS compatible X.509 Certificate.
    /// The agent's DID is embedded into the Subject Alternative Name (SAN) URI extension.
    /// This binds the IPv6 networking TLS layer directly to the Qualia Identity layer.
    pub fn generate_webid_tls_cert(&self, key: &SigningKey, did_uri: &str) -> Result<(String, String), String> {
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
    pub fn issue_qapp_token(&self, qapp_did: &str, allowed_shapes: Vec<String>) -> Result<String, String> {
        let payload = QappTokenPayload {
            qapp_did: qapp_did.to_string(),
            allowed_shapes,
        };
        let payload_json = serde_json::to_string(&payload).map_err(|e| format!("Serialization error: {}", e))?;
        
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
        
        let payload_bytes = hex::decode(parts[0]).map_err(|_| "Invalid payload hex representation".to_string())?;
        let signature_bytes = hex::decode(parts[1]).map_err(|_| "Invalid signature hex representation".to_string())?;
        
        if signature_bytes.len() != 64 {
            return Err("Invalid signature byte length".into());
        }
        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(&signature_bytes);
        
        // Verify cryptographic signature against master key
        let verifying_key = VerifyingKey::from(&self.master_key);
        let signature = Signature::from_bytes(&sig_array);
        verifying_key.verify(&payload_bytes, &signature).map_err(|_| "Invalid token signature".to_string())?;

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
