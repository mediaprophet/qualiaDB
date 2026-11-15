# Swarm Wave 3 claim board

Integrator-owned. Claimed 2026-11-15. File-disjoint. **Do not cargo.** EMF/QAC owner may still be editing; children never write owner files and never run `qualia-core-db --lib`.

Product: selective, governed streaming of a heterogeneous QBDL-profile HMC over QDNF. An HMC may contain **many entries of the same family**.

| Child | Tier | Status | Allowed writes | Forbidden |
|---|---|---|---|---|
| **M** mux loop | Expert | packet ready, not launched | NEW `crates/qualia-core-db/src/net/peer/replication/qbdl/service/mux.rs`; NEW `mux_tests.rs`; `evidence/swarm-wave3-m-summary.md` | `provider.rs`, `client.rs`, `describe.rs`, `session.rs`, `protected.rs`, `two_process.rs`, `service/mod.rs`, `service/tests.rs`, `plan/**`, `http/**`, `mixed/**`, cargo |
| **F** mixed HMC | Senior | packet ready, not launched | NEW `crates/qualia-core-db/src/bundle/stream/mixed/mod.rs`; NEW `bundle.rs`; NEW `tests.rs`; `evidence/swarm-wave3-f-summary.md` | `plan/**` (existing), `formats/**`, `http/**`, `qbdl/service/**`, `stream/mod.rs`, codec owner files, cargo |
| **H** HTTP mixed | Senior | packet ready, not launched | NEW `crates/qualia-core-db/src/bundle/stream/http/mixed.rs`; `evidence/swarm-wave3-h-summary.md` | `http/{mod,range,status,client,provider,tests}.rs`, `wss*`, `cscp_h2.rs`, `registries.rs`, `qbdl/service/**`, cargo |
| Integrator | — | live | parent `mod.rs` routing after stop; this board; continuation/progress | EMF/QAC owner files; default `target/` cargo until user notify **and** 3A source complete; git commit/push |
| Reviews M/F/H | Expert read-only | queued 3B | `evidence/swarm-wave3-{m,f,h}-review.md` | production writes; cargo |
| C6 records | Support | queued 3B | `evidence/swarm-wave3-c6-records.md` | production crates; PhysicalNetwork/browser claims |

Keep existing Wave 1 C3/C4/C5 and Wave 2/2.5 source. Do not revert it.

**Cargo hold** until the user says EMF/QAC is paused **and** M/F/H have stopped. One heavy rustc into default `target/` at a time when that hold lifts. Concurrent EMF cargo stays on `target/qac-legacy-fix-20261113`.

No commit/push. Do not kill Synth MCP (`C:\github\synth\target-studio\debug\synth.exe`). Shared tree. ≤3 live children. Review occupies a slot, so 3B starts after 3A children stop.

## Off-limits (all children)

`audio/qac/**`, `q42/field_*`, `render/gpu/emf_pipeline.rs`, `portal_emf*`, `portal_qac*`, `medical/**`, `sensing/acquisition/**`, etalon-render EMF, workspace etalon `[patch]`, `Cargo.toml` / `Cargo.lock`, `net/qdnf/registries.rs`, parent `mod.rs` files, `emf_spectrum.rs`.
