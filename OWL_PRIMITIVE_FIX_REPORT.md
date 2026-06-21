# Fix Report — Human-Centric Primitive: OWL → RDFS + SHACL

**Date:** 2026-06-21 · **Author:** Qualia (agent), at Timothy Holborn's direction
**Scope:** Correct the ontological primitive so a natural person is **never** reduced to a `thing`.
**Status:** ✅ Implemented and verified (5/5 targeted tests pass, including 2 new regression guards).

---

## 1. The problem (and how it was found)

Noticed by **manual code inspection** — not by any test or tool. A prior automated agent had grounded the
native human-centric classes in **OWL** (`owl:Class`) rather than **RDFS + SHACL**.

Why that is a real defect, not cosmetics: in OWL semantics, **`owl:Thing` is the universal superclass** —
every `owl:Class` individual is implicitly an `owl:Thing`. So declaring `q42:Principal a owl:Class`
silently makes a human being an `owl:Thing`. That reduction is precisely what this project exists to
refuse (the foundational-ontology chain: a person is the seat of selfhood/personhood/custody/agency, not
a member of a universal "Thing" class). The existing `sh:not owl:Thing` guard was patching, at
validation time, an implication the OWL grounding itself created.

**Important nuance preserved:** there was **no** place that literally typed a *person* as `owl:Thing` in
a data assertion. The genuine bug was the **metamodel choice** (`owl:Class` / `owl:ObjectProperty` for
native human-centric terms). Legitimate `owl:Thing` references were **kept**:
- the `sh:not owl:Thing` / `sh:not q42:Thing` **guards** (these are the protection — they reject any
  attempt, including from imported OWL data, to type a Principal as a thing);
- the **HCAIO standard drafts** (`docs/.../HCAIO/`), which correctly *critique* `owl:Thing` and use
  `hcai:TangibleEntity ⊑ owl:Thing` for **assets/accounts** (things), never for persons;
- RadLex anatomy in `owl.rs` (`radlex:AnatomicalEntity a owl:Class ⊑ q42:Thing`) — anatomy is a
  clinical *thing a Principal has findings about*, and RadLex is an external OWL input vocabulary the
  module lowers to SHACL.

---

## 2. What changed

### 2.1 `crates/qualia-core-db/shapes/qualia-agency.shacl.ttl` — re-grounded the primitive
- `q42:Principal` : `a owl:Class` → **`a rdfs:Class`**; added `rdfs:seeAlso hcai:NaturalPerson` and an
  explicit comment that it is **not** a subclass of `owl:Thing`/`q42:Thing` and not reducible to either.
- `q42:Thing`, `q42:ClinicalEntity`, `q42:ImagingEntity` : `a owl:Class` → **`a rdfs:Class`**
  (`q42:Thing` remains the class for *possessions*; a Principal must never be typed as one).
- Possession relations (`q42:hasCondition`, `hasFinding`, `hasImagingStudy`, `hasDicomSeries`) :
  `a owl:ObjectProperty` → **`a rdf:Property`** (RDFS-native), `rdfs:domain`/`rdfs:range` unchanged.
- Added a header rationale block; added `@prefix rdf:` and `@prefix hcai:`.
- **Kept** both SHACL gates (`sh:not q42:Thing`, `sh:not owl:Thing`) verbatim — defense-in-depth against
  legacy/external OWL typing.

### 2.2 `crates/qualia-core-db/src/q42_lexicon.rs` — SHACL is now a first-class format prefix
The `.q42` lexicon previously registered `owl` and `rdfs` but **not** SHACL — the very layer that
enforces the human-centric primitive could not even be named in the format. Both constructors
(`Q42Context::new`, `Q42Lexicon::new`) now register:
- `sh`  → `http://www.w3.org/ns/shacl#`
- `hcai` → `http://www.w3.org/ns/hcai#`
…with a comment recording that RDFS+SHACL are the native modelling layer and `owl` is **input-only**
(imported OWL vocabularies are lowered to SHACL, never used as the metamodel for persons).

### 2.3 `crates/qualia-core-db/src/modalities/logic/owl.rs` — regression guards (so it can't silently revert)
Two new tests lock the invariant a careless agent broke once:
- `principal_is_rdfs_grounded_not_owl_thing` — asserts `q42:Principal a rdfs:Class`, asserts the file
  does **not** contain `q42:Principal a owl:Class` or native `owl:ObjectProperty`, and asserts the
  `sh:not owl:Thing` dignity guard is still present.
- `q42_lexicon_registers_shacl_prefix` — asserts the `.q42` context registers `sh:` and `hcai:`.

### 2.4 Incidental (enabled the native test, kept) — `crates/qualia-cli/src/main.rs`
Fixed a pre-existing clap defect: a global `verbose: u8` collided with two subcommands' local
`verbose: bool`, panicking `llm test` / `llm comprehensive-test`. Gave the local args explicit `id`s.

---

## 3. Verification

```
cargo test -p qualia-core-db --lib modalities::logic::owl
  running 5 tests
  test ...::healthcare_owl_parses_and_emits_ie_shapes ... ok   # OWL→SHACL lowering unaffected
  test ...::radlex_xml_parses_part_of_relations       ... ok   # RadLex lowering unaffected
  test ...::agency_shape_file_exists                  ... ok   # existing guard still holds
  test ...::principal_is_rdfs_grounded_not_owl_thing  ... ok   # NEW — primitive is RDFS-grounded
  test ...::q42_lexicon_registers_shacl_prefix        ... ok   # NEW — SHACL/hcai registered
  test result: ok. 5 passed; 0 failed
```

---

## 4. What this does and does not guarantee

- **Does:** the native Qualia vocabulary now grounds the human-centric primitive in **RDFS classes +
  SHACL shapes**; SHACL and HCAI are nameable in the `.q42` format; the dignity guards remain; a
  regression test prevents silent reversion. The *modelling primitive is no longer `owl:Thing`.*
- **Does not (yet):** wire the `q42:PrincipalShape` SHACL gate into the **runtime ingest/orchestrate
  enforcement path**. The shape is declarative and tested, and the VM has the opcodes to enforce it
  (`CheckNotShape`), but `orchestrate_inference` does not yet *run* the agency shape on typed data.
  That is tracked as the follow-up below — it converts the guard from "declared + tested" to "enforced
  at the silicon/VM level," which is the STELLAR §2B deontic-gate direction.

### Recommended follow-up (separate task)
Register a `qualia-agency` extension in `shacl_extension_bridge.rs` (currently a silent `_ => {}`), emit
`CheckNotShape` opcodes for Principal-⊥-Thing, and invoke it on the typing boundary in
`orchestrator.rs::orchestrate_inference` (pre-flight), writing a conduct-violation Quin to the WAL if a
Principal is ever typed as a thing — the same mechanism already used for intent denial.
