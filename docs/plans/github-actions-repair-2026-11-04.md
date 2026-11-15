# GitHub Actions repair — 2026-11-04

## Diagnosis — done

Inspected the latest branch runs and failed job logs at source commit
`422697db29d34ea9091efb15f8e6d81d755dd8c0`, plus the latest Etalon integration
and comparative benchmark failures.

- Pages run `41814173241`: source attribution rejects current Etalon declarations.
- WASM run `41814173206`: six missing-import errors in capability profile checks.
- Etalon run `41811101039`: Windows product build mixes local and git Etalon types;
  Linux language/discovery job passes.
- Benchmark run `41811100948`: computation steps complete, but Schema.org result
  publishing receives GitHub's remote `Internal Server Error`. Its artifact upload
  is then skipped. Some comparison engines also report runtime failures inside the
  resulting data; workflow success alone does not establish their success.

Three independent implementation lanes cover Pages identity, WASM feature gates,
and Windows harness dependency overrides. The coordinator owns benchmark publishing,
review and final integration. No new test measurements yet. Human input: none this
step. Next: repair root causes, run focused regression checks and verify new CI runs.

## Implementation and focused regression checks — partial

Pages now captures each production Etalon declaration and resolved package identity,
including exact manifest/lock hashes, separately from the standalone harness revision.
The existing browser/server tests and new identity regressions pass: 19 tests, zero
failures. Actual source capture covers 19 declarations and 28 locked packages.

The Windows harness derives local overrides from all reviewed workspace members;
its harness pin is aligned with the existing production revision `0f929b49`.
Focused PowerShell tests pass. Metadata validation resolves 531 packages with no
remaining git-sourced Etalon packages and 68 workspace overrides. The local offline
fresh solve encountered yanked versions in the cached registry; copying the existing
production lock into the isolated validation fixture allowed this graph check.
The production lock was not changed. Actual CPU harness execution remains pending.

Benchmark publishing retries pushes up to three times with a fresh rebase before
each attempt; upload steps retain existing result files even after publishing
fails. Both real workflow shell loops pass three simulations each: immediate
success, one failed push followed by success, and failure after all three attempts.
The newest benchmark run `41814173195` independently confirms the same GitHub remote
Internal Server Error. Retries cannot guarantee recovery from a sustained outage.

Human input: none this step. Next: complete real Rust compilation/execution, review
all changes, publish and inspect replacement workflow runs.

## Published repair and hosted verification — partial

Published `bd25ae957c3b708a847a345b0acd50e407ab18e0` to the active
`0.0.40.12-etalon-migration` branch. All five local WASM checks pass using
Rust 1.95.0: ontology MCP, portal, logic, scientific and LLM profiles.

Hosted Etalon/Vibe run `41816199255` passes on Windows and Linux. Windows reports
`requested_checks_complete`; its optional adapter suite passes 10 tests. QDNF
run `41816198693` passes on both platforms. WASM run `41816198904` passes its
smoke/profile job and optimized playground build. Pages `41816198600` passes
source capture, Node checks, WASM profiles, portal/device builds and final asset
attribution. Benchmark `41816198892` passes the main job including result commit
and push; Schema.org is still running. Remaining release/deploy status is pending.

The required local default-feature core suite was built with Rust 1.96.1,
one build job, debug info/incremental disabled, and one test thread. Its 10,173
tests are still running. Two physical Qwen tests and three context-ring tests
have failed. The three exact ring tests separately pass on both the repaired
binary and a pre-change binary (six isolated passes); their source is unchanged
and depends on global epoch state. Physical Qwen tests autoactivate when the
hardcoded local model exists. Full-suite diagnostics remain pending: this is
not a claim that the broad native suite passes.

Human input: none this step. Next: finish hosted release/deployment, Schema.org
publication and native-suite failure classification.

## Native suite result — complete, not green

The default-feature suite finished in 725.20 seconds: **10,164 passed, 6 failed,
3 ignored**. Compilation took 7 minutes 24 seconds with the existing target cache.

- Two `physical_qwen_nvfp4` cases find the workstation's external model, then hit
  the existing fail-closed `qwen35moe` architecture rejection. One consequently
  receives `MissingTensorIndex` during prefill and produces no token. The guard
  and physical-model tests were not changed by this CI repair.
- `context_ring_push_pop`, `producer_cycle_with_global_substrate`, and
  `topology_draft_ring_roundtrip` fail their publication assertions after other
  tests alter shared epoch state. All three pass individually on both current
  and pre-change binaries. Their sources are unchanged.
- `cscp_control_then_qsession_on_local_tls_h2` fails with Windows socket error
  10048 (address already in use). Its exact isolated rerun passes in 0.27 seconds.

No assertion, architecture gate, or test was disabled to hide these results.
WASM Release `41816198904` is now entirely green, including both optimized
builds, size gates, viewport checklist and packaging. Human input: none this step.

The pre-change native test executable independently reproduces both physical
Qwen failures with the same architecture rejection (two tests, 11.83 seconds).
This confirms those two failures predate the workflow repair.

## Hosted completion — done

All five workflows triggered by repair commit `bd25ae957` completed successfully:

| Workflow | Run | Result |
| --- | --- | --- |
| Pages | [41816198600](https://github.com/mediaprophet/qualiaDB/actions/runs/41816198600) | Build and deployment succeeded |
| WASM Release | [41816198904](https://github.com/mediaprophet/qualiaDB/actions/runs/41816198904) | All three jobs succeeded |
| Etalon and VibeScript | [41816199255](https://github.com/mediaprophet/qualiaDB/actions/runs/41816199255) | Windows product CPU and Linux discovery succeeded |
| QDNF MSVC | [41816198693](https://github.com/mediaprophet/qualiaDB/actions/runs/41816198693) | Windows and Linux succeeded |
| Comparative Benchmarks | [41816198892](https://github.com/mediaprophet/qualiaDB/actions/runs/41816198892) | Main and Schema.org jobs, result pushes and artifact uploads succeeded |

Benchmark result commits `7bbc63bd9` and `dd43999cb` are preserved. Workflow success
does not imply that every external comparison engine succeeded under its memory
ceiling, nor that the six separate local native-suite failures above were repaired.
No mobile-device runtime qualification was performed by this CI repair.

Human input: none. The requested GitHub Actions repair and hosted verification
are complete. The native-suite findings remain explicitly recorded for their
own inference/test-isolation workstream.
