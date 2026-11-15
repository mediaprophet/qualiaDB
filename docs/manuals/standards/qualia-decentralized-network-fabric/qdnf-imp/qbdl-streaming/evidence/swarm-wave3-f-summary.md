# Wave 3-F summary — mixed QBDL-profile HMC constructor

File-disjoint. **Did not cargo. Did not rustc. Did not commit.** Synth MCP was not touched. Parent `bundle/stream/mod.rs` was not edited. Codec-owner files, `formats/**`, `plan/**`, `http/**`, `qbdl/service/**`, Cargo.toml/lock, and registries were not edited.

Tests were **written, not run**.

## Files written

| Path | Role |
|---|---|
| `crates/qualia-core-db/src/bundle/stream/mixed/mod.rs` | Re-exports + `#[cfg(test)] mod tests`. No `pub mod mixed` in parent. |
| `crates/qualia-core-db/src/bundle/stream/mixed/bundle.rs` | `publish_mixed`, `MixedHmc`, `mixed_plan`, `default_mixed_entries` (native test) |
| `crates/qualia-core-db/src/bundle/stream/mixed/tests.rs` | Seven tests, `#![cfg(not(target_arch = "wasm32"))]` |
| this summary | |

## Integrator routing

```
// bundle/stream/mod.rs
pub mod mixed;
```

Unrouted until that line lands. This child does not add it.

## Public API

- `MixedEntry`, `MixedHmc`, `MixedError { DuplicateId, Limit, Identity, Unsupported }`
- `publish_mixed` — `PlainPublisher` / `BundleWriter`, assignment ids ≠ 0
- `MixedHmc::{source, body, entry_page, identity, entry_id, family, len}` plus payload/role/obligation, Scene-like `installed` / `bound` / `request` / `select` so tests do not republish
- `mixed_plan` → `MixedPlan` → `Inventory::compile`; geometry EMF/QAC → first TenD; mandatory calibration first EMF → first Q42
- `default_mixed_entries` — `#[cfg(all(test, not(target_arch = "wasm32")))]` because Q42 uses `tempfile` + `write_unified_volume`

Empty `bytes` → `Limit`. Duplicate `id` (including two `EntryFamily::Qac`) → `DuplicateId`. Id 0 → `Identity`. Publisher/descriptor/policy/plan/bundle errors map into the four variants.

## Default corpus entry ids

Many entries per family (container rule), not 1-of-each.

| Id | Family | Role | Obligation | Source |
|---|---|---|---|---|
| 1 `MIXED_ID_TEND_A` | TenD | Base | Mandatory | `compile_mesh_to_10d` triangle A |
| 2 `MIXED_ID_TEND_B` | TenD | Base | Mandatory | `compile_mesh_to_10d` triangle B (distinct positions) |
| 10 `MIXED_ID_EMF_OPAQUE` | Emf | Base | Mandatory | `emf_candidate-opaque.emf` |
| 11 `MIXED_ID_EMF_ALIGN` | Emf | Base | Mandatory | `emf_candidate-aligned4096.emf` |
| 20 `MIXED_ID_QAC_OPAQUE` | Qac | Base | Mandatory | `qac_candidate-opaque.qac` |
| 21 `MIXED_ID_QAC_PCM` | Qac | Base | Mandatory | `qac_candidate-pcm-signed-zero.qac` |
| 30 `MIXED_ID_Q42` | Q42 | Metadata | Mandatory | `write_unified_volume` (same as `q42_range_tests`) |
| 40 `MIXED_ID_M64` | M64 | Enhancement | Optional | copied `m64_bytes(false, *b"m64\0", true)` |
| 50 `MIXED_ID_OPAQUE` | Opaque | Enhancement | Optional | `b"qbdl-mixed-opaque-v1"` |

9 entries. `identity.entry_count` must match.

## Fixture paths (include, not edited)

```
crates/qualia-core-db/src/q42/field_envelope/fixtures/candidate/
  qac_candidate-opaque.qac
  qac_candidate-pcm-signed-zero.qac
  emf_candidate-opaque.emf
  emf_candidate-aligned4096.emf

crates/qualia-core-db/src/audio/qac/fixtures/q4au-v2-contract.qac
  (legacy UA4Q negative test only)
```

From `mixed/`, `include_bytes!` is `../../../q42/field_envelope/fixtures/candidate/...`.

Not used as extra HMC families: `qac_candidate-aligned4096.qac`, `emf_candidate-two-entries.emf` (inner two-entries is one payload; this HMC already has two QAC + two EMF as separate QBDL entries).

## Admit views called (after `select_entry` + `AuthenticatedRange`)

| Family | View | Present |
|---|---|---|
| TenD | `TenDDirectoryView::admit` (header offset 0 len 64, table from header `section_table_offset` / `section_count`) | id in `present[]` |
| Qac | `QacStreamView::admit` + `QacCompleteView::admit`; `qac_present_id` | complete-entry only. **Do not drop pcm-signed-zero because `with_signal` is archival.** |
| Emf | `EmfStreamView::admit` (`Family::EmfV4Candidate`, envelope v4) + `select_region(0)`; `emf_present_id` | **Not** `EmfDirectoryView` (v2 brick) |
| M64 | `M64DirectoryView::admit` + `selected_chunk(0, …)` | id in `present[]` |
| Q42 | `Q42AuthenticatedRangeSource` + `Q42RangeVolume::open` | id in `present[]` |
| Opaque | no format admit | omitted from `present[]` (optional enhancement) |

Planner graph: four Geometry edges (both EMF, both QAC → TenD A) plus one mandatory Calibration (EMF opaque → Q42). `evaluate` Ready only when those present ids close. Dropping Q42 from `present` → `Readiness::Incomplete { missing_mandatory: 30 }` on EMF opaque.

## Tests written **not run**

Evidence class = source only. Native, not wasm (`tempfile`).

| Test | Asserts |
|---|---|
| `default_mixed_publishes_many_per_family` | TenD≥2, Emf≥2, Qac≥2, plus Q42, M64, Opaque; `identity.entry_count` == `len` |
| `each_family_admits_through_format_view` | `select_entry` each id; family view as above; Opaque skips |
| `original_entry_bytes_preserved` | source slice and leaf span == payload passed in |
| `inventory_compiles_with_geometry_edges` | `Inventory::compile`; Ready when mandatory ids present (including both QAC complete-entry ids) |
| `missing_mandatory_calibration_is_incomplete` | drop Q42 from present → Incomplete, not Ready/Opaque/Refused |
| `duplicate_entry_id_refuses` | two `EntryFamily::Qac` with the same id → `MixedError::DuplicateId` |
| `legacy_ua4q_as_qac_entry_refuses_admit` | `q4au-v2-contract.qac` tagged Qac; `QacCompleteView::admit` fails Family/Extent/Envelope |

Match on `Result`. No `Debug` on `MixedHmc`. No `unwrap_err` on success types.

## Candidates that would not admit

**Not executed.** On-disk candidates are the same writer output E/Q already documented:

- Both QAC files: complete-entry `QacStreamView::admit` expected Ok. pcm-signed-zero `with_signal` expected `OpaqueArchival` (missing shape key 15) — **present id still taken from complete-entry admit**, not from archival signal.
- Both EMF files: envelope v4 `EmfStreamView::admit` + `select_region(0)` expected Ok (`Q42EMF64`, version 4). Forcing `EmfDirectoryView` on these bytes would be wrong.
- Legacy UA4Q: admit **must fail** (test 7).
- 10D / M64 / Q42: constructed with the same public helpers as `ten_d_tests` / `m64_tests` / `q42_range_tests`. Q42 construction is the one-quin `write_unified_volume` that those range tests assert fits one `LEAF_BYTES` span.

If integrator cargo shows a candidate that does not admit, **stop**. Do not invent headers. Do not call QAC/EMF writers.

## Not done

- Did not cargo / rustc / commit / push.
- Did not edit `bundle/stream/mod.rs`.
- Did not kill Synth MCP.
- No Kind 13+. Native CBOR / exact-length binary only.
