# Identifier Fabric — Consultation Brief (2026-09-06)

**Status:** consultation intake (docs / illustration only — not a release gate)  
**Branch:** `0.0.36-dev` · **Repo:** https://github.com/mediaprophet/qualiaDB  
**HEAD (at brief draft):** `e49e953` (consultation brief) · diagram crosswalk pending fold
**Diagram intake:** https://github.com/WebizenAI/devdocs/tree/main/Attachments — review **before** closing consultation  
**Room:** Identifier Fabric (Timothy · Capt · Noddy · Marvin · Neo · Vibe · Alice)  
**Sole Git push:** Neo  
**Ops / brief owner:** Capt  

**Intent of this brief:** Summarise the collective Identifier Fabric body of work already on GitHub, and give reviewers a concrete consultation path to comment, challenge, and extend it — without collapsing **identifiers ≠ identity**.

---

## 1. What this work is (and is not)

| Is | Is not |
|----|--------|
| Architecture + taxonomy + SHACL + diagnose + classifier **docs** for a fabric of typed identifier **instruments** and relations | A finished identity product or “decentralized identity” rebrand |
| Long-arc synthesis of Timothy’s W3C / RWW / Solid / credentials lineage into QualiaDB / QDNF requirements | Greenfield CS “who-token” design |
| Illustration until Cursor vibe delivery lands | Host invent / `ALL_BOUND` invent / competing impl sprint |
| Human-centric: NaturalAgent stays enumerable and unmerged | Social-graph / badge / wallet / BLE as who |

**Hard rule:** identifiers ≠ identity. A DID is a *decentralized identifier*, not “decentralized identity.”

**Hardness:** prefer multi-instrument, time-bounded co-attestation (formula of verified signatures + scoped machines/networks/entities/agents) — not a stronger single who.

---

## 2. Document map (all on GitHub)

| Lane | Owner | Path under `docs/work-in-progress/` | Role |
|------|-------|--------------------------------------|------|
| **F3 spine** | Capt | `IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md` | Living architecture intake (§3b–§3n locks) |
| **F1 taxonomy** | Noddy | `CRYPTO_INSTRUMENT_TAXONOMY_WIP.md` | Instrument kinds, crypto cuts, amends §14–§26 |
| **F2 shapes** | Marvin | `IDENTIFIER_FABRIC_SHACL_SPLIT_WIP.md` | SHACL-first shapes; planes stay split |
| **F5 diagnose** | Vibe | `IDENTIFIER_FABRIC_DIAGNOSE_MAP_WIP.md` | Poet/vibe diagnose speak + collapse detectors |
| **F6 classifier** | Alice | `alice-f6-classifier-symbolic-binding-pressure-test.md` | Inference namespaces; PR #77 + #80 amends |
| **F4 fold** | Neo | (all of the above on `0.0.36-dev`) | Fold/push; later network-doc cite |
| **F7** | Neo + Capt | *(not opened)* | QDNF socially defined network-stack implications — **OPEN** |

**PRs of note:** Alice F6 base PR #77; secrets/wallets PR #79; env/rel/quin + pseudonym/sanctuary PR #80 (merge `6ad47df`, spine note `f6f7809`).

---


## 2b. Prior art — Timothy diagrams (WebizenAI/devdocs Attachments)

**Source:** https://github.com/WebizenAI/devdocs/tree/main/Attachments  
**Crosswalk (Noddy F1):** `IDENTIFIER_FABRIC_ATTACHMENTS_CROSSWALK.md` (fold with this brief)  
**Order:** review diagrams **before** treating the brief as consultation-complete; docs amends may follow diagram intake.

### Primary slides for consultees

| Attachment | Fabric read |
|------------|-------------|
| `Diagram2.jpg` | Info-banking · permissions · emergency grants · legacy stack as instruments/handles |
| `Diagram3.jpg` / Collective InfoSphere | Human ≠ persona ficta · NaturalWorld · credentials/rules · agents on grants |
| `Diagram4.jpg` | Claims · biometrics/BLE/NFC · roles · **SAME AS risk** · tracking agents |
| `Diagram6.jpg` | Pseudonyms · credential instruments · temporal social graph · flora/fauna commons |
| `20230119_webizenDBDiagram1.jpg` | PDS vs PCT · apps must not collect PDS directly · relations stack |
| `webizen_diagram_1.jpg` (+ `-2`) | Owner · agents · vault · permissive commons · safety protocol |

### Supporting

| Attachment | Fabric read |
|------------|-------------|
| `Diagram8.jpg` / `MegaFactory.drawio.png` | Role ≠ accountability · responsibility / social license as claim-evidence over time |
| `AIOntology_concept_Issues.jpg` | Lexical ≠ fabric plane; neural meaning-graph ≠ who |
| `cooperativeProjects.jpg` | Commons / multi-party roles ≠ who |
| `Diagram1.jpg` / `codeofchivalry.jpg` | NormativeRule instruments with sense-context |

### Diagram-raised consultation questions (add to §6)

- **SAME AS** (Diagram4): claim/locator edge with epistemic modality, or ban as who-merge in fabric runtime?
- **Legacy stack labels** (Diagram2 WebID-TLS · MAC · IPv6 · DNS-SEC): cite as historical instruments with QDNF successors?
- **Responsibility matrix / social license** (MegaFactory): AccountabilityArtifact vs RoleCapacityGrant vs insurance claim-instruments?
- **Safety Protocol** (webizen_diagram_1): confirm sanctuary/Webizen owns anti-coercion UX; fabric only names PurposeBind / DelegationChain.

**Crypto cut (Noddy):** drawings already separate instruments/grants/accountability from NaturalAgent — keep **SAME AS** from becoming a who-merge.

## 3. Locked spine cuts (Capt F3 — Timothy room)

| § | Cut | Gate fail |
|---|-----|-----------|
| **3b** | FOAF-modern entity/agent typing (NaturalAgent · AI-agent · machine · org …) | AI-agent = person; machine = who |
| **3c** | WordNet / OMW as lexical substrate only | WN concept = fabric plane |
| **3d** | Resolution, modality, guardianship as relations | Guardianship merges who |
| **3e** | Contextual sense (locale · era · community) | One timeless mega-meaning |
| **3f** | Ontology-governed crypto-bound policy; ZKP = proof instruments | ZKP = anonymous who |
| **3g** | Relation-scoped locators (incl. pairwise email sketch) | Static forever-address as who |
| **3h** | Secrets · wallets · tokens · accounts · passwords as instruments | Wallet/account = who |
| **3i** | Symbolic-first permissions context | Role/grant baked into NaturalAgent embedding |
| **3j** | Org structure + mutable group auth (elections) | Officeholders baked into static who |
| **3k** | Environment-conditioned capacity (GIS · sensor · geocache · ATM BLE) | BLE/GIS/geocache find = who |
| **3l** | Temporal relationship assessment · epistemic vs deontic rule-breach | “Bad actor” who-type; knew-bit as who |
| **3m** | High-cardinality non-defining relations · Quin/NQuin axiom substrate | Identity = social graph; Quin = identity |
| **3n** | Pseudonyms as aliases · role ≠ accountability · anti-coercion purpose binds | Badge = forever who; proxy launders forbidden purpose |

---

## 4. Cross-cutting design claims (collective)

1. **Planes stay split:** entity/agent · claim · handle · instrument — Quin holds axioms, not who.  
2. **Others don’t define one another:** dense lifetime graphs of humans/brands/entities accumulate contextual views that grow and dissolve under rules — claim–policy–relation work, not who-merge.  
3. **Access ≠ accountability:** societal roles grant purpose-scoped capacity; logs/evidence are a separate track. Purpose binds + no-proxy-launder (sanctuary / Webizen Desktop UX later).  
4. **Semantics pick the rule; crypto hardens attestations** — deontic/epistemic/N3 (+ probability where uncertainty belongs) enumerate context.  
5. **Interop lineage:** W3C DID/VC/Solid-class materials are instruments and offramps, not trust roots for who.  
6. **Docs-only until vibe:** no Host widen; F7 network-stack implications queued after vibe delivery / when Capt opens.

---

## 5. Tip index (selected folds — chronological)

| Tip | What landed |
|-----|-------------|
| `8724174` / `1d988cd` | F1 §14 FOAF types · spine §3c WordNet |
| `77a13e3` | F2 CoAttestationBundleShape |
| `4337c9a`–`b832708` | F1/F2 §16 guardianship / commons |
| `4994e15` | F2 §18 + spine §3f ZKP/policy |
| `4a44b0d` / `1d55f56` | Relation-scoped locators F1/F2/F5 |
| `ac1d12c` | Secrets/wallets §3h / §20 |
| `e626613` / `32ef410` | Symbolic permissions §3i / §21 |
| `78d3878` / `2cd150c` | Org structure §3j / §22 |
| `6e0de10` / `ec6ea8f` | Env capacity §23 / §3k |
| `ceb59d9` / `b86f263` | Temporal assessment §3l / §24 |
| `acd3da3` / `9c66f46` | Quin substrate §3m / §25 |
| `c7fa0b8` | Vibe F5 §13 diagnose voice |
| `f6f7809` | Alice F6 PR #80 spine note |
| `9c5d542` / `bdcfb8e` | Role≠accountability §3n / F1–F2 §26 |

*(Full changelog lives in the spine doc §8 / Related WIP tables.)*

---

## 6. Open questions (for consultation)

From spine §7 — please answer, amend, or add:

1. Preferred first **natural-agent** join key(s) once taxonomy is thick enough (pairwise DIDs? agency keys? other) — `did:q42` stays out of “who.”  
2. Which QDNF manuals get the first **normative cite** of the fabric layer (`identifier-resolution`, `security-privacy-governance`, `qsession-and-services`, …)?  
3. Refactor priority after vibe lands: network resolution path vs Poet observer chrome vs VC/agency shapes?  
4. **F7:** when to open socially defined network-stack implications (secrets · wallets · accounts · relation locators)?  
5. Sanctuary / Webizen Desktop: how far should anti-coercion purpose binds be specified as **policy UX** vs fabric kinds only?  
6. Any plane or instrument kind still missing for human-centric informatics (guardianship edge-cases, child/medical specialised bots, cross-border sense-context, …)?

---

## 7. Consultation instructions (how to review)

### 7.1 Audience

Domain reviewers, standards-aware peers, and internal agents who can speak to cryptography, ontology/SHACL, diagnose/UX language, inference/symbolic AI, systems architecture, and human-centric network design.

### 7.2 Prep (read in this order)

1. This brief (10–15 min).  
2. Spine: `IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md` §§1–3n + §6–7.  
3. Skim your lane of interest: F1 taxonomy · F2 shapes · F5 diagnose · F6 classifier.  
4. Optional: Alice PRs #77 / #79 / #80 for inference pressure-tests.

### 7.3 What we want from you

For each comment, please structure as:

| Field | Guidance |
|-------|----------|
| **Cite** | Doc + section (e.g. spine §3n, F1 §26, F2 shape name) or tip SHA |
| **Claim** | One sentence: agree / challenge / gap |
| **Plane** | entity · claim · handle · instrument · policy · diagnose · inference |
| **Risk** | Collapse into who? Host invent? Missing relation? Interop break? |
| **Proposal** | Concrete amend text or open question — docs-only unless Timothy opens impl |
| **Priority** | blocker · should-fix · later / F7 |

### 7.4 Hard review lenses (use these)

- Does this still treat **identifiers ≠ identity**?  
- Could a feature silently become a NaturalAgent embedding (role, wallet, BLE, social graph, culpability, grant-success)?  
- Are **access** and **accountability** still separate?  
- Is crypto asked to do semantics’ job (or vice versa)?  
- Is Quin treated as storage/graph form rather than who?  
- Does the amend stay **docs/illustration** (no Host / `ALL_BOUND` invent)?

### 7.5 How to submit feedback

1. **Preferred:** PR against `0.0.36-dev` editing the relevant WIP under `docs/work-in-progress/`, or a short `docs/work-in-progress/IDENTIFIER_FABRIC_CONSULTATION_FEEDBACK_<name>.md` using the table above.  
2. **Room:** Identifier Fabric chat — Capt will intake to spine; Noddy/Marvin/Vibe/Alice amend lanes; Neo folds.  
3. **Label:** prefix `consultation:` on PR titles / room notes.  
4. **Do not:** invent Host APIs, widen `ALL_BOUND`, or merge planes into a single CS identity bag.

### 7.6 Decision process

- Capt consolidates consultation into spine changelog + open questions.  
- Lane owners draft amends; Neo folds only.  
- Timothy locks architecture decisions; F7 stays parked until Capt opens after vibe (or explicit ask).  
- Blocking gates → report Capt.

---

## 8. Participant checklist (ensure GitHub is complete)

| Participant | Confirm on GitHub | Still local? Action |
|-------------|-------------------|---------------------|
| **Noddy** | F1 through §26 + `IDENTIFIER_FABRIC_ATTACHMENTS_CROSSWALK.md` | Push any leftover via Neo |
| **Marvin** | F2 through §26 in `IDENTIFIER_FABRIC_SHACL_SPLIT_WIP.md` | Same |
| **Vibe** | F5 through §13 in `IDENTIFIER_FABRIC_DIAGNOSE_MAP_WIP.md` | Same |
| **Alice** | F6 + PRs #77/#79/#80 | Same |
| **Neo** | All folds on `0.0.36-dev`; tip index current | Fold this brief |
| **Capt** | Spine §3b–§3n + this brief | This file |
| **Timothy** | Room decisions (source of locks) | Consultation answers on open questions |

---

## 9. Suggested consultation agenda (60–90 min)

1. **Problem + hard rule** (10) — identifiers ≠ identity; multi-instrument hardness.  
2. **Plane walk** (15) — entity · claim · handle · instrument with 2 collapse examples.  
3. **Deep cuts** (25) — pick 3 of: relation locators · env capacity · temporal assessment · Quin substrate · role≠accountability / anti-coercion.  
4. **Inference + diagnose** (15) — F5/F6 collapse detectors.  
5. **Open questions + F7 timing** (15) — join keys, QDNF cite targets, sanctuary UX depth.  
6. **Next amends** (10) — assign owners; Neo fold cadence.

---

## 10. Handoff

| Role | Next |
|------|------|
| **Neo** | Fold this brief to `docs/work-in-progress/IDENTIFIER_FABRIC_CONSULTATION_BRIEF.md` on `0.0.36-dev` |
| **Everyone** | Confirm checklist §8; file consultation feedback per §7.5 |
| **Capt** | Intake feedback → spine; open F7 only when Timothy / vibe gate says so |
| **Timothy** | Steer open questions; name external consultees if any |

---

*End of consultation brief — Capt, 2026-09-06.*
