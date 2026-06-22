# CML Library-Upgrade Protocol — reprocessing the corpus when the logic changes

A recurring pattern: we extend the **logic systems** (engine modalities, the CML/deontic
schema, the SHACL surface) and then must **reprocess the CML over the instruments / library
files** — a *library upgrade*. This is ongoing. This doc says how it works, what's
generated vs. curated, how versioning + staleness detection work, and the one command that
does it. The driver is [`tools/reprocess_library.py`](tools/reprocess_library.py); the
canonical version is in [`SCHEMA_VERSION`](SCHEMA_VERSION); changes are logged in
[`CML_VERSIONS.md`](CML_VERSIONS.md).

---

## 1. The two layers — SOURCE vs GENERATED (the load-bearing distinction)

Upgrades are safe **only** because human work and machine output live in different layers.

| | Layer | What | Versioning |
|--|-------|------|-----------|
| 🖐 | **SOURCE** (git-versioned, durable) | `un-instruments/*.n3` (TEXT), the model (`values.n3`, `agency.n3`, `jural.n3`, …), `overlays/**` (human curation), `tools/**` | Git history is the record |
| 🤖 | **GENERATED** (reproducible from SOURCE + schema) | `concepts/*.n3` (machine-PROPOSED CML), `dist/q42/*.q42` (volumes), `docs/data/*.json` (demo), `INDEX.md` | Stamped `cml:schemaVersion` + content hash |

**The rule that makes upgrades non-destructive:** regeneration rewrites **only the
machine-PROPOSED generated layer** (everything `cml:Proposed` / `values:HeuristicDerived`).
It must **never** touch:
- **Human attestations** — anything `cml:Attested` / `skos:exactMatch` (the Curation Prime
  Directive: machine proposes `closeMatch`/`Proposed`; only a signed human attests).
- **Curation overlays** — `overlays/{curation,amended-text,sense,jurisdiction}/*.n3` (PLAN §9.1).
- **Hand-crafted concept files** — the generator's `KEEP` set (e.g.
  `duty-to-suppress-forced-labour`) and non-slug files (e.g. `iccpr-non-derogable-overlay.n3`).

The engine reads **generated ⊕ overlays**, so a human's exactMatch survives any number of
machine regenerations. *That* is why we can re-run the logic→CML pipeline freely.

---

## 2. Versioning & staleness

- **`SCHEMA_VERSION`** — a single integer bumped whenever the *generated structure* changes
  (new concept fields, new deontic opcodes/modalities the CML must reflect, a changed
  derivation rule). Read by every generator.
- Every generated concept carries `cml:schemaVersion "<N>"` (+ `cml:generatedBy`,
  `prov:wasDerivedFrom`).
- **Stale** = a generated artifact whose `cml:schemaVersion` ≠ current **or** whose source
  `.n3` changed since it was generated. `reprocess_library.py --check` reports staleness and
  exits non-zero (a CI gate) — so "logic changed but the library wasn't reprocessed" is caught.

---

## 3. The pipeline (idempotent; one command)

```
SOURCE  ──generate_cml_concepts──►  concepts/*.n3   (CML CONCEPT layer, stamped)
        ──build_q42────────────►  dist/q42/*.q42  (verified round-trip volumes)
        ──build_demo_data──────►  docs/data/*.json
        ──build_index──────────►  INDEX.md
        ──validate_core_ontologies (Rust gate)
```

```
python tools/reprocess_library.py            # full reprocess at the current SCHEMA_VERSION
python tools/reprocess_library.py --check     # report stale artifacts, write nothing (CI)
```

Idempotent: with no SOURCE or SCHEMA change, re-running yields byte-identical output.

---

## 4. How to do a library upgrade (the recurring job)

1. Land the logic/schema change (engine modality, deontic opcode, CML field, SHACL shape).
2. If the **generated structure** changed, **bump `SCHEMA_VERSION`** and add a row to
   `CML_VERSIONS.md` (what changed · what regenerates · any breaking concern).
3. Run `python tools/reprocess_library.py`.
4. Review the diff (it should be confined to the generated layer; overlays/attestations
   untouched), run the Rust gate, commit `SCHEMA_VERSION` + regenerated `concepts/` + the
   changelog together.

---

## 5. Version-control policy

- **Git-tracked:** SOURCE (always); `concepts/*.n3` (tracked so diffs are reviewable, but
  reproducible — `--check` guards against drift); `SCHEMA_VERSION`; `CML_VERSIONS.md`.
- **Git-ignored:** `dist/q42/**` and `docs/data/*.json` — pure artifacts, rebuilt on demand.
- A schema bump is one atomic commit: `SCHEMA_VERSION` + regenerated `concepts/` + changelog.
- Human curation evolves on its own cadence in `overlays/**` (or signed attestations), never
  coupled to a machine regeneration.

---

## 6. Still to wire (honest)
- `overlays/**` directory does not exist yet (designed in PLAN §9.1) — when curation begins,
  create it; the protocol already reserves it as never-regenerated.
- `--check` currently compares the concept-layer `schemaVersion`; per-source content-hash
  staleness (regenerate only the instruments whose TEXT changed) is the next refinement
  (the `sourceContentHash`/`generatorVersion` fields already exist in `tiering.n3` §10.1).
