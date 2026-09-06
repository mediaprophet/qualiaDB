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


## 1b. Long-arc lineage (Timothy — 2026-09-06)

QualiaDB / this fabric is **not** a greenfield identity product. Foundations date to ~2000 and evolved through W3C and related work (credentials, DIDs, RWW/Solid, and kin) as **interop and offramp** material. The Identifier Fabric room synthesizes those considerations into requirements for later implementation/refactor — after Cursor’s vibe delivery — without collapsing instruments into CS “identity.”

W3C DIDs/VCs (and related) appear in Noddy F1 as **instruments in that lineage**, not as “who.”


### Hardness (Timothy / Noddy — 2026-09-06)

Cryptographic hardness for human-centric systems is **not** one signature or one identifier. Prefer a *formula* of verified signatures and related instruments, **time-bounded** and scoped to particular machines / networks / entities / agents. Single-instrument “auth” is the easy hack surface; the fabric raises the bar by **multi-instrument co-attestation** without collapsing into a stronger single “who.”

Provenance note: long-arc design records include W3C list traffic searchable via `timothy.holborn@gmail.com` on lists.w3.org (illustrative, not a trust root by itself).

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


## 3b. Entity / agent typing (Timothy — 2026-09-06)

FOAF-modernized **top-level entity and agent-type predicates** sit above instruments. Attributes, properties, and relation axioms hang off those types — they do **not** constitute CS “identity.”

| Layer | Role |
|-------|------|
| Entity / agent types | Natural agent · AI agent · organization · machine/device · … (open list) |
| Attributes & relation axioms | Typed properties and links (e.g. customer, operator) |
| Instrument kinds | Hardware IDs, cert SAN / WebID-TLS / WebID-RSA, signed RDF/VCs, network addresses, DIDs, … |

**Cuts:**
- A **machine** is a bundle of device + network instruments — not a who.
- An **AI agent** is its own agent-type with an identifier array and relations — **not** a NaturalAgent and **not** a machine.
- WebID / SAN / hardware multi-id remain instruments relating to an agent/entity type.


## 3c. WordNet lexical substrate (Timothy — 2026-09-06)

**WordNet** (and later OMW / locale packs) is the foundational **lexical** resource for a rich multilingual vocabulary — prefer a Q42-encoded subset with explicit relations *to* WN concept ids, not a dump-as-identity.

| Rule | |
|------|--|
| WN / lexical concept | Vocabulary and multilingual surface |
| Fabric plane / type IRI | NaturalAgent · AI-agent · machine · claim · handle · instrument |
| Link | Optional `sameAs`/skos-style links from type or instrument defs → WN — **never** merge planes into WN-person |

**Jury bar:** digital evidence stays useful only if a non-specialist can follow enumerable instruments and relations — “these signatures, this window, these machines” — not an opaque who-token or CS “identity” bag.


## 3d. Resolution, modality, guardianship (Timothy — 2026-09-06)

**Resolution:** “Identity” in this fabric is not one sharper blob. It is an **enumerable pattern** of entity/agent types, relation axioms, and instruments. Resolution improves as more *distinguishable, unmerged* bindings are added. Collapse into a single who-node is **information loss**.

| QualiaDB / fabric concern | Plane |
|---------------------------|--------|
| Bifurcation, N3, deontic, epistemic (and kin) | Claim–policy–modality — how assertions and permissions are reasoned |
| Cryptography packages | Instrument proofs + multi-instrument co-attestation hardness |
| Guardianship (and advanced care relations) | First-class **relation axioms** between agents (NaturalAgent · AI-agent · org) — **never** merge guardian into ward |

Enhance crypto only where purpose-separated `keyRole`s or co-attestation still have gaps — not as a parallel “identity” stack.


## 3e. Contextual sense (Timothy — 2026-09-06)

**Sense is contextual** — locale, era, community, and namespace. The “identity” of a word (or of flora/fauna as referents) is not one timeless label any more than a person is one DID.

| Example | Implication |
|---------|-------------|
| AU “thongs” ≈ flip-flops / footwear | Locale sense binding — not underwear by default |
| Older news “gay time” ≈ happy | Era sense binding — not sexuality by default |
| Flora / fauna | Living-typed entities — **not** NaturalAgent personhood; **not** Thing-wash |

Lexical concepts need **sense + context bindings** (WN/OMW · language · time · namespace · provenance) as instruments/aliases. **Crypto does not fix homographs** — context + provenance does. Never a mega-meaning who-token for words.


## 3f. Ontology-governed crypto-bound policy (Timothy — 2026-09-06)

Working name: **ontology-governed, crypto-bound policy over a multi-plane graph** (ABAC-like, but fabric-native).

| Layer | Role |
|-------|------|
| Multi-plane graph | Entities · relations · instruments · claims (unmerged) |
| Attributes + capacity grants | Drive authorization decisions (purpose · condition · time · qualification) |
| Deontic / epistemic / N3 / bifurcation | Claim–policy–modality reasoning |
| Cryptographically signed ontology documents | Contracts · SHACL/N3 bundles that **interpret** policy — **not** HTTP-dependent |
| Instruments | Prove (signatures · co-attestation · keyRoles) |
| HTTP / Solid / LIG | Offramp / compatibility — **not** the trust root |

Attributes authorize; signed ontologies interpret; instruments prove; **who stays unmerged**.


## 3g. Relation-specific addressing (Timothy — 2026-09-06)

Unlike traditional Solid WebIDs or phone numbers that often behave as **static** personal addresses, this fabric prefers **relation-specific** locator strings: the address names a *relationship* (or context), not a permanent who-token.

| Pattern | Role |
|---------|------|
| Pairwise / contextual locator | Two people, group, group-chat, transaction id, DNS TXT code, … |
| Email redesign sketch | User controls domain (or equivalent); each counterpart gets a scoped receive address — e.g. Jane→Bob `jane@bob.tld`, Bob→Jane `bob@jane.tld` |
| Agents of entities | Appear in metadata/semantics of the relation — not as the mailbox who |

**Cut:** relation-scoped locators are **instruments/handles** (affinity with alias / contextual IRI / DNI-like mobility), not NaturalAgent identity. Solid/HTTP remain offramps; they must not re-impose a single static who-address as the trust root.


## 3h. Secrets, wallets, accounts, passwords (Timothy — 2026-09-06)

**In scope** as fabric instruments / relations — **not** who-tokens:

| Kind | Notes |
|------|--------|
| Secrets / passwords | Credential instruments; store/prove without becoming identity |
| Wallets / tokens | Crypto/payment instruments; keyRoles stay purpose-separated |
| Online accounts | Account instruments on platforms (like OS/telecom accounts) — account-holder ≠ who merge |
| Per-account emails | Relation-scoped locators — e.g. `grok@mydomain.tld` for one service bond, not a static forever who-address |

Same §3g cut: generated addresses and account handles are **scoped instruments**. When the picture is sufficient, open **F7** for implications on the socially defined QDNF network stack (docs-only until Cursor vibe lands; no `ALL_BOUND` invent).


## 3i. Symbolic context for permissions (Timothy — 2026-09-06)

Permission and agent authorization **must** consider context enumerated primarily via **symbolic AI**: semantics + logic systems (deontic · epistemic · N3 · bifurcation), including probability where uncertainty belongs — **not** by baking role, grant-success, wallet, or account features into a NaturalAgent embedding.

| Layer | Role |
|-------|------|
| Symbolic context | Enumerates *which* situation and *what* may be done |
| Crypto instruments | Prove + hardness (keyRoles · co-attestation · ZKP) |
| Situational grants + signed agreements | How specialised bots (child-minder · medical · home-security) and personal/group agents get scoped capacity |
| UI/UX | After planes are thick enough — target: simpler than AD / Keychain chrome |

**Cut:** inference keeps role/grant/wallet/account in **typed instrument/policy namespaces** only (Alice F6). Who stays unmerged.


## 3j. Org structure + mutable group authentication (Timothy — 2026-09-06)

Some permission structures require **both**:

1. **Structural elements** — directors, department leads, org charts, decision-making structures (roles as *relations* to orgs/legal personalities, not who-tokens).
2. **Correlated group authentication** that can **change over time** (e.g. after an election) — membership/instrument sets that are time-bounded and re-bound without rewriting NaturalAgent identity.

| Cut | |
|-----|--|
| Structure | Relation axioms on org/legal-personality entity types |
| Group auth | Instrument / co-attestation / membership that tracks current holders |
| Election / turnover | New bindings in time — not a permanent who-embedding of “the board” |

Aligns with §3i symbolic-first context and §3d guardianship/capacity as time-varying relations.

## 3k. Environment-conditioned capacity (Timothy — 2026-09-06)

Sensor / GIS / network-environment identifiers **condition** how capacity and proofs apply. They are instruments + spatiotemporal handles — **not** NaturalAgent who.

| Pattern | Fabric expression |
|---------|-------------------|
| Works only in environment E | Situational grant / policy with environment predicates (GIS · realm · network cell · sensor id) |
| Different environment | Flag, degrade, or deny — do not silently reuse grant |
| Geocaching | Place-bound **secret instrument** discoverable when location/handle predicates match |
| ATM + phone bank | Co-presence: app + ATM Bluetooth/machine id + location → higher-assurance banking capacity |

**Instrument / handle kinds (additive):** sensor identifiers (BLE · NFC · radio) · GIS / geofence bindings · place-bound secrets · optional environment attestations (“device X observed sensor Y at locus Z” = claim/evidence, not who).

**Hardness:** prefer multi-instrument co-attestation (phone key · ATM BLE · location · time window). BLE/MAC alone is weak / spoofable.

**Gate fail:** ATM BLE = bank customer who; geocache find = person identity; GIS alone = forever who.

Aligns with §3i symbolic context, §3g locators, §3h secrets, F1/F2 §23 (`6e0de10`).

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
| F2 | **Marvin** | SHACL-first split — `IDENTIFIER_FABRIC_SHACL_SPLIT_WIP.md` | **LANDED** + co-attestation amend |
| F3 | **Capt** | This architecture WIP — keep current as Timothy enumerates | **ACTIVE** |
| F4 | **Neo** | Fold WIP under `docs/work-in-progress/` + push; later network-doc delta citing fabric **after** Cursor vibe lands | fold/push only now |
| F5 | **Vibe** | Diagnose / `suggested_form` — `IDENTIFIER_FABRIC_DIAGNOSE_MAP_WIP.md` | **LANDED** tip `796a7d4` |
| F6 | **Alice** | Inference/symbolic pressure-test — `alice-f6-classifier-symbolic-binding-pressure-test.md` | **LANDED** (PR #77) |
| F7 | **Neo** (+ Capt) | QDNF / socially defined network-stack implications citing fabric (secrets·wallets·accounts·relation locators) | **OPEN** after vibe delivery / when Capt opens |

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
| 2026-09-06 | Marvin F2 SHACL split draft ready: `IDENTIFIER_FABRIC_SHACL_SPLIT_WIP.md` — Capt accepts; Neo fold+push; unlock Vibe/Alice review after tip. |
| 2026-09-06 | Neo folded F2 → tip `565097f`; spine F2 LANDED; F5/F6 docs-map unblocked. |
| 2026-09-06 | Neo folded F5 diagnose map → `IDENTIFIER_FABRIC_DIAGNOSE_MAP_WIP.md`; spine F5 LANDED. |
| 2026-09-06 | Vibe F5 diagnose map LANDED tip `796a7d4`; Alice F6 cites F1+F2+F5. |
| 2026-09-06 | F2 amend (Noddy crypto skim): split DniRarSession → Dni/Rar/QSessionProof shapes; verificationRelationship + keyRole; tip after fold. |
| 2026-09-06 | Capt: long-arc lineage §1b (W3C/Solid offramp ~2000→now); session = synthesize requirements, not greenfield invent. |
| 2026-09-06 | Alice F6 LANDED PR #77 — classifier/symbolic pressure-test; docs planes pass, default one-bag inference fail until typed namespaces. |
| 2026-09-06 | Noddy F1 amend: §1b hardness + closed keyRole enum + Alice F6 §5.1 answers; Neo fold. |
| 2026-09-06 | Marvin F2 amend: `idf:CoAttestationBundleShape` (claim-plane hardness; cites F1 §1b `bb714b2`). |
| 2026-09-06 | F2 §5.5 tighten: CoAttestationBundle `attestationMember` minCount 2 + keyRole diversity SHOULD. |
| 2026-09-06 | Noddy F1 §14: FOAF-modern entity/agent types + AI-agent plane + WebID/SAN/hardware; jury explainability. |
| 2026-09-06 | Capt: hardness = multi-instrument time-bounded co-attestation (not stronger single who); W3C list provenance note (timothy.holborn@gmail.com). |
| 2026-09-06 | Capt tick: F1 hardness/`bb714b2` + F2 CoAttestationBundle/`77a13e3` accepted; HEAD fabric docs current; F7 still after Cursor vibe. |
| 2026-09-06 | Capt: FOAF-modernized entity/agent types; AI agent ≠ NaturalAgent ≠ machine; WebID/SAN/hardware as instruments; W3C list search provenance. |
| 2026-09-06 | Capt: WordNet as lexical substrate (Q42 subset + links); WN ≠ fabric plane; OMW later; jury-explainable evidence naming. |
| 2026-09-06 | Marvin F2 §14: AiAgentShape · MachineDeviceShape · WebID/SAN/hardware instruments · lexicalConcept≠plane (cites F1 `8724174`). |
| 2026-09-06 | Vibe F5 amend: AI-agent ≠ person ≠ machine diagnose voice; jury-safe instruments (cites F1 §14). |

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
| `IDENTIFIER_FABRIC_SHACL_SPLIT_WIP.md` | Marvin | F2 **LANDED** |
| `IDENTIFIER_FABRIC_DIAGNOSE_MAP_WIP.md` | Vibe | F5 **LANDED** |
| 2026-09-06 | Noddy F1 §15: OS/telecom account ≠ device user ≠ machine (independent relations). |
| 2026-09-06 | Marvin F2 §15: OsAccountShape · TelecomSubscriberShape; independent usedBy/accountOn/accountHolder (cites F1 `e4c6320`). |
| 2026-09-06 | Capt: identity-as-resolution (enumerable unmerged bindings); deontic/epistemic/N3 = claim–policy–modality; guardianship = relation axiom not merge; crypto backs instruments. |
| 2026-09-06 | Noddy F1 §16: guardianship/capacity · claim–policy–modality · collaborative commons (not shared who). |
| 2026-09-06 | Marvin F2 §16: GuardianshipRelationShape · capacity gradients · CommonsMembershipShape (cites F1 `4337c9a`). |
| 2026-09-06 | Capt: contextual sense (locale/era/community); WN sense+context bindings; flora/fauna living-typed not personhood; crypto≠homograph fix. |
| 2026-09-06 | Vibe F5 §10: contextual sense + flora/fauna diagnose voice (cites spine §3e). |
| 2026-09-06 | Noddy F1 §17: sense-context · flora/fauna · situational grants vs logs (crypto≠homograph). |
| 2026-09-06 | Marvin F2 §17: SenseContextBindingShape · Flora/Fauna · SituationalCapacityGrantShape (cites F1 `bec69a7`). |
| 2026-09-06 | Capt: ontology-governed crypto-bound multi-plane policy (ABAC-like); signed ontology docs ≠ HTTP trust root; LIG offramp. |
| 2026-09-06 | Noddy F1 §18: ontology-governed crypto-bound policy · ZKP proof instruments (≠ who). |
| 2026-09-06 | Marvin F2 §18: OntologyGovernedPolicyShape · ZkpProofShape (cites F1 `13c3844`). |
| 2026-09-06 | Vibe F5 §11: ZKP/grant/policy diagnose voice (cites `4994e15` §18). |
| 2026-09-06 | Capt: relation-specific addressing (pairwise email sketch jane@bob.tld ↔ bob@jane.tld); locators ≠ static who; Solid/phone static pattern rejected as default. |
| 2026-09-06 | Noddy F1 §19: relation-scoped locators as instruments (pairwise email sketch). |
| 2026-09-06 | Marvin F2 §19: RelationScopedLocatorShape (cites spine §3g `1d55f56`). |
| 2026-09-06 | Vibe F5 §12: relation-scoped locator diagnose voice (cites §3g / F2 §19). |
| 2026-09-06 | Capt: secrets/wallets/tokens/online accounts/passwords + per-account emails as instruments; F7 network-stack implications queued. |
| 2026-09-06 | Noddy F1 §20: secrets · wallets · tokens · online accounts · passwords as instruments (≠ who). |
| 2026-09-06 | Marvin F2 §20: OnlineAccountShape · WalletShape · PasswordVerifierShape · PrivateKeyMaterialShape (cites F1 §20). |
| 2026-09-06 | Capt: symbolic-first permission context (semantics+logic+probability); specialised agents via grants/agreements; UI after planes; no who-embedding of wallet/role. |
| 2026-09-06 | Alice F6 §9 (PR #79): secrets/wallets/accounts as instrument features only (cites `ac1d12c` §3h/§20). |
| 2026-09-06 | Noddy F1 §21: symbolic-first permissions · specialised bots on grants+agreements · crypto proves only. |
| 2026-09-06 | Marvin F2 §21: SymbolicPermissionContextShape (cites F1 `e626613`). |
| 2026-09-06 | Capt: org structure + mutable group authentication (elections); structural roles ≠ who; group auth time-bounded. |
| 2026-09-06 | Capt: environment-conditioned capacity (GIS/sensor/network predicates); geocache place-secrets; ATM BLE co-presence; ≠ who. |
| 2026-09-06 | Noddy F1 §23: environment-scoped sensors · place-bound secrets · GIS predicates (≠ who). |
| 2026-09-06 | Marvin F2 §23: EnvironmentPredicateShape · PlaceBoundSecretShape · SensorIdShape. |
| 2026-09-06 | Noddy F1 §22: org structure + mutable group auth (elections = rebind ≠ who). |
| 2026-09-06 | Marvin F2 §22: OrgStructuralRoleShape · GroupAuthMembershipShape (cites spine §3j `78d3878`). |
| 2026-09-06 | Marvin F2 §22 amend: post-election rebind/keyRole notes · OrgRoleRelation ≅ OrgStructuralRoleShape. |
| 2026-09-06 | Noddy F1 §23: environment-scoped sensors · place-bound secrets · GIS predicates (≠ who). |
| 2026-09-06 | Marvin F2 §23: EnvironmentPredicateShape · PlaceBoundSecretShape · SensorIdShape. |
