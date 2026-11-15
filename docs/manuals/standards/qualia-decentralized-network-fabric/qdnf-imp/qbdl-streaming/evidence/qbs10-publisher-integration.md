# QBS10 B1 publisher integration handoff

Source-only increment. No build/test execution by this worker; root core5809 uses the prior routed snapshot. All files listed below are new, unrouted children. Existing parent/source files remain unchanged.

## Exact routing after the root lifts the freeze

In bundle/stream/protection/mod.rs, replace its no-keys module description with "Private protected-v1 candidate preparation; application authority and durable publication remain separate." Add:

```rust
mod ids;
mod geometry;
mod metadata;
mod publisher;
mod seal;
mod finish;
pub(crate) use ids::Mapping;
pub(crate) use geometry::PreparedGeometry;
pub(crate) use metadata::PrivateReferences;
pub(crate) use publisher::{Artifact, CandidateSink, ProtectedCandidate, ProtectedPublisher, PublisherError};
#[cfg(test)]
mod publisher_tests;
```

publisher_tests.rs itself routes publisher_boundary_tests.rs with an explicit path; do not route that child twice.

In specialized_libs/cryptographic_library/key_management/managed_object/mod.rs add only:

```rust
mod effect;
mod publication_signing;
```

In protection/source.rs, the approved narrow accessor inside PreparedSource's impl is:

```rust
pub(super) fn checked_header(&self) -> crate::bundle::stream::index::CheckedHeader {
    self.index.header()
}
```

No Cargo, enum, public-secret API, key-store or primitive changes are required. The new managed child implementation can access its parent's existing private owner/control/workspace fields. All exposed new compositor items remain crate-private; no Debug implementation exposes private metadata.

## Ownership and behavior

PreparedGeometry retains the admitted original source, caller-owned sorted opaque-ID map, geometry refs and SHA384 original-source commitment. Each source object draws a nonzero unique u32 from existing OS entropy, with at most 16 attempts per object. Mapping rows retain original source ordinal separately from sorted geometry position; central metadata_master_row receives the sorted geometry position. Actual geometry/context/root/descriptor codecs are reused.

The caller encodes Context into separate stable immutable storage before ProtectedPublisher::bind; neither owner is self-referential. The actual ManagedObjectPublication owns the same mutable KeyManager borrow and its caller-owned PublicationWorkspace. Payloads are sealed monotonically, in sorted opaque-ID order. The private M serializer copies exact original header/index and finished masters into a caller buffer, which is erased on every return or unwind. Leaf scratch is erased even for early cancellation, expiry or invalid phase before a sealer begins. Masters never become public API values.

Each ciphertext leaf/tag and actual TreeBuilder outboard node is candidate-staged through the same managed control fence, with current provider policy and fresh clock checks before and after the bounded synchronous sink effect. A sink error, unwind, post-effect expiry or finish/sign error poisons and cancels the whole attempt, including after individual sealers have completed. The callback must not await or reenter managed control; staged bytes must remain discardable candidates. This is not a rollback guarantee for an arbitrary external side effect.

Finish recomputes the original source commitment, checks exact geometry/root alignment and canonical proof::empty_root for every zero-length object, replays the actual object_stream::Inventory, and only then stages the descriptor. Empty objects have no key, AEAD tag or leaf. Metadata is nonempty and encrypted. Signing reborrows the same KeyManager for the existing managed paired COSE signer; exact descriptor bytes/context, digest, fixed protected-v1 publisher purpose, scope-token audience, controller epoch and managed policy are checked. Signing executes without holding a content control lock; final copy/staging takes the fence again, avoiding recursive-lock deadlock.

## Authored verification, not execution evidence

The two new test children cover real managed ChaCha20-Poly1305 sealing plus SHA384 ciphertext trees and paired COSE signing; exact M layout/ordinal mapping; empty-root/no-leaf behavior; candidate public-byte privacy checks; error/unwind after completed sealers; cancellation/expiry before sealing and expiry during a sink; scratch erasure; missing signing policy, wrong key ID, digest, audience, generation and valid alternate descriptor refusal; deterministic forced reverse opaque-ID mapping, 16-draw zero/collision bounds and entropy failure. Independent primitive/codec fixtures remain those of Phase A; these new composition tests are not independent cryptographic qualification.

Root validation selector after routing: cargo test -p qualia-core-db --lib --features qdnf,pq-kem --locked --offline bundle::stream::protection -- --test-threads=2 (root owns build scheduling).

## Explicit remaining gates

This increment produces candidate artifacts, not application authorization or durable/public promotion. PrivateReferences are explicitly unverified input references; recognized authenticated closure, exact source whole-source grant, current publisher/recipient pin verification, Sentinel/labels/consent and consumed EstablishedSession PacketProtection delivery still belong to B2. No recipient wrap is emitted here. Actual private-file/candidate sink reservation, cleanup, sync/rename and crash recovery must be supplied by the durable storage compositor. Host immutable-snapshot/writer exclusion remains mandatory for externally backed source memory. A completed ProtectedCandidate is only an identifier pair, not an authority receipt.

Working bounds: <=8193 source objects/65 pages; caller metadata <=8MiB; leaf scratch exactly16400 bytes; bounded local geometry1560/root6728/COSE4608 scratch plus existing TreeBuilder frontier/provider workspace. Existing provider cold allocations and caller storage reservations are not claimed to be zero heap or automatically charged by this structural adapter.

Integration update: after root core5809 passed, the approved three-file parent batch above was applied and formatted. All publisher children are now routed; no worker core build was run. Root reported prior B1/source bundle selector148/148 passed before this publisher batch.
