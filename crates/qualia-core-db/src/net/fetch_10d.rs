//! Remote `.10d`-by-hash fetch and verification

use crate::container_10d;
use crate::net::disclosure::NetworkDisclosureRegistry;

pub struct Fetch10dService {
    // In a real implementation this would hold WebTorrent or HTTP clients.
}

impl Fetch10dService {
    /// Creates a default `Fetch10dService`.
    pub fn new() -> Self {
        Self {}
    }

    /// Fetches a .10d container by its hash from a specified endpoint,
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

        // Mock the fetch. In reality, we'd use `reqwest` or the `WebTorrent` seeder.
        let bytes = self.mock_network_fetch(endpoint, hash)?;

        // Verify CRC-32C
        // Because `container_10d::verify_whole_file_crc32c` expects `&mut [u8]`
        // and modifies the buffer to zeroes out the CRC field temporarily, we need it mutable.
        let mut verify_bytes = bytes.clone();
        if let Err(e) = container_10d::verify_whole_file_crc32c(&mut verify_bytes) {
            return Err(format!("CRC-32C verification failed: {}", e));
        }

        Ok(bytes)
    }

    fn mock_network_fetch(&self, endpoint: &str, _hash: &str) -> Result<Vec<u8>, String> {
        // If an explicit file or local path is passed, read it directly
        if let Some(file_path) = endpoint.strip_prefix("file://") {
            return std::fs::read(file_path).map_err(|e| format!("Failed to read local 10d asset: {e}"));
        }
        // Fallback mock payload for test structure when remote network transport is unconfigured
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

        // Without registration, it should fail
        let res1 = fetcher.fetch_10d_by_hash(hash, endpoint, &registry);
        assert!(res1.is_err());
        assert!(res1.unwrap_err().contains("Consent denied"));

        // Register the endpoint
        registry.register_egress(
            "fetch_10d",
            endpoint,
            "Fetch 10d asset by hash",
            "User requests asset",
        );

        // Now it should pass the consent check, but the mock payload fails CRC.
        let res2 = fetcher.fetch_10d_by_hash(hash, endpoint, &registry);
        assert!(res2.is_err());
        assert!(res2.unwrap_err().contains("CRC"));
    }

    #[test]
    fn test_fetch_10d_file_uri_with_valid_crc() {
        let fetcher = Fetch10dService::new();
        let mut registry = NetworkDisclosureRegistry::new();

        let mut buf = [0u8; 64];
        buf[0..4].copy_from_slice(b"10D\0");
        buf[4] = 1; // version
        buf[5] = 0; // flags
        buf[6..8].copy_from_slice(&0u16.to_le_bytes()); // section count
        buf[8..16].copy_from_slice(&64u64.to_le_bytes()); // total size
        crate::container_10d::seal_whole_file_crc32c(&mut buf);

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_valid.10d");
        std::fs::write(&file_path, &buf).unwrap();

        let endpoint = format!("file://{}", file_path.display());
        registry.register_egress("fetch_10d", &endpoint, "Fetch local test asset", "Testing");

        let fetched = fetcher.fetch_10d_by_hash("test_hash", &endpoint, &registry).unwrap();
        assert_eq!(fetched.len(), 64);
        assert_eq!(&fetched[0..4], b"10D\0");

        let _ = std::fs::remove_file(&file_path);
    }
}
