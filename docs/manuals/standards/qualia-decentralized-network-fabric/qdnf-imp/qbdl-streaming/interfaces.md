# QBS-01 interface closure record

Implementation contracts are reviewed incrementally; QBS-01 remains in progress until all required service/carrier and policy/consumer boundaries are frozen. These candidates do not claim public registry allocations.

Accepted sub-contracts: [bounded index](evidence/qbs01-index-contract.md), [paired COSE bytes](evidence/qbs03-cose-contract.md), [entry proofs](evidence/qbs05-proof-contract.md), [borrowed runtime owners](evidence/qbs08-runtime-contract.md), and [plaintext descriptor candidate](evidence/qbs06-descriptor-contract.md). The [CDDL shape](schemas/qasd-plain-v1.cddl) supplements the strict codec rules; it does not express their canonical encoding or cross-page checks.

The central [crypto-library reuse correction](evidence/crypto-library-reuse.md) is mandatory before production signing integration. Existing general signing profiles remain unchanged; COSE needs its explicit raw-message/empty-context profile. Full public pins are a publisher trust adapter, not another secret vault or semantic grant owner. did:qi already binds PQ key hashes in genesis; verified full-key chain composition is separate work.

Root owns shared routing, public schemas, manifests and task state. Cryptographic algorithms remain in QualiaDB's cryptographic library. Index admission proves structure only; entry proof verification proves integrity against its supplied root; source authority, policy/consent, plaintext authentication, inner-format validation, consumer commit, custody and settlement remain separate boundaries. Runtime credit proves reserved capacity and needs current source/policy authorization before a request. Cancellation retains I/O ownership until completion; scheduler identity uses exact records, never compact prefixes.

Plain descriptor candidates expose original header/index/source commitments and require whole-bundle disclosure authorization before public delivery. Ciphertext/confidential inventory cannot silently reuse the plain profile. Future native/HTTP/WSS/WebRTC carriers must preserve the same authority/proof/policy checks; TLS/DTLS alone is not a PQ claim. Exact carrier record/version identifiers and protected-object contracts remain pending.

## Initial QBS1 native service allocation (2026-11-13)

The root-reviewed [wire contract](evidence/qbs09-wire-contract.md) defines the initial internal service namespace `q42:QBDLStream/1`: QBS1 magic, version1/profile1, tags1–12, exact208-byte prefix, and the separate `QBDL-SCT-V1` NUL-terminated12-byte record domain. It reuses QSession frames and the same consumed endpoint packet owner. This is not an external registry or release-acceptance claim.

The maximum service record is65,776bytes, selected plaintext leaf record18,137bytes, Merkle inclusion proof1,537bytes, paired COSE envelope4,608bytes and signed closure manifest2,048bytes. The existing protected package domain and8,192-byte cap remain separate. All parsed records are untrusted; current authority, actual reservations, exact selected identity, proof verification and one-response state transitions are required by the owning runtime. The codec and its fixtures do not implement those effects or establish reliable-carrier qualification.
