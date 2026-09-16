//! Local catalogue of sealed semantic-instrument collectables (SI-07).
//!
//! Discovery records are mutable; package bytes are not. Listing is not
//! endorsement. Empty (interrupted) bytes never register.

use std::collections::BTreeMap;

use super::canonical::sha256_prefixed;
use super::category::ContentCategory;
use super::errors::InstrumentError;
use super::package::open_collectable;
use super::resolve::{DependencyLock, InstalledRelease, LocalRegistry};

/// Catalogue discovery record. `endorsed` stays false unless `endorse` is called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogueListing {
    pub release_id: String,
    pub name: String,
    pub purpose: String,
    pub content_category: ContentCategory,
    pub licence: String,
    pub byte_length: u64,
    pub content_digest: String,
    pub listed: bool,
    /// MUST default false; listing does not set this.
    pub endorsed: bool,
}

impl Default for CatalogueListing {
    fn default() -> Self {
        Self {
            release_id: String::new(),
            name: String::new(),
            purpose: String::new(),
            content_category: ContentCategory::Demo,
            licence: String::new(),
            byte_length: 0,
            content_digest: String::new(),
            listed: false,
            endorsed: false,
        }
    }
}

struct StoredRelease {
    listing: CatalogueListing,
    bytes: Vec<u8>,
}

/// In-memory local catalogue. No network; bytes are the published HMC.
#[derive(Default)]
pub struct LocalCatalogue {
    by_id: BTreeMap<String, StoredRelease>,
}

/// Guard: a listed record is not an endorsement unless `endorsed` was set.
pub fn assert_not_endorsement(listing: &CatalogueListing) -> Result<(), InstrumentError> {
    if listing.listed && !listing.endorsed {
        Err(InstrumentError::CatalogueIsNotEndorsement)
    } else {
        Ok(())
    }
}

impl LocalCatalogue {
    pub fn new() -> Self {
        Self {
            by_id: BTreeMap::new(),
        }
    }

    /// Publish sealed HMC bytes. Same `release_id` + different digest is
    /// `AlreadyPublished`. Empty bytes are `IncompleteDownload` and do not
    /// register. Does not mark endorsed.
    pub fn publish(&mut self, bytes: Vec<u8>) -> Result<CatalogueListing, InstrumentError> {
        if bytes.is_empty() {
            return Err(InstrumentError::IncompleteDownload);
        }
        let digest = sha256_prefixed(&bytes);
        let opened = open_collectable(&bytes)?;
        let release_id = opened.manifest.release_id.clone();
        if let Some(existing) = self.by_id.get(&release_id) {
            if existing.listing.content_digest != digest {
                return Err(InstrumentError::AlreadyPublished);
            }
            return Ok(existing.listing.clone());
        }
        let listing = CatalogueListing {
            release_id: release_id.clone(),
            name: opened.manifest.name.clone(),
            purpose: opened.manifest.purpose.clone(),
            content_category: opened.manifest.content_category,
            licence: opened.manifest.licence.clone(),
            byte_length: bytes.len() as u64,
            content_digest: digest,
            listed: true,
            endorsed: false,
        };
        self.by_id.insert(
            release_id,
            StoredRelease {
                listing: listing.clone(),
                bytes,
            },
        );
        Ok(listing)
    }

    /// Listings in deterministic `release_id` order.
    pub fn discover(&self) -> Vec<CatalogueListing> {
        self.by_id.values().map(|e| e.listing.clone()).collect()
    }

    /// Registered bytes for a release. Missing is `RequiredMissing`.
    pub fn download(&self, release_id: &str) -> Result<&[u8], InstrumentError> {
        self.by_id
            .get(release_id)
            .map(|e| e.bytes.as_slice())
            .ok_or(InstrumentError::RequiredMissing)
    }

    /// Verify stored bytes match the listing digest, then install. Closed iff
    /// the lock is empty or the caller passes `closed`.
    pub fn verify_and_collect(
        &self,
        release_id: &str,
        registry: &mut LocalRegistry,
        closed: bool,
    ) -> Result<(), InstrumentError> {
        let stored = self
            .by_id
            .get(release_id)
            .ok_or(InstrumentError::RequiredMissing)?;
        if sha256_prefixed(&stored.bytes) != stored.listing.content_digest {
            return Err(InstrumentError::DigestMismatch);
        }
        let lock = DependencyLock {
            release_id: stored.listing.release_id.clone(),
            entries: Vec::new(),
        };
        let closed = lock.is_empty() || closed;
        registry.install(InstalledRelease {
            release_id: stored.listing.release_id.clone(),
            content_digest: stored.listing.content_digest.clone(),
            lock,
            closed,
        })
    }

    /// Endorsement is a separate call; listing never sets this flag.
    pub fn endorse(&mut self, release_id: &str) -> Result<(), InstrumentError> {
        let stored = self
            .by_id
            .get_mut(release_id)
            .ok_or(InstrumentError::RequiredMissing)?;
        stored.listing.endorsed = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_instruments::demo::build_demo;
    use crate::semantic_instruments::package::{
        build_collectable, open_collectable, CollectableParts,
    };

    fn unit_convert_bytes() -> Vec<u8> {
        build_demo("unit-convert").expect("unit-convert demo")
    }

    fn variant_same_release(bytes: &[u8]) -> Vec<u8> {
        let opened = open_collectable(bytes).unwrap();
        let mut definition_n3 = opened.definition_n3;
        definition_n3.extend_from_slice(b"\n# catalogue-variant\n");
        build_collectable(CollectableParts {
            manifest: opened.manifest,
            definition_n3,
            hcf: opened.hcf,
            visual_10d: opened.visual_10d,
            small_q42: opened.small_q42,
        })
        .unwrap()
    }

    #[test]
    fn publish_discover_download_verify_collect() {
        let bytes = unit_convert_bytes();
        let mut cat = LocalCatalogue::new();
        let listing = cat.publish(bytes.clone()).unwrap();
        assert!(listing.listed);
        assert!(!listing.endorsed);
        assert_eq!(listing.byte_length, bytes.len() as u64);
        assert_eq!(listing.content_digest, sha256_prefixed(&bytes));

        let discovered = cat.discover();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].release_id, listing.release_id);

        let downloaded = cat.download(&listing.release_id).unwrap();
        assert_eq!(downloaded, bytes.as_slice());
        assert_eq!(sha256_prefixed(downloaded), listing.content_digest);

        let mut registry = LocalRegistry::new();
        cat.verify_and_collect(&listing.release_id, &mut registry, true)
            .unwrap();
        let installed = registry.lookup(&listing.release_id).expect("collected");
        assert_eq!(installed.content_digest, listing.content_digest);
        assert!(installed.closed);
        assert!(installed.lock.is_empty());
    }

    #[test]
    fn republish_different_bytes_is_already_published() {
        let bytes = unit_convert_bytes();
        let mut cat = LocalCatalogue::new();
        let first = cat.publish(bytes.clone()).unwrap();
        let same = cat.publish(bytes.clone()).unwrap();
        assert_eq!(same.content_digest, first.content_digest);

        let other = variant_same_release(&bytes);
        assert_ne!(sha256_prefixed(&other), first.content_digest);
        assert_eq!(
            cat.publish(other),
            Err(InstrumentError::AlreadyPublished)
        );
        assert_eq!(cat.discover().len(), 1);
        assert_eq!(
            cat.download(&first.release_id).unwrap(),
            bytes.as_slice()
        );
    }

    #[test]
    fn empty_publish_is_incomplete_and_does_not_register() {
        let mut cat = LocalCatalogue::new();
        assert_eq!(
            cat.publish(Vec::new()),
            Err(InstrumentError::IncompleteDownload)
        );
        assert!(cat.discover().is_empty());
    }

    #[test]
    fn listed_is_not_endorsed_until_endorse() {
        let mut cat = LocalCatalogue::new();
        let listing = cat.publish(unit_convert_bytes()).unwrap();
        assert!(listing.listed);
        assert!(!listing.endorsed);
        assert_eq!(
            assert_not_endorsement(&listing),
            Err(InstrumentError::CatalogueIsNotEndorsement)
        );
        assert!(!cat.discover()[0].endorsed);

        cat.endorse(&listing.release_id).unwrap();
        let after = cat.discover();
        assert_eq!(after.len(), 1);
        assert!(after[0].listed);
        assert!(after[0].endorsed);
        assert_eq!(assert_not_endorsement(&after[0]), Ok(()));
    }

    #[test]
    fn listing_exposes_licence_and_byte_length() {
        let bytes = unit_convert_bytes();
        let listing = LocalCatalogue::new().publish(bytes).unwrap();
        assert!(!listing.licence.is_empty());
        assert!(listing.byte_length > 0);
        assert!(listing.content_digest.starts_with("sha256:"));
    }
}
