# QBDL streaming focused conformance harness

This independent Cargo workspace imports the actual production source files through path modules. It isolates parser, proof, signature and runtime tests from the larger renderer/inference dependency graph. It does not replace full-core, browser, native-bearer or release qualification.

Run from the QualiaDB root: cargo test --manifest-path tools/qbdl-stream-conformance/Cargo.toml --locked --offline. The checked-in lock file pins test dependencies. Keep source paths and imported production owners intact; no substitute implementation or qualification stub belongs here.

The index corpus contains independently constructed structural byte fixtures. generate-tree-v1.mjs computes reference SHA-384 trees recursively with Node crypto. generate-cose-encoding-v1.mjs independently encodes the strict paired COSE profile; its default signatures are intentionally invalid. generate-cose-openssl-v1.mjs uses independent Node/OpenSSL ML-DSA-65 and Ed25519 providers and retains public keys/signatures only. Exact fixture bytes are checked in; regenerating randomized signature fixtures produces different valid bytes. Node documentation: https://nodejs.org/api/crypto.html#cryptosignalgorithm-data-key-callback.

The thread-local allocator counter measures only marked operations and has an allocating positive control. Metadata/runtime accounting tests establish no actual disk durability, network interoperability, consumer permission or GPU admission. Full qualification remains governed by the QBS task registry.

After managed signing integration, the focused COSE slice imports the actual codec/types/verifier/internal fixture signer and central signing_profile sources. It deliberately omits the managed-custody adapter and never substitutes a fake KeyManager or NQuin. Managed policy, key storage and central facade tests must run in the real core build.

The QASD corpus independently constructs full QBDL inputs, source commitments, context/pages and descriptor IDs with Node CBOR/SHA-2/CRC code. Storage fixed-wire vectors are untrusted checkpoint hints, not authenticated persistence. Production bounded semantic traversal imports the existing DependencyKind, manifest/canonical/error dependencies; it does not copy semantic types. The real publisher and custody tests remain in the full core package.
