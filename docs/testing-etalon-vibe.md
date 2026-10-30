# Etalon/VibeScript and accelerator harness

Run from the QualiaDB root with PowerShell 7 and Rust installed. The runner
always uses the selected local Etalon checkout, rather than silently testing
the production git pin.

| Mode             | What runs                                                                                        |
| ---------------- | ------------------------------------------------------------------------------------------------ |
| Default          | Lightweight checked VibeScript/discovery, then canonical Qualia CPU and hardware GPU integration |
| `-DiscoveryOnly` | Fresh NPU discovery; no language, GPU or NPU workload execution                                  |
| `-LanguageOnly`  | Lightweight checked VibeScript and NPU inventory; no full Qualia core build                      |
| `-CpuOnly`       | Lightweight language/discovery, then canonical Qualia CPU integration; no GPU or NPU execution   |
| `-NpuOpenVino`   | Adds the explicitly configured ORT/OpenVINO NPU workload to lightweight execution                |
| `-RequireNpu`    | Requires verified NPU execution; discovery alone cannot satisfy it                               |

`-DiscoveryOnly`, `-LanguageOnly` and `-CpuOnly` select distinct scopes.
`-LanguageOnly -NpuOpenVino` tests language and the requested provider without
building the full product. CPU-only excludes NPU workload execution, while
read-only inventory remains separate from execution.

```powershell
pwsh -File scripts/test-etalon-vibe.ps1 -DiscoveryOnly -EtalonPath ../etalon
pwsh -File scripts/test-etalon-vibe.ps1 -LanguageOnly -Offline
pwsh -File scripts/test-etalon-vibe.ps1 -CpuOnly -Offline
pwsh -File scripts/test-etalon-vibe.ps1 -EtalonPath ../etalon -Backend dx12
```

Both integrated and discrete GPUs are eligible. GPU mode rejects software CPU
adapters and verifies any explicitly requested backend. Use `-Backend vulkan`,
`metal` or `gl` on supported hosts, running each backend in a fresh process.
No NVIDIA card or CUDA SDK is required. Missing GPU hardware fails the
requested product GPU check; it is never counted as a CPU fallback pass.

For an explicitly installed modern ORT/OpenVINO NPU provider:

```powershell
pwsh -File scripts/test-etalon-vibe.ps1 -LanguageOnly -NpuOpenVino -RequireNpu -OrtLibraryPath C:/runtimes/onnxruntime.dll -OpenVinoEpPath C:/runtimes/openvino_plugin.dll
```

Supply the actual existing ORT 1.31/API-31 runtime and standalone OpenVINO
plugin library paths. These example filenames are placeholders. The wrapper
passes only these explicit paths as `ORT_DYLIB_PATH` and
`QUALIA_OPENVINO_EP_PATH` to the requested NPU process, clearing inherited
values elsewhere. It does not download providers or choose a CPU/GPU substitute.
The adapter selects a runtime-enumerated NPU device, disables CPU fallback,
checks the result against a CPU oracle and requires execution-profile evidence.
The combined receipt fingerprints the explicit runtime and provider libraries
before and after qualification; changed bytes fail the run. Fingerprinting uses
64 KiB buffers with a 1 GiB file limit and a 60-second local-read budget.
Hardware detection, runtime availability, workload support and execution
verification remain independent outcomes. An absent, unknown or unprobed NPU
does not add an NPU pass. A successful optional provider test does not enable
production graph scheduling on the NPU.

The lightweight target uses `tools/accelerator-harness/src/main.rs` and the shared
accelerator inventory. It checks a VibeScript CPU reference and rejection of an
external clock effect in a Pure cell. Product tests live in
`crates/qualia-core-db/tests/etalon_vibe_harness.rs`; checked scripts go through
the real `VibeHost`, covering detached graph commits/parity, GPU upload and
dispatch/readback, CPU comparisons at workgroup boundaries, changed-input reuse,
zero initialization, lifecycle and invalid requests. They do not attach to a
user's live graph or native authority.

Build manifests and lockfiles are generated under
`target/etalon-vibe-harness`, using local overrides for the nine Etalon workspace
crates consumed by this ecosystem. The lightweight manifest is
`inventory/Cargo.toml`; the product manifest stays at the harness root. Both
reuse the existing harness `target` directory. Production manifests and lockfiles
are not rewritten. The native product profile enables GPU/Forge and Windows
DX12, with CUDA omitted. On Windows, a bundled LLVM archiver is selected when
available unless `AR` is already set. Environment overrides apply only to child
processes.

Each invocation retains a marker-owned directory under `runs/`, containing
bounded logs, independent lightweight/product receipts and a combined
`report.json`. `-ReportPath path/to/report.json` also exports the combined receipt,
creating its parent directory. Failed builds, timeouts and missing product
events retain a failed receipt. Counts are separate for CPU, GPU and NPU:
passes require completed evidence; failures include missing required product
events. Successful phases are retained when a later phase fails. These are
qualified checks, not counts of raw device submissions.

Receipts record both repository HEADs, clean/dirty state, and a working-source
hash covering tracked changes and untracked source files. Hashing is bounded
to 4,096 changed entries and 256 MiB; exceeding that budget is explicitly marked
as an incompletely fingerprinted dirty tree. Source identities are checked
again after execution. Generated lockfile hashes, manifest paths, the shared
target directory and production-lockfile preservation are recorded. A source
or production lockfile change during qualification fails the run.

Defaults are two build jobs, no incremental cache, reduced debug output,
30-minute build deadlines, 120-second execution deadlines and a 4 GiB free-space
preflight before each build. Configure `-BuildJobs`, `-BuildTimeoutSeconds`,
`-TimeoutSeconds` and `-MinimumFreeGiB` as needed. Each output stream has a 16 MiB
log budget; excessive output or less than 512 MiB remaining during compilation
terminates the owned build and fails qualification. The runner never deletes
existing artifacts. `-Offline` requires all selected Cargo dependencies cached.

For CI's optional provider compilation and pure parser tests, use the generated
lightweight manifest and shared target directory from the receipt:

```powershell
$receipt = Get-Content target/qualification/report.json -Raw | ConvertFrom-Json
cargo test --manifest-path $receipt.lightweight_manifest --target-dir $receipt.target_directory --features npu-openvino
```

This does not execute the NPU workload or establish NPU hardware coverage.

The harness supplies ecosystem integration coverage, not WebGPU conformance.
Etalon's `docs/conformance-coverage.md` records the retired Deno CTS runner and
its retained revision/selector inventory. Existing Rust and browser suites
remain required. Multi-identical-GPU identity and binding a measured circuit
winner to a physical adapter remain separate limitations; report the actual
adapter used rather than claiming exhaustive scheduling coverage.

The [phone/browser harness](playground/etalon-vibe-harness.html) runs fixed public
fixtures through canonical Vibe CPU exports and the Rust Etalon browser WebGPU
frontend. Its separate `device-harness` WASM feature and bundle keep diagnostic
exports out of the main portal and playground downloads. The diagnostics bundle
has its own 16 MiB raw / 4 MiB gzip CI limit. Open the deployed harness at
`https://mediaprophet.github.io/qualiaDB/playground/etalon-vibe-harness.html`, tap
**Run checks**, then **Download JSON**. A missing manifest, stale bundle, failed
hash check, unsupported browser or timeout still leaves a downloadable report.
**Copy JSON** is optional and requires browser clipboard permission.

To email diagnostics to yourself, choose **Email report**, select your mail app
from the device share sheet, review the JSON attachment and recipient, and send
it there. File sharing depends on the browser, device and chosen app. Completing
the share sheet does not confirm delivery. Cancelling sharing leaves the run
and downloadable report intact.

If file sharing is unavailable, the harness requests a `mailto:` draft with a
compact summary. Use **Download JSON**, then attach that file manually in your
mail app; the fallback does not attach JSON or include full diagnostics in the
body. **Open email draft** also opens this fallback explicitly. Draft URLs are
limited to 1,800 bytes and are rejected rather than silently truncated. An
optional recipient field applies only to this draft and stays in local page
state: it is not included in the report, persisted or inferred automatically.
The static Pages UI has no SMTP credentials, upload endpoint or automatic mail
delivery. Review the report before sharing, and forward your received email
with its JSON attachment when ready.

This dedicated browser compute bridge is separate from the native product
`Render.gpu_*` path, which remains unsupported on WASM. The browser implements
WebGPU beneath Etalon's Rust browser frontend; the report does not establish
native HAL, native Naga validation, CTS or NPU execution coverage. CPU and GPU
case outcomes remain separate, and unsupported outcomes do not add passes.
Browser NPU hardware/runtime states remain unknown/unprobed with execution
`not_run`.

On HTTPS or localhost, the UI downloads and SHA256-verifies the published JS and WASM identified by
`playground/device-harness-build.json`, imports those verified JS bytes and
initializes directly from the verified WASM bytes. It records both source
revisions, clean/development attribution and asset hashes. A development build's
commit alone does not identify its sources; the tracked diff digest excludes
untracked content. Published glue must be a self-contained module. JS is capped
at 16 MiB and WASM at 128 MiB; build tooling applies its own stricter asset caps.

A dedicated worker loads once, reports CPU cases first, then runs GPU cases.
Loading has a 150-second deadline, CPU checks 20 seconds and GPU checks 60
seconds. Cancellation or timeout terminates the worker and preserves earlier
results. Reports are limited to 1 MiB, with at most 64 console records of 2,048
characters each. Reports include browser identity, adapter information and
public build details; inspect them before sharing. The harness does not upload
diagnostics or access a user graph, private input, account or model. Optional
sharing hands the chosen file to the app selected by the user.

For local desktop or phone use, follow
[local HTTPS/LAN hosting](device-harness-local-hosting.md). HTTPS or localhost is
required for WebGPU and verified assets. A phone loading an ordinary HTTP LAN
address can run CPU diagnostics, with `assets_verified:false` and build
attribution `manifest declared, unverified over insecure HTTP`; its GPU check is
unsupported with execution `not_run`. Source revisions and hashes in that
limited receipt are declarations, not authenticated identities. On-device
WebGPU support still depends on the browser, platform and driver.

Run the report validation/budget checks without a browser or GPU:

```powershell
node docs/tests/device-harness.test.mjs
```

## Qualification on 30 October 2026

The tested consumer revision was `a9668cb2123ad05ef64f7ad4c7b2551c4a5d95b8`,
with all production Etalon dependencies pinned to
`5721ae58285e229a30c0444d41e84f98014912d4`. Both lockfiles use that immutable
source without local patches; ORT remains exactly rc.15 and Tokio 1.54.1.

The [consuming CI run](https://github.com/mediaprophet/qualiaDB/actions/runs/41030925853)
and [Pages deployment](https://github.com/mediaprophet/qualiaDB/actions/runs/41030925879)
record the published implementation. Later documentation-only commits preserve
the tested Rust sources and dependency locks.

| Check                                          | Result                                                                                                        |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Default-feature core library, Rust 1.96.1 MSVC | 10,181 passed, zero failed, three existing external-data/network tests ignored                                |
| Native consuming harness                       | Two CPU cases and one Intel UHD 620 Vulkan case passed; eleven dispatch/readbacks checked against CPU results |
| Windows/Linux consuming CI                     | Product CPU, language/discovery, report/server and optional NPU adapter tests passed                          |
| Pages and WASM release                         | Build, size budgets and deployment passed                                                                     |
| Published HTTPS browser UI, Edge 156           | Two CPU cases and eleven WebGPU dispatch/readbacks passed at a 390×844 mobile viewport                        |
| Report and email controls                      | Asset hashes, exact JSON download, mocked sharing, cancellation and draft fallback passed; no email sent      |
| Fresh Windows NPU discovery                    | DXCore probe completed with no NPU detected; requiring NPU execution failed as expected with zero NPU passes  |

The three ignored library cases require a 440 MiB Monarch shard, an 8+ GiB
Monarch volume set, or a separate real-network integration run. Mobile viewport
emulation is not physical phone coverage. Browser NPU state remains unknown;
native provider execution requires suitable hardware and an installed provider.

The earlier core build failed at linking after external cleanup removed an active
dependency archive. A later combined library/integration build was stopped before
test execution when two compiler variants reached approximately 14 GiB combined
private memory. Completed dependencies were retained. The successful protected
retry used one build job, debug information disabled, optimization level zero,
normal debug assertions and no incremental compilation. Production lock hashes
were unchanged. Use one compiler job for this combined run on a 16-GiB machine:

```powershell
$env:CARGO_BUILD_JOBS = '1'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$env:CARGO_INCREMENTAL = '0'
cargo +1.96.1 test --locked -p qualia-core-db --lib --test etalon_vibe_harness -- --test-threads=1
```

Retired Deno CTS coverage remains absent. These application and Rust results
do not restore JavaScript/WebIDL conformance or prove unavailable hardware.
