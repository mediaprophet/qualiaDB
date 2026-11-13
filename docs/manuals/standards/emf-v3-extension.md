# Q42EMF64 v3: explicit interpretation and original timed PCM

This addendum defines the opt-in v3 envelope implemented by `q42/emf_container`. The repaired [v2 field contract](emf-container-format.md) remains the default writer and remains readable. V1 is accepted only through the explicitly named migration reader. A failed current-version integrity check never retries as legacy. Qualification status and test evidence belong in the implementation review notes; this document defines byte and semantic requirements, not a streaming certification.

## Physical field and version dispatch

The magic remains `Q42EMF64`; the header version at offset 8 is 3. Header, directory rows, cells, and alignment remain 64 bytes. A cell remains E (V/m), B (T), Poynting flux, physical wavelength (nm), optical constants, phase, Stokes values, legacy spectral sigma, and confidence. RGB and PCM are not new cell fields. All v2 mandatory full-chunk BLAKE3 trailer checks, CRC-32C checks, known flags, zero reserved fields, and limits remain in force. The original sixteen-byte directory prefix stays zero and does not claim a Bao proof.

The header and all directory rows precede canonical contiguous field payloads. V3 disallows duplicate `(spatial_hilbert_index, time_slice_index)` pairs and orders spatial keys monotonically; equal spatial keys at different times are valid. Every field payload must end at or before the extension start. Field payload APIs return only cell records, excluding trailers and extension data. `chunk_payload` never returns PCM.

## Extension layout

Immediately after the field payloads, append UTF-8 JSON metadata, zero padding to 64-byte alignment, the PCM section (each track followed by zero padding to alignment), a 64-byte full BLAKE3 trailer, and a final 64-byte locator. All integer locator fields are little-endian. The extension payload is exactly metadata + metadata padding + padded PCM section. The trailer covers those exact serialized bytes; its first 32 bytes are the complete BLAKE3 digest and its final 32 bytes must be zero.

| Locator byte offset | Size | Meaning |
|---|---:|---|
| 0 | 8 | ASCII `Q42EMFX3` |
| 8 | 8 | Absolute extension start |
| 16 | 8 | Exact JSON byte length, excluding padding |
| 24 | 8 | Padded PCM section byte length |
| 32 | 8 | Extension payload byte length |
| 40 | 4 | Track count |
| 44 | 4 | Extension schema version, 1 |
| 48 | 12 | Zero reserved bytes |
| 60 | 4 | CRC-32C of locator bytes 0..60 |

The locator is the final file extent. `extension_start + payload_length + 64 + 64` must equal file length exactly. Every addition, conversion, multiplication, and alignment is checked. Field and extension extents may not overlap. Readers validate the locator, admitted lengths, and complete extension digest before allocating/parsing metadata. Unknown flags, schema versions, enum values, duplicate JSON fields, unknown metadata keys, nonzero padding, inconsistent lengths, and overlapping track ranges are errors.

## Typed metadata (schema 1)

The root has exactly `schema_version: 1`, `field`, and `audio_tracks`. Unknown fields are rejected throughout the model.

`field` contains:

- `colour`: tagged `model` is `spectral_only`, or `rgb_preview` with `primaries` (`rec709`, `rec2020`, `display_p3`), `white_point` (`d65`, `d50`), `transfer` (`linear`, `srgb`, `bt709`, `pq`, `hlg`), `observer: cie1931_2deg`, and finite positive `reference_white_nits` up to 1e9. This declares a derivative interpretation, not stored RGB channels or a completed conversion.
- `timebase`: exact `epoch` (`scale`, signed 64-bit `seconds`, `nanoseconds` in 0..1e9 exclusive), rational positive `ticks_per_second_num/den`, signed `slice_start_tick`, and positive `slice_step_ticks`. Epoch scales are `relative`, `unix_posix` (1970-01-01T00:00:00 UTC, POSIX leap exclusion), or `tai` (1970-01-01T00:00:00 TAI). No scale conversion is implicit.
- `hilbert_order`: explicit 1..8, sufficient for all brick coordinates. Dense decoding and preview lookup use this value, without v2 inference.
- `spectral_semantics`: `authored_wavelength_legacy_sigma`, or `amplitude_centroid_legacy_sigma`. Neither means a sampled full spectrum. The emitter compiler uses the latter because multiple sources collapse to one amplitude-weighted wavelength centroid per cell.

Slice index `s` starts at checked tick `slice_start_tick + s * slice_step_ticks`. Tick `t` is at exact epoch plus `t * ticks_per_second_den / ticks_per_second_num` seconds. Numerator is at most 1e12; denominator at most 1e9. The header's legacy f64 epoch is a projection of exact epoch metadata; agreement allows at most two ULPs of normal f64 conversion rounding, including large epochs. Exact integers/nanoseconds remain authoritative.

## Original PCM tracks

Each `audio_tracks` entry contains `codec: pcm_f32_le`, a typed `spec`, positive `frames`, `pcm_offset` relative to the PCM section, `pcm_bytes`, and `blake3` (32 digest byte integers). Payload size must equal `frames * channels * 4`; offsets are canonical aligned nonoverlapping extents, with zero padding. Per-track BLAKE3 authenticates the original interleaved little-endian f32 bytes independently of the whole extension digest. All samples must be finite. Signed zero and exact finite sample bits are retained.

A `spec` has unique ASCII alphanumeric/underscore/hyphen `id` (1..64 bytes), `purpose` (`recorded`, `sonification`, `reference`), `channels` (1..32), `channel_layout` (`mono`, `stereo`, `discrete`), rational positive `sample_rate_num/den`, signed `start_tick`, `units`, and finite positive `units_per_sample`. Mono requires one channel; stereo requires two. Sample-rate numerator is at most 1e12, denominator at most 1e9, and actual rate at most 1e9 frames/second. This is a storage admission bound, not an audible-band restriction or a playback capability claim.

`units` is tagged `unit`: `normalized_amplitude` requires calibration 1 and samples in [-1,1]; `pascal` preserves calibrated pressure including finite values outside [-1,1]; `arbitrary` requires a nonempty bounded label with no control characters. Frame `i` begins at epoch plus `start_tick * tick_den/tick_num + i * rate_den/rate_num` seconds. Rational arithmetic for all extents is checked with signed 128-bit intermediates. No resampling, peak normalization, hearing-band filtering, or sigma-to-audio conversion occurs during serialization.

## Admission and APIs

Conservative extension limits are 16 MiB whole file, 64 KiB metadata, 16 MiB original PCM aggregate, and 16 tracks. The reader also applies its field chunk count/file limits; callers may supply tighter or explicitly budgeted larger limits before allocation. Dense field decode keeps the existing 1,048,576-cell cap across all time slices.

`EmfContainerWriter::to_bytes()` still writes v2. `to_bytes_v3(field, tracks, limits)` is opt-in. Current `EmfContainerReader` accepts v2/v3. `extension()` provides typed metadata, exact PCM bytes, and individual f32 sample access; `parse_with_extension_limits` admits total bytes before decoding. `find_chunk_at_time` and `lookup_brick_cell_at_time` select exact stored snapshots; legacy spatial lookup wrappers select time zero deterministically. A missing selected time never substitutes another snapshot.

The Hypermedia `compile_emf` operation opts in with `interpretation: EmfFieldInterpretation`, optionally `audio_tracks: [{spec, audio: mono_handle}]` or `[{spec, input: retained_pcm_f32le_key}]`. Exactly one audio source is allowed per track; source handle rate/channels must agree, or original raw PCM uses its fully supplied descriptor. Current field generation stores time zero; the core format can retain multiple snapshots. V3 emitter wavelengths admit 1e-9..1e18 nm, subject to finite stored f32 fields; v2 emitter bounds remain unchanged. `verify_emf` reports version, explicit integrity scopes, typed extension, and track counts/byte totals.

Both spectral slice and DDA raymarch accept `time_slice=0` and report the selected source time. They use the shared `supports_legacy_visible_projection` policy: `spectral_only` permits an explicitly labelled legacy human-visible derivative, including out-of-band false colour/clamping; all declared `rgb_preview` models currently refuse that fixed projection because its gamut/transfer/luminance calibration is not implemented. The physical source remains intact. Inspection/audio access must remain possible independently of render support; no consumer may silently lose tracks or derive v3 audio from visible sigma. Portal render/sonification capabilities are separate from format storage.

This extension provides local full-byte BLAKE3 integrity. It does not provide Bao trees, range proofs, QBDL transport, outer SHA-384 signatures, post-quantum authentication, full-spectrum reconstruction, or a generic electromagnetic-to-colour/audio equivalence.
