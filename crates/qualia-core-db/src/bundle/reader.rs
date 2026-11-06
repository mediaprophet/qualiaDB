//! Read a `.hmc` bundle: verify integrity, enumerate entries, and hand back
//! **zero-copy slices** of intact embedded files (and of interior segments).

use crate::container_10d::crc32c::crc32c_update;

#[path = "reader_limits.rs"]
mod limits;
pub use limits::BundleReadLimits;
#[cfg(test)]
#[path = "reader_limits_tests.rs"]
mod limits_tests;

use super::format::{
    BundleEntry, BundleError, BUNDLE_FLAG_PAGE_ALIGN_4096, BUNDLE_HEADER_SIZE, BUNDLE_MAGIC,
    BUNDLE_PAGE_ALIGN_4096, BUNDLE_VERSION, OFF_CRC32C, OFF_FLAGS, OFF_INDEX_LENGTH,
    OFF_INDEX_OFFSET, OFF_MAGIC, OFF_TOTAL_LENGTH, OFF_VERSION,
};

#[inline]
fn read_u32(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

#[inline]
fn read_u64(b: &[u8], off: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[off..off + 8]);
    u64::from_le_bytes(a)
}

/// Whole-file CRC-32C computed with the 4 CRC-field bytes treated as zero — no
/// copy of the payload (streams the three ranges through the incremental CRC),
/// so it is cheap even over a memory-mapped multi-hundred-MB bundle.
fn whole_file_crc(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    crc = crc32c_update(crc, &bytes[..OFF_CRC32C]);
    crc = crc32c_update(crc, &[0u8; 4]);
    crc = crc32c_update(crc, &bytes[OFF_CRC32C + 4..]);
    !crc
}

/// A parsed, integrity-checked view over a `.hmc` bundle's bytes. Borrows the
/// input; `get`/`segment` return slices into it (zero copy).
pub struct BundleReader<'a> {
    bytes: &'a [u8],
    entries: Vec<BundleEntry>,
    flags: u16,
}

impl<'a> BundleReader<'a> {
    /// Bounded cold admission for the current definite-length index encoding.
    /// Preflight borrows the CBOR fields before the legacy serde decoder can
    /// allocate. Unknown fields, indefinite encodings and trailing index data
    /// require an explicit compatibility profile instead of implicit skipping.
    /// This verifies bytes and structure, not source rights or asset acceptance.
    pub fn parse_with_limits(bytes: &'a [u8], limits: BundleReadLimits) -> Result<Self, BundleError> {
        let count = self::limits::preflight(bytes, limits)?;
        let reader = Self::parse(bytes)?;
        if reader.entries.len() != count {
            return Err(BundleError::Cbor("Bundle entry count differs".into()));
        }
        for entry in &reader.entries {
            if entry.key.is_empty()
                || entry.offset % super::format::BUNDLE_ENTRY_ALIGN as u64 != 0
                || entry.sha256.len() != 32
            {
                return Err(BundleError::Cbor("Bundle entry integrity differs".into()));
            }
        }
        // One reference per entry fits inside the preflight's four-times-entry
        // reservation. Both unstable sorts are in-place; no tree/node allocation.
        let mut ordered = Vec::new();
        ordered
            .try_reserve_exact(count)
            .map_err(|_| BundleError::Cbor("Bundle validation scratch unavailable".into()))?;
        ordered.extend(reader.entries.iter());
        ordered.sort_unstable_by(|a, b| a.key.cmp(&b.key));
        for pair in ordered.windows(2) {
            if pair[0].key == pair[1].key {
                return Err(BundleError::DuplicateKey(pair[0].key.clone()));
            }
        }
        ordered.sort_unstable_by_key(|entry| (entry.offset, entry.length));
        for pair in ordered.windows(2) {
            // Legacy parse has already proved each sum and its payload bounds.
            if pair[0].offset + pair[0].length > pair[1].offset {
                return Err(BundleError::Cbor("Bundle payload ranges overlap".into()));
            }
        }
        // Prove disjointness BEFORE SHA work. Hash direct slices (no key lookup)
        // so the total payload hashed is at most the admitted file byte count.
        use sha2::{Digest, Sha256};
        for entry in ordered {
            let payload = &bytes[entry.offset as usize..(entry.offset + entry.length) as usize];
            if Sha256::digest(payload).as_slice() != entry.sha256.as_slice() {
                return Err(BundleError::Cbor("Bundle entry payload hash differs".into()));
            }
        }
        Ok(reader)
    }

    /// Parse and fully validate a bundle: magic, version, declared length,
    /// whole-file CRC, and every entry's bounds. On success the payloads are
    /// known-good and in-bounds, so `get`/`segment` are total on valid keys.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, BundleError> {
        if bytes.len() < BUNDLE_HEADER_SIZE {
            return Err(BundleError::TooShort);
        }
        if bytes[OFF_MAGIC..OFF_MAGIC + 4] != BUNDLE_MAGIC {
            return Err(BundleError::BadMagic);
        }
        let version = u16::from_le_bytes([bytes[OFF_VERSION], bytes[OFF_VERSION + 1]]);
        if version != BUNDLE_VERSION {
            return Err(BundleError::UnsupportedVersion(version));
        }
        let flags = u16::from_le_bytes([bytes[OFF_FLAGS], bytes[OFF_FLAGS + 1]]);

        let total_length = read_u64(bytes, OFF_TOTAL_LENGTH);
        if total_length as usize != bytes.len() {
            return Err(BundleError::LengthMismatch {
                header: total_length,
                actual: bytes.len(),
            });
        }

        // Integrity before trusting any offset.
        let stored_crc = read_u32(bytes, OFF_CRC32C);
        let got = whole_file_crc(bytes);
        if got != stored_crc {
            return Err(BundleError::CrcMismatch {
                expected: stored_crc,
                got,
            });
        }

        let index_offset = read_u64(bytes, OFF_INDEX_OFFSET);
        let index_length = read_u64(bytes, OFF_INDEX_LENGTH);
        let end = index_offset
            .checked_add(index_length)
            .ok_or(BundleError::BadIndexPointer {
                offset: index_offset,
                length: index_length,
                total: bytes.len(),
            })?;
        if index_offset < BUNDLE_HEADER_SIZE as u64 || end as usize > bytes.len() {
            return Err(BundleError::BadIndexPointer {
                offset: index_offset,
                length: index_length,
                total: bytes.len(),
            });
        }

        let index_slice = &bytes[index_offset as usize..end as usize];
        let entries: Vec<BundleEntry> =
            ciborium::from_reader(index_slice).map_err(|e| BundleError::Cbor(e.to_string()))?;

        // Every entry must lie within the payload region (before the index).
        let check_4096 = (flags & BUNDLE_FLAG_PAGE_ALIGN_4096) != 0;
        for e in &entries {
            let e_end =
                e.offset
                    .checked_add(e.length)
                    .ok_or_else(|| BundleError::EntryOutOfBounds {
                        key: e.key.clone(),
                        offset: e.offset,
                        length: e.length,
                    })?;
            if e.offset < BUNDLE_HEADER_SIZE as u64 || e_end > index_offset {
                return Err(BundleError::EntryOutOfBounds {
                    key: e.key.clone(),
                    offset: e.offset,
                    length: e.length,
                });
            }
            if check_4096 && (e.offset % BUNDLE_PAGE_ALIGN_4096 as u64 != 0) {
                return Err(BundleError::UnpageAligned {
                    key: e.key.clone(),
                    offset: e.offset,
                    alignment: BUNDLE_PAGE_ALIGN_4096,
                });
            }
        }

        Ok(Self {
            bytes,
            entries,
            flags,
        })
    }

    /// The bundle's entries (index order).
    pub fn entries(&self) -> &[BundleEntry] {
        &self.entries
    }

    /// Reserved format flags from the header (0 in v1; reserved for future use
    /// such as a signed/compressed-entry variant).
    pub fn flags(&self) -> u16 {
        self.flags
    }

    /// True when the bundle header declares 4096-byte OS page alignment.
    pub fn is_page_aligned_4096(&self) -> bool {
        (self.flags & BUNDLE_FLAG_PAGE_ALIGN_4096) != 0
    }

    /// The index record for `key`, if present.
    pub fn entry(&self, key: &str) -> Option<&BundleEntry> {
        self.entries.iter().find(|e| e.key == key)
    }

    /// A zero-copy slice of the intact embedded file for `key`. This slice **is**
    /// a byte-identical standalone `.q42` / `.10d` / `.p64` — hand it straight to
    /// that format's reader.
    pub fn get(&self, key: &str) -> Option<&'a [u8]> {
        let e = self.entry(key)?;
        Some(&self.bytes[e.offset as usize..(e.offset + e.length) as usize])
    }

    /// A zero-copy slice of an **interior segment** of an entry
    /// (`[seg_offset .. seg_offset+seg_len)` within the embedded file). Returns
    /// `None` if the key is unknown or the segment runs past the entry — proving
    /// the bundle does not interfere with segment-level access into an entry.
    pub fn segment(&self, key: &str, seg_offset: u64, seg_len: u64) -> Option<&'a [u8]> {
        let e = self.entry(key)?;
        let seg_end = seg_offset.checked_add(seg_len)?;
        if seg_end > e.length {
            return None;
        }
        let start = (e.offset + seg_offset) as usize;
        Some(&self.bytes[start..start + seg_len as usize])
    }

    /// Verify one entry's payload against its recorded SHA-256.
    pub fn verify_entry(&self, key: &str) -> bool {
        use sha2::{Digest, Sha256};
        let Some(e) = self.entry(key) else {
            return false;
        };
        let Some(payload) = self.get(key) else {
            return false;
        };
        let mut hasher = Sha256::new();
        hasher.update(payload);
        hasher.finalize().as_slice() == e.sha256.as_slice()
    }

    /// Retrieve and decode the embedded digital twin family manifest, if present.
    pub fn get_family_manifest(
        &self,
    ) -> Result<Option<super::variant_matrix::TwinFamilyManifest>, BundleError> {
        match self.get(super::variant_matrix::FAMILY_MANIFEST_ENTRY_KEY) {
            Some(bytes) => {
                let manifest = super::variant_matrix::TwinFamilyManifest::decode_from_cbor(bytes)?;
                Ok(Some(manifest))
            }
            None => Ok(None),
        }
    }

    /// Retrieve and decode the embedded holistic species metadata, if present.
    pub fn get_species_meta(
        &self,
    ) -> Result<Option<super::species_meta::HolisticSpeciesMeta>, BundleError> {
        match self.get(super::species_meta::SPECIES_META_ENTRY_KEY) {
            Some(bytes) => {
                let meta = super::species_meta::HolisticSpeciesMeta::decode_from_cbor(bytes)?;
                Ok(Some(meta))
            }
            None => Ok(None),
        }
    }
}

/// Open a `.hmc` bundle from disk via `mmap` for zero-copy access (native).
///
/// The mapping backs the slices returned by [`BundleReader`], so an embedded
/// file (or one of its interior segments) can be read without copying the
/// bundle into the heap — the intended path for shipping a large asset pack.
#[cfg(not(target_arch = "wasm32"))]
pub struct BundleMmap {
    map: memmap2::Mmap,
}

#[cfg(not(target_arch = "wasm32"))]
impl BundleMmap {
    /// Memory-map a bundle file.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, BundleError> {
        let file = std::fs::File::open(path).map_err(|e| BundleError::Io(e.to_string()))?;
        // SAFETY: the map is read-only and lives as long as `self`; callers get
        // slices bounded by `self`.
        let map =
            unsafe { memmap2::Mmap::map(&file) }.map_err(|e| BundleError::Io(e.to_string()))?;
        Ok(Self { map })
    }

    /// The raw mapped bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.map
    }

    /// A validated reader borrowing the mapping.
    pub fn reader(&self) -> Result<BundleReader<'_>, BundleError> {
        BundleReader::parse(&self.map)
    }
}

#[cfg(test)]
mod tests {
    use super::super::writer::BundleWriter;
    use super::*;

    fn sample_bundle() -> Vec<u8> {
        let mut w = BundleWriter::new();
        w.add_file(
            "liver.10d",
            "10d",
            (0..200u32).map(|i| i as u8).collect(),
            Some(vec![0xAA, 0xBB]),
        )
        .unwrap();
        w.add_file(
            "graph.q42",
            "q42",
            b"hello q42 segment world".to_vec(),
            None,
        )
        .unwrap();
        w.build().unwrap()
    }

    #[test]
    fn roundtrip_is_byte_identical() {
        let liver: Vec<u8> = (0..200u32).map(|i| i as u8).collect();
        let bytes = sample_bundle();
        let r = BundleReader::parse(&bytes).unwrap();
        assert_eq!(r.entries().len(), 2);
        // The embedded file comes back byte-for-byte — the transparency guarantee.
        assert_eq!(r.get("liver.10d").unwrap(), liver.as_slice());
        assert_eq!(r.get("graph.q42").unwrap(), b"hello q42 segment world");
        assert_eq!(r.entry("liver.10d").unwrap().kind, "10d");
        assert_eq!(
            r.entry("liver.10d").unwrap().meta.as_deref(),
            Some(&[0xAA, 0xBB][..])
        );
        assert!(r.get("missing").is_none());
    }

    #[test]
    fn interior_segment_access_is_not_interfered_with() {
        let bytes = sample_bundle();
        let r = BundleReader::parse(&bytes).unwrap();
        // Pull the interior segment "q42 segment" (offset 6, len 11) directly.
        assert_eq!(r.segment("graph.q42", 6, 11).unwrap(), b"q42 segment");
        // A segment past the end is refused, not read out of the entry.
        assert!(r.segment("graph.q42", 20, 100).is_none());
    }

    #[test]
    fn per_entry_sha256_verifies_and_detects_tamper() {
        let mut bytes = sample_bundle();
        let r = BundleReader::parse(&bytes).unwrap();
        assert!(r.verify_entry("liver.10d"));
        assert!(r.verify_entry("graph.q42"));
        drop(r);
        // Flip a payload byte inside an entry: whole-file CRC now fails on parse.
        let off = {
            let r = BundleReader::parse(&bytes).unwrap();
            r.entry("liver.10d").unwrap().offset as usize
        };
        bytes[off] ^= 0xFF;
        assert!(matches!(
            BundleReader::parse(&bytes),
            Err(BundleError::CrcMismatch { .. })
        ));
    }

    #[test]
    fn rejects_bad_magic_and_short_input() {
        assert!(matches!(
            BundleReader::parse(&[0u8; 10]),
            Err(BundleError::TooShort)
        ));
        let mut bytes = sample_bundle();
        bytes[0] = b'X';
        assert!(matches!(
            BundleReader::parse(&bytes),
            Err(BundleError::BadMagic)
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn mmap_roundtrip() {
        let bytes = sample_bundle();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pack.hmc");
        std::fs::write(&path, &bytes).unwrap();
        let m = BundleMmap::open(&path).unwrap();
        let r = m.reader().unwrap();
        assert_eq!(r.get("graph.q42").unwrap(), b"hello q42 segment world");
        assert!(r.verify_entry("liver.10d"));
    }
}
