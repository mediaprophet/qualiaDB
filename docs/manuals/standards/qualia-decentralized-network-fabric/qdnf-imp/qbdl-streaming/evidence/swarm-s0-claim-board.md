# Swarm S0 claim board

Read-only support inventory. No implementation, no cargo, no kill, no git commit/push, no production Rust edits.

Captured: 2026-11-13 from `C:\github\qualiaDB`.

## 1. Git snapshot

```
branch:  codex/production-integration-20261113
HEAD:    04cfa2ad280de0df57ca2fa0a1f8f4e8fda0445a
```

`git status --porcelain` (20 lines):

```
 M AGENTS.md
 M docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/qbdl-streaming/progress-log.md
?? docs/graph-theatre-foaf-framed.png
?? docs/graph-theatre-foaf.png
?? docs/graph-theatre-star.png
?? docs/graph-theatre-webgl2.png
?? docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/qbdl-streaming/evidence/
?? docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/qbdl-streaming/task-registry.json
?? docs/manuals/standards/references/
?? tools/qbdl-stream-conformance/fixtures/generate-cose-encoding-v1.mjs
?? tools/qbdl-stream-conformance/fixtures/generate-cose-openssl-v1.mjs
?? tools/qbdl-stream-conformance/fixtures/generate-endpoint-v1.mjs
?? tools/qbdl-stream-conformance/fixtures/generate-protected-v1.mjs
?? tools/qbdl-stream-conformance/fixtures/generate-qasd-plain-v1.mjs
?? tools/qbdl-stream-conformance/fixtures/generate-service-wire-v1.mjs
?? tools/qbdl-stream-conformance/fixtures/generate-storage-v1.mjs
?? tools/qbdl-stream-conformance/fixtures/generate-tree-v1.mjs
?? vendor/etalon-model-6ace6cc/.gitignore
?? vendor/etalon-model-6ace6cc/tests/support/p64_stream_golden/
?? vendor/etalon-model-6ace6cc/tests/support/p64_writer_golden/
```

Counts: **modified = 2**, **untracked = 18**, porcelain lines = 20.

Streaming-owned dirty (not Wave S1 exclusive; integrator/docs): `progress-log.md`, `evidence/`, `task-registry.json`.

## 2. Concurrent dirty files to preserve

Must not be attributed to streaming Wave S1 and must not be deleted. Anything outside `docs/.../qbdl-streaming/` and `crates/qualia-core-db/src/{bundle/stream,net/peer/replication/qbdl,net/qdnf,crypto/network,p2p/connectivity}`, plus vendor etalon goldens, graph-theatre docs, `AGENTS.md`.

| Status | Path |
| --- | --- |
| ` M` | `AGENTS.md` |
| `??` | `docs/graph-theatre-foaf-framed.png` |
| `??` | `docs/graph-theatre-foaf.png` |
| `??` | `docs/graph-theatre-star.png` |
| `??` | `docs/graph-theatre-webgl2.png` |
| `??` | `docs/manuals/standards/references/` |
| `??` | `tools/qbdl-stream-conformance/fixtures/generate-cose-encoding-v1.mjs` |
| `??` | `tools/qbdl-stream-conformance/fixtures/generate-cose-openssl-v1.mjs` |
| `??` | `tools/qbdl-stream-conformance/fixtures/generate-endpoint-v1.mjs` |
| `??` | `tools/qbdl-stream-conformance/fixtures/generate-protected-v1.mjs` |
| `??` | `tools/qbdl-stream-conformance/fixtures/generate-qasd-plain-v1.mjs` |
| `??` | `tools/qbdl-stream-conformance/fixtures/generate-service-wire-v1.mjs` |
| `??` | `tools/qbdl-stream-conformance/fixtures/generate-storage-v1.mjs` |
| `??` | `tools/qbdl-stream-conformance/fixtures/generate-tree-v1.mjs` |
| `??` | `vendor/etalon-model-6ace6cc/.gitignore` |
| `??` | `vendor/etalon-model-6ace6cc/tests/support/p64_stream_golden/` |
| `??` | `vendor/etalon-model-6ace6cc/tests/support/p64_writer_golden/` |

No dirty files under `crates/qualia-core-db/src/` in this snapshot.

## 3. Authority owner-identity gap (Wave S1 files)

| Path | Exists | `CanonicalIncarnation` used |
| --- | --- | --- |
| `crates/qualia-core-db/src/net/qdnf/authority/owner.rs` | yes | **no** |
| `crates/qualia-core-db/src/net/qdnf/authority/permit.rs` | yes | **no** |
| `crates/qualia-core-db/src/net/qdnf/authority/current_guard.rs` | yes | **no** |
| `crates/qualia-core-db/src/net/qdnf/authority/current_guard/tests.rs` | yes | **no** |
| `crates/qualia-core-db/src/net/qdnf/authority/owner_tests.rs` | yes | **no** |
| `crates/qualia-core-db/src/net/peer/runtime/incarnation.rs` | yes | **defines** `CanonicalIncarnation(usize)` |

Ripgrep `CanonicalIncarnation` under `net/qdnf/authority/`: **no matches**.

Type lives in `incarnation.rs` (`UNASSIGNED`, `bind`, `fresh`, atomic `NEXT`). Present consumers (out of S1 write set): `net/qdnf/session/handshake/endpoint/resource.rs` and `.../resource/retained.rs`.

Optional new file **absent**: `authority/identity.rs` and `authority/owner/identity.rs` do not exist. `owner/` currently has only `retirement.rs`, `retirement_tests.rs`.

`CurrentAuthorityGuard::check_affinity` — **`core::ptr::eq` only**:

```rust
pub(crate) fn check_affinity(&self, owner: &Mutex<AuthorityOwner>) -> Result<(), E> {
    if !core::ptr::eq(self.owner, owner) {
        return Err(E::Unauthorized);
    }
    Ok(())
}
```

Callers: `check_handle`, `revoke_handle`. Tests assert mutex affinity even when records collide (`exact_actual_owner_affinity_even_when_records_collide`). Gap: affinity is pointer identity of the `Mutex<AuthorityOwner>`, not `CanonicalIncarnation`.

## 4. Proposed exclusive claims for Wave S1

Do not edit these files in this support pass.

**Allowed writes**

- `crates/qualia-core-db/src/net/qdnf/authority/owner.rs`
- `crates/qualia-core-db/src/net/qdnf/authority/permit.rs`
- `crates/qualia-core-db/src/net/qdnf/authority/current_guard.rs`
- `crates/qualia-core-db/src/net/qdnf/authority/current_guard/tests.rs`
- `crates/qualia-core-db/src/net/qdnf/authority/owner_tests.rs`
- optional new `crates/qualia-core-db/src/net/qdnf/authority/owner/identity.rs`
- `authority/mod.rs` only if a new `identity` submodule must be routed

**Forbidden**

- `bundle/stream/policy/` (including any future `remote/`)
- `bundle/stream/storage/`
- `net/peer/replication/qbdl/wire/`
- `Cargo.toml` / workspace Cargo
- `task-registry.json`
- parent `mod.rs` other than `authority/mod.rs` if routing is needed
- `incarnation.rs` unless integrator reassigns (type already exists; S1 should import, not relocate)

**Integrator-owned**

- Cargo / lockfile / feature flags
- `task-registry.json`
- shared `mod.rs` outside `authority/`
- serial `cargo check` slot (`evidence/swarm-s0-core-check.log`)

Wait for the live check to finish before starting S1 writes.

## 5. Directory inventory (names only)

### `crates/qualia-core-db/src/bundle/stream/`

Top: `descriptor/`, `formats/`, `index/`, `mod.rs`, `policy/`, `proof/`, `protection/`, `publish/`, `runtime/`, `selection/`, `storage/`.

- `descriptor/`: cbor, digest, inventory, mod, pages, refs, roots, tests, types
- `formats/`: brick_tests, emf, emf_v3, emf_v3_tests, geometry, geometry_tests, m64, m64_tests, mod, q42_range_tests, q42_ranges, range, ten_d, ten_d_tests, tests
- `index/`: admit, cbor, error, header, limits, mod, owned, probe, tests
- `proof/`: builder, codec, hash, mod, path, path_tests, span, types, verify
- `protection/`: coverage, error, finish, geometry, ids, metadata, mod, publisher, publisher_boundary_tests, publisher_tests, seal, source, source_tests
- `publish/`: context, entries, finish, mod, owner, tests, tests/{independent,rejection,roundtrip}
- `runtime/`: dispatch, mod, owner, owner/{close,counted}, plan, recovery_source/, sparse_workspace/, state/, storage_stage.rs, storage_stage/{controlled,recovery,sparse}, tests, tests/{capacity,lifecycle,storage_controlled,storage_stage}, transfer, types
- `selection/`: entry, mod, tests
- `storage/`: bitmap, bitmap/owned, checkpoint, error, FORMAT.md, identity, leaf, mod, native/, tests, tests/{checkpoint,identity,leaf_bitmap}, wire

### `crates/qualia-core-db/src/net/peer/replication/qbdl/`

Top: `carrier/`, `mod.rs`, `provider/`, `wire/`.

- `carrier/`: mod, receive, send, tests
- `provider/`: mod, outboard, outboard_tests
- `wire/`: limits, mod, prefix, record, selector, tests, variants

### `crates/qualia-core-db/src/bundle/stream/policy/`

Present: `authenticated.rs`, `grant.rs`, `grant/scoped.rs`, `manifest.rs`, `manifest/transport.rs`, `mod.rs`, `rejection_tests.rs`, `scoped_tests.rs`, `tests.rs`, `types.rs`.

**`policy/remote/` does not exist** (`Test-Path` = False). Empty/absent `remote/` is a valid finding.

## 6. Leftover compile note

Processes queried via `Get-CimInstance Win32_Process`. **Observed live cargo/rustc CommandLine; not killed.**

| PID | Name | CommandLine (truncated) |
| --- | --- | --- |
| 7408 | powershell.exe | `cargo check -p qualia-core-db --tests --locked --offline -j 1` tee `evidence/swarm-s0-core-check.log` |
| 39776 | cargo.exe | `C:\Users\Hp\.cargo\bin\cargo.exe check -p qualia-core-db --tests --locked --offline -j 1` |
| 16152 | cargo.exe | rustup `stable-x86_64-pc-windows-msvc` same `cargo check -p qualia-core-db --tests --locked --offline -j 1` |
| 45400 | rustc.exe | `--crate-name qualia_core_db --edition=2021 crates\qualia-core-db\src\lib.rs ... --check-cfg cfg(feature, values(...))` |

This is the integrator Wave S0 serial check. Support did not start cargo and did not interrupt it.
