# Continuation pointer — 13 November 2026

## Paused overnight (13 Nov ~22:10 AEDT)

User is powering down the laptop. Integrator stopped **our** C0 cargo/watch (`swarm-c0-retry-emf-idle.log` was in M64 compile; no `M64_EXIT` this run). Did not kill Synth MCP (`synth\target-studio\debug\synth.exe`) or other sessions (`asset-production-vibe`, EMF/QAC). No commit/push.

### Morning, in order

1. Read this file, [implementation-review-hmc-multiplexing-2026-11-13.md](implementation-review-hmc-multiplexing-2026-11-13.md), [progress-log.md](progress-log.md).
2. Confirm no `qualia-core-db` rustc on default `target/`. Leave any EMF/QAC cargo on `target/qac-legacy-fix-20261113` and portal-qac harness alone. **Do not edit** `render/gpu/emf_pipeline.rs`, `portal_emf_state`, `portal_load_emf`, etalon-render EMF, or `audio::qac`.
3. **C0** — `cargo test -p qualia-core-db --lib --locked --offline -j 1` sequential filters: `m64_v3` → `qbdl::service::` → `protection::wrap` → `lifecycle_tests` → `storage::native::job` → `storage::native::write` → `p2p::connectivity::wss`. UTF-8 log under `evidence/`. LNK1104: retry once, keep the log. One heavy rustc into default `target/` at a time.
4. After green: independent **R2 re-review** (`join_enrollment` + live `recognize` + two-lifetime `InstallRequest`). Then **route** unrouted `crates/qualia-core-db/src/net/peer/replication/qbdl/service/protected.rs` and finish C1 (R01 protected channel, R03 `RuntimeAccess::issue_credit`, R04 recheck-before-disclosure / admitted deadline). Plan: [evidence/swarm-c1-protected-channel-plan.md](evidence/swarm-c1-protected-channel-plan.md).
5. HTTP waits for C1. C3 QAC waits for the EMF/QAC codec owner ([evidence/swarm-c3-qac-adapter-inventory.md](evidence/swarm-c3-qac-adapter-inventory.md)). C4 mixed HMC, C5 adapters, C6 qualification.
6. Two-process fixture is **not** QBS-09 complete. No commit/push until user accept.

The dated [remaining-work-and-build-handoff.md](remaining-work-and-build-handoff.md) is **history**. Do not treat its “345 tests / authority still open / two-process missing” statements as current. This file plus [progress-log.md](progress-log.md) and [evidence/](evidence/) are the live tracker.

Scope correction (must keep): [implementation-review-hmc-multiplexing-2026-11-13.md](implementation-review-hmc-multiplexing-2026-11-13.md). The product is **selective, governed streaming of a heterogeneous QBDL-profile HMC over QDNF**. M64 is one inner format. ZIP `.hmc` is a different profile. Multiplexing lives in the transfer/session planner, not a silent container rewrite or HLS/DASH.

## Live status (integrator, this session)

| Slice | Review ID | State |
|---|---|---|
| C0 compile/tests | R09 | EMF/QAC owner confirmed C0 can compile and paused `qualia-core-db` edits. Post-patch `swarm-c0-2026-11-14-c.log`: M64 5/5, SERVICE 1+10, JOB 3/3, WRITE 0, WSS 11/11. Recargo wrap **4/4** and R2 `lifecycle_tests` **22/22** in `evidence/swarm-wrap-r2-2026-11-14.log`. |
| S1 owner identity | — | Closed. 37/37. Review `open_bugs: 0`. |
| S2 R1 remote schema | — | Closed. 11/11. Review `open_bugs: 0`. |
| S2 R2 issuer/grant | R09 | Source patched (`join_enrollment` + live `recognize` in recheck) plus unenrolled/rotate tests (`audience: b"reader-rotated"`). Two-lifetime `InstallRequest`. Cargo **22/22**. Independent re-review `swarm-s2-remote-r2-review-2.md`: **pass: true, open_bugs: 0**. |
| C2 write_leaf | R05 | Production path drops stage before OS I/O. Review `open_bugs: 0`. Cargo `job` 3/3, `write` 0 tests. **Wave 2.5-C2** splitting crash-wrapper so tests drop stage during wait. |
| S3 two-process | R01 | Framing fixture **2/2**. Review `open_bugs: 0`. **Wave 2.5-P** ProtectedSession handshake-over-pipe in source. Still not PhysicalNetwork. |
| S4 M64 v3 | R06 | Routed `M64GeoRasterDirectoryView`. Review `open_bugs: 0`. Adapter only. Cargo **5/5**. |
| S4 B2 wrap | — | `authorize_and_wrap` review `open_bugs: 0`. Happy-path unwrap clock is `SystemTime::now()`; recipient `expires=4_000_000_000`. Cargo **4/4**. Still `LocalWholeSourceGrant`; remote grant binding is Wave 2 C1-W. |
| S5 WSS | §4 audit | Framing + `seal_close` Conflict. Review-2 `open_bugs: 0`. Cargo **11/11**. HTTP is Wave 1 C5 (NEW `bundle/stream/http/`). |
| C1 R01–R04 | R01–R04 | In-process production I/O **17/17**. Wave 2 Describe class 1/7 + remote wrap in source. **Wave 2.5-D** class 8/9 body/entry_page. **Wave 2.5-P** protected two-process. |
| C3 QAC/EMF | R07 | Wave 1 **source landed**. `QacCompleteView::admit` → `QacStorageView::open`. Routed. Tests not run (**cargo hold**). |
| C4 mux plan | R06/R08 | Wave 1 **source landed**. Integrator added many-same-family planner tests (HMC may contain N of each family). Mixed-bundle **binary** is Wave 3 (needs real EMF/QAC writers). |
| C5 HTTP | QBS-14 | Wave 1 **source landed**. `bundle/stream/http` range/status/client/provider. Routed. |
| Wave 3 mux join | R06/R08 §6 | **Packets ready, not launched.** M mux loop, F mixed HMC, H HTTP mixed. Board: [evidence/swarm-wave3-claim-board.md](evidence/swarm-wave3-claim-board.md). Pipeline: [evidence/swarm-wave3-pipeline.md](evidence/swarm-wave3-pipeline.md). |

Swarm board: [evidence/swarm-wave3-claim-board.md](evidence/swarm-wave3-claim-board.md), [evidence/swarm-wave25-claim-board.md](evidence/swarm-wave25-claim-board.md), [evidence/swarm-c3-c6-claim-board.md](evidence/swarm-c3-c6-claim-board.md). Wave 1+2+2.5 source routed. Wave 3A packets written 2026-11-15 (M/F/H NEW files, no cargo). **Cargo hold** until user says EMF/QAC is paused **and** Wave 3A source is complete. Streaming does not edit EMF/QAC owner files. `emf_spectrum.rs` stays original `super::super::PortalGpu`.

## Corrective sequence (from HMC review §7)

1. **C0** — finish serialized compile; nonzero focused selectors; keep LNK1104 logs distinct.
2. **C1** — R01–R04: protected-channel production service, `associate(&request)` on RequestLeaves, conserved runtime credit, recheck-before-disclosure + admitted deadline. Strong independent review.
3. **C2** — R05: real write pipeline outside owner locks (current write_leaf child).
4. **C3** — QAC complete-entry fallback + versioned EMF/QAC admission; M64 stays bounded.
5. **C4** — mixed HMC orchestration (10d + q42 + emf + qac + m64 + optional opaque).
6. **C5** — HTTP/WSS parity, browser, Etalon, Commons.
7. **C6** — qualification / R1 release record.

Do not mark QBS-09 complete because the process fixture works. Evidence classes stay distinct: Windows process, Unix process, physical network, HTTP, WSS, browser.

No commit/push until user accept. Live Synth MCP is left running.
