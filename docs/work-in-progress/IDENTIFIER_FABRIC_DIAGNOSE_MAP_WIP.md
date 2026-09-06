# WIP — Identifier Fabric diagnose / suggested_form map (F5)

**Status:** work-in-progress · **Not standards** · **Branch:** `0.0.36-dev`  
**Owner:** Vibe (language · DevRel · diagnose) · **Taxonomy:** Noddy · **Shapes:** Marvin · **Fold/push:** Neo · **Ops:** Capt.  
**Against:** tip `565097f`+ (F1+F2) · spine `IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md`  
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

*End F5 — Vibe Identifier Fabric diagnose map.*
