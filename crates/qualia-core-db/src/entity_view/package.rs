//! Bifurcated socio-cognitive package wings (layout + digests).
//!
//! Skeleton for private vs offered/commons representation - not encryption by itself.

use super::entity_id::EntityId;
use super::observer::RepresentationWing;
use crate::hypermedia::fnv60;
use serde::{Deserialize, Serialize};

/// One wing of a bifurcated package (fixed header; bodies referenced by digest).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageWing {
    pub wing: RepresentationWing,
    /// Entities included in this wing (bounded list in host; here free Vec for cold path).
    pub entity_ids: Vec<u64>,
    /// Content digest of wing payload bytes (0 if empty).
    pub payload_digest: u64,
}

/// Bifurcated presentation / socio-cognitive package v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BifurcatedPackage {
    pub version: u32,
    pub package_id: u64,
    pub private_wing: PackageWing,
    pub offered_wing: PackageWing,
    pub commons_wing: PackageWing,
    /// Combined digest of the three wing digests (ordering fixed).
    pub package_digest: u64,
}

impl BifurcatedPackage {
    pub const VERSION: u32 = 1;

    pub fn new(
        package_key: &str,
        private_ids: &[EntityId],
        offered_ids: &[EntityId],
        commons_ids: &[EntityId],
    ) -> Self {
        let private_wing = wing(RepresentationWing::Private, private_ids);
        let offered_wing = wing(RepresentationWing::Offered, offered_ids);
        let commons_wing = wing(RepresentationWing::Commons, commons_ids);
        let package_id = fnv60(package_key.as_bytes());
        let package_digest = fold_digests(&[
            private_wing.payload_digest,
            offered_wing.payload_digest,
            commons_wing.payload_digest,
            package_id,
        ]);
        Self {
            version: Self::VERSION,
            package_id,
            private_wing,
            offered_wing,
            commons_wing,
            package_digest,
        }
    }

    /// Select wing for observer without exposing other wing entity lists to caller policy.
    pub fn wing_for(&self, wing: RepresentationWing) -> &PackageWing {
        match wing {
            RepresentationWing::Private => &self.private_wing,
            RepresentationWing::Offered => &self.offered_wing,
            RepresentationWing::Commons => &self.commons_wing,
        }
    }
}

fn wing(kind: RepresentationWing, ids: &[EntityId]) -> PackageWing {
    let entity_ids: Vec<u64> = ids.iter().map(|e| e.raw()).collect();
    let mut bytes = Vec::with_capacity(entity_ids.len() * 8 + 1);
    bytes.push(kind as u8);
    for id in &entity_ids {
        bytes.extend_from_slice(&id.to_le_bytes());
    }
    PackageWing {
        wing: kind,
        payload_digest: if entity_ids.is_empty() {
            0
        } else {
            fnv60(&bytes)
        },
        entity_ids,
    }
}

fn fold_digests(parts: &[u64]) -> u64 {
    let mut bytes = Vec::with_capacity(parts.len() * 8);
    for p in parts {
        bytes.extend_from_slice(&p.to_le_bytes());
    }
    fnv60(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_view::entity_id::EntityId;

    #[test]
    fn bifurcate_package_stable_digest() {
        let a = EntityId::from_uri("urn:a");
        let b = EntityId::from_uri("urn:b");
        let p1 = BifurcatedPackage::new("pkg:1", &[a], &[b], &[]);
        let p2 = BifurcatedPackage::new("pkg:1", &[a], &[b], &[]);
        assert_eq!(p1.package_digest, p2.package_digest);
        assert_eq!(p1.private_wing.entity_ids.len(), 1);
        assert_eq!(p1.offered_wing.entity_ids.len(), 1);
        assert!(p1.commons_wing.entity_ids.is_empty());
    }
}
