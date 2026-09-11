# Handoff — DID-QI-IMPL Qualia Identifier CRUD runtime

## Result

- Package and child IDs: **DID-QI-IMPL** child **done** for the allowed write set. Parent **DID-QI** remains integrator-owned (spec file exists; runtime is not wired into `lib.rs`). Do not tick CSCP-08/12. Method is **not** registered.
- Changed files (allowed writes only; no `lib.rs`, no `Cargo.toml`, no `identity/`, no honesty flags):
  - `crates/qualia-core-db/src/did_qi/mod.rs`
  - `crates/qualia-core-db/src/did_qi/id.rs`
  - `crates/qualia-core-db/src/did_qi/document.rs`
  - `crates/qualia-core-db/src/did_qi/method.rs`
  - `crates/qualia-core-db/src/did_qi/git_object.rs`
  - `crates/qualia-core-db/src/did_qi/utxo.rs`
  - `crates/qualia-core-db/src/did_qi/service.rs`
  - `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/handoffs/DID-QI-impl.md`
- Public behavior: frozen API `METHOD_NAME`, `DidQi`, `format_did` / `parse_did`, `create` / `read` / `update` / `deactivate` over `QiStore`. In-process `GitObjectStore` (32 slots). Caller-supplied Bitcoin-family tx parse. No network, Chronik, git daemon, or DHT.
- Accepted predecessor: DID-QI-01 (`decisions/did-qi-git-utxo.md`); `docs/manuals/standards/did-qi-method.md` v0.1.0 encodings used without editing that file; human-centric nomenclature (☉ HCInet, not 🎯 HCAI).
- Evidence: standalone crate tests (module not yet in `qualia-core-db` `lib.rs`):
  - `CARGO_TARGET_DIR=/tmp/qdnf-continue-target cargo test --manifest-path /tmp/did-qi-check/Cargo.toml --offline --lib -- --test-threads=1` → **22 passed**, 0 failed.
  - Log: `/opt/cursor/artifacts/did-qi-standalone-tests.log`
  - rustc via cargo 1.x, Linux x86_64, `--offline`.
- Checks executed: create/read round-trip (generation 0); update bumps to 1 and old generation is stale; deactivate then update fails closed; relay-only Direct locator rejected; git blob SHA-256 deterministic; UTXO live OP_RETURN `6a23 5149 01` matches unsigned digest, wrong digest fails, `5149 ff` tombstone deactivates; `parse_did` rejects `did:q42:` / `did:hcai:` / `did:hci:` / `did:qualia:` / hostname form; empty store read `NotFound`; spec vector DID `did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu` from QCDE-1 genesis; spec vector 1 live tx parses.
- Checks not executed: `cargo test -p qualia-core-db --lib --offline did_qi` — **blocker**: integrator has not added `pub mod did_qi;` at crate root. Live Chronik, git daemon, public DHT, Gate B: out of scope.
- Resource bounds: `GitObjectStore` cap 32; unsigned QCDE-1 buffer 2048 octets (spec max 8192; this bounded subset fits test vectors); hot parse/verify/read has no `String`/`Box`. Fail-closed: `NotFound`, `Deactivated`, `DirectLocatorForbidden`, `CommitmentMismatch`, `RejectedMethod`, `StoreFull`.
- Review findings: none by this worker (implementer). Independent review is the integrator’s next step after wiring.
- Provisional assumptions: QCDE-1 encoder is the restricted document profile in `did-qi-method.md` §8 (lexicographic ASCII keys, no floats), not a general RFC 8785 library. Git slot payload is packed `flags||generation||unsigned QCDE-1||Ed25519` hashed as `blob <len>\0` + bytes (SHA-256). Spec §16 git id of the *full signed JSON document* is available via `encode_signed` + `blob_object_id` but is not the slot payload. UTXO `txid` in `Outpoint` is SHA-256d wire order (not RPC display reverse).

## Integration

- Shared exports / Cargo / honesty flags / `workstream.md` / `task-registry.json`: **one integrator edit** — in `crates/qualia-core-db/src/lib.rs` add `pub mod did_qi;` at crate root (not under `identity::`, so the filter `did_qi` does not hit `identity::`). Do **not** add this worker’s module under `identity/`. `Cargo.toml` already has `ed25519-dalek` and `sha2`.
- Consumer packages: none until wired. After `pub mod did_qi`, rerun:
  ```
  CARGO_TARGET_DIR=/tmp/qdnf-continue-target
  cargo test -p qualia-core-db --lib --offline did_qi -- --test-threads=1
  ```
- Merge/conflict: only `crates/qualia-core-db/src/did_qi/` (new) and this handoff. Concurrent Cargo.toml / CSCP briefs / `did-qi-method.md` were **not** modified.
- Rollback: delete the `did_qi/` directory and this handoff. No ABI in the published crate until `pub mod` exists. No durable schema.
- Outstanding ownership: integrator wires `pub mod did_qi`, reruns the in-crate test filter, optionally requests independent review. This worker must not edit `lib.rs`.
- Next owner: **integrator**. Next action: add `pub mod did_qi;` then run the cargo filter above. Proposed registry transition: DID-QI-IMPL evidence = this handoff + 22 standalone tests; parent DID-QI stays incomplete until in-crate tests pass. **Do not register `qi`. Do not mark Gate B / CSCP-08/12 complete.**

Honesty: **create/read/update/deactivate work with tests in the module.** They are not yet visible to `qualia-core-db --lib` until the integrator adds `pub mod did_qi`.
