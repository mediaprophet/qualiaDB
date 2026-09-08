# Dependency roadmap and delivery milestones

**Status:** Planning baseline. The [task register](./task-registry.json) is authoritative for package
dependencies and state; the [workstream checklists](./README.md) own the individual to-do items.

## 1. Start with semantics and verified boundaries

FND-01 records the live source and ownership baseline. FND-02 then defines roles, authority,
resource meanings and evidence lifecycles before interfaces or packet layouts are frozen.
Existing core range/storage work and the independent test harness can proceed alongside this
semantic work. Crypto-provider integration informs the common interface baseline; final protocol
bytes and vectors belong to CRY-02 after that baseline. This avoids a circular “freeze everything
before either side can start” dependency.

Preserve the distinction between current source evidence and intended behavior. The
[core audit](../core-memory-and-parallel-networking.md) documents why a 42 MiB Webizen table,
a 512 MiB field on WorkerCell and an existing range API do not alone establish enforced
memory limits, one Sentinel per cell or bounded total query work.

## 2. Dependency waves

These are earliest package readiness waves assuming all prior packages are accepted. They are
not dates, staffing commitments or permission for every ready package to write shared files.
A package may contain multiple reviewable changes; dispatch child IDs with narrow file claims.
Maintain the package owner across those changes and use [swarm protocol](./swarm-protocol.md)
for conflict detection and interface handoff.

| Wave | Packages eligible after earlier waves |
|---|---|
| 0 | FND-01 |
| 1 | FND-02, CORE-02, QA-01 |
| 2 | CORE-01, CORE-03, CRY-01 |
| 3 | FND-03, SEM-01 |
| 4 | CORE-04, CRY-02, RT-01, EVD-01 |
| 5 | NET-01, RT-02, ECO-01 |
| 6 | NET-02 |
| 7 | NET-03 |
| 8 | NET-04, NET-05 |
| 9 | RT-03 |
| 10 | SVC-01, ECO-02, OPS-01 |
| 11 | SVC-02, SVC-03, EVD-02 |
| 12 | OPS-02, QA-02 |
| 13 | REL-01 |

Prototype against a reviewed provisional interface where useful, but mark that work provisional.
A stub, test fixture or partial predecessor cannot satisfy a completion dependency. QA-01 builds
the independent harness early; domain agents own their tests throughout, with QA-02 integrating
adversarial and cross-boundary evidence later.

Concurrency is limited by available compute, worktree/storage budgets, integration capacity and
actual disjoint ownership. Two ready tasks in the same source file must be serialized or split
through an approved decomposition. Use a shared interface owner for exports, manifests, Cargo
features, registries and profile schemas. No agent count is a correctness or performance target.

## 3. Claimable task graph

| ID | Package | Owner role | Required predecessors | Checklist |
|---|---|---|---|---|
| FND-01 | Source baseline and scope | integration | None | [00-foundation](./workstreams/00-foundation.md) |
| FND-02 | Semantic and authority contracts | semantics | FND-01 | [00-foundation](./workstreams/00-foundation.md) |
| FND-03 | Versioned interface and profile baseline | integration | FND-02, CRY-01 | [00-foundation](./workstreams/00-foundation.md) |
| CORE-01 | Caller-owned Webizen and scoped cache | core | FND-01, FND-02 | [01-core-storage-webizen](./workstreams/01-core-storage-webizen.md) |
| CORE-02 | Bounded large-graph reads and work | core | FND-01 | [01-core-storage-webizen](./workstreams/01-core-storage-webizen.md) |
| CORE-03 | Atomic artifacts, effects and recovery | storage | FND-02, CORE-02 | [01-core-storage-webizen](./workstreams/01-core-storage-webizen.md) |
| CORE-04 | QNF decision and selected representation | storage | CORE-03, FND-03 | [01-core-storage-webizen](./workstreams/01-core-storage-webizen.md) |
| CRY-01 | Existing primitive and key-provider integration | crypto | FND-01, FND-02 | [02-crypto](./workstreams/02-crypto.md) |
| CRY-02 | PQ handshake and proof profiles | crypto | CRY-01, FND-03 | [02-crypto](./workstreams/02-crypto.md) |
| NET-01 | Frames and native bearers | network | FND-03, RT-01 | [03-native-network](./workstreams/03-native-network.md) |
| NET-02 | QLink adjacency and discovery | network | NET-01, CRY-02 | [03-native-network](./workstreams/03-native-network.md) |
| NET-03 | QRoute realms and mobility | network | NET-02, FND-02 | [03-native-network](./workstreams/03-native-network.md) |
| NET-04 | DNI, resolution, aliases and QSR | resolution | NET-03, CRY-02, CORE-03, SEM-01 | [03-native-network](./workstreams/03-native-network.md) |
| NET-05 | QSession and policy delivery boundary | network | NET-03, CRY-02, SEM-01 | [03-native-network](./workstreams/03-native-network.md) |
| RT-01 | Lease kernel and admission ledger | runtime | FND-03, CORE-01 | [04-peer-runtime-cells](./workstreams/04-peer-runtime-cells.md) |
| RT-02 | Supervised cells and parallel execution | runtime | RT-01, CORE-02, CORE-03 | [04-peer-runtime-cells](./workstreams/04-peer-runtime-cells.md) |
| RT-03 | Public facade and service integration | runtime | RT-01, NET-04, NET-05 | [04-peer-runtime-cells](./workstreams/04-peer-runtime-cells.md) |
| SEM-01 | Identifier Fabric and contract compiler | semantics | FND-02, CORE-01, CORE-03, CRY-01 | [05-semantic-services](./workstreams/05-semantic-services.md) |
| SVC-01 | Recoverable QSync and swarms | services | RT-03, CORE-03, SEM-01 | [05-semantic-services](./workstreams/05-semantic-services.md) |
| SVC-02 | Authorized subscriptions and projections | services | SVC-01 | [05-semantic-services](./workstreams/05-semantic-services.md) |
| SVC-03 | Custody and bounded compute services | services | SVC-01, SEM-01, RT-02 | [05-semantic-services](./workstreams/05-semantic-services.md) |
| ECO-01 | Typed resource accounts and budgets | economics | SEM-01, RT-01, CORE-03 | [06-roles-economics-evidence](./workstreams/06-roles-economics-evidence.md) |
| ECO-02 | Provider roles, funding and settlement | economics | ECO-01, RT-03 | [06-roles-economics-evidence](./workstreams/06-roles-economics-evidence.md) |
| EVD-01 | Retention, holds and promotion | evidence | CORE-03, SEM-01 | [06-roles-economics-evidence](./workstreams/06-roles-economics-evidence.md) |
| EVD-02 | Evidence custody, export and renewal | evidence | EVD-01, ECO-02, CRY-02 | [06-roles-economics-evidence](./workstreams/06-roles-economics-evidence.md) |
| OPS-01 | Transition, browser and LIG profiles | platform | RT-03 | [07-release](./workstreams/07-release.md) |
| OPS-02 | Application migration and operator workflows | integration | OPS-01, SVC-02, ECO-02, RT-02 | [07-release](./workstreams/07-release.md) |
| QA-01 | Independent test and measurement harness | verification | FND-01 | [00-foundation](./workstreams/00-foundation.md) |
| QA-02 | Adversarial, platform and scale qualification | verification | QA-01, CORE-04, RT-02, SVC-02, SVC-03, ECO-02, EVD-02, OPS-01 | [07-release](./workstreams/07-release.md) |
| REL-01 | Release evidence and replacement completion | integration | QA-02, OPS-02 | [07-release](./workstreams/07-release.md) |

## 4. Demonstration and release milestones

| Milestone | Reviewable outcome | Required evidence |
|---|---|---|
| M0 — semantic and source baseline | Agreed entity/claim/handle/instrument boundaries, resource and evidence semantics, measured current gaps, owned interfaces and harness | FND-01–03, CRY-01 and QA-01; source audit and baseline build outcomes |
| M1 — first native vertical slice | Two real applications use QPR over a supported non-IP bearer, with policy-bound hybrid sessions and libp2p absent | Relevant CORE, CRY, NET, SEM and RT child checks accepted; memory/lease measurements and native trace |
| M2 — useful governed services | Multi-realm resolution, resumable graph sync, subscriptions and enabled provider roles with commons funding and optional payment | NET/RT/SEM/SVC-01–02/ECO packages accepted; replay, revocation, budget and recovery evidence |
| M3 — enterprise and preservation profiles | Multiple bounded cells, bounded compute/custody, large graph operation, selected evidence and selected gateway/browser profiles | CORE-01–04, RT-02, SVC-03, EVD-01–02, OPS-01 and cross-boundary qualification results |
| M4 — application migration and replacement release | Application migration, operational recovery and full selected scope independently qualified | QA-02, OPS-02 and REL-01 plus every transitive predecessor complete |

M1 is a deliberately narrow demonstration, not completion of entire NET/RT packages. An
intra-realm vertical slice can be reviewed while inter-realm work is still pending; the parent
package remains open. Track the exact accepted child IDs, profile limitations and integration
source state. M4 requires reconciliation of the full programme rather than rebranding M1.

QNF has an explicit adopt/defer decision in CORE-04. If existing core artifacts meet the
requirements, conditional QNF construction checks close with a reviewed non-adoption decision
and equivalent storage evidence, never an invented implementation claim. Optional payment rails,
multipath, exceptional workloads and platform profiles similarly require explicit selected,
unsupported or deferred status. Conditionality cannot waive core authority, memory, native
replacement, evidence integrity or required service guarantees.

## 5. Original P0–P21 coverage

This crosswalk lists implementation/domain owners. QA-02 additionally qualifies the whole
programme and is intentionally excluded here so its broad scope cannot conceal missing owners.

| Source package | Implementation packages |
|---|---|
| [P0](../implementation-conformance.md) | FND-01, FND-03, CORE-01, CRY-01, NET-01 |
| [P1](../implementation-conformance.md) | NET-01 |
| [P2](../implementation-conformance.md) | CRY-02, NET-02 |
| [P3](../implementation-conformance.md) | NET-03 |
| [P4](../implementation-conformance.md) | CORE-02, CORE-03, NET-04 |
| [P5](../implementation-conformance.md) | CRY-02, NET-05 |
| [P6](../implementation-conformance.md) | NET-03, NET-04, SVC-01 |
| [P7](../implementation-conformance.md) | OPS-01 |
| [P8](../implementation-conformance.md) | OPS-01 |
| [P9](../implementation-conformance.md) | FND-02, CORE-03, SEM-01, ECO-01, ECO-02 |
| [P10](../implementation-conformance.md) | RT-01, RT-02, RT-03 |
| [P11](../implementation-conformance.md) | CORE-02, CORE-03, SVC-01, SVC-02 |
| [P12](../implementation-conformance.md) | SVC-03, OPS-01 |
| [P13](../implementation-conformance.md) | FND-01, RT-03, OPS-02, REL-01 |
| [P14](../implementation-conformance.md) | FND-03, CRY-01, CRY-02 |
| [P15](../implementation-conformance.md) | FND-02, FND-03, CORE-01, SEM-01 |
| [P16](../implementation-conformance.md) | CORE-03, CORE-04 |
| [P17](../implementation-conformance.md) | CORE-01, CORE-02, RT-01, RT-02 |
| [P18](../implementation-conformance.md) | SVC-03, ECO-01 |
| [P19](../implementation-conformance.md) | FND-02, ECO-02, OPS-02 |
| [P20](../implementation-conformance.md) | FND-02, CORE-03, EVD-01, EVD-02 |
| [P21](../implementation-conformance.md) | FND-02, NET-04, SEM-01, EVD-02 |

Preserve these mappings when splitting tasks. The integrator updates the register, this roadmap,
downstream dependencies and the validator's expected package set together after review; workers
must not add hidden prerequisites only in a handoff message.

## 6. Escalation and acceptance order

Resolve semantic disagreements with the semantics owner, byte/profile disagreements with their
domain owner and resource/persistence disagreements with the responsible core owner. Record
a [decision](./templates/decision-record.md), affected consumers and migration cost. Unresolved
security, authority or durability assumptions block acceptance of the affected task.

Use dependency evidence from the integrated source state, then independent review, then package
completion. Rerun affected checks after conflicts, interface changes or new failure findings.
Do not rerun unrelated suites merely to produce a larger report. Implementation dispatch and
external release actions remain separate from approval of this plan.
