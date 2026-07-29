//! Network disclosure discipline for outbound connections.
//!
//! Enforces that no adapter driver can reach out to the internet without
//! explicit, pre-registered consent for its recipient and purpose.

use std::collections::HashMap;

/// An outbound network endpoint that an adapter requires access to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RegisteredEndpoint {
    pub url: String,
    pub purpose: String,
    pub trigger: String,
}

/// The registry controlling egress to external layers.
pub struct NetworkDisclosureRegistry {
    /// Maps adapter ID (e.g. "dem_adapter") to its permitted endpoints
    allowed_endpoints: HashMap<String, Vec<RegisteredEndpoint>>,
}

impl NetworkDisclosureRegistry {
    pub fn new() -> Self {
        Self {
            allowed_endpoints: HashMap::new(),
        }
    }

    /// Registers a permitted endpoint for an adapter.
    pub fn register_egress(&mut self, adapter_id: &str, url: &str, purpose: &str, trigger: &str) {
        let entry = self
            .allowed_endpoints
            .entry(adapter_id.to_string())
            .or_insert_with(Vec::new);
        entry.push(RegisteredEndpoint {
            url: url.to_string(),
            purpose: purpose.to_string(),
            trigger: trigger.to_string(),
        });
    }

    /// Fail-closed check before initiating `reqwest` or `/proxy/fetch`.
    ///
    /// Returns true if the endpoint is registered, false otherwise.
    pub fn check_egress_consent(&self, adapter_id: &str, target_url: &str) -> bool {
        if let Some(endpoints) = self.allowed_endpoints.get(adapter_id) {
            for ep in endpoints {
                if target_url.starts_with(&ep.url) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disclosure_registry() {
        let mut registry = NetworkDisclosureRegistry::new();

        registry.register_egress(
            "dem_adapter",
            "https://elevation.example.com",
            "Fetch Cloud-Optimized GeoTIFF for terrain",
            "User pans camera to quadrant",
        );

        // Exact match
        assert!(registry.check_egress_consent("dem_adapter", "https://elevation.example.com"));
        // Prefix match
        assert!(registry.check_egress_consent(
            "dem_adapter",
            "https://elevation.example.com/tile/1/2/3.tif"
        ));

        // Unregistered endpoint for the adapter
        assert!(!registry.check_egress_consent("dem_adapter", "https://tracker.example.com/log"));

        // Unregistered adapter
        assert!(!registry.check_egress_consent("unknown_adapter", "https://elevation.example.com"));
    }
}
