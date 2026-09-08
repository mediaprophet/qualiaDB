# Workstream 07 — Compatibility, migration, qualification and release

**Status:** Implementation plan; all tasks pending.

Use the [library layout](../library-layout.md), [swarm protocol](../swarm-protocol.md) and
[validation matrix](../validation-matrix.md). Read [implementation conformance](../../implementation-conformance.md),
[operations](../../operations-and-deployment.md), [gateway design](../../legacy-internet-gateway.md)
and [peer runtime](../../peer-runtime.md). This workstream integrates evidence from the other
owners; it does not substitute release checklists for their domain tests.

## OPS-01 — Transition, browser and LIG profiles

**Dependencies:** RT-03.

**Ownership and construction:** Platform owner; carrier adapters, gateway policy, DNS/HTTP translation, publication, browser bridge and tests in separate directory-backed libraries.

- [ ] OPS-01.01 Declare supported transition carriers and their trust/dependency boundaries; UDP, WireGuard, WebSocket and browser transport retain native QPR identity, policy and service machinery.
- [ ] OPS-01.02 Keep an optional libp2p migration adapter outside replacement features and acceptance binaries; demonstrate dependency closure with the adapter disabled.
- [ ] OPS-01.03 Enable reliable-carrier optimizations only when the selected channel is ordered and reliable end to end; separately bound QPR framing, buffering, flow control, cancellation and reconnect state.
- [ ] OPS-01.04 Implement the selected initial carrier profile before optional multipath or concurrent-carrier extensions; require new sessions when migration cannot safely preserve cryptographic and delivery state.
- [ ] OPS-01.05 Bound NAT traversal, probes, relay discovery, connection racing and endpoint disclosure; bind observations to their provenance without treating them as signed route authority.
- [ ] OPS-01.06 Implement explicit LIG requests and an isolated translation policy; native unresolved names must not silently trigger DNS or Internet fetches.
- [ ] OPS-01.07 Separate gateway DNS, address validation, TLS verification, redirects and HTTP translation; reauthorize every changed target and prevent private-network access outside the caller's scope.
- [ ] OPS-01.08 Require scoped authority for inbound publication and address/port mappings; implement expiry, withdrawal, quotas and restart reconciliation without expanding service access.
- [ ] OPS-01.09 Keep legacy server claims, gateway observations and native evidence distinguishable; a gateway signature cannot establish an Internet origin's native authority or post-quantum security.
- [ ] OPS-01.10 Implement the browser profile through its supported sandbox interfaces and declare IP/WebPKI/platform dependencies; do not treat a compiling empty WASM feature as a working native bearer.
- [ ] OPS-01.11 Bound untrusted web response size, parsing and transformations; route executable content only through explicitly selected application sandbox policy.
- [ ] OPS-01.12 Test loss, reconnect, gateway rejection, malicious redirects, publication withdrawal and browser cancellation; report unsupported platforms and optional profiles explicitly.

**Acceptance and handoff:** Selected profile adapters and gateway/browser demonstrations, with dependencies disclosed, handed to OPS-02 and QA-02.

## OPS-02 — Application migration and operator workflows

**Dependencies:** OPS-01, SVC-02, ECO-02, RT-02.

**Ownership and construction:** Integration owner; application adapters, operator commands, role workflows, migration and rollback tools remain separate from protocol libraries.

- [ ] OPS-02.01 Inventory live application uses of libp2p, legacy peer APIs, chat/share/sync, connection identifiers and storage schemas; record an explicit migration or retained compatibility boundary for each.
- [ ] OPS-02.02 Adopt the public peer facade in representative real applications; preserve cancellation, observable errors and recipient policy instead of integrating only a toy transport.
- [ ] OPS-02.03 Specify qcx1 compatibility and qcx2 migration using the existing identifier design, method-authorized proofs, expiry and durable nonce replay protection.
- [ ] OPS-02.04 Use shadow verification and a single authoritative effect writer during cutover; compare decisions without duplicating payments, delivery, custody or graph mutations.
- [ ] OPS-02.05 Implement resumable schema/data migration with bounded workspace, backups and version checks; demonstrate rollback compatibility or document a controlled forward-recovery boundary.
- [ ] OPS-02.06 Provide operator-visible role enabling, funding and resource admission with previews of delegated authority, commitments, limits and withdrawal behavior.
- [ ] OPS-02.07 Support gifts, reciprocity and community-funded routing as well as selected paid rails; successful role activation must not require a wallet when its funding terms do not.
- [ ] OPS-02.08 Expose saturation, budget depletion, uncertain clocks/meters, stale authority, failed preservation and degraded routing with actionable states that do not leak private peer relationships.
- [ ] OPS-02.09 Provide supervised drain, role withdrawal, cell replacement and exception-workload controls; account for outstanding custody, settlement and evidence obligations before releasing resources.
- [ ] OPS-02.10 Document recovery from lost peers, unavailable funding rails and interrupted upgrades; test these workflows against the actual services and persisted state.
- [ ] OPS-02.11 Remove mandatory libp2p imports/features from migrated replacement paths and verify the applications run with that dependency absent.
- [ ] OPS-02.12 Hand operators tested installation, configuration, upgrade, recovery and limitation documentation with exact profile versions and evidence references.
- [ ] OPS-02.13 Provide accessible contributor compensation acceptance/dispute and user quote/fulfilment views, including the distinction between finite creation recovery and ongoing service costs.
- [ ] OPS-02.14 Provide private block/mute/report/withdraw-consent and independently reachable help without automatic notification to an alleged abusive guardian; prevent unilateral safeguard removal and distinguish help access from account takeover.
- [ ] OPS-02.15 Support role/capacity changes, officeholder succession and safe recovery with scoped authority; do not transfer a predecessor's private persona or return control automatically to a contested party.
- [ ] OPS-02.16 Test free personal/humanitarian service, corporate-delegation classification and exhausted sponsorship with honest degradation and no hidden personal debt or compulsory identity exposure.
- [ ] OPS-02.17 Provide known-peer medical-sharing views showing actual decrypting recipients, selected records, standing permissions and pending/delivered/reviewed status, without repeated prompts inside a valid accepted grant.
- [ ] OPS-02.18 Provide explicit care-team/referral/recording controls and agreed failure-contact instructions; never silently forward records to a substitute service or assume guardian access to every child consultation.

**Acceptance and handoff:** Application migration and operator workflow evidence handed to QA-02/REL-01; no external deployment implied.

## QA-02 — Adversarial, platform and scale qualification

**Dependencies:** QA-01, CORE-04, RT-02, SVC-02, SVC-03, ECO-02, EVD-02, OPS-01.

**Ownership and construction:** Independent verification owner; reusable fixtures, protocol interoperability, fault injection, platform adapters, scale experiments and reports have distinct owners/files.

- [ ] QA-02.01 Resolve the release scope/feature/target matrix and pin test source states, dependency versions, build commands, profiles, random seeds and independent-oracle versions.
- [ ] QA-02.02 Exercise every supported parser with valid, truncated, malformed, oversized and adversarial inputs; verify bounds before allocation, iteration, state installation or external effects.
- [ ] QA-02.03 Use independent protocol/proof encoders and captured vectors to test exact bytes, transcript binding, negotiation, replay, downgrade rejection, rekey and restart behavior.
- [ ] QA-02.04 Demonstrate two applications communicating over a supported native non-IP bearer with libp2p, DNS, IP routing and mandatory external trust services absent from that path.
- [ ] QA-02.05 Test revocation and expiry races across resolution, route, session, policy cache, delivery and queued effects; verify current authority again at the irreversible boundary.
- [ ] QA-02.06 Test pseudonym changes, conflicting claims, aliases, co-attestations, Sybil peers and purpose changes without accidental person joins or pooled authority.
- [ ] QA-02.07 Inject process/storage failures at artifact, graph, receipt, reservation, hold and external-effect boundaries; reconcile durable outcomes without duplicate spend or fabricated finality.
- [ ] QA-02.08 Measure actual Tier-1 allocations across construction, success, rejection, cancellation and recovery; account for dependency scratch, worker stacks and outstanding I/O ownership.
- [ ] QA-02.09 Measure complete Webizen-pass, ordinary-cell and aggregate-host budgets independently under hostile load; show backpressure and preserved control progress rather than relying on constants.
- [ ] QA-02.10 Run deterministic 1/2/4/more-cell partition and handoff experiments within admitted hardware; test lease conservation, per-flow ownership, stable results and bounded aggregate work.
- [ ] QA-02.11 Exercise logical datasets larger than memory with cross-segment joins and resumable work; report maximum per-call work, cache residency, storage amplification and global-query limitations.
- [ ] QA-02.12 Test concurrent providers and role changes against aggregate resource/money/exposure caps; verify unknown energy/compute remains unknown and failure receipts do not claim useful output.
- [ ] QA-02.13 Verify preservation and offline evidence examination across holds, expiry, key rotation, missing dependencies, remote acknowledgement and storage pressure; report exact scope and incompleteness.
- [ ] QA-02.14 Run selected native OS, browser/WASM, constrained and gateway profiles on their actual execution targets; mocks supplement but cannot replace platform evidence.
- [ ] QA-02.15 Publish throughput, latency distributions, CPU, memory, I/O and measurement uncertainty against declared workloads and baselines; never infer linear scale or energy savings from architecture alone.
- [ ] QA-02.16 Obtain independent security, semantics, persistence and resource reviews; fix material findings and rerun affected gates before forwarding a qualification recommendation.
- [ ] QA-02.17 Qualify finite recovery under cross-cell/provider concurrency, last-payment races, offline exposure, duplicate funds and terminal fulfilment using an independent conservation oracle.
- [ ] QA-02.18 Qualify corporate-agent, personal-employee, humanitarian-organization and mixed/unknown capacity cases against the pinned ontology/exception model.
- [ ] QA-02.19 Exercise child/PEP discovery, introduction, presence/location leakage, paid contact attempts, abusive guardians and private reporting with independent authorized scenario fixtures.
- [ ] QA-02.20 Verify protection/control/help capacity and entitled access behavior under resource pressure, provider failure and unavailable backends without fake success or silent surveillance fallback.
- [ ] QA-02.21 Demonstrate private known-peer exchanges for an adult PEP and a child of a VIP under their selected care policies, with independent metadata-disclosure and actual recipient-boundary checks.
- [ ] QA-02.22 Exercise standing clinical grants, replies, care-team changes, revoked queued delivery and compromised/replaced instruments; prove that receipt of bytes is not reported as clinical review or response.

**Acceptance and handoff:** A reproducible qualification dossier and unresolved limitations for REL-01; failed required targets keep this package incomplete.

## REL-01 — Release evidence and replacement completion

**Dependencies:** QA-02, OPS-02.

**Ownership and construction:** Integration owner; release manifest, migration tooling, operator documents and dependency evidence are separate artifacts with independent review.

- [ ] REL-01.01 Reconcile all 30 package statuses and child evidence; ensure every required predecessor and selected profile is accepted, with deferred optional scope visible and not claimed as implemented.
- [ ] REL-01.02 Publish exact source revisions/dirty-state fingerprints, builds, dependency/feature closure, licenses and reproducible artifact hashes for the release candidate.
- [ ] REL-01.03 Verify all selected protocol versions, ontology/context bundles, contract profiles, resource units, file-format decisions and compatibility matrices agree across implementations.
- [ ] REL-01.04 Review that exported APIs and new libraries follow ownership boundaries and file-size rules; close tracked decompositions required for changed oversized implementations.
- [ ] REL-01.05 Confirm representative applications operate with libp2p absent and optional transition dependencies isolated; distinguish an intermediate native demonstration from completion of the full programme.
- [ ] REL-01.06 Package test results, independent reviews, benchmarks and known limitations with redacted shareable evidence; preserve raw artifacts only under explicit ownership and byte/retention budgets.
- [ ] REL-01.07 Perform a local/staging upgrade, interrupted-upgrade recovery and rollback exercise; document any irreversibility before an operator chooses a production migration.
- [ ] REL-01.08 Document key compromise, vulnerability response, version withdrawal, provider departure and evidence-renewal procedures with assigned operational owners.
- [ ] REL-01.09 Check operator consent and funding defaults, private disclosure boundaries, retention defaults and accessibility of degraded/error states against accepted semantic contracts.
- [ ] REL-01.10 Record release approval and remaining operational prerequisites; publishing, deployment or messages to external parties require the applicable explicit authorization.
- [ ] REL-01.11 Update design conformance statements to verified behavior and link unsupported/deferred profiles; do not label cryptography audited or evidence legally sufficient without the corresponding basis.
- [ ] REL-01.12 Close the programme only after independent acceptance of the selected full scope and explicit reconciliation of every conditional task; archive handoffs and measurements with future maintenance owners.

**Acceptance and handoff:** Reviewable release candidate, conformance report and operator handoff. This planning task does not perform release actions.
