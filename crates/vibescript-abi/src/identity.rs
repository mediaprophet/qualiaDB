//! Canonical IRI hashing (`q_hash` FNV-1a 64-bit), provider-scoped stable function IDs,
//! and generational prepared runtime dispatch handles (`VibeOpCode`, `PreparedCallHandle`).

/// 64-bit FNV-1a offset basis used across QualiaDB (`qualia_core_db::q_hash`) and `vibe`.
pub const FNV1A_64_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// 64-bit FNV-1a prime used across QualiaDB (`qualia_core_db::q_hash`) and `vibe`.
pub const FNV1A_64_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Compile-time and runtime 64-bit FNV-1a hash over raw bytes (`q_hash`).
#[inline]
pub const fn q_hash(bytes: &[u8]) -> u64 {
    let mut hash = FNV1A_64_OFFSET_BASIS;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(FNV1A_64_PRIME);
        i += 1;
    }
    hash
}

/// Compile-time and runtime 64-bit FNV-1a hash over a UTF-8 string slice.
#[inline]
pub const fn q_hash_str(s: &str) -> u64 {
    q_hash(s.as_bytes())
}

/// Globally stable semantic identifier pairing an immutable IRI with its `q_hash`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalIri {
    /// Canonical UTF-8 IRI (e.g. `"https://qualiadb.org/ext/etalon/sim/xpbd_step"`).
    pub iri: &'static str,
    /// Precomputed 64-bit FNV-1a hash (`q_hash_str(iri)`).
    pub fnv1a_hash: u64,
}

impl CanonicalIri {
    /// Constructs a `CanonicalIri` and computes its `q_hash` at compile time.
    #[inline]
    pub const fn new(iri: &'static str) -> Self {
        Self {
            iri,
            fnv1a_hash: q_hash_str(iri),
        }
    }

    /// Constructs a `CanonicalIri` with an explicit hash (verified during registration).
    #[inline]
    pub const fn with_explicit_hash(iri: &'static str, fnv1a_hash: u64) -> Self {
        Self { iri, fnv1a_hash }
    }

    /// Verifies that the stored `fnv1a_hash` matches `q_hash_str(self.iri)` and the IRI is non-empty.
    #[inline]
    pub const fn is_valid(&self) -> bool {
        !self.iri.is_empty() && self.fnv1a_hash == q_hash_str(self.iri)
    }
}

/// Provider-scoped stable function identifier (`sub_namespace_id`, `func_id`).
///
/// Unlike process-local registry slots, `StableFuncId` is immutable within a
/// provider major/minor release and independent of provider registration order.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StableFuncId {
    /// Sub-namespace index within the provider (e.g. `0 = kinematics`, `1 = dynamics`).
    pub sub_namespace_id: u16,
    /// Function index within the sub-namespace.
    pub func_id: u16,
}

impl StableFuncId {
    /// Creates a new `StableFuncId`.
    #[inline]
    pub const fn new(sub_namespace_id: u16, func_id: u16) -> Self {
        Self {
            sub_namespace_id,
            func_id,
        }
    }

    /// Packs `(sub_namespace_id, func_id)` into a `u32`.
    #[inline]
    pub const fn to_u32(self) -> u32 {
        ((self.sub_namespace_id as u32) << 16) | (self.func_id as u32)
    }

    /// Unpacks a `u32` into `StableFuncId`.
    #[inline]
    pub const fn from_u32(packed: u32) -> Self {
        Self {
            sub_namespace_id: (packed >> 16) as u16,
            func_id: (packed & 0xFFFF) as u16,
        }
    }
}

/// Process-local 64-bit prepared dispatch handle: `[ns_slot: u32][sub_id: u16][func_id: u16]`.
///
/// **Important**: `VibeOpCode` is a process-local runtime handle bound to a
/// `RegistryGeneration`. It is never persisted as a canonical semantic ID and
/// never written into `NQuin.predicate[0..7]` (`SlgOpcode`).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VibeOpCode(pub u64);

impl VibeOpCode {
    /// Packs a registry slot and `StableFuncId` into a `VibeOpCode`.
    #[inline]
    pub const fn pack(ns_slot: u32, stable_id: StableFuncId) -> Self {
        let upper = (ns_slot as u64) << 32;
        let lower = stable_id.to_u32() as u64;
        Self(upper | lower)
    }

    /// Extracts the registry provider slot (`bits 32..63`).
    #[inline]
    pub const fn ns_slot(self) -> u32 {
        (self.0 >> 32) as u32
    }

    /// Extracts the sub-namespace ID (`bits 16..31`).
    #[inline]
    pub const fn sub_namespace_id(self) -> u16 {
        ((self.0 >> 16) & 0xFFFF) as u16
    }

    /// Extracts the function ID (`bits 0..15`).
    #[inline]
    pub const fn func_id(self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    /// Extracts the `StableFuncId`.
    #[inline]
    pub const fn stable_id(self) -> StableFuncId {
        StableFuncId {
            sub_namespace_id: self.sub_namespace_id(),
            func_id: self.func_id(),
        }
    }
}

/// Monotonic generation counter of the extension registry.
///
/// Incremented on every provider registration or unload so stale `PreparedCallHandle`s
/// fail closed in $O(1)$ before dispatch.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegistryGeneration(pub u32);

impl RegistryGeneration {
    /// Initial generation before any registration.
    pub const INITIAL: Self = Self(1);

    /// Returns the next generation, saturating at `u32::MAX`.
    #[inline]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Prepared call token produced during the cold `Prepare` stage and consumed in $O(1)$
/// during the hot `Admit -> Execute` stages.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedCallHandle {
    /// Packed `O(1)` dispatch slot (`[ns_slot: u32][sub_id: u16][func_id: u16]`).
    pub opcode: VibeOpCode,
    /// Registry generation at preparation time; must equal current registry generation on dispatch.
    pub registry_generation: RegistryGeneration,
    /// Reserved alignment padding.
    pub reserved: u32,
    /// Canonical `q_hash` of the provider namespace IRI.
    pub provider_iri_hash: u64,
    /// Canonical `q_hash` of the function IRI.
    pub func_iri_hash: u64,
    /// Full 32-byte SHA-256 ontological contract digest bound at preparation time.
    pub contract_digest: [u8; 32],
}
