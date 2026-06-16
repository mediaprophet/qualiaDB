//! Sanctuary Lane Cryptography - PBKDF2 Key Derivation and Nonce Management
//!
//! Implements 48-byte key derivation for sanctuary lanes with implicit domain-separated
//! nonce derivation. Eliminates nonce storage and provides zero-heap hot path optimization.

#[cfg(feature = "sanctuary-crypto")]
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

/// Derived key material for sanctuary lanes
#[derive(Debug, Clone)]
pub struct SanctuaryKeyMaterial {
    /// 32-byte cipher key for AEAD encryption
    pub cipher_key: [u8; 32],
    /// 16-byte volume root tweak for nonce derivation
    pub volume_tweak: [u8; 16],
}

/// Derives 48 bytes of key material from a PIN and salt using PBKDF2
/// Returns [32 bytes cipher key | 16 bytes volume root tweak]
///
/// # Arguments
/// * `pin` - User PIN or passphrase
/// * `salt` - Unique salt for the sanctuary volume
/// * `iterations` - PBKDF2 iterations (310,000 recommended for production)
///
/// # Returns
/// * `Ok(SanctuaryKeyMaterial)` - Derived key material
/// * `Err(String)` - Derivation failure
#[cfg(feature = "sanctuary-crypto")]
pub fn derive_sanctuary_key_material(
    pin: &[u8],
    salt: &[u8],
    iterations: u32,
) -> Result<SanctuaryKeyMaterial, String> {
    // Derive 48 bytes: [32 bytes key | 16 bytes tweak]
    let mut key_material = [0u8; 48];
    
    pbkdf2_hmac::<Sha256>(
        pin,
        salt,
        iterations,
        &mut key_material,
    );
    
    // Split into cipher key and volume tweak
    let mut cipher_key = [0u8; 32];
    let mut volume_tweak = [0u8; 16];
    
    cipher_key.copy_from_slice(&key_material[0..32]);
    volume_tweak.copy_from_slice(&key_material[32..48]);
    
    Ok(SanctuaryKeyMaterial {
        cipher_key,
        volume_tweak,
    })
}

/// Derives a nonce for a specific chunk or block using XOR-based implicit derivation
/// 
/// Formula: Per_Chunk_Nonce = Volume_Root_Tweak ⊕ (Chunk_Index_or_Offset)
///
/// # Arguments
/// * `volume_tweak` - 16-byte volume root tweak from PBKDF2 derivation
/// * `chunk_index_or_offset` - 8-byte chunk index or byte offset (converted to 16 bytes)
///
/// # Returns
/// * 12-byte nonce for AES-256-GCM or XChaCha20-Poly1305
pub fn derive_chunk_nonce(volume_tweak: &[u8; 16], chunk_index_or_offset: u64) -> [u8; 12] {
    // Convert the 8-byte index/offset to a 16-byte value for XOR
    let index_bytes: [u8; 16] = {
        let mut arr = [0u8; 16];
        arr[8..].copy_from_slice(&chunk_index_or_offset.to_le_bytes());
        arr
    };
    
    // XOR the volume tweak with the index bytes
    let mut nonce_bytes = [0u8; 16];
    for i in 0..16 {
        nonce_bytes[i] = volume_tweak[i] ^ index_bytes[i];
    }
    
    // Return first 12 bytes for standard AEAD nonces
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_bytes[0..12]);
    nonce
}

/// Derives a 24-byte nonce for XChaCha20-Poly1305 (extended nonce mode)
/// 
/// # Arguments
/// * `volume_tweak` - 16-byte volume root tweak from PBKDF2 derivation
/// * `chunk_index_or_offset` - 8-byte chunk index or byte offset
///
/// # Returns
/// * 24-byte nonce for XChaCha20-Poly1305
pub fn derive_xchacha_nonce(volume_tweak: &[u8; 16], chunk_index_or_offset: u64) -> [u8; 24] {
    // For XChaCha20, we use the full 16-byte XOR result as the first 16 bytes
    // and extend with 8 more bytes from the index for the extended nonce
    let index_bytes: [u8; 16] = {
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&chunk_index_or_offset.to_le_bytes());
        arr
    };
    
    let mut nonce_bytes = [0u8; 24];
    // First 16 bytes: XOR of volume tweak and index
    for i in 0..16 {
        nonce_bytes[i] = volume_tweak[i] ^ index_bytes[i];
    }
    // Last 8 bytes: Additional index material
    nonce_bytes[16..24].copy_from_slice(&chunk_index_or_offset.to_le_bytes());
    
    nonce_bytes
}

/// Encrypt data using sanctuary lane cryptography with derived nonce
/// This integrates with the existing AEAD ciphers while using implicit nonce derivation
///
/// # Arguments
/// * `cipher_key` - 32-byte cipher key from PBKDF2 derivation
/// * `volume_tweak` - 16-byte volume root tweak from PBKDF2 derivation
/// * `chunk_index` - Chunk index or byte offset for nonce derivation
/// * `plaintext` - Data to encrypt
/// * `additional_data` - Additional authenticated data (AAD)
///
/// # Returns
/// * `Ok((ciphertext, tag, nonce))` - Encrypted data with authentication tag and used nonce
/// * `Err(String)` - Encryption failure
#[cfg(feature = "sanctuary-crypto")]
pub fn encrypt_sanctuary_chunk(
    cipher_key: &[u8; 32],
    volume_tweak: &[u8; 16],
    chunk_index: u64,
    plaintext: &[u8],
    additional_data: Option<&[u8]>,
) -> Result<(Vec<u8>, Vec<u8>, [u8; 12]), String> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };
    
    // Derive nonce for this chunk
    let nonce = derive_chunk_nonce(volume_tweak, chunk_index);
    let nonce_obj = Nonce::from_slice(&nonce);
    
    // Initialize cipher
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(cipher_key);
    let cipher = Aes256Gcm::new(key);
    
    // Encrypt with AAD if provided
    let ciphertext = if let Some(aad) = additional_data {
        cipher.encrypt(nonce_obj, aead::Payload { msg: plaintext, aad })
            .map_err(|e| format!("AES-256-GCM encryption failed: {}", e))?
    } else {
        cipher.encrypt(nonce_obj, plaintext)
            .map_err(|e| format!("AES-256-GCM encryption failed: {}", e))?
    };
    
    // Split ciphertext and tag (AES-GCM appends 16-byte tag)
    if ciphertext.len() < 16 {
        return Err("Ciphertext too short to contain tag".to_string());
    }
    
    let tag_start = ciphertext.len() - 16;
    let data = ciphertext[..tag_start].to_vec();
    let tag = ciphertext[tag_start..].to_vec();
    
    Ok((data, tag, nonce))
}

/// Decrypt data using sanctuary lane cryptography with derived nonce
///
/// # Arguments
/// * `cipher_key` - 32-byte cipher key from PBKDF2 derivation
/// * `volume_tweak` - 16-byte volume root tweak from PBKDF2 derivation
/// * `chunk_index` - Chunk index or byte offset for nonce derivation
/// * `ciphertext` - Encrypted data
/// * `tag` - Authentication tag
/// * `additional_data` - Additional authenticated data (AAD)
///
/// # Returns
/// * `Ok(plaintext)` - Decrypted data
/// * `Err(String)` - Decryption failure
#[cfg(feature = "sanctuary-crypto")]
pub fn decrypt_sanctuary_chunk(
    cipher_key: &[u8; 32],
    volume_tweak: &[u8; 16],
    chunk_index: u64,
    ciphertext: &[u8],
    tag: &[u8],
    additional_data: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };
    
    // Derive nonce for this chunk
    let nonce = derive_chunk_nonce(volume_tweak, chunk_index);
    let nonce_obj = Nonce::from_slice(&nonce);
    
    // Initialize cipher
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(cipher_key);
    let cipher = Aes256Gcm::new(key);
    
    // Combine ciphertext and tag for decryption
    let mut encrypted = ciphertext.to_vec();
    encrypted.extend_from_slice(tag);
    
    // Decrypt with AAD if provided
    let plaintext = if let Some(aad) = additional_data {
        cipher.decrypt(nonce_obj, aead::Payload { msg: &encrypted, aad })
            .map_err(|e| format!("AES-256-GCM decryption failed: {}", e))?
    } else {
        cipher.decrypt(nonce_obj, encrypted.as_ref())
            .map_err(|e| format!("AES-256-GCM decryption failed: {}", e))?
    };
    
    Ok(plaintext)
}

#[cfg(test)]
#[cfg(feature = "sanctuary-crypto")]
mod tests {
    
    #[test]
    fn test_sanctuary_key_derivation() {
        let pin = b"test_pin_123";
        let salt = b"test_salt_456";
        let iterations = 1000; // Reduced for test speed
        
        let result = derive_sanctuary_key_material(pin, salt, iterations).unwrap();
        
        // Verify cipher key and tweak are different
        assert_ne!(result.cipher_key, result.volume_tweak);
        assert_eq!(result.cipher_key.len(), 32);
        assert_eq!(result.volume_tweak.len(), 16);
        
        // Verify deterministic derivation
        let result2 = derive_sanctuary_key_material(pin, salt, iterations).unwrap();
        assert_eq!(result.cipher_key, result2.cipher_key);
        assert_eq!(result.volume_tweak, result2.volume_tweak);
    }
    
    #[test]
    fn test_nonce_derivation() {
        let volume_tweak = [1u8; 16];
        let chunk_index = 42u64;
        
        let nonce1 = derive_chunk_nonce(&volume_tweak, chunk_index);
        let nonce2 = derive_chunk_nonce(&volume_tweak, chunk_index + 1);
        
        // Verify nonces are different for different chunks
        assert_ne!(nonce1, nonce2);
        
        // Verify deterministic derivation
        let nonce1_again = derive_chunk_nonce(&volume_tweak, chunk_index);
        assert_eq!(nonce1, nonce1_again);
    }
    
    #[test]
    fn test_xchacha_nonce_derivation() {
        let volume_tweak = [1u8; 16];
        let chunk_index = 42u64;
        
        let nonce = derive_xchacha_nonce(&volume_tweak, chunk_index);
        
        assert_eq!(nonce.len(), 24);
        assert_ne!(nonce[0..16], nonce[16..24]);
    }
    
    #[test]
    fn test_nonce_uniqueness_across_volume() {
        let volume_tweak = [1u8; 16];
        
        // Test first 1000 chunks for uniqueness
        let mut nonces = std::collections::HashSet::new();
        for i in 0..1000 {
            let nonce = derive_chunk_nonce(&volume_tweak, i);
            assert!(nonces.insert(nonce), "Duplicate nonce found at index {}", i);
        }
    }
}