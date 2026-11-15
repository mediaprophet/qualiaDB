# EMF v2/v3 admit + QAC declared interval

File-disjoint. **Did not run cargo. Did not rustc. Did not commit.** Synth MCP was not touched. Parent `formats/mod.rs` was not edited. Codec-owner trees were not edited (`emf.rs`, `emf_v3.rs`, `qac.rs`, `qac_tests.rs`, `audio/qac/**`, `q42/field_*`, `q42/emf_container/**`, portal/pipeline EMF).

Tests were **written, not run**.

## Files written

- EDIT `crates/qualia-core-db/src/bundle/stream/formats/range.rs` — `AuthenticatedRange::subrange` only
- EDIT `crates/qualia-core-db/src/bundle/stream/formats/emf_stream.rs` — inner enum + v2/v3 split-range admit
- EDIT `crates/qualia-core-db/src/bundle/stream/formats/emf_stream_tests.rs`
- EDIT `crates/qualia-core-db/src/bundle/stream/formats/qac_stream.rs` — timed-flag `declared_interval`
- EDIT `crates/qualia-core-db/src/bundle/stream/formats/qac_stream_tests.rs`
- this summary

## `AuthenticatedRange::subrange`

Parent range is already authenticated. A subslice of `self.bytes` at relative offset stays authenticated (`selected` copied, `offset` becomes absolute). `Bounds` on overflow or `get` miss.

Whole-entry `EmfStreamView::admit` still requires `range.offset()==0` and `range.bytes().len() as u64 == logical_length` (`Extent` otherwise). Then split header `(0,64)` and directory from header `directory_offset` / `chunk_count`.

## `EmfInner`

```
enum EmfInner<'a> {
    EnvelopeV4(StructuralView<'a>),
    DirectoryV2(EmfDirectoryView<'a>),
    InterpretationV3(EmfV3DirectoryView<'a>),
}
```

Unknown-critical v4 stays `EnvelopeV4` inner with `EmfProfile::OpaqueArchival` so `select_region` / `structure()` still work. No empty `Archival` variant (would be unused).

`structure()` is now `Option<&StructuralView>` — `Some` only for envelope v4. Mixed/HTTP callers do not use it. Admit signature is unchanged (`range`, `field_envelope::Limits`).

## Dispatch (magic/version bytes, not filename)

| bytes | result |
| --- | --- |
| magic ≠ `Q42EMF64` | `Family` |
| version 4 | `StructuralView::open` → EnvelopeV4 or OpaqueArchival |
| version 2 (`EMF_CONTAINER_VERSION`) | split ranges → `EmfDirectoryView::admit` → DirectoryV2 |
| version 3 (`EMF_EXTENDED_VERSION`) | split header/directory/tail → `EmfV3DirectoryView::admit` → InterpretationV3 |
| version 1 (`EMF_LEGACY_VERSION`) | `Unsupported` (no silent fallback) |
| other version | `Version` |

No silent v2 fallback for a failed v4 open. A v4 envelope with the version byte poked to 2/3 is **not** opened as EnvelopeV4; brick admit then refuses (`Format(Integrity)` / `Extent` / `Geometry`).

v3 tail start is the footer `field_end` at the last 64 bytes (offset 8). `parse_extension_tail` remains the validator; this slice only locates the range.

`FormatLimits` is mapped from envelope `Limits` (`file_bytes` / `entries` / `metadata_bytes` / `chunk_bytes`), with `max_rows` capped at scratch `4096` (same cap as `EmfDirectoryView`). v3 `EmfExtensionLimits` keeps `conservative()` metadata/audio/track caps and uses envelope `file_bytes` for `max_file_bytes`. Scratch is stack `[PayloadInterval; 4096]` inside admit.

`select_region` on v2/v3 calls `selected_chunk` with a payload `subrange` located from the directory row. `declared_interval` on v2 is `None` (no timed flag; `reference_epoch_s` is not a tick). v3 returns `(slice_tick(time_slice_index), slice_step_ticks)` from the admitted extension timebase when the chunk exists.

`FormatError` mapping: `Unsupported`/`Limit`/`Bounds` → `Unsupported`/`Limit`/`Extent`; remainder `Format(other)` so truncated directory can be `Extent` or `Format(Geometry)`.

## QAC `declared_interval`

Public `StructuralView::entry` has `flags()` / `start_tick()` / `time_count()`. First complete-entry row (`entry(1)`):

- `flags() & 2 == 0` → `None` (not epoch zero). Opaque and two-entries fixtures are this case.
- timed flag set → `RationalInterval::declared(start_tick, 1, start_tick+time_count, 1)`. Denominator 1 is tick units, not an invented seconds clock.

`qac_candidate-pcm-signed-zero.qac` has flags `0x3`, `start_tick=0`, `time_count=4`. Test asserts `Some` matching those fields. Owner fixtures were not edited.

## Tests written (not run)

EMF:

1. Existing v4 tests still typecheck; `structure()` call sites match `Option`.
2. `v2_brick_admits_directory_profile` — `EmfContainerWriter` 1+2 cell brick (512 B ≤ `LEAF_BYTES`, one leaf; no covering multi-leaf span needed). Profile `DirectoryV2`.
3. `version_byte_mismatch_is_not_treated_as_v4` — v4 envelope version 2/3 is not EnvelopeV4; version 1 `Unsupported`; 99 `Version`.
4. `truncated_v2_directory_is_extent_or_geometry` — 80/100-byte whole payloads.
5. `select_region_on_admitted_v2_chunk` — chunk 0/1 records match brick payload (no trailer).

QAC:

1. `declared_interval_absent_is_not_epoch_zero` — opaque + two-entries, timed flag clear, `None`.
2. `declared_interval_matches_timed_pcm_entry` — pcm-signed-zero `Some` matching `start_tick`/`time_count`.

Match on `Result`. No `Debug` derive on the stream views. No `unwrap_err`.

## Not done

- cargo / rustc / commit
- parent `formats/mod.rs`
- live v3 admit success test (v3 path is compiled; version-byte test refuses v4-as-v3). A covering `max_leaves > 1` span was not needed because the v2 writer fixture fits one leaf.
