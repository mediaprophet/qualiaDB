# Etalon and sensing qualification — 2026-11-01

## Current release candidate

Etalon candidate `9b888fb1bb71f4e3c7e61a72624eb6031df0f8d0` is published on
`dev`, with workspace versions set to 0.0.2. It is not yet tagged as a completed
release. The full [candidate CI matrix](https://github.com/mediaprophet/etalon/actions/runs/41268646883)
is running at this checkpoint. The [candidate Pages workflow](https://github.com/mediaprophet/etalon/actions/runs/41268646914)
passed and the live page's revision was verified.

The earlier candidate `9912f722fa7c17aec44fb90a26ac7b548d6bbbdb` passed all
40 CI jobs. That result does not certify later commits or physical mobile GPUs.

## Browser execution and failure reporting

The device page runs the actual Etalon Rust/WASM module. Its 13 cases include
NQuin wire/parity/filter/sort and WGSL Forge checks on the WASM CPU, plus 11
browser WebGPU cases for error scopes, validation, buffer initialization,
upload/readback, compute dispatch boundaries, changed inputs and rendered pixels.
JavaScript loads the module and presents its results; it does not replace these
library algorithms or assertions.

For candidate `9b888fb1`, hosted browser qualification recorded:

| Lane | Result |
| --- | --- |
| Software WebGPU, Core request | 13 passed, 0 failed, 0 unsupported |
| Software WebGPU, Compatibility request | 13 passed, 0 failed, 0 unsupported |
| Default runner availability | 2 CPU passes, 11 GPU unsupported; GPU qualification FAILED |
| Injected null adapter, Core and Compatibility | Expected unavailable classification and failed GPU qualification verified |
| Injected rejected adapter request | Browser rejection message preserved; failed GPU qualification verified |

Synthetic boundary checks are not GPU execution evidence. The proctor inspects
options arriving at the browser API: Core uses upstream's omitted defaults;
Compatibility is explicit; false fallback selection is omitted. Real GPU
qualification requires every case to pass. Missing adapters, unrun cases,
initialization failures and cleanup failures cannot produce an overall pass.

User-supplied physical-phone reports for `30d6f803` and `9912f722` showed
adapter acquisition unavailable, with 2 CPU passes and 11 unsupported GPU cases.
Both Core and Compatibility were attempted at `9912f722`. No physical mobile
GPU success has been established. A phone result for `9b888fb1` remains pending.
Private device reports have not been uploaded to GitHub.

Astra compared acquisition code against upstream `2375b1cd83da57b4f70acd8db590524cfeb49890`.
Navigator access, context initialization, promise plumbing and nullable handling
had no relevant fork difference; resolved wasm-bindgen package versions matched.
The default option-property difference was removed. The shared upstream error
collapse was corrected with `BrowserRequestRejected`, preserving browser errors
instead of presenting them as an unavailable adapter.

Chromium has a documented PowerVR 25.1 driver restriction, but the phone's driver
has not been established. This is a lead, not a confirmed cause or excuse for a
successful mobile claim. Native Vulkan/GLES still use drivers; WASM CPU execution
must be labelled CPU execution, and the Noop backend is not a computation fallback.

## Canonical QualiaDB integration

The canonical checkout contains 109 migrated implementation files plus preserved
historical plans/reviews. The original-copy inventory is
`sensing-canonical-migration-2026-11-01.json`; it predates integration fixes and is
not a final release digest manifest. The core registers `pub mod sensing` once.
The bridge and inspector use the canonical sibling Etalon checkout rather than
removed worktree paths. Production QualiaDB GPU dependency pins are unchanged.

Core and bridge compilation passed. All 34 bridge tests passed when run outside
the Windows filesystem sandbox; initial hard-link access-denied failures were
resolved by execution permissions, without weakening custody semantics.
Inspector packaging and the additional filtered core suite have not been run.
Do not describe those components as qualified binaries. Unrelated canonical
changes remain untouched. Historical sensing-plan documents remain preserved.

Astra source review covered authority/payload boundaries, canonical sidecars,
uncertainty rounding, custody/revocation/quota/recovery, radio/NFC association and
inspection verification. Its identified integration findings were addressed.
Windows custody head publication may return `Indeterminate` where parent-directory
durability cannot be confirmed; hosts must reconcile that state.

## Native and licensing boundaries

Prior fully green CI exercised native tests, vendor checks/mock tests and the
explicit Linux prepared-allocation and prepared-sensing checks. The Linux adapter
was Mesa software Vulkan, not physical GPU hardware. The sensing provider retains
`ColdUnbounded` until its deployment is qualified. Native DSP/NPU/SDR execution
requires the actual SDK/runtime/firmware/device and authenticated deployment.
Hosted validation uses source and synthetic test data, not sensor captures,
subject records or custody artifacts.

Root CC licensing and inherited notices remain unchanged. All 30 retained SDK
references were checked against recorded lengths and SHA-256 values. Optional
vendor adapters remain in their separate workspace; this is not a claim that
workspace separation removes downstream licensing obligations.

## Published entry points

- [Capabilities](https://mediaprophet.github.io/etalon/capabilities.html)
- [Rust/WASM device tests](https://mediaprophet.github.io/etalon/testing.html)
- [Consumer guide](https://mediaprophet.github.io/etalon/getting-started.html)
- [API](https://mediaprophet.github.io/etalon/doc/etalon/index.html)
