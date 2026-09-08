# Agent Swarm Execution Protocol

## 1. Roles and assignment

| Role | Responsibility |
|---|---|
| Programme integrator | Live source baseline, task claims, shared files, dependency acceptance and release evidence |
| Domain implementer | One bounded task or disjoint subtask, exact write set, tested code and handoff |
| Independent reviewer | Examine behavior/security/bounds and evidence; do not approve solely from author summary |
| Verification owner | Independent vectors/peer harness, platform runs, fault campaigns and reproducible measurements |
| Semantic/crypto/storage reviewers | Review domain-specific authority, interpretations, cryptographic profiles and durability |

A role is not a permanent person or agent identity. Record assigned agent/task, principal authority,
branch/base revision and claim time without creating a global score. No agent can approve its own
security-sensitive completion by renaming its role.

## 2. Before claiming work

Read the repository instructions, [library layout](./library-layout.md), assigned checklist, linked
design and live collision feed if present. Recheck source evidence because previous audits are
snapshots. Existing concurrent work is preserved. The current canonical workspace is the user-selected
checkout; obey its active branch/worktree policy rather than a stale hardcoded path.

The integrator claims exact paths in the live coordination feed or, where absent, this programme's
progress log and registry claim. It records allowed writes, forbidden shared files, base revision,
input interface versions, reviewer, resources and handoff consumers. No overlapping writers.
A task package may be split into child assignments without minting another completion ID: each
child owns explicit files and the parent stays incomplete until all checks and integration pass.

## 3. Readiness and parallel dispatch

A dependency in [task-registry.json](./task-registry.json) must be complete with accepted evidence
before dependent production integration. Exploration, fixtures and implementations against reviewed
interfaces may proceed earlier under a provisional claim, but cannot be marked complete or exposed
as supported until upstream evidence is accepted.

Choose concurrency from disjoint writes, interface stability, host memory/CPU/I/O and build capacity.
One native build or fault campaign may consume most of the host. Reuse configured build caches,
coordinate expensive tests and isolate test ports/bearers/datasets; do not run invasive network
tests on the user's live connection. Ephemeral test domains must be explicitly bounded and owned.

Dependencies are package completion gates, not a requirement that one agent works an entire package
serially. For example, reviewed bearer interfaces allow independent OS adapters, and reviewed source
record contracts allow storage/crypto fixtures in parallel. Shared-interface changes return to the
integrator before consumers update.

## 4. State transitions

`pending -> claimed -> in_progress -> review -> complete`

Any active stage can enter `blocked` with a concrete reason, preserved work, owner and unblock
condition. A failed review returns to `in_progress`; a changed dependency reopens affected consumers.
There is no timeout-based completion. A stale claim is reclaimed only after confirming the old
writer stopped and inspecting its work; lack of a heartbeat is not proof that it released files.

The integrator updates register status and child checks together after evidence review. Do not
check boxes because a stub compiles, a test was written but not run, or a dependency has the right
class name. Conditional choices have explicit decision/evidence records; choosing existing artifacts
instead of QNF can satisfy CORE-04 after its comparison and required guarantees pass.

## 5. Worker loop

1. Accept a [task brief](./templates/task-brief.md); inspect inputs and verify write scope.
2. Establish baseline tests/build for the owned slice and record pre-existing failures.
3. Implement the smallest coherent lifecycle with focused files and explicit errors.
4. Test success, malformed/unauthorized input, resource exhaustion, cancellation and recovery where
   relevant. Use existing meaningful suites; do not manufacture tests that merely mirror constants.
5. Append an outcome to the programme log through the integrator after each step. Record actual
   commands, revision, results, measurements/unknowns and next work; disclose regressions.
6. Produce a [handoff](./templates/handoff.md), including exact changed files, interfaces, evidence,
   unresolved issues and downstream compatibility. Keep the task active until review/integration.
7. Integrator accepts the patch into the authorized integration state, runs crossing-boundary tests
   and checks status only when the common completion standard is met.

No automatic commits, merges, pushes, external messages or service publication follow from a task
brief unless the current implementation assignment authorizes them. Ordinary reversible edits/tests
within the assigned scope proceed without repeated permission requests.

## 6. Interface and decision changes

Record alternatives and evidence in a [decision record](./templates/decision-record.md). Semantic
requirements precede choosing offsets/opcodes. Schema/domain-label changes invalidate relevant
vectors and dependent generation/profile bindings; notify affected owners through the integrator.

Cryptography changes require crypto review and independent vectors. Core ABI/parity migration needs
caller/fixture classification. Evidence retention/disclosure and settlement semantics require their
domain reviewers. Human input is requested only for missing authority/preferences that cannot be
resolved from the accepted design; prepare concrete alternatives and complete independent work first.

## 7. Handoff and recovery

A handoff names source revision plus dirty-file fingerprints if uncommitted, exact patches, public
exports, input/output contracts, ownership, bounds, cancellation and evidence paths. A build from a
different source state cannot certify the handoff. Rebase/reconcile under the repository's branch
rules without overwriting unrelated edits; rerun affected crossing-boundary checks after conflicts.

On agent interruption or context loss, the replacement reads the claim, log, checklist and artifacts;
it does not start over or claim the predecessor's tests were run on new changes. Quarantine uncertain
external effects using the original operation identity. Record superseded/retracted conclusions
rather than deleting inconvenient history.

## 8. Completion standard

A package is complete only when its requested behavior works on the claimed targets, all applicable
child checks have evidence, resource/security invariants pass, public docs reflect actual support,
shared interfaces are integrated and an independent reviewer accepts the result. Unsupported targets
are explicit; they cannot pass by returning empty success. Deferred required work keeps the package
open. Release claims are assessed separately in [the validation matrix](./validation-matrix.md).
