# Swarm C3–C6 claim board

Integrator-owned. Wave 1 claimed 2026-11-14.

| Child | Tier | Status | Allowed writes | Forbidden |
|---|---|---|---|---|
| C3 QAC | Senior | **source landed** (`01a1d880-6d53-7db0-85bc-8069cd5b170a`). Integrator routed `formats/mod.rs`. Tests not run | `formats/qac.rs`, `formats/qac_tests.rs`; `evidence/swarm-c3-qac-summary.md` | `audio/qac/**`, `q42/field_*`, cargo until Wave 1 complete |
| C4 mux plan | Expert | **source landed** (`01a1d880-6d53-7db0-85bc-807bd3ba5ce5`). Integrator routed `pub mod plan`. Tests not run | `bundle/stream/plan/**`; `evidence/swarm-c4-plan-summary.md` | `formats/**`, `http/**`, `qbdl/service/**` |
| C5 HTTP | Senior | **source landed** (`01a1d880-6d53-7db0-85bc-80868a2e036a`). Integrator routed `mod http`. Tests not run | `bundle/stream/http/**`; `evidence/swarm-c5-http-summary.md` | `wss*`, `cscp_h2.rs`, `registries.rs` |
| C1-D Describe | Expert | Wave 2 class 1/7 + Wave 2.5 class 8/9 **source landed**. Integrator wired `PlainProvider::answer_describe_pages` | `describe.rs` | Kind 13+ |
| C1-P protected two-process | Expert | Wave 2.5 **source landed** (`QBDL-PSK1` + ProtectedSession over pipe) | `two_process.rs` | PhysicalNetwork claim |
| C1-W remote wrap | Expert | Wave 2 source landed (`authorize_and_wrap_remote`) | wrap path under `bundle/stream/protection/` | `policy/remote/**` unless integrator grants |
| C2 residual | Senior | Wave 2.5 **source landed** (crash tests drop stage during wait) | `write.rs` + crash tests | WriteStep names frozen |
| C6 qual | Support/senior | queued Wave 3B — brief [swarm-wave3-c6-brief.md](swarm-wave3-c6-brief.md) | evidence records only | production crates |
| Reviews | Expert read-only | after Wave 1 source | evidence `swarm-c3-qac-review.md`, `swarm-c4-plan-review.md`, `swarm-c5-http-review.md` | production writes |

Wave 3A (packets ready, not launched): **E** `emf_stream.rs`, **Q** `qac.rs` `admit_signal` + `qac_stream.rs`, **M** mux of real QAC+EMF. 3A2: F mixed, H HTTP. [swarm-wave3-claim-board.md](swarm-wave3-claim-board.md).

Integrator retains: `bundle/stream/mod.rs`, `formats/mod.rs`, `qbdl/mod.rs`, `qbdl/service/mod.rs`, `http/mod.rs`, Cargo.toml / Cargo.lock, `net/qdnf/registries.rs`, default `target/` cargo slot, git.

**Cargo hold:** EMF/QAC owner is still editing. Do **not** run `qualia-core-db` lib tests on default `target/` until they pause. Wave 1 source is routed; first compile hit E0433 in `render/gpu/emf_spectrum.rs` (`PortalGpu`). Streaming reverted that file. EMF cargo stays on `target/qac-legacy-fix-20261113`.

No commit/push. Do not kill Synth MCP.
