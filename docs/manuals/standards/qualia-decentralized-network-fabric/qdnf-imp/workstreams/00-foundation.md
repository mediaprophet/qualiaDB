# Foundations and Independent Harness

Own the baseline and shared contracts before implementation fans out. Dependencies and claim state live in the [register](../task-registry.json); all checks below start unfinished.

All packages follow [library boundaries](../library-layout.md), [swarm handoffs](../swarm-protocol.md)
and the [shared validation standard](../validation-matrix.md). Package dependencies are authoritative
in the register; early interface-based work cannot bypass their completion evidence.

## FND-01 — Source baseline and implementation scope

**Owner and inputs:** Integration owner; inputs: live repository instructions, [source review](../../source-and-current-stack-review.md), [core audit](../../core-memory-and-parallel-networking.md).

**Construction boundary:** Baseline report, source-owner map, claimed target/profile matrix and existing test failures. Shared root/feature edits belong only to the integrator.

- [ ] FND-01.01 Record branch/commit, dirty file fingerprints, current instructions and concurrent claims; preserve unrelated changes.
- [ ] FND-01.02 Recheck each P0–P21 source finding and classify reuse, repair or new implementation with exact owners.
- [ ] FND-01.03 Inventory current tests, toolchains, crate features, supported targets and available isolated test bearers; distinguish absent hardware from passing support.
- [ ] FND-01.04 Map both WorkerCell implementations, all SlgArena construction paths and existing parallel workspace owners; choose integration ownership without adding a third competing engine.
- [ ] FND-01.05 Record 48-byte ABI, parity producer/reader discrepancies, cache scope, rule growth, incomplete result paths and allocation constraints.
- [ ] FND-01.06 Audit module sizes and planned write sets; schedule required extraction before adding behavior to oversized files.
- [ ] FND-01.07 Publish selected release scope: native replacement, enterprise cells, contracts/provider roles/evidence, supported browser/gateway profiles and optional rails.
- [ ] FND-01.08 Name shared-file, crypto, semantic, storage and verification reviewers; reserve build/test resources.
- [ ] FND-01.09 Record current crypto/library versions and exact reviewed specification profiles; use primary sources when updating them.
- [ ] FND-01.10 Produce a baseline build/test report with actual commands and existing failures; do not fix unrelated lanes without coordination.

Exit: every downstream owner can locate real code, inherited defects and the agreed target matrix. Handoff to all lanes; no declaration that inherited stubs are implemented.

## FND-02 — Semantic and authority contracts

**Owner and inputs:** Semantics lead with crypto/storage/runtime reviewers; inputs: role, fabric, contracts, economics and evidence designs.

**Construction boundary:** Versioned meaning/authority inventory plus state/ownership/error contracts; use existing ontology owners and record unresolved vocabulary choices explicitly.

- [ ] FND-02.01 Enumerate entities, claims, handles and instruments; prohibit person merging through DID, wallet, location, role, alias or classifier similarity.
- [ ] FND-02.02 Define operation intent, purpose, target, temporal grant, co-attestation authority/independence and recovery transitions without choosing a universal person join key.
- [ ] FND-02.03 Specify provider mandate, offer, commitment, funding, withdrawal, delivery boundary and remedy meanings for router agents.
- [ ] FND-02.04 Bind energy, typed time, compute counting profiles, capacity, useful outcomes, prices and credits as independently interpreted quantities.
- [ ] FND-02.05 Define exact-source, semantic-projection and compiled-view authority/lifetime relationships; cache hits never renew permission.
- [ ] FND-02.06 Specify diagnostic/audit/evidence tiers, balanced selection, holds, source/proof/interpretation closure and custody/disclosure distinctions.
- [ ] FND-02.07 Write minimal coherent examples for local commons routing, paid transit, a changing multi-hop route, role turnover and offline evidence examination.
- [ ] FND-02.08 Specify supported SHACL/N3/modality fragments, bounded expansion and explicit unknown/unsupported/incomplete outcomes.
- [ ] FND-02.09 Define block/revocation/expiry precedence, historical versus current authority, and conflicts without a global reputation score.
- [ ] FND-02.10 State bootstrap/control allowances independently of optional monetary settlement; prevent circular paid-route authorization.
- [ ] FND-02.11 Identify exact payloads and metadata authorized for each operation, including projection proofs, evidence exports and private alias resolution.
- [ ] FND-02.12 Publish reviewed interface semantics before assigning new binary fields; record open decisions with owner and measurable decision criteria.

Exit: semantics are coherent enough to implement against versioned contracts. Concrete wire/container choices still require their own review; handoff to CORE, CRY, SEM and NET.

## FND-03 — Versioned interface and profile baseline

**Owner and inputs:** Integrator with all interface consumers; consumes FND-02 meanings and CRY-01 primitive constraints.

**Construction boundary:** Reviewed shared types and interface bundles; focused core modules for typed identifiers/digests/errors/leases. No duplicated NQuin or protocol-local identity engine.

- [ ] FND-03.01 Define full identifier, typed digest, scope/generation and operation ID boundaries; short hashes are lookup indexes only.
- [ ] FND-03.02 Define event/effect, clock/entropy, bearer, crypto provider, storage transaction and policy-result interfaces with ownership and completion rules.
- [ ] FND-03.03 Set bounded byte/count/work profiles with checked arithmetic; separate protocol maximums, local cell limits and admitted outstanding credit.
- [ ] FND-03.04 Inventory conflicting old/new frame, digest and crypto assumptions; publish explicit profile/version negotiation and no-downgrade rules.
- [ ] FND-03.05 Define canonical bytes, domain separation, extensions, reserved/critical values, range errors and unknown-profile outcomes for each interface.
- [ ] FND-03.06 Specify how crypto state/proof stages bind protocol inputs; CRY-02 owns final transcript and independent crypto vectors.
- [ ] FND-03.07 Publish deterministic golden fixtures for shared representations and malformed variants before dependent implementations diverge.
- [ ] FND-03.08 Define core commit/revocation ordering, durable versus transport completion and asynchronous cancellation/late completion semantics.
- [ ] FND-03.09 Bind reusable views/cursors to source identity, policy/profile generation and valid scope; no raw pointer or portable live permit.
- [ ] FND-03.10 Define dependency feature boundaries and public export ownership; assign integration-only edits to Cargo and crate roots.
- [ ] FND-03.11 Record ABI/alignment/endianness and C/WASM ownership conventions with versioned error outcomes.
- [ ] FND-03.12 Review and publish the baseline with consumer sign-off; subsequent changes list invalidated vectors and affected dependent tasks.

Exit: consumers share reviewed contracts and can implement independently. Do not pretend a fixture baseline freezes every future service profile.

## QA-01 — Independent test and measurement harness

**Owner and inputs:** Verification owner; coordinate fixture interfaces with FND-03 while establishing the harness from FND-01.

**Construction boundary:** Focused test transport, clocks/entropy, fault controller, independent parser/peer and evidence reporter; separate test-only helpers from product APIs.

- [ ] QA-01.01 Build deterministic fake bearer/clock/entropy drivers with bounded buffers and repeatable seeds.
- [ ] QA-01.02 Implement loss, reorder, duplication, corruption, truncation, partition, restart and delayed completion injection.
- [ ] QA-01.03 Create an independent codec/peer oracle that does not simply invoke the same production encoder and decoder.
- [ ] QA-01.04 Add thread-local hot allocation measurement covering success, error and cancellation without global serial-test assumptions.
- [ ] QA-01.05 Track reservation/lease conservation, pool ownership, live generations and peak referenced memory.
- [ ] QA-01.06 Provide crash points around file sync, root publication, replay/application commit, hold promotion and external settlement acknowledgment.
- [ ] QA-01.07 Create isolated no-IP/native and transition test environments; record emitted traffic without touching live user networking.
- [ ] QA-01.08 Define golden fixture provenance, schema/digest version, independent review and artifact byte budgets.
- [ ] QA-01.09 Separate wall time, typed compute, energy measured/estimated/unknown, useful throughput and storage/copy counts.
- [ ] QA-01.10 Publish reproducible result manifests with source state, target/feature set, commands, seed/input digest, expected/observed outcomes and limitations.

Exit: domain lanes can demonstrate their invariants against an independent fault model. Handoff harness versions and measurement boundaries to every lane.
