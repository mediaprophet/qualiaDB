# C4 brief — mixed-HMC transfer planner (inventory, readiness, schedule)

Expert implementer. File-disjoint. **Do not cargo.** Integrator routes `bundle/stream/mod.rs` `pub mod plan;` after you stop.

## Objective

Deterministic, explainable multiplexing of related HMC entries. Bind relationships to **immutable entry/descriptor identities**, not filenames or entry order. Do not invent clocks. Do not rewrite the container. Do not call QAC/EMF parsers (C3 owns admit).

This slice is the **planner types + tests**, not the full §6 mixed-bundle binary fixture (Wave 3).

## Read first

- `docs/.../qbdl-streaming/implementation-review-hmc-multiplexing-2026-11-13.md` §§5–6
- `docs/.../qbdl-streaming/architecture.md` §§6–7 (credits, DAG closure, ready queues)
- `bundle/stream/descriptor/` `CandidateIdentity`
- `bundle/stream/policy/types.rs` `ManifestFields`
- `net/peer/replication/qbdl/wire/prefix.rs` `Kind` 1–12 — **do not mint opcodes**
- `net/peer/replication/qbdl/service/` (read-only; do not edit)

## Allowed writes

- NEW `crates/qualia-core-db/src/bundle/stream/plan/mod.rs`
- NEW `inventory.rs`
- NEW `readiness.rs`
- NEW `schedule.rs`
- NEW `tests.rs`
- `docs/.../qbdl-streaming/evidence/swarm-c4-plan-summary.md`

## Types (keep bounded, no heap in hot decide path if possible; tests may allocate)

`EntryFamily`: `TenD | Q42 | Emf | Qac | M64 | P64 | Opaque` — from authenticated metadata, not extension sniffing.

`PlanEntry`: candidate identity, entry id, family, role/representation, optional rational time interval **only if the source declared one**, resource estimate, mandatory vs optional.

`DependencyEdge`: from_entry → to_entry, `Mandatory | Optional`, reason tag (frame / calibration / clock / geometry / query). Unknown critical interpretation → refuse, never playable silence.

`Readiness`: `Ready | Incomplete { missing_mandatory } | Opaque | Refused`. Missing mandatory ≠ empty field ≠ complete query answer. Unknown optional stays `Opaque`.

`WorkClass`: `Control | Metadata | BaseRepresentation | Enhancement`. Control is never starved.

`Schedule::next(...)`: pick at most one admitted record-boundary unit. Delay M64/P64 while EMF/QAC/10D that are Ready continue. Shared dependency charged once (identity equality, not filename). Cancel obsolete work: mark cancelled, occupancy retained until caller completes (this module records the intent; it does not own the ledger).

`SelectionRequest`: scene/analysis intent (entry ids + families + optional time/region). Not an M64 path.

No JSON. No new wire Kind. No `BearerProfile`. No clocks where the source has none.

## Tests

- mixed inventory of 10d+q42+emf+qac+m64+opaque constructs
- missing mandatory calibration → `Incomplete`, not Ready
- missing optional enhancement → `Opaque`, base stays Ready
- delay M64: `next` returns EMF/QAC/control while M64 is deferred
- cancel obsolete region: cancelled item is not scheduled; Ready siblings still schedule
- mutate dependency identity → refuse (`Refused` / identity error)
- static 10d with no time interval is valid
- invented equal timestamps across EMF/QAC without declared shared timebase are **not** treated as one clock
- fairness: two principals, neither starves Control
- planner does not mark bytes verified (no storage/proof calls)

## Stop

Summary: exports, invariants, tests written **not run**, what Wave 3 still needs (real writers, service wiring, HTTP parity). Do not cargo. Do not commit. Do not edit `formats/**` or `service/**`.
