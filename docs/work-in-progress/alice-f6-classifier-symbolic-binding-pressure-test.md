# WIP — Alice F6: classifier / symbolic-binding pressure-test

**Status:** work-in-progress · **Not standards** · **Branch:** `0.0.36-dev`  
**Owner:** Alice (inference / ML / symbolic AI) · **Fold/push:** Neo · **Ops:** Capt.  
**Against fabric HEAD:** `c7fa0b85` (F5 **§13** env-capacity · relationship≠identity · dense-graph≠who · Quin≠identity) · includes F2 **§25** `9c66f46d` · F1 **§25** + spine **§3m** `acd3da35` · F2 **§24** `b86f2639` · F1 **§24** + spine **§3l** `ceb59d93` · spine **§3k** `ec6ea8f4` · F1/F2 **§23** `6e0de103` · prior secrets fold `ac1d12c` (§3h / F1+F2 §20) · prior locator fold `eabccc9` / `4a44b0d` (§3g / F2 §19 / F5 §12)  
**Room tips resolved on this HEAD:** `ec6ea8f` · `ceb59d9` · `b86f263` · `acd3da3` · **F5 §13** `c7fa0b8`  
**F2 content SHA (history):** `565097f` · **F5 diagnose SHA:** `796a7d4` · **F5 §11:** `1d55f560` · **F5 §12 / F2 §19:** `4a44b0d` · **§3h / F1+F2 §20:** `ac1d12c` · **§3k / F1+F2 §23:** `ec6ea8f` / `6e0de10` · **§3l / F1+F2 §24:** `ceb59d9` / `b86f263` · **§3m / F1+F2 §25:** `acd3da3` / `9c66f46` · **F5 §13:** `c7fa0b85`  
**F2 §19:** `idf:RelationScopedLocatorShape` **LANDED** tip `4a44b0d` — typed instrument namespace for spine §3g locators.  
**F2 §20:** `idf:OnlineAccountShape` · `idf:WalletShape` · `idf:BearerTokenShape` / `idf:OAuthTokenShape` · `idf:PasswordVerifierShape` · `idf:PrivateKeyMaterialShape` **LANDED** tip `ac1d12c` — secrets/wallets/tokens/accounts are instruments, not who.  
**F2 §23:** `idf:EnvironmentPredicateShape` · `idf:PlaceBoundSecretShape` · `idf:SensorIdShape` · `idf:GisEnvironmentBindingShape` · `idf:EnvironmentAttestationShape` **LANDED** `6e0de10`.  
**F2 §24:** `idf:RelationshipAssessmentClaimShape` · `idf:KnowabilityAssertionShape` · `idf:NormativeRuleShape` · `idf:RuleBreachClaimShape` **LANDED** `b86f263`.  
**F2 §25:** `idf:RelationLifecycleShape` + Quin substrate note **LANDED** `9c66f46`.  
**F5 §13:** env-capacity · relationship≠identity · dense-graph≠who · Quin≠identity **LANDED** `c7fa0b85`.  
**Constraint:** docs / illustration only. **No Host invent. No `ALL_BOUND` invent. No vibe-host surface widen. No F7 network-stack invent.** Neo folds.

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
- **Secrets / wallets / tokens / online accounts / passwords ≠ who** — spine §3h + F1/F2 §20: instrument/relation kinds; never NaturalAgent / who embeddings. Per-account emails (e.g. `grok@mydomain.tld`) are the same locator cut as §3g / §8 (see §9)
- **Env-conditioned capacity ≠ who** — spine §3k + F1/F2 §23 + F5 §13: GIS / network / sensor identifiers (ATM BLE, geocache place-secrets) *scope* grants/proofs — never who (see §10)
- **Relationship arc / cultural rules ≠ who** — spine §3l + F1/F2 §24 + F5 §13: time-indexed claim–policy (RelationshipAssessmentClaim · KnowabilityAssertion · NormativeRule · RuleBreachClaim); deontic ≠ epistemic; culpability stays off who (see §11)
- **Dense non-defining graph / Quin ≠ who** — spine §3m + F1/F2 §25 + F5 §13: high-cardinality relations do not define counterparts; Quin/NQuin = axiom substrate, not identity (see §12)
- **Pseudonyms (privacy or legal) ≠ second who** — locator/instrument aliases; same collapse as §8 relation-scoped addresses (see §13)
- **Anti-coercion / sanctuary policy ≠ who** — claim–policy / deontic instruments in the sanctuary environment (Webizen Desktop); **forward pointer** — no dedicated fabric shapes on this tip (see §14)

`did:q42` / observer DID remain **provisional topology/coord join keys**, not “who.”

**Gate fail:** any neural concat, softmax, shared URI class, or SHACL target that merges the four strata (or merges keyRole / DNI·RAR·QSession / locator-as-who / ZKP-or-grant-as-who / secret·wallet·token·account-as-who / env-grant-or-co-presence-as-who / relationship-changed-as-identity / dense-graph-as-who / pseudonym-as-who / sanctuary-policy-as-who) into one CS-style “identity” / feature bag.

---

## 0. Stance (locked)

- F6 is a **docs map** for later inference constraints. It does not invent Host methods, dotted `qualia.*`, opcodes, or ALL_BOUND rows.
- Spine §1b (`55d811b`): this fabric is **not** a greenfield identity product. W3C DIDs/VCs/Solid appear in Noddy F1 as **interop/offramp instruments in a ~2000→now lineage**, not as “who.” Classifiers must not treat “W3C identity stack” as a single embedding class.
- Spine §3g (`1d55f560`): relation-scoped locators are instruments/handles, not NaturalAgent. Solid/phone static-address pattern is rejected as the default.
- Spine §3h (`ac1d12c`): secrets · wallets · tokens · online accounts · passwords · per-account emails are instruments/relations, not who-tokens. **F7** (QDNF network-stack implications) is a forward pointer only.
- Spine §3k (`ec6ea8f`) / F1+F2 §23 / F5 §13 (`c7fa0b85`): environment identifiers condition capacity; they do not name who.
- Spine §3l (`ceb59d9`) / F1+F2 §24 / F5 §13: relationship quality and rule-breach are claim–policy over time; they do not rewrite who.
- Spine §3m (`acd3da3`) / F1+F2 §25 / F5 §13: high-cardinality relations + Quin edges are graph form, not identity.
- Completeness bar here = honest pressure-test + named failure modes + later constraints. Implementation waits on Capt / Cursor vibe / Neo fold.

---

## 1. Sources read

Paths as they exist at tip `c7fa0b85`. Short note = what each says about the four strata (and intra-instrument dims).

### 1.1 Identifier Fabric WIP (primary)

| Path | What it says about the strata |
|------|-------------------------------|
| `docs/work-in-progress/IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md` | Spine. CS “identity” = auth subject + identifier + attributes + claims in **one bag** is the problem. Four pillars: natural agent · claim/opinion · spatiotemporal/route handle · instrument kinds. `did:q42` / observer DID = provisional topology, not who. **§1b:** long-arc lineage; DIDs/VCs are instruments, not who. **§3g** (`1d55f560`): locators name a *relationship*, not a permanent who-token. **§3h** (`ac1d12c`): secrets · wallets · tokens · online accounts · passwords are instruments/relations, not who-tokens. **§3k** (`ec6ea8f`): GIS/sensor/network identifiers condition capacity — instruments + handles, not who. **§3l** (`ceb59d9`): relationship assessment + epistemic rule-breach are claim–policy, not who-rewrite. **§3m** (`acd3da3`): high-cardinality non-defining relations; Quin/NQuin = axiom substrate, not identity. **F7** queued as QDNF network-stack pointer only. |
| `docs/work-in-progress/CRYPTO_INSTRUMENT_TAXONOMY_WIP.md` | **F1 / Noddy.** Planes table: who ≠ claim ≠ handle ≠ instrument. Instruments bind *relations*; they do not replace the agent. QDNF role table stays authoritative for *what question an identifier answers*. Biometric **family** vs **instance**. Collapse of any kind into “who” = gate fail. **Alias** row: collapse to who = **fail**. **§20:** password/token/wallet/online-account kinds. **§23:** environment-scoped sensors · place-bound secrets · GIS predicates. **§24:** RelationshipAssessmentClaim · KnowabilityAssertion · NormativeRule · RuleBreachClaim (time + epistemic). **§25:** high-cardinality non-defining relations · Quin axiom substrate. Alice handoffs: sensor/GIS = handle/instrument; culpability/arc = claim–policy; counterpart-set ≠ NaturalAgent embedding. |
| `docs/work-in-progress/IDENTIFIER_FABRIC_SHACL_SPLIT_WIP.md` | **F2 / Marvin** (content `565097f` + crypto-skim amend `42dc709`; tip through **§25**). Four plane shapes + specialized instrument/claim packs. **§19** locators · **§20** secrets/wallets/tokens/accounts. **§23** env predicates / place-bound secrets / sensor ids. **§24** relationship-assessment + epistemic/deontic breach shapes. **§25** `idf:RelationLifecycleShape` + Quin ≠ who. Cross-plane preds are **relations, not merges**. No dedicated Pseudonym / AntiCoercion fabric shapes on this tip — cite Alias + locator / policy+deontic until Marvin lands them. |
| `docs/work-in-progress/IDENTIFIER_FABRIC_DIAGNOSE_MAP_WIP.md` | **F5 / Vibe** (`796a7d4` + §11/§12 + **§13 `c7fa0b85`**). **§2 plane voice** = locked feature-space *labels*. **§3 collapse detectors** include alias-as-route-who. **§11:** ZKP/grant/policy ≠ who. **§12:** locators ≠ who. **§13:** env-grant/co-presence ≠ who · relationship-changed ≠ identity-changed · social-graph/counterpart-set ≠ who · Quin ≠ who-token. Alice cites this voice; does **not** invent further F5 sections. |

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

### 2.7 Secrets / wallets / tokens / online accounts / passwords — instruments, not who (spine §3h)

| Track | Verdict | Why |
|-------|---------|-----|
| (a) Neural | **fail** if password, wallet, token, online-account, seed, or per-account-email features are concatenated into a person / NaturalAgent vector, or if login/possession success is used as a who label | Spine §3h + F1/F2 §20: these are instrument/relation kinds. Concat is the CS bag. Possession ≠ who (same as ZKP/grant). Per-account `grok@mydomain.tld` is a §3g locator, not static identity. |
| (b) Symbolic | **pass** only if binders use typed `instrument.*` + F2 §20 shapes (`OnlineAccount` · `Wallet` · `BearerToken`/`OAuthToken` · `PasswordVerifier` · `PrivateKeyMaterial`) and §19 locators for per-account mailboxes; **fail** if they reuse `idf:NaturalAgentShape` or a shared `Identity`/`Agent` URI | Vault hygiene: secrets/keys never Quin/log/who features. `accountHolder` ≠ `usedBy` ≠ who-forever. |

### 2.8 Env-conditioned capacity — instruments + handles, not who (spine §3k)

| Track | Verdict | Why |
|-------|---------|-----|
| (a) Neural | **fail** if GIS / sensor / BLE / place-secret / env-grant-success or co-presence features are concatenated into a person vector | Spine §3k + F1/F2 §23 + F5 §13: these *scope* grants/proofs. Env-grant or ATM BLE co-presence ≠ who. |
| (b) Symbolic | **pass** if binders use F2 §23 shapes (`EnvironmentPredicate` · `PlaceBoundSecret` · `SensorId` · `GisEnvironmentBinding` · `EnvironmentAttestation`) on `instrument.*` / `handle.*`; **fail** if they reuse `idf:NaturalAgentShape` | Stale env grant MUST NOT silently pass; BLE/MAC alone is weak. |

### 2.9 Relationship arc / cultural rules — claim–policy, not who (spine §3l)

| Track | Verdict | Why |
|-------|---------|-----|
| (a) Neural | **fail** if good-faith→adverse, “reasonably knowable,” knew-vs-didn’t-understand, or culpability features rewrite a person / NaturalAgent embedding | Spine §3l + F1/F2 §24 + F5 §13: time-indexed claims. Deontic (broke) ≠ epistemic (knew). Relationship-changed ≠ identity-changed. |
| (b) Symbolic | **pass** if binders use F2 §24 claim–policy shapes only; **fail** if they type “bad actor” as NaturalAgent | Prior good-faith claim stays provenance. Cultural rule ≠ universal who-attribute. |

### 2.10 Dense non-defining graph / Quin — axiom substrate, not who (spine §3m)

| Track | Verdict | Why |
|-------|---------|-----|
| (a) Neural | **fail** if a social-graph / counterpart-set / brand-follow embedding is used as who | Spine §3m + F1/F2 §25 + F5 §13: high-cardinality relations do not constitute the agent. Packing lifetime counterparts into one who vector is the CS bag. |
| (b) Symbolic | **pass** if binders keep views on relation/context + `idf:RelationLifecycleShape` + Quin as storage form; **fail** if Quin/NQuin is treated as a who-token | Dissolution ≠ erase provenance; dissolution ≠ rewrite person type. |

### 2.11 Pseudonyms (privacy or legal) — locator/instrument aliases, not a second who

| Track | Verdict | Why |
|-------|---------|-----|
| (a) Neural | **fail** if a privacy or legal pseudonym string is embedded as a second NaturalAgent / person vector | Same collapse as §8 locators + F1 Alias row + F5 §3 alias detector. Pseudonym names a *scoped alias instrument*, not another who. |
| (b) Symbolic | **pass** only as typed `instrument.*` / locator-alias (F2 Alias + §19 locator affinity); **fail** if a second `idf:NaturalAgentShape` is minted from the string | No dedicated Pseudonym shape on this tip — do not invent one; do not invent a who-IRI. |

### 2.12 Anti-coercion / sanctuary policy — claim–policy / deontic, not who (forward pointer)

| Track | Verdict | Why |
|-------|---------|-----|
| (a) Neural | **fail** if sanctuary-mode, duress-lane, or anti-coercion framework features are used as a person / “safe-who” embedding | Webizen Desktop / sanctuary environment is policy + deontic instrument context, not identity. Possession of a sanctuary unlock ≠ who (same as ZKP/grant). |
| (b) Symbolic | **hold / not-yet** — no dedicated AntiCoercion / SanctuaryPolicy fabric shapes on this tip; cite F2 §18 policy + deontic/claim planes | Forward pointer only. No Host invent. Do not mint who from sanctuary status. |

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
| **Secret/wallet/token/account-as-who** | Password, wallet, bearer/OAuth token, online-account, or seed features concatenated into a person vector, or login/possession used as who | Spine §3h / F1+F2 §20: instrument/relation kinds. Possession ≠ who. Typed `instrument.*` only |
| **Per-account-email-as-who** | `grok@mydomain.tld` (or any service-bond mailbox) embedded as static forever identity | Same cut as §3g / §8: relation-scoped locator, not NaturalAgent |
| **Env-grant / co-presence-as-who** | GIS match, ATM BLE, geocache find, or env-scoped grant success used as person vector | Spine §3k / F1+F2 §23 / F5 §13: instruments + handles *scope* capacity; they are not who |
| **Relationship-changed-as-identity** | Good-faith→adverse, knowability, or culpability baked into NaturalAgent / “bad actor” type | Spine §3l / F1+F2 §24 / F5 §13: claim–policy over time; deontic ≠ epistemic |
| **Dense-graph-as-who** | Social-graph / counterpart-set / brand-follow embedding, or Quin used as who-token | Spine §3m / F1+F2 §25 / F5 §13: non-defining high-cardinality; Quin = axiom substrate |
| **Pseudonym-as-who** | Privacy or legal alias string as a second NaturalAgent / person embedding | Locator/instrument alias (F1 Alias · §8); not a second who |
| **Sanctuary-policy-as-who** | Anti-coercion / sanctuary-environment / duress-lane status as identity or safer-who | Claim–policy / deontic instruments; forward pointer — no fabric shape on this tip |

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
11. **Relation-scoped locators.** Feature only as typed `instrument.*` (`idf:RelationScopedLocatorShape`). Never `who.*`. Never concat directed pair / group roster into a NaturalAgent vector. Agents-of-entities stay metadata of the *relation*, not who features. See §8.
12. **Secrets / wallets / tokens / online accounts / passwords.** Feature only as typed `instrument.*` (F2 §20 shapes). Never `who.*`. Never concat password/wallet/token/account features into a person vector. Per-account emails stay locator instruments (§8 / §9). No login/possession → who entailment. **F7 QDNF** is a forward pointer only — do not invent network-stack features here.
13. **Env-conditioned capacity.** Sensor / GIS / BLE / place-secret / env-predicate features = `instrument.*` + `handle.*` only (F2 §23). Never `who.*`. Env-grant success or co-presence ≠ person vector. See §10. Cite F5 §13.
14. **Relationship arc / cultural rules.** Culpability and assessment stay in `claim.*` / policy namespaces (`RelationshipAssessmentClaim` · `KnowabilityAssertion` · `NormativeRule` · `RuleBreachClaim`). Deontic (broke) ≠ epistemic (knew vs didn’t understand). Relationship-changed ≠ identity-changed. See §11. Cite F5 §13.
15. **Dense non-defining graph / Quin.** High-cardinality counterpart-set / social-graph embeddings ≠ who. Quin/NQuin = axiom substrate (typed, time-bounded edges), not identity. See §12. Cite F5 §13.
16. **Pseudonyms (privacy or legal).** Feature only as locator/instrument aliases (`instrument.*` / Alias + §19 locator affinity). Never a second `who.*` / NaturalAgent. Same collapse as §8. See §13.
17. **Anti-coercion / sanctuary policy.** Feature only as claim–policy / deontic instruments in the sanctuary environment. Never `who.*`. **Forward pointer** — no dedicated fabric shapes on this tip. See §14. No Host invent.

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
7. F2 §20 instrument shapes **LANDED** `ac1d12c` (`OnlineAccount` · `Wallet` · `BearerToken`/`OAuthToken` · `PasswordVerifier` · `PrivateKeyMaterial`). Confirm per-account email stays on §19 locator (not a second who-shaped mailbox). Secret/wallet/token/account→who binds remain **fail**.
8. F2 §23 env shapes **LANDED** `6e0de10`. Confirm sensor/GIS/place-secret stay handle/instrument — env-grant→who binds remain **fail**.
9. F2 §24 claim–policy shapes **LANDED** `b86f263`. Confirm culpability/arc never type NaturalAgent. Relationship-changed→identity-changed remains **fail**.
10. F2 §25 `idf:RelationLifecycleShape` + Quin note **LANDED** `9c66f46`. Confirm counterpart-set / Quin never become who.
11. Prefer a later **`idf:PseudonymAliasShape`** (privacy/legal) as a specialized locator/instrument — or confirm Alias + §19 suffice. Do not mint a second NaturalAgent from a handle.
12. **Anti-coercion / sanctuary policy:** no dedicated fabric shape on this tip. When (if) one lands, keep it on claim–policy / deontic — never who. Alice will not invent Host or SHACL here.

---

## 6. Fold notes (Neo)

- **This file only** (amend of already-landed F6). Do not invent Host, SHACL, further F5 sections, or F7 QDNF network-stack docs in this PR.
- **Cite (already on tip `c7fa0b85`):** F5 §13 · spine §3k–§3m · F1/F2 §23–§25. Do **not** invent F5 §13 — it is **LANDED** `c7fa0b85`.
- Pseudonyms = locator/instrument aliases (§13). Anti-coercion / sanctuary = claim–policy / deontic **forward pointer** (§14) — no dedicated shapes on this tip; no Host invent.
- Do **not** treat this PR as Host, vibe-host, or ALL_BOUND work.
- Spine F6 row stays **LANDED**; this is a docs amend against tip `c7fa0b85`. Implementation remains blocked on vibe delivery + Capt unlock.
- Cite in later F7 refactor list: typed feature namespaces; refuse DNI=RAR=QSession and keyRole wash; refuse QRC-as-who; refuse locator-as-who / directed-pair concat; ZKP/grant ≠ who; refuse secret/wallet/token/account-as-who; **refuse env-grant/co-presence-as-who**; **refuse relationship-changed-as-identity**; **refuse dense-graph/Quin-as-who**; **refuse pseudonym-as-who**; **refuse sanctuary-policy-as-who**. F7 = pointer only until Capt opens.
- Marvin: F2 §19–§25 landed through `9c66f46`; Alice binds typed `instrument.*` / `handle.*` / `claim.*` only.

---

## 7. Changelog

| When | Note |
|------|------|
| 2026-09-06 | Alice F6 pressure-test parked against fabric HEAD `55d811b` (includes §1b long-arc + F2 crypto-skim `42dc709`). Cites F1 taxonomy, F2 SHACL split, F5 diagnose map §2–§3. Verdict: docs planes pass; default neural bag fail/risk; symbolic pass if specialized shapes + keyRole held. |
| 2026-09-06 | **Amend:** relation-scoped locators as inference constraints (§8) against tip `1d55f560` (spine §3g). Locators = instruments, not who. Email sketch is two directed relation instruments. Agents-of-entities ≠ NaturalAgent. Collapse: static forever-address or locator-as-identity → fail; typed instrument namespace only. Cites spine §3g, F5 diagnose speak (§2–§3 / §11); F2 `RelationScopedLocatorShape` named, **not on this tip** (F2 ends §18). Prior locks held: who ≠ claim ≠ spatiotemporal ≠ instruments; ZKP/grant ≠ who; `keyRole`; DNI ≠ RAR ≠ QSession. No Host invent. |
| 2026-09-06 | Neo fold: PR #78 → `eabccc9`; cite refresh — F2 §19 / F5 §12 on `4a44b0d`. |
| 2026-09-06 | **Amend:** secrets / wallets / tokens / online accounts / passwords as inference instruments (§9) against tip `ac1d12c` (spine §3h · F1/F2 §20). Instrument/relation kinds — never NaturalAgent / who embeddings. Per-account emails (e.g. `grok@mydomain.tld`) = §3g / §8 locators. Collapse: password/wallet/token/account concat into a person vector → fail; typed `instrument.*` only. F7 QDNF = forward pointer only. Prior locks held: who ≠ claim ≠ spatiotemporal ≠ instruments; locators §8; ZKP/grant ≠ who; `keyRole`; DNI ≠ RAR ≠ QSession. No Host invent. |
| 2026-09-06 | **Amend:** inference constraints §10–§14 against tip `c7fa0b85` (F5 §13 **LANDED**; spine §3k–§3m; F1/F2 §23–§25). **§10** env-conditioned capacity (GIS/sensor/BLE/place-secrets scope grants ≠ who). **§11** relationship arc / cultural rules (claim–policy time+epistemic; deontic ≠ epistemic; relationship-changed ≠ identity-changed). **§12** dense non-defining graph / Quin (axiom substrate ≠ who). **§13** privacy/legal **pseudonyms** = locator/instrument aliases, never a second NaturalAgent. **§14** anti-coercion / sanctuary policy = claim–policy / deontic **forward pointer** (no fabric shapes on tip). Prior locks held: who ≠ claim ≠ spatiotemporal ≠ instruments; locators; secrets/wallets; ZKP/grant ≠ who; `keyRole`; DNI ≠ RAR ≠ QSession. No Host invent. |

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

## 9. Amend — secrets / wallets / tokens / online accounts / passwords as inference instruments (spine §3h)

**Cite:** Capt spine [`IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md`](./IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md) **§3h** at tip `ac1d12c6f727a05aed0e1481e18082de4b55232b` · Noddy F1 **§20** (`CRYPTO_INSTRUMENT_TAXONOMY_WIP.md`) · Marvin F2 **§20** (`idf:OnlineAccountShape` · `idf:WalletShape` · `idf:BearerTokenShape` / `idf:OAuthTokenShape` · `idf:PasswordVerifierShape` · `idf:PrivateKeyMaterialShape`; per-account email prefers `idf:RelationScopedLocatorShape`) · F6 §8 / spine §3g locators · F5 instrument speak + §3 address detector / §11 ZKP·grant · F1 §15 OS/telecom accounts.

**Prior locks remain:** who ≠ claim ≠ spatiotemporal ≠ instruments · locators ≠ who (§8) · ZKP / grant / policy ≠ who · `idf:keyRole` purpose-split · DNI ≠ RAR ≠ QSession.

**F7** (QDNF / socially defined network-stack implications) is a **forward pointer only**. This amend does not invent network-stack docs, Host methods, or ALL_BOUND rows.

No Host invent. No SHACL invent. No `qualia.*` / ALL_BOUND. This is a **docs map** for later inference constraints.

### 9.1 These are instrument / relation kinds — never who

Secrets, passwords, wallets, tokens, and online accounts are **instruments** (and account *relations*). They bind capability, possession, or a service bond. They are never `idf:NaturalAgentShape` / `who.*` embeddings.

| Kind | What it is | Not |
|------|------------|-----|
| Password / passphrase | Low-entropy authenticator aid (vault-scoped; not key material unless PAKE-specified) | Person vector / who-class |
| Bearer / API / session token | Time-bound capability instrument; possession ≠ who | NaturalAgent identity |
| Refresh / OAuth token | Account-scoped instrument on an online-account relation | Who-forever |
| Wallet (software/hardware) | Artifact container of keys/instruments; may *relate* to an agent | The agent |
| Private key / seed | Purpose-separated `keyRole` material — vault only | Who; never Quin/log features |
| Online account | Platform account instrument (sibling to OsAccount) — `accountHolder` ≠ `usedBy` ≠ who-forever | Living principal |
| Per-account email | Relation-scoped locator (same cut as §3g / §8) bound to that account | Static identity / forever mailbox |

**No entailment** from password-success · token-possession · wallet-unlock · online-account login to NaturalAgent who (same discipline as F5 §11 / F2 §18 ZKP and situational grant).

**Diagnose speak (align F5 §2 instrument):** password verifier · wallet · token · platform account · per-account mailbox · scoped service bond.  
**Never say:** your identity · the person · seed-as-who · forever account-email · login success = who.

### 9.2 Per-account relation emails — same cut as §3g / §8

Spine §3h: `grok@mydomain.tld` (and the like) is a **relation-scoped locator** for one service bond, not a static forever who-address.

Same inference cut as F6 §8:

| String | Instrument | Forbidden |
|--------|------------|-----------|
| `grok@mydomain.tld` | Locator for that online-account / service relation | `who.*` / NaturalAgent key |
| Directed pair or roster of per-account mailboxes | Separate `instrument.locator.relation.*` features | Concat into one person vector |

Later binders MUST keep these in `instrument.locator.relation.*` / `idf:RelationScopedLocatorShape` (F2 §19 **LANDED**). Do not reuse `idf:NaturalAgentShape`, a shared `Identity`/`Agent` URI, or legacy `idf:NetworkAddressShape`.

### 9.3 Collapse detector (hard negative)

| Input pattern | Verdict | suggested_form |
|---------------|---------|----------------|
| Password / wallet / token / account features concatenated into a person / NaturalAgent vector | **fail** | Typed `instrument.*` only; relate via `idf:relatedByInstrument` |
| Password-success or token-possession used as who label | **fail** | Envelope/capability instrument; no who entailment |
| Online-account login ⇒ NaturalAgent identity | **fail** | `idf:OnlineAccountShape`; `accountHolder` optional; `usedBy` independent |
| Wallet or seed phrase as person embedding / chrome identity | **fail** | Name the wallet / keyRole member; vault-scoped; never who |
| One email forever = who (including per-account `grok@mydomain.tld`) | **fail** | Relation-scoped locator (§8); not static identity |
| Typed `instrument.*` (`instrument.account.*` · `instrument.wallet.*` · `instrument.token.*` · `instrument.secret.verifier.*` · locator namespace) | **pass** when F2 §20 shapes are bound; **hold / not-yet** if a binder guesses who | F5 gates: held / not yet — never guess a who |

**Typed `instrument.*` only.** Secrets must not enter who feature space (F1 §20 Alice handoff). Private keys / seeds never become Quin, log, or embedding features (vault hygiene).

Optional future fixture id (docs only — no invent now): **F6-S** secret/wallet/token/account-as-who / feature-concat — reject; suggest separated instruments.

### 9.4 What this does not relax

- Four strata stay unmerged: **who ≠ claim ≠ spatiotemporal ≠ instruments**.
- Locators stay instruments, not who (§8 / spine §3g).
- ZKP success / situational grant / signed policy ≠ who (F5 §11, F2 §18).
- `idf:keyRole` stays purpose-separated; session-authentication ≠ route-update ≠ controller-signing. Wallet keys that co-attest stay **members with distinct keyRoles**, not a mega-who.
- DNI ≠ RAR ≠ QSession; deprecated `idf:DniRarSessionShape` remains a hard-negative.
- `did:q42` / observer DID remain provisional topology, not who.
- **F7 QDNF** = forward pointer only — no network-stack invent from this amend.
- No Host / vibe-host / ALL_BOUND / dotted `qualia.*` from this amend.

---

## 10. Amend — env-conditioned capacity as inference constraints (spine §3k / F1–F2 §23 / F5 §13)

**Cite:** Capt spine [`IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md`](./IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md) **§3k** at tip `ec6ea8f4f2c58f5ce6fdb71d3b6157c38d9686c8` · Noddy F1 **§23** · Marvin F2 **§23** (`idf:EnvironmentPredicateShape` · `idf:PlaceBoundSecretShape` · `idf:SensorIdShape` · `idf:GisEnvironmentBindingShape` · `idf:EnvironmentAttestationShape`) at `6e0de10351e2b5c867ebf197a38565cd33732f48` · Vibe F5 **§13** at tip `c7fa0b8583bb62cf4ba225046a58985aabc3fdb0` · F1 §17 situational grants · F2 §18 policy/ZKP · G-COORD Position · F6 §8 locators · F6 §9 secrets.

**Prior locks remain:** who ≠ claim ≠ spatiotemporal ≠ instruments · locators ≠ who (§8) · secrets/wallets/tokens/accounts ≠ who (§9) · ZKP / grant / policy ≠ who · `idf:keyRole` purpose-split · DNI ≠ RAR ≠ QSession.

No Host invent. No SHACL invent. No `qualia.*` / ALL_BOUND. This is a **docs map** for later inference constraints.

### 10.1 Instruments + spatiotemporal handles that *scope* grants — never who

GIS / network / sensor identifiers (ATM BLE, geocache place-secrets, realm / network cell) are **instruments + spatiotemporal handles**. They condition *how* capacity and proofs apply. They do not name a NaturalAgent.

| Pattern | Feature namespace | Not |
|---------|-------------------|-----|
| Works only in environment E | `instrument.*` + `handle.*` predicates on a situational grant / policy | Person vector / who-class |
| Different environment | Re-evaluate; flag, degrade, or deny | Silent reuse as who-forever |
| Geocaching | Place-bound **secret instrument** (discovery when location/handle predicates match) | Person identity from find |
| ATM + phone bank | Co-presence: app + ATM BLE/machine id + location → higher-assurance **capacity** | Bank-customer who from BLE |
| Environment attestation | Claim/evidence: “device X observed sensor Y at locus Z” | Who |

**Diagnose speak (align F5 §13):** context instruments + handles · place-bound secret instrument.  
**Never say:** who · person · banker-from-ATM-BLE · person identity from find.

**Hardness:** prefer multi-instrument co-attestation (phone key · ATM BLE · location · time window). BLE/MAC alone is weak / spoofable — not identity.

### 10.2 Collapse detector (hard negative)

| Input pattern | Verdict | suggested_form (F5 §13) |
|---------------|---------|-------------------------|
| Env-grant success ⇒ person / NaturalAgent vector | **fail** | Split instruments+handles vs NaturalAgent |
| Co-presence (ATM BLE + app + locus) used as who | **fail** | Name the scoped grant / co-attestation; keep who empty of sensor literals |
| Geocache find or GIS-alone as forever who | **fail** | Place-bound secret / handle; not person |
| Typed `instrument.*` + `handle.*` (F2 §23 shapes) | **pass** when bound; **hold / not-yet** if a binder guesses who | F5 gates: held / not yet — never guess a who |

**Typed `instrument.*` / `handle.*` only.** No entailment from environment-match or co-presence to NaturalAgent (same discipline as F5 §11 ZKP/grant).

Optional future fixture id (docs only — no invent now): **F6-E** env-grant / co-presence-as-who — reject; suggest separated instruments+handles.

### 10.3 What this does not relax

- Four strata stay unmerged: **who ≠ claim ≠ spatiotemporal ≠ instruments**.
- Locators stay instruments, not who (§8). Secrets/wallets/accounts stay instruments, not who (§9).
- ZKP success / situational grant / signed policy ≠ who (F5 §11, F2 §18).
- `idf:keyRole` stays purpose-separated; DNI ≠ RAR ≠ QSession.
- `did:q42` / observer DID remain provisional topology, not who.
- No Host / vibe-host / ALL_BOUND / dotted `qualia.*` from this amend.

---

## 11. Amend — relationship arc / cultural rules as inference constraints (spine §3l / F1–F2 §24 / F5 §13)

**Cite:** Capt spine **§3l** at tip `ceb59d938fbaee855d15c92af7b6baa5a400d862` · Noddy F1 **§24** · Marvin F2 **§24** (`idf:RelationshipAssessmentClaimShape` · `idf:KnowabilityAssertionShape` · `idf:NormativeRuleShape` · `idf:RuleBreachClaimShape`) at `b86f26398f4691d7bf4b0f3e5cebfca56cb86cca` · Vibe F5 **§13** at `c7fa0b85` · F1 §16 modalities · §17 sense-context · §18 policy · §21 symbolic permissions.

**Prior locks remain:** who ≠ claim ≠ spatiotemporal ≠ instruments · locators ≠ who · secrets/wallets ≠ who · env-capacity ≠ who (§10) · ZKP / grant / policy ≠ who · `idf:keyRole` · DNI ≠ RAR ≠ QSession.

No Host invent. No SHACL invent. No `qualia.*` / ALL_BOUND.

### 11.1 Good-faith → adverse / reasonably-knowable = time-indexed claim–policy

Relationship quality and norm compliance **evolve in time**. They are claim–policy + epistemic/deontic assertions over *relations and contexts* — not NaturalAgent who-tokens, and not a static “good/bad person” embedding.

| Pattern | Fabric expression | Feature namespace |
|---------|-------------------|-------------------|
| Met in good faith; later adverse / knowable over time | Time-indexed **RelationshipAssessmentClaim** + evidence; prior good-faith claim remains provenance | `claim.*` — not deleted who-rewrite |
| Inference over that arc | Typed edges on **relation / context** graphs (parties · epochs · evidence) | Never bake into `who.*` |
| Broke cultural/community rule, didn’t understand | Epistemic: `¬K(rule)` / low awareness · **deontic breach still recorded** · sense-context binds *which* rule | `claim.*` + policy |
| Broke rule knowing it | Epistemic: `K(rule)` · same deontic breach, different culpability on the **claim** | Not a who-merge |
| Depends on the rule | **NormativeRule** + community sense-context; crypto only hardens attestations | Instrument/policy cite, not who |

**Deontic (broke) ≠ epistemic (knew vs didn’t understand).** Culpability / arc features stay in claim–policy namespaces only — never who rewrite.

**Diagnose speak (align F5 §13):** views grow/dissolve on claim–relation lifecycle · epistemic + deontic on claim–policy.  
**Never say:** identity-changed · who-rewrite · permanent who-bit · bad-actor type.

### 11.2 Collapse detector (hard negative)

| Input pattern | Verdict | suggested_form (F5 §13) |
|---------------|---------|-------------------------|
| Relationship-changed ⇒ identity-changed / who rewrite | **fail** | Keep RelationshipAssessmentClaim / RelationLifecycle; NaturalAgent stays |
| Knew-vs-unknowing as permanent who-bit | **fail** | KnowabilityAssertion on the claim; deontic breach recorded separately |
| Cultural rule as universal who-attribute / “bad actor” NaturalAgent type | **fail** | NormativeRule + sense-context; parties stay living who, unmerged |
| Typed `claim.*` / policy (F2 §24 shapes) | **pass** when bound; **hold / not-yet** if a binder guesses who | F5 gates: held / not yet |

Optional future fixture id (docs only): **F6-R** relationship-arc-as-who — reject; suggest claim–policy namespaces.

### 11.3 What this does not relax

- Four strata stay unmerged: **who ≠ claim ≠ spatiotemporal ≠ instruments**.
- Env-conditioned capacity stays instruments+handles, not who (§10).
- Locators · secrets/wallets · ZKP/grant · `keyRole` · DNI ≠ RAR ≠ QSession remain locked.
- No Host / vibe-host / ALL_BOUND / dotted `qualia.*` from this amend.

---

## 12. Amend — dense non-defining graph / Quin as inference constraints (spine §3m / F1–F2 §25 / F5 §13)

**Cite:** Capt spine **§3m** at tip `acd3da359fbac6bfa5f0b4ff0d160f6437d30e39` · Noddy F1 **§25** · Marvin F2 **§25** (`idf:RelationLifecycleShape` + Quin substrate note) at `9c66f46d09fac06809bbb160772a52826c06e024` · Vibe F5 **§13** at `c7fa0b85` · F1 §24 temporal assessment · ADR 0001 Quin alignment · human-centric cut.

**Prior locks remain:** who ≠ claim ≠ spatiotemporal ≠ instruments · locators · secrets/wallets · env-capacity (§10) · relationship arc (§11) · ZKP/grant ≠ who · `idf:keyRole` · DNI ≠ RAR ≠ QSession.

No Host invent. No SHACL invent. No `qualia.*` / ALL_BOUND.

### 12.1 High-cardinality relations do not define one another

Humans (and brands, orgs, AI-agents) have **thousands** of relationships/interactions per month–year–lifetime. Others **do not define** one another. Contextual, developmental subjective/objective views grow and dissolve under rules — on the claim–policy–relation graph, never a who-merge.

| Pattern | Feature namespace | Forbidden |
|---------|-------------------|-----------|
| Many human–human / human–brand / entity relations | Relation edges + assessment claims (cardinality expected) | Social-graph → `who.*` embedding |
| Non-defining counterparts | Views hang on **relation/context** nodes | Counterpart set constitutes NaturalAgent |
| Growth and dissolution | Time-bounded relation axioms (`RelationLifecycle` begin · revise · end) | Dissolution rewrites person type; dissolution erases provenance |
| Quin / NQuin | Axiom **storage/graph form** — typed edges, time-bounded relations (ADR 0001) | Quin as identity / who-token |

**No social-graph→who embeddings.** Quin/NQuin is the axiom substrate, not identity.

**Diagnose speak (align F5 §13):** many relations (expected) · axiom storage / graph form.  
**Never say:** who · “they define each other” · identity = social graph · Quin as identity.

### 12.2 Collapse detector (hard negative)

| Input pattern | Verdict | suggested_form (F5 §13) |
|---------------|---------|-------------------------|
| Dense graph / counterpart-set embedding as who | **fail** | Refuse — non-defining high-cardinality relations |
| Brand follows / lifetime roster packed into one person vector | **fail** | Keep each relation + assessment on context nodes |
| Quin / NQuin used as who-token | **fail** | Name as storage substrate for typed edges |
| Typed relation/lifecycle + claim assessments (F2 §25) | **pass** when bound; **hold / not-yet** if a binder guesses who | F5 gates: held / not yet |

Optional future fixture id (docs only): **F6-Q** dense-graph / Quin-as-who — reject; suggest axiom substrate, not identity.

### 12.3 What this does not relax

- Four strata stay unmerged: **who ≠ claim ≠ spatiotemporal ≠ instruments**.
- Relationship-arc features stay claim–policy (§11). Env features stay instruments+handles (§10).
- Private keys / seeds never become Quin/log/who features (§9 / F1 §20).
- Locators · ZKP/grant · `keyRole` · DNI ≠ RAR ≠ QSession remain locked.
- No Host / vibe-host / ALL_BOUND / dotted `qualia.*` from this amend.

---

## 13. Amend — privacy / legal pseudonyms as locator-instrument aliases (not a second who)

**Cite:** F1 Alias row (collapse to who = **fail**) · F2 Alias Assertions / QDNF alias role · F5 §3 “Alias alone as route authority” · spine **§3g** / F6 **§8** relation-scoped locators · F2 §19 `idf:RelationScopedLocatorShape` · F5 §12 locator voice · QDNF `identifier-resolution.md` (alias answers a *different question* than who).

**Prior locks remain:** who ≠ claim ≠ spatiotemporal ≠ instruments · locators ≠ who · secrets/wallets ≠ who · env-capacity ≠ who · relationship arc ≠ who · dense graph / Quin ≠ who · ZKP/grant ≠ who · `idf:keyRole` · DNI ≠ RAR ≠ QSession.

No Host invent. No SHACL invent. No dedicated `idf:PseudonymAliasShape` on this tip — **do not mint one here**.

### 13.1 A pseudonym is an alias instrument, not another NaturalAgent

Privacy or legal **pseudonyms** (handles, pen-names, court/protected aliases, pairwise presentation names) are **locator/instrument aliases**. They bind a *scoped presentation* of a relation or context. They are never a second `idf:NaturalAgentShape` / `who.*`.

| Kind | What the string names | Not |
|------|----------------------|-----|
| Privacy pseudonym | Scoped alias instrument for that context / relation | A second living who |
| Legal / protected alias | Locator or presentation instrument under a policy/grant | NaturalAgent rewrite |
| Pairwise handle / nick | Relation-scoped locator (same cut as §8) | Forever mailbox / static who-address |

**Same collapse as relation-scoped addresses:** pseudonym string ≠ person embedding. Concatenating legal-name ⊕ privacy-handle into one NaturalAgent vector invents a who that neither string names.

**Diagnose speak (align F5 §2 instrument + §3 alias detector + §8):** scoped alias · locator instrument · presentation handle.  
**Never say:** your other identity · second person · who-from-pseudonym · forever alias-as-who.

### 13.2 Collapse detector (hard negative)

| Input pattern | Verdict | suggested_form |
|---------------|---------|----------------|
| Pseudonym string embedded as a second NaturalAgent / person key | **fail** | Typed `instrument.*` / locator-alias; keep `idf:NaturalAgentShape` empty of alias literals |
| Legal name ⊕ privacy handle concat → one who vector | **fail** | Two instruments; relate via `idf:relatedByInstrument` only |
| Alias used as sole route authority *and* who | **fail** | F5 §3: alias needs provenance; never sole route who |
| Typed Alias + §19 locator affinity | **pass** (namespace); **hold / not-yet** until a specialized shape exists if Marvin wants one | F5 gates: held / not yet — never guess a who |

Optional future fixture id (docs only): **F6-P** pseudonym-as-who — reject; suggest locator/instrument alias.

### 13.3 What this does not relax

- Four strata stay unmerged. Locators stay instruments (§8). Secrets/wallets stay instruments (§9).
- Env-capacity · relationship arc · dense Quin graph remain locked (§10–§12).
- ZKP/grant · `keyRole` · DNI ≠ RAR ≠ QSession remain locked.
- No Host / vibe-host / ALL_BOUND / dotted `qualia.*` from this amend.

---

## 14. Note — anti-coercion frameworks as sanctuary / policy (forward pointer)

**Cite:** F2 **§18** `idf:OntologyGovernedPolicyShape` / `idf:ZkpProofShape` (policy ≠ who; ZKP ≠ who) · F1 §16–§18 modalities / ontology-governed policy · F1 §21 symbolic-first permissions · F5 §11 ZKP·grant·policy ≠ who · F5 sanctuary *voice* (keep / commit on real success — not identity) · Webizen Desktop / sanctuary environment (vault / duress-lane product docs; not a fabric who-plane).

**No dedicated AntiCoercion / SanctuaryPolicy fabric shapes on tip `c7fa0b85`.** This section is a **forward pointer only**. No Host invent. No SHACL invent. No `qualia.*` / ALL_BOUND.

### 14.1 Sanctuary / anti-coercion is claim–policy / deontic, not who

Anti-coercion frameworks (Webizen Desktop sanctuary environment, duress/decoy lanes, fail-closed sanctuary policy) are **claim–policy and deontic instruments**. They constrain *what may be revealed or committed under coercion*. They do not name, replace, or strengthen NaturalAgent who.

| Surface | Plane | Forbidden |
|---------|-------|-----------|
| Sanctuary lock / unlock / fail-closed policy | Policy / deontic instrument context | `who.*` / safer-who embedding |
| Duress or decoy lane | Capability / policy instrument (possession ≠ who) | Person vector from lane choice |
| Anti-coercion framework status | Claim–policy evidence / grant scope | Identity-changed · who-forever |

**Same discipline as ZKP/grant:** sanctuary success or duress-lane use does not entail NaturalAgent who.

**Diagnose speak:** sanctuary policy · deontic instrument · fail-closed context.  
**Never say:** sanctuary identity · safer-who · duress-PIN-as-person.

### 14.2 Collapse detector (hard negative)

| Input pattern | Verdict | suggested_form |
|---------------|---------|----------------|
| Sanctuary / anti-coercion status as who or safer-who | **fail** | Claim–policy / deontic; NaturalAgent stays |
| Duress-lane unlock used as person label | **fail** | Capability instrument; no who entailment |
| Invented `idf:AntiCoercionShape` / Host method in this amend | **fail** (out of scope) | Forward pointer — Marvin/Capt land shapes later |
| Cite F2 §18 policy + deontic/claim planes until a shape exists | **hold / not-yet** | F5 gates: held / not yet — never guess a who |

### 14.3 What this does not relax

- Four strata stay unmerged: **who ≠ claim ≠ spatiotemporal ≠ instruments**.
- Prior F6 locks stay: locators · secrets/wallets · env-capacity · relationship arc · dense Quin graph · **pseudonyms** · ZKP/grant ≠ who · `keyRole` · DNI ≠ RAR ≠ QSession.
- **F7 QDNF** remains a forward pointer only.
- No Host / vibe-host / ALL_BOUND / dotted `qualia.*` from this note.

---

*End of WIP — Alice F6 classifier / symbolic-binding pressure-test. No Host invent.*
