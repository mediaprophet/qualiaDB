//! On-disk layout, index record, constants, and errors for the `.hmc` bundle.
//!
//! A `.hmc` bundle is a **transparent container of files**. It concatenates
//! each embedded file *byte-for-byte*, page-aligned, at an absolute offset
//! recorded in a CBOR index, and **never touches the interior of an entry**.
//! That transparency is the whole point: a consumer can `mmap` the bundle and
//! hand an entry's `[offset .. offset+length]` slice straight to the existing
//! `.q42` / `.10d` / `.p64` reader, and that reader's *interior* segment offsets
//! resolve unchanged — because the slice is a bit-identical standalone file.
//! Nothing is compressed, re-chunked, or reframed. HTTP range-fetching one
//! interior segment of one embedded file therefore works directly, too
//! (`entry.offset + seg.offset`, `seg.len`).
//!
//! ```text
//! offset  size  field
//! 0       4     magic = "QBDL"
//! 4       2     version (u16 LE)
//! 6       2     flags   (u16 LE, reserved = 0)
//! 8       4     entry_count (u32 LE)
//! 12      8     index_offset (u64 LE)   — absolute offset of the CBOR index
//! 20      8     index_length (u64 LE)
//! 28      8     total_length (u64 LE)   — == file length
//! 36      4     crc32c (u32 LE)         — whole-file CRC-32C, computed with
//!                                          these 4 bytes treated as zero
//! 40      24    reserved (zero)
//! 64      …     page-aligned intact file payloads …
//! …       …     CBOR index footer: Vec<BundleEntry>
//! ```
//!
//! The index lives at the end (referenced from the fixed front header) so entry
//! offsets are known before the index is written, and so a consumer can fetch
//! just the header + index to enumerate the bundle without downloading payloads.

use serde::{Deserialize, Serialize};

/// Magic bytes at the start of every `.hmc` (hyper-media-container) bundle:
/// `QBDL`. The 4-byte on-disk format tag is retained across the `.qualia`→`.hmc`
/// rename (extension ≠ magic, as with most formats), so bundles already built
/// keep parsing; only the file extension and human-facing name changed.
pub const BUNDLE_MAGIC: [u8; 4] = *b"QBDL";

/// Current bundle format version.
pub const BUNDLE_VERSION: u16 = 1;

/// Fixed header size in bytes. Payloads begin at (or after) this offset.
pub const BUNDLE_HEADER_SIZE: usize = 64;

/// Alignment (bytes) applied to every entry payload **and** the index footer.
/// Page alignment (64) preserves the interior page-alignment of embedded
/// `.q42` / `.p64` / `.10d` files, so their own segment offsets stay aligned and
/// `mmap` / zero-copy segment access into an entry is unaffected. This matches
/// the `Page` alignment tier of the `container_10d` writer and `p64_weight`.
pub const BUNDLE_ENTRY_ALIGN: usize = 64;

// --- header field byte offsets ---
pub(crate) const OFF_MAGIC: usize = 0;
pub(crate) const OFF_VERSION: usize = 4;
pub(crate) const OFF_FLAGS: usize = 6;
pub(crate) const OFF_ENTRY_COUNT: usize = 8;
pub(crate) const OFF_INDEX_OFFSET: usize = 12;
pub(crate) const OFF_INDEX_LENGTH: usize = 20;
pub(crate) const OFF_TOTAL_LENGTH: usize = 28;
pub(crate) const OFF_CRC32C: usize = 36;
// [40 .. 64) reserved, must be zero.

/// Length of a SHA-256 digest in bytes.
pub const SHA256_LEN: usize = 32;

/// One row of the bundle index (the CBOR footer). Describes one intact embedded
/// file: its logical key, its format `kind`, where it lives (absolute, aligned),
/// its length, a SHA-256 of the exact bytes, and opaque per-entry `meta` (a
/// nested CBOR blob whose schema is agreed by the producer/consumer of a given
/// bundle `kind` — e.g. an anatomy pack stores each organ's body-system,
/// position and default colour there). The bundle format itself stays domain-
/// agnostic: it moves intact files; `meta` carries any domain semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleEntry {
    /// Logical key, unique within the bundle (e.g. `"3d-vh-f-kidney-l.glb.10d"`).
    pub key: String,
    /// The embedded file's format, e.g. `"10d"`, `"q42"`, `"p64"`, `"manifest"`.
    pub kind: String,
    /// Absolute byte offset from the start of the bundle. `BUNDLE_ENTRY_ALIGN`-aligned.
    pub offset: u64,
    /// Length of the intact embedded file in bytes.
    pub length: u64,
    /// SHA-256 of exactly `[offset .. offset+length]` — per-entry integrity,
    /// identical to hashing the standalone file.
    pub sha256: Vec<u8>,
    /// Opaque per-entry CBOR metadata (domain-specific), or `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<Vec<u8>>,
}

/// Round `n` up to the next multiple of `align` (a power of two).
#[inline]
pub(crate) fn align_up(n: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (n + align - 1) & !(align - 1)
}

/// An error building or reading a `.hmc` bundle.
#[derive(Debug)]
pub enum BundleError {
    /// A key was empty.
    EmptyKey,
    /// Two entries shared a key.
    DuplicateKey(String),
    /// The byte slice is shorter than the fixed header.
    TooShort,
    /// The magic bytes were not `QBDL`.
    BadMagic,
    /// The version is not one this build understands.
    UnsupportedVersion(u16),
    /// The header's `total_length` did not match the actual byte length.
    LengthMismatch { header: u64, actual: usize },
    /// The whole-file CRC did not match (corruption or tampering).
    CrcMismatch { expected: u32, got: u32 },
    /// The index offset/length pointed outside the file.
    BadIndexPointer {
        offset: u64,
        length: u64,
        total: usize,
    },
    /// An entry's `[offset, length)` fell outside the payload region.
    EntryOutOfBounds {
        key: String,
        offset: u64,
        length: u64,
    },
    /// The CBOR index could not be encoded/decoded.
    Cbor(String),
    /// An I/O error (mmap/open), native only.
    Io(String),
}

impl std::fmt::Display for BundleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BundleError::EmptyKey => write!(f, "bundle: empty entry key"),
            BundleError::DuplicateKey(k) => write!(f, "bundle: duplicate entry key {k:?}"),
            BundleError::TooShort => write!(f, "bundle: input shorter than header"),
            BundleError::BadMagic => write!(f, "bundle: bad magic (not a .hmc bundle)"),
            BundleError::UnsupportedVersion(v) => write!(f, "bundle: unsupported version {v}"),
            BundleError::LengthMismatch { header, actual } => {
                write!(
                    f,
                    "bundle: length mismatch (header {header}, actual {actual})"
                )
            }
            BundleError::CrcMismatch { expected, got } => {
                write!(
                    f,
                    "bundle: CRC mismatch (expected {expected:#010x}, got {got:#010x})"
                )
            }
            BundleError::BadIndexPointer {
                offset,
                length,
                total,
            } => {
                write!(
                    f,
                    "bundle: bad index pointer (offset {offset}, length {length}, total {total})"
                )
            }
            BundleError::EntryOutOfBounds {
                key,
                offset,
                length,
            } => {
                write!(
                    f,
                    "bundle: entry {key:?} out of bounds (offset {offset}, length {length})"
                )
            }
            BundleError::Cbor(e) => write!(f, "bundle: CBOR error: {e}"),
            BundleError::Io(e) => write!(f, "bundle: I/O error: {e}"),
        }
    }
}

impl std::error::Error for BundleError {}
