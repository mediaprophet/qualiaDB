# QPR Post-Quantum Security and Crypto Reuse

**Status:** Proposed replacement-runtime security profile; integration and independent review required

**Working profile:** `qpr-pq-1`, target default for the new implementation

## 1. Security objective and profile boundary

The new [Qualia Peer Runtime](./peer-runtime.md) targets quantum-resistant confidentiality and
authentication using established primitives from QualiaDB's crypto libraries. The innovation is
their integration with governed peer operation, compact evidence and bounded execution. This design
does not invent a cipher, KEM or signature algorithm, or assert that the proposed protocol is proven.

The earlier `qdnf-crypto-1` suite is a classical compatibility profile. It cannot satisfy the QPR
replacement's post-quantum release claim. `qpr-pq-1` is a new, explicitly negotiated profile, with
distinct transcript/record/schema versions. Its identifiers and larger digest fields cannot be
inserted into version-1 bytes without a version change. No silent fallback is permitted.

Distinguish three claims: resistance to later decryption of recorded traffic, resistance to an
active quantum attacker forging authentication, and durability of signed historical evidence.
Hybrid key establishment alone addresses neither the controller-authority chain nor stored-record
forgery. Each capability, route, contract, checkpoint and recovery authorization in a claimed PQ
trust path must have an accepted PQ binding. Transport upgrade does not repair classical-only roots.

## 2. Target algorithm choices

| Purpose | `qpr-pq-1` choice | Role |
|---|---|---|
| Online key establishment | Fresh X25519 plus ML-KEM-768 | Combine classical and post-quantum contributions, with validated inputs and transcript-bound key confirmation |
| Controller/session and durable record proofs | ML-DSA-65 and Ed25519, both required under one bound proof policy | Migration hedge against failure of either family; never accept whichever signature happens to verify |
| Transcript, authority and artifact commitments | SHA-384 with explicit algorithm/length | Versioned full digests outside compact Quin fields; avoid a hidden SHA-256-only commitment bottleneck |
| Key derivation and private tokens | HKDF-SHA-384 / HMAC-SHA-384 with separate purpose labels | Independent link/session/record/discovery keys; no ambiguous concatenation of secrets or contexts |
| Packet encryption | ChaCha20-Poly1305 with 256-bit keys | Symmetric traffic protection, bounded usage and nonce discipline; retain separate forgery/data limits |
| Optional offline recovery/root evidence | SLH-DSA-SHA2-256s under a separate profile | Hash-based algorithm diversity for infrequent evidence, not packet signing or an automatic fallback |

ML-KEM, ML-DSA and SLH-DSA are specified respectively by
[NIST FIPS 203](https://csrc.nist.gov/pubs/fips/203/final),
[FIPS 204](https://csrc.nist.gov/pubs/fips/204/final) and
[FIPS 205](https://csrc.nist.gov/pubs/fips/205/final). Parameter-set labels do not establish one
end-to-end security category for the protocol. Hashes, AEAD tags, authentication roots, implementation
leakage, and key handling all constrain a claim. Dependency names do not imply FIPS module validation.

## 3. Existing libraries and repair boundaries

| Repository anchor | Existing implementation | Required network integration |
|---|---|---|
| [crypto dependencies](../../../../crates/qualia-core-db/Cargo.toml) | `fips203`, `fips204`, `fips205`, SHA-2 and HKDF dependencies; ML-KEM shim behind `pq-kem` | Select feature closure explicitly; audit actual locked versions, advisories and native/WASM behavior |
| [KEM shim](../../../../crates/qualia-core-db/src/crypto/pq_kem_shim.rs) | Real ML-KEM key generation/encapsulation/decapsulation with fixed-size wrappers | Name on-wire algorithms ML-KEM, not historical Kyber variants; remove secret `Copy`/`Debug` exposure, provide zeroizing ownership, bounded nonallocating errors and measured workspaces |
| [ML-DSA signer](../../../../crates/qualia-core-db/src/crypto/fiduciary_crypto.rs) | Real ML-DSA-65 signing/verification | Keys/signatures/context use `Vec`; current context hashing concatenates domain and purpose without lengths. Add a versioned unambiguous network context and caller-buffered primitives without changing old signatures implicitly |
| [crypto library signing](../../../../crates/qualia-core-db/src/specialized_libs/cryptographic_library/signing.rs) | ML-DSA and SLH-DSA calls | Reuse primitive dispatch, separate bounded protocol verification from authoring/audit allocations; test selected COSE semantics |
| [key management](../../../../crates/qualia-core-db/src/specialized_libs/cryptographic_library/key_management/mod.rs) | Generation, vault/access, rotation and recovery structure | Bind paired keys, authorized epochs and recovery thresholds to verified controller records; no generic algorithm label as proof of authority |

Implement focused network-facing adapters beside the existing crypto owners; do not fork primitive
implementations into the peer library. A fixed-size struct is not evidence of zero allocations in
the dependency or its rejection paths. Large signature/KEM variants must be accounted at their
actual enum/slab size, including worker stacks and any secret copies. Do not embed kilobyte proof
arrays in every peer-table entry; use leased buffers and validated evidence handles.

## 4. Online handshake design

[RFC 10024](https://www.rfc-editor.org/rfc/rfc10024.html) standardizes X25519MLKEM768 for TLS 1.3.
Its client share is ML-KEM public key followed by X25519 public key; its server share is ML-KEM
ciphertext followed by X25519 public key. Its combined secret orders the ML-KEM contribution first.
QPR adopts those component encodings/order as prior art. QSession is not TLS, so the QDNF transcript,
combiner use and state machine still need their own review and vectors.

The proposed QPR exchange has explicit stages to avoid deriving encrypted handshake keys from a
transcript that already includes the ciphertext they would encrypt:

1. **Reachability and admission:** exchange a compact offer/retry nonce and prove return reachability
   or an existing relationship. Reserve a handshake slot, bytes, work and deadline. This grants no
   identity, application permission, or security downgrade.
2. **Key shares:** initiator provides fresh X25519 and ML-KEM-768 encapsulation keys; responder
   returns fresh X25519 and an ML-KEM ciphertext. Bind roles, nonces, offered/selected profiles,
   observed path/bearer, retry chain, target and permitted context into the share transcript.
3. **Handshake keys:** validate component keys/lengths and reject all-zero X25519. Feed the ordered
   32-byte ML-KEM secret plus 32-byte X25519 secret into transcript-bound HKDF-SHA-384. Use separate
   link/session domains and directional handshake labels. Neither secret may be omitted or XORed.
4. **Authentication:** encrypted proof messages bind the share transcript, paired controller keys,
   full authority evidence, service request, RAR/DNI and critical features. Verify both selected
   signatures and controller authority. Anonymous requester mode must still meet its declared
   capability-possession profile; it cannot claim mutually authenticated principals.
5. **Confirmation and traffic:** each party proves possession of the combined secret with a Finished
   MAC over the authenticated transcript. Derive application traffic/exporter keys from the confirmed
   state with separate labels. No application payload or irreversible operation precedes this boundary.
6. **Erasure:** erase ephemeral KEM decapsulation keys, DH keys and intermediate secrets after their
   required confirmation/retransmission lifetime. Retransmissions use retained exact handshake bytes,
   not regenerated randomness under a reused transcript or nonce.

ML-KEM's implicit rejection behavior must remain intact. A decapsulation result is not, by itself,
evidence of valid peer input or identity; Finished/authentication failure rejects the handshake
without a distinguishable secret-dependent error oracle. Use audited constant-time primitives and
bound failed attempts before expensive work.

QLink and QSession use distinct exchanges/secrets. A PQ QSession can protect payloads over a weaker
or hostile carrier, but cannot claim PQ protection of classical link/route metadata. A full native
PQ claim requires hybrid QLink plus PQ-authenticated control authority. Packet forwarding uses
symmetric authentication and current compiled evidence handles, not ML-DSA per packet.

Traffic-key derivation from an old secret is key update, not recovery from compromise. Fresh hybrid
rehandshake and newly verified authority are required for any recovery claim; periodic rekey alone
does not provide post-compromise security. Disable 0-RTT application data in the initial PQ profile.

## 5. Dual proofs, semantic binding and algorithm agility

The initial proof bundle contains two COSE_Sign1 objects over the same exact deterministic-CBOR
binding object. That object names the proof-policy version, both algorithms and key references,
controller/epoch, purpose, audience, record type, exact payload or its typed full digest, and semantic
bundle digest where applicable. Both signatures are required. Altering the pair, stripping a proof,
changing the contract context, or importing a proof from another role fails validation.

Use [RFC 9964](https://www.rfc-editor.org/rfc/rfc9964.html) for the ML-DSA COSE algorithm/key profile
(ML-DSA-65 has COSE algorithm value `-49`); this is distinct from QDNF's unsigned local registry IDs.
Follow its signing input and FIPS context rules exactly. The current `CryptoContext` wrapper is not
automatically interoperable. Freeze both COSE proofs, external associated data and shared binding
object bytes; this is a new application proof policy, not a claim that two signatures form a newly
standardized composite primitive.

All security-relevant references in the PQ record schemas use typed digests, initially SHA-384
(48 bytes). The existing SHA-256 operation/checkpoint recipe in the initial QSync draft is a
different record profile. The PQ variant uses new domain/version labels and SHA-384 throughout the
operation ID, envelope/content, semantic bundle and Merkle tree commitments. Recompute from exact
source bytes: hashing an old 32-byte digest into 48 bytes does not upgrade its collision resistance.

Q42 indexes, 60-bit hashes and 128-bit DNI routing coordinates remain lookup/routing aids. Verify
full evidence at endpoints/admission boundaries. Do not expand the 48-byte NQuin ABI or assume a
48-byte SHA-384 value fits alongside all Quin fields. Persist exact digests in core evidence records
and reference them with collision-checked handles. Decode variable digest lengths only within the
fixed allowlist and schema bounds; never assume every digest is `[u8; 32]`.

Profile negotiation authenticates the entire offered set, selected suite and minimum local policy.
Cache known peer/realm minimums with authority and expiry; an untrusted failure cannot lower them.
An attacker who strips PQ support causes failure, not classical retry. Legacy mode requires explicit
local policy and is reported as classical; it is excluded from PQ conformance and cannot inherit a
PQ-required contract or confidential-data grant. Certificate/controller reissuance, recovery and
rollback follow the same minimums. Multiple independent signatures can express an M-of-N rule;
that is not a threshold ML-DSA algorithm or an aggregate-signature proof.

## 6. Wire size, fragmentation and denial of service

| Component | Encoded primitive bytes |
|---|---:|
| ML-KEM-768 public key / ciphertext | 1,184 / 1,088 |
| Hybrid initiator / responder share, including X25519 | 1,216 / 1,120 |
| ML-DSA-65 public key / signature | 1,952 / 3,309 |
| Ed25519 public key / signature added to a dual proof | 32 / 64 |

KEM sizes follow RFC 10024/FIPS 203; ML-DSA sizes follow FIPS 204 and the repository's signer
constants. Framing, two-way proofs, authority chains, ciphertext tags and semantic references add
more. A single dual signature is already 3,373 bytes before those extras. Do not place full proofs
in beacons or pretend they fit a 1,200-byte datagram.

The current QLink rule permits fragmentation only after neighbor authentication. PQ bootstrap
therefore requires an explicit **admission-stage handshake chunk profile**, not an implicit exception
or a temporary classical session represented as PQ. A cookie/relationship MAC gates a bounded slot
and binds locator, handshake ID, offered profile, expected total length and expiry; it proves return
reachability/secret possession only. Initially cap one handshake flight at 16 KiB and 16 chunks,
with at most 32 in-progress handshakes globally and two per admitted locator/relationship, further
limited by the aggregate arena. Count all pending flights, retransmits, keys and worker scratch.

Before return validation, enforce a small fixed reply/amplification budget; afterwards enforce
independent global/work limits because cookie holders can still attack. Authenticate each chunk
where a relationship secret permits it; otherwise bind complete ordered chunk bytes in the final
hybrid transcript before trust. Duplicates do not extend deadlines. Conflicting lengths/chunks
terminate bounded state. These messages never become application data or general unauthenticated
64 KiB reassembly. Their cookie, framing and retry vectors must freeze before PQ QLink is enabled.

If a bearer cannot carry a flight within its chunk/MTU limits, report unsupported profile or use an
explicitly authorized capable route; do not drop the PQ component. Large authority chains/SLH-DSA
objects use prior cached evidence or separately bounded retrieval after an adequate secure channel;
the initial profile may reject uncached oversized chains. Parsing untrusted evidence provisionally
does not authenticate it. The first native PQ slice should use short, provisioned authority chains.

## 7. Custody, recovery and long-lived evidence

The classical X25519 HPKE envelope in `qdnf-crypto-1` does not satisfy a PQ custody claim. Define a
separately versioned hybrid recipient-key envelope using the chosen established KEM/KDF/AEAD
construction; do not label arbitrary KEM concatenation as RFC 9180 HPKE interoperability. Bind
recipient paired keys/epoch, expiry, purpose, inner operation ID and ciphertext digest, and authenticate
the sender under the accepted record profile. Keep this feature disabled until its envelope and
key-evolution vectors are independently tested. Offline retrieval remains optional for the first
online replacement release.

Static recipient-key compromise can expose stored ciphertext even with PQ algorithms. Forward
secrecy requires a tested ephemeral/prekey/ratchet lifecycle and actual erasure. Rewrap retained data
keys and reissue authorizations under approved migration policy when algorithms/keys change; retain
original evidence and migration provenance without claiming that an old forged signature becomes
trustworthy by being re-signed. A recovery authority must itself meet the PQ policy before replacing
controller keys. SLH-DSA root evidence offers optional algorithm diversity with different size/work
costs; it never automatically substitutes for the mandatory online proof pair.

## 8. Implementation and acceptance

P14 in [Implementation and Conformance](./implementation-conformance.md#qdnf-p14--post-quantum-replacement-security)
owns the crypto adapters, paired authority, transcript/proof/digest schemas and handshake chunking.

Required evidence includes primitive known-answer and malformed-input vectors, independent hybrid
transcripts and Finished checks, cross-role/key-pair substitution, stripping/downgrade/rollback,
wrong semantic digest, real revocation, replay and restart. Measure allocations, stack and arena
peaks, entropy failures, secret zeroization and cancellation at each crypto stage. Test short MTUs,
full handshake queues, malicious lengths, missing chunks, burst verification and concurrent cold work.

Benchmark complete authorized connections and durable record validation, including PQ bytes,
evidence caching, energy/time and denied inputs. Audit side-channel behavior and dependency versions;
passing round trips does not establish quantum resistance or a reviewed protocol. Report separate
claims for PQ sessions, native link/control authority and offline custody. The production replacement
target requires the first two; reduced classical demonstrations remain explicitly experimental.
