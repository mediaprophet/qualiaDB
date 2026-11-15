# Wave 3-Q — QAC streaming support (senior)

File-disjoint. **Do not cargo. Do not commit.** Integrator routes `mod qac_stream;` in `bundle/stream/formats/mod.rs` after you stop.

## Gap (R07)

`QacCompleteView::admit` opens a **whole** current `.qac` via `QacStorageView::open`. That is the complete-entry fallback and stays the **transfer unit** until a precise partial-QAC contract exists. Still missing for streaming:

- `admit_signal` → `audio::qac::signal::admit` (C3 reserved `QacAdmitError::Unsupported` for this)
- version/profile matrix: complete-entry vs scientific signal vs opaque archival
- declared time interval for the planner (**do not invent a clock**)
- `present[]` id from a successful admit
- unknown critical schema stays archival, never playable audio
- exact signed-zero sample bytes
- many QAC entries in one HMC
- unresolved clock/frame/calibration → `Readiness::Incomplete`, not silence

Do not implement a partial-legacy QAC view. Partial `AuthenticatedRange` still `Extent`. Never `LegacyQacView`. Never rewrite `.qac` keys.

## Read first (read-only)

- Review R07 and §6 items 3–6, 9
- `bundle/stream/formats/qac.rs` and `qac_tests.rs` (keep those tests; do not regress complete-entry)
- `audio::qac::signal::admit` in `audio/qac/signal/view.rs` (read-only)
- `audio::qac::{QacStorageView, QacStorageOptions}`
- `bundle/stream/plan/{inventory,readiness}.rs`
- Candidates: `qac_candidate-opaque.qac`, `qac_candidate-pcm-signed-zero.qac`, `qac_candidate-aligned4096.qac`, `qac_candidate-two-entries.qac`
- Legacy refuse: `audio/qac/fixtures/q4au-v2-contract.qac`

Do **not** edit `audio/qac/**` or `q42/field_*`. Call those APIs.

## Allowed writes

- EDIT `crates/qualia-core-db/src/bundle/stream/formats/qac.rs` — add `admit_signal` only; keep existing `admit`
- NEW `crates/qualia-core-db/src/bundle/stream/formats/qac_stream.rs`
- NEW `crates/qualia-core-db/src/bundle/stream/formats/qac_stream_tests.rs`
- `docs/.../qbdl-streaming/evidence/swarm-wave3-q-summary.md`

Do **not** edit `qac_tests.rs` (C3 corpus stays). Do not edit `formats/mod.rs`.

## Behavior

On `QacCompleteView` (`qac.rs`):

```rust
impl<'a> QacCompleteView<'a> {
    /// Scientific interpretation. Storage admit already succeeded.
    /// Unsupported / Profile / unknown critical → QacAdmitError::Unsupported
    /// (caller may still keep the complete entry as OpaqueArchival).
    pub fn admit_signal<'s>(
        &'s self,
        selected: u32, // 1-based entry inside the .qac, not the HMC entry id
        scratch: &mut crate::q42::field_semantics::Scratch<'_>,
        shared: crate::q42::field_semantics::Limits,
        limits: crate::audio::qac::signal::Limits,
    ) -> Result<crate::audio::qac::signal::SignalView<'s>, QacAdmitError>;
}
```

Wire `signal::admit(self.storage().structure(), selected, scratch, shared, limits)` and map `signal::Error` to `QacAdmitError` without collapsing `Unsupported` vs `Limit` vs `Index`. Inspect live `signal::Error` variants; do not invent names.

`qac_stream.rs`:

```rust
pub enum QacProfile {
    CompleteEntry,
    SignalScientific,
    OpaqueArchival,
}

pub struct QacStreamView<'a> { /* QacCompleteView + profile */ }

impl<'a> QacStreamView<'a> {
    pub fn admit(
        range: AuthenticatedRange<'a>,
        options: QacStorageOptions,
    ) -> Result<Self, QacAdmitError>; // complete-entry; profile = CompleteEntry

    pub fn with_signal(
        self,
        selected: u32,
        scratch: &mut Scratch<'_>,
        shared: Limits,
        limits: signal::Limits,
    ) -> Result<Self, QacAdmitError>; // SignalScientific or OpaqueArchival on Unsupported

    pub const fn profile(&self) -> QacProfile;
    pub fn complete(&self) -> &QacCompleteView<'a>;
    pub const fn ready_for_plan(&self) -> bool; // CompleteEntry or SignalScientific

    /// Source-declared interval only. Absence means no clock.
    pub fn declared_interval(&self) -> Option<crate::bundle::stream::plan::RationalInterval>;
}

pub fn qac_present_id(entry_id: u32, view: &QacStreamView<'_>) -> Option<u32>;
```

`declared_interval`: read only if the authenticated envelope already exposes a rational interval/timebase through **public** storage/structure getters. If those getters do not exist, return `None` and say so in the summary. Do not parse a private layout. Do not equalize EMF and QAC timestamps.

Transfer unit remains the **whole** authenticated QAC entry (complete-entry fallback). Interval is planner metadata so M can schedule two QAC entries with different declared intervals; it is not a byte-range slice of the `.qac` file.

Unknown critical / `signal::Error::Unsupported` / `Profile`: `QacProfile::OpaqueArchival`, `ready_for_plan == false` if the HMC obligation is scientific; complete bytes may still exist on the wire as opaque. Mandatory critical refuse → `QacAdmitError::Unsupported` so F/M mark `Readiness::Refused`, never playable silence.

## Tests (write, do not run)

1. **`complete_entry_still_admits_opaque_candidate`** — same as C3 success; profile `CompleteEntry`.
2. **`admit_signal_on_pcm_signed_zero_preserves_bytes`** — `qac_candidate-pcm-signed-zero.qac`; signal admit; payload bytes equal the verified chunk (signed zero preserved). If signal admit is `Unsupported` on this fixture, assert archival profile and still complete-entry admit — record which in the summary.
3. **`two_qac_entries_admit_independently`** — opaque + pcm-signed-zero (or two-entries fixture as two HMC entries). Two `present` ids.
4. **`partial_range_still_extent`** — do not weaken C3.
5. **`legacy_ua4q_refuses`** — Family/Extent as live open dictates.
6. **`unknown_critical_or_unsupported_signal_is_archival_not_audio`** — `ready_for_plan` false or `Unsupported`; no “empty PCM”.
7. **`declared_interval_absent_is_not_epoch_zero`** — static/untimed fixture → `None`.

## Stop

Summary: `admit_signal` mapping table, whether `declared_interval` had a public getter, tests written **not run**. Do not cargo. Do not commit. Do not edit codec owners or `qac_tests.rs`.
