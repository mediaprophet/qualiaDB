# Wave 3-M — mux transfer loop (expert)

File-disjoint. **Do not cargo. Do not commit.** Integrator routes `mod mux;` in `qbdl/service/mod.rs` after you stop.

## Gap

`Schedule::compile` / `Schedule::next` exist and are tested only inside `bundle/stream/plan/tests.rs`. Production service still transfers **one** `SelectedPlainEntry` via `transfer_one_leaf`. The product needs one session to walk a compiled plan: at most one `RecordUnit` per `next`, Control never starved, M64/P64 delayed while Ready live families continue, shared dependencies charged once by identity, cancel marks work cancelled and occupancy stays until `acknowledge_complete`.

This slice is the **service loop**. It does not construct the mixed binary HMC (F) and does not speak HTTP (H).

## Read first (read-only)

- `docs/.../qbdl-streaming/implementation-review-hmc-multiplexing-2026-11-13.md` §§5–6
- `bundle/stream/plan/{mod,inventory,readiness,schedule}.rs`
- `qbdl/service/mod.rs` `transfer_one_leaf` / `run_one_leaf`
- `qbdl/service/{provider,client,describe,session}.rs`
- `qbdl/service/tests.rs` (ipc_pair + Fixture pattern)

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

## Describe bootstrap (R01 §6.1)

At least one test: the **client** starts from `DescribeClient::bind(identity, policy)` only. Provider may `select_entry` to build catalogs. Client walks class 1 → 6 header, 7 index, 8 body, 9 entry_page via `answer_describe_pages`, then `admit_header` / `admit_index` / `select_entry` on **received** slices, then `PlainClient::bind`. Client must not be constructed from `fixture.entry_page()` / local `SelectedPlainEntry` for the scheduled entries.

Use existing `Fixture` (`policy/tests.rs`) and/or `PlainPublisher` multi-entry (`publish/tests.rs` pattern: `BundleWriter` + `EntryAssignment` per payload). For **this** slice, payloads may be distinct plain bytes. Tag inventory families as `Opaque` (and one `M64` only if you also delay it — do **not** tag a blob as `Qac`/`Emf`/`TenD`). Real family bytes are F’s job.

`ipc_pair` + `Session` / `SessionOwner` as in `service/tests.rs`. In-process is enough. Do not claim `EvidenceClass::Process` or PhysicalNetwork. Do not edit `two_process.rs`.

## Tests (write, do not run)

Name them. Do not derive Debug for `unwrap_err`.

1. **`describe_then_schedule_transfers_two_opaque_entries`** — two published payloads, inventory two Opaque Base Mandatory, `Schedule` scene both, client Describe 8/9 then `transfer_scheduled`. Both leaves arrive. Control (if enqueued) runs before payload.
2. **`next_delays_m64_while_ready_opaque_continues`** — Opaque + optional M64 in inventory; present both; scene both. First transferred family is Opaque. M64 unit is still queued (`delayed_deferred >= 1`) after the Opaque unit. After Opaque `acknowledge_complete`, a later `next` may yield M64.
3. **`cancel_obsolete_entry_skips_and_retains_occupancy`** — two Opaque; cancel one after compile; sibling transfers; cancelled id is not in the report’s transferred set; `occupancy_retained` true until explicit acknowledge of the cancelled unit (or document that cancel-without-issue has no occupancy — match `Schedule::cancel` behavior).
4. **`missing_mandatory_dependency_refuses_before_leaf`** — mandatory calibration entry absent from `present`; `evaluate` Incomplete; `transfer_scheduled` returns `Admission` / `Unsupported` with payload buffer untouched.
5. **`shared_dependency_charged_once`** — two principals select overlapping Opaque dep; that entry’s leaf is sent once; second principal records reuse (`shared_reused >= 1`).
6. **`wrong_family_on_bound_entry_is_identity`** — `BoundEntry.entry_id` matches but inventory family disagrees with the test’s declared intent → refuse before send. (If you cannot observe family on `PlainProvider`, skip this and cover it in F.)

Reuse `LEAF_BYTES` sized payloads from `service/tests.rs` `payload()` if easier than tiny blobs. Fixture pages fit `MAX_LEAF_RECORD_BYTES`.

## Stop

Summary: exports, `PlanError` mapping, tests written **not run**, whether Describe bootstrap is in the loop or only in tests, what F/H still own. Do not cargo. Do not commit. Do not edit parent `mod.rs`.
