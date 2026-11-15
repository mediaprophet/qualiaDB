//! Browser HMC mesh playback and resident-upload bookkeeping.
//!
//! The bundle reader owns byte validation and zero-copy entry access. This module owns the
//! smaller lifecycle contract needed by the browser portal: resolve one verified `.10d` mesh,
//! upload it at most once for a resident source digest, and retain enough receipt data for a
//! playback caller to distinguish a new upload from a resident reuse.

use crate::bundle::BundleReader;

/// A verified `.10d` entry borrowed from an HMC bundle.
#[derive(Debug)]
pub struct HmcMeshSource<'a> {
    pub bytes: &'a [u8],
    pub digest: [u8; 32],
}

/// Resolve and verify one mesh entry without copying the HMC payload.
pub fn resolve_hmc_mesh<'a>(bytes: &'a [u8], asset_key: &str) -> Result<HmcMeshSource<'a>, String> {
    let bundle = BundleReader::parse(bytes).map_err(|error| format!("HMC bundle: {error}"))?;
    let entry = bundle
        .entry(asset_key)
        .ok_or_else(|| format!("HMC asset is missing: {asset_key}"))?;
    if entry.kind != "10d" {
        return Err(format!(
            "HMC asset {asset_key} has kind {:?}, expected 10d",
            entry.kind
        ));
    }
    if entry.sha256.len() != 32 || !bundle.verify_entry(asset_key) {
        return Err(format!("HMC asset digest check failed: {asset_key}"));
    }
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&entry.sha256);
    let payload = bundle
        .get(asset_key)
        .ok_or_else(|| format!("HMC asset payload is missing: {asset_key}"))?;
    Ok(HmcMeshSource {
        bytes: payload,
        digest,
    })
}

/// Receipt returned by the resident mesh lifecycle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HmcMeshResidencyReceipt {
    pub digest: [u8; 32],
    pub generation: u64,
    pub upload_count: u32,
    pub vertex_count: u32,
    pub index_count: u32,
    pub triangle_count: u32,
}

/// Tracks the one mesh currently bound by the browser portal.
#[derive(Default)]
pub struct HmcMeshResidency {
    resident: Option<HmcMeshResidencyReceipt>,
    generation: u64,
    upload_count: u32,
}

impl HmcMeshResidency {
    /// Return the existing receipt when the requested source is already resident.
    pub fn resident(&self, digest: [u8; 32]) -> Option<HmcMeshResidencyReceipt> {
        self.resident.filter(|receipt| receipt.digest == digest)
    }

    /// Return the current receipt, regardless of source digest.
    pub fn current(&self) -> Option<HmcMeshResidencyReceipt> {
        self.resident
    }

    /// Record a successful replacement upload and advance the lifecycle generation.
    pub fn record_upload(
        &mut self,
        digest: [u8; 32],
        vertex_count: u32,
        index_count: u32,
        triangle_count: u32,
    ) -> HmcMeshResidencyReceipt {
        self.generation = self.generation.saturating_add(1);
        self.upload_count = self.upload_count.saturating_add(1);
        let receipt = HmcMeshResidencyReceipt {
            digest,
            generation: self.generation,
            upload_count: self.upload_count,
            vertex_count,
            index_count,
            triangle_count,
        };
        self.resident = Some(receipt);
        receipt
    }

    /// Invalidate the source identity when another mesh API replaces the portal mesh.
    pub fn clear(&mut self) {
        self.resident = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::BundleWriter;

    #[test]
    fn resolves_verified_mesh_without_copying_payload() {
        let payload = b"verified-10d".to_vec();
        let mut writer = BundleWriter::new();
        writer
            .add_file("scene.10d", "10d", payload.clone(), None)
            .unwrap();
        let bundle = writer.build().unwrap();

        let source = resolve_hmc_mesh(&bundle, "scene.10d").unwrap();
        assert_eq!(source.bytes, payload.as_slice());
        assert_ne!(source.digest, [0; 32]);
    }

    #[test]
    fn rejects_non_mesh_entry() {
        let mut writer = BundleWriter::new();
        writer
            .add_file("scene.q42", "q42", b"graph".to_vec(), None)
            .unwrap();
        let bundle = writer.build().unwrap();

        let error = resolve_hmc_mesh(&bundle, "scene.q42").unwrap_err();
        assert!(error.contains("expected 10d"));
    }

    #[test]
    fn resident_lifecycle_reuses_and_replaces_uploads() {
        let mut residency = HmcMeshResidency::default();
        let first_digest = [1; 32];
        let second_digest = [2; 32];

        assert!(residency.resident(first_digest).is_none());
        let first = residency.record_upload(first_digest, 3, 6, 2);
        assert_eq!(first.generation, 1);
        assert_eq!(first.upload_count, 1);
        assert_eq!(residency.resident(first_digest), Some(first));
        assert!(residency.resident(second_digest).is_none());

        let second = residency.record_upload(second_digest, 4, 9, 3);
        assert_eq!(second.generation, 2);
        assert_eq!(second.upload_count, 2);
        assert!(residency.resident(first_digest).is_none());
        assert_eq!(residency.resident(second_digest), Some(second));

        residency.clear();
        assert!(residency.resident(second_digest).is_none());
    }
}
