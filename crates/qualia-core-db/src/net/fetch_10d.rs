//! Remote `.10d`-by-hash fetch and verification

use crate::container_10d;
use crate::net::disclosure::NetworkDisclosureRegistry;

pub struct Fetch10dService {
    // In a real implementation this would hold WebTorrent or HTTP clients.
}

impl Fetch10dService {
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

    fn mock_network_fetch(&self, _endpoint: &str, _hash: &str) -> Result<Vec<u8>, String> {
        // Return a dummy valid 10d container payload
        // Real implementation would make the network call
        Ok(vec![0; 64]) // Just returning a mock payload for test structure
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

        // Now it should pass the consent check, but the mock payload might fail CRC.
        // We just assert we don't get the consent error.
        let res2 = fetcher.fetch_10d_by_hash(hash, endpoint, &registry);
        assert!(res2.is_err());
        assert!(res2.unwrap_err().contains("CRC"));
    }
}
