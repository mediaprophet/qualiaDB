# Wave 3-E — EMF streaming support (senior)

File-disjoint. **Do not cargo. Do not commit.** Integrator routes `mod emf_stream;` in `bundle/stream/formats/mod.rs` after you stop.

## Gap (R07)

`EmfDirectoryView` (v2) and `EmfV3DirectoryView` (v3 interpretation/PCM) admit a directory. The mux has no version/profile capability matrix, no region selection that feeds `Schedule`, no `present[]` id from a successful admit, and no rule that unknown critical interpretation stays **opaque archival** rather than a validated field. Review §6 requires concurrent EMF **region** retrieve with 10D/frame/calibration dependencies, truncated later planes refused, last digest-byte corruption refused, and EMF not reduced to visible colour.

This slice is the **streaming EMF support** used by M/F/H. It does not edit codec owners (`q42/field_*`, `q42/emf_container/**`, `render/gpu/emf_pipeline.rs`, portal EMF, etalon-render).

## Read first (read-only)

- Review R07 and §6 items 3–6, 9
- `bundle/stream/formats/{emf,emf_v3,range}.rs`
- `q42/emf_container/header.rs` version constants (`EMF_CONTAINER_VERSION`, `EMF_EXTENDED_VERSION`)
- `bundle/stream/plan/{inventory,readiness}.rs` `EntryFamily::Emf`, `Readiness`
- Candidate fixtures: `q42/field_envelope/fixtures/candidate/emf_candidate-{opaque,aligned4096,two-entries}.emf`

Do **not** edit those files. Do **not** copy directory/chunk parsers into this module — call `EmfDirectoryView::admit` / `EmfV3DirectoryView::admit` / `selected_chunk`.

## Allowed writes

- NEW `crates/qualia-core-db/src/bundle/stream/formats/emf_stream.rs`
- NEW `crates/qualia-core-db/src/bundle/stream/formats/emf_stream_tests.rs` (path-declared from `emf_stream.rs`)
- `docs/.../qbdl-streaming/evidence/swarm-wave3-e-summary.md`

Do not edit `emf.rs`, `emf_v3.rs`, `emf_v3_tests.rs`, `formats/mod.rs`.

## Behavior

```rust
pub enum EmfProfile {
    DirectoryV2,
    InterpretationV3,
    OpaqueArchival,
}

pub struct EmfStreamView<'a> { /* wraps admitted v2 or v3 view; no heap in admit if possible */ }

impl<'a> EmfStreamView<'a> {
    /// Dispatch on header version bytes, not filename. Malformed recognized magic: no fallback.
    pub fn admit(
        header: AuthenticatedRange<'a>,
        directory: AuthenticatedRange<'a>,
        tail: Option<AuthenticatedRange<'a>>, // required for v3
        limits: FormatLimits,
        scratch: &mut [PayloadInterval],
    ) -> Result<Self, FormatError>;

    pub const fn profile(&self) -> EmfProfile;
    pub fn header(&self) -> &EmfContainerHeader;

    /// Planner `present` id. Successful structural admit of a mandatory Emf entry.
    pub const fn ready_for_plan(&self) -> bool; // false when OpaqueArchival

    /// Selected brick only. Not a complete field-volume receipt.
    pub fn select_region(
        &self,
        chunk_index: usize,
        payload: AuthenticatedRange<'a>,
    ) -> Result<EmfChunkView<'a>, FormatError>;
}
```

Rules:

- v2 header → `EmfDirectoryView::admit`. v3 header → `EmfV3DirectoryView::admit` (tail required; missing tail → `Unsupported` / `Bounds`, not silent v2).
- Unknown version / unsupported critical extension → `EmfProfile::OpaqueArchival` **or** `FormatError::Unsupported` if the field is mandatory-critical. Archival profile is never a validated measurement and never a GPU display cell.
- `select_region` on a truncated payload (length &lt; directory row `byte_length`) → `Bounds` / `Integrity`. Later plane missing → refuse, not a shorter valid brick.
- Last BLAKE3 trailer byte flipped → `Integrity` (existing `verify_blake3_trailer` in `selected_chunk`).
- Do not map spectral identity to sRGB / portal `DisplayEmfCell`. This module does not call `emf_pipeline` or portal state.
- Geometry/frame/calibration **dependencies** are planner edges (F/M). This module may expose optional declared interval/timebase **only if the authenticated header/extension already has them**. Do not invent a clock.

`present` helper for M/F (no Schedule ownership):

```rust
pub fn emf_present_id(entry_id: u32, view: &EmfStreamView<'_>) -> Option<u32> {
    view.ready_for_plan().then_some(entry_id)
}
```

Many EMF entries: tests admit **two** distinct candidate files as two QBDL entries. `EntryFamily::Emf` is a per-entry tag.

## Tests (write, do not run)

1. **`v2_candidate_admits_directory_profile`** — `emf_candidate-opaque.emf` (or aligned) → `DirectoryV2`, `ready_for_plan`.
2. **`v3_admits_interpretation_profile_when_fixture_is_v3`** — if a v3 fixture exists in-tree, use it; if only v2 candidates exist, assert v2 path and a constructed/skip note in the summary. Do not fabricate a v3 tail.
3. **`version_byte_mismatch_is_unsupported_not_v2_fallback`**.
4. **`select_region_returns_authenticated_chunk`** — `selected_chunk` records match payload; trailer verifies.
5. **`truncated_chunk_payload_refuses`**.
6. **`last_digest_byte_refuses_integrity`**.
7. **`two_emf_entries_are_distinct_regions`** — two candidate files, two admits, two `select_region` results, identities differ.
8. **`opaque_archival_is_not_ready_for_plan`** — unknown-critical / unsupported profile → `ready_for_plan == false` (or admit error if you treat it as refuse). Never a “valid empty field”.

Match on `Result`. No Debug derive on success types for `unwrap_err`.

## Stop

Summary: profile enum, fixtures used, whether a live v3 fixture existed, tests written **not run**. Do not cargo. Do not commit. Do not edit `formats/mod.rs` or codec owners.
