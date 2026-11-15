# Swarm Wave 3 — mux join + mixed HMC + HTTP parity

Integrator-owned. Packets prepared 15 November 2026. This is **not** kernel-swarm.rhai / `/execute-plan`. **Do not launch cargo** from these children.

Product: **selective, governed streaming of a heterogeneous QBDL-profile HMC over QDNF**. An HMC may contain **many entries of the same family**. `EntryFamily` is a per-entry tag, not a unique slot. Review §6 “one of each” is the **minimum first mixed-bundle fixture**, not a container invariant.

Mux lives in the transfer/session planner (`bundle/stream/plan`) plus a service loop that drives `Schedule::next`. Do not rewrite the container. Do not mint HLS/DASH.

Native interchange is CBOR / CBOR-LD / exact-length binary. JSON is exceptional ingest only.

## Already in source (do not rewrite)

| Slice | Where | State |
|---|---|---|
| C1 production I/O | `ProtectedSession`, `ServiceIo`, `transfer_one_leaf` | Cargo 17/17 in-process. Wave 2.5 two-process + Describe 8/9 in source, untested |
| C2 write_leaf | `prepare_leaf_write` / `wait_leaf_write` / `commit_leaf_write` | Production path + crash tests in source |
| C3 QAC admit | `formats/qac.rs` `QacCompleteView::admit` | Source routed, untested |
| C4 planner | `bundle/stream/plan/{inventory,readiness,schedule}` | Types + tests, including many-same-family. `Schedule::compile` used only in `plan/tests.rs` |
| C5 HTTP | `bundle/stream/http/` | Adapter + single-entry loopback tests, untested |
| Describe 8/9 | `qbdl/service/describe.rs` `answer_describe_pages` | Source. Client can Select/Open from the wire |

The remaining **product code** is joining those pieces: a service loop over `Schedule::next`, a real multi-entry QBDL-profile HMC, and HTTP tests of the same plan outcomes.

## Waves

### Wave 3A — now, NEW files only, no cargo, ≤3 live children

| Child | Tier | Writes | Depends |
|---|---|---|---|
| **M** mux loop | Expert | NEW `qbdl/service/mux.rs`, `mux_tests.rs`; evidence `swarm-wave3-m-summary.md` | C4 types, `transfer_one_leaf`, Describe 8/9 (read-only) |
| **F** mixed HMC | Senior | NEW `bundle/stream/mixed/{mod,bundle,tests}.rs`; evidence `swarm-wave3-f-summary.md` | `PlainPublisher`, format `admit` views, on-disk canonical fixtures |
| **H** HTTP mixed | Senior | NEW `bundle/stream/http/mixed.rs`; evidence `swarm-wave3-h-summary.md` | C5 `HttpProvider`, C4 `Schedule` (read-only) |

Integrator routes **after authors stop**:

```rust
// qbdl/service/mod.rs
mod mux;
pub use mux::{transfer_scheduled, MuxReport, MuxTransfer};

// bundle/stream/mod.rs
pub mod mixed;

// bundle/stream/http/mod.rs
#[cfg(all(test, not(target_arch = "wasm32")))]
mod mixed;
```

Children **do not** edit those parent files. Children **do not cargo**.

### Wave 3B — after 3A source is routed **and** the user says EMF/QAC is paused

Integrator only: one locked `cargo test -p qualia-core-db --lib --locked --offline -j 1` into default `target/`. UTF-8 log under `evidence/`. LNK1104: retry once, keep the log.

Then, still ≤3 live, **read-only / docs**:

| Child | Writes |
|---|---|
| Independent reviews of M, F, H | `evidence/swarm-wave3-{m,f,h}-review.md` |
| C6 records | `evidence/swarm-wave3-c6-records.md` |

Briefs for 3B: [swarm-wave3-review-brief.md](swarm-wave3-review-brief.md), [swarm-wave3-c6-brief.md](swarm-wave3-c6-brief.md). **Do not launch 3B until cargo is green.**

## Shared rules (every child)

- Exclusive file claims. No two writers on one file.
- Integrator owns parent `mod.rs`, Cargo.toml/lock, `registries.rs`, default `target/` cargo slot, git.
- One heavy `qualia-core-db` rustc into default `target/` at a time. Children do not cargo. Concurrent EMF cargo stays on `target/qac-legacy-fix-20261113`.
- Do not edit EMF/QAC owner files: `audio/qac/**`, `q42/field_*`, `render/gpu/emf_pipeline.rs`, `portal_emf*`, `portal_qac*`, `medical/**`, `sensing/acquisition/**`, etalon-render EMF, workspace etalon `[patch]`.
- Do not kill Synth MCP (`C:\github\synth\target-studio\debug\synth.exe`).
- No commit / push / deploy until user accept.
- Shared tree (`.grok` worktrees break `../etalon` Cargo patch).
- Kind 1–12 frozen. No `BearerProfile` 8+.
- WouldBlock sleeps drop `CurrentAuthorityGuard` / ledger / `RuntimeAccess`.
- Do not derive `Debug` on success types just for `unwrap_err`. Use `match`.
- `accept_local_scoped` closures that return service `Error` wrap: `Ok((|| -> Result<(), Error> { ... })())`.
- ZIP `.hmc` is a different profile. Do not sniff by extension.

## Out of this wave

PhysicalNetwork, browser/WASM, ZIP HMC, session `DescriptorPage` buffer lift to `MAX_RECORD_BYTES`, `fixtures::write` still holding a stage, `CryptographicLibrary::get_key` vis, `Intent::decode`, full G01–G14 release qualification.

## Briefs

- [swarm-wave3-m-mux-brief.md](swarm-wave3-m-mux-brief.md)
- [swarm-wave3-f-mixed-brief.md](swarm-wave3-f-mixed-brief.md)
- [swarm-wave3-h-http-brief.md](swarm-wave3-h-http-brief.md)
- [swarm-wave3-c6-brief.md](swarm-wave3-c6-brief.md) (queued 3B)
- [swarm-wave3-review-brief.md](swarm-wave3-review-brief.md) (queued 3B)
- Claims: [swarm-wave3-claim-board.md](swarm-wave3-claim-board.md)
