# Wave 3-Q summary — QAC streaming support

File-disjoint. **Did not run cargo. Did not commit.** Synth MCP was not touched. `formats/mod.rs` was not edited. `qac_tests.rs`, `audio/qac/**`, `q42/field_*`, `emf.rs`, `emf_stream.rs`, mux, Cargo, and registries were not edited.

Tests were **written, not run**.

## Files written

- EDIT `crates/qualia-core-db/src/bundle/stream/formats/qac.rs` — `admit_signal` + live `signal::Error` mapping. Existing `admit` unchanged. C3 `From` impls unchanged.
- NEW `crates/qualia-core-db/src/bundle/stream/formats/qac_stream.rs`
- NEW `crates/qualia-core-db/src/bundle/stream/formats/qac_stream_tests.rs` (path-declared from `qac_stream.rs`)
- this summary

## `admit_signal`

```rust
impl<'a> QacCompleteView<'a> {
    pub fn admit_signal<'s>(
        &'s self,
        selected: u32, // 1-based inner QAC entry
        scratch: &mut field_semantics::Scratch<'_>,
        shared: field_semantics::Limits,
        limits: signal::Limits,
    ) -> Result<signal::SignalView<'s>, QacAdmitError>;
}
```

Calls `audio::qac::signal::admit(self.storage().structure(), selected, scratch, shared, limits)`. Never `LegacyQacView`. Never rewrites `.qac` keys. Partial `AuthenticatedRange` still `Extent` on `admit`.

## `signal::Error` → `QacAdmitError` mapping

Live `audio::qac::signal::Error`: `Envelope`, `Shared`, `Unsupported`, `Shape`, `Scalar`, `Unit`, `Clock`, `Overflow`, `Nonfinite`, `Index`, `Limit`, `Profile`.

C3 `From<field_envelope::Error>` / `From<QacStorageError>` / `From<FormatError>` are unchanged. New arms are only for signal-path names that would otherwise collapse into `Unsupported`/`Envelope`.

| live `signal::Error` | `QacAdmitError` |
| --- | --- |
| `Envelope(e)` | existing envelope table (`Family`/`Version`/`Extent`/`Crc`/`IndexDigest`/`PayloadDigest`/`Limit`, else `Envelope(e)`) |
| `Unsupported` | `Unsupported` |
| `Profile` | `Unsupported` |
| `Limit` | `Limit` |
| `Index` | `Index` (new; not `Envelope(EntryIndex)`) |
| `Shape` | `Shape` |
| `Scalar` | `Scalar` |
| `Unit` | `Unit` |
| `Clock` | `Clock` |
| `Overflow` | `Overflow` |
| `Nonfinite` | `Nonfinite` |
| `Shared(UnknownCriticalSchema)` | `Unsupported` |
| `Shared(UnknownCriticalField)` | `Unsupported` |
| `Shared(UnknownCriticalType)` | `Unsupported` |
| `Shared(UnsupportedDomain)` | `Unsupported` |
| `Shared(RequiredCritical)` | `Unsupported` (mandatory critical refuse → F/M `Readiness::Refused`) |
| `Shared(MissingField)` | `Unsupported` (scientific profile not admitted; see pcm-signed-zero) |
| `Shared(Limit)` | `Limit` |
| `Shared(Overflow)` | `Overflow` |
| `Shared(other)` including `Frame`/`Time`/`Reference`/`Scratch` | `Shared(other)` (unresolved frame/time stay distinct so F/M can mark Incomplete, not silence) |

`Unsupported` vs `Limit` vs `Index` vs `Envelope` are not collapsed.

## `QacStreamView`

```rust
pub enum QacProfile { CompleteEntry, SignalScientific, OpaqueArchival }
pub struct QacStreamView<'a> { /* QacCompleteView + profile; no Debug */ }
pub fn qac_present_id(entry_id: u32, view: &QacStreamView<'_>) -> Option<u32>;
```

- `admit` → complete-entry transfer unit, `QacProfile::CompleteEntry`, `ready_for_plan == true`
- `with_signal` → `SignalScientific` on success; `OpaqueArchival` on `QacAdmitError::Unsupported`; other errors still `Err` (Limit/Index/Clock/Envelope/…)
- `ready_for_plan` is `CompleteEntry | SignalScientific` only
- `qac_present_id` is `ready_for_plan().then_some(entry_id)`
- Archival is never playable audio and is not a present id

## `declared_interval`

**No public authenticated timebase getter.** `StructuralView::entry` exposes `start_tick` / `time_count` (and flags bit 2 for temporal), but no public storage/structure API authenticates a timebase identity or rational rate/epoch. `SignalView::timing()` exists only after scientific admit and is not a storage getter; `QacStreamView` does not retain a `SignalView`. Mapping ticks with an invented denominator, or treating `start_tick == 0` as epoch zero, would mint a clock. Method always returns `None`. Absence is not epoch zero. EMF timestamps are not equalized.

pcm-signed-zero directory has `start_tick = 0`, `time_count = 4`, flags temporal; still `None`.

## Candidate fixture observation (not executed)

Inspected on-disk headers/metadata only:

- `qac_candidate-opaque.qac` — mode `0x104`, one entry, complete-entry admits. Schema 205 shape is `urn:qualia:candidate:opaque…` and lacks required shape key 15.
- `qac_candidate-pcm-signed-zero.qac` — mode `0x100`, flags 3, records 4, stride 4. Chunk prefix `00 00 00 00 | 00 00 00 80` (+0.0 / −0.0). Shape model `urn:qualia:candidate:pcm`, quantity `digital-audio`, units `normalized`; required shape key 15 absent. Live `signal::admit` is expected to fail at `admit_shared` with `Shared(MissingField)` → mapped `Unsupported` → `with_signal` **OpaqueArchival**, complete-entry and signed-zero chunk bytes retained. Test accepts either scientific success (bytes == verified chunk) or this archival path.
- `qac_candidate-two-entries.qac` — two inner `0x104` entries; used as one HMC payload for independent `admit_signal(1)` / `admit_signal(2)`. Two HMC present ids come from opaque + pcm as two QBDL entries.
- `qac_candidate-unknown-critical-schema.qac` — structural open succeeds; block schema 65000 critical → `UnknownCriticalSchema` → `Unsupported` → archival, `ready_for_plan == false`, payload still verified.
- Legacy `q4au-v2-contract.qac` (72 B) — same C3 refuse (`Extent` before Family).

## Tests written (not run)

Path-declared `qac_stream_tests.rs`. Match on `Result`. No `unwrap_err` on success types (`QacStreamView` / `QacCompleteView` have no `Debug`). Native `include_bytes!` only. No JSON. No Kind 13+.

1. `complete_entry_still_admits_opaque_candidate` — C3 success via `QacStreamView`; profile `CompleteEntry`; present id 1
2. `admit_signal_on_pcm_signed_zero_preserves_bytes` — verified chunk signed-zero bits; scientific `as_bytes() == chunk` **or** archival + complete-entry retained
3. `two_qac_entries_admit_independently` — opaque + pcm as two HMC entries, two present ids, distinct `candidate_identity`; plus two-entries inner `admit_signal(1)` / `admit_signal(2)`
4. `partial_range_still_extent` — short length and `offset != 0`
5. `legacy_ua4q_refuses` — Family/Extent/Envelope as live open dictates
6. `unknown_critical_or_unsupported_signal_is_archival_not_audio` — `OpaqueArchival`, not ready, no present id, no empty PCM
7. `declared_interval_absent_is_not_epoch_zero` — opaque and pcm (complete-entry and after `with_signal`) → `None`

## Integrator routing snippet

```
// formats/mod.rs (beside existing qac pair; do not drop qac_tests)
mod qac_stream;
pub use qac_stream::{qac_present_id, QacProfile, QacStreamView};
```

Tests compile through `#[path = "qac_stream_tests.rs"]` inside `qac_stream.rs`; no extra `mod qac_stream_tests` line. No `Cargo.toml` change. Unrouted until those lines land.

## Risks

- Unrouted: nothing typechecks until `mod qac_stream`.
- pcm-signed-zero is a candidate envelope, not a writer-produced `urn:qualia:qac:pcm-preservation:1` document. Expected streaming profile is **OpaqueArchival**, not `SignalScientific`. Signed-zero proof is the verified chunk, not `SignalView::as_bytes`, unless a later fixture grows full scientific metadata.
- `Shared(MissingField)` mapped to `Unsupported` so in-tree candidates take the reserved archival path rather than a hard `Err` that would drop the complete entry from `with_signal`. Writer-produced PCM (storage_cutover documents) still hit `Ok` → `SignalScientific`.
- `declared_interval` is always `None`. Planner cannot schedule two QAC entries on distinct declared clocks until a public authenticated timebase getter exists.
- Complete-entry remains the transfer unit. No partial-QAC view.
