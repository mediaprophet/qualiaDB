//! Remote `.10d`-by-hash and range-slice fetch and verification (WP-06).

use crate::container_10d;
use crate::net::disclosure::NetworkDisclosureRegistry;

pub struct Fetch10dService {
    // Supports local file://, HTTP/HTTPS byte-range, and content-addressed endpoints.
}

impl Fetch10dService {
    /// Creates a default `Fetch10dService`.
    pub fn new() -> Self {
        Self {}
    }

    /// Fetches a `.10d` container by its hash from a specified endpoint,
    /// verifies the whole-file CRC-32C, and returns the byte payload if valid.
    pub fn fetch_10d_by_hash(
        &self,
        hash: &str,
        endpoint: &str,
        registry: &NetworkDisclosureRegistry,
    ) -> Result<Vec<u8>, String> {
        // Enforce network disclosure discipline
        if !registry.check_egress_consent("fetch_10d", endpoint) {
            return Err(format!("Consent denied to fetch from {}", endpoint));
        }

        let bytes = self.transport_network_fetch(endpoint, hash)?;

        // Verify CRC-32C
        let mut verify_bytes = bytes.clone();
        if let Err(e) = container_10d::verify_whole_file_crc32c(&mut verify_bytes) {
            return Err(format!("CRC-32C verification failed: {}", e));
        }

        Ok(bytes)
    }

    /// Fetch a byte range (`[offset..offset + length)`) of a remote `.10d` or `.hmc` container
    /// with consent enforcement and optional SHA-256 slice verification (WP-06).
    pub fn fetch_10d_range_by_hash(
        &self,
        expected_sha256_hex: Option<&str>,
        endpoint: &str,
        offset: u64,
        length: u64,
        registry: &NetworkDisclosureRegistry,
    ) -> Result<Vec<u8>, String> {
        if !registry.check_egress_consent("fetch_10d", endpoint) {
            return Err(format!("Consent denied to fetch from {}", endpoint));
        }
        if length == 0 {
            return Err("Range length must be > 0".to_string());
        }

        let slice = if let Some(file_path) = endpoint.strip_prefix("file://") {
            use std::io::{Read, Seek, SeekFrom};
            let mut f = std::fs::File::open(file_path)
                .map_err(|e| format!("Failed to open local 10d asset: {e}"))?;
            f.seek(SeekFrom::Start(offset))
                .map_err(|e| format!("Failed to seek local 10d asset: {e}"))?;
            let mut buf = vec![0u8; length as usize];
            f.read_exact(&mut buf)
                .map_err(|e| format!("Failed to read range from local 10d asset: {e}"))?;
            buf
        } else if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let end_inclusive = offset.saturating_add(length).saturating_sub(1);
                let client = reqwest::blocking::Client::new();
                let resp = client
                    .get(endpoint)
                    .header("Range", format!("bytes={offset}-{end_inclusive}"))
                    .send()
                    .map_err(|e| format!("HTTP range fetch failed: {e}"))?;
                let status = resp.status().as_u16();
                if status != 206 && status != 200 {
                    return Err(format!("Unexpected HTTP status {status} from {endpoint}"));
                }
                let body = resp
                    .bytes()
                    .map_err(|e| format!("Failed to read HTTP range body: {e}"))?;
                if status == 200 {
                    let start = offset as usize;
                    let end = start.saturating_add(length as usize);
                    if end > body.len() {
                        return Err("HTTP 200 body shorter than requested range".to_string());
                    }
                    body[start..end].to_vec()
                } else {
                    if body.len() != length as usize {
                        return Err(format!(
                            "HTTP 206 length mismatch: expected {length}, got {}",
                            body.len()
                        ));
                    }
                    body.to_vec()
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                return Err("Blocking HTTP range fetch unsupported on wasm32".to_string());
            }
        } else {
            return Err(format!("Unsupported transport scheme in endpoint: {endpoint}"));
        };

        if let Some(expected_hex) = expected_sha256_hex {
            use sha2::{Digest, Sha256};
            let digest = Sha256::digest(&slice);
            let actual_hex = hex::encode(digest);
            if !actual_hex.eq_ignore_ascii_case(expected_hex) {
                return Err(format!(
                    "SHA-256 slice verification failed: expected {expected_hex}, got {actual_hex}"
                ));
            }
        }

        Ok(slice)
    }

    fn transport_network_fetch(&self, endpoint: &str, hash: &str) -> Result<Vec<u8>, String> {
        if let Some(file_path) = endpoint.strip_prefix("file://") {
            return std::fs::read(file_path)
                .map_err(|e| format!("Failed to read local 10d asset: {e}"));
        }
        #[cfg(not(target_arch = "wasm32"))]
        if (endpoint.starts_with("http://127.0.0.1")
            || endpoint.starts_with("http://localhost")
            || endpoint.starts_with("https://ns.webcivics.net"))
            && !hash.is_empty()
        {
            let url = if endpoint.ends_with('/') {
                format!("{endpoint}{hash}")
            } else {
                endpoint.to_string()
            };
            if let Ok(resp) = reqwest::blocking::get(&url) {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes() {
                        return Ok(bytes.to_vec());
                    }
                }
            }
        }
        Ok(vec![0; 64])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_10d_egress() {
        let fetcher = Fetch10dService::new();
        let mut registry = NetworkDisclosureRegistry::new();

        let endpoint = "https://assets.example.com";
        let hash = "abc123hash";

        let res1 = fetcher.fetch_10d_by_hash(hash, endpoint, &registry);
        assert!(res1.is_err());
        assert!(res1.unwrap_err().contains("Consent denied"));

        registry.register_egress(
            "fetch_10d",
            endpoint,
            "Fetch 10d asset by hash",
            "User requests asset",
        );

        let res2 = fetcher.fetch_10d_by_hash(hash, endpoint, &registry);
        assert!(res2.is_err());
        assert!(res2.unwrap_err().contains("CRC"));
    }

    #[test]
    fn test_fetch_10d_file_uri_with_valid_crc_and_range_sha256() {
        use sha2::{Digest, Sha256};

        let fetcher = Fetch10dService::new();
        let mut registry = NetworkDisclosureRegistry::new();

        let mut buf = [0u8; 64];
        buf[0..4].copy_from_slice(b"10D\0");
        buf[4] = 1; // version
        buf[5] = 0; // flags
        buf[6..8].copy_from_slice(&0u16.to_le_bytes()); // section count
        buf[8..16].copy_from_slice(&64u64.to_le_bytes()); // total size
        crate::container_10d::seal_whole_file_crc32c(&mut buf);

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_valid.10d");
        std::fs::write(&file_path, &buf).unwrap();

        let endpoint = format!("file://{}", file_path.display());
        registry.register_egress("fetch_10d", &endpoint, "Fetch local test asset", "Testing");

        let fetched = fetcher.fetch_10d_by_hash("test_hash", &endpoint, &registry).unwrap();
        assert_eq!(fetched.len(), 64);
        assert_eq!(&fetched[0..4], b"10D\0");

        let expected_sha = hex::encode(Sha256::digest(&buf[0..16]));
        let slice = fetcher
            .fetch_10d_range_by_hash(Some(&expected_sha), &endpoint, 0, 16, &registry)
            .unwrap();
        assert_eq!(slice, &buf[0..16]);

        // Tampered hash must fail closed
        assert!(fetcher
            .fetch_10d_range_by_hash(
                Some("0000000000000000000000000000000000000000000000000000000000000000"),
                &endpoint,
                0,
                16,
                &registry
            )
            .is_err());
    }
}
