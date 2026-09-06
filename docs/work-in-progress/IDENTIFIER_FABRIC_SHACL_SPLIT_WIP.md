# WIP — Identifier Fabric SHACL-first split (F2)

**Status:** work-in-progress · **Not standards** · **Branch:** `0.0.36-dev`  
**Against tip:** F2 §14 `9ad8fc8` · F1 §15 `e4c6320` · architecture spine `IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md`
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
## 13. Changelog

| When | Note |
|------|------|
| 2026-09-06 | F2 initial — four planes + instrument kinds from Noddy F1; tip `565097f` / spine `9cf6855`. |
| 2026-09-06 | Noddy crypto skim: split `idf:DniRarSessionShape` → `idf:DniShape` · `idf:RarShape` · `idf:QSessionProofShape`; DID `verificationRelationship`; capability/discovery discrimination + `idf:keyRole`. |
| 2026-09-06 | Co-attestation hardness: `idf:CoAttestationBundleShape` on claim plane (time-bounded multi-instrument formula + distinct keyRoles); cites F1 §1b tip `bb714b2` — never stronger single who. |
| 2026-09-06 | Noddy §5.5 tighten: `attestationMember` sh:minCount **2**; `keyRole` diversity SHOULD (not all identical); tip base `77a13e3`. |
| 2026-09-06 | F1 §14: FOAF-modern type shapes (`AiAgent` · `MachineDevice` · Org/Service); WebID-TLS/RSA · SAN · hardware instruments; WN/OMW `lexicalConcept` ≠ plane; jury explainability. Base F1 `8724174`. |
| 2026-09-06 | F1 §15: `OsAccountShape` · `TelecomSubscriberShape`; independent `usedBy` / `accountOn` / `accountHolder`; tip `e4c6320`. |

---

*End of WIP — Marvin F2 Identifier Fabric SHACL-first split.*
