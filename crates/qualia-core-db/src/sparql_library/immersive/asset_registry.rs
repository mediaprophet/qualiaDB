//! QISP dense-asset registry — generation-safe, fail-closed handles to
//! content-addressed dense assets (plan §3.4, §3.6, §10.1 QISP-R03/R04).
//!
//! # Security contract (read before touching this file)
//!
//! A [`DenseAssetRef`] is a **validated, process-local handle**. It carries only
//! numeric fields — a content-derived 60-bit token, a generation number, a section
//! kind, a byte offset, a byte length, and a digest prefix. **No Rust address, GPU
//! buffer pointer, or unchecked file offset is ever stored in it or derivable from
//! it** (plan §2.2 item 5, §14 non-goals, QISP-R04). The public RDF term is an
//! absolute IRI; it is resolved *to* one of these handles internally — never the
//! other way around.
//!
//! [`DenseAssetRegistry::resolve`] **fails closed**: a forged token, a tampered
//! offset/length, or a stale generation returns a named [`QispError`] and never
//! fabricates a record. It never panics on a bad handle. This is a §15
//! security-critical requirement.

use super::value::QispError;

/// The dense-asset section kinds (plan §3.4 / §3.6 representation table).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SectionKind {
    /// A render/exchange mesh (glTF/GLB or `.10d` mesh section).
    Mesh = 0,
    /// A Tensor10D buffer/section.
    Tensor10D = 1,
    /// A GeoSPARQL WKT/GML geometry literal payload.
    Wkt = 2,
    /// A trajectory (time-parameterised path).
    Trajectory = 3,
    /// A bounding-volume hierarchy / spatial index section.
    Bvh = 4,
    /// An opaque raw byte section (media type carried in the RDF descriptor).
    Raw = 5,
}

/// Hard capacity ceiling for a single registry. The registry is a *cold* structure
/// (query/endpoint scoped), so a bounded `Vec` is used, but it never grows past
/// this — over-capacity insertion fails with [`QispError::BudgetExceeded`] rather
/// than allocating without bound (plan §6.2 "fixed-capacity result-handle table").
pub const MAX_ASSETS: usize = 4096;

/// A validated, generation-safe, **process-local** handle to a dense asset.
///
/// All fields are numeric and private; only the accessors below are public, so no
/// caller can inject a raw address. This is the record resolved from a public
/// absolute-IRI RDF term (plan §3.4).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DenseAssetRef {
    /// 60-bit content-derived token (top 4 bits reserved for tag bits). NOT a pointer.
    token: u64,
    /// Generation of the owning slot at mint time; used to reject stale reuse.
    generation: u32,
    /// Which kind of section this asset is.
    section: SectionKind,
    /// Byte offset of the section within its container. Validated on resolve.
    offset: u64,
    /// Byte length of the section. Validated on resolve.
    length: u64,
    /// Truncated content digest prefix (integrity cross-check; not the full digest).
    digest_prefix: u64,
}

impl DenseAssetRef {
    /// The 60-bit content token (never a memory address).
    pub const fn token(&self) -> u64 {
        self.token
    }
    /// The generation this handle was minted against.
    pub const fn generation(&self) -> u32 {
        self.generation
    }
    /// The section kind.
    pub const fn section(&self) -> SectionKind {
        self.section
    }
    /// The byte offset (validated, container-relative — not a raw pointer).
    pub const fn offset(&self) -> u64 {
        self.offset
    }
    /// The byte length.
    pub const fn length(&self) -> u64 {
        self.length
    }
    /// The truncated content digest prefix.
    pub const fn digest_prefix(&self) -> u64 {
        self.digest_prefix
    }
}

/// The immutable record a registry stores for a live asset. Structurally identical
/// to [`DenseAssetRef`]; returned by `resolve` so callers can read the *validated*
/// fields (never a pointer).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetRecord {
    token: u64,
    generation: u32,
    section: SectionKind,
    offset: u64,
    length: u64,
    digest_prefix: u64,
}

impl AssetRecord {
    /// The 60-bit content token.
    pub const fn token(&self) -> u64 {
        self.token
    }
    /// The current generation of this record's slot.
    pub const fn generation(&self) -> u32 {
        self.generation
    }
    /// The section kind.
    pub const fn section(&self) -> SectionKind {
        self.section
    }
    /// The byte offset.
    pub const fn offset(&self) -> u64 {
        self.offset
    }
    /// The byte length.
    pub const fn length(&self) -> u64 {
        self.length
    }
    /// The truncated content digest prefix.
    pub const fn digest_prefix(&self) -> u64 {
        self.digest_prefix
    }
    /// Mint a handle that refers to this record at its current generation.
    pub const fn to_ref(&self) -> DenseAssetRef {
        DenseAssetRef {
            token: self.token,
            generation: self.generation,
            section: self.section,
            offset: self.offset,
            length: self.length,
            digest_prefix: self.digest_prefix,
        }
    }
}

/// One physical slot in the registry. A slot's `generation` is monotonic across
/// eviction + reuse, so a handle minted before a reuse is rejected as stale.
#[derive(Debug, Clone, Copy)]
struct Slot {
    occupied: bool,
    record: AssetRecord,
}

/// A bounded, generation-safe registry mapping content tokens to validated dense
/// asset records.
///
/// Insertion is content-addressed and idempotent; resolution fails closed on
/// unknown, tampered, or stale handles.
#[derive(Debug, Default)]
pub struct DenseAssetRegistry {
    slots: Vec<Slot>,
}

impl DenseAssetRegistry {
    /// A new, empty registry.
    pub fn new() -> Self {
        Self { slots: Vec::new() }
    }

    /// Number of currently-live (occupied) assets.
    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.occupied).count()
    }

    /// Whether the registry has no live assets.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Deterministically derive the 60-bit content token for an asset from its
    /// section kind, offset, length, and digest prefix. Same content → same token.
    fn derive_token(section: SectionKind, offset: u64, length: u64, digest_prefix: u64) -> u64 {
        let mut bytes = [0u8; 25];
        bytes[0] = section as u8;
        bytes[1..9].copy_from_slice(&offset.to_le_bytes());
        bytes[9..17].copy_from_slice(&length.to_le_bytes());
        bytes[17..25].copy_from_slice(&digest_prefix.to_le_bytes());
        // Runtime 60-bit token (top 4 bits reserved for tag bits), reusing the
        // crate's canonical FNV-1a token generator.
        crate::lexicon::generate_60bit_token(&bytes)
    }

    /// Insert (or return the existing handle for) a dense asset, minting a
    /// content-derived token and a fresh generation.
    ///
    /// Returns [`QispError::BudgetExceeded`] if the registry is at `MAX_ASSETS`
    /// capacity and no slot is free. Inserting content that already has a live slot
    /// is idempotent and returns the existing handle.
    ///
    /// > Note: the plan sketches this as `-> DenseAssetRef`; it is returned as a
    /// > `Result` here so the hard capacity bound can fail closed instead of
    /// > panicking or allocating without bound (plan §6.2, §12 completeness bar).
    pub fn insert(
        &mut self,
        section: SectionKind,
        offset: u64,
        length: u64,
        digest_prefix: u64,
    ) -> Result<DenseAssetRef, QispError> {
        let token = Self::derive_token(section, offset, length, digest_prefix);

        // Idempotent: identical live content returns its existing handle.
        for slot in self.slots.iter() {
            if slot.occupied && slot.record.token == token {
                return Ok(slot.record.to_ref());
            }
        }

        // Reuse a freed slot, bumping its generation so old handles go stale.
        for slot in self.slots.iter_mut() {
            if !slot.occupied {
                let generation = slot.record.generation.wrapping_add(1);
                slot.record = AssetRecord {
                    token,
                    generation,
                    section,
                    offset,
                    length,
                    digest_prefix,
                };
                slot.occupied = true;
                return Ok(slot.record.to_ref());
            }
        }

        // Otherwise append a brand-new slot at generation 1, respecting the cap.
        if self.slots.len() >= MAX_ASSETS {
            return Err(QispError::BudgetExceeded);
        }
        let record = AssetRecord {
            token,
            generation: 1,
            section,
            offset,
            length,
            digest_prefix,
        };
        self.slots.push(Slot {
            occupied: true,
            record,
        });
        Ok(record.to_ref())
    }

    /// Resolve a handle to its validated record. **Fails closed:**
    ///
    /// - [`QispError::UnknownAsset`] — the token is not present, or any of the
    ///   handle's section/offset/length/digest fields were tampered so they no
    ///   longer match the stored record.
    /// - [`QispError::StaleAsset`] — the token is present but the handle's
    ///   generation is older than the slot's current generation (the slot was
    ///   evicted and reused).
    ///
    /// Never panics on a bad handle.
    pub fn resolve(&self, r: &DenseAssetRef) -> Result<&AssetRecord, QispError> {
        for slot in self.slots.iter() {
            if slot.occupied && slot.record.token == r.token {
                // Generation check first: a matching token with an old generation
                // is a stale-handle reuse, reported distinctly.
                if slot.record.generation != r.generation {
                    return Err(QispError::StaleAsset);
                }
                // Integrity cross-check: every carried field must match the stored
                // record. A mutated offset/length/section/digest (token untouched)
                // fails as unknown rather than resolving to something else.
                if slot.record.section != r.section
                    || slot.record.offset != r.offset
                    || slot.record.length != r.length
                    || slot.record.digest_prefix != r.digest_prefix
                {
                    return Err(QispError::UnknownAsset);
                }
                return Ok(&slot.record);
            }
        }
        Err(QispError::UnknownAsset)
    }

    /// Evict a live asset, freeing its slot. The slot's generation is retained so a
    /// later re-insert bumps past it, invalidating any outstanding handle.
    ///
    /// Fails closed the same way as [`resolve`](Self::resolve): a forged or stale
    /// handle cannot evict a live asset.
    pub fn evict(&mut self, r: &DenseAssetRef) -> Result<(), QispError> {
        // Validate against the same rules as resolve before mutating.
        let mut target: Option<usize> = None;
        for (i, slot) in self.slots.iter().enumerate() {
            if slot.occupied && slot.record.token == r.token {
                if slot.record.generation != r.generation {
                    return Err(QispError::StaleAsset);
                }
                if slot.record.section != r.section
                    || slot.record.offset != r.offset
                    || slot.record.length != r.length
                    || slot.record.digest_prefix != r.digest_prefix
                {
                    return Err(QispError::UnknownAsset);
                }
                target = Some(i);
                break;
            }
        }
        match target {
            Some(i) => {
                self.slots[i].occupied = false;
                Ok(())
            }
            None => Err(QispError::UnknownAsset),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn ref_carries_no_address_only_numeric_fields() {
        // Structural proof there is no pointer field: the ref is exactly the sum of
        // its numeric fields under repr(C) (u64 + u32 + u8 + pad + u64*3 = 40).
        assert_eq!(size_of::<DenseAssetRef>(), 40);
        assert_eq!(size_of::<AssetRecord>(), 40);
        // And it is Copy (no owned heap buffer behind a pointer).
        fn assert_copy<T: Copy>() {}
        assert_copy::<DenseAssetRef>();
    }

    #[test]
    fn insert_then_resolve_round_trip() {
        let mut reg = DenseAssetRegistry::new();
        let r = reg
            .insert(SectionKind::Mesh, 128, 4096, 0xDEAD_BEEF)
            .unwrap();
        assert_eq!(r.section(), SectionKind::Mesh);
        assert_eq!(r.offset(), 128);
        assert_eq!(r.length(), 4096);
        assert_eq!(r.digest_prefix(), 0xDEAD_BEEF);

        let rec = reg.resolve(&r).unwrap();
        assert_eq!(rec.token(), r.token());
        assert_eq!(rec.offset(), 128);
        assert_eq!(rec.length(), 4096);
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn insert_is_idempotent_for_identical_content() {
        let mut reg = DenseAssetRegistry::new();
        let a = reg.insert(SectionKind::Tensor10D, 0, 400, 7).unwrap();
        let b = reg.insert(SectionKind::Tensor10D, 0, 400, 7).unwrap();
        assert_eq!(a, b);
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn mutated_token_fails_unknown() {
        let mut reg = DenseAssetRegistry::new();
        let r = reg.insert(SectionKind::Wkt, 16, 64, 1).unwrap();
        // Flip the token without a matching slot -> not found.
        let forged = DenseAssetRef {
            token: r.token() ^ 0x1,
            ..r
        };
        assert_eq!(reg.resolve(&forged), Err(QispError::UnknownAsset));
    }

    #[test]
    fn mutated_offset_or_length_fails_unknown() {
        let mut reg = DenseAssetRegistry::new();
        let r = reg.insert(SectionKind::Raw, 100, 200, 0xABCD).unwrap();

        // Same token, tampered offset.
        let bad_offset = DenseAssetRef {
            offset: 101,
            ..r
        };
        assert_eq!(reg.resolve(&bad_offset), Err(QispError::UnknownAsset));

        // Same token, tampered length.
        let bad_length = DenseAssetRef {
            length: 999,
            ..r
        };
        assert_eq!(reg.resolve(&bad_length), Err(QispError::UnknownAsset));

        // Same token, tampered section.
        let bad_section = DenseAssetRef {
            section: SectionKind::Bvh,
            ..r
        };
        assert_eq!(reg.resolve(&bad_section), Err(QispError::UnknownAsset));

        // The untampered handle still resolves.
        assert!(reg.resolve(&r).is_ok());
    }

    #[test]
    fn stale_handle_after_slot_reuse_fails_stale() {
        let mut reg = DenseAssetRegistry::new();
        let r1 = reg.insert(SectionKind::Mesh, 32, 512, 0xF00D).unwrap();
        assert_eq!(r1.generation(), 1);

        // Evict, then re-insert identical content -> same slot, bumped generation.
        reg.evict(&r1).unwrap();
        let r2 = reg.insert(SectionKind::Mesh, 32, 512, 0xF00D).unwrap();
        assert_eq!(r2.token(), r1.token());
        assert_eq!(r2.generation(), 2);

        // The old handle is now stale, not fabricated.
        assert_eq!(reg.resolve(&r1), Err(QispError::StaleAsset));
        // The new handle resolves.
        assert!(reg.resolve(&r2).is_ok());
    }

    #[test]
    fn evict_rejects_forged_and_stale_handles() {
        let mut reg = DenseAssetRegistry::new();
        let r = reg.insert(SectionKind::Trajectory, 8, 128, 5).unwrap();

        let forged = DenseAssetRef {
            token: r.token() ^ 0xFF,
            ..r
        };
        assert_eq!(reg.evict(&forged), Err(QispError::UnknownAsset));

        let bad_gen = DenseAssetRef {
            generation: r.generation() + 5,
            ..r
        };
        assert_eq!(reg.evict(&bad_gen), Err(QispError::StaleAsset));

        // The genuine handle still evicts.
        assert!(reg.evict(&r).is_ok());
        // And is unknown afterwards.
        assert_eq!(reg.resolve(&r), Err(QispError::UnknownAsset));
    }

    #[test]
    fn capacity_bound_is_respected() {
        let mut reg = DenseAssetRegistry::new();
        // Fill to capacity with distinct content (distinct digest prefixes).
        for i in 0..MAX_ASSETS as u64 {
            reg.insert(SectionKind::Raw, 0, 1, i).unwrap();
        }
        assert_eq!(reg.len(), MAX_ASSETS);
        // One past capacity fails closed.
        let over = reg.insert(SectionKind::Raw, 0, 1, MAX_ASSETS as u64);
        assert_eq!(over, Err(QispError::BudgetExceeded));
    }

    #[test]
    fn resolve_never_panics_on_empty_registry() {
        let reg = DenseAssetRegistry::new();
        let bogus = DenseAssetRef {
            token: 1234,
            generation: 1,
            section: SectionKind::Mesh,
            offset: 0,
            length: 0,
            digest_prefix: 0,
        };
        assert_eq!(reg.resolve(&bogus), Err(QispError::UnknownAsset));
    }
}
