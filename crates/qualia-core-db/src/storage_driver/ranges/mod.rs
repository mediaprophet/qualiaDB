//! Canonical native range staging (QBS11 phase B), not verified durability.
//!
//! Contract written before implementation:
//! - MmapDriver::create_range_staging consumes a private non-Copy StagingCharge
//!   from the EXISTING CustodyQuota. No copied caps/new quota/root registry.
//! - The token charges the complete explicit u64 allowance before any staging
//!   filesystem mutation. Its root is borrowed; it does not retain a mutable
//!   quota borrow, so several admitted attempts can coexist. Zero attempt IDs
//!   and allowances refuse; every charge survives failure, close, drop/unwind.
//! - RangeLimits contains separate u64 payload/proof/checkpoint ceilings.
//!   Checked positive sum must fit the charge; individual zero ceilings are
//!   allowed. File lengths grow only through bounded writes, not preallocation.
//! - Only exact constant data filenames payload/proof/checkpoint are used inside one
//!   private TempDir created under the same canonical existing MmapDriver root.
//!   A create-only 64-byte .owner control marker binds magic/version, zero
//!   reserved bytes and the exact admitted 32-byte attempt ID. Marker/filesystem
//!   control overhead is host-covered, outside the three data-byte ceilings,
//!   matching existing quota semantics for head/lock/filesystem overhead.
//!   create_new, Unix0600 (directory0700), Windows share_mode(0), and retained
//!   File::try_lock are used. Linux locks are explicitly cooperative only.
//! - read_exact_at/write_exact_at use caller slices, checked u64 offset+length,
//!   and actual File seek/read_exact/write_all. No whole-value read or mmap.
//!   Sparse/zero reads are bytes only and never imply verified availability.
//! - sync_file observes actual File::sync_all return. No method returns a
//!   DurableVerifiedRange, atomic publication, serving permission or financial
//!   receipt. Windows directory power-loss durability remains unqualified.
//! - Success close drops all file handles before TempDir::close. Cleanup errors
//!   return a private cleanup-pending owner retaining path/root/charge for retry.
//!   Explicit recursive retry first verifies the absolute owned child remains
//!   within the resolved canonical parent, rejects links/reparse points, and
//!   verifies the exact bounded ownership marker before recursive deletion.
//!   Native partial cleanup may remove the marker first. ONLY after a cleanup
//!   started from verified ownership then failed, scoped recovery rechecks the
//!   canonical child and restores the internally derived marker with create_new
//!   if absent. Any existing marker must validate exactly and is never overwritten.
//!   Initial missing/malformed marker refusal never enables restoration.
//!   A missing/malformed/replaced marker retains pending ownership and disarms
//!   TempDir automatic cleanup. This is host-protected namespace ownership,
//!   not marker authentication against an actor allowed to replace that root.
//!   Drop/unwind closes handles then uses the same child/marker guard. Because
//!   Drop cannot return pending ownership, any refused/failed cleanup leaves
//!   charged uncertain artifacts; explicit close preserves retry diagnostics.
//!   A creation failure before a valid marker exists also retains pending
//!   ownership rather than bypassing marker validation. Host protected-root
//!   namespace stability is required between validation and cleanup. Quota is
//!   never refunded. No directory scan or arbitrary-path cleanup API exists.
//! - Host owns protected root ACL/ancestor/locality, authenticated usage freshness,
//!   aggregate quota lease, resource-stage/policy/deadline checks and worker drain.
//!   Root/backend mutation errors can leave uncertain bytes, never durable success.

pub mod recovery;
mod session;
mod types;
pub use session::RangeSession;
pub use types::*;

#[cfg(test)]
mod tests;
