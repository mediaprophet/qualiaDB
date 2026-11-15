# Swarm Wave 2.5 claim board

Integrator-owned. Claimed 2026-11-14. File-disjoint. **Do not cargo.** EMF/QAC owner is still editing; they will notify. Streaming does not run `qualia-core-db --lib` until that notify **and** this wave’s source is complete.

Product: selective, governed streaming of a heterogeneous QBDL-profile HMC over QDNF. An HMC may contain **many entries of the same family**. `EntryFamily` is a per-entry tag, not a unique slot. The 13 Nov review §6 “one of each” is a **minimum first mixed-bundle fixture**, not a container invariant.

| Child | Tier | Status | Allowed writes | Forbidden |
|---|---|---|---|---|
| D Describe class 8/9 | Expert | **source landed** (`01a1d8f8-c6de-70b1-82a5-61e7b17693fa`). Integrator wired `PlainProvider::answer_describe_pages` + `pub use`. | `describe.rs`; evidence `swarm-wave25-d-summary.md` | Kind 13+; JSON; cargo |
| P Protected two-process | Expert | **source landed** (`01a1d8f8-c6de-70b1-82a5-61fa6567b4ce`). Integrator mapped `begin_paired_signing` errors and used `Box::write` for `EndpointWorkspace`. | `two_process.rs`; evidence `swarm-wave25-p-summary.md` | JSON; PhysicalNetwork claim; cargo |
| C2 write split | Senior | **source landed** (`01a1d8f8-c6de-70b1-82a5-6203c998c28d`) | `write.rs` + crash tests; evidence `swarm-wave25-c2-summary.md` | `job.rs`, `tests.rs`, `fixtures.rs`, WriteStep **names**; cargo |
| Integrator | — | live | `plan/tests.rs` (many-same-family); `provider.rs` / `service/mod.rs` routing; this board | EMF/QAC owner files; default `target/` cargo; git commit/push |

Keep existing Wave 1 C3/C4/C5 source. Do not revert it.

**Cargo hold** until the user says EMF/QAC is done. One heavy rustc into default `target/` at a time when that hold lifts. Concurrent EMF cargo stays on `target/qac-legacy-fix-20261113`.

No commit/push. Do not kill Synth MCP (`C:\github\synth\target-studio\debug\synth.exe`). Shared tree (`.grok` worktrees break `../etalon` patch). ≤3 live children.

## Off-limits (all children)

`audio/qac/**`, `q42/field_*`, `render/gpu/emf_pipeline.rs`, `portal_emf*`, `portal_qac*`, `medical/**`, `sensing/acquisition/**`, etalon-render EMF, workspace etalon `[patch]`, `Cargo.toml` / `Cargo.lock`, `net/qdnf/registries.rs`, parent `mod.rs` files.

## Wave 3 (not this wave)

Mixed-HMC **binary** fixture using real EMF/QAC writers. Do not fabricate labeled blobs. Wait for user notify.
