# ADR 0009: Adopt ShEx (scoped) alongside SHACL — describe vs enforce

## Status
Accepted (2026-06-22, 0.0.19) — directed by Timothy. Scoped adoption; supersedes the earlier
"considered/deferred" stance recorded in project memory `project-shex-vs-shacl-decision`.

## Context
SHACL is the project's incumbent shape/validation language and is wired into the VM / governance
path — the **SHACL firewall**, `shacl_compiler.rs` (SlgOpcode-compiled shapes), `LOGIC_MODALITY_SHAPES`,
and the SHACL extension modules. Critically, SHACL produces **violation reports**: the deontic /
curation layer needs to know *what* failed and *why*, not just pass/fail. For enforcement, SHACL is
correct and load-bearing.

There are, however, three jobs SHACL fits poorly:

1. **Recursion.** SHACL's recursion semantics are underspecified / implementation-dependent. The
   governance topology is **inherently recursive** — guardianship / delegation chains and the
   lineage / merge DAG (the "model birth record", `INGEST_PIPELINE_SPEC.md` §1, `RENDERER_DEFINITION.md`
   §12). ShEx has well-defined recursive-shape semantics.
2. **Human-readable schema contracts.** ShEx's compact syntax (**ShExC**) is well suited to
   *publishing / exchanging* the expected shape of a credential, a qapp manifest, a birth-record, or
   an instrument — a description/interface contract, complementary to SHACL's validate-and-report.
3. **Interop.** ShEx is the lingua franca of Wikidata **EntitySchemas**; adopting it eases linked-data
   interchange.

The hazard in adopting a second shape language is **drift / two sources of truth** — the same failure
mode as the duplicated renderer crates — which the over-engineering guard (`feedback-affordability-
honest-scope`) warns against.

## Decision
Adopt ShEx in a **scoped, complementary** role under one rule: **ShEx *describes*, SHACL *enforces*.**

1. **SHACL remains the sole enforcement/firewall language** on the VM / governance path (violation
   reports, deontic gating, the extension modules). **ShEx does not touch that path.** This decision
   makes **no change** to the SHACL firewall — the explicit "won't undermine SHACL" constraint.
2. **ShEx is used only for** (adopt incrementally, per concrete need — not a blanket migration):
   (a) **recursive relational** structure validation/description (guardianship & delegation chains,
   the lineage / merge DAG) where SHACL recursion is weak; (b) **compact human-readable schema
   contracts** (ShExC) for publishing/exchanging shapes; (c) **Wikidata EntitySchema interop**.
3. **One-source rule (the anti-drift guard).** Where a shape is wanted in both forms, it is derived
   from **one source** (SOURCE → GENERATED, as in the CML library-upgrade), so ShEx and SHACL cannot
   disagree. The same shape is **never** hand-maintained in both languages.

## Consequences
- **Positive:** recursion handled correctly for the relational topology; a human-readable contract
  format for interchange; Wikidata interop — three real gaps closed.
- **Positive:** SHACL's enforcement role is untouched; no regression to the firewall / governance path.
- **Neutral / cost:** a ShEx engine (ShExC parser + validator) must be added as a dependency when need
  (a/b/c) first lands; scope kept minimal by incremental adoption + the one-source rule.
- **Negative / watch:** drift risk **iff** the one-source rule is violated — treat that as the invariant
  to protect. ShEx is a **W3C Community Group report** (not a Recommendation like SHACL 2017), so its
  tooling is less mature — verify the chosen ShEx implementation before relying on it for (a).

## References
- Project memory: `project-shex-vs-shacl-decision` (this ADR supersedes its "deferred" stance);
  `principle-governance-topology-relational` (the recursive relational structures ShEx suits);
  `feedback-affordability-honest-scope` (the over-engineering / one-source guard).
- `RENDERER_DEFINITION.md` §12 + `INGEST_PIPELINE_SPEC.md` §1 (the lineage / birth-record DAG = a
  recursive structure).
- SHACL surface: `shacl_compiler.rs`, `LOGIC_MODALITY_SHAPES`, the SHACL extension modules.
- ShEx: W3C CG report; compact syntax ShExC; Wikidata EntitySchemas.
