# Swarm Wave 3-M — mux transfer loop (expert)

File-disjoint. **Did not cargo. Did not commit.** Exclusive writes: `qbdl/service/mux.rs`, `qbdl/service/mux_tests.rs`, this summary. Integrator routes `mod mux;` in `qbdl/service/mod.rs` after this child stops.

## Files written

| Path | Role |
|---|---|
| `crates/qualia-core-db/src/net/peer/replication/qbdl/service/mux.rs` | `MuxTransfer` / `BoundEntry` / `MuxReport` / `transfer_scheduled`; `#[cfg(test)] #[path = "mux_tests.rs"] mod mux_tests;` |
| `crates/qualia-core-db/src/net/peer/replication/qbdl/service/mux_tests.rs` | Six in-process tests over real `.qac` / `.emf` candidates |
| this summary | |

Not edited: `service/mod.rs`, `provider.rs`, `client.rs`, `describe.rs`, `session.rs`, `protected.rs`, `two_process.rs`, `service/tests.rs`, `formats/**`, `plan/**`, `http/**`, Cargo.toml/lock, registries.rs.

## Exports

`mux.rs` (not re-exported from `service/mod.rs` until the integrator adds `mod mux;`):

- `MuxTransfer::compile(inventory, selections, present)` — `Schedule::compile` plus Incomplete/Refused gate
- `MuxTransfer::next_unit` / `acknowledge` / `acknowledge_complete` / `cancel` / `occupancy_retained` / `push_control` / `is_charged`
- `BoundEntry { entry_id, provider }`
- `MuxReport { transferred, control, delayed_deferred, cancelled_skipped, shared_reused, order, order_len }`
- `transfer_scheduled(...)` — drive `next` until empty

No Kind 13+. No copied Open/Credit/RequestLeaves encoding. Native binary / CBOR only.

## PlanError → service Error

| PlanError | Error |
|---|---|
| Capacity / Limit | Limit |
| DuplicateId / Identity / Cycle / Graph | Identity |
| Field / Unsupported | Unsupported |
| Refused | Admission |

Lookup miss for a scheduled `entry_id` → `Error::Identity`. Control (`entry_id == None`) never looks up a provider.

`evaluate` Incomplete/Refused on a selected intent → `Error::Admission` **before** any leaf I/O. Compile may still succeed (Schedule only enqueues Ready). `payload_out` is left untouched.

## Grant scoping

`LocalWholeSourceGrant::accept_local_scoped` is one-shot (`spent`). `transfer_one_leaf` wraps that once. The mux walk therefore takes **one** grant scope and calls `run_one_leaf` (the body of `transfer_one_leaf`) per unit, then drops the effect. No `CurrentAuthorityGuard` / `RuntimeAccess` / ledger is held across WouldBlock; `run_one_leaf` rechecks per open/leaf. Shared `EntryRef` already charged by `Schedule` is not sent again (`MuxReport.shared_reused`).

## EMF and QAC candidates transferred

One multi-entry QBDL (`PlainPublisher` / `BundleWriter`) per test. Family tags `EntryFamily::Qac` / `Emf` only after `select_entry` **and** admit:

- QAC whole-entry: `logical_length == bytes.len()`, then `QacStorageView::open` (the inner of `QacCompleteView::admit`)
- EMF envelope v4: same whole-entry check, then `q42::field_envelope::StructuralView::open(bytes, Family::EmfV4Candidate, Limits::default())`

`EmfStreamView` is **not** on disk; this slice does not wait and does not edit `formats/**`. Failed admit → id omitted from `present[]`, never transferred as that family.

| File | Size | Role |
|---|---|---|
| `qac_candidate-opaque.qac` | 768 B | live QAC |
| `qac_candidate-pcm-signed-zero.qac` | 960 B | live QAC |
| `emf_candidate-opaque.emf` | 1088 B | live EMF |
| `emf_candidate-aligned4096.emf` | 8192 B | live EMF |

All four fit one `LEAF_BYTES` (16384) and `MAX_LEAF_RECORD_BYTES` (18137) DescriptorPage/leaf record. Extra Opaque/M64/TenD blobs are delay/shared/negative-gate only — never tagged as Qac/Emf.

QAC transfer unit is the whole entry (leaf 0). Concurrent `Schedule` includes both QAC ids and both EMF ids; `next` delivers Ready QAC/EMF/Control before M64/P64.

## Describe bootstrap

**Tests only.** `transfer_scheduled` does not speak Describe. Test 1 binds `DescribeClient::bind(identity, policy)` only; provider may `select_entry` for catalogs. Client walks class 1→6 header, 7 index, 8 body, 9 entry_page via `answer_describe_pages`, then `admit_header` / `admit_index` / `select_entry` on **received** slices, then `PlainClient::bind`. Scheduled clients are not built from `scene.entry_page()` / local `SelectedPlainEntry`.

Grant closures wrap service `Error` as `Ok((|| -> Result<(), Error> { ... })())`. Tests `match` on `Result`. No `Debug` on `MuxTransfer` / `BoundEntry` / `MuxReport`.

## Tests written **not run**

In-process `ipc_pair` + `Session` / `SessionOwner`. Evidence class = **Component**. Not Process. Not PhysicalNetwork.

| Test | What it asserts |
|---|---|
| `describe_then_schedule_transfers_two_qac_and_two_emf` | Four real candidates in one HMC; client Describe 8/9; `transfer_scheduled` delivers all four; Control first if enqueued; received select matches publication; each entry admits again as QAC or EMF |
| `next_delays_m64_while_ready_qac_and_emf_continue` | QAC + EMF + M64; first families are Qac/Emf/Control; `delayed_deferred >= 1`; M64 after live |
| `cancel_obsolete_qac_skips_and_sibling_emf_continues` | Cancel one QAC; sibling EMF (and the other QAC) transfer; `occupancy_retained` until `acknowledge_complete` |
| `missing_mandatory_calibration_refuses_before_leaf` | Mandatory Q42 calib absent from `present`; Incomplete → `Error::Admission`; `payload_out` unchanged |
| `shared_geometry_dep_charged_once` | Two principals, overlapping TenD geometry dep plus distinct QAC/EMF; TenD leaf sent once; `shared_reused >= 1` |
| `blob_tagged_as_qac_is_not_present` | Non-QAC payload assigned `EntryFamily::Qac` fails admit, omitted from `present`; not sent as acoustic content |

## Leftover (E / Q / F / H / integrator)

- **E** — `EmfStreamView` profile/region/`select_region`; this mux admits envelope v4 via `StructuralView` only.
- **Q** — QAC signal/profile helpers; mux uses complete-entry `QacStorageView` / whole-entry extent.
- **F** — mixed binary HMC construction (3A2); mux consumes an already-compiled `Inventory`/`Schedule`.
- **H** — HTTP. This slice is `ipc_pair` only.
- **Integrator** — one line `mod mux;` in `qbdl/service/mod.rs`. Do not re-export until that route exists.
