# Handoff — DID-QI-SPEC Qualia Identifier method specification

## Result

- Package and child IDs: **DID-QI-SPEC** complete for this assignment. Parent **DID-QI** remains pending (DID-QI-IMPL runtime, DID Spec Registries registration, live git/UTXO not in this child). Do not tick parent complete.
- Changed files (allowed writes only; no `.rs`, no `Cargo.toml`, no honesty flags):
  - `docs/manuals/standards/did-qi-method.md` (new W3C CG-style DID Method specification)
  - `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/handoffs/DID-QI-spec.md` (this file)
- Public behavior: **unchanged**. No resolver, no minted production DIDs, no CSCP wire change, no `parse_did_q42` change.
- Accepted predecessor/interface versions: DID-QI-01 naming/backing split in `cscp-imp/decisions/did-qi-git-utxo.md`; human-centric nomenclature; identifier-resolution.md §3 QRC split; HCAI-ANP `did:web` Frontdoor-only. W3C DID Core 1.0 CRUD bar.
- Evidence: the specification itself. CRUD is fully specified (Create §11, Read §12, Update §13, Deactivate §14) with algorithms and fail-closed error tokens — not stubs. Four numbered offline vectors (create, update, deactivate, relay-only forbids Direct) plus parse-reject table.
- Checks executed: source read on branch `0.0.38` at `a8164506`; SHA-256 / Bitcoin base58btc / RFC 8032 Ed25519 / git-object / OP_RETURN vectors computed locally with Python `hashlib` + `cryptography` (no network). Spec grep: no `did:☉` in DID strings; no CSCP-08/12 completion claim; method not registered.
- Checks not executed: `cargo test` / DID-QI-IMPL (forbidden `.rs` in this child); live git daemon; live chain; DID Spec Registries insert; CSCP-08 URL; CSCP-12 datatracker.
- Measured resource bounds: unsigned QCDE-1 cap 8192 octets; signed 9216; max 4 verification methods, 8 services, 8 relay hints. Enforcement is specified; not measured in a runtime (no implementation in this child). Error behavior: tokens in spec §4 fail closed.
- Review findings: not self-approved as a registered method. Independent reviewer is the parent integrator / DID-QI-IMPL author against “is this a DID method spec?” — intended answer **yes**.
- Provisional assumptions still blocking **parent** acceptance: method string `qi` vs reserved `hcinet` (progress log: principal did not choose `did:hcinet`; spec uses `qi`); first UTXO constitution list (vectors demonstrate `bip122:` parameterization, do not pick eCash as the only chain). Removed assumption: that `did:q42` or `did:web` could stand in for HCInet instrument identity.

## Integration

- Shared exports / Cargo / profile / registry: **none requested**. Integrator must not add `pub mod did_qi` from this child. DID-QI-IMPL owns `crates/qualia-core-db/src/did_qi/`.
- Consumer packages and exact interface migration: DID-QI-IMPL MUST hash QCDE-1 (RFC 8785 subset), derive `did:qi:z` || base58btc(SHA-256(genesis payload)), sign `did:qi:document:v1 || 0x00 || unsignedDigest` with Ed25519, reject Direct locators under relay-only, map `qi.generation` to git tag analogue and CSCP `ContactDescriptor.generation`. Test vectors in spec §20 are the fixture bytes.
- Merge/conflict: only the two markdown paths above. Concurrent DID-QI-IMPL / CSCP-09 workers own disjoint files. Do not commit unrelated `Cargo.lock` / `Cargo.toml` / workstream edits from the swarm tree.
- Rollback/recovery: delete the two markdown files. No schema, no durable store, no ABI. Durable compatibility: none (spec-only).
- Outstanding ownership, temporary artifacts and cleanup: integrator may record this handoff in `cscp-imp/progress-log.md` / task-registry; this worker must not edit those files. Temporary artifacts: none (vector generation was in-process, not written to disk).
- Next owner: **DID-QI-IMPL** implements create/read/update/deactivate against spec §20. Integrator wires `pub mod did_qi` after review. Human still owns CSCP-08 URL and CSCP-12 datatracker. Registration of `qi` is not authorised.
- Proposed registry transition: child DID-QI-SPEC = specified. Parent DID-QI stays **not complete**. Do not mark CSCP-08/12 done. Do not set Internet honesty flags.

Honesty: **a DID method spec is not a relay, not Gate B, and not a registry entry.**
