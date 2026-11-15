# Swarm Wave 3 — EMF/QAC streaming supports + mux join

Integrator-owned. Packets revised 15 November 2026 to include **required EMF and QAC streaming supports** (R07), not fixture-only Opaque stand-ins. This is **not** kernel-swarm.rhai / `/execute-plan`. **Do not launch cargo** from these children.

Product: **selective, governed streaming of a heterogeneous QBDL-profile HMC over QDNF**. An HMC may contain **many entries of the same family**. Mux lives in the planner plus a service loop over `Schedule::next`. Inner EMF/QAC interpretation uses the codec owners’ public readers; streaming adapters must not copy those parsers and must not edit owner files.

Native interchange is CBOR / CBOR-LD / exact-length binary. JSON is exceptional ingest only.

## EMF and QAC supports this wave must deliver

These are product requirements, not a later track.

| Support | Owner packet | Contract |
|---|---|---|
| QAC complete-entry fallback | C3 (landed) + **Q** | Whole current `.qac` via `QacStorageView::open`. Partial range `Extent`. UA4Q refuses. |
| QAC scientific signal | **Q** | `admit_signal` → `audio::qac::signal::admit`. Unknown critical / unsupported profile → opaque archival, never playable audio. Signed-zero bytes preserved. |
| QAC declared interval | **Q** | Planner metadata only if the envelope already declares it. No invented clock. Transfer unit stays the whole entry until a partial-QAC contract exists. |
| EMF v2 directory / v3 interpretation | existing `emf.rs` / `emf_v3.rs` + **E** | Dispatch on header version. No silent v2 fallback. Region = `selected_chunk`, not a volume receipt. |
| EMF truncated plane / last digest | **E** | Truncated later plane refuses. Trailer byte corruption → Integrity. |
| Many QAC and many EMF in one HMC | **Q**, **E**, **F**, **M** | ≥2 of each family. `EntryFamily` is a per-entry tag. |
| Concurrent EMF region + QAC interval | **M** (uses E/Q) | Interleave Ready EMF and QAC; delay M64/P64; shared 10D/calib charged once. |
| `present[]` from admit | **E**, **Q** | Successful structural/scientific admit → planner present id. Incomplete/Refused/Opaque never look like a valid empty field. |
| HTTP same outcomes | **H** (3A2) | Component loopback of the same QAC/EMF entries. |

Streaming still does **not** edit `audio/qac/**`, `q42/field_*`, `q42/emf_container/**`, `render/gpu/emf_pipeline.rs`, portal EMF/QAC, etalon-render. Call public APIs. On-disk `qac_candidate-*.qac` / `emf_candidate-*.emf` are real writer output.

## Already in source (do not rewrite)

| Slice | Where | State |
|---|---|---|
| C1 production I/O | `ProtectedSession`, `transfer_one_leaf` | Cargo 17/17 in-process |
| C3 QAC complete-entry | `formats/qac.rs` `QacCompleteView::admit` | Source routed; **no** `admit_signal` yet |
| EMF v2/v3 directory | `formats/{emf,emf_v3}.rs` | Admit + `selected_chunk`; no stream profile / present helper |
| C4 planner | `bundle/stream/plan/` | Types include `Emf`/`Qac`; `Schedule::compile` only in `plan/tests.rs` |
| C5 HTTP | `bundle/stream/http/` | Single-entry adapter |

## Waves (≤3 live children)

### Wave 3A — EMF/QAC supports + mux (NEW/edit as claimed, no cargo)

| Child | Tier | Writes | Role |
|---|---|---|---|
| **E** EMF stream | Senior | NEW `formats/emf_stream.rs` + tests | Version matrix, region, present, truncated/digest, many EMF |
| **Q** QAC stream | Senior | EDIT `formats/qac.rs` (`admit_signal`); NEW `qac_stream.rs` + tests | Signal, interval, archival vs scientific, many QAC |
| **M** mux loop | Expert | NEW `qbdl/service/mux.rs` + `mux_tests.rs` | `Schedule::next` + `transfer_one_leaf` of **real** EMF and QAC entries |

M must `include_bytes!` canonical `.qac`/`.emf` payloads, `admit` them, and transfer **two QAC + two EMF** in one plan. M may call existing `QacCompleteView` / `EmfDirectoryView` if E/Q are not yet routed; it must not tag blobs as `Qac`/`Emf`.

Integrator after 3A stop:

```rust
// formats/mod.rs
mod emf_stream;
pub use emf_stream::{EmfProfile, EmfStreamView};
mod qac_stream;
pub use qac_stream::{QacProfile, QacStreamView};
// qac.rs already routed; Q’s admit_signal lands in place

// qbdl/service/mod.rs
mod mux;
pub use mux::{transfer_scheduled, MuxReport, MuxTransfer};
```

### Wave 3A2 — mixed HMC binary + HTTP (after 3A authors stop, still no cargo)

| Child | Tier | Writes |
|---|---|---|
| **F** mixed HMC | Senior | NEW `bundle/stream/mixed/{mod,bundle,tests}.rs` |
| **H** HTTP mixed | Senior | NEW `http/mixed.rs` |

F publishes one QBDL-profile HMC with ≥2 TenD, ≥2 EMF, ≥2 QAC, plus Q42, M64, Opaque, using E/Q admit + `present` ids and geometry/calibration edges. H serves those QAC/EMF objects over HTTP (`EvidenceClass::Component`).

One slot free in 3A2 (review occupies it if started early — do not).

### Wave 3B — cargo, then reviews + C6

EMF/QAC codecs are **done** (QualiaDB `91a4c3746`). No codec deferral. Integrator cargo after 3A+3A2 source is routed:

`cargo test -p qualia-core-db --lib --locked --offline -j 1`

Then read-only: reviews of E/Q/M/F/H and C6 Component/Process records.

## Shared rules (every child)

- Exclusive file claims. No two writers on one file.
- Integrator owns parent `mod.rs`, Cargo.toml/lock, `registries.rs`, default `target/` cargo slot, git.
- Children do not cargo. One heavy rustc into default `target/` at a time when the hold lifts. Concurrent codec cargo stays on `target/qac-legacy-fix-20261113`.
- Off-limits: `audio/qac/**`, `q42/field_*`, `q42/emf_container/**`, `render/gpu/emf_pipeline.rs`, `portal_emf*`, `portal_qac*`, `medical/**`, `sensing/acquisition/**`, etalon-render EMF, etalon `[patch]`, `emf_spectrum.rs`.
- Do not kill Synth MCP (`C:\github\synth\target-studio\debug\synth.exe`).
- No commit / push / deploy until user accept.
- Shared tree (`.grok` worktrees break `../etalon` patch).
- Kind 1–12 frozen. No `BearerProfile` 8+.
- WouldBlock drops `CurrentAuthorityGuard` / ledger / `RuntimeAccess`.
- ZIP `.hmc` is a different profile. Do not sniff by extension.
- EMF is not sRGB. QAC is not “audible playback” unless `admit_signal` succeeded.

## Out of this wave

PhysicalNetwork, browser/WASM, ZIP HMC, partial legacy QAC views, `DescriptorPage` buffer lift, full G01–G14 release qualification, portal/etalon-render EMF display.

## Briefs

- [swarm-wave3-e-emf-brief.md](swarm-wave3-e-emf-brief.md)
- [swarm-wave3-q-qac-brief.md](swarm-wave3-q-qac-brief.md)
- [swarm-wave3-m-mux-brief.md](swarm-wave3-m-mux-brief.md)
- [swarm-wave3-f-mixed-brief.md](swarm-wave3-f-mixed-brief.md)
- [swarm-wave3-h-http-brief.md](swarm-wave3-h-http-brief.md)
- [swarm-wave3-c6-brief.md](swarm-wave3-c6-brief.md) (queued 3B)
- [swarm-wave3-review-brief.md](swarm-wave3-review-brief.md) (queued 3B)
- [swarm-wave3-integrator.md](swarm-wave3-integrator.md)
- Claims: [swarm-wave3-claim-board.md](swarm-wave3-claim-board.md)
