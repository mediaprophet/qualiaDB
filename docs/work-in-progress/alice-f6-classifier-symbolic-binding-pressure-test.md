# WIP — Alice F6: classifier / symbolic-binding pressure-test

**Status:** work-in-progress · **Not standards** · **Branch:** `0.0.36-dev`  
**Owner:** Alice (inference / ML / symbolic AI) · **Fold/push:** Neo · **Ops:** Capt.  
**Against fabric HEAD:** `4a44b0d` (F1/F2 §19 · F5 §12 relation-scoped locators) · merge base `1d55f560` (§3g) · this amend merge `eabccc9`  
**F2 content SHA (history):** `565097f` · **F5 diagnose SHA:** `796a7d4` · **F5 §11:** `1d55f560` · **F5 §12 / F2 §19:** `4a44b0d`  
**F2 §19:** `idf:RelationScopedLocatorShape` **LANDED** tip `4a44b0d` — typed instrument namespace for spine §3g locators.  
**Constraint:** docs / illustration only. **No Host invent. No `ALL_BOUND` invent. No vibe-host surface widen.** Neo folds.

**Pressure-test question:** Can classifiers and symbolic bindings keep these four strata distinct **without collapsing into a single feature-space bag**?

| Stratum | Answers | F5 diagnose voice (`IDENTIFIER_FABRIC_DIAGNOSE_MAP_WIP.md` §2) |
|---------|---------|---------------------------------------------------------------|
| **who** | natural-agent identity (living principal) | person · people · living · being · kin |
| **claim** | objective/subjective opinions / assertions about an agent (or resource) | claim · assertion · opinion · attestation |
| **spatiotemporal handle** | space-time / location / session / topology coordinates | place · where · when · route · how now |
| **instruments** | crypto-backed keyed instruments that *bind* who without absorbing claims or handles | tool · DID (identifier) · credential · machine id · volume · digest · biometric family/instance |

**Intra-instrument dimensions that must also stay distinct** (F2 crypto-skim amend at `42dc709`, still present at `55d811b`):

- `idf:keyRole` — purpose-separated: `route-update` ≠ `session-authentication` ≠ `transport-aead` ≠ `controller-signing` ≠ `capability-presentation` ≠ `discovery-psk`
- **DNI ≠ RAR ≠ QSession** — `idf:DniShape` ≠ `idf:RarShape` ≠ `idf:QSessionProofShape` (deprecated lump: `idf:DniRarSessionShape`)
- **ZKP / grant / policy ≠ who** — F5 §11 / F2 §18: proof instrument and situational grant stay off the NaturalAgent plane
- **Relation-scoped locators ≠ who** — spine §3g: pairwise / group / chat / transaction / DNS-code strings are **instruments**, not a static forever who-address (see §8)

`did:q42` / observer DID remain **provisional topology/coord join keys**, not “who.”

**Gate fail:** any neural concat, softmax, shared URI class, or SHACL target that merges the four strata (or merges keyRole / DNI·RAR·QSession / locator-as-who / ZKP-or-grant-as-who) into one CS-style “identity” / feature bag.

---

## 0. Stance (locked)

- F6 is a **docs map** for later inference constraints. It does not invent Host methods, dotted `qualia.*`, opcodes, or ALL_BOUND rows.
- Spine §1b (`55d811b`): this fabric is **not** a greenfield identity product. W3C DIDs/VCs/Solid appear in Noddy F1 as **interop/offramp instruments in a ~2000→now lineage**, not as “who.” Classifiers must not treat “W3C identity stack” as a single embedding class.
- Spine §3g (`1d55f560`): relation-scoped locators are instruments/handles, not NaturalAgent. Solid/phone static-address pattern is rejected as the default.
- Completeness bar here = honest pressure-test + named failure modes + later constraints. Implementation waits on Capt / Cursor vibe / Neo fold.

---

## 1. Sources read

Paths as they exist at tip `1d55f560`. Short note = what each says about the four strata (and intra-instrument dims).

### 1.1 Identifier Fabric WIP (primary)

| Path | What it says about the strata |
|------|-------------------------------|
| `docs/work-in-progress/IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md` | Spine. CS “identity” = auth subject + identifier + attributes + claims in **one bag** is the problem. Four pillars: natural agent · claim/opinion · spatiotemporal/route handle · instrument kinds. `did:q42` / observer DID = provisional topology, not who. **§1b:** long-arc lineage; DIDs/VCs are instruments, not who. **§3g** (this tip `1d55f560`): relation-specific addressing — locators name a *relationship* (pairwise, group, chat, transaction, DNS TXT), not a permanent who-token; email sketch `jane@bob.tld` ↔ `bob@jane.tld`; agents-of-entities in relation metadata ≠ mailbox who. Cut: locators are **instruments/handles**, not NaturalAgent. |
| `docs/work-in-progress/CRYPTO_INSTRUMENT_TAXONOMY_WIP.md` | **F1 / Noddy.** Planes table: who ≠ claim ≠ handle ≠ instrument. Instruments bind *relations*; they do not replace the agent. QDNF role table stays authoritative for *what question an identifier answers*. Biometric **family** vs **instance**. Collapse of any kind into “who” = gate fail. VC = origin+integrity, not truth. QRC has **no crypto by itself**. |
| `docs/work-in-progress/IDENTIFIER_FABRIC_SHACL_SPLIT_WIP.md` | **F2 / Marvin** (content `565097f` + crypto-skim amend `42dc709`; tip through §18). Four plane shapes: `idf:NaturalAgentShape` (living-SHACL, never Thing-wash) · `idf:ClaimOpinionShape` · `idf:SpatiotemporalHandleShape` · `idf:InstrumentShape`. Cross-plane preds (`relatedByInstrument`, `about`, `assertedBy`, `places`) are **relations, not merges**. Amend: split DNI/RAR/QSessionProof; `idf:keyRole` + `idf:verificationRelationship`; §18 policy/ZKP shapes. **§19 `idf:RelationScopedLocatorShape`:** **LANDED** tip `4a44b0d` — typed instrument namespace for §3g locators. Existing nearest kind: `idf:NetworkAddressShape` (not DID; not who; LIG only for legacy IP/DNS) — pairwise/relation locators are **not** that legacy bag. |
| `docs/work-in-progress/IDENTIFIER_FABRIC_DIAGNOSE_MAP_WIP.md` | **F5 / Vibe** (`796a7d4` + §11 fold on this tip). **§2 plane voice** = locked feature-space *labels* for later classifiers (never “identity” as auth bag; never DID-as-who). **§3 collapse detectors** = refuse-merge list: DID/observer/`did:q42` as who; VC verified ⇒ claim true ⇒ who; **DNI / address as persistent who** (speak how-now handle; not person); biometric instance as timeless who; machine ID = principal; alias as route authority. **§11:** ZKP/grant/policy ≠ who; HTTP/Solid = offramp, not identity. Locator copy aligns with instrument + how-now speak — never “your identity” / forever mailbox. |

### 1.2 QDNF / standards substrate (do not re-invent)

| Path | What it says about the strata |
|------|-------------------------------|
| `docs/manuals/standards/qualia-decentralized-network-fabric/identifier-resolution.md` | DID / content ID / resource IRI / QRC / DNI / alias answer **different questions**. Natural person is not reducible to a DID. QRC = 60-bit index, no security authority. DNI = how-now. VC proves origin/integrity, not claim truth. Session `authentication` key cannot update routes unless separately authorized. |
| `docs/manuals/standards/qualia-decentralized-network-fabric/cryptographic-profile.md` | Purpose-separated **key roles** (controller signing ≠ route-update ≠ QSession auth/AEAD ≠ discovery). Compact Q42 hashes are indexes, not cryptographic hashes. Profile **does not** prove a natural person’s identity as a whole. |
| `docs/manuals/standards/qualia-decentralized-network-fabric/ontological-contracts.md` | Explicit terms for natural person vs agent/operator. **Signing with an agent key does not identify its human principal.** `owl:sameAs` cannot widen authority. |
| `docs/manuals/standards/qualia-decentralized-network-fabric/security-privacy-governance.md` §6 | Identifier ≠ identity. Many pairwise DIDs. Automated cross-context correlation prohibited without consent. Location aliases must not become permanent identity evidence. |
| `docs/manuals/standards/shacl-first-vs-owl-ok-class-list.md` | Living/person = SHACL-first (never `owl:Thing`). Instruments/IDs/CRS = OWL-ok artifacts. **Mixed:** Position (coords vs *what* is placed); Provenance/Claim (structure vs subject). |
| `docs/manuals/standards/g-coord-coordinate-system-shapes.md` | `did:q42` here is storage/QRC locus, not a DNI. **Locus is not a person.** Position = mixed. |
| `AGENTS.md` §1 / `identifier.rs` note | `did:q42` = topological pointer (MSB dispatch). Confirms QRC-as-coord, not who. |

### 1.3 Adjacent risk sources (not F6 SoT; named so classifiers do not “helpfully” reuse them as who)

| Path | Collapse invite |
|------|-----------------|
| `docs/manuals/standards/AGENT_INTENT_LOGGING_SPEC.md` Vector 1 | Labels `did:q42:root` as “Who (The Identity Resolution Graph).” Fabric rule: QRC/root hash is an **instrument/coord**, not natural-agent who. Do not train a who-classifier on this vector name. |
| `crates/qualia-core-db/src/modalities/identity_fabric.rs` | Resilience fabric of *cryptographic anchors* (k-of-n). Useful for instrument quorum; **must not** be read as “anchors = the person.” |
| `docs/work-in-progress/ontology-design-notes-marvin.md` | Confirms B-OWL-PERSON / living-safe copy; mixed Position. Aligns with F2 framing. |

---

## 2. Verdicts

Legend: **pass** = current docs already force a typed split a later binder can cite · **risk** = docs split the plane but existing surfaces / habits invite bagging · **fail** = a present design would force one-bag collapse if copied into features.

### 2.1 Who — natural-agent identity

| Track | Verdict | Why |
|-------|---------|-----|
| (a) Neural classifiers / embeddings / feature bags | **fail** if a single “identity” or `did:*` embedding is used as the person vector; **risk** even with a dedicated who-head, because substrate copy (`did:q42:root` as Vector 1 “Who”, observer DID in chrome) will leak into training labels | Living who has **no required single DID**. Pairwise/contextual instruments are expected. Concatenating DID + VC subject + QRC + biometric instance into one person embedding *is* the CS bag. |
| (b) Symbolic bindings / typed slots / SHACL | **pass** on F2 `idf:NaturalAgentShape` + living-SHACL + forbidden Thing superclass; **risk** if any later `sh:targetClass` or `owl:sameAs` bridges agent → DID/QRC | Shape pack already forbids `idf:collapsesToWho`. Join key still unsettled (architecture §7 Q1) — do not let a classifier invent the join. |

### 2.2 Claim — opinions / assertions

| Track | Verdict | Why |
|-------|---------|-----|
| (a) Neural | **risk** | Claim text, VC payload, alias assertions, and epistemic/deontic modality records are the easiest features to concat onto a “person vector.” Soft-max over {true-who, false-who} from a verified VC is exactly F5 §3’s “VC verified ⇒ claim true ⇒ who.” |
| (b) Symbolic | **pass** | `idf:ClaimOpinionShape` + `idf:about` / `idf:assertedBy` keep claim off the who plane. Envelope (OWL-ok) ≠ payload truth. Modality stays on claim plane (F1 §5.6). |

### 2.3 Spatiotemporal handle — place / when / route / how-now

| Track | Verdict | Why |
|-------|---------|-----|
| (a) Neural | **fail** if session, DNI, path, G-COORD Position, or observer/`did:q42` coords are concatenated into the who vector (re-identification + “who forever” from how-now) | F5 §2 never-say: “who forever · controller fact from path alone.” Location aliases must not become permanent identity evidence (QDNF privacy §10). |
| (b) Symbolic | **pass** on plane shape; **risk** inside the handle/instrument border | `idf:SpatiotemporalHandleShape` is mixed (coords OWL-ok; *what* is placed may be living). G-COORD already says locus ≠ person. **Do not** reuse one `Position` slot as who. |

### 2.4 Instruments — keyed bindings (and intra-instrument dims)

| Track | Verdict | Why |
|-------|---------|-----|
| (a) Neural | **fail** for any shared “crypto-id” / “auth subject” embedding that pools DID + VC + machine id + biometric + DNI + session proof | F1 mapping matrix: **every** kind → collapse to who = fail. §1b: W3C DID/VC lineage is instrument interop, not a who class. |
| (b) Symbolic | **pass** after F2 amend `42dc709` *if* binders use specialized shapes + `idf:keyRole`; **fail** if they keep deprecated `idf:DniRarSessionShape` or omit `idf:verificationRelationship` | DNI ≠ RAR ≠ QSessionProof. Session authentication MUST NOT entail route-update or DID controller. Agent key ≠ human principal. |

### 2.5 Headline

**Docs planes: pass. Inference-ready default: fail/risk.**  
F1+F2+F5 give typed names a later binder can cite. Nothing in the current Host/runtime is allowed to implement that binder yet. If Alice (or any ML path) ships a single feature bag *before* typed namespaces exist, F6 **fails**. The pressure-test answer is: **only if** later inference obeys §4 constraints; **not** if it follows CS identity / embedding-concat habit.

### 2.6 Relation-scoped locators — instruments, not who (spine §3g)

| Track | Verdict | Why |
|-------|---------|-----|
| (a) Neural | **fail** if a locator string (email-like, WebID, phone, DNS TXT, chat/transaction id) is embedded as a static forever who-address, or if directed pair `jane@bob.tld` ⊕ `bob@jane.tld` is concatenated into one NaturalAgent vector | Spine §3g cut: the string names a *relation*, not a person. Concat is CS bag + QDNF §6 correlation. Agents-of-entities in metadata are not who features. |
| (b) Symbolic | **pass** only if binders use a **typed instrument namespace** (`instrument.locator.relation.*` / `idf:RelationScopedLocatorShape`); **fail** if they reuse `idf:NaturalAgentShape`, a shared `Identity`/`Agent` URI, or legacy `idf:NetworkAddressShape` as the who slot | F2 §19 LANDED `4a44b0d`. Align F5 §2 / §12 instrument speak + §3 “address as persistent who.” |

---

## 3. Concrete failure modes (one-bag collapse)

Use F5 §3 as the copy-side detectors. Below is the **feature-space** form of the same collapses.

| Mode | What it looks like in a classifier / binder | Why it fails the fabric |
|------|---------------------------------------------|-------------------------|
| **Thing-wash** | `owl:Thing` / “entity” / generic `object` class as the who label; living subjects in an artifact embedding space | B-OWL-PERSON / F2 forbidden superclass; F5 never-say: thing · object · entity |
| **Attribute-as-who** | Soft-max or slot-fill: role, guardian, VC type, risk score, “auth subject” → person id | Attributes and claims are not the natural agent (spine §1) |
| **Embedding concat** | `who ⊕ claim ⊕ place ⊕ instrument` (or late-fusion without stratum tags) as one vector for retrieval / clustering / “same person” | The exact CS bag. Cross-context correlation without consent (QDNF §6) |
| **Shared URI class** | One `Identity` / `Agent` / `did:*` class for person, DID, QRC, DNI, VC subject | Identifiers ≠ identity; DID may name many subject types |
| **QRC-as-who** | `did:q42` / observer DID / `did:q42:root` used as the person key in features | Provisional topology/coord only (F1 §6, F2 §5.1, F5 §3, G-COORD) |
| **VC-verified-as-true-who** | Verification success bit concatenated into who or used as claim-truth label | Envelope ≠ payload; payload ≠ who (F1 §5.5, F5 §3) |
| **How-now-as-who-forever** | DNI, path hint, Position, session id as persistent identity features | F5 §3; QDNF location aliases expire |
| **DNI=RAR=QSession bag** | One “network identity” / `DniRarSession` embedding or SHACL target | F2 amend: purpose-separated shapes; session proof ≠ route-update |
| **keyRole wash** | One key embedding reused across controller-signing, route-update, session-authentication, transport-AEAD, discovery PSK | Crypto profile + `idf:keyRole`; agent key ≠ principal |
| **Biometric instance = who** | Sample/template vector as timeless person embedding; no family link | Family vs instance (F1 §5.7); instance without family = shape fail |
| **Machine / capability = who** | Device id or capability presentation as principal | Possession ≠ natural-agent identity |
| **Alias = route = who** | Multilingual alias embedding used as both discovery and routing and person | Alias needs provenance; never sole route authority |
| **Lineage wash (§1b)** | “W3C / Solid / DID-VC stack” as a single who/identity feature class | Those are interop/offramp **instruments**, not who |
| **Anchor-quorum = who** | `identity_fabric.rs` surviving-anchor ratio used as a person classifier score | Quorum reconstructs *instrument fabric*, not the living principal |
| **SameAs widening** | Symbolic `owl:sameAs` or neural nearest-neighbour merge across planes | Contracts §3: sameAs cannot widen authority; F2: session MUST NOT sameAs into route-update |
| **Locator-as-who** | Static forever-address (Solid WebID / phone pattern) or relation locator (`jane@bob.tld`, group/chat/tx/DNS code) used as NaturalAgent key or concat-embedded into who | Spine §3g: locators are instruments/handles, not who. F5 §3: address ≠ persistent who. Typed instrument namespace only (later F2 `RelationScopedLocatorShape`) |
| **Directed-pair who-concat** | `jane@bob.tld` ⊕ `bob@jane.tld` (or group roster of locators) fused into one person embedding | Two directed **relation** instruments. Concat invents a who that neither string names |
| **Agent-metadata-as-who** | Agents-of-entities listed on a locator / mailbox / chat relation treated as NaturalAgent who features | Spine §3g: those agents appear in **relation metadata/semantics**, not as the mailbox who |

---

## 4. Recommended inference constraints (later — no Host invent now)

These are **requirements to cite** when inference/symbolic bind work is unlocked. They are not binds, not `qualia.*` methods, not ALL_BOUND rows.

1. **Typed feature namespaces** aligned to F5 §2 labels and F2 `idf:plane`:
   - `who.*` / `idf:NaturalAgent`
   - `claim.*` / `idf:ClaimOpinion`
   - `handle.*` / `idf:SpatiotemporalHandle`
   - `instrument.*` / `idf:Instrument` plus `instrumentKind` + `keyRole`
2. **Refuse cross-stratum soft-max.** A head that classifies `{person, claim, place, tool}` as one closed set, or that answers “who” from claim/handle/instrument logits, is a fabric-collapse (F5 §3 refuse-merge).
3. **Separate embedding spaces**, or **gated fusion with explicit stratum tags** (and explicit `keyRole` / `instrumentKind` tags). Untagged concat is forbidden. Fusion output must remain inspectable as four (plus keyRole) streams, not one “identity” vector.
4. **Symbolic pre-check before ML bind.** Validate plane + `idf:keyRole` + DNI/RAR/QSessionProof discrimination (F2 §6–§7) *before* any embedding is written or a bind is accepted. SHACL/shape miss → held / not yet (F5 gates), not a guessed who.
5. **Join-key hygiene.** Do not train or bind who on `did:q42`, observer DID, or QRC. Until Capt settles architecture §7 Q1, pairwise/contextual instruments **relate** via `idf:relatedByInstrument` only.
6. **VC / modality path.** Verification bit → instrument envelope features only. Payload → claim space (epistemic/deontic/paraconsistent stay there). Never a who label.
7. **Session / route path.** `session-authentication` features must not be reused as `route-update` or `controller-signing`. DNI how-now features must expire with the epoch (no permanent who memory).
8. **Living-safe labels.** Training and eval vocab follow F5 §2 / Marvin class list: never thing/object/entity for persons.
9. **No W3C-stack who-class.** DID method, VC type, Solid webid are instrument/lineage features (§1b), not a natural-agent class.
10. **Correlation brake.** Cross-context nearest-neighbour over who-space requires the same consent/necessity bar as QDNF §6 — not a silent embedding join.
11. **Relation-scoped locators.** Feature only as typed `instrument.*` (later `idf:RelationScopedLocatorShape`). Never `who.*`. Never concat directed pair / group roster into a NaturalAgent vector. Agents-of-entities stay metadata of the *relation*, not who features. See §8.

---

## 5. Open questions

### 5.1 For Noddy (crypto instruments)

1. Should `idf:keyRole` be a **closed** enum for the first inference namespace (`route-update`, `session-authentication`, `transport-aead`, `capability-presentation`, `discovery-psk`, `controller-signing`), or must classifiers treat unknown roles as held/not-yet rather than “other-id”?
2. Is a **biometric-instance** embedding ever licit as a *gated instrument* feature (family-linked, non-who), or must ML refuse sample vectors entirely until a later crypto/privacy profile?
3. Dual-VC (W3C vs native) — confirm the envelope features that may enter `instrument.*` without leaking payload-as-who (F1 open Q4 / uplift audit).
4. Any additional purpose-separation Alice should treat as hard-negative beyond DNI ≠ RAR ≠ QSession (e.g. QLink ephemeral DH vs QSession traffic AEAD as distinct `keyRole`s in feature space)?

### 5.2 For Marvin (SHACL class split)

1. Confirm `idf:plane` + `idf:instrumentKind` + `idf:keyRole` are the **only** first-class axes Alice should namespace on; or add a fifth axis for “lineage/offramp” (W3C/Solid) so §1b does not get stuffed into `instrumentKind`.
2. Preferred **durable NaturalAgent node** strategy vs pairwise-only graph (F2 §12 Q1) — classifiers must not invent a who-IRI.
3. Should deprecated `idf:DniRarSessionShape` be a **shape fail** (closed `sh:not`) so symbolic pre-check rejects the lump, not just a docs deprecation?
4. How should mixed `idf:places` (living *what* vs artifact CRS) appear in typed slots so a Position binder cannot silently promote coords → who?
5. When (if ever) observer DID / QRC may graduate from provisional topology — until then Alice will refuse them as who-features.
6. F2 §19 `idf:RelationScopedLocatorShape` **LANDED** `4a44b0d` as the first-class shape for spine §3g locators, distinct from legacy `idf:NetworkAddressShape` (LIG IP/DNS). Locator→who binds remain **fail**.

---

## 6. Fold notes (Neo)

- **This file only** (amend of already-landed F6). Do not invent F2 §19 SHACL or an F5 §12 in this PR — cite the room name + existing F5 speak.
- Do **not** treat this PR as Host, vibe-host, or ALL_BOUND work.
- Spine F6 row stays **LANDED**; this is a docs amend against tip `1d55f560` (§3g). Implementation remains blocked on vibe delivery + Capt unlock.
- Cite in later F7 refactor list: typed feature namespaces; refuse DNI=RAR=QSession and keyRole wash; refuse QRC-as-who; refuse locator-as-who / directed-pair concat; ZKP/grant ≠ who.
- Marvin: `idf:RelationScopedLocatorShape` F2 §19 landed `4a44b0d`; Alice binds that typed instrument namespace.

---

## 7. Changelog

| When | Note |
|------|------|
| 2026-09-06 | Alice F6 pressure-test parked against fabric HEAD `55d811b` (includes §1b long-arc + F2 crypto-skim `42dc709`). Cites F1 taxonomy, F2 SHACL split, F5 diagnose map §2–§3. Verdict: docs planes pass; default neural bag fail/risk; symbolic pass if specialized shapes + keyRole held. |
| 2026-09-06 | **Amend:** relation-scoped locators as inference constraints (§8) against tip `1d55f560` (spine §3g). Locators = instruments, not who. Email sketch is two directed relation instruments. Agents-of-entities ≠ NaturalAgent. Collapse: static forever-address or locator-as-identity → fail; typed instrument namespace only. Cites spine §3g, F5 diagnose speak (§2–§3 / §11); F2 `RelationScopedLocatorShape` named, **not on this tip** (F2 ends §18). Prior locks held: who ≠ claim ≠ spatiotemporal ≠ instruments; ZKP/grant ≠ who; `keyRole`; DNI ≠ RAR ≠ QSession. No Host invent. |

---

## 8. Amend — relation-scoped locators as inference constraints (spine §3g)

**Cite:** Capt spine [`IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md`](./IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md) **§3g** at tip `1d55f56092295f72fcbe61a5583c781b4e270602` · F5 diagnose map §2 plane voice / §3 “DNI / address as persistent who” / §11 HTTP·Solid offramp · F2 later **§19 `idf:RelationScopedLocatorShape`** (name only; shape not on this tip) · F1 §5.1 network address / locator (not DID; not who).

**Prior locks remain:** who ≠ claim ≠ spatiotemporal ≠ instruments · ZKP / grant / policy ≠ who · `idf:keyRole` purpose-split · DNI ≠ RAR ≠ QSession.

No Host invent. No SHACL invent. No `qualia.*` / ALL_BOUND. This is a **docs map** for later inference constraints.

### 8.1 Locators are instruments, not who

A relation-scoped locator string is an **instrument** (handle-adjacent: alias / contextual IRI / DNI-like mobility). It binds a *relation* or context — not a static forever who-address.

| Scoped to | What the string names | Not |
|-----------|----------------------|-----|
| Pairwise | Two-party directed relation | Either person’s NaturalAgent identity |
| Group | Group membership / roster context | Shared who-bag for members |
| Chat | Conversation / channel context | Participant who embedding |
| Transaction | That transaction / exchange | Payer/payee who |
| DNS code (e.g. TXT) | That published relation/code | Domain owner as timeless who |

Solid WebIDs and phone numbers that behave as **static personal addresses** are the rejected default (spine §3g). Solid/HTTP remain **offramps** (F5 §11); they must not re-impose a single static who-address as the trust root.

**Diagnose speak (align F5 §2 instrument + §3 address detector):** relation locator · pairwise / group / chat / transaction instrument · scoped handle · how now.  
**Never say:** your identity · the person · forever mailbox · who-email · static who-address · controller fact from the locator alone.

### 8.2 Email sketch — two directed instruments, never a who concat

Spine §3g sketch: the user controls the domain (or equivalent); each counterpart gets a **scoped receive address**.

| String | Instrument | Direction |
|--------|------------|-----------|
| `jane@bob.tld` | Jane→Bob relation locator | Directed receive/context for that pair |
| `bob@jane.tld` | Bob→Jane relation locator | The **other** directed instrument |

These are **two** typed instruments. Later binders MUST keep them in `instrument.locator.relation.*` (or F2 `idf:RelationScopedLocatorShape` when landed).

**Gate fail:** concatenate `jane@bob.tld` ⊕ `bob@jane.tld` (or a roster of such strings) into a `who.*` / NaturalAgent embedding, a shared “identity” URI, or a single person vector. The concat invents a who that neither locator names.

### 8.3 Agents-of-entities in metadata ≠ NaturalAgent who

Spine §3g: agents of entities appear in the **metadata / semantics of the relation**, not as the mailbox who.

| Feature | Namespace | Forbidden |
|---------|-----------|-----------|
| Locator string | `instrument.locator.relation.*` | `who.*` |
| Agents listed on the relation (bots, org agents, service agents) | relation metadata / AI-agent or instrument plane (F5 §9) | NaturalAgent who features |
| Living counterpart (Jane, Bob) | `who.*` via `idf:relatedByInstrument` only | Invented who-IRI from the locator |

Signing or speaking *as* an agent-of-entity on the relation does not identify a human principal (QDNF ontological-contracts; F2 `idf:agentKeyOf`).

### 8.4 Collapse detector (hard negative)

| Input pattern | Verdict | suggested_form (F5) |
|---------------|---------|---------------------|
| Locator treated as static forever-address / Solid-or-phone who-token | **fail** | Split: instrument/handle vs NaturalAgent; speak scoped relation, not person |
| Locator string embedded as identity / person key / who-class | **fail** | Rename to typed instrument; keep `idf:NaturalAgentShape` empty of locator literals |
| Directed pair or group roster concat → one who vector | **fail** | Keep each locator a separate `instrument.*` feature; relate via `idf:relatedByInstrument` |
| Agents-of-entities on the locator promoted to who | **fail** | Metadata of the relation; AI-agent / instrument plane (F5 §9), not kin/person |
| Typed instrument namespace (`instrument.locator.relation.*` / later `idf:RelationScopedLocatorShape`) | **pass** (when shape exists); **hold / not-yet** until F2 §19 lands | F5 gates: held / not yet — never guess a who |

**Typed instrument namespace only.** Do not reuse `idf:NaturalAgentShape`, a generic `Identity`/`Agent`/`did:*` class, or legacy `idf:NetworkAddressShape` (LIG IP/DNS) as the who slot for §3g locators.

Optional future fixture id (docs only — no invent now): **F6-L** locator-as-who / directed-pair concat — reject; suggest separated instruments.

### 8.5 What this does not relax

- Four strata stay unmerged: **who ≠ claim ≠ spatiotemporal ≠ instruments**.
- ZKP success / situational grant / signed policy ≠ who (F5 §11, F2 §18).
- `idf:keyRole` stays purpose-separated; session-authentication ≠ route-update ≠ controller-signing.
- DNI ≠ RAR ≠ QSession; deprecated `idf:DniRarSessionShape` remains a hard-negative.
- `did:q42` / observer DID remain provisional topology, not who.
- No Host / vibe-host / ALL_BOUND / dotted `qualia.*` from this amend.

---

*End of WIP — Alice F6 classifier / symbolic-binding pressure-test. No Host invent.*
| 2026-09-06 | Neo fold: PR #78 → `eabccc9`; cite refresh — F2 §19 / F5 §12 on `4a44b0d`. |
