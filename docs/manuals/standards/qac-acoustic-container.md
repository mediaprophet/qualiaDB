# Qualia Acoustic Container (.qac)

Development specification, 13 November 2026. The canonical extension is **.qac**. The updated typed format is the implementation target. This is a new product: former internal development formats, extension aliases and migration paths impose no backward-compatibility requirement. Producers, readers and consumers must adopt the updated format together. External measurements and imported source objects retain exact original bytes and identities.

Native current-format production and publication are implemented. Remaining provider and browser integration is tracked separately; old development sources are not an alternative standard.

## Current wire identity and integrity

QAC v1 uses magic bytes 71 61 63 00 00 00 00 00 and a little-endian u16 version1 at offset8. The common envelope header and each directory row are128 bytes. Payload starts are64-byte aligned, or4096-byte aligned when header bit0 is set. Codec0 is raw. Reserved bytes and alignment gaps are zero.

The full128-byte header/row CRC-32C calculation zeros its own final4 bytes. The BLAKE3 index commitment uses domain bytes QUALIA-FIELD-INDEX-v1 followed by NUL, the header with digest/CRC fields zero, then exact directory and metadata bytes. Each row commits a full32-byte BLAKE3 stored-payload digest. A requested whole chunk is verified before typed numeric access. Integrity does not establish publisher identity, clinical validity or device authority.

Metadata blocks and fields use16-byte headers and8-byte padded values. IDs and field keys are strictly increasing and nonzero. Flags, reserved bytes, primitive counts, finite numeric values and UTF-8 strings are checked and bounded. Unknown critical semantics cannot be silently interpreted.

## Implemented typed signal profiles

| Mode | Profile | Interpretation |
|---|---|---|
| 0x0100 | PCM | Explicit LE f32/f64 or signed16/24/32-bit; time/channel layout |
| 0x0101 | STFT | Full-bin complex f32 or magnitude f32; frame/channel/bin layout |
| 0x0104 | Original bytes | Exact opaque bytes; no inferred signal semantics |

PCM/STFT metadata distinguish digital values unrelated to pressure, raw integer counts, and values already expressed in physical base units. Supported physical pressure uses Pa. Raw counts do not become Pa merely because calibration metadata exists. Physical values and raw-count preservation use scale1 and offset0. Calibration retains lineage and validity intervals; reading derived physical values does not apply calibration again.

Current STFT admission supports a rectangular window, all N bins in stored order, forward negative-exponent convention, forward-unscaled/inverse-1/N normalization and no padding. It validates FFT size, hop, bins, channels, shape and exact cadence. Frequency offsets are signed rational Hz; even-length Nyquist uses the negative representative. They are not inferred physical RF carriers. Magnitudes must be nonnegative; nonfinite interpreted samples reject. Opaque archival remains independent.

Schema210 revision1 records channel order, units per stored sample, stored offset, purpose and physical relation. Channel labels are nonempty, unique and count-matched. Purpose is descriptive, not authority. Schema201 supplies transform conventions;202 supplies calibration lineage;205 identifies shape/model. Descriptors bind these to shared clocks, frames and provenance. Unsupported critical profiles reject typed admission.

## Time and scientific scope

Shared schemas1–5 cover clocks, frames, descriptors, provenance and instants, with typed references and bounded acyclic graphs. Timing keeps epoch seconds/nanoseconds, rational tick rate, start tick and rational sample rate separate. Sample times use checked integer rational arithmetic; no integer-rate rounding replaces authoritative timing.

Acoustic pressure, particle velocity, directional fields and playback are distinct quantities. Directional particle-velocity profiles, CQT, HOA, pressure/velocity energy models, advanced calibration execution and remaining acoustic field modes require implementation and qualification. This initial adapter does not narrow QAC scope.

EMF describes electromagnetic observations across the full spectrum; QAC covers acoustic/mechanical quantities within the shared .10d environment. DICOM and external acquisitions retain source provenance and explicit physical interpretation. Frequency alone does not identify a modality or establish electromagnetic-to-acoustic energy conversion. See the [full-spectrum field contract](full-spectrum-field-contract.md).

## Implemented Rust APIs

- q42::field_envelope::StructuralView::open validates bounded structure/index integrity. Current QAC has the temporary Rust variant name Family::QacV1Candidate.
- q42::field_semantics::admit_shared validates shared metadata, reference closure, provenance and validity using caller scratch/work limits.
- audio::qac::signal::admit validates every supported entry and whole payload, then returns an immutable borrowed SignalView. Scalar/byte budgets precede payload hashing/scanning. Accessors preserve scalar bits and decode explicit LE independently of input alignment.
- audio::qac::signal::encode builds in caller staging, performs typed admission and copies to final output only on success. Every error leaves final output unchanged. Staging may change and must not be published after an error.
- EnvelopeInput::encoded_len/encode_into provide deterministic structural encoding and checked capacity. Structural encoding alone does not establish scientific validity.

Default envelope limits:256MiB file,4MiB metadata,4096 entries/blocks,128 fields per block,16KiB strings and16MiB chunks. Callers select explicit limits and provide graph/output workspace. These are admission limits, not physical band restrictions.

## Publication and integration status

Native publication uses BLAKE3 over all final bytes and content-addressed spectral/audio/{64 lowercase digest hex digits}.qac paths. Reuse requires exact byte equality. QacStorageView accepts only the current envelope by default; obsolete development layouts reject. publish_qac verifies the envelope and every payload digest. produce_pcm and publish_current_signal additionally perform complete typed signal admission before filesystem creation. No old preview-hash alias writer remains. Portal synthetic preview-sheet production has been removed; its actual-data QAC reader and remaining providers still require integration/qualification. Do not add legacy aliases or compatibility facades.

Publication roots must be exclusively controlled. Existing redirect checks do not sandbox concurrent directory replacement. Per-object limits are not total disk quotas; directory-fsync crash durability is not established.

Focused typed tests cover exact bits, unaligned access, rational time, signed FFT offsets/phase, metadata/shape/unit rejection, corruption, pre-hash budgets and unchanged final output on error. Independent common-envelope fixtures reproduce byte-for-byte. These checks do not establish all acoustic providers, complete DICOM support, production cutover or GPU/WASM/Portal qualification. Those remain tracked implementation gates.
