# Wave 3-F — mixed QBDL-profile HMC constructor (senior)

File-disjoint. **Queued Wave 3A2** — start after E/Q/M have stopped and integrator has routed `emf_stream` / `qac_stream` / `mux`. **Do not cargo. Do not commit.** Integrator routes `pub mod mixed;` in `bundle/stream/mod.rs` after you stop.

## Gap

Planner tests construct typed `PlanEntry` arrays. Format adapters admit **one** selected range. There is still no canonical **multi-entry QBDL-profile HMC** whose payloads are real `.10d` / `.q42` / `.emf` / `.qac` / `.m64` bytes. Review §6 requires that fixture, with **many entries per family** as the container rule (two or more QAC, two or more EMF, two or more 10D — not a 1-of-each showcase).

Do not label random blobs with family tags. Bytes must admit through the existing format views.

## Read first (read-only)

- Review §§5–6 (`implementation-review-hmc-multiplexing-2026-11-13.md`)
- `bundle/stream/publish/{mod,owner,tests}.rs` — `PlainPublisher::prepare` + `EntryAssignment` + `BundleWriter`
- `bundle/stream/policy/tests.rs` `Fixture` (single-entry pattern to generalize)
- `bundle/stream/formats/{qac,emf,emf_v3,ten_d,m64,m64_v3,q42_ranges}.rs` admit APIs
- `bundle/stream/plan/{mod,inventory}.rs` — compile inventory from real entry ids
- Canonical candidate files (include, do not edit):

```
crates/qualia-core-db/src/q42/field_envelope/fixtures/candidate/
  qac_candidate-opaque.qac
  qac_candidate-pcm-signed-zero.qac
  qac_candidate-aligned4096.qac
  emf_candidate-opaque.emf
  emf_candidate-aligned4096.emf
  emf_candidate-two-entries.emf
```

Those files are **writer output already on disk**. Using `include_bytes!` is using real canonical readers’ corpus. Do **not** edit `q42/field_*` or `audio/qac/**`. If a candidate file is missing or no longer admits, stop and write that in the summary rather than synthesizing a header.

10D: `crate::render::compile_10d::compile_mesh_to_10d` (or `_with_nodes`) on two distinct small `Mesh` values — two different 10D byte strings.

M64: reuse the synthetic constructor in `formats/m64_tests.rs` (`m64_bytes`) by copying the **construction logic into `mixed/bundle.rs` as a small helper**, or call a public M64 encoder if one exists. Do not edit `m64_tests.rs`.

Q42: a small authentic Q42 volume/range already used by `formats/q42_range_tests.rs`. `include_bytes!` of that fixture, or the same public encoder the range tests use. One Q42 entry is enough for the minimum set; two is better.

Opaque: a distinct non-format payload (e.g. `b"qbdl-mixed-opaque-v1"`) as `EntryFamily::Opaque` optional enhancement.

P64: omit unless a complete-entry public encoder is a one-call helper. Do not pull GGUF compilation into this slice.

## Allowed writes

- NEW `crates/qualia-core-db/src/bundle/stream/mixed/mod.rs`
- NEW `crates/qualia-core-db/src/bundle/stream/mixed/bundle.rs`
- NEW `crates/qualia-core-db/src/bundle/stream/mixed/tests.rs`
- `docs/.../qbdl-streaming/evidence/swarm-wave3-f-summary.md`

Integrator adds `pub mod mixed;` to `bundle/stream/mod.rs`. You do not.

## Behavior

Publish **one** QBDL-profile HMC (plain candidate) whose inner files are the real payloads:

```rust
pub struct MixedEntry<'a> {
    pub id: u32,
    pub family: EntryFamily,
    pub role: RepresentationClass,
    pub obligation: Obligation,
    pub bytes: &'a [u8],
}

pub struct MixedHmc {
    source: Vec<u8>,
    body: Vec<u8>,
    entry_page: Vec<u8>,
    identity: CandidateIdentity,
    // bounded: ids + families, MAX_PLAN_ENTRIES
}

pub fn publish_mixed(entries: &[MixedEntry<'_>]) -> Result<MixedHmc, MixedError>;

impl MixedHmc {
    pub fn source(&self) -> &[u8];
    pub fn body(&self) -> &[u8];
    pub fn entry_page(&self) -> &[u8];
    pub fn identity(&self) -> CandidateIdentity;
    pub fn entry_id(&self, i: usize) -> u32;
    pub fn family(&self, i: usize) -> EntryFamily;
    pub fn len(&self) -> usize;
}

/// Default §6 corpus: N of several families, not 1-of-each as a uniqueness rule.
pub fn default_mixed_entries() -> Result<MixedHmc, MixedError>;
```

`default_mixed_entries` must include **at least**:

| Family | Count | Source |
|---|---|---|
| TenD | ≥2 | `compile_mesh_to_10d` two meshes |
| Emf | ≥2 | distinct `emf_candidate-*.emf` |
| Qac | ≥2 | distinct `qac_candidate-*.qac` |
| Q42 | ≥1 | existing Q42 fixture/encoder |
| M64 | ≥1 | synthetic valid M64 |
| Opaque | ≥1 | distinct blob |

Assignment ids are stable integers you choose (do not use 0). Duplicate `id` → `MixedError::DuplicateId`. Empty `bytes` → `Limit`. `PlainPublisher` + `BundleWriter::add_file` per entry, same finish path as `publish/tests.rs`.

Also export a helper that builds `PlanEntry` / `DependencyEdge` / `Inventory` from the published HMC (geometry edges from EMF/QAC → a TenD, optional calibration Q42). Missing mandatory calibration test uses this graph with that id absent from `present`.

After publish, **E/Q admit is mandatory**:

- each QAC id → `QacStreamView::admit` (and `with_signal` where the fixture supports it); `qac_present_id` feeds `present[]`
- each EMF id → `EmfStreamView::admit` + `select_region` on at least one chunk; `emf_present_id` feeds `present[]`
- `Readiness::Ready` only when those present ids and mandatory edges close
- archival/unsupported signal or unknown-critical EMF is `Opaque` / not present, never a valid empty field

If E/Q names differ after routing, call the exported helpers. Do not reimplement envelope parsers.

Authenticated relationships: bind `CandidateIdentity` + entry id. Filename and `BundleWriter` path are not identity.

Preserve original inner bytes: after publish, `select_entry` + leaf span of each entry equals the payload passed in. No resampling, no `.qac` key rewrite, no format upgrade.

## Tests (write, do not run)

1. **`default_mixed_publishes_many_per_family`** — counts: TenD≥2, Emf≥2, Qac≥2, plus Q42, M64, Opaque. `identity.entries` matches.
2. **`each_family_admits_through_format_view`** — `select_entry` each id; `AuthenticatedRange`; then `TenDDirectoryView::admit` / `QacCompleteView::admit` / `EmfDirectoryView` or `EmfV3DirectoryView` / `M64DirectoryView` or `M64GeoRasterDirectoryView` / Q42 range view. Opaque has no format admit (skip). Match on `Result`, no Debug-derive for errors.
3. **`original_entry_bytes_preserved`** — payload round-trip per id.
4. **`inventory_compiles_with_geometry_edges`** — `Inventory::compile` succeeds; `evaluate` Ready when all mandatory ids present.
5. **`missing_mandatory_calibration_is_incomplete`** — drop calib from `present` → `Readiness::Incomplete`.
6. **`duplicate_entry_id_refuses`** — even when both are `EntryFamily::Qac`.
7. **`legacy_ua4q_as_qac_entry_refuses_admit`** — optional: include `audio/qac/fixtures/q4au-v2-contract.qac` as a Qac-tagged entry and assert `QacCompleteView::admit` fails. Do not edit that fixture.

Do not call QAC/EMF **writers**. Do not edit `formats/**`. Do not edit `plan/tests.rs`.

## Stop

Summary: entry id table, fixture paths used, admit views called, tests written **not run**, any candidate file that would not admit (stop, do not invent bytes). Do not cargo. Do not commit. Do not edit parent `mod.rs`.
