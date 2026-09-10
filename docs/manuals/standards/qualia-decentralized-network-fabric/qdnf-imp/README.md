# QDNF Implementation Programme

**Status:** Implementation plan; all implementation packages start pending
**Date:** 2026-09-07
**Outcome:** An independent Qualia Peer Runtime replacing libp2p, built on the QualiaDB core,
Webizen Sentinel, semantic authority, bounded cells and recoverable services.

## 1. Entry point

For the implemented `0.0.37` baseline, start with the [implementation review and enhancement plan](0.0.37-enhancement-plan.md), the [sensitive-operations blueprint](sensitive-operations-blueprint.md) and the [advanced algorithm recipes](advanced-algorithm-recipes.md). These add evidence-based priorities, junior-developer assignments, hostile-environment medical/biometric profiles and mandatory response-marking inheritance. They do not mark the original packages complete.

This package expands [P0–P21](../implementation-conformance.md) into 30 claimable task packages
with 490 individually numbered child checks across eight workstreams. Read the [design suite](../README.md) and
[verified core findings](../core-memory-and-parallel-networking.md) first. Plan completion does not
implement networking, certify cryptography or change memory constants.

| Document | Use |
|---|---|
| [Dependency roadmap](./dependency-roadmap.md) | Order, parallel work, milestones and P0–P21 crosswalk |
| [Swarm protocol](./swarm-protocol.md) | Claims, write scopes, interface changes, review and handoffs |
| [Library layout](./library-layout.md) | Single-purpose modules, source owners and decomposition rules |
| [Task register](./task-registry.json) | Canonical dependencies, ownership roles and implementation status |
| [Validation matrix](./validation-matrix.md) | Required test evidence, platform matrix and completion standard |
| [Progress log](./progress-log.md) | Dated outcomes, measurements, limitations and next actions |
| [Internet peer connectivity architecture](./internet-peer-connectivity-architecture.md) | Incremental SocialWebNet: retain WireGuard; A–E. Local production path on `0.0.38`; not Internet. |
| [QUIC-native connectivity research](./quic-native-connectivity-research-2026.md) | 32-source report: published vs experimental vs proposed; DCUtR/Pkarr/Holepunch corrections |
| [Capability-scoped connection fabric](./capability-scoped-connection-fabric.md) | Architecture note for CSCP. Local supervisor in-tree; not MASQUE/noq/Internet/RFC. |
| [draft-webcivics-cscp-00](./draft-webcivics-cscp-00.md) | Proposed IETF Internet-Draft: purpose-bound connect, exclude-then-rank, leases, evidence, receipts. Working document, not an RFC. |

## 2. Detailed checklists

| Workstream | Packages | Checklist |
|---|---|---|
| Foundations | FND-01–03, QA-01 | [Baseline, semantics, interfaces, harness](./workstreams/00-foundation.md) |
| Core | CORE-01–04 | [Webizen, large graphs, storage, format decision](./workstreams/01-core-storage-webizen.md) |
| Cryptography | CRY-01–02 | [Primitives, key ownership, PQ protocol](./workstreams/02-crypto.md) |
| Native network | NET-01–05 | [QFrame, QLink, QRoute, QResolve, QSession](./workstreams/03-native-network.md) |
| Runtime | RT-01–03 | [Leases, cells, parallelism, facade](./workstreams/04-peer-runtime-cells.md) |
| Semantics/services | SEM-01, SVC-01–03 | [Fabric, contracts, sync, subscriptions, custody/compute](./workstreams/05-semantic-services.md) |
| Commons/evidence | ECO-01–02, EVD-01–02 | [Accounting, roles, electronic evidence](./workstreams/06-roles-economics-evidence.md) |
| Delivery | OPS-01–02, QA-02, REL-01 | [Compatibility, migration, qualification, release](./workstreams/07-release.md) |

## 3. Invariants for every assignment

- Webizen is the Sentinel. Its 42 MiB table allocation does not establish one arena per cell or
  complete-pass enforcement. Implement explicit ownership and account for all referenced state.
- Ordinary cells have an admitted ceiling at most 512 MiB. Hosts can run multiple cells, with
  aggregate memory/work/I/O reservations. Exceptional LLM/similar execution remains explicitly
  classified and isolated; it cannot consume essential network control reserves.
- Tier-1/ABI/Sentinel paths use caller-owned fixed storage, including construction and errors.
  Existing vectors are repair boundaries, not exemptions. Apply actual repository Tier-2 rules
  only where appropriate; never relabel evaluators as cold to evade allocation measurements.
- Preserve the 48-byte NQuin ABI. Reconcile parity, context-bound caches and eviction semantics.
  Short hashes, cached triples, insertion counts and signatures alone cannot authorize delivery.
- Reuse core graph, range, artifact, indexing and transaction owners. Persisted datasets need not
  fit in RAM. Evaluate QNF; neither large graphs nor PQ proofs automatically require a new format.
- Keep entity, claim, handle and instrument planes distinct. No universal NaturalAgent join key,
  reputation score, or authority from similarity, wallet possession or nominal signature count.
- Energy, time and typed compute remain separate. Router/provider roles have resource, funding
  and contribution lifecycles; payment rails are optional and cannot enlarge consent or budgets.
- Diagnostic expiry, operational audit and preserved evidence are separate lifecycles. Holds
  protect required dependencies; a digest cannot recover deleted source material.
- Replacement acceptance runs with libp2p absent. Explicit compatibility carriers and the LIG
  cannot satisfy that requirement by wrapping the previous stack.

## 4. Tracking

Mandatory application profiles include [finite project compensation](../finite-project-compensation.md)
and [socially defined protection](../socially-defined-protection.md). Coverage is explicit:
SEM-01.17–24; ECO-01.17–20; ECO-02.17–30; NET-04.31–34; NET-05.17–20;
EVD-01.17–18; EVD-02.17–18; OPS-02.13–16; QA-02.17–20.
These cover contributor remuneration, personal/humanitarian exemptions, corporate delegation,
atomic finite pay-down and private protective contact/recovery. Every check remains pending.

[Known-peer clinical exchange](../socially-defined-protection.md#61-known-peer-confidential-clinical-exchange)
adds SEM-01.25–26, NET-05.21–24, OPS-02.17–18 and QA-02.21–22: private patient/clinician pairing,
standing care permissions, selected medical disclosure, declared decryption endpoints and
freshness-gated delivery without public patient discovery.

The register owns package status/dependencies; Markdown owns child checks. Keep checks unchecked
until supported by recorded evidence. Completing a package requires all child checks, dependency
acceptance and the shared definition of done. Writing a test name or a draft API is not completion.

Use [task briefs](./templates/task-brief.md), [handoffs](./templates/handoff.md),
[evidence results](./templates/evidence-result.md) and [decision records](./templates/decision-record.md).
The integrator owns registry and progress-log writes; workers propose status transitions.

The user requested this plan, not automatic execution of every future task or external deployment.
Dispatch bounded assignments when implementation is authorized. Do not launch 30 agents simply
because there are 30 packages. Concurrency follows available resources and disjoint write scopes.

## 5. Plan validation

The programme is complete as a plan, not a frozen low-level specification or implemented runtime.
API sketches, candidate capacities/layouts, unresolved profile encodings and command templates
remain explicit inputs to the relevant freeze tasks. Test fixtures and simulated peers support
development; they cannot satisfy acceptance of the real boundary they replace. The replacement
uses QSession's own stream machinery; Yamux belongs only to inherited/optional libp2p paths.

Run [validate-plan.ps1](./validate-plan.ps1) with PowerShell. It checks DAG cycles, checklist IDs,
package coverage, state/evidence consistency, local links and P0–P21 coverage. It validates this
plan's structure, not Rust conformance or remote links. FND-01 rechecks live instructions, branch,
ownership and source state before any implementation dispatch.
