# WIP — Identifier Fabric SHACL-first split (F2)

**Status:** work-in-progress · **Not standards** · **Branch:** `0.0.36-dev`  
**Against tip:** fold after Noddy F1 (`CRYPTO_INSTRUMENT_TAXONOMY_WIP.md`) · architecture spine `IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md`  
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

## 6. Instrument kind shapes (map Noddy §5)

| Kind | Shape id | Framing | Key constraints |
|------|----------|---------|-----------------|
| Network address / locator | `idf:NetworkAddressShape` | Spatiotemporal + instrument | Not DID; not who; LIG only for legacy IP/DNS |
| Machine / device ID | `idf:MachineIdShape` | Artifact | Device ≠ human principal; link to agent is relation |
| DID / DID URL | `idf:DidInstrumentShape` | Artifact instrument | **DID ≠ identity**; pairwise/contextual ok; may name many subject types |
| QRC (`did:q42:`) | `idf:QrcShape` | Artifact index | **No crypto by itself**; not who; provisional join only |
| Verifiable credential | `idf:VerifiableCredentialShape` | Artifact → claim | Origin+integrity; subject ≠ automatic NaturalAgent who |
| Verifiable claim / opinion | `idf:ClaimOpinionShape` (plane) | Claim | Alias Assertions / provenance sit here or claim-adjacent |
| Biometric **family** | `idf:BiometricFamilyShape` | Living-safe instrument class | Durable *kind*; relates to NaturalAgent; never global unique who |
| Biometric **instance** | `idf:BiometricInstanceShape` | Mutable sample artifact | Drifts; family link required; sample ≠ timeless who |
| Content / strong digest | `idf:ContentDigestShape` | Artifact | Explicit algorithm; `q_hash` ≠ strong digest |
| DNI / RAR / session proof | `idf:DniRarSessionShape` | Spatiotemporal + session | How now ≠ who forever; transport key ≠ DID controller |
| Capability / discovery secret | `idf:CapabilityInstrumentShape` | Artifact authz | Possession ≠ natural-agent identity |

**Uniform closed constraint (all kinds):** `idf:collapsesToWho` MUST NOT be entailed; SHACL `sh:not` / documentation gate for designs that merge kind → NaturalAgent identity.

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

No `owl:sameAs` widening of authority (contracts §3).

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

Gaps only → Capt WIP / Vibe deltas — **no** Host invent from F2.

---

## 9. Gate checks (for Capt / Neo / UAT later)

1. No NaturalAgent NodeShape subclasses `owl:Thing` as commodity.
2. No chrome/diagnose string calls persons/living “things” (Vibe F5).
3. Biometric instance without family link = shape fail.
4. QRC / observer DID labeled topology/coord in docs — not “identity.”
5. VC verification success ≠ claim truth ≠ NaturalAgent who.
6. DNI/session proof ≠ DID controller authority unless separately authorized (cite crypto profile).
7. Inference feature spaces (Alice F6) keep who / claim / handle as **separate** dimensions.

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
| **Neo** | Fold this file to `docs/work-in-progress/IDENTIFIER_FABRIC_SHACL_SPLIT_WIP.md`; link F2 from architecture spine; tick F2 state; no `ALL_BOUND` invent |
| **Noddy** | Review instrument↔shape matrix for crypto mis-map |
| **Capt.** | Amend spine §5 F2 → landed; blockers to Capt. |
| **Vibe** | F5 diagnose / `suggested_fix` can cite plane names; never who→claim/handle collapse |
| **Alice** | F6 pressure-test separate feature spaces against §5–§7 |
| **Marvin** | Idle on F2 after fold; amend only if Capt enumerates new kinds |

---

## 12. Open questions (non-blocking)

1. Preferred durable NaturalAgent node IRI strategy vs pairwise-only graph (architecture §7 Q1).
2. Biometric family registry without global unique who.
3. When observer DID may graduate from provisional topology role.
4. Dual-VC Poet exposure honesty details (cite uplift audit).

---

*End of WIP — Marvin F2 Identifier Fabric SHACL-first split.*
