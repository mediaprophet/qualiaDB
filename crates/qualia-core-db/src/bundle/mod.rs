//! `.qualia` — a **transparent container-of-files** bundle for shipping a set of
//! sealed Qualia assets (`.10d` meshes, `.q42` graph volumes, `.p64` weights, …)
//! as one attestable unit (a release artefact, a downloadable pack, a signed
//! asset set).
//!
//! # Why a bundle, and why *transparent*
//! The Qualia container family (`.10d` / `.q42` / `.p64`) are all internally
//! navigable — you can pull a *segment* (a mesh section, a subgraph, a weight
//! shard) without reading the whole file, ideally `mmap` zero-copy. A bundle
//! must not destroy that. So a `.qualia` bundle only **concatenates intact files,
//! page-aligned, with an index of absolute offsets** — it never compresses,
//! re-chunks, or reframes an entry. Consequences:
//!
//! - `reader.get(key)` returns a zero-copy slice that **is** a byte-identical
//!   standalone file — feed it straight to the existing `.q42`/`.10d`/`.p64`
//!   reader; its interior offsets resolve unchanged.
//! - `reader.segment(key, off, len)` reaches an interior segment of an entry
//!   directly — the bundle does not interfere with segment-level access.
//! - HTTP range-fetching one interior segment works: `entry.offset + seg.off`.
//!
//! It lives in `qualia-core-db` so it is **one reader for both channels** —
//! native (`BundleMmap`, zero-copy) and WASM (parse fetched/ranged bytes).
//!
//! See [`format`] for the on-disk layout.

mod format;
mod reader;
mod writer;

pub use format::{
    BundleEntry, BundleError, BUNDLE_ENTRY_ALIGN, BUNDLE_HEADER_SIZE, BUNDLE_MAGIC, BUNDLE_VERSION,
    SHA256_LEN,
};
pub use reader::BundleReader;
pub use writer::BundleWriter;

#[cfg(not(target_arch = "wasm32"))]
pub use reader::BundleMmap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heterogeneous_pack_of_three_kinds_roundtrips() {
        // A bundle can carry different sealed formats side by side.
        let ten_d: Vec<u8> = (0..300u32).map(|i| (i * 7) as u8).collect();
        let q42 = b"q42 graph volume bytes".to_vec();
        let p64: Vec<u8> = vec![0x40; 129];

        let mut w = BundleWriter::new();
        w.add_file("body.10d", "10d", ten_d.clone(), None).unwrap();
        w.add_file("wordnet.q42", "q42", q42.clone(), None).unwrap();
        w.add_file("model.p64", "p64", p64.clone(), None).unwrap();
        let bytes = w.build().unwrap();

        let r = BundleReader::parse(&bytes).unwrap();
        assert_eq!(r.entries().len(), 3);
        assert_eq!(r.get("body.10d").unwrap(), ten_d.as_slice());
        assert_eq!(r.get("wordnet.q42").unwrap(), q42.as_slice());
        assert_eq!(r.get("model.p64").unwrap(), p64.as_slice());
        assert_eq!(r.entry("model.p64").unwrap().kind, "p64");
        // Every entry is page-aligned so its interior alignment is preserved.
        for e in r.entries() {
            assert_eq!(e.offset % BUNDLE_ENTRY_ALIGN as u64, 0);
        }
        // Every entry verifies against its recorded hash.
        assert!(r.entries().iter().all(|e| r.verify_entry(&e.key)));
    }

    #[test]
    fn empty_bundle_is_valid() {
        let bytes = BundleWriter::new().build().unwrap();
        let r = BundleReader::parse(&bytes).unwrap();
        assert!(r.entries().is_empty());
    }
}
