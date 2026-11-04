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
