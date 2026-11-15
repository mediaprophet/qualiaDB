# Remaining gaps folded into work (Wave 2.5+)

Integrator inventory, 2026-11-14. Cargo is held until EMF/QAC notify **and** source for the items below that do not need those codecs is complete.

## Closed in Wave 2.5 source (untested)

| Gap | Owner | Notes |
|---|---|---|
| Describe class 8/9 so Select/Open bootstraps from the wire | D + integrator | `answer_describe_pages` + `PlainProvider::answer_describe_pages` |
| ProtectedSession two-process handshake | P | Binary `QBDL-PSK1`; EvidenceClass::Process |
| Crash tests hold StorageStage across I/O | C2 | prepare / wait_leaf_write / commit / finish |
| Many entries per family in the planner | Integrator | `plan/tests.rs` |

## Wave 3 packets (2026-11-15, source not started)

| Packet | Writes | Launch |
|---|---|---|
| M mux | NEW `qbdl/service/mux.rs` + `mux_tests.rs` | 3A, file-disjoint |
| F mixed HMC | NEW `bundle/stream/mixed/` | 3A; `include_bytes!` of on-disk candidate `.qac`/`.emf` + `compile_mesh_to_10d`; do not edit codec owners |
| H HTTP mixed | NEW `http/mixed.rs` | 3A, Component |
| Reviews M/F/H | `evidence/swarm-wave3-*-review.md` | 3B after cargo |
| C6 records | `evidence/swarm-wave3-c6-records.md` | 3B after cargo |

Pipeline: [swarm-wave3-pipeline.md](swarm-wave3-pipeline.md). Board: [swarm-wave3-claim-board.md](swarm-wave3-claim-board.md).

## Waiting on EMF/QAC notify (do not fake)

| Gap | Why wait |
|---|---|
| Recargo `qualia-core-db --lib` | Lib tests typecheck `emf_pipeline.rs` / `audio::qac` |
| C3/C4/C5/Wave 2/3 test execution | Same compile slot |
| Versioned EMF streaming adapter beyond current formats/emf | Codec owner |

## After green cargo (Wave 3B)

| Gap | Notes |
|---|---|
| Independent reviews of C3/C4/C5/Wave 2.5 and Wave 3 M/F/H | Read-only evidence `*-review.md`; Wave 3: [swarm-wave3-review-brief.md](swarm-wave3-review-brief.md) |
| C6 evidence-class records | [swarm-wave3-c6-brief.md](swarm-wave3-c6-brief.md) — Component and Process only this close-out |
| WSS mixed-bundle parity | Adapter framing green; mixed WSS test not in 3A |
| Browser / Etalon / Commons consumers | Later extensions |
| QBS-09 PhysicalNetwork | Distinct from Process fixture |
| ZIP `.hmc` profile | Separate admission design; do not sniff by extension |

## Durable leftovers (do not paper over)

- `fixtures::write` may still call `write_leaf` with a live stage (convenience path).
- Recovery `store_verified` may still run under a live stage.
- Handshake Fixture in `endpoint/tests.rs` stays private/random; two-process reconstructs keys from the PSK file.
- Kind 1–12 frozen. No BearerProfile 8+.
- Native interchange remains CBOR / exact-length binary. JSON exceptional.
