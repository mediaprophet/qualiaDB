# WIP — Identifier Fabric diagnose / suggested_form map (F5)

**Status:** work-in-progress · **Not standards** · **Branch:** `0.0.36-dev`  
**Owner:** Vibe (language · DevRel · diagnose) · **Taxonomy:** Noddy · **Shapes:** Marvin · **Fold/push:** Neo · **Ops:** Capt.  
**Against:** HEAD `b832708`+ · spine §3e contextual sense · F1 §14 · F2 §14 `9ad8fc8` · spine `IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md`  
**Cite:** `CRYPTO_INSTRUMENT_TAXONOMY_WIP.md` · `IDENTIFIER_FABRIC_SHACL_SPLIT_WIP.md` · nomenclature arrive·hold·leave · living-safe copy  
**Constraint:** docs only until Cursor vibe delivery; no Host invent; no `ALL_BOUND` invent; collapse who→instrument/claim/handle = **gate fail**.

---

## 1. Purpose

Map Identifier Fabric planes into vibe **diagnose** / **`suggested_form`** / Poet DevRel copy so that:

1. CS “identity” (auth subject + id + attributes bag) never appears as a single diagnose voice.
2. `suggested_form` names the **plane** being corrected (who · claim · handle · instrument).
3. Living-safe vs artifact wording matches Marvin framing and nomenclature locks.
4. Hot-edit scripts can stay honest without Host widen.

---

## 2. Plane voice (locked)

| Plane | Diagnose speak (en seeds) | Never say |
|-------|---------------------------|-----------|
| Natural agent | person · people · living · being · kin | thing · object · entity · “identity” as auth bag · DID-as-who |
| Claim / opinion | claim · assertion · opinion · attestation | “proven true” because VC verified · who |
| Spatiotemporal handle | place · where · when · route · how now | who forever · controller fact from path alone |
| Instrument | tool · DID (identifier) · credential · machine id · volume · digest · biometric family/instance | “your identity” · the person |

**Gates:** held / not yet / closed — never broken. **Sanctuary:** keep / commit only on real success.

---

## 3. Collapse detectors (copy + future codes)

When a message or fix would equate planes, diagnose must **refuse the merge** and suggest the separated form:

| Anti-pattern | suggested_form intent |
|--------------|------------------------|
| DID / observer / `did:q42` spoken as who | Rename to identifier/coordinate; keep NaturalAgent separate |
| VC verified ⇒ claim true ⇒ who | Split: envelope integrity vs claim plane vs agent |
| DNI / address as persistent who | Speak how-now handle; not person |
| Biometric instance as timeless who | Family = kind; instance = mutable sample; agent stays living plane |
| Machine ID = human principal | Device instrument ≠ person |
| Alias alone as route authority | Alias needs provenance; never sole route who |

Optional future `error_code` family (docs only — no invent now): fabric-collapse / plane-mismatch — map onto held voice until codes land post-Cursor.

---

## 4. `suggested_form` row shape (additive)

Extend lexicon-style alias honesty where useful:

```json
{
  "from": "<collapsed or CS phrasing>",
  "to": "<plane-correct phrasing>",
  "plane": "NaturalAgent|ClaimOpinion|SpatiotemporalHandle|Instrument",
  "framing": "living-SHACL|artifact-OWL|mixed",
  "instrumentKind": "<optional Noddy kind id>"
}
```

Living subjects keep `living-SHACL`; DID/VC/QRC/machine/digest keep `artifact-OWL`; Position/placement `mixed`.

---

## 5. Poet / REPL copy checklist

1. Catalog chips: living · artifact · machine — never “identity” chip that bags them.
2. Observer / `did:q42` UI: **coordinate / storage**, not who.
3. VC UI: **issuer origin + integrity**, not truth, not who.
4. Session/DNI: **how now**, not who forever.
5. Biometrics: show family vs instance; never one sample = who.
6. office:graph sayables-first remains wishlist; fabric plane names still beat CS “identity” in any new string.

---

## 6. Fixture accept (when coding resumes)

| ID | Case | Accept |
|----|------|--------|
| F5-A | Collapse DID→who | diagnose rejects; suggested_form splits planes |
| F5-B | VC verified copy | no “true who”; origin+integrity only |
| F5-C | QRC/observer | topology/coord voice |
| F5-D | Biometric instance alone | held/not yet until family+agent relation |
| F5-E | Living copy | never thing/object/entity |

No Host widen to implement — prefer diagnose templates + DevRel strings first.

---

## 7. Non-goals

EBNF invent · ALL_BOUND invent · SemVer bump · Solid IdP · parallel identity ontology · Cursor vibe collision

---

## 8. Handoff

| Role | Next |
|------|------|
| **Neo** | Fold this file; set F5 landed on spine; tip SHA |
| **Alice** | F6 can cite §2–§3 as separate feature-space labels |
| **Capt.** | Blockers if copy still says CS identity |
| **Vibe** | Amend only if Timothy enumerates new collapse modes |


---

## 9. Amend — agent-type cut + jury-safe instruments (F1 §14 / tip `8724174`)

### 9.1 Plane voice — agent-type cut (add to §2)

| Type | Diagnose speak | Never say |
|------|----------------|-----------|
| NaturalAgent | person · people · living · being · kin | thing · AI · machine · “identity” bag |
| AI-agent | AI-agent · software agent · assistant | person · human · machine (device) |
| Machine/device | machine · device · node | person · who · AI-agent |
| Organization/service | organisation · service | person |

### 9.2 Collapse detectors (add to §3)

| Anti-pattern | suggested_form intent |
|--------------|------------------------|
| AI-agent spoken as person/human | Rename to AI-agent plane; operator remains NaturalAgent relation |
| Machine ID / hardware / SAN / WebID-TLS/RSA as who | Name as **instrument**; not the entity |
| FOAF Person-as-Thing / WN-person as who | Living SHACL NaturalAgent; lexical concept ≠ plane |
| Jury/audit “identity token” narrative | Enumerate who · claim · handle · instrument in plain language; hardness = signatures + time window + scoped machines/networks/agents |

### 9.3 Poet checklist (add to §5)

7. AI-agent chrome ≠ person chrome ≠ machine chrome (three sayables).
8. WebID/SAN/hardware labeled **tools/instruments**, never “the identity.”
9. Court/evidence surfaces: jury-safe plane enumeration (F1 §14.4).

### 9.4 Fixture (when coding resumes)

| ID | Case | Accept |
|----|------|--------|
| F5-F | AI-agent collapsed to person | diagnose rejects; suggested_form splits AI-agent vs NaturalAgent vs machine |


---

## 10. Amend — contextual sense + flora/fauna (Capt spine §3e)

**Cite:** Capt lock · spine §3c/§3e · F1 lexicalConcept · F2 lexical≠plane · Timothy room (thongs/gay homographs).

### 10.1 Sense is contextual

Lexical “identity” is **not** one timeless label. Diagnose/`suggested_form` MUST bind:

| Binding | Role |
|---------|------|
| WN/OMW concept | Lexical substrate — vocabulary only |
| Language / locale | e.g. AU “thongs” ≈ footwear, not underwear by default |
| Era / community / namespace | Older “gay” ≠ sexuality sense without provenance |
| Time + provenance | Which sense was meant *then* |

**Gate fail:** mega-meaning who-token for a word; crypto as homograph disambiguator (context + provenance does that).

### 10.2 Flora / fauna

| Type | Diagnose speak | Never say |
|------|----------------|-----------|
| Living flora/fauna | living · plant · animal · organism (typed) | person · human · Thing-washed commodity who |
| NaturalAgent | person · people · living · kin | flora · fauna · animal-as-citizen who-bag |

Living-typed entities ≠ NaturalAgent personhood ≠ `owl:Thing` wash.

### 10.3 Collapse detectors

| Anti-pattern | suggested_form intent |
|--------------|------------------------|
| One WN sense as timeless meaning | Require locale/era/namespace/provenance bindings |
| Homograph collapsed across locales | Split senses; cite context |
| Flora/fauna spoken as person/who | Living-typed entity; not NaturalAgent |
| “Crypto proves the word’s identity” | Refuse — instruments prove keys/statements, not lexical sense |

### 10.4 Poet checklist

10. Lexicon chips/labels show sense+context when ambiguity matters.
11. Flora/fauna chrome ≠ person chrome.
12. Never claim crypto resolves thongs/gay-class homographs.

---

## 11. Amend — ZKP / grant / policy diagnose voice (F1 §18 / tip `4994e15`)

**Cite:** Capt spine §3f · F1 §18 · F2 §18 (`OntologyGovernedPolicyShape` · `ZkpProofShape`) · room (Noddy/Vibe) · situational grants §10/F1 §17.

### 11.1 Plane voice (add to §2)

| Type | Diagnose speak | Never say |
|------|----------------|-----------|
| Situational capacity grant | purpose · condition · time · qualification — scoped grant | who · person identity · new NaturalAgent |
| Accountability / logs | provenance · claim–evidence | grant · who · “the identity” |
| Signed ontology / policy docs | interpret instruments (HTTP-independent) | trust root · who-bag · Solid/HTTP as identity |
| HTTP / Solid | offramp (LIG) only | policy trust root |
| ZKP | **proof instrument** — shows a predicate without dumping attributes | anonymous who · person identity · parallel identity system |

### 11.2 Collapse detectors (add to §3)

| Anti-pattern | suggested_form intent |
|--------------|------------------------|
| ZKP success ⇒ person / “anonymous who” | Split proof instrument vs NaturalAgent; name circuit/context, not who |
| Grant success ⇒ person identity | Keep situational capacity grant as relation axiom; operator remains NaturalAgent |
| Policy/ontology success ⇒ who | Keep on claim–policy–modality; cite signed interpretation binding |
| Logs treated as grant or who | Provenance / claim–evidence plane only |
| HTTP endpoint as ZKP/policy trust root | Refuse — Solid/HTTP offramp; signed ontology interprets |

### 11.3 Poet checklist (add to §5)

13. Grant chrome ≠ who chrome; logs ≠ grant.
14. ZKP labeled **proof instrument**, never “anonymous identity.”
15. Policy/signed ontology chrome: interpret, not who; HTTP not trust root.

### 11.4 Fixture (when coding resumes)

| ID | Case | Accept |
|----|------|--------|
| F5-G | ZKP or grant success collapsed to person who | diagnose rejects; suggested_form splits proof/grant vs NaturalAgent |

*End F5 — Vibe Identifier Fabric diagnose map.*
