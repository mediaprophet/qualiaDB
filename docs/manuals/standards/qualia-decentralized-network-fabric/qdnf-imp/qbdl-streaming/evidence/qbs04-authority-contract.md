# QBS-04 pinned paired-authority candidate contract

12 November 2026. Scope: explicit local full-pair bootstrap, current publication-proof authority and asynchronous provider-operation fencing. Signed controller chains, persisted anti-rollback state and independently driven hybrid handshake/key delivery remain open. QBS-04 must remain incomplete until those integration gates are qualified.

## Existing owners and boundary

The current net/qdnf/authority/AuthorityOwner already owns semantic grants, contact state and ExecutionPermit issuance. This module does not replace it or mint its permit/credential/contact types. It reuses TemporalGrant::current_at for scoped validity, ProviderGrant::authorize for provider-purpose/controller/deadline/current-grant-generation checks, RotationState::paired for monotonic controller epochs, and the existing LeaseTable for nonwrapping asynchronous slot generations. Full paired public keys and their existing COSE key IDs add the PQ trust-anchor boundary that those public local metadata records alone do not provide.

PinAuthority::new_local_bootstrap returns an owner plus an opaque LocalBootstrap admin capability. Both carry an OS-random 128-bit instance identity, using existing fill_os; entropy failure refuses creation. The admin token has no wire decoder or Clone/Copy. Trusted host setup must retain it outside untrusted network handlers. Possessing a signed claim, self-signed pair, classical-only DID proof, ProviderGrant, or another owner's admin token does not install trust. The host is responsible for authenticating the local administrator and displaying/adopting the full pair: this library does not claim user verification occurred.

## Pin and proof admission

Eight fixed pin slots retain full ML-DSA-65 (1952 bytes) plus Ed25519 (32 bytes) public keys. Installation requires local admin, a nonzero full StrongDigest principal and compact provider controller lookup, exact purpose (1..64) and audience (1..128), controller epoch, explicit provider KeyPurpose, and not_before < expires. Installation rejects expired records, malformed/weak keys and duplicate principal/purpose/audience scope. Keys are validated with existing providers; no private material is retained. Future-dated pins may be installed but cannot verify until not_before.

The compact controller lookup is used only by the legacy provider grant API; it never substitutes for full paired pin/principal equality. Scope strings are exact opaque bytes with no wildcard or implicit normalization. Every successful install/rotation gets a nonwrapping Generation; handles include owner instance, slot and generation and cannot be constructed or decoded by external callers.

AuthoritySource has LocalPin(handle), SignedControllerChain(bytes) and ClassicalDid(bytes). Only the explicitly installed local-pin profile is supported: chain input returns Unsupported and classical-only input returns Unauthorized. Neither branch imports keys or fetches URLs. verify_source checks current trusted pin validity/revocation, compares the complete expected COSE binding with its key IDs/epoch/purpose/audience, then calls the actual paired COSE verifier. No receipt exists until BOTH signatures verify. The private-constructor, non-Copy AuthorityReceipt binds the selected pin generation, full principal and descriptor digest. It is publication-proof authority under this explicit pin only, not consent, a legal licence, a semantic closure or a data-release permit. Rechecking through the same owner is required before dependent use; possession of a historical receipt alone is insufficient.

## Revocation and rotation

Admin revocation disables the existing pin and invalidates all its pending key operations. Rotation requires the same full principal, compact lookup, exact purpose/audience and provider purpose; epoch strictly increases through RotationState::paired; both public keys must change. Rotation replaces the old pair atomically after validation, assigns a new nonwrapping pin generation, and invalidates old receipts/operations. This deliberately supports prospective full-pair replacement only, with no overlap or automatic recovery. An explicit admin may rotate a revoked pin to a strictly newer full pair. Revoked slots are retained, never silently recycled or reinstalled at a lower epoch; exhaustion returns Capacity. Restart requires authenticated host restoration, including epochs/revocations; this in-memory owner does not claim durable anti-rollback.

## Provider operation fencing

Key operations reuse the existing 32-slot LeaseTable. An opaque KeyOperation binds owner/pin generations, the receipt's exact descriptor, existing provider grant snapshot, cell epoch, and an explicit deadline capped by both pin and provider validity. Begin checks a fresh valid receipt plus ProviderGrant::authorize, exact key epoch and cell epoch. The caller supplies the CURRENT provider authority generation from its existing trusted owner; this is distinct from the new local pin generation. No global generation or grant system is introduced.

Cancel releases its lease once. Completion first consumes the exact live pending lease, then rechecks pin, deadline and current provider grant snapshot/generation; any error consumes that operation and requires a fresh begin. Cancelled/old/reused/wrong-owner operations cannot consume a later slot. Revocation/rotation clear matching pending leases immediately; late callbacks fail. A KeyCompletion is only a private-constructor accounting receipt that the provider result remained admissible at completion. This module deliberately exposes no DEK, key exporter or InstalledSessionKeys constructor: actual secret release must integrate the separately confirmed handshake and existing trusted provider. A successful metadata gate is not cryptographic key confirmation.

## Limits and qualification

Fixed storage only; no wire pointer tokens, unbounded maps, generic chain parsing, IO or live network actions. Private key ownership remains in existing providers. Time values are supplied by a trusted host clock; hardware monotonicity and persisted rollback protection are not proven here. New files remain under crypto/network/authority only. Root owns module routing, builds and task status. Tests will cover actual signed proof success plus forged/untrusted/classical/chain/wrong-scope/expired/revoked/rotated/foreign-owner and cancellation/reuse/provider-change cases. Independent review and measurement remain mandatory; no production success/frozen flag is added.

## Candidate handoff

Seven new files implement the pinned profile and 12 local regression sources. Executed here: source inspection and rustfmt --edition 2021 crates/qualia-core-db/src/crypto/network/authority/mod.rs (exit 0). No Cargo build/test, network peer, secret-key release or trust enrollment was executed. Root adds the authority module export and runs the focused tests in its owned target:

    cargo test -p qualia-core-db --lib --features qdnf,pq-kem --locked --offline crypto::network::authority::

Existing parent semantic-grant/provider/rotation/runtime files were read, not edited. Full QBS-04 acceptance remains open for independent tests/review, authenticated persistent trust restoration, accepted signed PQ chains, confirmed hybrid session ownership and actual provider key delivery. An operation-completion receipt is evidence of checked metadata only.

| Candidate file | Lines | SHA-256 |
|---|---|---|
| mod.rs | 24 | 9d51eec02354ae3386023341ba0d6e2865bab0af84e397603212be262dc8a6c0 |
| types.rs | 128 | bc8ad4ce8d707f44940118cb87f7298b46f0b6a3c777e55aa58d8cde51213ccb |
| pins.rs | 241 | 2742f55891fb4451e6189d86c3a64dd291e8c96d99532e96145d7f4832de5995 |
| operations.rs | 175 | f7300cf8d41dc3f836fccc8111ac2f65524b5eb460f8f042bf5456758ef769f6 |
| test_support.rs | 110 | b4125b1ec760c6585736ddc670ad24b0cbecb1f85176138878c59091281742b3 |
| tests.rs | 261 | 230453fa298b20b5394c8d4fd7c246cf8f006bc72392cb58667f1b9fe8db6ecc |
| operation_tests.rs | 178 | b4e01883922a4967c7fd2b7c77359d683454284d2d1dce269f64be7713a9c79c |

## Existing-library and DID boundary correction (12 November 2026)

The existing did:qi GenesisPayload explicitly binds controller ML-DSA and QLink ML-KEM hashes as well as classical keys. The ClassicalDid refusal concerns insufficient classical-only evidence; it is not a claim that did:qi architecture lacks PQ binding. The inspected current did_qi document proof verifier is Ed25519-based, so a complete verified paired-key chain still needs composition with the existing genesis/document/transition owners.

PinAuthority is a public-key trust registry/profile adapter, not a new private-key store or semantic grant owner. COSE production signing now routes through the existing CryptographicLibrary/KeyManager with explicit policies and a single-use managed operation; see qbs03-cose-contract.md and crypto-library-reuse.md. The PinAuthority KeyCompletion remains metadata only. Connecting pin revocation/rotation, provider authority generations and managed SigningControl cancellation is an explicit trusted-host composition requirement, not an automatic equivalence between their generations or a completed secret-release path.
