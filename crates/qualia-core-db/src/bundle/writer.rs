//! Build a `.hmc` bundle from a set of intact files.

use sha2::{Digest, Sha256};

use crate::container_10d::crc32c::crc32c;

use super::format::{
    align_up, BundleEntry, BundleError, BUNDLE_ENTRY_ALIGN, BUNDLE_FLAG_PAGE_ALIGN_4096,
    BUNDLE_HEADER_SIZE, BUNDLE_MAGIC, BUNDLE_PAGE_ALIGN_4096, BUNDLE_VERSION, OFF_CRC32C,
    OFF_ENTRY_COUNT, OFF_FLAGS, OFF_INDEX_LENGTH, OFF_INDEX_OFFSET, OFF_MAGIC, OFF_TOTAL_LENGTH,
    OFF_VERSION,
};

/// Accumulates intact files and serialises them into a `.hmc` bundle.
///
/// Files are stored **verbatim** — no compression, no transformation — each
/// page-aligned so the interior alignment (and thus zero-copy segment access)
/// of embedded `.q42` / `.p64` / `.10d` files is preserved. Entry order in the
/// index follows insertion order (deterministic bundles for a given input).
#[derive(Default)]
pub struct BundleWriter {
    // (key, kind, bytes, meta)
    entries: Vec<(String, String, Vec<u8>, Option<Vec<u8>>)>,
    align_4096: bool,
}

impl BundleWriter {
    /// Create an empty bundle writer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure whether entries and index footer are aligned to 4096-byte page boundaries.
    pub fn with_page_align_4096(mut self, enabled: bool) -> Self {
        self.align_4096 = enabled;
        self
    }

    /// Set whether entries and index footer are aligned to 4096-byte page boundaries.
    pub fn set_page_align_4096(&mut self, enabled: bool) {
        self.align_4096 = enabled;
    }

    /// Whether 4096-byte page alignment is enabled.
    pub fn is_page_align_4096(&self) -> bool {
        self.align_4096
    }

    /// Number of files added so far.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when no files have been added.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Add one intact file. `key` must be unique and non-empty. `kind` names the
    /// embedded format (`"10d"`, `"q42"`, `"p64"`, `"manifest"`, …). `meta` is an
    /// optional opaque per-entry CBOR blob (domain-specific).
    pub fn add_file(
        &mut self,
        key: impl Into<String>,
        kind: impl Into<String>,
        bytes: Vec<u8>,
        meta: Option<Vec<u8>>,
    ) -> Result<&mut Self, BundleError> {
        let key = key.into();
        if key.is_empty() {
            return Err(BundleError::EmptyKey);
        }
        if self.entries.iter().any(|(k, _, _, _)| *k == key) {
            return Err(BundleError::DuplicateKey(key));
        }
        self.entries.push((key, kind.into(), bytes, meta));
        Ok(self)
    }

    /// Add a digital twin model family manifest describing variants, sexual dimorphism,
    /// life stages, and LOD hierarchy to the bundle.
    pub fn add_family_manifest(
        &mut self,
        manifest: &super::variant_matrix::TwinFamilyManifest,
    ) -> Result<&mut Self, BundleError> {
        let bytes = manifest.encode_to_cbor()?;
        self.add_file(
            super::variant_matrix::FAMILY_MANIFEST_ENTRY_KEY,
            super::variant_matrix::FAMILY_MANIFEST_KIND,
            bytes,
            None,
        )
    }

    /// Add a W3C Verifiable Credential manifest encoded as CBOR-LD.
    pub fn add_cbor_ld_manifest(
        &mut self,
        manifest: &super::cbor_ld_manifest::CborLdManifest,
    ) -> Result<&mut Self, BundleError> {
        let bytes = manifest.encode_to_cbor()?;
        self.add_file(
            super::cbor_ld_manifest::CBOR_LD_MANIFEST_ENTRY_KEY,
            super::cbor_ld_manifest::CBOR_LD_MANIFEST_KIND,
            bytes,
            None,
        )
    }

    /// Add holistic species metadata (flora, fauna, or mechanical assembly) to the bundle.
    pub fn add_species_meta(
        &mut self,
        meta: &super::species_meta::HolisticSpeciesMeta,
    ) -> Result<&mut Self, BundleError> {
        let bytes = meta.encode_to_cbor()?;
        self.add_file(
            super::species_meta::SPECIES_META_ENTRY_KEY,
            super::species_meta::SPECIES_META_KIND,
            bytes,
            None,
        )
    }

    /// Serialise the bundle to a `Vec<u8>`.
    pub fn build(&self) -> Result<Vec<u8>, BundleError> {
        let mut out = vec![0u8; BUNDLE_HEADER_SIZE];
        let mut index: Vec<BundleEntry> = Vec::with_capacity(self.entries.len());
        let align = if self.align_4096 {
            BUNDLE_PAGE_ALIGN_4096
        } else {
            BUNDLE_ENTRY_ALIGN
        };

        for (key, kind, bytes, meta) in &self.entries {
            // Page-align the entry so its interior stays aligned.
            let padded = align_up(out.len(), align);
            out.resize(padded, 0);
            let offset = out.len() as u64;
            out.extend_from_slice(bytes);

            let mut hasher = Sha256::new();
            hasher.update(bytes);
            let sha256 = hasher.finalize().to_vec();

            index.push(BundleEntry {
                key: key.clone(),
                kind: kind.clone(),
                offset,
                length: bytes.len() as u64,
                sha256,
                meta: meta.clone(),
            });
        }

        // Page-align the index footer too (tidy range fetches of the directory).
        let padded = align_up(out.len(), align);
        out.resize(padded, 0);
        let index_offset = out.len() as u64;

        let mut index_bytes = Vec::new();
        ciborium::into_writer(&index, &mut index_bytes)
            .map_err(|e| BundleError::Cbor(e.to_string()))?;
        out.extend_from_slice(&index_bytes);
        let index_length = index_bytes.len() as u64;
        let total_length = out.len() as u64;
        let flags: u16 = if self.align_4096 {
            BUNDLE_FLAG_PAGE_ALIGN_4096
        } else {
            0
        };

        // Fill the header (CRC field stays zero for the CRC computation).
        out[OFF_MAGIC..OFF_MAGIC + 4].copy_from_slice(&BUNDLE_MAGIC);
        out[OFF_VERSION..OFF_VERSION + 2].copy_from_slice(&BUNDLE_VERSION.to_le_bytes());
        out[OFF_FLAGS..OFF_FLAGS + 2].copy_from_slice(&flags.to_le_bytes());
        out[OFF_ENTRY_COUNT..OFF_ENTRY_COUNT + 4]
            .copy_from_slice(&(self.entries.len() as u32).to_le_bytes());
        out[OFF_INDEX_OFFSET..OFF_INDEX_OFFSET + 8].copy_from_slice(&index_offset.to_le_bytes());
        out[OFF_INDEX_LENGTH..OFF_INDEX_LENGTH + 8].copy_from_slice(&index_length.to_le_bytes());
        out[OFF_TOTAL_LENGTH..OFF_TOTAL_LENGTH + 8].copy_from_slice(&total_length.to_le_bytes());

        // Whole-file CRC-32C over the buffer with the CRC field == 0.
        let crc = crc32c(&out);
        out[OFF_CRC32C..OFF_CRC32C + 4].copy_from_slice(&crc.to_le_bytes());

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_and_duplicate_keys() {
        let mut w = BundleWriter::new();
        assert!(matches!(
            w.add_file("", "10d", vec![1], None),
            Err(BundleError::EmptyKey)
        ));
        w.add_file("a", "10d", vec![1, 2], None).unwrap();
        assert!(matches!(
            w.add_file("a", "q42", vec![3], None),
            Err(BundleError::DuplicateKey(_))
        ));
    }

    #[test]
    fn entries_are_page_aligned() {
        let mut w = BundleWriter::new();
        w.add_file("a", "10d", vec![0xAB; 7], None).unwrap();
        w.add_file("b", "10d", vec![0xCD; 130], None).unwrap();
        let bytes = w.build().unwrap();
        let reader = super::super::reader::BundleReader::parse(&bytes).unwrap();
        for e in reader.entries() {
            assert_eq!(
                e.offset % BUNDLE_ENTRY_ALIGN as u64,
                0,
                "entry {} unaligned",
                e.key
            );
        }
    }

    #[test]
    fn entries_are_4096_page_aligned_when_enabled() {
        let mut w = BundleWriter::new().with_page_align_4096(true);
        w.add_file("a.10d", "10d", vec![0xAB; 7], None).unwrap();
        w.add_file("b.m64", "m64", vec![0xCD; 130], None).unwrap();
        let bytes = w.build().unwrap();
        let reader = super::super::reader::BundleReader::parse(&bytes).unwrap();
        assert!(reader.is_page_aligned_4096());
        for e in reader.entries() {
            assert_eq!(
                e.offset % BUNDLE_PAGE_ALIGN_4096 as u64,
                0,
                "entry {} not 4096-byte aligned",
                e.key
            );
        }
    }
}
