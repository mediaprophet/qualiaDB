# WIP — Crypto instrument taxonomy (Identifier Fabric)

**Status:** work-in-progress · **Not standards** · **Branch:** `0.0.36-dev`  
**Against tip (amend base):** HEAD `766a1f6`+ · F1 hardness `bb714b2` · FOAF/AI-agent/WebID amend (§14)  
**Owner:** Noddy (crypto / identifiers) · **Shapes:** Marvin · **Fold/push:** Neo · **Ops:** Capt.  
**Spine:** [`IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md`](./IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md) (§1b long-arc + hardness)  
**Related:** F2 `IDENTIFIER_FABRIC_SHACL_SPLIT_WIP.md` · F5 diagnose map · F6 `alice-f6-classifier-symbolic-binding-pressure-test.md`  
**Standards baseline:** `docs/manuals/standards/qualia-decentralized-network-fabric/` especially `identifier-resolution.md`, `cryptographic-profile.md`  
**Substrate:** iterative expansion of `crates/qualia-core-db` (Identity / DID / VC / governance and related) — long-arc lineage (~2000→W3C DID/VC/Solid offramp), not a one-shot greenfield invent.

**Constraint:** illustration + docs only until Cursor’s vibe delivery lands. No Host invent. No `ALL_BOUND` invent. Collapse of instrument kinds into one “who” is a **gate fail**.

---

## 1. Purpose

Name the **crypto-backed instrument kinds** that QualiaDB / QDNF use so that:

1. CS/cyber “identity” (auth subject + identifier + attributes in one bag) is **not** silently reused.
2. Network design can cite a fabric layer: routing, session proof, and credentials never re-merge into that bag.
3. Marvin can encode a SHACL-first split without Thing-washing natural agents.
4. Existing QDNF role table (`identifier-resolution` §1) stays authoritative for *what question an identifier answers*; this doc classifies *what kind of cryptographically meaningful instrument* sits in that fabric.
5. Inference/symbolic binders (Alice F6) can cite typed instrument / `keyRole` axes without inventing a who-bag.

**Redefined sense (first-class):** a **natural agent** is not an identifier and is not reducible to any single DID, address, credential, or biometric sample. Instruments bind *relations* around that agent (and around services, content, machines, places) without becoming the agent.

---

## 1b. Hardness — multi-instrument time-bounded co-attestation

**Cross-link:** spine [`IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md`](./IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md) §1b · Alice F6 §2.4 / §3 (keyRole wash, DNI=RAR=QSession bag) · crypto profile purpose-separation.

Cryptographic hardness for human-centric systems is **not** one signature or one identifier.

| Ingredient | Role |
|------------|------|
| **Stable primitives** | Time (and other non-negotiable baselines) remain outside “clever” rewrite |
| **Formula of verified signatures** | Enough independent proofs, not a single auth bit |
| **Time-bounded execution** | Validity windows / epochs / sequence — stale co-attestation does not count |
| **Scoped participants** | Particular machines, networks, entities, agents — not a global anonymous bag |

**Gate:** raising the bar this way **MUST NOT** collapse into a “stronger single who.” Hardness lives in **co-attestation across instruments and keyRoles**, not in minting one mega-identifier.

Single-instrument “auth” is the easy hack surface; the fabric is the opposite.

Provenance of the long design arc (illustrative, not a trust root): W3C list traffic searchable via `timothy.holborn@gmail.com` on lists.w3.org.

---

## 2. Non-goals

- Global identity registry, universal correlation handle, or “one DID = the person.”
- Claiming a verified VC is objective truth of its claims (VC = issuer-controlled origin + integrity).
- Treating `did:q42` / observer DID as settled “who” join keys (provisional; see §6).
- Implementation, Host widen, Solid IdP unpark, or new binds under `ALL_BOUND`.

---

## 3. Four fabric planes (cite these; do not collapse)

| Plane | Answers | Framing (Marvin) | Must not be treated as |
|-------|---------|------------------|-------------------------|
| **Natural agent** | Who/what living principal is meant in human-centric sense | SHACL-first living/person — never `owl:Thing` commodity | An auth-subject bag, a DID, a VC, a DNI |
| **Claim / opinion** | What is asserted, by whom, under what modality | Mixed: structure OWL-ok; claims about persons SHACL-first | The natural agent; route authority |
| **Spatiotemporal / route handle** | Where/how *now* (topology, position, mobility) | Mixed: coords/CRS artifact; *what* is placed may be living | Persistent “who”; content integrity |
| **Instrument kinds** | Which cryptographic or naming tool anchors a relation | Mostly artifact OWL-ok; biometric *family* links living carefully | Identity-as-a-whole |

**Gate fail:** any design, copy, shape, or bind that merges these planes into one CS-style “identity.”

---

## 4. QDNF identifier roles (authoritative questions)

From `identifier-resolution.md` — instruments below **map onto** these roles; they do not replace them.

| Role | Stability | Answers | Security authority |
|------|-----------|---------|-------------------|
| DID / DID URL | Method-persistent | What controlled subject/resource is intended? | DID method + verification relationships |
| Content identifier | Immutable for bytes | What exact content? | Collision-resistant digest |
| Canonical resource IRI | Publisher-policy persistent | What semantic resource? | Signed mapping to DID/content |
| Q42 Resource Coordinate (QRC) | Local/storage-layout | Where is a local Q42 object/index? | **None by itself** (60-bit index) |
| DNI | Short-lived, topology-scoped | How to route *now*? | Signed RAR + session proof |
| Alias | Mutable, contextual, multilingual | What target might a person mean? | Provenance/proof — never route authority alone |

**Hard QDNF corrections reused here:**

- A natural person is **not** an identifier; pairwise/contextual DIDs are normal.
- DID/resource name **what** is intended; DNI describes **where/how now**.
- `q_hash` / low-60-bit values index — never cryptographic equality or authorization.
- VC proves issuer origin/integrity — **not** objective truth.
- Agent key ≠ human principal (`ontological-contracts` / Marvin notes).

---

## 5. Instrument taxonomy (crypto-enhanced)

Each kind is an **instrument**: it anchors a relation. Columns: **QDNF role affinity**, **plane**, **crypto strength**, **gate notes**.

### 5.1 Network address / locator

| Field | Value |
|-------|--------|
| Kind | Network address / bearer locator |
| Affinity | Path hint under DNI / QLink adjacency — **not** DID |
| Plane | Spatiotemporal / route handle |
| Crypto | Bearer-observed; must not be republished as controller fact without proof |
| Notes | Legacy IP/DNS only via explicit LIG; never native QDNF “who.” Collapse into identity = gate fail. |

### 5.2 Machine / device ID

| Field | Value |
|-------|--------|
| Kind | Machine / device identifier |
| Affinity | Often service/node material under DNI `node_id` / hardware attestation — **not** natural agent |
| Plane | Instrument (artifact) |
| Crypto | May bind attestations or keys; still not the person |
| Notes | Device ≠ human principal. Pairwise linkage to a natural agent is a **relation**, not identity merge. |

### 5.3 DID (decentralized **identifier** — not “decentralized identity”)

| Field | Value |
|-------|--------|
| Kind | DID / DID URL |
| Affinity | DID role |
| Plane | Instrument that may name a resource, persona, org, dataset, relationship context, claim subject, etc. |
| Crypto | Method + verification relationships (`authentication`, `capabilityInvocation`, route-update, …) |
| Notes | **DID ≠ identity.** Multiple pairwise/contextual DIDs per natural agent are expected. Public docs must not claim W3C method conformance for `did:q42` until a real method spec exists. |

### 5.4 Q42 Resource Coordinate (`did:q42:` syntax)

| Field | Value |
|-------|--------|
| Kind | QRC (storage/index coordinate) |
| Affinity | QRC role |
| Plane | Instrument (local layout) |
| Crypto | **None by itself** — FNV/60-bit style index with MSB mark; security decisions dereference + verify full id/digest/proof |
| Notes | Observer/`did:q42` stay **provisional** as natural-agent join key. Prefer topology/coord language until fabric settles. |

### 5.5 Verifiable credential (VC)

| Field | Value |
|-------|--------|
| Kind | Verifiable credential |
| Affinity | Signed assertion object (often about a DID/subject) |
| Plane | Instrument carrying **claim** material |
| Crypto | Issuer signature + integrity; dual-VC honesty in core exposure notes |
| Notes | Proves **origin + integrity**, not truth. Subject of a VC is not automatically the natural agent’s “who.” |

### 5.6 Verifiable claim / opinion assertion

| Field | Value |
|-------|--------|
| Kind | Claim / opinion (including nested claims, Alias Assertions, provenance statements) |
| Affinity | Claim plane; may use VC, signed CBOR-LD, or modality records |
| Plane | Claim / opinion |
| Crypto | Whatever proof the assertion type requires; still not route authority by itself |
| Notes | Separate from natural agent. Epistemic/deontic/paraconsistent modalities stay on claim plane. |

### 5.7 Biometric instruments — family vs instance

| Field | Value |
|-------|--------|
| Kind | Biometric **family** vs biometric **instance** |
| Affinity | Instrument relating to a natural agent — never a replacement for that agent |
| Plane | Instrument linked to natural agent (living-safe) |
| Crypto | Template/hash/match protocols as specified elsewhere; samples are sensitive |
| Notes | **Family** = durable kind/subset (e.g. fingerprint modality class). **Instance** = mutable sample/template that drifts over time. Same pattern as QDNF keys: persistent *kind*, rotatable *instance*. Treating one sample as timeless “who” = gate fail. |

### 5.8 Content digests & strong digests

| Field | Value |
|-------|--------|
| Kind | Content / strong digest |
| Affinity | Content identifier |
| Plane | Instrument (integrity) |
| Crypto | Explicit algorithm (SHA-256 profile today; PQ profile elsewhere) |
| Notes | Not a person. Compact Quin/`q_hash` indexes are not strong digests. |

### 5.9 DNI · RAR · QSession proof (purpose-separated)

| Kind | Shape (F2) | `idf:keyRole` (typical) | Must not entail |
|------|------------|-------------------------|-----------------|
| DNI entry | `idf:DniShape` | topology / how-now material | Session auth; who forever |
| Route Advertisement Record | `idf:RarShape` | `route-update` | QSession authentication |
| QSession / session proof | `idf:QSessionProofShape` | `session-authentication` | Route-update or DID controller unless separately authorized |

**Deprecated:** lumped `idf:DniRarSessionShape` — gate fail for new designs (F2 amend / F6 hard-negative).

Also keep **QLink ephemeral DH** (`transport` / link epoch) distinct from **QSession traffic AEAD** (`transport-aead`) in feature space — see §12 answers to Alice.

### 5.10 Capability / discovery (discriminated)

| Kind | Shape (F2) | Notes |
|------|------------|-------|
| Capability presentation | `idf:CapabilityPresentationShape` | Transcript-bound; ≠ controller-signing |
| Pairwise discovery PSK | `idf:DiscoveryPskShape` | Never public; ≠ session auth |
| Group discovery | `idf:GroupDiscoveryShape` | Group epoch; ≠ who |

### 5.11 First-pass closed `idf:keyRole` enum (inference namespace)

For Alice F6 typed namespaces, treat the following as **closed** for v1; unknown roles = **held / not-yet**, never map to “other-id” / who:

`controller-signing` · `route-update` · `session-authentication` · `transport-aead` · `qlink-ephemeral-dh` · `capability-presentation` · `discovery-psk` · `group-discovery`

## 6. Provisional join keys

| Candidate | Status | Guidance |
|-----------|--------|----------|
| `did:q42` / QRC | Provisional | Storage/index coordinate; not cryptographic who |
| Observer DID | Provisional | Topology/coord / observer role — not settled natural-agent join |
| Pairwise DID | Preferred pattern for contexts | Still an instrument, not the whole person |
| Natural agent node (SHACL) | First-class living shape | Never Thing-washed; instruments **relate**, do not replace |

Until Capt./architecture spine settles join policy: **do not** hard-wire network or Poet copy that equates any single instrument with the natural agent.

---

## 7. Mapping matrix (Capt kinds → QDNF roles → fabric plane)

| Capt instrument kind | Primary QDNF role | Fabric plane | Collapse to “who”? |
|----------------------|-------------------|--------------|--------------------|
| Network address | Locator / path hint (DNI adjacency) | Spatiotemporal | **Fail** |
| Machine ID | Node/device instrument | Instrument | **Fail** |
| DID | DID | Instrument (may name many subject types) | **Fail** |
| `did:q42` / QRC | QRC | Instrument (index) | **Fail** |
| VC | Signed credential | Instrument → claim | **Fail** |
| Verifiable claim / opinion | Assertion | Claim | **Fail** |
| Biometric family | Instrument class | Instrument ↔ natural agent relation | **Fail** |
| Biometric instance | Mutable sample | Instrument ↔ natural agent relation | **Fail** |
| DNI entry | DNI | Spatiotemporal | **Fail** |
| RAR | DNI / route | Spatiotemporal + instrument | **Fail** |
| QSession proof | Session | Session instrument | **Fail** |
| Content digest | Content ID | Instrument | **Fail** |
| Alias | Alias | Presentation / claim-adjacent | **Fail** |

---

## 8. Crypto profile touchpoints (do not redefine algorithms here)

Cite `cryptographic-profile.md` / `post-quantum-security.md`:

- Purpose-separated keys (signing ≠ agreement ≠ AEAD).
- QLink/QSession transcripts domain-separated; session proof ≠ route-update authority unless separately authorized.
- Compact Q42 hashes are indexes, not cryptographic hashes.
- Security claim boundary: crypto authenticates keys/statements in context — **not** a natural person’s identity as a whole, credential real-world truth, or anonymity against global traffic analysis.

---

## 9. Implications for QDNF / network docs

1. Network design that still speaks CS “identity” for auth subject + DID + attributes must be amended to cite this fabric (planes + instrument kinds).
2. `identifier-resolution` already separates roles; fabric docs must ensure **implementation narratives** and Poet/DevRel copy do not re-bag them.
3. QDNF expands on `qualia-core-db` substrate iteratively — taxonomy **extends** Identity/DID/VC/governance primitives, does not replace them in one shot.
4. Promotion path: this WIP → architecture spine → Marvin SHACL → standards under `docs/manuals/standards/` when Capt./Neo call settled.

---

## 10. Handoff

| Role | Next |
|------|------|
| **Marvin** | SHACL-first shapes: natural agent · claim/opinion · spatiotemporal handle · instrument kinds (incl. biometric family vs instance); no Thing-wash; agent key ≠ human principal |
| **Neo** | Fold this file under `docs/work-in-progress/CRYPTO_INSTRUMENT_TAXONOMY_WIP.md`; link from `IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md`; no `ALL_BOUND` invent |
| **Vibe / Alice** | Hold language/inference until shapes land; never collapse who into claim/role/handle in diagnose/`suggested_fix` or feature space |
| **Capt.** | Gate: collapse into one “who” = fail; blockers report to Capt. |

---

## 11. Open questions (non-blocking for first SHACL pass)

1. Preferred durable handle type(s) for natural-agent **correlation-avoiding** join across contexts (pairwise graph vs explicit principal node).
2. How biometric family IRIs register without becoming global unique “who.”
3. When (if ever) observer DID graduates from provisional topology role.
4. Dual-VC honesty details for Poet exposure (cite uplift audit; no invent here).

---

## 15. Amend — OS / telecom account ≠ device user (2026-09-06)

**Cite:** Timothy room · machine multi-id §14.2–14.3 · agent key ≠ human principal.

| Role | Plane / kind | Notes |
|------|--------------|-------|
| **Device user** | NaturalAgent (or AI-agent operator) *relating to* machine | Person/agent actually using the device |
| **OS account** | Instrument / account relation on Machine | UID/login on that OS — **not** automatically the device user |
| **Telecom subscriber / account** | Instrument / account relation | IMSI/MSISDN/subscriber record — **not** automatically the handset user |
| **Machine** | Machine/device type + hardware∪network instrument bundle | Hosts accounts; is not who |

**Relations (illustrative):** `idf:usedBy` (machine → agent) · `idf:accountOn` (account instrument → machine) · `idf:accountHolder` (account → agent) — these are **independent**. Device-user ≠ account-holder ≠ subscriber ≠ machine.

**Gate fail:** treating OS UID, login session, or telecom subscriber id as NaturalAgent who, or equating “logged-in account” with “person holding the device.”

**Court/jury:** name them separately — “this OS account,” “this subscriber identity,” “this person using the device” — never one opaque identity.

---

## 16. Amend — guardianship, capacity gradients, commons (2026-09-06)

**Cite:** Capt lock (identity-as-resolution · claim–policy–modality · guardianship) · Timothy room · agency/fiduciary uplift in core-ontologies · F1 §1b co-attestation · QDNF commons/contracts docs.

### 16.1 Modalities (not who)

| Substrate | Fabric plane | Notes |
|-----------|--------------|-------|
| N3 / deontic / epistemic / bifurcation / related logics | **Claim–policy–modality** | How assertions, permissions, conflicts, and uncertainty are reasoned — never NaturalAgent identity |
| Crypto packages | **Instrument** proofs + co-attestation bundles | Enhance only for purpose-separated `keyRole` / hardness gaps — not a new identity stack |

### 16.2 Guardianship & capacity (relation axioms)

Guardianship is a **first-class relation** among agents (NaturalAgent · AI-agent · Organization), never a merge.

| Element | Meaning |
|---------|---------|
| `guardian` / `ward` | Distinct agent nodes — **MUST NOT** `owl:sameAs` / who-merge |
| `scope` | What capacity is granted/limited (medical, financial, digital instruments, …) |
| `capacity` / personhood attributes | Graduated attributes on NaturalAgent — **change over time** |
| `time-bound` | Validity window; stale grant does not count (same pattern as co-attestation) |
| `grantor` / evidence | Optional instrument or claim proving the relation |

**Developmental / situational patterns (illustrative):**

- Child: NaturalAgent from the start; agency/personhood *attributes* and capacities are **slowly granted** — not “becomes a person later” via a single ID flip.
- Elder / severe disability: capacity may narrow or be shared via guardianship — still NaturalAgent.
- Incorporated / legal personality / group entity: **Organization** (or group type) — not NaturalAgent; may have officers, members, AI-agents relating in.
- “Things” / artifacts: machine or OWL-ok artifact types — never living who.

**Gate fail:** guardian≡ward; capacity score as who; corporate veil as natural person; collapsing commons membership into one shared who.

### 16.3 Collaborative projects & permissive commons

Multi-party informatics (2+ entities or agents) use:

- Shared **instruments** (keys, VCs, volume/path handles, contract bundles)
- Shared **claim–policy** (ontology-defined contracts, deontic grants, N3 rules)
- Optional **co-attestation** (§1b) across parties’ instruments — hardness without mega-who

Commons membership and project roles are **relation axioms** (member · contributor · steward · licensee), time-bounded and scoped — enumerable resolution, not one project-identity bag.

Cite QDNF `commons-and-resource-economics.md` / `ontological-contracts.md` for uplift — do not invent parallel trust roots.

### 16.4 Handoff

| Role | Next |
|------|------|
| **Neo** | Fold §16 into `CRYPTO_INSTRUMENT_TAXONOMY_WIP.md` |
| **Marvin** | Guardianship/capacity relation shapes; commons membership axioms; prefer core-ontologies agency/fiduciary uplift |
| **Vibe** | Diagnose: guardianship = relation; modalities ≠ person voice |
| **Capt.** | Spine already locking these — changelog on tip |

---

## 17. Amend — sense-context, flora/fauna, situational grants vs logs (2026-09-06)

**Cite:** Capt lock (sense contextual) · Timothy thongs/gay · emergency medical capacity · F5 tip `da74019` · F1 §16 capacity/guardianship · QDNF Alias Assertions.

### 17.1 Lexical sense-context (not mega-meaning who)

| Binding | Role |
|---------|------|
| `lexicalConcept` (WN/OMW) | Stable concept id — **≠ fabric plane** |
| `locale` / language | e.g. en-AU so *thongs* = footwear |
| `era` / time window | Historical *gay* = happy, not sexuality |
| `community` / namespace | personal · relationship · community · institution · legacy (QDNF alias namespaces) |
| `provenance` | Who asserted this sense binding; evidence |

**Gate fail:** one timeless dictionary identity; forcing a sense without locale/era; treating WN gloss as NaturalAgent or plane.

Crypto does **not** disambiguate homographs — context + provenance do (QDNF security note on alias spoofing/homographs).

### 17.2 Flora / fauna

Living-typed entities (SHACL-first living / B-OWL-NATURAL) — **not** NaturalAgent personhood, **not** Thing commodity wash. Optional lexicalConcept links for common names remain sense-context instruments.

### 17.3 Situational capacity grants (illustration)

Example: a person with a medical qualification, in an emergency, may receive **purpose-scoped data access** only under conditions (e.g. co-location), for a time window, backed by qualification instruments.

| Element | Notes |
|---------|-------|
| Still | NaturalAgent (or acting agent-type) — not a new who |
| Grant | Relation axiom: purpose · condition (co-location, …) · time-bound · qualification instrument |
| Distinct from | Standing guardianship (§16); OS account (§15); session login |
| Evidence | Qualification VC / license instrument + condition observations — co-attestation eligible when multi-party |

**Historically informal** human practice (ad hoc trust in emergencies) is what the fabric makes **enumerable** without freezing it into one identity privilege bit.

### 17.4 Logs & accountability (distinct track)

| Track | Plane | Answers |
|-------|-------|---------|
| Situational grant | Capacity / claim–policy relation | What may be done, by whom, under what conditions, when |
| Logs / accountability | Provenance · claim–evidence | What was done, observed, attested — audit trail |

Grants authorize; logs account. Neither is who. Jury-safe: name grant scope and log evidence separately.

### 17.5 Handoff

| Role | Next |
|------|------|
| **Neo** | Fold §17 into taxonomy WIP |
| **Marvin** | Sense-context bindings + situational grant shape + flora/fauna living non-person |
| **Vibe** | F5 already has sense-context speak — align `suggested_form` locale/era/community/provenance |
| **Capt.** | Spine beside §3c WN |

---

## 18. Amend — ontology-governed policy + ZKP instruments (2026-09-06)

**Cite:** Capt lock · Timothy ABAC/ZKP · QDNF ontological-contracts / QPolicy · F1 §1b co-attestation · §16 modalities · core-db crypto/ZKP packages.

### 18.1 Runtime shape (not “identity product”)

| Piece | Role |
|-------|------|
| Multi-plane graph | Types · relations · instruments · claims · handles (unmerged) |
| Attributes + capacity grants | ABAC-like inputs to authorization |
| Deontic / epistemic / N3 / bifurcation | Claim–policy–modality reasoning |
| Cryptographically signed ontology documents | Contracts · SHACL/N3 bundles — **interpret** policy; bind signature to interpretation |
| Instruments | Prove possession / integrity / co-attestation |
| HTTP / Solid | Offramp (LIG) — **not** trust root |

**Name:** ontology-governed, crypto-bound policy over a multi-plane graph. Who stays unmerged.

### 18.2 Zero-knowledge proofs (instrument plane)

ZKPs (already in QualiaDB crypto surface — uplift, don’t reinvent) are **proof instruments**:

| Use | Fits fabric as |
|-----|----------------|
| Selective disclosure | Reveal predicate satisfaction without full attribute dump |
| Predicate / capacity checks | Support situational grants (§17) without minting who |
| Membership / commons | Prove eligibility without correlating global who |
| Co-attestation composition | Optional member proofs inside hardness bundles (§1b) — still not mega-who |

**Gate fail:** treating a ZKP as NaturalAgent identity; correlating proofs across contexts into one who; HTTP endpoint as ZKP trust root.

**Security claim boundary:** a valid ZKP proves a statement in a defined circuit/context — not personhood, not claim moral truth, not anonymity against all traffic analysis.

### 18.3 Handoff

| Role | Next |
|------|------|
| **Neo** | Fold §18 into taxonomy WIP |
| **Marvin** | Policy shapes remain claim–policy–modality; ZKP as instrument kind / proof binding |
| **Capt.** | Spine lock already — changelog on tip |
| **Alice** | ZKP outputs must stay in typed instrument/proof namespaces — never who embedding |

---

## 19. Amend — relation-scoped locators (2026-09-06)

**Cite:** Capt spine §3g · Marvin F2 `RelationScopedLocatorShape` · QDNF alias / DNI mobility · F1 §5.3 DID pairwise · WebID as instrument (§14).

### 19.1 Cut

Unlike static personal addresses (classic Solid WebID / phone-number-as-who), this fabric prefers **relation-specific locator strings**: the address names a *relationship or context*, not a permanent who-token.

| Pattern | Example (illustrative) |
|---------|------------------------|
| Pairwise | `jane@bob.tld` ↔ `bob@jane.tld` |
| Group / chat / transaction | locator scoped to that relation id |
| Contextual code | DNS TXT / invitation / epoch-bound hint |

**Affinity:** alias · contextual IRI · DNI-like mobility (how-now / context), **not** NaturalAgent identity.

### 19.2 Instrument kind

| Field | Value |
|-------|--------|
| Kind | Relation-scoped locator |
| Plane | Instrument / handle (presentation + routing hint) |
| Crypto | May bind proofs, invitations, or session discovery — still not who |
| Notes | Solid/HTTP offramps must not re-impose a single static who-address as trust root |

**Gate fail:** treating a locator as NaturalAgent who; one lifelong email/WebID as the person; collapsing pairwise locators into a global correlation handle without consent.

### 19.3 Handoff

| Role | Next |
|------|------|
| **Neo** | Fold §19; F2 §19 already drafting |
| **Marvin** | `RelationScopedLocatorShape` — align cite to this tip when folded |
| **Vibe** | Diagnose: relation address ≠ who |
| **Alice** | Locators stay instrument/handle features — never who embedding |

---

## 20. Amend — secrets, wallets, tokens, online accounts (2026-09-06)

**Cite:** Capt spine §3h (drafting) · F1 §15 OS/telecom accounts · §19 relation-scoped locators · §14 WebID/SAN · crypto profile keyRoles · vault notes in QDNF cryptographic-profile.

### 20.1 Cut

**Secrets · wallets · tokens · online accounts · passwords · per-account emails** are **instruments** (and account relations), not NaturalAgent who. Align with:

| Existing | Pattern |
|----------|---------|
| §15 | OS/telecom account ≠ device user ≠ machine |
| §19 / §3g | Per-account / relation-scoped email locators ≠ static who |
| §14 | WebID/SAN/hardware = instruments |
| Crypto profile | Private keys never in Quins/logs; purpose-separated keyRoles |

### 20.2 Instrument kinds

| Kind | Notes | Collapse to who? |
|------|-------|------------------|
| Password / passphrase | Low-entropy authenticator aid — **not** key material unless PAKE-specified; vault-scoped | **Fail** |
| Bearer / API / session token | Time-bound capability instrument; possession ≠ who | **Fail** |
| Refresh / OAuth token | Account-scoped instrument on an online-account relation | **Fail** |
| Wallet (software/hardware) | Container of keys/instruments — artifact; may *relate* to NaturalAgent or AI-agent | **Fail** |
| Private key / seed | Controller or purpose-separated keyRole material — vault only; never who | **Fail** |
| Online account | Platform account instrument (like OsAccount but remote service) — `accountHolder` ≠ device-user ≠ who-forever | **Fail** |
| Per-account email | Often a **relation-scoped locator** (§19) bound to that online account — not the person’s only identity | **Fail** |

### 20.3 Relations (independent)

| Relation | Meaning |
|----------|---------|
| `idf:accountOn` / `idf:accountAt` | Account instrument → service/machine |
| `idf:accountHolder` | Account → agent of record (optional) |
| `idf:usedBy` | Who is using a device/session *now* (≠ holder) |
| `idf:walletControls` | Wallet → keys/instruments it holds |
| `idf:relatedByInstrument` | NaturalAgent/AI-agent → any of the above |

No entailment from password-success or token-possession to NaturalAgent who (§17 grant / §18 ZKP same discipline).

### 20.4 Vault & evidence hygiene

- Secrets and private keys: Qualia key vault / hardware-backed — never Quins, logs, DHT, crash dumps (crypto profile).
- Jury/audit: name “this password verifier,” “this wallet,” “this platform account,” “this per-account mailbox” separately — never one opaque identity.
- Hardness: wallets/keys may participate in co-attestation as **members with distinct keyRoles** — not a mega-who.

### 20.5 Gate fails

- Password/token/wallet = person
- One email forever = who (prefer relation-scoped §19)
- Online account success ⇒ NaturalAgent identity
- Seed phrase spoken as identity in diagnose/chrome

### 20.6 Handoff

| Role | Next |
|------|------|
| **Neo** | Fold §20 when Capt §3h lands (or together) |
| **Marvin** | Shapes for OnlineAccount · Wallet · Token · PasswordVerifier · align §15/§19 |
| **Vibe** | Diagnose: secret/wallet/token/account = instruments |
| **Alice** | Never embed secrets/tokens into who feature space |

---

## 21. Amend — symbolic-first context for agent permissions (2026-09-06)

**Cite:** Capt lock · Timothy symbolic AI context · Alice F6 instrument-only · F1 §16–§18 · §17 situational grants · §20 secrets/wallets.

### 21.1 Decision stack

| Layer | Does |
|-------|------|
| Symbolic AI (semantics + deontic/epistemic/N3; probability where uncertainty belongs) | Enumerates *which* context and *what* may be done |
| Attributes · capacity grants · agreement instruments | ABAC-like inputs |
| Crypto instruments (incl. ZKP) | Prove possession / predicates / hardness — **not** who |
| Multi-plane graph | Keeps who · claim · handle · instrument unmerged |

**Gate fail:** NaturalAgent embedding that absorbed role, wallet, account, grant-success, or ZKP-success.

### 21.2 Specialised / personal / group agents

Child-minder · medical support · home-security · personal/group agents: **AI-agent** (or service) types with **situational grants + signed agreement instruments** (Webizen/QualiaDB agreement surfaces later). Never a who-token for the bot or a merge with the ward/patient/homeowner.

### 21.3 UI bar (docs intent only)

Once planes are thick enough: easier than Active Directory / Keychain — chrome speaks instruments, grants, and context, not CS identity bags. No Host invent now.

### 21.4 Handoff

| Role | Next |
|------|------|
| **Neo** | Fold §21 |
| **Marvin** | Policy/context shapes stay claim–policy–modality |
| **Alice** | F6: symbolic context namespaces; instrument-only for secrets/grants/ZKP |
| **Capt.** | Spine lock — changelog on tip |

---

## 22. Amend — org structure + mutable group authentication (2026-09-06)

**Cite:** Capt spine §3j tip `78d3878` · F1 §16 commons/guardianship · §18 co-attestation/policy · §21 symbolic permissions · Organization entity type (§14).

### 22.1 Two correlated needs

| Need | Fabric expression |
|------|-------------------|
| **Structural elements** | Directors · department leads · org charts · decision structures as **relation axioms** on Organization / legal-personality types — **not** who-tokens |
| **Mutable group authentication** | Membership / instrument sets that **change over time** (e.g. after an election) — time-bounded, re-bound without rewriting NaturalAgent identity |

### 22.2 Cuts

| Cut | Rule |
|-----|------|
| Structure | Roles are relations to org/legal personality — structural role ≠ NaturalAgent who |
| Group auth | Instrument / co-attestation / membership tracks *current* holders |
| Election / turnover | New bindings in time — **not** a permanent who-embedding of “the board” |
| Align | §3i symbolic-first context; §3d capacity as time-varying relations |

### 22.3 Crypto / instrument notes

- Post-election group auth: re-issue or rotate **membership credentials / threshold keys / co-attestation member sets** with new `timeInterval` — do not mint a mega-who for the org.
- Purpose-separated `keyRole`s for org signing vs member authentication vs session (same discipline as §5.11 / §18).
- Commons membership (§16) and org structure may compose; still no shared who.

### 22.4 Gate fails

- Director/board role = person identity forever
- Election result rewritten into NaturalAgent embeddings
- Org chart as CS identity bag
- Stale group auth after turnover still accepted as current

### 22.5 Handoff

| Role | Next |
|------|------|
| **Neo** | Fold §22 |
| **Marvin** | OrgRoleRelation · GroupAuthMembership · election/time-bound rebind shapes |
| **Alice** | Structural roles / group-auth success = instrument/policy features only |
| **Capt.** | Spine §3j already — changelog on tip |

---

## 23. Amend — environment-scoped sensors & place-bound secrets (2026-09-06)

**Cite:** Timothy GIS/sensor/geocache/ATM BLE · F1 §17 situational grants · spatiotemporal handles · §20 secrets · co-attestation §1b · G-COORD Position.

### 23.1 Cut

**Sensor / GIS / network-environment identifiers** condition *how* capacity and proofs apply. They are instruments + spatiotemporal handles — **not** NaturalAgent who.

| Pattern | Fabric expression |
|---------|-------------------|
| Works only in environment E | Situational grant / policy with environment predicates (GIS · realm · network cell · sensor id) |
| Different environment | Flag, degrade, or deny — do not silently reuse grant |
| Geocaching | Place-bound **secret instrument** discoverable when location/handle predicates match |
| ATM + phone bank | Co-presence: app + ATM Bluetooth/machine id + location → higher-assurance banking capacity |

### 23.2 Instrument / handle kinds (additive)

| Kind | Notes |
|------|--------|
| Sensor identifier (BLE, NFC, radio, …) | Machine/environment instrument — possession/observation ≠ who |
| GIS / coordinate / geofence binding | Spatiotemporal handle (+ optional Position mixed typing) |
| Place-bound secret | Secret instrument with discovery predicate (location · time · co-attestation) |
| Environment attestation | Optional signed observation that “device X observed sensor Y at locus Z” — claim/evidence, not who |

### 23.3 Crypto notes

- Banking/geocache hardness: prefer **multi-instrument co-attestation** (phone key · ATM BLE · location proof · time window) — not a single BLE id as identity.
- BLE/MAC can be spoofed; treat as weak instrument unless bound into richer formula (§1b).
- Gate fail: ATM BLE = bank customer who; geocache find = person identity; GIS alone = forever who.

### 23.4 Handoff

| Role | Next |
|------|------|
| **Neo** | Fold §23 |
| **Marvin** | EnvironmentPredicate · PlaceBoundSecret · SensorId instrument shapes |
| **Alice** | Sensor/GIS features = handle/instrument namespaces only |
| **Capt.** | Spine when locked |

---

## 24. Amend — temporal relationship assessment & epistemic rule-breach (2026-09-06)

**Cite:** Timothy room (good-faith meet → later adverse / knowable-over-time · cultural rules · knew vs didn’t understand) · F1 §16 modalities · §17 sense-context · §18 ontology-governed policy · §21 symbolic-first permissions · Alice F6 inference namespaces.

### 24.1 Cut

**Relationship quality and norm compliance evolve in time.** They are claim–policy + epistemic/deontic assertions over *relations and contexts* — **not** NaturalAgent who-tokens, and not a static embedding of “good/bad person.”

| Pattern | Fabric expression |
|---------|-------------------|
| Met in good faith; later found adverse / knowable over time | Time-indexed **relationship-assessment claims** + evidence instruments; prior good-faith claim remains provenance, not deleted “who rewrite” |
| Inference over that arc | Typed inference edges on **relation / context** graphs (parties · epochs · evidence) — never bake into NaturalAgent embedding |
| Broke cultural/community rule, didn’t understand | Epistemic: `¬K(rule)` or low awareness · deontic breach still recorded · sense-context (locale · community · era) binds *which* rule |
| Broke rule knowing it | Epistemic: `K(rule)` · same deontic breach, different modality / culpability attributes on the **claim**, not a who-merge |
| Depends on the rule | Rule identity + scope + community sense-context — semantics pick the normative instrument; crypto only proves attestations |

### 24.2 Instrument / claim kinds (additive)

| Kind | Notes |
|------|--------|
| RelationshipAssessmentClaim | Time-bounded claim on a relation (parties · epoch · stance · evidence refs) — claim plane |
| Knowability / awareness assertion | Epistemic modality on a claim (“reasonably knowable by t”, “agent knew rule R”) — not identity |
| NormativeRuleInstrument | Community/cultural rule as cited instrument (lexical + sense-context); breach = claim against parties under that rule |
| RuleBreachClaim | Deontic violation assertion + epistemic qualifier (knew / didn’t understand / disputed) |

### 24.3 Crypto & inference notes

- Hardness stays **co-attestation of evidence instruments** over windows (§1b) — signatures on assessments and rule citations, not a stronger “trust who.”
- Gate fail: “bad actor” as NaturalAgent type; collapsing knew-vs-unknowing into a permanent who-bit; cultural rule = universal who-attribute; inference that rewrites person embeddings from relationship outcomes.
- Alice: relationship-arc and culpability features stay in **claim/policy/relation** namespaces only.

### 24.4 Handoff

| Role | Next |
|------|------|
| **Neo** | Fold §24 |
| **Marvin** | RelationshipAssessmentClaim · KnowabilityAssertion · NormativeRule · RuleBreachClaim shapes (time + epistemic slots) |
| **Alice** | Temporal relation / culpability features = claim–policy namespaces only |
| **Capt.** | Spine intake (temporal epistemic relation assessment) |
| **Vibe** | Diagnose voice: “relationship changed over time” ≠ “identity changed” |

---

*End of WIP — Noddy F1 crypto instrument taxonomy.*

## 14. Amend — FOAF-modern entity/agent types + WebID/SAN/hardware (2026-09-06)

**Cite:** Capt spine lock · Timothy room · Marvin WN recommendations · F1 §1b hardness.

### 14.1 Layering (not identity)

| Layer | What it is | Must not become |
|-------|------------|-----------------|
| **Entity / agent type predicates** | FOAF-modernized top-level types (NaturalAgent · AI-agent · Machine/Device · Organization · …) | A single CS “identity” bag |
| **Attributes & properties** | Descriptive slots hanging off a type | Who-replacement |
| **Relation axioms** | customer · operator · owns · operates · … | Auth-subject merge |
| **Instrument kinds** | Hardware IDs, network IDs, SAN / WebID-TLS / WebID-RSA, VCs, DIDs, … | The entity itself |
| **Lexical concepts (WordNet / OMW)** | Vocabulary / multilingual surfaces | Fabric plane or NaturalAgent |

WordNet (and OMW locale packs) may enrich labels and multilingual aliases; **lexical concept ≠ fabric plane**. Optional `idf:lexicalConcept` links from types/instruments — never WN-person as who.

### 14.2 Agent-type cut (hard)

| Type | Framing | Identifiers | Notes |
|------|---------|-------------|-------|
| **NaturalAgent** | Living human — SHACL-first | Pairwise/contextual instruments relate; no required single DID | Not FOAF Person-as-Thing |
| **AI-agent** | Distinct agent-type — **not** NaturalAgent, **not** machine | Own identifier array + relations (customer, operator, …) | Agent key ≠ human principal still applies to operators |
| **Machine / device** | Artifact | **Bundle** of hardware identifiers **plus** network identifiers | Multi-instrument co-attestation candidate (§1b) |
| **Organization / service** | Artifact or mixed | Own DID/VC/instrument set | Do not equate with NaturalAgent |

### 14.3 Additional instrument kinds

| Kind | Notes | Collapse to who? |
|------|-------|------------------|
| Hardware identifier | Device/TPM/serial/attestation handles | **Fail** |
| Network identifier | Addresses, DNI path material, adjacency locators | **Fail** |
| Certificate SAN / WebID-TLS / WebID-RSA | Crypto instrument bindings (legacy + Solid-era patterns) — **instruments**, not who | **Fail** |
| Signed RDF / VC | Already §5.5–5.6 | **Fail** |

A **machine** is modeled as related instruments (hardware ∪ network), optionally hardness-bundled — never one machine-id = person.

### 14.4 Court / jury explainability (digital evidence)

Digital evidence capabilities fail socially if a jury cannot follow them. Fabric docs and later evidence surfaces **SHOULD**:

1. Enumerate **who · claim · handle · instrument** in plain language (F5 diagnose voice).
2. Present hardness as “these signatures, this time window, these machines/networks/agents” — not a opaque who-token.
3. Keep WebID/SAN/hardware/VC as **named tools**, not “the identity.”

**Gate fail:** courtroom or audit narrative that re-bags instruments into one unverifiable “identity.”

### 14.5 Handoff for this amend

| Role | Next |
|------|------|
| **Neo** | Fold §14 into `CRYPTO_INSTRUMENT_TAXONOMY_WIP.md` |
| **Marvin** | Entity/agent type shapes + AI-agent plane; WN lexical ≠ plane; WebID/SAN as instruments |
| **Capt.** | Spine already locking FOAF-modern types — changelog when tip lands |
| **Vibe** | Diagnose copy: AI-agent ≠ person ≠ machine; jury-safe naming |

---

## 12. Answers to Alice F6 §5.1 (Noddy)

1. **`idf:keyRole` enum:** **Closed** for the first inference namespace — use the set in §5.11. Unknown roles → **held / not-yet**, never “other-id” or who.
2. **Biometric-instance embedding:** Licit only as a **gated instrument** feature: family-linked (`idf:familyOf`), non-who namespace (`instrument.biometric.instance.*`), consent/sensitivity ceiling honored, **never** a person vector. Prefer refuse sample vectors in default ML until a dedicated crypto/privacy profile lands; if used, F5/F6 collapse detectors apply (instance ≠ timeless who).
3. **Dual-VC envelope features for `instrument.*`:** Allowed — proof algorithm, issuer verification-method digest, issuance/expiry, dual-stack tag (W3C vs native), verification *success bit as envelope only*. **Forbidden in instrument→who path:** payload claims, subject-as-person label, “verified ⇒ true ⇒ who.”
4. **Additional hard-negatives:** Yes — treat **QLink ephemeral DH** ≠ **QSession traffic AEAD** as distinct `keyRole`s (`qlink-ephemeral-dh` vs `transport-aead`). Also: discovery-psk ≠ capability-presentation ≠ controller-signing; RAR route-update ≠ session-authentication (already in F2).

---


## 13. Changelog

| When | Note |
|------|------|
| 2026-09-06 | F1 initial taxonomy landed (Neo fold). |
| 2026-09-06 | F2 crypto skim: DNI/RAR/QSession purpose-separation. |
| 2026-09-06 | **Amend:** §1b hardness (multi-instrument time-bounded co-attestation); cross-link spine §1b + F6; closed `keyRole` enum; QLink DH vs QSession AEAD; answers to Alice F6 §5.1. |
| 2026-09-06 | **Amend §14:** FOAF-modern entity/agent types; AI-agent ≠ NaturalAgent ≠ machine; WebID/SAN/hardware multi-id; WN lexical≠plane; jury explainability. |

---

*End of WIP — Noddy F1 crypto instrument taxonomy (hardness + F6 cross-link amend).*
| 2026-09-06 | **Amend §15:** OS/telecom account ≠ device user ≠ machine; independent usedBy/accountOn/accountHolder. |
| 2026-09-06 | **Amend §16:** guardianship/capacity gradients; claim–policy–modality uplift; collaborative commons as multi-party instruments/relations (not shared who). |
| 2026-09-06 | **Amend §17:** sense-context bindings; flora/fauna living non-person; situational capacity grants vs logs/accountability; crypto≠homograph fix. |
| 2026-09-06 | **Amend §18:** ontology-governed crypto-bound policy naming; ZKP as proof instruments (selective disclosure ≠ who); HTTP/Solid offramp not trust root. |
| 2026-09-06 | **Amend §19:** relation-scoped locators as instruments (pairwise email sketch); ≠ static who. |
| 2026-09-06 | **Amend §20:** secrets · wallets · tokens · online accounts · passwords as instruments (≠ who). |
| 2026-09-06 | **Amend §21:** symbolic-first permission context; specialised bots via grants+agreements; crypto proves only. |
| 2026-09-06 | **Amend §22:** org structure + mutable group authentication (elections = rebind, not who). |
| 2026-09-06 | **Amend §23:** environment-scoped sensors · place-bound secrets · GIS predicates (≠ who). |
| 2026-09-06 | **Amend §24:** temporal relationship assessment · epistemic rule-breach (knew vs didn’t understand); ≠ who rewrite. |
