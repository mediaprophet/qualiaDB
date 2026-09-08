# Semantic Network Roles and Service Provision

**Status:** Architectural and semantic proposal; vocabulary and realization remain open
**Date:** 2026-09-06

## 1. Networking as an agent purpose

An agent may enable a network-provider role and devote capacity to routing, relay, discovery,
resolution or custody. It can operate as a dedicated router, share a host with other roles, or
spread work across several cells. A role describes purpose, authority, commitments and resources;
a cell supplies execution. Agent, role, cell, identifier and physical machine remain distinct.

Economic support belongs in the role lifecycle. Enabling a role binds available resources,
beneficiaries, terms, funding/contribution mode and exposure limits. Gifts, sponsor-funded access,
community pools, reciprocity and optional micropayments use the same account model. A wallet is
required only by a selected settlement mechanism; native participation does not require payment.

Define these capabilities semantically first. Ontology terms, CBOR-LD tables, routing algorithms,
file offsets and runtime APIs remain candidate realizations. Semantic evolution is explicit and
versioned; a downloaded ontology cannot introduce executable authority. Follow the
[Identifier Fabric integration](./identifier-fabric-integration.md): roles, credentials, wallets,
routes and accountability claims never merge into a NaturalAgent or a universal identity token.

## 2. Semantic capability model

| Concept | Meaning and necessary relationships |
|---|---|
| Role mandate | Who authorizes an agent to offer a service, for which realm/resources, purpose and validity |
| Service offer | Capability, coverage, capacity, outcomes, constraints and interpretation profiles |
| Participation agreement | Beneficiaries, rights, duties, funding/contribution terms, withdrawal and dispute procedure |
| Resource envelope | Memory, typed compute, energy, time, bandwidth/storage and concurrent financial exposure |
| Service commitment | Bounded accepted obligation linking consumer, provider, offer version, reservation and delivery boundary |
| Observation/evidence | Attributed measurements or claims, issuer, uncertainty, scope and expiry |
| Outcome/receipt | Accepted, delivered, failed or uncertain work, and which obligation it satisfies |
| Execution assignment | Cells and leased resources implementing a role, with explicit ownership and failure handling |

The [Q42 networking modality](../q42-network-modality-draft.md) represents these relationships.
Exact signed records use core-managed artifacts, with [QNF](../qnf-network-container-draft.md) a
proposed specialized representation. Verified views support packet processing. The ontology keeps
agreements independent of container choice. Access capacity and
[evidence preservation](./electronic-evidence-and-retention.md) require separate authority.

## 3. Enabling and operating a role

```text
mandate + resources + funding -> scoped offer -> accepted commitment
                                                    |
                                         admitted service + observations
                                                    |
                                      outcome + obligation reconciliation
```

“Enable routing” selects whom to serve, bearers/realms, maximum resources/exposure, availability
and funding terms. A local profile can supply defaults. Validate authority, capacity, interpretation
and funding before advertising a usable offer; advertise only services the agent can deliver.

Reserve resources and economic allowance atomically on commitment. Advertising an offer does not
reserve capacity or create debt. Packets consume an installed bounded allowance using counters and
current handles; contracts, proofs and settlement stay outside forwarding loops. Community
entitlements or relationships can authorize standing allowances without negotiation per packet.

On depletion, thermal pressure or disable, withdraw new offers and stop new commitments; drain or
close existing commitments under their accepted limits. Reserve closure and reconciliation capacity.
Security revocation can stop delivery immediately and leaves an explicit outcome. A failed payment
adapter cannot extend credit, switch rails or repeat a charge under a new operation identity.

## 4. Routing economics and bootstrap

Provision bounded discovery, authentication, error and closure from local, community or sponsored
allowances. A node may decline service when none is available. Do not require a paid route to reach
the only service able to authorize that same route. Abuse-limited bootstrap is not unlimited relay.

Forwarding, recipient receipt, application acceptance and payment finality are different outcomes.
A router's forwarding receipt alone does not prove end-to-end delivery. Self-generated traffic and
collusion can inflate counts; pool-funded rewards require accepted evidence and bounded exposure.

For multi-hop service, identify which providers receive allowances and who is responsible for each
hop or the aggregate result. Each bilateral commitment has its own evidence and settlement state;
the design does not assume atomic payment across a changing path. Reserve aggregate exposure before
replacement hops, and reconcile uncertain work before charging again. One signed quote cannot
authorize arbitrary new providers, higher prices or broader disclosure after rerouting.

Coarse price/capacity hints guide candidates but are not binding quotes. Check reachability, consent,
capability and hard limits before comparing eligible offers. Preferences can weigh delay, joules,
contribution mode, reliability and price without one global score. Missing evidence means unknown.

## 5. Semantic opportunities for network improvement

| Capability | Semantic contribution | Realization to investigate |
|---|---|---|
| Discover by intent | Required output/service, permitted purpose and supported interpretation | Authorized indexes and compiled capability matching |
| Choose routes/providers | Scoped offers, authority, contact windows, obligations and resource vectors | Bounded feasible-candidate filtering, then a versioned objective |
| Move less data | Authorized useful projection, freshness and completeness | QSync deltas, subscriptions, shared exact evidence and scoped caches |
| Place work | Data access, compute, custody and permitted execution locations | Compute near authorized data or use an explicitly accepted provider |
| Adapt to energy/availability | Deadlines, intermittent contact and acceptable degradation | Batching, sleep-aware scheduling and bounded custody commitments |
| Reuse verified knowledge | Provenance, uncertainty, validity and interpretation | Generation-bound views invalidated by changed authority |
| Coordinate duties | Permissions, obligations, conflicting claims and remedies | Supported deontic/temporal/paraconsistent evaluation with explicit unknown outcomes |
| Sustain service provision | Useful contributions, subsidies and obligations independent of payment rail | Role resource accounts and bounded receipt workflows |

These are testable hypotheses. Compare useful authorized outcomes and energy/time/work costs under
equivalent workloads, including semantic compilation, proof acquisition and invalidation overhead.
Retain routing safety: loop prevention, bounded forwarding, expiry, congestion and resource control.
Similarity cannot establish identity, authority or reachability; an inferred preference cannot
weaken a hard constraint. Conflicting claims retain provenance and unresolved status.

Advertise the minimum permitted capability summary, not private graphs or global relationship
lists. Pinned semantic bundles make interpretation reproducible. Unsupported reasoning returns an
explicit outcome instead of executing downloaded code or treating lexical similarity as consent.

## 6. Multiple cells for dedicated providers

Use the established **512 MiB** ordinary-cell ceiling, with smaller envelopes on constrained hosts.
A router agent may use several [Network Cells](./network-cell.md) within a host reservation. Cells
add capacity/isolation; they do not multiply funding, identities, quotas or authority. Group bearer
flows, proof workers, route preparation or custody according to measured ownership and work.

Each live flow/session has one mutation owner. Moving it requires a quiesce/handoff boundary with
exclusive transfer of state, buffers and counters. Durable replay IDs, economic reservations and
policy generations remain coherent through core publication and the atomic parent governor.

The exceptional category for LLM or similar work can exceed the ordinary ceiling under a declared
host profile. Routing does not inherit that exception because the same agent also runs an LLM.
Exceptional work still describes memory/compute/energy/financial exposure and remains isolated from
bounded network services. A large host scales cells after reserving actual capacity and overhead.

## 7. Next design decisions

[P19](./implementation-conformance.md#qdnf-p19--semantic-network-provider-roles) develops a coherent
ontology and examples of community routing, paid transit and intermittent custody. Identify which
reasoning fragments compile into existing bounded modalities before freezing representations.
Validate enable/disable/restart, funding exhaustion, multi-cell ownership, incompatible meanings and
honest outcomes. Numerical layouts in earlier drafts are candidates, not final assignments.
