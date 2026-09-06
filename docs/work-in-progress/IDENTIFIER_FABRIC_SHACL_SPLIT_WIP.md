# WIP — Identifier Fabric SHACL-first split (F2)

**Status:** work-in-progress · **Not standards** · **Branch:** `0.0.36-dev`  
**Against tip:** F2 §25 `9c66f46` · F1 §26 `9c5d542` · spine §3n · architecture spine
**Owner:** Marvin (ontology / shapes) · **Taxonomy:** Noddy · **Fold/push:** Neo · **Ops:** Capt.  
**Standards baseline:** `docs/manuals/standards/qualia-decentralized-network-fabric/` (`identifier-resolution.md`, `ontological-contracts.md`, `cryptographic-profile.md`) · `docs/manuals/standards/shacl-first-vs-owl-ok-class-list.md`  
**Substrate:** uplift `crates/qualia-core-db` Identity / DID / VC / agency / governance primitives — **not** parallel invent.

**Constraint:** illustration + docs only until Cursor’s vibe delivery lands. No Host invent. No `ALL_BOUND` invent. Collapse of planes into one CS-style “identity” / Thing-wash of natural agents = **gate fail**.

---

## 1. Purpose

Encode Noddy’s four fabric planes + instrument kinds as a **SHACL-first shape pack** so that:

1. Living **natural agents** never land under `owl:Thing` as commodities (B-OWL-PERSON / B-OWL-NATURAL).
2. **Claims**, **spatiotemporal/route handles**, and **instruments** stay enumerable as distinct relations (QDNF role table remains authoritative for *what question an identifier answers*).
3. Poet / vibe diagnose copy and Alice’s inference feature spaces can cite stable shape IDs without re-bagging “who.”
4. Existing core-db / QDNF surfaces are **mapped and uplifted**, not replaced in one shot.

**Redefined sense (normative for this WIP):** a natural agent is not an identifier and is not reducible to any single DID, address, credential, biometric sample, or auth-subject bag. Instruments **relate**; they do not replace.

---

## 2. Non-goals

- Settling join-key policy (`did:q42` / observer DID stay **provisional** — topology/coord, not “who”).
- Host widen, Solid IdP, new binds, or dotted `qualia.*` invent.
- Replacing Noddy’s taxonomy or Capt’s architecture spine (this doc **implements shapes**; those remain SoT for kinds / serial F#).
- Claiming VC payload truth, route authority from claims, or global unique biometric “who.”

---

## 3. Framing rules (locked)

| Framing | Use for | Never |
|---------|---------|-------|
| **SHACL-first / living** | Natural agent; kinship/sacred relations; claims *about* persons/living as subjects; biometric *relation to* agent | `owl:Thing` subclass; “thing” chrome/diagnose |
| **OWL-ok / artifact** | Instrument envelopes (DID doc as artifact, VC envelope, QRC index, machine ID, digest, DNI/RAR records, capability tokens) | Calling these “the person” |
| **Mixed** | Position / placement; claim *structure* vs claim *subject*; biometric family (living link) vs instance (mutable sample artifact) | Flattening mixed into one plane |

Cite: `shacl-first-vs-owl-ok-class-list.md` · B-OWL-PERSON · B-OWL-NATURAL · B-OWL-LIFE-UPLIFT.

**Agent key ≠ human principal** (`ontological-contracts` §3) — shape as explicit non-entailment.

---

## 4. Shape namespaces (proposed — docs only)

| Prefix | IRI stub (illustrative) | Role |
|--------|-------------------------|------|
| `idf:` | `urn:qualia:idf:` | Identifier Fabric shape pack |
| `qdnf:` | existing QDNF terms | Role affinities (DID, QRC, DNI, alias, …) |
| `q42:` | existing | Volume / storage coordinates |
| `agency:` / core-ontologies | existing where present | Prefer uplift over new parallel vocab |

No opcode / Host allocation from this document.

---

## 5. Four plane shapes (first-class)

### 5.1 `idf:NaturalAgentShape` — SHACL-first

| Property / constraint | Cardinality / note |
|-----------------------|--------------------|
| `idf:plane` | constant `NaturalAgent` |
| `idf:framing` | `living-SHACL` |
| Forbidden | `rdfs:subClassOf` / `owl:Class` under `owl:Thing` as commodity framing |
| Optional | `idf:scale` ∈ {micro, meso, macro} for living entities (B-OWL-LIFE-UPLIFT) |
| Relations out | `idf:relatedByInstrument` → instrument node (0..*); never identity-merge |
| Join | Pairwise/contextual instruments expected; **no** required single DID |

**Gate fail:** treating one DID, VC subject, biometric instance, or observer/`did:q42` as the whole agent.

### 5.2 `idf:ClaimOpinionShape` — mixed

| Property / constraint | Cardinality / note |
|-----------------------|--------------------|
| `idf:plane` | constant `ClaimOpinion` |
| Structure | OWL-ok envelope ok (signed assertion, modality record) |
| `idf:about` | target may be NaturalAgent (SHACL-first subject) or resource |
| `idf:assertedBy` | issuer / speaker instrument or agent relation — not automatic “who” |
| `idf:modality` | epistemic / deontic / paraconsistent as already in core (cite, don’t invent) |
| VC note | Envelope = origin+integrity; payload claims stay on this plane |

**Gate fail:** claim truth ⇒ natural agent; claim ⇒ route authority.

**Hardness (cross-link F1 §1b):** multi-instrument time-bounded co-attestation is modeled as a **claim/attestation graph over instruments**, never as a property of `NaturalAgent`. See §5.5.

### 5.3 `idf:SpatiotemporalHandleShape` — mixed

| Property / constraint | Cardinality / note |
|-----------------------|--------------------|
| `idf:plane` | constant `SpatiotemporalHandle` |
| Affinity | DNI / path hint / Position / epoch — answers **where/how now** |
| Coords / CRS | artifact OWL-ok |
| `idf:places` | optional link to living or artifact *what* is placed (living side SHACL-first) |
| Stability | short-lived / topology-scoped preferred for route handles |

**Gate fail:** DNI/address as persistent “who”; path observation republished as controller fact without proof (Noddy §5.1).

### 5.4 `idf:InstrumentShape` (abstract) — mostly OWL-ok

| Property / constraint | Cardinality / note |
|-----------------------|--------------------|
| `idf:plane` | constant `Instrument` |
| `idf:instrumentKind` | enum/IRI from §6 |
| `idf:qdnfRole` | DID \| ContentId \| ResourceIRI \| QRC \| DNI \| Alias \| other |
| `idf:relatesTo` | 0..* targets (NaturalAgent, Claim, Handle, Resource) — **relation**, not merge |
| Crypto | cite cryptographic-profile; purpose-separated keys |

Concrete kinds specialize this shape (NodeShape + `sh:targetClass` or `sh:filter`).

---

## 5.5 `idf:CoAttestationBundleShape` — claim-plane hardness (not who)

**Cite:** Noddy F1 §1b · spine §1b · crypto profile purpose-separation · Alice F6 keyRole axes.

Hardness for human-centric systems is **not** one signature or one identifier. SHACL encodes it as a **bundle on the claim plane** that *relates* instruments (and optionally a NaturalAgent) without becoming that agent.

| Property / constraint | Note |
|-----------------------|------|
| `idf:plane` | `ClaimOpinion` (hardness attestation ≠ NaturalAgent) |
| `idf:framing` | Mixed: bundle structure OWL-ok; any living principal referenced stays SHACL-first via `idf:about` / relation — never Thing-wash |
| `idf:timeInterval` | Required bounded window (notBefore / expiresAt or equivalent) — stale co-attestation does not count |
| `idf:stablePrimitive` | Optional cite of non-negotiable baselines (e.g. time) — outside “clever” rewrite |
| `idf:attestationMember` | **sh:minCount 2** (links to instrument nodes: DID, machine ID, network/DNI, RAR, QSessionProof, VC envelope, …). Formula documents “enough ≥ 2”; a lone member cannot satisfy the bundle |
| `idf:memberKeyRole` | Each member carries `idf:keyRole` from F1 §5.11 closed enum — session-authentication ≠ route-update ≠ transport-aead ≠ controller-signing |
| `idf:keyRoleDiversity` | **SHOULD:** member `keyRole` values MUST NOT all be identical (hardness ≠ N copies of one role). Prefer ≥2 distinct roles in the window; document as SHACL sparql/JS constraint or review gate until native diversity facet exists |
| `idf:formula` | Documentation / optional structured recipe: *enough* independent verified signatures in the window — not a single auth bit; aligns with minCount 2 + role diversity |
| `idf:scopedTo` | Machines · networks · entities · agents (instruments + relations) — not a global anonymous bag |
| Forbidden entailments | Bundle success ⇒ NaturalAgent who; single member ⇒ whole bundle; all-identical keyRoles ⇒ hardness; session proof ⇒ route-update; VC verify ⇒ claim truth ⇒ who |

**Gate fail:** minting one “mega-identifier” or stronger single who from the formula. Hardness lives in **co-attestation across instruments and keyRoles** (plural members + plural roles).

**TTL sketch (illustrative):**

```turtle
idf:CoAttestationBundleShape a sh:NodeShape ;
  sh:property [
    sh:path idf:plane ;
    sh:hasValue idf:ClaimOpinion ;
    sh:minCount 1 ; sh:maxCount 1 ;
  ] ;
  sh:property [
    sh:path idf:timeInterval ;
    sh:minCount 1 ; sh:maxCount 1 ;
  ] ;
  sh:property [
    sh:path idf:attestationMember ;
    sh:node idf:InstrumentShape ;
    sh:minCount 2 ;
  ] .
  # No owl:Thing / NaturalAgent superclass.
  # Members keep idf:keyRole; diversity gate: not all roles identical (review/sparql until native facet).
```

## 6. Instrument kind shapes (map Noddy §5)

| Kind | Shape id | Framing | Key constraints |
|------|----------|---------|-----------------|
| Network address / locator | `idf:NetworkAddressShape` | Spatiotemporal + instrument | Not DID; not who; LIG only for legacy IP/DNS |
| Machine / device ID | `idf:MachineIdShape` | Artifact | Device ≠ human principal; link to agent is relation |
| DID / DID URL | `idf:DidInstrumentShape` | Artifact instrument | **DID ≠ identity**; pairwise/contextual ok; **require `idf:verificationRelationship`** ∈ {`authentication`, `capabilityInvocation`, `route-update`, …} — DID URL alone ≠ authority |
| QRC (`did:q42:`) | `idf:QrcShape` | Artifact index | **No crypto by itself**; not who; provisional join only |
| Verifiable credential | `idf:VerifiableCredentialShape` | Artifact → claim | Origin+integrity; subject ≠ automatic NaturalAgent who |
| Verifiable claim / opinion | `idf:ClaimOpinionShape` (plane) | Claim | Alias Assertions / provenance sit here or claim-adjacent |
| Biometric **family** | `idf:BiometricFamilyShape` | Living-safe instrument class | Durable *kind*; relates to NaturalAgent; never global unique who |
| Biometric **instance** | `idf:BiometricInstanceShape` | Mutable sample artifact | Drifts; family link required; sample ≠ timeless who |
| Content / strong digest | `idf:ContentDigestShape` | Artifact | Explicit algorithm; `q_hash` ≠ strong digest |
| DNI entry | `idf:DniShape` | Spatiotemporal route instrument | Answers **how now**; epoch/node/service scoped; **not** session auth; **not** route-update authority by itself |
| Route Advertisement Record | `idf:RarShape` | Spatiotemporal + controller-signed route set | Route-update verification relationship; sequence/epoch; **not** QSession authentication |
| QSession / session proof | `idf:QSessionProofShape` | Session instrument | Session `authentication` key purpose; **MUST NOT** entail route-update or DID controller authority unless separately authorized |
| Capability presentation | `idf:CapabilityPresentationShape` | Artifact authz | Bound to transcript; possession ≠ who; **not** controller-signing |
| Discovery PSK (pairwise) | `idf:DiscoveryPskShape` | Artifact discovery | Purpose-separated from long-term identity / controller signing |
| Group discovery | `idf:GroupDiscoveryShape` | Artifact discovery | Discriminated from pairwise PSK and capability presentation; never shares controller-signing role |

**Deprecated alias (do not use in new designs):** `idf:DniRarSessionShape` — lumped DNI·RAR·session; crypto profile purpose-separates route-update signing ≠ session authentication ≠ transport/AEAD. Prefer the three specialized shapes above (or abstract `idf:InstrumentShape` + `idf:instrumentKind` with `sh:in` and distinct `idf:keyRole` constraints).

**Abstract specialization pattern:** `idf:InstrumentShape` + `idf:instrumentKind` + optional `idf:keyRole` ∈ {`route-update`, `session-authentication`, `transport-aead`, `capability-presentation`, `discovery-psk`, `controller-signing`, …}. A design that treats session proof as route authority = **gate fail**.

**Uniform closed constraint (all kinds):** `idf:collapsesToWho` MUST NOT be entailed; SHACL `sh:not` / documentation gate for designs that merge kind → NaturalAgent identity.

### 6.1 Noddy crypto skim amend (2026-09-06)

Accepted mis-map: single `idf:DniRarSessionShape` risked collapsing purpose-separated keys. Split applied. Optional sharpenings also applied: DID verification relationship required/documented; capability / discovery PSK / group discovery discriminated.

---

## 7. Cross-plane relation vocabulary (minimal)

| Predicate | Domain → range | Meaning |
|-----------|----------------|---------|
| `idf:relatedByInstrument` | NaturalAgent → Instrument | Agent has instrument relation |
| `idf:about` | Claim → (NaturalAgent \| Resource \| …) | Claim subject |
| `idf:assertedBy` | Claim → (Instrument \| AgentRelation) | Issuer/speaker |
| `idf:places` | SpatiotemporalHandle → (NaturalAgent \| Resource) | What is located *now* |
| `idf:familyOf` | BiometricInstance → BiometricFamily | Instance belongs to family |
| `idf:bindsSampleOf` | BiometricInstance → NaturalAgent | Sensitive living relation — never identity merge |
| `idf:agentKeyOf` | Instrument (key) → ? | **Does not** entail NaturalAgent who (`ontological-contracts`) |
| `idf:verificationRelationship` | DidInstrument → relationship IRI/token | `authentication` / `capabilityInvocation` / `route-update` / … — DID URL alone ≠ authority |
| `idf:keyRole` | Instrument → role token | Purpose separation: route-update ≠ session-authentication ≠ transport-aead ≠ controller-signing |
| `idf:instrumentKind` | Instrument → kind IRI | Discriminates DNI / RAR / QSessionProof / capability / discovery variants |
| `idf:attestationMember` | CoAttestationBundle → Instrument | Member of time-bounded multi-instrument formula |
| `idf:memberKeyRole` | (bundle, member) → keyRole | Must match F1 closed enum; purpose-separated |
| `idf:timeInterval` | CoAttestationBundle → interval | Required validity window |

No `owl:sameAs` widening of authority (contracts §3). Session proof MUST NOT widen into route-update via sameAs or silent role merge. Co-attestation success MUST NOT entail NaturalAgent who.

---

## 8. Uplift map (core-db / QDNF → shapes)

| Existing surface | Maps to | Action |
|------------------|---------|--------|
| Observer DID / `did:q42` usage | `idf:QrcShape` (+ provisional topology note) | Document as coord; do not promote to NaturalAgent join |
| VC dual stack (W3C / native) | `idf:VerifiableCredentialShape` | Dual-VC honesty from uplift audit; envelope ≠ truth |
| Agency / fiduciary / guardianship vocab | NaturalAgent relations + capability instruments | Prefer core-ontologies terms; SHACL-first for persons |
| Volume / path handles | OWL-ok artifact (existing `q42:Volume`) | Out of fabric “who”; cite, don’t re-bag |
| Alias Assertions (QDNF) | Claim-adjacent / Alias under instrument+claim | Provenance required; never route authority alone |
| G-COORD Position / Realm | SpatiotemporalHandle + mixed Position | Living *what* vs artifact CRS per class list |
| DNI / RAR / QSession (QDNF) | `idf:DniShape` · `idf:RarShape` · `idf:QSessionProofShape` | Map separately; never one bag |
| Capability / discovery PSKs | `idf:CapabilityPresentationShape` · `idf:DiscoveryPskShape` · `idf:GroupDiscoveryShape` | Discriminate; no shared controller-signing role |
| Multi-instrument hardness (F1 §1b) | `idf:CoAttestationBundleShape` | Claim-plane graph over instruments; not NaturalAgent property |

Gaps only → Capt WIP / Vibe deltas — **no** Host invent from F2.

---

## 9. Gate checks (for Capt / Neo / UAT later)

1. No NaturalAgent NodeShape subclasses `owl:Thing` as commodity.
2. No chrome/diagnose string calls persons/living “things” (Vibe F5).
3. Biometric instance without family link = shape fail.
4. QRC / observer DID labeled topology/coord in docs — not “identity.”
5. VC verification success ≠ claim truth ≠ NaturalAgent who.
6. DNI ≠ RAR ≠ QSession proof (purpose-separated); session proof ≠ route-update ≠ DID controller authority unless separately authorized (cite crypto profile).
7. Inference feature spaces (Alice F6) keep who / claim / handle as **separate** dimensions.
8. Co-attestation bundle lives on claim plane; ≥2 members + time-bound + keyRole diversity (not all identical); never collapses to stronger single who (F1 §1b).
9. AI-agent ≠ NaturalAgent ≠ Machine; WebID/SAN/hardware are instruments; WN lexicalConcept ≠ fabric plane (F1 §14).
10. Device-user ≠ OS account-holder ≠ telecom subscriber ≠ machine (F1 §15); `usedBy` / `accountOn` / `accountHolder` independent.
11. Guardianship/capacity/commons are relation axioms — never guardian≡ward, capacity-as-who, or commons mega-who (F1 §16); modalities on claim–policy only.
12. Sense-context bindings (locale·era·community·provenance); flora/fauna living non-person; situational grants ≠ logs ≠ who (F1 §17).
13. Ontology-governed policy on claim–policy–modality; ZKP as proof instrument — never who (F1 §18).
14. Relation-scoped locators (pairwise/email/group/txn) are instruments — not static who-addresses (spine §3g).
15. Secrets/wallets/tokens/online accounts are instruments — never who (F1 §20); align §15/§19.
16. Symbolic permission context on claim–policy–modality; specialised AI-agents use grants/agreements — never who-merge (F1 §21).
17. Org structural roles + mutable group-auth membership are time-bounded relations/instruments — not who (spine §3j).

---

## 10. Example shape sketches (TTL — illustrative, not loaded)

```turtle
@prefix sh:   <http://www.w3.org/ns/shacl#> .
@prefix idf:  <urn:qualia:idf:> .
@prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .

idf:NaturalAgentShape a sh:NodeShape ;
  sh:closed false ;
  sh:property [
    sh:path idf:plane ;
    sh:hasValue idf:NaturalAgent ;
    sh:minCount 1 ; sh:maxCount 1 ;
  ] ;
  sh:property [
    sh:path idf:framing ;
    sh:hasValue idf:living-SHACL ;
    sh:minCount 1 ; sh:maxCount 1 ;
  ] ;
  sh:property [
    sh:path idf:relatedByInstrument ;
    sh:node idf:InstrumentShape ;
    sh:minCount 0 ;
  ] .
  # Intentionally NO owl:Thing superclass requirement.

idf:BiometricInstanceShape a sh:NodeShape ;
  sh:property [
    sh:path idf:familyOf ;
    sh:node idf:BiometricFamilyShape ;
    sh:minCount 1 ; sh:maxCount 1 ;
  ] ;
  sh:property [
    sh:path idf:bindsSampleOf ;
    sh:node idf:NaturalAgentShape ;
    sh:minCount 0 ; sh:maxCount 1 ;
  ] .
  # Sample drift allowed; never entails timeless who.
```

Full TTL fixtures → later under `shapes/` only when Capt./Neo unlock coding; until then keep in WIP.

---

## 11. Handoff

| Role | Next |
|------|------|
| **Neo** | Re-fold co-attestation amend over `docs/work-in-progress/IDENTIFIER_FABRIC_SHACL_SPLIT_WIP.md`; note on spine changelog; no `ALL_BOUND` invent |
| **Noddy** | Crypto skim accepted; optional re-check §6 DNI/RAR/QSession + capability discrimination |
| **Capt.** | Spine changelog: F2 amend (purpose-separated route/session shapes); blockers to Capt. |
| **Vibe** | F5 diagnose / `suggested_fix` can cite plane names; never who→claim/handle collapse; never “session = route authority” |
| **Alice** | F6 pressure-test separate feature spaces against §5–§7; keep keyRole dimensions distinct |
| **Marvin** | §14 entity/agent + AI-agent + WebID/SAN shapes; idle after Neo fold unless Capt asks more |
| **Vibe** | Diagnose: AI-agent ≠ person ≠ machine; jury-safe naming (F1 §14.4) |

---

## 12. Open questions (non-blocking)

1. Preferred durable NaturalAgent node IRI strategy vs pairwise-only graph (architecture §7 Q1).
2. Biometric family registry without global unique who.
3. When observer DID may graduate from provisional topology role.
4. Dual-VC Poet exposure honesty details (cite uplift audit).

---


## 14. Amend — FOAF-modern entity/agent type shapes (F1 §14)

**Cite:** F1 §14 tip `8724174` · Capt spine · Timothy FOAF-modern + WN · jury explainability.

### 14.1 Layering shapes (lexical ≠ plane ≠ who)

| Layer | Shape / link | Framing |
|-------|--------------|---------|
| Entity / agent **type** | `idf:EntityTypeShape` specializations below | Type predicates — not CS identity bag |
| Attributes / properties | hang off type via ordinary properties | Never who-replacement |
| Relation axioms | `idf:customerOf` · `idf:operatedBy` · `idf:owns` · … | Relations — not auth merge |
| Instruments | existing §6 + §14.3 | Relate to types |
| Lexical concepts | `idf:lexicalConcept` → WN/OMW synset/sense | Vocabulary only — **≠ fabric plane** |

**Gate fail:** WordNet `person`/`entity` sense used as NaturalAgent who or under `owl:Thing` commodity framing.

### 14.2 Type shapes (hard cuts)

#### `idf:NaturalAgentShape` (existing §5.1 — reinforced)
- Living human principal · SHACL-first · never FOAF Person-as-Thing
- Optional `idf:lexicalConcept` (WN sense) for *labels* only
- Instruments relate via `idf:relatedByInstrument` — no required single DID

#### `idf:AiAgentShape` — distinct agent-type plane
| Property | Note |
|----------|------|
| `idf:plane` / type | `AiAgent` — **not** NaturalAgent, **not** Machine |
| Framing | Artifact/agent-system (OWL-ok for the runtime artifact); **operators/customers** who are humans stay SHACL-first via relations |
| `idf:identifierArray` | 0..* instruments (DID, keys, service IDs, …) — array ≠ who |
| `idf:customerOf` / `idf:operatedBy` / `idf:delegatedBy` | Relations to NaturalAgent / Organization — never identity merge |
| Agent key | `idf:agentKeyOf` **does not** entail human principal (`ontological-contracts`) |

**Gate fail:** AI-agent = person; AI-agent = machine; operator key = NaturalAgent who.

#### `idf:MachineDeviceShape` — artifact bundle
| Property | Note |
|----------|------|
| Type | Machine/device — artifact OWL-ok |
| `idf:hasHardwareId` | 1..* → `idf:HardwareIdShape` |
| `idf:hasNetworkId` | 0..* → network address / DNI path instruments |
| Bundle | Machine = **related instruments**, not one id = person |
| Hardness | Eligible as `idf:attestationMember` set inside `idf:CoAttestationBundleShape` (§5.5) |

**Gate fail:** machine-id = NaturalAgent; single hardware id = whole machine who-bag.

#### `idf:OrganizationShape` / `idf:ServiceShape`
- Own DID/VC/instrument set · artifact or mixed
- Do not equate with NaturalAgent
- Optional lexicalConcept for labels only

### 14.3 Additional instrument shapes (WebID / SAN / hardware)

| Kind | Shape id | Notes |
|------|----------|-------|
| Hardware identifier | `idf:HardwareIdShape` | TPM/serial/attestation handle — artifact; ≠ who |
| Network identifier | (existing NetworkAddress / DNI path) | Addresses & adjacency — ≠ who |
| Certificate SAN | `idf:CertSanShape` | SAN entries as instrument bindings |
| WebID-TLS | `idf:WebIdTlsShape` | Legacy/Solid-era TLS WebID binding — **instrument** |
| WebID-RSA | `idf:WebIdRsaShape` | RSA WebID pattern — **instrument** |
| Signed RDF / VC | existing VC + Claim shapes | Origin≠truth |

All: `idf:relatesTo` entity/agent type nodes; collapse to who = **gate fail**.

### 14.4 WordNet / OMW binding (docs only)

```turtle
idf:NaturalAgentShape sh:property [
  sh:path idf:lexicalConcept ;
  sh:datatype xsd:anyURI ;  # WN/OMW sense IRI
  sh:minCount 0 ;
] .
# lexicalConcept MUST NOT entail idf:plane NaturalAgent identity merge
# Subset packs: volume-backed LexiconPack citing WN; upliftFrom prior pack SemVer
```

Recommend: volume-backed **subset** LexiconPack (≤ WN full size; often ≪ 100MB encoded) with `upliftFrom` + localeSurfaces (OMW) — concept ids stable; fabric planes separate.

### 14.5 Jury / audit explainability (shape constraint on docs)

Evidence surfaces SHOULD enumerate **who · claim · handle · instrument** in plain language (cite F5). Co-attestation presents “these signatures, this window, these machines/networks/agents” — never an opaque who-token. WebID/SAN/hardware/VC named as **tools**.

### 14.6 TTL sketches (illustrative)

```turtle
idf:AiAgentShape a sh:NodeShape ;
  sh:property [ sh:path idf:agentType ; sh:hasValue idf:AiAgent ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:identifierArray ; sh:node idf:InstrumentShape ; sh:minCount 0 ] ;
  sh:property [ sh:path idf:operatedBy ; sh:node idf:NaturalAgentShape ; sh:minCount 0 ] .
  # NOT NaturalAgentShape; NOT MachineDeviceShape.

idf:MachineDeviceShape a sh:NodeShape ;
  sh:property [ sh:path idf:hasHardwareId ; sh:node idf:HardwareIdShape ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:hasNetworkId ; sh:minCount 0 ] .

idf:WebIdTlsShape a sh:NodeShape ;
  sh:property [ sh:path idf:instrumentKind ; sh:hasValue idf:WebIdTls ] ;
  sh:property [ sh:path idf:relatesTo ; sh:minCount 0 ] .
  # Instrument only — never NaturalAgent who.
```

---

## 15. Amend — OS / telecom account ≠ device user (F1 §15)

**Cite:** F1 §15 tip `e4c6320` · Timothy room · MachineDevice §14.2 · agent key ≠ human principal.

### 15.1 Hard independence (three ≠)

| Role | Shape | Must not entail |
|------|-------|-----------------|
| **Device user** | NaturalAgent or AI-agent *via* `idf:usedBy` | OS account · subscriber · machine |
| **OS account** | `idf:OsAccountShape` (instrument) | NaturalAgent who · “person holding device” |
| **Telecom subscriber / account** | `idf:TelecomSubscriberShape` (instrument) | Handset user · NaturalAgent who |
| **Machine** | `idf:MachineDeviceShape` | Who · account · subscriber |

**Gate fail:** OS UID / login session / IMSI / MSISDN / subscriber id as NaturalAgent who; “logged-in account” = “person holding the device.”

### 15.2 Instrument shapes

#### `idf:OsAccountShape`
| Property | Note |
|----------|------|
| `idf:instrumentKind` | `OsAccount` |
| Framing | Artifact instrument on a machine |
| `idf:accountOn` | → MachineDevice (required) — account lives *on* that OS/host |
| `idf:accountHolder` | 0..1 → NaturalAgent or AI-agent — **independent** of device user |
| `idf:accountUid` / login label | Local identifier material — not who |
| Session | Optional link to login/session instrument — still ≠ device user |

#### `idf:TelecomSubscriberShape`
| Property | Note |
|----------|------|
| `idf:instrumentKind` | `TelecomSubscriber` |
| Framing | Artifact instrument (IMSI/MSISDN/subscriber record) |
| `idf:accountHolder` | 0..1 → agent — subscriber of record |
| `idf:boundHandset` / relatesTo machine | 0..* — optional; handset user may differ |
| Collapse | Subscriber id ≠ NaturalAgent who |

### 15.3 Relation axioms (independent)

| Predicate | Domain → range | Meaning |
|-----------|----------------|---------|
| `idf:usedBy` | MachineDevice → (NaturalAgent \| AiAgent) | Who is *using* the device now/then |
| `idf:accountOn` | OsAccount → MachineDevice | Account hosted on machine |
| `idf:accountHolder` | (OsAccount \| TelecomSubscriber) → agent | Account/subscriber of record |
| `idf:boundHandset` | TelecomSubscriber → MachineDevice | Optional device binding |

These three edges **MUST NOT** be inferred from each other. SHACL: no rule that `accountHolder` entails `usedBy` or vice versa.

### 15.4 Jury / diagnose voice

Name separately: “this OS account,” “this subscriber identity,” “this person using the device,” “this machine” — never one opaque identity (F1 §15 · F5).

### 15.5 TTL sketch (illustrative)

```turtle
idf:OsAccountShape a sh:NodeShape ;
  sh:property [ sh:path idf:instrumentKind ; sh:hasValue idf:OsAccount ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:accountOn ; sh:node idf:MachineDeviceShape ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:accountHolder ; sh:minCount 0 ; sh:maxCount 1 ] .
  # accountHolder MUST NOT entail idf:usedBy on the same machine.

idf:TelecomSubscriberShape a sh:NodeShape ;
  sh:property [ sh:path idf:instrumentKind ; sh:hasValue idf:TelecomSubscriber ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:accountHolder ; sh:minCount 0 ; sh:maxCount 1 ] .
```

---

## 16. Amend — guardianship, capacity, commons relations (F1 §16)

**Cite:** Noddy F1 §16 tip `4337c9a` · Capt lock · core-ontologies agency/fiduciary · QDNF commons/contracts · co-attestation §5.5.

**Uplift first:** prefer existing agency / fiduciary / guardianship vocab in `core-ontologies` + QDNF `commons-and-resource-economics` / `ontological-contracts` — do not parallel-invent trust roots.

### 16.1 Modalities stay off who

| Substrate | Shape home | Forbidden |
|-----------|------------|-----------|
| N3 / deontic / epistemic / bifurcation | Claim–policy–modality shapes (extend ClaimOpinion / contract bundles) | Attaching as NaturalAgent “identity” |
| Crypto packages | Instrument + `CoAttestationBundleShape` | New identity stack |

### 16.2 `idf:GuardianshipRelationShape`

First-class **relation** among agents — never a who-merge.

| Property | Note |
|----------|------|
| `idf:guardian` | → NaturalAgent \| AiAgent \| Organization — distinct node |
| `idf:ward` | → NaturalAgent (typical) \| AiAgent — **MUST NOT** same node as guardian; no `owl:sameAs` |
| `idf:scope` | medical · financial · digital-instruments · … |
| `idf:timeInterval` | Required validity window (stale grant does not count) |
| `idf:grantEvidence` | 0..* claim or instrument proving the relation |
| Framing | Living parties SHACL-first; relation structure OWL-ok |

**Gate fail:** guardian≡ward; treating guardianship edge as identity of either party.

### 16.3 Capacity / personhood attributes (gradients)

| Shape / property | Note |
|------------------|------|
| `idf:capacityAttribute` on NaturalAgent | Graduated, **time-varying** attributes — not a who flip |
| Developmental pattern | Child is NaturalAgent from the start; capacities *granted slowly* — not “becomes person later” via ID |
| Elder / disability | Capacity may narrow or be shared via guardianship — still NaturalAgent |
| Organization / legal personality | `OrganizationShape` — not NaturalAgent |
| Artifacts / machines | OWL-ok — never living who |

**Gate fail:** capacity score as who; corporate veil as natural person.

### 16.4 Commons / collaborative project relations

| Shape | Note |
|-------|------|
| `idf:CommonsMembershipShape` | member · contributor · steward · licensee — time-bounded, scoped |
| `idf:ProjectRoleShape` | Role on collaborative informatics among 2+ agents/entities |
| Shared instruments | Keys, VCs, volumes, contract bundles — remain instruments |
| Shared claim–policy | Ontology-defined contracts / deontic grants / N3 — claim plane |
| Optional hardness | Parties’ instruments in `CoAttestationBundleShape` — no mega-who |

**Gate fail:** collapsing commons membership into one shared who / project-identity bag.

### 16.5 TTL sketches (illustrative)

```turtle
idf:GuardianshipRelationShape a sh:NodeShape ;
  sh:property [ sh:path idf:guardian ; sh:minCount 1 ; sh:maxCount 1 ] ;
  sh:property [ sh:path idf:ward ; sh:minCount 1 ; sh:maxCount 1 ] ;
  sh:property [ sh:path idf:scope ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:timeInterval ; sh:minCount 1 ] ;
  sh:sparql [
    sh:message "guardian MUST NOT be ward" ;
    sh:select """
      SELECT $this WHERE {
        $this idf:guardian ?g ; idf:ward ?w .
        FILTER (sameTerm(?g, ?w))
      }
    """ ;
  ] .

idf:CommonsMembershipShape a sh:NodeShape ;
  sh:property [ sh:path idf:member ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:role ; sh:in (idf:member idf:contributor idf:steward idf:licensee) ] ;
  sh:property [ sh:path idf:timeInterval ; sh:minCount 0 ] .
```

### 16.6 Diagnose / jury voice (for Vibe)

Guardianship = **relation**; modalities ≠ person voice; capacity = attributes over time; commons = scoped membership — enumerable resolution, never opaque who.

---

## 17. Amend — sense-context, flora/fauna, situational grants vs logs (F1 §17)

**Cite:** Noddy F1 §17 tip `bec69a7` · Capt sense-contextual lock · Timothy thongs/gay · emergency medical capacity · F5 `da74019` · F2 §14 lexicalConcept · §16 capacity/guardianship · QDNF Alias Assertions.

### 17.1 `idf:SenseContextBindingShape` (lexical — not mega-meaning who)

| Property | Note |
|----------|------|
| `idf:lexicalConcept` | WN/OMW sense IRI — stable concept id; **≠ fabric plane** |
| `idf:locale` | e.g. `en-AU` — *thongs* = footwear |
| `idf:era` / time window | Historical *gay* = happy ≠ sexuality |
| `idf:community` / namespace | QDNF alias namespaces: personal · relationship · community · institution · legacy |
| `idf:provenance` | Who asserted this binding; evidence instrument/claim |
| Surface form | Optional sayable / orthography |

**Gate fail:** one timeless dictionary identity; sense without locale/era when homograph risk; WN gloss as NaturalAgent or plane. Crypto does **not** disambiguate homographs.

### 17.2 `idf:FloraShape` / `idf:FaunaShape` (living non-person)

| Constraint | Note |
|------------|------|
| Framing | SHACL-first living (B-OWL-NATURAL) — micro·meso·macro as needed |
| Forbidden | NaturalAgent personhood; `owl:Thing` commodity wash |
| Optional | `idf:lexicalConcept` + SenseContextBinding for common names |

### 17.3 `idf:SituationalCapacityGrantShape`

Purpose-scoped · condition-scoped · time-bounded · qualification-backed — still the same NaturalAgent (or acting agent-type).

| Property | Note |
|----------|------|
| `idf:grantee` | NaturalAgent \| AiAgent — **not** a new who |
| `idf:purpose` | e.g. emergency clinical data access |
| `idf:condition` | e.g. co-location / proximity constraint |
| `idf:timeInterval` | Required — stale grant does not count |
| `idf:qualificationInstrument` | VC / license / credential instrument |
| Distinct from | Standing `GuardianshipRelationShape` (§16); `OsAccountShape` (§15); session login |
| Evidence | Qualification + condition observations — co-attestation eligible when multi-party |

**Gate fail:** freezing informal emergency trust into one identity privilege bit; grant ≡ who.

Makes historically informal practice **enumerable** without who-merge.

### 17.4 Logs / accountability — distinct track

| Track | Shape home | Answers |
|-------|------------|---------|
| Situational grant | Capacity / claim–policy relation (§17.3) | What *may* be done, by whom, under what conditions, when |
| Logs / accountability | Provenance · claim–evidence (`idf:AccountabilityLogShape` sketch) | What *was* done, observed, attested |

Grants authorize; logs account. Neither is who. Jury-safe: name grant scope and log evidence separately.

```turtle
idf:SenseContextBindingShape a sh:NodeShape ;
  sh:property [ sh:path idf:lexicalConcept ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:locale ; sh:minCount 0 ] ;
  sh:property [ sh:path idf:era ; sh:minCount 0 ] ;
  sh:property [ sh:path idf:provenance ; sh:minCount 0 ] .

idf:SituationalCapacityGrantShape a sh:NodeShape ;
  sh:property [ sh:path idf:grantee ; sh:minCount 1 ; sh:maxCount 1 ] ;
  sh:property [ sh:path idf:purpose ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:condition ; sh:minCount 0 ] ;
  sh:property [ sh:path idf:timeInterval ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:qualificationInstrument ; sh:node idf:InstrumentShape ; sh:minCount 0 ] .

idf:FloraShape a sh:NodeShape ;
  sh:property [ sh:path idf:framing ; sh:hasValue idf:living-SHACL ; sh:minCount 1 ] .
  # NOT NaturalAgentShape; NOT owl:Thing commodity.
```

### 17.5 Diagnose alignment (Vibe F5)

`suggested_fix` optional fields: `locale` · `era` · `community` · `provenance` beside `plane`/`framing`. Collapse detectors: AU thongs; period gay; flora/fauna as person; WN gloss as who.

---

## 18. Amend — ontology-governed policy + ZKP instruments (F1 §18)

**Cite:** Noddy F1 §18 tip `13c3844` · Capt ABAC/ZKP lock · QDNF ontological-contracts / QPolicy · §1b/§5.5 co-attestation · §16 modalities · §17 situational grants · core-db crypto/ZKP uplift.

### 18.1 Runtime framing (not identity product)

Authorization runs over an **unmerged multi-plane graph** (types · relations · instruments · claims · handles) with:

- Attribute / capacity-grant inputs (ABAC-like)
- Deontic / epistemic / N3 / bifurcation on **claim–policy–modality** shapes
- Cryptographically signed ontology documents (contracts · SHACL/N3 bundles) that **interpret** policy — signature binds interpretation
- Instruments for possession / integrity / co-attestation / ZKP
- HTTP / Solid as offramp (LIG) — **not** trust root

Who stays unmerged. Name: ontology-governed, crypto-bound policy.

### 18.2 `idf:OntologyGovernedPolicyShape` (claim–policy–modality)

| Property | Note |
|----------|------|
| Plane | Claim–policy–modality — **not** NaturalAgent |
| `idf:policyBundle` | Pinned ontology / SHACL / N3 / contract digest (cite ontological-contracts) |
| `idf:interpretationBinding` | Signature covers exact interpretation bytes/bundle — no silent re-interpret |
| Inputs | Attributes · capacity grants · instrument proofs — referenced, not who-merged |
| Forbidden | Policy success ⇒ NaturalAgent who; HTTP endpoint as trust root |

### 18.3 `idf:ZkpProofShape` (instrument / proof kind)

Uplift existing QualiaDB ZKP packages — do not reinvent algorithms here.

| Property | Note |
|----------|------|
| `idf:instrumentKind` | `ZkpProof` |
| Framing | Artifact proof instrument |
| `idf:circuitOrStatement` | Defined circuit/context — what is proven |
| `idf:provesPredicate` | Predicate satisfaction / selective disclosure / membership / capacity check |
| `idf:relatesTo` | Grant, commons membership, co-attestation member, etc. — **relation** |
| Use with | SituationalCapacityGrant (§17); CommonsMembership (§16); CoAttestationBundle members (§5.5) |

**Gate fail:** ZKP as NaturalAgent identity; cross-context proof correlation into one who; valid ZKP ⇒ personhood / moral claim truth / global anonymity.

**Security claim boundary:** proves a statement in a defined circuit/context only.

```turtle
idf:ZkpProofShape a sh:NodeShape ;
  sh:property [ sh:path idf:instrumentKind ; sh:hasValue idf:ZkpProof ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:circuitOrStatement ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:relatesTo ; sh:minCount 0 ] .
  # MUST NOT entail NaturalAgent who.

idf:OntologyGovernedPolicyShape a sh:NodeShape ;
  sh:property [ sh:path idf:plane ; sh:hasValue idf:ClaimOpinion ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:policyBundle ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:interpretationBinding ; sh:minCount 0 ] .
```

### 18.4 Alice / Vibe notes

- ZKP outputs → typed `instrument` / `proof` namespaces — never who embedding (Alice F6).
- Diagnose: “proof of predicate in context,” not “proved identity.”

---

## 19. Amend — relation-scoped locators (spine §3g)

**Cite:** Capt spine §3g tip `1d55f56` · Timothy email sketch · QDNF Alias / contextual IRI / DNI mobility · F2 NetworkAddress / Alias · Solid/HTTP offramp only.

### 19.1 Cut

Traditional Solid WebIDs / phone numbers often behave as **static** personal addresses. This fabric prefers **relation-specific** locator strings: the address names a *relationship* (or context), not a permanent who-token.

| Pattern | Shape role |
|---------|------------|
| Pairwise / contextual locator | Two people, group, group-chat, transaction id, DNS TXT code, … |
| Email redesign sketch | User-controlled domain (or equivalent); directed scoped receive addresses — `jane@bob.tld` ↔ `bob@jane.tld` |
| Agents of entities | Metadata/semantics on the relation — not the mailbox who |

**Gate fail:** static Solid/phone-style “one address forever” as NaturalAgent who; HTTP/Solid re-imposed as trust root.

### 19.2 `idf:RelationScopedLocatorShape`

| Property | Note |
|----------|------|
| `idf:instrumentKind` | `RelationScopedLocator` (affinity: alias / contextual IRI / DNI-like mobility) |
| Framing | Artifact instrument / handle — **not** who |
| `idf:locatorString` | The relation-scoped address string |
| `idf:relationKind` | pairwise · group · group-chat · transaction · dns-code · email-scoped · … |
| `idf:partyA` / `idf:partyB` | 0..* agent or entity nodes in the relation (directed edges ok) |
| `idf:direction` | Optional — e.g. Jane→Bob vs Bob→Jane as **two** locator instruments |
| `idf:controlledDomain` | Optional — user-controlled domain (or equivalent) hosting the receive side |
| `idf:agentMetadata` | 0..* AI-agent / delegate refs in semantics — not mailbox who |
| Distinct from | Static WebID-as-who; phone MSISDN as who (§15 telecom subscriber still instrument); DNI how-now |

```turtle
idf:RelationScopedLocatorShape a sh:NodeShape ;
  sh:property [ sh:path idf:instrumentKind ; sh:hasValue idf:RelationScopedLocator ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:locatorString ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:relationKind ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:partyA ; sh:minCount 0 ] ;
  sh:property [ sh:path idf:partyB ; sh:minCount 0 ] .
  # MUST NOT entail NaturalAgent who. Directed pairwise email = two locator nodes.
```

### 19.3 Email sketch (enumerable, not who)

| Locator | Relates |
|---------|---------|
| `jane@bob.tld` | Jane’s send/receive context **toward** Bob’s domain |
| `bob@jane.tld` | Bob’s send/receive context **toward** Jane’s domain |

Two instruments, one relation; neither string *is* Jane or Bob.

### 19.4 Diagnose / Alice notes

- Speak: “relation locator,” never “your identity address.”
- Collapse: locator success ⇒ who → suggested_form splits locator vs NaturalAgent.
- F6: keep locator features in `instrument.locator.*` — never who embedding.

---

## 20. Amend — secrets, wallets, tokens, online accounts (F1 §20)

**Cite:** Noddy F1 §20 draft · spine §3h · F2 §15 OsAccount · §19 RelationScopedLocator · §14 WebID/SAN · crypto profile vault/keyRoles.

### 20.1 Cut

Secrets · wallets · tokens · online accounts · passwords · per-account emails are **instruments** (and account relations), **not** NaturalAgent who.

### 20.2 Instrument shapes

| Kind | Shape id | Notes |
|------|----------|-------|
| Password / passphrase | `idf:PasswordVerifierShape` | Low-entropy authenticator aid — not key material unless PAKE-specified; vault-scoped |
| Bearer / API / session token | `idf:BearerTokenShape` | Time-bound capability instrument; possession ≠ who |
| Refresh / OAuth token | `idf:OAuthTokenShape` | Account-scoped on online-account relation |
| Wallet (software/hardware) | `idf:WalletShape` | Artifact container of keys/instruments; may *relate* to agent |
| Private key / seed | `idf:PrivateKeyMaterialShape` | Purpose-separated keyRole; vault only — never Quins/logs/who |
| Online account | `idf:OnlineAccountShape` | Remote platform account (sibling to OsAccount); holder ≠ device-user ≠ who-forever |
| Per-account email | Prefer `idf:RelationScopedLocatorShape` (§19) bound to that account | Not the person’s only identity |

### 20.3 `idf:OnlineAccountShape` / `idf:WalletShape` (detail)

| Property | OnlineAccount | Wallet |
|----------|---------------|--------|
| `idf:accountAt` / service | → service/org (required) | — |
| `idf:accountHolder` | 0..1 agent of record | optional controller agent |
| `idf:walletControls` | — | → keys/instruments held |
| `idf:relatedByInstrument` | from NaturalAgent/AI-agent | from NaturalAgent/AI-agent |
| Independent of | `usedBy` (device user now) | NaturalAgent who |

**No entailment:** password-success · token-possession · wallet-unlock · online-account login ⇒ NaturalAgent who (same discipline as §17 grant / §18 ZKP).

### 20.4 Vault & jury hygiene

- Secrets/private keys: Qualia key vault / hardware-backed — never Quins, logs, DHT, crash dumps.
- Jury: name “this password verifier,” “this wallet,” “this platform account,” “this per-account mailbox” separately.
- Hardness: wallet/keys may be co-attestation **members with distinct keyRoles** — not mega-who.

```turtle
idf:OnlineAccountShape a sh:NodeShape ;
  sh:property [ sh:path idf:instrumentKind ; sh:hasValue idf:OnlineAccount ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:accountAt ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:accountHolder ; sh:minCount 0 ; sh:maxCount 1 ] .

idf:WalletShape a sh:NodeShape ;
  sh:property [ sh:path idf:instrumentKind ; sh:hasValue idf:Wallet ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:walletControls ; sh:node idf:InstrumentShape ; sh:minCount 0 ] .

idf:PasswordVerifierShape a sh:NodeShape ;
  sh:property [ sh:path idf:instrumentKind ; sh:hasValue idf:PasswordVerifier ; sh:minCount 1 ] .
  # MUST NOT entail NaturalAgent who.
```

### 20.5 Gate fails / diagnose

- Password/token/wallet = person; one email forever = who; online-account success ⇒ who; seed phrase as identity in chrome.
- Vibe: secret/wallet/token/account = instruments. Alice: never embed secrets/tokens into who feature space.

---

## 21. Amend — symbolic-first context for agent permissions (F1 §21)

**Cite:** Noddy F1 §21 tip `e626613` · Capt/Timothy symbolic AI context · Alice F6 · F2 §16–§18 · §17 situational grants · §20 secrets/wallets · OntologyGovernedPolicyShape.

### 21.1 Decision stack (shapes)

| Layer | Shape home | Does |
|-------|------------|------|
| Symbolic context | Claim–policy–modality (+ optional `idf:SymbolicPermissionContextShape`) | Enumerates *which* context and *what* may be done (semantics · deontic/epistemic/N3; probability only where uncertainty belongs) |
| Attributes · capacity grants · agreements | SituationalCapacityGrant · Guardianship · Commons · agreement instruments | ABAC-like inputs |
| Crypto / ZKP | ZkpProofShape · wallets/keys · co-attestation | Prove possession / predicates / hardness — **not** who |
| Multi-plane graph | NaturalAgent · claim · handle · instrument | Keep unmerged |

**Gate fail:** NaturalAgent embedding that absorbed role, wallet, account, grant-success, or ZKP-success.

### 21.2 `idf:SymbolicPermissionContextShape`

| Property | Note |
|----------|------|
| Plane | Claim–policy–modality — **not** NaturalAgent |
| `idf:contextFrame` | Situation / domain (home · clinical · child-care · …) |
| `idf:mayDo` / permissions | Enumerated allowable actions under this context |
| `idf:inputs` | 0..* attributes, grants, agreement instruments, proof instruments |
| `idf:policyBundle` | Optional link to OntologyGovernedPolicyShape |
| Probability | Only as explicit uncertainty modality — never as who score |

### 21.3 Specialised agents (AI-agent + grants)

Child-minder · medical support · home-security · personal/group agents:

- Type: `AiAgentShape` (or service/org) — **not** NaturalAgent who, **not** merge with ward/patient/homeowner
- Authorization: SituationalCapacityGrant + signed agreement instruments (+ optional ZKP predicates)
- Relation axioms stay independent (operatedBy · customerOf · guardianship · usedBy)

### 21.4 UI bar (docs intent only)

When planes are thick enough: chrome speaks **instruments · grants · context**, not CS identity bags — easier than AD/Keychain. No Host invent now.

```turtle
idf:SymbolicPermissionContextShape a sh:NodeShape ;
  sh:property [ sh:path idf:plane ; sh:hasValue idf:ClaimOpinion ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:contextFrame ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:inputs ; sh:minCount 0 ] .
  # MUST NOT entail NaturalAgent who from grant/ZKP success.
```

### 21.5 Alice / Vibe

- F6: symbolic context namespaces; instrument-only for secrets/grants/ZKP.
- Diagnose: “permission in context,” never “agent identity cleared.”

---

## 22. Amend — org structure + mutable group authentication (spine §3j)

**Cite:** Capt spine §3j tip `78d3878` · §3i symbolic permissions · §3d guardianship/capacity · OrganizationShape §14 · CommonsMembership §16 · CoAttestationBundle §5.5.

### 22.1 Two correlated but unmerged tracks

| Track | Shape home | Notes |
|-------|------------|-------|
| **Structural elements** | Relation axioms on Organization / legal-personality | Directors · department leads · org-chart roles — *relations* to orgs, not who-tokens |
| **Group authentication** | Membership / instruments / co-attestation | Current holders’ instrument set — **time-bounded**, re-bound on election/turnover |

**Gate fail:** embedding “the board” as permanent NaturalAgent who; election winner identity-merge; structure role = person forever.

### 22.2 `idf:OrgStructuralRoleShape`

| Property | Note |
|----------|------|
| `idf:roleKind` | director · dept-lead · officer · committee-member · … |
| `idf:organization` | → OrganizationShape / legal personality (required) |
| `idf:holder` | 0..1 NaturalAgent \| AiAgent — current holder via relation |
| `idf:timeInterval` | Role occupancy window (turnover without rewriting who) |
| Framing | Relation axiom — holder remains NaturalAgent unmerged |

### 22.3 `idf:GroupAuthMembershipShape`

| Property | Note |
|----------|------|
| `idf:group` | Org / board / electorate / auth-set |
| `idf:memberInstrument` | 1..* instruments of **current** authenticators (keys, VCs, locators, …) |
| `idf:memberAgent` | 0..* agents currently bound — via relation, not who-bag |
| `idf:timeInterval` | Required — post-election rebind = new interval / new membership node |
| Optional hardness | Members may enter CoAttestationBundle with distinct keyRoles |

Election / turnover: **new bindings in time** — not rewriting NaturalAgent identity of prior holders.

```turtle
idf:OrgStructuralRoleShape a sh:NodeShape ;
  sh:property [ sh:path idf:organization ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:roleKind ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:holder ; sh:minCount 0 ; sh:maxCount 1 ] ;
  sh:property [ sh:path idf:timeInterval ; sh:minCount 0 ] .

idf:GroupAuthMembershipShape a sh:NodeShape ;
  sh:property [ sh:path idf:group ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:memberInstrument ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:timeInterval ; sh:minCount 1 ] .
  # MUST NOT entail permanent NaturalAgent who for "the board."
```

### 22.4 Alignments

- SymbolicPermissionContext (§21) enumerates *which* org context and *what* may be done.
- Guardianship/capacity (§16): same time-varying relation pattern.
- Diagnose: “this role occupancy,” “this group-auth set (epoch),” never “board identity.”

---

## 23. Amend — environment-scoped sensors & place-bound secrets (F1 §23)

**Cite:** Noddy F1 §23 draft · Timothy GIS/sensor/geocache/ATM BLE · §17 situational grants · spatiotemporal handles · §20 secrets · §5.5 co-attestation · G-COORD Position.

### 23.1 Cut

Sensor / GIS / network-environment identifiers condition *how* capacity and proofs apply — instruments + spatiotemporal handles, **not** NaturalAgent who.

| Pattern | Shape expression |
|---------|------------------|
| Works only in environment E | SituationalCapacityGrant / SymbolicPermissionContext + `EnvironmentPredicate` |
| Different environment | Flag, degrade, or deny — do not silently reuse grant |
| Geocaching | `PlaceBoundSecretShape` discoverable when location/handle predicates match |
| ATM + phone bank | Co-presence: app + ATM BLE/machine id + location → higher-assurance capacity via co-attestation |

### 23.2 Shapes

| Kind | Shape id | Notes |
|------|----------|-------|
| Sensor identifier | `idf:SensorIdShape` | BLE/NFC/radio/… — observation ≠ who |
| GIS / geofence binding | `idf:GisEnvironmentBindingShape` | Spatiotemporal handle (+ optional Position mixed typing) |
| Place-bound secret | `idf:PlaceBoundSecretShape` | Secret instrument + discovery predicates (location · time · co-attestation) |
| Environment attestation | `idf:EnvironmentAttestationShape` | Claim/evidence: “device X observed sensor Y at locus Z” — not who |
| Environment predicate | `idf:EnvironmentPredicateShape` | Constraint pack attached to grants/policy (GIS · realm · network cell · sensor id) |

### 23.3 Detail

#### `idf:SensorIdShape`
- Artifact instrument; may relate to MachineDevice
- Spoofable alone (BLE/MAC) — weak unless in multi-instrument formula

#### `idf:PlaceBoundSecretShape`
- Extends secret instrument (§20) with required discovery predicates
- Unlock ≠ NaturalAgent who

#### `idf:EnvironmentPredicateShape`
- Inputs to SituationalCapacityGrant / OntologyGovernedPolicy / SymbolicPermissionContext
- Change of environment → re-evaluate; stale env grant MUST NOT silently pass

```turtle
idf:SensorIdShape a sh:NodeShape ;
  sh:property [ sh:path idf:instrumentKind ; sh:hasValue idf:SensorId ; sh:minCount 1 ] .

idf:PlaceBoundSecretShape a sh:NodeShape ;
  sh:property [ sh:path idf:instrumentKind ; sh:hasValue idf:PlaceBoundSecret ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:discoveryPredicate ; sh:minCount 1 ] .

idf:EnvironmentPredicateShape a sh:NodeShape ;
  sh:property [ sh:path idf:envConstraint ; sh:minCount 1 ] .
  # Attach to grants/policy — MUST NOT entail NaturalAgent who.
```

### 23.4 Crypto / gate fails

- Prefer multi-instrument co-attestation (phone key · ATM BLE · location · time) for banking/geocache hardness.
- Gate fail: ATM BLE = customer who; geocache find = person identity; GIS alone = forever who.
- Alice: sensor/GIS features = handle/instrument namespaces only.

---

## 24. Amend — temporal relationship assessment & epistemic rule-breach (F1 §24)

**Cite:** Noddy F1 §24 tip `ceb59d9` · spine §3l · §16 modalities · §17 sense-context · §18 policy · §21 symbolic permissions · Alice F6 claim–policy namespaces.

### 24.1 Cut

Relationship quality and norm compliance **evolve in time**. They are claim–policy + epistemic/deontic assertions over *relations and contexts* — **not** NaturalAgent who-tokens, and not a static “good/bad person” embedding.

| Pattern | Shape expression |
|---------|------------------|
| Met in good faith; later adverse / knowable over time | Time-indexed RelationshipAssessmentClaim + evidence; prior good-faith claim remains provenance — not deleted who-rewrite |
| Inference over that arc | Typed edges on relation/context graphs — never bake into NaturalAgent embedding |
| Broke rule, didn’t understand | RuleBreachClaim with epistemic `¬K(rule)` / low awareness + sense-context binding |
| Broke rule knowing it | Same deontic breach, epistemic `K(rule)` — culpability on **claim**, not who-merge |
| Depends on the rule | NormativeRule + sense-context; crypto only hardens attestations |

**Gate fail:** “bad actor” as NaturalAgent type; knew-vs-unknowing as permanent who-bit; cultural rule as universal who-attribute; relationship-changed ⇒ identity-changed.

### 24.2 Shapes

#### `idf:RelationshipAssessmentClaimShape` (claim plane)
| Property | Note |
|----------|------|
| `idf:parties` | 2..* agents/entities in the relation |
| `idf:timeInterval` / epoch | Required — assessments are time-indexed |
| `idf:stance` | good-faith · adverse · mixed · disputed · … |
| `idf:evidence` | 0..* instruments/claims |
| Prior assessments | Remain as provenance; do not rewrite who |

#### `idf:KnowabilityAssertionShape` (epistemic modality)
| Property | Note |
|----------|------|
| `idf:aboutClaim` or `idf:aboutRule` | Target claim or NormativeRule |
| `idf:epistemicStatus` | K(rule) · ¬K(rule) · reasonably-knowable-by-t · disputed |
| `idf:asOf` | Time of knowability judgment |
| Forbidden | Entail NaturalAgent who |

#### `idf:NormativeRuleShape` (normative instrument / cited rule)
| Property | Note |
|----------|------|
| `idf:ruleId` | Stable rule identifier |
| `idf:senseContext` | locale · community · era (SenseContextBinding) |
| `idf:scope` | What the rule covers |
| Framing | Lexical + community instrument — not who |

#### `idf:RuleBreachClaimShape` (deontic + epistemic)
| Property | Note |
|----------|------|
| `idf:rule` | → NormativeRule |
| `idf:parties` | Alleged breachers / affected |
| `idf:deonticAssertion` | breach recorded |
| `idf:epistemicQualifier` | knew / didn’t understand / disputed — via KnowabilityAssertion |
| `idf:timeInterval` | When breach alleged |

```turtle
idf:RelationshipAssessmentClaimShape a sh:NodeShape ;
  sh:property [ sh:path idf:parties ; sh:minCount 2 ] ;
  sh:property [ sh:path idf:timeInterval ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:stance ; sh:minCount 1 ] .

idf:RuleBreachClaimShape a sh:NodeShape ;
  sh:property [ sh:path idf:rule ; sh:node idf:NormativeRuleShape ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:epistemicQualifier ; sh:minCount 0 ] .
  # Culpability on claim — MUST NOT entail NaturalAgent who.
```

### 24.3 Crypto / diagnose

- Hardness: co-attestation of evidence instruments over windows — not stronger “trust who.”
- Vibe: “relationship changed over time” ≠ “identity changed.”
- Alice: culpability/arc features in claim–policy/relation namespaces only.

---

## 25. Amend — high-cardinality relations & Quin axiom substrate (F1 §25)

**Cite:** Noddy F1 §25 tip `acd3da3` · spine §3m · §24 temporal assessment · §16–§18 · ADR 0001 Quin · human-centric cut.

### 25.1 Cut

**Relations are high-cardinality and non-defining.** NaturalAgent (or brand, org, AI-agent) is **not** constituted by the set of others they interact with. Views hang on **relation/context** nodes; Quin/NQuin holds typed edges for interdependent axioms — **storage/graph form**, not identity.

| Pattern | Shape expression |
|---------|------------------|
| Thousands of interactions | Many relation edges + assessment claims — cardinality expected |
| Others don’t define one another | No counterpart-set embedding into NaturalAgent who |
| Subj/obj views over time | RelationshipAssessmentClaim (§24) |
| Growth and dissolution | `RelationLifecycle` slots — begin · revise · end; dissolution ≠ erase provenance |
| Interdependent axioms | OntologyGovernedPolicy + modalities over multi-plane graph |
| Quin structure | Native substrate for those edges — not who |

**Gate fail:** identity = social graph; brand follows = who; dissolving friendship rewrites person type; lifetime counterparts packed into one who-embedding.

### 25.2 `idf:RelationLifecycleShape` (optional slots on relation axioms)

| Property | Note |
|----------|------|
| `idf:relates` | Parties / context (agents · brands · orgs · AI-agents) |
| `idf:begunAt` / `idf:revisedAt` / `idf:endedAt` | Growth and dissolution in time |
| `idf:assessment` | 0..* RelationshipAssessmentClaim |
| `idf:nonDefining` | Constant true for fabric docs — counterparts do not constitute who |
| Provenance | Ended relations retain historical claims |

```turtle
idf:RelationLifecycleShape a sh:NodeShape ;
  sh:property [ sh:path idf:relates ; sh:minCount 2 ] ;
  sh:property [ sh:path idf:begunAt ; sh:minCount 0 ] ;
  sh:property [ sh:path idf:endedAt ; sh:minCount 0 ] ;
  sh:property [ sh:path idf:assessment ; sh:node idf:RelationshipAssessmentClaimShape ; sh:minCount 0 ] .
  # MUST NOT entail NaturalAgent constitution by counterpart set.
```

### 25.3 Quin note (docs)

Quins/NQuins store typed relation/claim/instrument edges for human-centric interdependent axioms. Private keys never as Quin/log features (§20). Dense graph = resolution, not identity collapse.

### 25.4 Diagnose / Alice

- Vibe: dense relation graph ≠ identity collapse.
- Alice: counterpart-set / social-graph features ≠ NaturalAgent embedding.

---

## 26. Amend — pseudonyms, role≠accountability, anti-coercion purpose binds (F1 §26)

**Cite:** Noddy F1 §26 tip `9c5d542` · spine §3n · §15–§17 · §19 locators · §21–§22 roles · F5 §13 · Alice F6 · sanctuary/Webizen (docs intent).

### 26.1 Cut

Uplifted identifiers ≠ enumerated who. Pseudonyms and societal roles ride instruments / grants / claim–policy — they do **not** rewrite NaturalAgent. Role access ≠ accountability logs. Badge does not license stalking an ex; proxy cannot launder forbidden purpose.

| Pattern | Shape expression |
|---------|------------------|
| Pseudonym (privacy/legal) | PseudonymAlias — locator/instrument alias, not a second NaturalAgent |
| Societal role → access | RoleCapacityGrant — purpose · scope · time · environment |
| Accountability | AccountabilityArtifact / logs — distinct from grant success |
| Anti-coercion | PurposeBind / AntiCoercionConstraint — forbidden targets, COI, no-stalk, no-proxy-launder |
| Proxy | DelegationChain — inherits purpose+target; break on mismatch |

**Gate fail:** badge/role = forever who; pseudonym = second person; grant-success = accountability done; proxy washes forbidden purpose; LEO office ⇒ ex-surveillance rights.

### 26.2 Shapes

#### `idf:PseudonymAliasShape`
| Property | Note |
|----------|------|
| `idf:instrumentKind` | PseudonymAlias (align RelationScopedLocator / Alias) |
| `idf:aliasOf` | Optional link to NaturalAgent or other agent — **relation**, not who-fork |
| `idf:purpose` / privacy-legal flag | Why the alias exists |
| Forbidden | Entail second NaturalAgent type |

#### `idf:RoleCapacityGrantShape`
| Property | Note |
|----------|------|
| Extends | SituationalCapacityGrant / OrgStructuralRole patterns |
| `idf:role` | Societal/office role |
| `idf:purpose` · `idf:scope` · `idf:timeInterval` · env predicates | Required purpose bind |
| Distinct from | AccountabilityArtifact |

#### `idf:PurposeBindShape` / `idf:AntiCoercionConstraintShape`
| Property | Note |
|----------|------|
| `idf:allowedPurpose` / `idf:forbiddenTarget` | Policy instruments |
| `idf:conflictOfInterest` | e.g. ex-partner, self-deal |
| `idf:noProxyLaunder` | true — delegates inherit binds |
| Surface | Sanctuary / Webizen Desktop policy UX (docs intent; no Host invent) |

#### `idf:DelegationChainShape`
| Property | Note |
|----------|------|
| `idf:delegator` → `idf:delegate` | Chain of agency |
| `idf:inheritedPurpose` / `idf:inheritedTarget` | Must match upstream PurposeBind |
| Break | Purpose mismatch ⇒ deny (not who-rewrite) |

#### `idf:AccountabilityArtifactShape`
| Property | Note |
|----------|------|
| Plane | Claim–evidence / logs |
| Answers | What was done — not what role permits |

```turtle
idf:PseudonymAliasShape a sh:NodeShape ;
  sh:property [ sh:path idf:instrumentKind ; sh:hasValue idf:PseudonymAlias ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:aliasOf ; sh:minCount 0 ; sh:maxCount 1 ] .
  # MUST NOT entail second NaturalAgent who.

idf:RoleCapacityGrantShape a sh:NodeShape ;
  sh:property [ sh:path idf:purpose ; sh:minCount 1 ] ;
  sh:property [ sh:path idf:timeInterval ; sh:minCount 1 ] .

idf:DelegationChainShape a sh:NodeShape ;
  sh:property [ sh:path idf:inheritedPurpose ; sh:minCount 1 ] .
  # Break chain on purpose mismatch — no proxy-launder.
```

### 26.3 Diagnose / Alice

- Vibe: “has role” ≠ “may target anyone”; pseudonym ≠ second who.
- Alice: role/purpose/pseudonym = instrument/policy namespaces only.
- Sanctuary/Webizen: anti-coercion UX later — F2 names kinds only.

---
## 13. Changelog

| When | Note |
|------|------|
| 2026-09-06 | F2 initial — four planes + instrument kinds from Noddy F1; tip `565097f` / spine `9cf6855`. |
| 2026-09-06 | Noddy crypto skim: split `idf:DniRarSessionShape` → `idf:DniShape` · `idf:RarShape` · `idf:QSessionProofShape`; DID `verificationRelationship`; capability/discovery discrimination + `idf:keyRole`. |
| 2026-09-06 | Co-attestation hardness: `idf:CoAttestationBundleShape` on claim plane (time-bounded multi-instrument formula + distinct keyRoles); cites F1 §1b tip `bb714b2` — never stronger single who. |
| 2026-09-06 | Noddy §5.5 tighten: `attestationMember` sh:minCount **2**; `keyRole` diversity SHOULD (not all identical); tip base `77a13e3`. |
| 2026-09-06 | F1 §14: FOAF-modern type shapes (`AiAgent` · `MachineDevice` · Org/Service); WebID-TLS/RSA · SAN · hardware instruments; WN/OMW `lexicalConcept` ≠ plane; jury explainability. Base F1 `8724174`. |
| 2026-09-06 | F1 §15: `OsAccountShape` · `TelecomSubscriberShape`; independent `usedBy` / `accountOn` / `accountHolder`; tip `e4c6320`. |
| 2026-09-06 | F1 §16: `GuardianshipRelationShape` · capacity gradients · `CommonsMembershipShape` / project roles; core-ontologies uplift; modalities ≠ who. |
| 2026-09-06 | F1 §17: `SenseContextBindingShape` · `FloraShape`/`FaunaShape` · `SituationalCapacityGrantShape` · grants≠logs; cite F5 `da74019`. |
| 2026-09-06 | F1 §18: `OntologyGovernedPolicyShape` · `ZkpProofShape`; policy≠who; ZKP uplift not reinvent. |
| 2026-09-06 | Spine §3g: `RelationScopedLocatorShape` (jane@bob.tld ↔ bob@jane.tld sketch); locators ≠ static who; tip base `1d55f56`. |
| 2026-09-06 | F1 §20: `OnlineAccountShape` · `WalletShape` · `BearerTokenShape`/`OAuthTokenShape` · `PasswordVerifierShape` · `PrivateKeyMaterialShape`; secrets≠who. |

| 2026-09-06 | F1 §21: `SymbolicPermissionContextShape`; specialised AI-agents + grants/agreements; symbolic-first permissions; instruments/ZKP proof-only. |

| 2026-09-06 | Spine §3j: `OrgStructuralRoleShape` · `GroupAuthMembershipShape` (election turnover = rebind, not who-rewrite); tip `78d3878`. |

| 2026-09-06 | F1 §23: `EnvironmentPredicateShape` · `PlaceBoundSecretShape` · `SensorIdShape` · GIS binding · EnvironmentAttestation; env≠who. |

| 2026-09-06 | F1 §24: `RelationshipAssessmentClaimShape` · `KnowabilityAssertionShape` · `NormativeRuleShape` · `RuleBreachClaimShape`; relationship≠who; tip `ceb59d9`. |

| 2026-09-06 | F1 §25: `RelationLifecycleShape` · non-defining high-cardinality relations · Quin = axiom substrate ≠ who; tip `acd3da3`. |

| 2026-09-06 | F1 §26: `PseudonymAliasShape` · `RoleCapacityGrantShape` · `PurposeBind`/`AntiCoercionConstraint` · `DelegationChainShape` · AccountabilityArtifact; role≠accountability; tip `9c5d542`. |

---

*End of WIP — Marvin F2 Identifier Fabric SHACL-first split.*
