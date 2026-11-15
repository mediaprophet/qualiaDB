# Swarm C4 — mixed-HMC transfer planner (types + tests)

Deterministic inventory, readiness, and record-boundary schedule for a heterogeneous QBDL-profile HMC. Multiplexing lives in this planner. M64 is one inner family. ZIP `.hmc` is out of scope. This slice is **types + tests**, not the §6 mixed-bundle binary fixture.

This child did not run `cargo`. Tests are written, **not run**. Integrator routes `pub mod plan;` after stop.

## Files written

- `crates/qualia-core-db/src/bundle/stream/plan/mod.rs`
- `crates/qualia-core-db/src/bundle/stream/plan/inventory.rs`
- `crates/qualia-core-db/src/bundle/stream/plan/readiness.rs`
- `crates/qualia-core-db/src/bundle/stream/plan/schedule.rs`
- `crates/qualia-core-db/src/bundle/stream/plan/tests.rs`
- this summary

## Integrator route

`crates/qualia-core-db/src/bundle/stream/mod.rs` (after `pub mod selection;`):

```rust
pub mod plan;
```

No `Cargo.toml` change. Do not edit `formats/**`, `http/**`, `qbdl/service/**`.

## Exports

| Type | Role |
|---|---|
| `EntryFamily` | `TenD \| Q42 \| Emf \| Qac \| M64 \| P64 \| Opaque` from authenticated metadata, not a filename suffix |
| `PlanEntry` / `PlanEntrySpec` | Candidate identity, entry id, family, role/representation, optional declared rational interval, optional `TimebaseId`, resource estimate, mandatory/optional |
| `DependencyEdge` | `from → to` bound to `CandidateIdentity` + entry id. Reasons: frame / calibration / clock / geometry / query |
| `Inventory` | Bounded DAG compile (`MAX_PLAN_ENTRIES=64`, `MAX_PLAN_EDGES=256`) |
| `Readiness` | `Ready \| Incomplete { missing_mandatory } \| Opaque \| Refused` |
| `evaluate` | Presence-flag readiness. No storage, proof, or codec admit |
| `WorkClass` | `Control \| Metadata \| BaseRepresentation \| Enhancement`. Distinct from `net::peer::runtime::WorkClass` |
| `SelectionRequest` / `SelectionIntent` / `PrincipalSelection` | Scene/analysis (entry ids + families + optional time/region). Not an M64 path |
| `Schedule` | Fair `next` of at most one `RecordUnit`. Occupancy intent flag only |
| `PlanError` | `Capacity, Limit, DuplicateId, Identity, Cycle, Graph, Field, Unsupported, Refused` |

## Invariants

- Bind relationships to immutable `CandidateIdentity` + entry id. Filename and entry order are not identity.
- Interval/timebase exist only when the source declared them. Equal timestamps without the same `TimebaseId` are not one clock. A `Clock` edge without a shared declared timebase is `Refused`.
- Static 10D with no interval is valid.
- Missing mandatory ≠ empty field ≠ complete query answer (`Incomplete`). Unknown optional stays `Opaque`. Unknown critical (mandatory `Opaque`, or family mismatch) is `Refused`. Never playable silence.
- `Schedule::next` yields at most one admitted record-boundary unit. Control is never starved. M64/P64 wait while Ready EMF/QAC/10D (and other live families) remain.
- Shared dependencies are charged once by identity equality, not filename.
- Cancel marks work cancelled; occupancy stays until `acknowledge_complete`. This module does not own `ReservationLedger`.
- No JSON. No new wire `Kind`. No `BearerProfile`. No QAC/EMF parser calls. No storage/proof calls. No verified-bytes state.

Hot `next` is a fixed-array scan. Compile uses stack arrays. Tests may allocate.

## Tests written, not run (`plan/tests.rs`)

- `mixed_inventory_constructs_10d_q42_emf_qac_m64_opaque`
- `missing_mandatory_calibration_is_incomplete_not_ready`
- `missing_optional_enhancement_is_opaque_base_ready`
- `next_delays_m64_while_ready_emf_qac_control_continue` (shared 10D charged once)
- `cancel_obsolete_region_skips_cancelled_ready_siblings_schedule`
- `mutate_dependency_identity_refuses`
- `static_10d_without_time_interval_is_valid`
- `emf_qac_equal_timestamps_without_shared_timebase_are_not_one_clock`
- `two_principals_do_not_starve_control`
- `planner_does_not_mark_bytes_verified`

## Wave 3 remaining

- Real canonical writers/readers for the §6 mixed QBDL-profile HMC (10d, q42, emf, qac, m64, optional opaque, P64 if the model gate requires). This slice constructs typed inventory only.
- C3 format admit (QAC complete fallback / shared-envelope) feeding `present` ids. Planner must not grow parsers.
- Integrator routes `pub mod plan;` then one cargo. Wire `Schedule` into qbdl service credits/cancel/occupancy (`ReservationLedger` stays with runtime).
- HTTP/WSS parity of the same plan outcomes (C5). Browser/WebRTC remain later.
- C6 mixed-HMC adversarial suite and `EvidenceClass` records. Do not mark QBS-09 complete from this types slice.

## Stop notes

- Did not edit `bundle/stream/mod.rs`, `formats/**`, `http/**`, `qbdl/service/**`, `audio/qac/**`, `q42/field_*`, Cargo.toml/lock, `registries.rs`.
- Did not mint opcodes, JSON, or `BearerProfile`.
- Did not run `cargo test` / `cargo check` / `cargo build`.
- No git commit/push. Synth MCP was not touched.
- Formatted the five new Rust files with `rustfmt --edition 2021`.
