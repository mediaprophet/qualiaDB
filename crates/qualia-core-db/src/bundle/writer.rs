//! Build a `.qualia` bundle from a set of intact files.

use sha2::{Digest, Sha256};

use crate::container_10d::crc32c::crc32c;

use super::format::{
    align_up, BundleEntry, BundleError, BUNDLE_ENTRY_ALIGN, BUNDLE_HEADER_SIZE, BUNDLE_MAGIC,
    BUNDLE_VERSION, OFF_CRC32C, OFF_ENTRY_COUNT, OFF_INDEX_LENGTH, OFF_INDEX_OFFSET, OFF_MAGIC,
    OFF_TOTAL_LENGTH, OFF_VERSION,
};

/// Accumulates intact files and serialises them into a `.qualia` bundle.
///
/// Files are stored **verbatim** — no compression, no transformation — each
/// page-aligned so the interior alignment (and thus zero-copy segment access)
/// of embedded `.q42` / `.p64` / `.10d` files is preserved. Entry order in the
/// index follows insertion order (deterministic bundles for a given input).
#[derive(Default)]
pub struct BundleWriter {
    // (key, kind, bytes, meta)
    entries: Vec<(String, String, Vec<u8>, Option<Vec<u8>>)>,
}

impl BundleWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of files added so far.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

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

    /// Serialise the bundle to a `Vec<u8>`.
    pub fn build(&self) -> Result<Vec<u8>, BundleError> {
        let mut out = vec![0u8; BUNDLE_HEADER_SIZE];
        let mut index: Vec<BundleEntry> = Vec::with_capacity(self.entries.len());

        for (key, kind, bytes, meta) in &self.entries {
            // Page-align the entry so its interior stays aligned.
            let padded = align_up(out.len(), BUNDLE_ENTRY_ALIGN);
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
        let padded = align_up(out.len(), BUNDLE_ENTRY_ALIGN);
        out.resize(padded, 0);
        let index_offset = out.len() as u64;

        let mut index_bytes = Vec::new();
        ciborium::into_writer(&index, &mut index_bytes)
            .map_err(|e| BundleError::Cbor(e.to_string()))?;
        out.extend_from_slice(&index_bytes);
        let index_length = index_bytes.len() as u64;
        let total_length = out.len() as u64;

        // Fill the header (CRC field stays zero for the CRC computation).
        out[OFF_MAGIC..OFF_MAGIC + 4].copy_from_slice(&BUNDLE_MAGIC);
        out[OFF_VERSION..OFF_VERSION + 2].copy_from_slice(&BUNDLE_VERSION.to_le_bytes());
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
            assert_eq!(e.offset % BUNDLE_ENTRY_ALIGN as u64, 0, "entry {} unaligned", e.key);
        }
    }
}
