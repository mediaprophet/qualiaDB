# Wave 3-M — mux transfer loop (expert)

File-disjoint. **Do not cargo. Do not commit.** Integrator routes `mod mux;` in `qbdl/service/mod.rs` after you stop.

## Gap

`Schedule::compile` / `Schedule::next` exist and are tested only inside `bundle/stream/plan/tests.rs`. Production service still transfers **one** `SelectedPlainEntry` via `transfer_one_leaf`. The product needs one session to walk a compiled plan: at most one `RecordUnit` per `next`, Control never starved, M64/P64 delayed while Ready live families continue, shared dependencies charged once by identity, cancel marks work cancelled and occupancy stays until `acknowledge_complete`.

This slice is the **service loop over real EMF and QAC entries**. It does not construct the full mixed binary HMC (F, 3A2) and does not speak HTTP (H). E/Q own profile/region/signal helpers; M must still transfer authentic `.emf` / `.qac` bytes in this wave.

## Read first (read-only)

- `docs/.../qbdl-streaming/implementation-review-hmc-multiplexing-2026-11-13.md` §§5–6 and R07
- `bundle/stream/plan/{mod,inventory,readiness,schedule}.rs`
- `qbdl/service/mod.rs` `transfer_one_leaf` / `run_one_leaf`
- `qbdl/service/{provider,client,describe,session}.rs`
- `qbdl/service/tests.rs` (ipc_pair + Fixture pattern)
- `bundle/stream/formats/{qac,emf,emf_v3}.rs` admit APIs
- Candidate files: `q42/field_envelope/fixtures/candidate/qac_candidate-{opaque,pcm-signed-zero}.qac`, `emf_candidate-{opaque,aligned4096}.emf`

Do **not** edit those files.

## Allowed writes

- NEW `crates/qualia-core-db/src/net/peer/replication/qbdl/service/mux.rs`
- NEW `crates/qualia-core-db/src/net/peer/replication/qbdl/service/mux_tests.rs`
- `docs/.../qbdl-streaming/evidence/swarm-wave3-m-summary.md`

`mux.rs` must `#[cfg(test)] mod mux_tests;` **or** `mux_tests.rs` is a sibling module that `mux.rs` declares. Integrator only adds `mod mux;` in `service/mod.rs`. Prefer:

```rust
// mux.rs
#[cfg(test)]
#[path = "mux_tests.rs"]
mod mux_tests;
```

so the integrator route is one line.

## Behavior

Reuse `PlainProvider`, `PlainClient`, `DescribeClient`, `transfer_one_leaf`, `Schedule`. Do not copy Open/Credit/RequestLeaves encoding. Do not mint Kind 13+. Do not hold `CurrentAuthorityGuard` / `RuntimeAccess` / ledger across WouldBlock — `transfer_one_leaf` already scopes the grant per leaf; call it per unit and drop.

Suggested types (names may vary; keep bounded, no heap on the decide path; tests may allocate):

```rust
pub struct MuxTransfer {
    schedule: Schedule,
}

pub struct BoundEntry<'src, 'sel, 'nodes> {
    pub entry_id: u32,
    pub provider: PlainProvider<'src, 'sel, 'nodes>,
}

pub struct MuxReport {
    pub transferred: u16,
    pub control: u16,
    pub delayed_deferred: u16, // M64/P64 units still queued when a live unit ran
    pub cancelled_skipped: u16,
}

impl MuxTransfer {
    pub fn compile(
        inventory: &Inventory<'_>,
        selections: &[PrincipalSelection<'_>],
        present: &[u32],
    ) -> Result<Self, Error> { /* map PlanError → Error::Admission / Identity / Unsupported */ }

    pub fn next_unit(&mut self) -> Option<ScheduledUnit> { self.schedule.next() }

    pub fn acknowledge(&mut self, unit: ScheduledUnit) -> Result<(), Error> { /* schedule.acknowledge_complete */ }

    pub fn cancel(&mut self, entry_id: u32, region: Option<Region>) -> usize { self.schedule.cancel(entry_id, region) }
}

/// Drive `next` until empty. Control units acknowledge with no payload.
/// Entry units look up `BoundEntry` by id, then `transfer_one_leaf` for `unit.unit().leaf()`.
pub fn transfer_scheduled<PS: ServiceIo, CS: ServiceIo>(
    mux: &mut MuxTransfer,
    entries: &[BoundEntry<'_, '_, '_>],
    clients: &mut [PlainClient<'_>], // same order / entry_id as entries, or look up by id
    provider_session: &mut PS,
    client_session: &mut CS,
    grant: &mut LocalWholeSourceGrant<'_, '_>,
    pins: &PinAuthority,
    clock: impl Fn() -> u64,
    payload_out: &mut [u8],
) -> Result<MuxReport, Error>;
```

Mapping `PlanError` → service `Error` (do not panic):

| PlanError | Error |
|---|---|
| Capacity / Limit | Limit |
| DuplicateId / Identity / Cycle / Graph | Identity |
| Field / Unsupported / Refused | Unsupported / Admission |

Lookup miss for a scheduled `entry_id` → `Error::Identity`. Control (`entry_id == None`) never looks up a provider.

`Readiness::Incomplete` / `Refused` from `evaluate` must refuse **before** any `transfer_one_leaf`. Compile may still succeed with incomplete present-flags; `transfer_scheduled` must not disclose a unit whose entry is not Ready.

Shared dependency: if `Schedule` already charged an `EntryRef`, transferring it a second time for a second principal is a bug in this loop — skip with `Error::Credit` or count as already-complete. Prefer skip + increment a `shared_reused` field on `MuxReport` rather than a second leaf send.

Cancel: after `mux.cancel(id, region)`, later `next` must not yield that unit; `occupancy_retained` stays true until `acknowledge`.

## EMF and QAC (required in this slice)

Publish **one** multi-entry QBDL with `PlainPublisher` / `BundleWriter` whose inner files are `include_bytes!` of canonical candidates — at least **two distinct `.qac` and two distinct `.emf`**. Tag those inventory rows `EntryFamily::Qac` and `EntryFamily::Emf` only after:

- `select_entry` for that id
- `QacCompleteView::admit` or `EmfDirectoryView::admit` (or `EmfStreamView` / `QacStreamView` if E/Q are already routed)

If admit fails, that id is not `present` and must not transfer. Do **not** tag a random blob as Qac/Emf. An extra Opaque or M64 payload is allowed for delay/cancel tests.

QAC transfer unit is the **whole entry** (complete-entry fallback). EMF may transfer the entry leaf that contains the selected region; if `EmfStreamView::select_region` is routed, prefer that chunk’s authenticated bytes as the useful payload check.

Concurrent: one `Schedule` scene that includes both QAC ids and both EMF ids. `next` must deliver Ready QAC and EMF (and Control) **before** M64/P64. Shared 10D/calibration, if you add a fifth entry as a mandatory geometry dep, is charged once.

If E/Q modules are not on disk yet, call the existing complete-entry / directory admit APIs. Do not wait and do not edit `formats/**`.

## Describe bootstrap (R01 §6.1)

At least one test: the **client** starts from `DescribeClient::bind(identity, policy)` only. Provider may `select_entry` to build catalogs. Client walks class 1 → 6 header, 7 index, 8 body, 9 entry_page via `answer_describe_pages`, then `admit_header` / `admit_index` / `select_entry` on **received** slices, then `PlainClient::bind`. Client must not be constructed from `fixture.entry_page()` / local `SelectedPlainEntry` for the scheduled entries.

`ipc_pair` + `Session` / `SessionOwner` as in `service/tests.rs`. In-process is enough. Do not claim `EvidenceClass::Process` or PhysicalNetwork. Do not edit `two_process.rs`.

## Tests (write, do not run)

Name them. Do not derive Debug for `unwrap_err`.

1. **`describe_then_schedule_transfers_two_qac_and_two_emf`** — four real candidate payloads in one HMC; client Describe 8/9; `transfer_scheduled` delivers all four; each received leaf admits again as QAC or EMF. Control (if enqueued) runs before payload.
2. **`next_delays_m64_while_ready_qac_and_emf_continue`** — QAC + EMF + optional M64; first transferred families are Qac/Emf/Control; `delayed_deferred >= 1` while a live unit remains.
3. **`cancel_obsolete_qac_skips_and_sibling_emf_continues`** — cancel one QAC id; EMF sibling transfers; occupancy matches `Schedule::cancel`.
4. **`missing_mandatory_calibration_refuses_before_leaf`** — mandatory calib absent from `present`; `evaluate` Incomplete; no QAC/EMF payload written to `payload_out`.
5. **`shared_geometry_dep_charged_once`** — two principals select overlapping TenD or Opaque geometry dep plus distinct QAC/EMF; that dep’s leaf is sent once (`shared_reused >= 1`).
6. **`blob_tagged_as_qac_is_not_present`** — a non-QAC payload assigned `EntryFamily::Qac` fails admit and is omitted from `present`; `transfer_scheduled` does not send it as acoustic content.

Candidate files may exceed `LEAF_BYTES`; use the publication’s real leaf geometry (`TreeBuilder` / fixture helper as in `service/tests.rs`). Pages must still fit session buffers (`MAX_LEAF_RECORD_BYTES` for DescriptorPage). If a candidate is too large for the fixture leaf cap, use the smaller candidates (`qac_candidate-opaque`, `emf_candidate-opaque`) and note the size in the summary.

## Stop

Summary: exports, `PlanError` mapping, which QAC/EMF candidates transferred, tests written **not run**, whether Describe bootstrap is in the loop or only in tests, what E/Q/F/H still own. Do not cargo. Do not commit. Do not edit parent `mod.rs` or `formats/**`.
