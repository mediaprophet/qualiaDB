# Swarm execution, capability routing and cost control

Revision 1, 12 November 2026. Extends the parent [swarm protocol](../swarm-protocol.md). This document defines future assignments; it does not launch agents, authorize API spending or mark networking implemented.

## 1. Assign work by failure cost and interface stability

| Tier | Suitable assignments | Example model selection if available | Boundaries |
|---|---|---|---|
| Support | Source inventory, link/schema checks, fixture transcription from frozen vectors, documentation crosswalk, mechanical test plumbing | `gpt-6-luna`, low/medium reasoning | No independent security interpretation, unsafe memory, wire design, accounting semantics or cross-repository edits |
| Senior implementation | One codec adapter, HTTP/WSS client, storage adapter, bounded scheduler implementation against accepted interfaces | `gpt-6.1-sol`, medium/high reasoning; `gpt-6.1-terra` for scoped browser/contained code work after a pilot | Escalate ambiguous lifetimes, crypto boundaries and protocol changes; independent review required |
| Expert design/review | Cryptographic composition, PQ authority, proof format, concurrent cancellation, distributed escrow, numerical equivalence, cross-owner integration | `gpt-6-astra`, high reasoning; highest suitable available model if unavailable | Do not substitute model confidence for independent vectors, actual tests or external expert review |
| Integrator | Claims, interface versions, dirty-state preservation, evidence acceptance, release graph and shared files | Senior model by default; expert for disputed cross-domain decisions | One logical owner of shared state; cannot self-approve its own sensitive implementation |

The tier choices align with [official OpenAI model-selection guidance](https://developers.openai.com/api/docs/guides/model-selection) and the [model catalogue](https://developers.openai.com/api/docs/models), checked on 12 November 2026. The task-to-tier mapping is this programme's engineering judgment. Names are routing examples from the available agent catalogue, not guaranteed availability, price rankings or benchmark claims. At dispatch inspect actual available models, reasoning modes, tool access and rates. If a lower-cost model lacks repository/tool competence, use the senior tier rather than repeatedly repairing its work. Reassess model choice with a small representative pilot. Do not benchmark by sharing real secrets or protected data.

Optimize **cost per independently accepted work package**, including rereads, review and rework. A cheap agent that creates an incompatible wire format is expensive. Use support agents after schema freeze for parallel fixture families; use an expert early where a mistaken abstraction would invalidate many downstream tasks. Keep implementation and reviewer contexts independent enough for the reviewer to find omitted cases.

## 2. Initial team and dispatch schedule

Start with **one integrator plus up to three active child agents**. Review occupies a worker slot; it is not extra unbounded concurrency. Use at most one expensive native build/fuzz/platform campaign on the laptop at a time. Long provider tests use isolated ports/files/bearers and do not change the user's live network. Child spawning requires the integrator's assigned scope; no recursive swarm fan-out by default.

First dispatch QBS-00 to a senior inventory owner with a support subtask only if file lists are disjoint. The integrator accepts the baseline, then QBS-01 uses expert architecture/crypto review. Once interfaces are accepted, run QBS-02 (senior parser), QBS-03 (expert proof envelope) and QBS-05 (expert tree contract) in parallel. Rotate a completed author out to admit an independent reviewer; continue QBS-17 fixture work with support/senior agents against frozen specifications. Later waves follow the registry, not a calendar promise.

QBS-15 browser work waits for HTTP and WSS contracts. QBS-27 WebRTC follows R1 and gets a dedicated networking/privacy reviewer; it is not a small extension of the WebSocket implementation. QBS-19 escrow and QBS-20 solver equivalence require expert reasoning and domain review even if routine storage/tests are delegated to cheaper children.

## 3. Effort budgets and escalation

Registry caps are **planning limits per lead attempt**, not expected consumption: 40,000 tokens for a senior lead and 60,000 for an expert lead. Start support subtasks at 8,000–12,000 tokens, focused independent reviews at 12,000–20,000, and split tasks before granting larger contexts. Caps include investigation and output; avoid repeatedly passing the entire repository history. Use scoped source excerpts plus exact source links and immutable interface artifacts.

Budget each wave from actual model input/cached-input/output rates and observed pilot usage. Track model, reasoning level, input/output tokens, elapsed time, build compute and accepted outcome. Do not infer dollar prices from model names. Reserve roughly 25–35% of the wave's planning allowance for independent review, integration and adverse-case verification; adjust from evidence. The registry's aggregate maximum is not an authorization or forecast to spend it all.

At approximately 50% of a task allowance, require concrete evidence: interface decision, reproducible failing case or tested implementation slice. At 80%, emit a handoff and split/rescope remaining work. A budget limit never means completion. After one unsuccessful repair of a misunderstood invariant, escalate to the relevant expert with a minimal reproduction; do not run repeated lower-tier retries. A flaky environment/build issue goes to the integrator, not an automatic higher-cost reasoning loop.

## 4. Claims and file isolation

Before dispatch record: package ID; agent/task identity; base commit plus dirty-file fingerprints; exact allowed files; forbidden shared files; interface version hashes; dependencies/evidence; model/reasoning; effort/time limits; test resources; reviewer and downstream consumer. Broad registry paths are proposals and MUST be narrowed into a claim. If the module already exists, reuse its owner and read source; the proposal does not license a duplicate implementation.

Use isolated worktrees under the active repository policy where practical. Otherwise enforce exclusive file claims in the shared workspace. No two writers own the same file. `mod.rs`, common enums, Cargo manifests/locks, registries, task statuses and interface schemas are integrator-owned; workers submit requested changes as patches. Coordinated child claims can share a parent package without inventing independent completion IDs. New modules stay focused and normally below 500 lines; directory `mod.rs` files route exports. Preserve the 48-byte NQuin ABI and existing allocation tiers.

Create `codex/` branches only when the implementation session authorizes branch work. Do not commit, merge, push, send external messages, deploy services or replace a live MCP binary merely because a task brief exists. Reversible scoped editing/testing proceeds without repeated approval once assigned. Actual network spend/provider access requires the task's existing authorization; planning a test is not permission to submit a paid QPU job.

## 5. Dispatch template

```text
Package: QBS-xx; child scope if any
Objective: observable behavior and refusal conditions
Release/profile: R1/R2/R3; exact interface/version hashes
Source: repository/base commit; dirty-file fingerprints
Read first: repository instructions; architecture sections; owner source; accepted vectors
Allowed writes: exact paths
Forbidden writes: shared registries/manifests/other agents' files
Prerequisites: accepted evidence IDs (or explicitly provisional fixture-only status)
Agent: capability tier, available model, reasoning level, effort cap
Contracts: input/output ownership, bounds, errors, cancellation and policy generations
Acceptance: specific checks from registry plus relevant qualification gates
Resources: target directory, ports, fixture budgets, build reservation
Reviewer: different agent/domain owner; external review gate if needed
Handoff: changed files, evidence, risks, unresolved work and next action
Stop/escalate: scope/interface conflict, secret exposure risk, failed invariant, budget boundary
```

Support agent example: transcribe independently supplied Merkle fixtures into a single test-data file, validate their schema and report inconsistencies; do not decide the tree algorithm. Senior example: implement HTTP 206/200/416 handling in its adapter with hostile-server tests; do not change proof acceptance. Expert example: review nonce/key/context lifecycle and attack the crash/resume construction; do not sign off by reading only the author's summary.

## 6. Evidence and completion workflow

Use `pending → claimed → in_progress → review → complete`; `blocked` names a concrete external prerequisite and preserved work. The integrator alone updates canonical statuses. A dependency change invalidates affected evidence and reopens consumers. Reclaim stale claims only after confirming the writer has stopped; lack of heartbeat is not evidence of release.

Each handoff includes exact patch/base state, new exports, threat/bounds assumptions, actual commands and exits, test counts, allocation/stack measurements where relevant, platform/capability skips, independent fixtures, unresolved defects and downstream migration. Tests written but not run are marked not run. A second binary importing the same verifier is not an independent proof implementation. Reviewers read code and exercise failure behavior; they cannot approve their own work under another role name.

The integrator runs crossing-boundary tests after combining changes and after conflict resolution. Save evidence in an owned bounded run directory with source fingerprint, then link it from the package. Temporary artifacts use existing lifecycle rules, not loose root-level logs. Checkpoint before travel or interruption: claims, partial changes, pending tool/test processes, evidence and next command. Resume from that checkpoint; do not restart the audit or inherit test claims onto changed code.

## 7. Release authority

Agent review is necessary but not sufficient for cryptographic composition, distributed financial invariants or safety-critical numerical claims. Obtain the qualification document's independent review before enabling those features. Prepare concrete artifacts and evidence first; request human decisions only for genuinely unresolved authority or policy choices. Optional research does not block unrelated R1 work, and unresolved mandatory work cannot be relabeled optional to achieve a completion date.
