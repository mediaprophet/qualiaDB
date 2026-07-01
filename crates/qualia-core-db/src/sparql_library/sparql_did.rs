//! SPARQL-DID Integration
//!
//! Zero-allocation implementation of DID extension functions for SPARQL.
//! Implements Appendix B of the SPARQL-DID Integration Specification.

use crate::sparql_ast::*;
use crate::NQuin;

/// DID resolution result for the SPARQL ABI layer.
///
/// This is the **pointer-resolution** view: a `did:q42` URI is a topological
/// coordinate (see [`crate::identifier::parse_did_q42`]), and the SPARQL query
/// engine addresses DIDs as `u64` pointers. `endpoint_url` and
/// `verification_method` here are therefore `q_hash`-space pointers to the
/// human-readable strings produced by [`DIDResolver::resolve`].
///
/// Fixed-size (`#[repr(C)]`) to preserve the zero-allocation ABI used by the
/// SPARQL `did:resolve` magic-property wrapper.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DidResolutionPointer {
    pub did: u64,
    pub endpoint_url: u64,
    pub verification_method: u64,
    pub expires: u64,
}

/// DID resolution result (human-readable, string-based).
///
/// Produced by [`DIDResolver::resolve`]. This is **not** a hot path (the SPARQL
/// ABI layer uses [`DidResolutionPointer`]), so it owns `String`s and carries
/// the real, deterministic endpoint URL and verification method for a DID.
#[derive(Debug, Clone)]
pub struct DidResolutionResult {
    /// The DID that was resolved (e.g. `did:q42:z6MkpTHR8VNs`).
    pub did: String,
    /// A valid, deterministic HTTPS endpoint URL for the DID method.
    pub endpoint_url: String,
    /// The verification method identifier (a DID URI fragment or absolute URI).
    pub verification_method: String,
    /// Unix-epoch milliseconds at which the resolution was performed.
    pub resolved_at: u64,
}

/// DID signature verification result
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DidVerificationResult {
    pub did: u64,
    pub valid: bool,
    pub algorithm: u8,
}

/// DID permission check result
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DidPermissionResult {
    pub did: u64,
    pub graph: u64,
    pub has_permission: bool,
    pub permission_type: u8, // 0=read, 1=write, 2=admin
}

/// DID cache entry (fixed-size array for zero-allocation)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DidCacheEntry {
    pub did: u64,
    pub resolution: DidResolutionPointer,
    pub timestamp: u64,
    pub ttl: u32,
}

/// SPARQL-DID Handler
pub struct SparqlDidHandler<'a> {
    pub quins: &'a [NQuin],
    pub did_cache: [Option<DidCacheEntry>; 32],
    pub cache_count: u8,
}

impl<'a> SparqlDidHandler<'a> {
    pub fn new(quins: &'a [NQuin]) -> Self {
        Self {
            quins,
            did_cache: [None; 32],
            cache_count: 0,
        }
    }

    /// Resolve a `u64` DID pointer to ABI-layer endpoint/verification pointers
    /// (with caching).
    ///
    /// `endpoint_url` and `verification_method` are derived via FNV-1a over the
    /// DID's 60-bit identity bytes, salted with a method tag — **not** XOR with a
    /// magic number. They live in the same `q_hash` pointer space as the rest of
    /// the SPARQL engine. The human-readable URL and verification-method string
    /// for a DID are available via [`DIDResolver::resolve`].
    pub fn resolve_did(&mut self, did: u64) -> Result<DidResolutionPointer, String> {
        // Check cache first (zero-allocation lookup)
        for i in 0..self.cache_count as usize {
            if let Some(entry) = self.did_cache[i] {
                if entry.did == did {
                    let now = self.current_timestamp();
                    if now - entry.timestamp < entry.ttl as u64 {
                        return Ok(entry.resolution);
                    }
                }
            }
        }

        // Deterministic pointer derivation: FNV-1a over the 60-bit identity,
        // salted with a domain tag so endpoint and verification-method pointers
        // cannot collide with each other or with plain dictionary hashes.
        let id_bytes = (did & 0x0FFF_FFFF_FFFF_FFFF).to_le_bytes();
        let endpoint_url = fnv1a_tagged(b"did:endpoint:", &id_bytes);
        let verification_method = fnv1a_tagged(b"did:vm:", &id_bytes);

        let resolution = DidResolutionPointer {
            did,
            endpoint_url,
            verification_method,
            expires: self.current_timestamp() + 3600000, // 1 hour TTL
        };

        // Cache the result
        if self.cache_count < 32 {
            let entry = DidCacheEntry {
                did,
                resolution,
                timestamp: self.current_timestamp(),
                ttl: 3600,
            };
            self.did_cache[self.cache_count as usize] = Some(entry);
            self.cache_count += 1;
        }

        Ok(resolution)
    }

    /// Verify DID signature (zero-allocation using stack-allocated key frame)
    pub fn verify_signature(
        &self,
        _did: u64,
        signature: &[u8],
        data: &[u8],
    ) -> Result<DidVerificationResult, String> {
        if signature.len() != 64 {
            return Err("Invalid signature length".to_string());
        }
        
        let _ = (signature, data);
        
        #[cfg(feature = "interop-crypto")]
        {
            use ed25519_dalek::{Signature, Verifier, VerifyingKey};
            
            let mut sig_bytes = [0u8; 64];
            sig_bytes.copy_from_slice(signature);
            if let Ok(sig) = Signature::from_bytes(&sig_bytes) {
                // Fast-path: if the SPARQL query supplies the public key prepended to the data
                // (32 bytes PK + payload), we can verify it immediately at the boundary.
                if data.len() > 32 {
                    let mut pk_bytes = [0u8; 32];
                    pk_bytes.copy_from_slice(&data[0..32]);
                    if let Ok(verifying_key) = VerifyingKey::from_bytes(&pk_bytes) {
                        let valid = verifying_key.verify(&data[32..], &sig).is_ok();
                        if valid {
                            return Ok(DidVerificationResult {
                                did: _did,
                                valid: true,
                                algorithm: 1, // Ed25519
                            });
                        }
                    }
                }
            }
        }
        
        // If we reach here, either interop-crypto is disabled or the fast-path failed.
        // We do not have the public key locally, so we fail closed.
        Err("did:verify fast-path failed: no resolvable verification key is provisioned \
             in the SPARQL read-side shim. Verify via the identity/key-vault layer \
             (KeyVault::verify_signature)."
            .to_string())
    }

    /// Check DID permission for graph access
    pub fn check_permission(
        &self,
        did: u64,
        graph: u64,
        permission_type: u8,
    ) -> Result<DidPermissionResult, String> {
        // Check DID has 0x8 prefix
        if (did & 0x8000000000000000) == 0 {
            return Err("Invalid DID: missing 0x8 prefix".to_string());
        }

        let _ = (graph, permission_type);

        // FAIL CLOSED. Access control must be decided by an authority that has
        // actually evaluated the permission graph (the Webizen VM / deontic policy
        // layer), not granted unconditionally here. Returning `has_permission: true`
        // would be an authorization bypass: any caller would be granted any graph.
        Err(
            "did:permission is not available in the SPARQL query layer: \
             access-control decisions must be evaluated against the policy graph by \
             the governance layer, not granted unconditionally here."
                .to_string(),
        )
    }

    /// Authenticate with DID (strips heavy payloads at boundary)
    pub fn authenticate_did(
        &self,
        did: u64,
        auth_method: u8,
        _auth_payload: &[u8],
    ) -> Result<bool, String> {
        // Check DID has 0x8 prefix
        if (did & 0x8000000000000000) == 0 {
            return Err("Invalid DID: missing 0x8 prefix".to_string());
        }

        let _ = auth_method;

        // FAIL CLOSED. Authentication requires verifying `_auth_payload` (a JSON-LD
        // proof / VC / challenge response) against a resolved verification method.
        // This shim strips that payload at the boundary and holds no key material, so
        // it cannot authenticate anyone. Returning `Ok(true)` would authenticate every
        // caller. Route authentication through the identity layer instead.
        Err("did:auth is not available in the SPARQL query layer: \
             the authentication proof is stripped at this boundary and cannot be \
             verified here. Authenticate via the identity/key-vault layer."
            .to_string())
    }

    fn current_timestamp(&self) -> u64 {
        // In production, use actual system time
        // Simplified: return placeholder
        1234567890
    }

    /// Sign data with DID (zero-allocation)
    pub fn sign_with_did(&self, did: u64, data: &[u8]) -> Result<Vec<u8>, String> {
        // Check DID has 0x8 prefix
        if (did & 0x8000000000000000) == 0 {
            return Err("Invalid DID: missing 0x8 prefix".to_string());
        }

        // The SPARQL-DID handler is a read-side query shim: it resolves DIDs to u64
        // pointers and deliberately strips heavy crypto payloads at the boundary
        // (see `authenticate_did`). It holds NO private key material, so it cannot and
        // must not produce a signature here. Signing is the responsibility of the
        // identity / key-vault layer (e.g. `WebizenIdentityManager` over `key_vault`,
        // or `CryptographicLibrary::sign_data`), which owns the secret keys.
        //
        // Fail closed rather than returning a forged all-zero signature that would
        // falsely signal success to callers.
        let _ = data;
        Err("did:sign is not available in the SPARQL query layer: \
             no private key is provisioned here. Sign via the identity/key-vault \
             layer (WebizenIdentityManager / CryptographicLibrary::sign_data)."
            .to_string())
    }

    /// Invalidate cache entry
    pub fn invalidate_cache(&mut self, did: u64) {
        for i in 0..self.cache_count as usize {
            if let Some(entry) = self.did_cache[i] {
                if entry.did == did {
                    self.did_cache[i] = None;
                    // Compact cache
                    for j in i..self.cache_count as usize - 1 {
                        self.did_cache[j] = self.did_cache[j + 1];
                    }
                    self.cache_count -= 1;
                    return;
                }
            }
        }
    }
}

impl<'a> Default for SparqlDidHandler<'a> {
    fn default() -> Self {
        Self::new(&[])
    }
}

/// DID extension functions (Appendix B)
/// These are assigned to 0x0 standard dictionary type prefix during planning
/// to identify them as Magic Property Functions

/// did:resolve - Resolve DID to Document
///
/// ABI-layer wrapper: emits the `endpoint_url` and `verification_method`
/// **pointers** (in `q_hash` space) for the SPARQL binding. For the
/// human-readable URL, use [`DIDResolver::resolve`].
pub fn did_resolve(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let did = args[0];

    let mut handler = SparqlDidHandler::new(quins);
    match handler.resolve_did(did) {
        Ok(resolution) => {
            result.slots[0] = Some(resolution.endpoint_url);
            result.slots[1] = Some(resolution.verification_method);
            true
        }
        Err(_) => false,
    }
}

/// did:verify - Verify DID signature
pub fn did_verify(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.len() < 3 {
        return false;
    }
    let did = args[0];
    let signature_ptr = args[1];
    let data_ptr = args[2];

    let handler = SparqlDidHandler::new(quins);
    // In production, convert pointers to actual byte slices
    let signature = if signature_ptr != 0 {
        unsafe { std::slice::from_raw_parts(signature_ptr as *const u8, 64) }
    } else {
        &[0u8; 64]
    };
    
    let data = if data_ptr != 0 {
        unsafe { std::slice::from_raw_parts(data_ptr as *const u8, 256) }
    } else {
        &[0u8; 256]
    };

    match handler.verify_signature(did, signature, data) {
        Ok(verification) => {
            result.slots[0] = Some(if verification.valid { 1 } else { 0 });
            result.slots[1] = Some(verification.algorithm as u64);
            true
        }
        Err(_) => false,
    }
}

/// did:auth - Authenticate with DID
pub fn did_auth(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.len() < 2 {
        return false;
    }
    let did = args[0];
    let auth_method = args[1] as u8;

    let handler = SparqlDidHandler::new(quins);
    let auth_payload = if args.len() > 2 {
        let payload_ptr = args[2];
        if payload_ptr != 0 {
            unsafe { std::slice::from_raw_parts(payload_ptr as *const u8, 256) }
        } else {
            &[0u8; 256]
        }
    } else {
        &[0u8; 256]
    };

    match handler.authenticate_did(did, auth_method, auth_payload) {
        Ok(valid) => {
            result.slots[0] = Some(if valid { 1 } else { 0 });
            true
        }
        Err(_) => false,
    }
}

/// did:sign - Sign with DID
pub fn did_sign(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.len() < 2 {
        return false;
    }
    let did = args[0];
    let data_ptr = args[1];

    let handler = SparqlDidHandler::new(quins);
    // In production, convert pointer to actual byte slice
    let data = if data_ptr != 0 {
        unsafe { std::slice::from_raw_parts(data_ptr as *const u8, 256) }
    } else {
        &[0u8; 256]
    };

    match handler.sign_with_did(did, data) {
        Ok(_signature) => {
            result.slots[0] = Some(1); // Success indicator
            true
        }
        Err(_) => false,
    }
}

/// did:permission - Check DID permission
pub fn did_permission(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.len() < 3 {
        return false;
    }
    let did = args[0];
    let graph = args[1];
    let permission_type = args[2] as u8;

    let handler = SparqlDidHandler::new(quins);
    match handler.check_permission(did, graph, permission_type) {
        Ok(permission) => {
            result.slots[0] = Some(if permission.has_permission { 1 } else { 0 });
            result.slots[1] = Some(permission.permission_type as u64);
            true
        }
        Err(_) => false,
    }
}

/// FNV-1a over a domain tag followed by a byte payload, truncated to 60 bits.
///
/// Used by [`SparqlDidHandler::resolve_did`] to derive deterministic
/// `endpoint_url` / `verification_method` pointers from a DID's identity bits
/// without XOR-ing against a magic constant. Shares the same FNV constants and
/// 60-bit mask as [`crate::q_hash`] / [`crate::identifier::parse_did_q42`].
#[inline]
fn fnv1a_tagged(tag: &[u8], payload: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in tag {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    for &b in payload {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash & 0x0FFF_FFFF_FFFF_FFFF
}

/// Human-readable DID resolver.
///
/// Maps a DID string to a deterministic, valid HTTPS endpoint URL and
/// verification-method identifier according to the DID method's resolution
/// convention. This is **not** HTTP resolution (no network call is made); it
/// produces the URL that *would* be fetched by a universal resolver / the
/// method's native resolution endpoint. This module is not a hot path, so it
/// owns `String`s freely.
///
/// # Supported methods
/// | Method | Endpoint URL | Verification method |
/// |--------|--------------|---------------------|
/// | `did:q42:` | `https://q42.network/agents/{id_hex}` | `{did}#q42-key` |
/// | `did:web:` | `https://{domain}/.well-known/did.json` | `{did}#key-1` |
/// | `did:key:` | `https://dev.uniresolver.io/1.0/identifiers/{did}` | `{did}` |
pub struct DIDResolver;

impl DIDResolver {
    /// Construct a new resolver.
    pub fn new() -> Self {
        Self
    }

    /// Resolve a DID to a human-readable endpoint URL and verification method.
    ///
    /// Delegates to [`Self::resolve_did`]; provided as the canonical entry point
    /// named in the SPARQL-DID Integration Specification.
    pub fn resolve(&self, did: &str) -> Result<DidResolutionResult, String> {
        self.resolve_did(did)
    }

    /// Resolve a DID to a human-readable endpoint URL and verification method.
    pub fn resolve_did(&self, did: &str) -> Result<DidResolutionResult, String> {
        let resolved_at = current_epoch_millis();

        if let Some(rest) = did.strip_prefix("did:q42:") {
            if rest.is_empty() {
                return Err("did:q42 resolution failed: empty identifier".to_string());
            }
            // Parse via the identifier module to validate the DID and obtain the
            // canonical 60-bit topological pointer; the low 60 bits are the
            // identity hash, which we render as hex for the agent URL.
            let pointer = crate::identifier::parse_did_q42(did.as_bytes())
                .map_err(|e| format!("did:q42 resolution failed: {:?}", e))?;
            let id_hex = format!("{:015x}", pointer & 0x0FFF_FFFF_FFFF_FFFF);
            Ok(DidResolutionResult {
                did: did.to_string(),
                endpoint_url: format!("https://q42.network/agents/{}", id_hex),
                verification_method: format!("{}#q42-key", did),
                resolved_at,
            })
        } else if let Some(rest) = did.strip_prefix("did:web:") {
            if rest.is_empty() {
                return Err("did:web resolution failed: empty domain".to_string());
            }
            // did:web resolution: replace ':' with '/' for path components, then
            // append the standard well-known DID document location.
            let domain_path = rest.replace(':', "/");
            Ok(DidResolutionResult {
                did: did.to_string(),
                endpoint_url: format!("https://{}/.well-known/did.json", domain_path),
                verification_method: format!("{}#key-1", did),
                resolved_at,
            })
        } else if let Some(rest) = did.strip_prefix("did:key:") {
            if rest.is_empty() {
                return Err("did:key resolution failed: empty key".to_string());
            }
            // did:key embeds the public key in the DID itself; the DID is its own
            // verification method (per the did:key spec). There is no native HTTP
            // endpoint, so we point at a universal resolver for retrieval.
            Ok(DidResolutionResult {
                did: did.to_string(),
                endpoint_url: format!("https://dev.uniresolver.io/1.0/identifiers/{}", did),
                verification_method: did.to_string(),
                resolved_at,
            })
        } else if did.starts_with("did:") {
            // Unknown DID method.
            let method = did
                .get(4..)
                .and_then(|s| s.split(':').next())
                .unwrap_or("unknown");
            Err(format!(
                "DID resolution failed: unsupported DID method '{}'. \
                 Supported methods: did:q42, did:web, did:key.",
                method
            ))
        } else {
            Err(format!(
                "DID resolution failed: '{}' is not a valid DID (must start with 'did:')",
                did
            ))
        }
    }

    /// Verify a DID's authentication signature.
    ///
    /// This performs only the **structural** checks available to the read-side
    /// SPARQL query layer (that the DID is well-formed and resolvable, and that
    /// a non-empty signature and payload were supplied). The actual
    /// cryptographic verification requires public-key material that this shim
    /// deliberately does not hold — it is the responsibility of the identity /
    /// key-vault layer ([`crate::key_vault::KeyVault::verify_signature`] /
    /// `WebizenIdentityRegistry::verify_signature`).
    ///
    /// Fails closed: never reports a signature as valid from this layer.
    pub fn verify_authentication_signature(
        &self,
        did: &str,
        signature: &[u8],
        payload: &[u8],
    ) -> Result<bool, String> {
        // Structural validation: the DID must resolve (be well-formed + known method).
        self.resolve_did(did)?;

        if signature.is_empty() {
            return Err("DID signature verification failed: empty signature".to_string());
        }
        if payload.is_empty() {
            return Err("DID signature verification failed: empty payload".to_string());
        }

        // FAIL CLOSED — see the security rationale on
        // [`SparqlDidHandler::verify_signature`]. This layer holds no
        // verifiable public key, so it cannot perform a real Ed25519/ML-DSA
        // check. Returning `true` would forge a positive verification.
        Err("did:verify is not available in the SPARQL query layer: \
             no resolvable verification key is provisioned here. Verify via the \
             identity/key-vault layer (KeyVault::verify_signature / \
             WebizenIdentityRegistry::verify_signature)."
            .to_string())
    }
}

impl Default for DIDResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Current time as Unix-epoch milliseconds.
///
/// The SPARQL ABI layer (`SparqlDidHandler::current_timestamp`) returns a
/// placeholder because it must stay allocation-free and deterministic for the
/// query planner. The human-readable resolver is not a hot path, so it uses
/// real wall-clock time.
fn current_epoch_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_did_handler_creation() {
        let quins = vec![];
        let handler = SparqlDidHandler::new(&quins);
        assert_eq!(handler.cache_count, 0);
    }

    #[test]
    fn test_resolve_did() {
        let quins = vec![];
        let mut handler = SparqlDidHandler::new(&quins);

        let result = handler.resolve_did(0x8000000000000001); // With 0x8 prefix
        assert!(result.is_ok());
        assert_eq!(result.unwrap().did, 0x8000000000000001);
    }

    #[test]
    fn test_verify_signature_fails_closed() {
        // Security regression: the SPARQL/DID query shim must NOT rubber-stamp
        // signatures. It has no resolvable key here and must fail closed so callers
        // route verification through the key-vault/identity layer.
        let quins = vec![];
        let handler = SparqlDidHandler::new(&quins);

        let signature = &[0u8; 64];
        let data = &[0u8; 256];

        let result = handler.verify_signature(0x8000000000000001, signature, data);
        assert!(
            result.is_err(),
            "verify_signature must fail closed, not return valid"
        );
    }

    #[test]
    fn test_check_permission_fails_closed() {
        // Security regression: permission must not be granted unconditionally.
        let quins = vec![];
        let handler = SparqlDidHandler::new(&quins);

        let result = handler.check_permission(0x8000000000000001, 123, 0);
        assert!(
            result.is_err(),
            "check_permission must fail closed, not grant access"
        );
    }

    #[test]
    fn test_authenticate_did_fails_closed() {
        // Security regression: authentication must not succeed for everyone.
        let quins = vec![];
        let handler = SparqlDidHandler::new(&quins);

        let result = handler.authenticate_did(0x8000000000000001, 1, &[0u8; 256]);
        assert!(
            result.is_err(),
            "authenticate_did must fail closed, not authenticate all"
        );
    }

    // ===== DIDResolver (human-readable, string-based) tests =====

    #[test]
    fn test_resolver_did_q42_produces_valid_url() {
        let resolver = DIDResolver::new();
        let did = "did:q42:z6MkpTHR8VNs";
        let result = resolver.resolve(did).expect("did:q42 must resolve");

        assert_eq!(result.did, did);
        // Endpoint must be a valid HTTPS URL — not XOR garbage.
        assert!(
            result.endpoint_url.starts_with("https://q42.network/agents/"),
            "endpoint_url should be a q42.network agent URL, got '{}'",
            result.endpoint_url
        );
        // The agent id must be a non-empty hex string (the 60-bit identity).
        let agent_id = result
            .endpoint_url
            .strip_prefix("https://q42.network/agents/")
            .unwrap();
        assert!(!agent_id.is_empty(), "agent id must not be empty");
        assert!(
            agent_id.chars().all(|c| c.is_ascii_hexdigit()),
            "agent id must be hex, got '{}'",
            agent_id
        );
        assert_eq!(result.verification_method, format!("{}#q42-key", did));
        assert!(result.resolved_at > 0, "resolved_at must be set");
    }

    #[test]
    fn test_resolver_did_q42_is_deterministic() {
        let resolver = DIDResolver::new();
        let did = "did:q42:z6MkpTHR8VNs";
        let a = resolver.resolve(did).unwrap();
        let b = resolver.resolve(did).unwrap();
        assert_eq!(a.endpoint_url, b.endpoint_url, "resolution must be deterministic");
        assert_eq!(a.verification_method, b.verification_method);
    }

    #[test]
    fn test_resolver_did_q42_distinct_payloads_distinct_urls() {
        let resolver = DIDResolver::new();
        let a = resolver.resolve("did:q42:z6MkpTHR8VNs").unwrap();
        let b = resolver.resolve("did:q42:z6MkpTHR8VNt").unwrap();
        assert_ne!(a.endpoint_url, b.endpoint_url, "distinct DIDs must resolve to distinct URLs");
    }

    #[test]
    fn test_resolver_did_web_maps_to_well_known() {
        let resolver = DIDResolver::new();
        let did = "did:web:example.com";
        let result = resolver.resolve(did).expect("did:web must resolve");

        assert_eq!(result.did, did);
        assert_eq!(
            result.endpoint_url, "https://example.com/.well-known/did.json",
            "did:web must map to the standard well-known DID document URL"
        );
        assert_eq!(result.verification_method, "did:web:example.com#key-1");
    }

    #[test]
    fn test_resolver_did_web_with_path_components() {
        // did:web uses ':' as a path separator after the domain.
        let resolver = DIDResolver::new();
        let did = "did:web:example.com:users:alice";
        let result = resolver.resolve(did).unwrap();
        assert_eq!(
            result.endpoint_url,
            "https://example.com/users/alice/.well-known/did.json"
        );
    }

    #[test]
    fn test_resolver_did_key() {
        let resolver = DIDResolver::new();
        let did = "did:key:z6MkhaXgBZDvotDkL5v7wB9QkN8eYfH2";
        let result = resolver.resolve(did).expect("did:key must resolve");

        assert_eq!(result.did, did);
        // did:key has no native HTTP endpoint; point at a universal resolver.
        assert_eq!(
            result.endpoint_url,
            format!("https://dev.uniresolver.io/1.0/identifiers/{}", did)
        );
        // The did:key DID is its own verification method.
        assert_eq!(result.verification_method, did);
    }

    #[test]
    fn test_resolver_unknown_method_returns_error() {
        let resolver = DIDResolver::new();
        let result = resolver.resolve("did:foo:bar");
        assert!(result.is_err(), "unknown DID method must error");
        let err = result.unwrap_err();
        assert!(
            err.contains("unsupported DID method"),
            "error should mention unsupported method, got: {}",
            err
        );
        assert!(err.contains("foo"), "error should name the method 'foo'");
    }

    #[test]
    fn test_resolver_non_did_returns_error() {
        let resolver = DIDResolver::new();
        let result = resolver.resolve("https://example.com/not-a-did");
        assert!(result.is_err(), "non-DID input must error");
        assert!(result.unwrap_err().contains("not a valid DID"));
    }

    #[test]
    fn test_resolver_empty_q42_payload_errors() {
        let resolver = DIDResolver::new();
        assert!(resolver.resolve("did:q42:").is_err());
    }

    #[test]
    fn test_resolver_empty_web_domain_errors() {
        let resolver = DIDResolver::new();
        assert!(resolver.resolve("did:web:").is_err());
    }

    #[test]
    fn test_resolver_empty_key_errors() {
        let resolver = DIDResolver::new();
        assert!(resolver.resolve("did:key:").is_err());
    }

    #[test]
    fn test_resolver_resolve_delegates_to_resolve_did() {
        // `resolve()` and `resolve_did()` must produce identical results.
        let resolver = DIDResolver::new();
        let did = "did:web:example.com";
        let a = resolver.resolve(did).unwrap();
        let b = resolver.resolve_did(did).unwrap();
        assert_eq!(a.did, b.did);
        assert_eq!(a.endpoint_url, b.endpoint_url);
        assert_eq!(a.verification_method, b.verification_method);
    }

    #[test]
    fn test_verify_authentication_signature_fails_closed() {
        // The SPARQL query layer holds no verifiable public key, so signature
        // verification must fail closed even when the DID and payload are valid.
        let resolver = DIDResolver::new();
        let did = "did:q42:z6MkpTHR8VNs";
        let result =
            resolver.verify_authentication_signature(did, &[1u8; 64], &[2u8; 32]);
        assert!(result.is_err(), "must fail closed, not return valid");
        assert!(result.unwrap_err().contains("identity/key-vault layer"));
    }

    #[test]
    fn test_verify_authentication_signature_rejects_empty_signature() {
        let resolver = DIDResolver::new();
        let result =
            resolver.verify_authentication_signature("did:q42:z6MkpTHR8VNs", &[], &[2u8; 32]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty signature"));
    }

    #[test]
    fn test_verify_authentication_signature_rejects_bad_did() {
        let resolver = DIDResolver::new();
        // An unresolvable DID should be rejected before any crypto consideration.
        let result =
            resolver.verify_authentication_signature("did:foo:bar", &[1u8; 64], &[2u8; 32]);
        assert!(result.is_err());
    }

    // ===== ABI-layer pointer resolution (XOR placeholder removed) =====

    #[test]
    fn test_resolve_did_pointer_is_not_xor_garbage() {
        // The u64 ABI resolver must no longer use `did ^ 0xDEADBEEF`. Endpoint
        // and verification-method pointers must be distinct, deterministic, and
        // in the 60-bit q_hash space (top 4 bits clear).
        let quins = vec![];
        let mut handler = SparqlDidHandler::new(&quins);
        let did = 0x8000000000000001;
        let result = handler.resolve_did(did).unwrap();

        // Distinct from the old XOR placeholders.
        assert_ne!(result.endpoint_url, did ^ 0xDEADBEEF);
        assert_ne!(result.verification_method, did ^ 0xCAFEBABE);
        // Endpoint and verification method must differ from each other.
        assert_ne!(result.endpoint_url, result.verification_method);
        // Must be deterministic.
        let again = handler.resolve_did(did).unwrap_or_else(|_| {
            handler.invalidate_cache(did);
            handler.resolve_did(did).unwrap()
        });
        // (cache returns the same value; if cache hit, fields are equal anyway)
        assert_eq!(result.endpoint_url, again.endpoint_url);
        // Pointers live in the 60-bit identity space (top 4 bits clear).
        assert_eq!(result.endpoint_url >> 60, 0);
        assert_eq!(result.verification_method >> 60, 0);
    }
}
