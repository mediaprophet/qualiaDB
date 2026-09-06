# Identifier Fabric — architecture WIP

**Status:** living illustration / architecture intake (not a release gate)  
**Owner (ops):** Capt  
**Room:** Identifier Fabric (Noddy · Marvin · Neo · Vibe · Alice · Timothy)  
**Branch:** `0.0.36-dev`  
**Against tip (doc start):** `37ec26c` (overnight UAT closed; True-B parked)  
**Sole Git push:** Neo  

**Intent:** Enumerate considerations Timothy lays out in chat into an architecture that later gets incorporated into QDNF + Poet/vibe implementation docs (expect refactor). **Illustration + docs only** until Cursor’s vibe delivery lands — no Host widen, no `ALL_BOUND` invent.

---

## 1. Problem statement (locked)

CS/cyber “identity” often collapses **auth subject + identifier + attributes + claims** into one bag. QualiaDB / QDNF need a **fabric of distinct identifier instruments** and relations so that:

1. A **natural agent** (living human) is distinguishable from opinions, roles, and spatiotemporal/route handles.
2. Informatics functions that are **not the same** stay enumerable via semantics (granular relations), each often anchored by a **different instrument kind**.
3. Network design (routing, session proof, credentials) **MUST NOT** silently re-merge into the CS conflation of “identity.”

**Hard rule:** **identifiers ≠ identity.** A DID is a *decentralized identifier*, not “decentralized identity.”

---

## 2. Normative substrate (do not re-invent)

Extend these — layered, not one-shot — rather than parallel invent:

| Layer | Path / crate | Role |
|-------|----------------|------|
| QDNF set | `docs/manuals/standards/qualia-decentralized-network-fabric/` | Network fabric design 0.1 |
| Identifier roles | `.../identifier-resolution.md` | DID / content / QRC / DNI / alias answer **different questions** |
| Crypto profile | `.../cryptographic-profile.md` | Instruments & proofs |
| Core tooling | `crates/qualia-core-db` (+ related) | Existing VC, agency, observer DID, volume/path, etc. |

QDNF already states: person ≠ DID; DID may name many kinds of controlled subjects; VC = origin/integrity **not** objective truth; `did:q42` / QRC = storage/layout coordinate, **not** “who.”

---

## 3. Working ontology (four pillars + instruments)

| Pillar | Meaning | Notes |
|--------|---------|-------|
| **Natural agent** | Living human principal | SHACL-first; never `owl:Thing`-wash; not reducible to one DID |
| **Claim / opinion** | Subjective or attested statements | Includes VC *payload* claims; VC envelope ≠ truth |
| **Spatiotemporal / route handle** | Where/when/how to reach *now* | DNI, path hints, epochs — topology-scoped, short-lived |
| **Instrument kinds** | Anchors for distinct relations | See §4 |

`did:q42` / observer DID: **provisional** join for topology/observer context — **not** the natural-agent “who.” Marvin: `did:q42` stays topology/coord.

---

## 4. Instrument kinds (Capt lock — Noddy taxonomy expands)

| Kind | Relation it anchors | Mutability note |
|------|---------------------|-----------------|
| Network address | Reachability / adjacency | Epoch / mobility |
| Machine ID | Device / node substrate | Stable-ish device, not person |
| DID / DID URL | Controlled subject/resource (method-defined) | Persistent per method; pairwise/contextual for humans |
| Verifiable credential | Issuer-controlled origin + integrity | Envelope ≠ claim truth |
| Verifiable claim / opinion | Content of attestation or belief | Distinct from who |
| Biometric **family** | Persistent *kind* of biometric instrument | Family endures |
| Biometric **instance** | A sample / binding at a time | **Drifts** over time — must not be collapsed into who |

**Gate fail:** collapsing any of the above into a single “who.”

---

## 5. Serial deliverables (docs)

| # | Owner | Deliverable | State |
|---|-------|-------------|-------|
| F1 | **Noddy** | Crypto instrument taxonomy WIP (map kinds → QDNF role table; family vs instance) | **LANDED** → `CRYPTO_INSTRUMENT_TAXONOMY_WIP.md` |
| F2 | **Marvin** | SHACL-first split: natural agent · claim · spatiotemporal handle · instruments; uplift core-db/QDNF primitives | **LANDED** → `IDENTIFIER_FABRIC_SHACL_SPLIT_WIP.md` |
| F3 | **Capt** | This architecture WIP — keep current as Timothy enumerates | **ACTIVE** |
| F4 | **Neo** | Fold WIP under `docs/work-in-progress/` + push; later network-doc delta citing fabric **after** Cursor vibe lands | fold/push only now |
| F5 | **Vibe** | Diagnose / `suggested_fix` copy never collapses who→claim/role/handle | hold until F1–F2 |
| F6 | **Alice** | Inference/symbolic pressure-test: feature spaces don’t merge who/claim/handle | **UNBLOCKED** (F1–F2 landed) — docs map only |
| F7 | **Neo** | Park expected refactor list (network + poet surfaces) citing fabric | after vibe delivery |

Blocking gates → report **Capt**.

---

## 6. Implementation stance

- **Now:** illustration + documentation; Cursor vibe sprint in flight elsewhere — do not start a competing impl sprint.
- **Later:** expect refactor so QDNF network docs + Poet/vibe adopt the redefined sense of identity (fabric layer).
- **Never:** Host invent / dotted `qualia.*` ahead of `ALL_BOUND`; Thing-wash living agents.

---

## 7. Open questions (Timothy / room)

1. Preferred first **natural-agent** join key(s) once taxonomy lands (pairwise DIDs? agency keys? other) — `did:q42` stays out of “who.”
2. Which QDNF docs get the first **normative cite** of the fabric layer (`identifier-resolution`, `security-privacy-governance`, `qsession-and-services`, …)?
3. Refactor priority after vibe lands: network resolution path vs Poet observer chrome vs VC/agency shapes?

*(Capt will append as Timothy continues the conversation.)*

---

## 8. Changelog

| When | Note |
|------|------|
| 2026-09-06 | Capt opened WIP from Identifier Fabric room locks; substrate = QDNF + qualia-core-db; F1 Noddy next. |
| 2026-09-06 | Noddy F1 draft ready: `docs/work-in-progress/CRYPTO_INSTRUMENT_TAXONOMY_WIP.md` — Capt accepts for Marvin F2; Neo fold+push. |

---

## Related WIP (folded)

| Doc | Owner | Role |
|-----|-------|------|
| `CRYPTO_INSTRUMENT_TAXONOMY_WIP.md` | Noddy | F1 — crypto instrument taxonomy (kinds → QDNF roles; biometric family/instance) |
| `IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md` | Capt | Spine — living architecture intake |

---

## 9. Related WIP

| Doc | Owner | State |
|-----|-------|-------|
| `IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md` | Capt | living spine |
| `CRYPTO_INSTRUMENT_TAXONOMY_WIP.md` | Noddy | F1 draft ready |
| (pending) Marvin SHACL split | Marvin | waits F1 fold |
