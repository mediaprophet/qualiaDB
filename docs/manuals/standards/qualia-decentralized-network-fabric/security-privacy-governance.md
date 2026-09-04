# QDNF Security, Privacy, and Governance

**Status:** Normative design 0.1

## 1. Objectives and limits

QDNF targets authenticated neighbor and route bindings, encrypted sessions, replay-safe expiring
state, local discovery control, offline resilience, bounded resource use, and evidence that separates
origin from truth.

It does not guarantee traffic-flow anonymity, global availability, credential truth, endpoint
security, safety after controller-key compromise, or deletion from every prior recipient.

## 2. Trust boundaries

Untrusted until independently verified:

- bearer locators and unauthenticated discovery beacons;
- DHT nodes, route gossip, relays, gateways, and introducers;
- aliases, search rankings, and remote diagnostics;
- route cost, latency, emergency, importance, and location claims;
- public keys embedded in self-signed messages; and
- semantic facts or credentials received from any route.

QLink authenticates an ephemeral adjacency. QRoute verifies routing authority. DID/resource proofs
authenticate controller bindings. QPolicy authorizes operations. QSession encrypts end-to-end. Each
is a separate gate.

## 3. Threats and controls

| Threat | Required control |
|---|---|
| Bearer/MAC spoofing | Bind adapter-observed source locator, both ephemeral keys, nonces, scope, and transcript in QLink proof; rotate link IDs. |
| Discovery surveillance | Pairwise/group rotating rendezvous tags; no stable DID/name/location in private beacons. |
| Fake adjacency | Ephemeral DH plus proof; possession of rendezvous secret where private; short expiry and rekey. |
| Route poisoning | Verify LSA/path/RAR signer authority, sequence, expiry, path constraints, and strong digest. |
| Route loop/black hole | Repeated-realm rejection, hop limit, multipath, expiry, local observations, and bounded failover. |
| Sybil/eclipse | Relationship bootstrap, diverse paths/providers, realm admission policy, no DHT reputation root. |
| Replay | Single-use nonces, per-context windows, sequence/epoch, expiry, withdrawal, and operation IDs. |
| Key substitution | DID-method verification relationship plus transcript binding of QLink/QRoute/QSession keys. |
| Controller compromise | Short TTL, withdrawal, key rotation, recovery transition, and M-of-N for high-risk changes. |
| Social graph traversal | Pairwise IDs, directional/non-transitive relations, encrypted RARs, introduction capability, hop limit. |
| Correlation | Per-context route keys, link/route epochs, rotating mailbox and rendezvous tags. |
| Malicious gateway | Explicit LIG action, end-to-end content digest where available, isolated caches/policy, bounded receipts. |
| Malicious subnet gateway | SDR scope, child authorization, no capability widening, path loop checks. |
| Malicious swarm replica | Controller replica grant, content digest, signed-op verification, paraconsistent isolation. |
| Homograph/deceptive alias | Language/script/source display, confusable warning, canonical target confirmation, disambiguation. |
| DoS | Pre-crypto quotas, per-source budgets, admission puzzles only where proportionate, rate limits, 42 MiB ceiling. |
| Downgrade | Explicit namespace/carrier/gateway selection; no DNS/IP fallback after native failure. |
| Cross-protocol replay | Distinct domain separators for every signed record and transcript. |

## 4. Relationship policy

A relationship is a scoped fact/agreement, not a universal trust score. `friend`, `guardian`,
`clinician`, `coworker`, and `community member` have different contexts and allowed actions.

- Peering may disclose a private route.
- Peering alone never permits graph read/write, inference delegation, or third-party introduction.
- Relationships are directional unless a bilateral agreement explicitly binds both sides.
- Trust and authority do not transit through friends, gateways, relays, or DHT neighbors.
- Delegation states grantor, delegate, resource/context, action, purpose, expiry, and proof.
- `foundation/crdt.rs::verify_delegation` must perform real signature verification before QDNF use;
  its current placeholder behavior is an automatic deny at this boundary.

## 5. Blocking and revocation

Active block state is checked before discovery, dialing, capability exchange, application delivery,
or introduction. Blocking need not notify the blocked party.

Revocation:

- removes a peer/route from new selection;
- drains active sessions according to sensitivity policy;
- invalidates capabilities under their revocation rules;
- emits minimal integrity-protected local evidence; and
- never claims remote deletion that cannot be proven.

Route withdrawal, relationship revocation, capability revocation, and content deletion are distinct
events.

## 6. Human agency and identity fabric

- An identifier is not an identity.
- Natural persons may hold many pairwise/contextual DIDs and recovery anchors.
- Automated cross-context correlation is prohibited without specific consent and necessity.
- Loss of a key/device does not conceptually erase a person; identity recovery uses an explicit quorum
  transition without publishing recovery material.
- Guardianship and fiduciary roles are scoped duties, not ownership.
- Interfaces use “identifier,” “relationship persona,” “device,” or “resource,” not “the person's
  identity” merely because a DID is present.

## 7. QPolicy and deontic evaluation

Before sensitive reads or state changes, the node builds a bounded policy frame containing requester,
target, context, operation, purpose, shape, capability/consent/delegation, expiry/revocation,
sensitivity, routing lane, and applicable norms/defeaters.

Outcomes:

- **allow**;
- **allow and audit**;
- **prioritize** after verified local policy and proportionality;
- **preventive block** before harm; or
- **interactive** for ambiguity or required M-of-N approval.

Ambiguity never means permission. A remote peer cannot self-assert humanitarian priority. Emergency
overrides cannot bypass non-derogable protections.

## 8. Conflict handling

Security contradictions do not use ordinary LWW:

- same target and sequence with different signed RARs is equivocation;
- conflicting LSAs/path advertisements at one sequence are isolated;
- conflicting issuer claims remain distinct epistemic assertions;
- paraconsistent contexts prevent one contradiction from exploding or silently disappearing;
- saturation can trigger a human circuit breaker; and
- later valid state can restore operation without erasing evidence.

LWW remains acceptable only for explicitly designated low-risk, same-author mutable fields.

## 9. Privacy profiles

| Profile | Discovery | Identifier | Publication |
|---|---|---|---|
| `private-pairwise` | shared rotating tag | pairwise DID | encrypted direct |
| `closed-group` | group epoch tag | group-context DID | encrypted to group |
| `community` | realm directory | contextual resource DID | community-visible RAR |
| `public-service` | public service beacon/DHT | service DID | short-lived public RAR |
| `content-public` | provider discovery | content ID | public replica providers |

Person-controlled routes default to `private-pairwise`. Public publication is explicit and explains
correlation/traffic-analysis consequences.

## 10. Location

- Exact coordinates never appear in public RARs.
- Coarse regions may support public service discovery with controller consent.
- Personal proximity should rely on bearer reachability or selective-disclosure proof rather than
  coordinates.
- Location aliases expire and do not become permanent identity evidence.
- A “near me” query reveals the minimum region and prefers local indexes.
- Child, health, shelter, refuge, and similar vulnerable contexts default to no public spatial data.

## 11. Multilingual fairness and accessibility

No language, script, trademark office, government registry, commercial directory, or popularity
ranking is a universal root. Clients preserve source distinctions and local issuer policy.

UIs render native Unicode and metadata, support screen readers and non-text pairing, warn about
confusables without stigmatizing scripts, show cryptographic status separately from fame/brand, and
always provide an exact-identifier route when semantic search is unavailable.

## 12. Logging

A resolution/session audit should contain time, policy version, target/RAR/DNI/transcript digests,
source kind, verification outcomes, stable reason codes, capability/deontic decision digest, and
bounded performance observations.

It should not contain raw private DIDs, full locators, exact location, credential bodies, messages,
or recovery material unless a specific lawful/user-governed purpose requires them. Audit retention is
sensitivity-labelled and purpose-bounded; accountability does not justify indefinite personal-data
retention.

## 13. Key lifecycle

- Signing, route, QLink, QSession, key-agreement, envelope, and recovery keys are purpose-separated.
- Private keys never enter Q42 volumes, invitations, RARs, DHT, or logs.
- Link, route, and relay tokens rotate by epoch.
- High-risk controller changes should require M-of-N or equivalent recovery governance.
- Intermediate key material is zeroized where supported.
- Unsupported algorithms fail closed.
- Hybrid post-quantum additions remain explicit and bounded.

## 14. Prohibited security claims

Implementations must not claim “ARP/DNS eliminated” when only using QDNF-over-IP transition mode;
“DID verified” after checking only an embedded key; “zero trust” merely because signatures exist;
“anonymous” when stable traffic metadata remains; “credential verified, therefore true”; “deleted
everywhere” without proof; or “decentralized” when a vendor relay/directory is mandatory.
