# WIP — Crypto instrument taxonomy (Identifier Fabric)

**Status:** work-in-progress · **Not standards** · **Branch:** `0.0.36-dev`  
**Against tip (amend base):** fabric HEAD `4fa5109` · F2 crypto-skim `42dc709` · F6 PR #77  
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

*End of WIP — Noddy F1 crypto instrument taxonomy.*

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

---

*End of WIP — Noddy F1 crypto instrument taxonomy (hardness + F6 cross-link amend).*
