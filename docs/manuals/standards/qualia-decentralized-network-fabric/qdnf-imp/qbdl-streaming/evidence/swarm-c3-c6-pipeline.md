# Swarm C3–C6 — remaining HMC mux implementation

Integrator-owned. Launched 14 November 2026 after C1 in-process `ProtectedSession` went green (17/17). This is **not** a kernel-swarm.rhai / `/execute-plan` run.

Product: **selective, governed streaming of a heterogeneous QBDL-profile HMC over QDNF**. Mux lives in the transfer/session planner. M64 is one inner format. ZIP `.hmc` is a different profile.

Native interchange is CBOR / CBOR-LD / exact-length binary. JSON is exceptional ingest only.

## Honest remainder (C1 is not the product)

| Slice | In source now | Still open |
|---|---|---|
| C0 | Focused selectors green (M64 5, wrap 4, R2 22, WSS 11, jobs 3) | Keep LNK1104 logs distinct |
| C1 R01–R04 | `ServiceIo` + `ProtectedSession` + conserved `issue_credit` + recheck + admitted deadline. In-process handshake tests 17/17 | Two-process still framing `Session`. No Describe/index bootstrap. Wrap still `LocalWholeSourceGrant`. R01 acceptance (fresh process handshake, restricted transfer not reconstructed by fixture) **open** |
| C2 R05 | Production `write_leaf` drops stage before OS I/O. Review 0 | Crash wrapper still takes `&mut StorageStage`; `ArmedNativeWrite` remains a scalar wait |
| C3 R07 | Inventory only | No `formats/qac.rs` |
| C4 R06/R08 | None | No authenticated multi-entry inventory, readiness, or deterministic schedule |
| C5 | WSS framing green; HTTP inventory written | No `bundle/stream/http/` |
| C6 | None | Mixed-HMC adversarial suite + evidence-class records |

Do not mark QBS-09 complete from the process fixture.

## Waves (≤3 live children; review occupies a slot)

### Wave 1 — now, NEW files only, no cargo

| Child | Tier | Writes | Depends |
|---|---|---|---|
| C3 QAC adapter | Senior | NEW `formats/qac.rs`, `formats/qac_tests.rs`; evidence summary | C1 in source (done) |
| C4 mux planner | Expert | NEW `bundle/stream/plan/{mod,inventory,readiness,schedule,tests}.rs`; evidence summary | None (types only; does not call QAC admit) |
| C5 HTTP/LIG | Senior | NEW `bundle/stream/http/{mod,range,status,client,provider,tests}.rs`; evidence summary | C1 + WSS (done) |

Integrator routes `formats/mod.rs` and `bundle/stream/mod.rs` **after authors stop**. One cargo then. Children **do not cargo**.

### Wave 2 — after Wave 1 route + one cargo

| Child | Tier | Writes |
|---|---|---|
| C1-D Describe bootstrap | Expert | NEW `qbdl/service/describe.rs`; tiny provider/client hooks via integrator |
| C1-P protected two-process | Expert | EDIT `qbdl/service/two_process.rs` only to attach `ProtectedSession` over the named-pipe Bearer |
| C1-W remote wrap binding | Expert | EDIT wrap path to carry recognized source-controller enrollment; refuse a locally trusted pin with wrong source rights |
| Independent reviews | Expert read-only | evidence `*-review.md` for C3, C4, C5 |

C1-D / C1-P / C1-W are **serial among themselves** if they would share `service/mod.rs`. Prefer NEW describe.rs + two_process.rs-only + wrap.rs so they stay file-disjoint.

### Wave 3 — mixed scenario + qualification (packets 2026-11-15)

See [swarm-wave3-pipeline.md](swarm-wave3-pipeline.md) and [swarm-wave3-claim-board.md](swarm-wave3-claim-board.md).

Wave 3A (≤3 children): **E** EMF stream, **Q** QAC `admit_signal` + stream profile, **M** mux of two QAC + two EMF. Wave 3A2: **F** mixed HMC, **H** HTTP mixed. C6 and reviews are Wave 3B after cargo. Browser/WebRTC remain later extensions. See [swarm-wave3-pipeline.md](swarm-wave3-pipeline.md).

## Shared rules

- Exclusive file claims. No two writers on one file.
- Integrator owns parent `mod.rs`, Cargo.toml/lock, `registries.rs`, cargo slot, git.
- One heavy `qualia-core-db` rustc into default `target/` at a time (`--locked --offline -j 1`). Children do not cargo. Concurrent EMF cargo stays on `target/qac-legacy-fix-20261113`.
- Do not edit EMF/QAC owner files: `audio/qac/**`, `q42/field_*`, `render/gpu/emf_pipeline.rs`, `portal_emf*`, `portal_qac*`, `medical/**`, `sensing/acquisition/**`, `etalon-render` EMF, workspace etalon `[patch]`.
- Do not kill Synth MCP (`C:\github\synth\target-studio\debug\synth.exe`).
- No commit / push / deploy until user accept.
- `.grok` worktrees break `../etalon` Cargo patch; shared tree + exclusive claims.
- WouldBlock sleeps drop `CurrentAuthorityGuard` / ledger / `RuntimeAccess`.
- Do not mint `BearerProfile` 8+.
- Do not copy parsers into network code.

## Briefs

- [swarm-c3-qac-brief.md](swarm-c3-qac-brief.md)
- [swarm-c4-plan-brief.md](swarm-c4-plan-brief.md)
- [swarm-c5-http-brief.md](swarm-c5-http-brief.md)
- Claims: [swarm-c3-c6-claim-board.md](swarm-c3-c6-claim-board.md)
