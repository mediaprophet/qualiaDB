# Swarm Wave 3 claim board

Integrator-owned. Revised 2026-11-15 to include EMF/QAC streaming supports. File-disjoint. **Do not cargo.** Children never write codec-owner files and never run `qualia-core-db --lib`.

Product: selective, governed streaming of a heterogeneous QBDL-profile HMC over QDNF. An HMC may contain **many entries of the same family**. EMF region + QAC complete-entry (and signal when admitted) are required supports of this wave.

## Wave 3A (launch together, ≤3)

| Child | Tier | Status | Allowed writes | Forbidden |
|---|---|---|---|---|
| **E** EMF stream | Senior | **accepted** `01a1dcad-1a6a-7903-8185-28be788cb76e`. Integrator routed `mod emf_stream`. | NEW `formats/emf_stream.rs`; NEW `emf_stream_tests.rs`; `evidence/swarm-wave3-e-summary.md` | `emf.rs`, `emf_v3.rs`, `q42/emf_container/**`, `q42/field_*`, portal/pipeline EMF, cargo |
| **Q** QAC stream | Senior | **accepted** `01a1dcad-1a6b-7052-a1a0-2bb862a0b481`. Integrator routed `mod qac_stream`. | EDIT `formats/qac.rs` (`admit_signal` only); NEW `qac_stream.rs`; NEW `qac_stream_tests.rs`; `evidence/swarm-wave3-q-summary.md` | `qac_tests.rs`, `audio/qac/**`, `q42/field_*`, cargo |
| **M** mux loop | Expert | **accepted** `01a1dcad-1a6c-79c3-86ac-42d85f7c2bbc`. Integrator routed `mod mux`. | NEW `qbdl/service/mux.rs`; NEW `mux_tests.rs`; `evidence/swarm-wave3-m-summary.md` | `provider.rs`, `client.rs`, `describe.rs`, `session.rs`, `protected.rs`, `two_process.rs`, `service/mod.rs`, `service/tests.rs`, `formats/**`, `http/**`, `mixed/**`, cargo |

## Wave 3A2 (after 3A stop, still no cargo)

| Child | Tier | Status | Allowed writes | Forbidden |
|---|---|---|---|---|
| **F** mixed HMC | Senior | **accepted** `01a1dcc0-6cbd-7813-85ec-16855662946e`. Integrator routed `pub mod mixed`. | NEW `bundle/stream/mixed/{mod,bundle,tests}.rs`; `evidence/swarm-wave3-f-summary.md` | `plan/**` (existing), `formats/**`, `http/**`, `qbdl/service/**`, `stream/mod.rs`, codec owners, cargo |
| **H** HTTP mixed | Senior | **accepted** `01a1dcce-20ae-71b3-8596-5fd7c8105501` (resume of `01a1dcc0-6cbd-7813-85ec-16926213d86c`). Integrator routed `mod mixed`. Q42 HTTP Range via `Q42AuthenticatedRangeSource`. | NEW `http/mixed.rs`; `evidence/swarm-wave3-h-summary.md` | `http/{mod,range,status,client,provider,tests}.rs`, `wss*`, `cscp_h2.rs`, `registries.rs`, `qbdl/service/**`, cargo |

## After cargo (3B)

| Child | Status | Allowed writes |
|---|---|---|
| Reviews E/Q/M/F/H | queued | `evidence/swarm-wave3-{e,q,m,f,h}-review.md` |
| C6 records | queued | `evidence/swarm-wave3-c6-records.md` |

Integrator: parent `mod.rs` routing after each wave stop; this board; continuation/progress. EMF/QAC codecs done (`91a4c3746`) — no codec deferral. One locked cargo after 3A+3A2 source is routed. No commit/push. H also serves **Q42 via HTTP Range** (`Q42AuthenticatedRangeSource`, not `HttpRangeSource`).

Keep Wave 1 C3/C4/C5 and Wave 2/2.5 source. Do not revert it.

## Remaining source (2026-11-15, landed — cargo next)

| Child | Status | Writes |
|---|---|---|
| C2 stage-drop | **accepted** `01a1dce0-0932-7331-b9de-9a8e71cd9dbc` then integrator follow-on | `fixtures.rs` split; production `write_leaf` now runtime+ticket short stages; recover copy-out; resume session I/O outside stage. Source `read_exact_at` still a recovery-buffer loan for the copy only. |
| EMF v2/v3 + Q interval | **accepted** `01a1dce0-0933-7143-9c87-d7d4e7786d6d` | `formats/range.rs` `AuthenticatedRange::subrange`; `emf_stream.rs` inner `EnvelopeV4\|DirectoryV2\|InterpretationV3`; QAC `declared_interval` from `flags()&2` |
| WSS mixed | **accepted** `01a1dce0-0933-7143-9c87-d7e5c05b18f7`. Integrator routed `mod wss_mixed;` | NEW `http/wss_mixed.rs` (Component, native mux over `WssBearer`) |
| Intent::decode | integrator | `handshake/endpoint/intent.rs`; `two_process.rs` `decode_intent` now calls it |
| P64 complete-entry | integrator | NEW `formats/p64_stream.rs`; routed `mod p64_stream`; default mixed corpus includes `MIXED_ID_P64` |
| MixedHmc → MuxTransfer | integrator | NEW `mixed/transfer.rs` (`compile_from_mixed`, `transfer_mixed`) |

Honest leftovers (do not fake): recovery source `read_exact_at` still uses a `RangeRecoveryRead` loan for the copy-out only; `resume_hint_for_leaf` still takes an outer `StorageStage`; PhysicalNetwork two-host; browser/WASM consumer; C6 records and independent reviews after cargo.

ZIP `.hmc` is the legacy document-archive profile. The platform is unreleased; do not add ZIP streaming support. QBDL admission already dispatches on magic `QBDL` and refuses PK. Do not sniff `.hmc`.

No kill Synth MCP (`C:\github\synth\target-studio\debug\synth.exe`). Shared tree.

## Off-limits (all children)

`audio/qac/**`, `q42/field_*`, `q42/emf_container/**`, `render/gpu/emf_pipeline.rs`, `portal_emf*`, `portal_qac*`, `medical/**`, `sensing/acquisition/**`, etalon-render EMF, workspace etalon `[patch]`, `Cargo.toml` / `Cargo.lock`, `net/qdnf/registries.rs`, parent `mod.rs` files, `emf_spectrum.rs`.
