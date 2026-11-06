//! Semantic versioning (`SemVer64`), Cargo-compatible caret/range matching (`SemVerReq`),
//! and the five independent contract version axes (`ContractVersionAxes`).

/// Packed 64-bit semantic version: `[major: u16][minor: u16][patch: u16][abi_flags: u16]`.
///
/// `abi_flags == 0` denotes a stable release. Non-zero `abi_flags` denote pre-release
/// or target-specific ABI qualifiers and are never matched by stable ranges unless
/// explicitly requested.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SemVer64(pub u64);

impl SemVer64 {
    /// Packs `(major, minor, patch, abi_flags)` into a `SemVer64`.
    #[inline]
    pub const fn new(major: u16, minor: u16, patch: u16, abi_flags: u16) -> Self {
        let m = (major as u64) << 48;
        let n = (minor as u64) << 32;
        let p = (patch as u64) << 16;
        let f = abi_flags as u64;
        Self(m | n | p | f)
    }

    /// Packs a stable release `(major, minor, patch)` with `abi_flags = 0`.
    #[inline]
    pub const fn release(major: u16, minor: u16, patch: u16) -> Self {
        Self::new(major, minor, patch, 0)
    }

    /// Major version component (`bits 48..63`).
    #[inline]
    pub const fn major(self) -> u16 {
        (self.0 >> 48) as u16
    }

    /// Minor version component (`bits 32..47`).
    #[inline]
    pub const fn minor(self) -> u16 {
        ((self.0 >> 32) & 0xFFFF) as u16
    }

    /// Patch version component (`bits 16..31`).
    #[inline]
    pub const fn patch(self) -> u16 {
        ((self.0 >> 16) & 0xFFFF) as u16
    }

    /// ABI / pre-release flags (`bits 0..15`).
    #[inline]
    pub const fn abi_flags(self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    /// Returns `true` when `abi_flags == 0` (stable release).
    #[inline]
    pub const fn is_stable(self) -> bool {
        self.abi_flags() == 0
    }

    /// Compares `(major, minor, patch)` release tuple, ignoring `abi_flags`.
    #[inline]
    pub const fn release_key(self) -> u64 {
        self.0 & 0xFFFF_FFFF_FFFF_0000
    }
}

/// Deterministic version requirement evaluated without heap allocation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemVerReq {
    /// Matches any stable version (`abi_flags == 0`).
    AnyStable,
    /// Matches exact `(major, minor, patch, abi_flags)`.
    Exact(SemVer64),
    /// Cargo caret requirement (`^major.minor.patch`):
    /// - `^1.2.3` -> `>=1.2.3, <2.0.0`
    /// - `^0.4.0` -> `>=0.4.0, <0.5.0`
    /// - `^0.0.3` -> `>=0.0.3, <0.0.4`
    Caret(SemVer64),
    /// Cargo tilde requirement (`~major.minor.patch`):
    /// `>=major.minor.patch, <major.(minor+1).0`.
    Tilde(SemVer64),
    /// Explicit half-open release range `[min_inclusive, max_exclusive)`.
    Range {
        min_inclusive: SemVer64,
        max_exclusive: SemVer64,
    },
}

impl SemVerReq {
    /// Evaluates whether `candidate` satisfies this version requirement.
    #[inline]
    pub const fn matches(&self, candidate: SemVer64) -> bool {
        match *self {
            Self::AnyStable => candidate.is_stable(),
            Self::Exact(req) => candidate.0 == req.0,
            Self::Caret(req) => {
                if candidate.abi_flags() != req.abi_flags() {
                    return false;
                }
                let c_key = candidate.release_key();
                let r_key = req.release_key();
                if c_key < r_key {
                    return false;
                }
                if req.major() > 0 {
                    candidate.major() == req.major()
                } else if req.minor() > 0 {
                    candidate.major() == 0 && candidate.minor() == req.minor()
                } else {
                    candidate.major() == 0
                        && candidate.minor() == 0
                        && candidate.patch() == req.patch()
                }
            }
            Self::Tilde(req) => {
                if candidate.abi_flags() != req.abi_flags() {
                    return false;
                }
                candidate.major() == req.major()
                    && candidate.minor() == req.minor()
                    && candidate.patch() >= req.patch()
            }
            Self::Range {
                min_inclusive,
                max_exclusive,
            } => {
                if !candidate.is_stable() {
                    return false;
                }
                let c_key = candidate.release_key();
                c_key >= min_inclusive.release_key() && c_key < max_exclusive.release_key()
            }
        }
    }
}

/// The five independent version axes governing a VibeScript extension provider.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContractVersionAxes {
    /// VibeScript language grammar & evaluation semantics version (e.g. `0.1.0`).
    pub language_version: SemVer64,
    /// `vibescript-abi` layout & trait surface version (e.g. `0.3.0`).
    pub abi_version: SemVer64,
    /// Host capability seam version (e.g. `0.1.0` for `vibe-host-0.1`).
    pub host_version: SemVer64,
    /// Domain provider implementation version (matched by `SemVerReq`).
    pub provider_version: SemVer64,
    /// Self-describing ontology schema version for this provider.
    pub ontology_version: SemVer64,
}

impl ContractVersionAxes {
    /// Current canonical VibeScript 0.1 + ABI 0.3 baseline for a provider at `provider_version`.
    #[inline]
    pub const fn v0_3_default(provider_version: SemVer64, ontology_version: SemVer64) -> Self {
        Self {
            language_version: SemVer64::release(0, 1, 0),
            abi_version: SemVer64::release(0, 3, 0),
            host_version: SemVer64::release(0, 1, 0),
            provider_version,
            ontology_version,
        }
    }

    /// Verifies that the provider's ABI and language versions are compatible with the host engine.
    #[inline]
    pub const fn is_engine_compatible(&self) -> bool {
        SemVerReq::Caret(SemVer64::release(0, 1, 0)).matches(self.language_version)
            && SemVerReq::Caret(SemVer64::release(0, 3, 0)).matches(self.abi_version)
            && SemVerReq::Caret(SemVer64::release(0, 1, 0)).matches(self.host_version)
    }
}
