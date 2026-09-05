# POET Specification, Design, and Implementation Reconciliation

**Status:** Work in progress  
**Date:** 2026-09-05  
**Branch reviewed:** `0.0.36-dev`  
**Reviewed tip:** `4eade061`  
**Frozen Vibe host surface:** `vibe-host-0.1` at `6dc2b8b8`

## Purpose

This document reconciles two valid bodies of work:

1. The bounded POET product-integrity, shell UX, and person-controlled Health
   programme recorded in the lower-cost playbook and sequential ledger.
2. The newer post-freeze lane plans for POET chrome, visual grammar, ontology,
   Vibe language, QualiaDB seams, toolchest work, maps, rendering, and later
   Webizen Desktop reuse.

The newer plans extend the product direction. They do not erase completed UX or
Health work, and they do not bypass the earlier safety and review gates.

## Reconciled status

| Area | Evidence-based status | Governing continuation |
|---|---|---|
| Integrity foundation | Implemented through `BASE-01` to `BASE-03e` | Preserve inventory, delegation ceiling, shared honest states, and decomposed modules |
| Shell honesty and hierarchy | Implemented through `UX-01` to `UX-04` | Davinci and Monet plans operate as delta audits over this baseline |
| Health Overview and corrections | Implemented through `HLT-01` | Preserve append-only correction and provenance behavior |
| Vitals selection and accessible table | Implemented through `HLT-02` | Preserve unit separation and non-interpretive presentation |
| Consent service contract | Implemented as `HLT-03`; review still required | Review authorization, expiry, replay, and revocation guarantees before Gate A |
| Consent/disclosure workspace | Implemented through `HLT-04` | Validate against the reviewed consent contract |
| Conditions and medicines | Implemented through `HLT-05` | Preserve active/history, provenance, and no-unsourced-claim rules |
| Health documents and reports | Implemented through `HLT-06` | Preserve text-extract path and honest binary/OCR limitations |
| Clinical calculators | Not completed in the ledger | `HLT-07`, using the higher-assurance route from the earlier playbook |
| Health completion pack | Not completed | `HLT-08`, then Review Gate A |
| Vibe host facade | Frozen and implemented at `6dc2b8b8` | No host widening; invoke live catalog IDs only |
| POET/Studio separation | Implemented by `f784d0b2` | Verify behavior remains honest across standalone, daemon, and desktop hosts |
| Graph toolchest first slice | Implemented in the G-POET-TOOLCHEST commit series | Continue inventory-first; do not invent capability IDs |
| Volume open/commit seam | Implemented in G-B-001 commit series | Preserve fail-closed wasm and denied/fault states |
| Davinci, Monet, Marvin, Neo, Vibe plan files | Documents landed | Their numbered stages remain planned unless separate implementation evidence exists |
| G-COORD **bind** | Gated | Shapes landed; live CRS invoke needs owner gate. Not a DNS/IP replacement. |
| QDNF (DNS/IP-free network) | Specified | `docs/manuals/standards/qualia-decentralized-network-fabric/` — separate from G-COORD. Current WireGuard is Transition. |
| Solid IdP | Parked | Solid is an exit adapter from QualiaDB/Webizen/Poet, not the source of identity or storage. |

## What the newer work advances

- It freezes a narrow Vibe host contract and prevents UI code from depending on
  a wide host trait.
- It gives POET, QualiaDB, ontology, visual design, and Vibe language lanes a
  common rule: use live catalog bindings or present an honest gated state.
- It adds an inventory-driven route for finishing the Tool Chest.
- It establishes Layout, Stage, and Timeline as related views of each surface.
- It makes Container, Manifold, Link, Volume, Position, and Realm shared design
  and ontology concerns instead of isolated UI inventions.
- It carries POET chrome and motion rules toward later Webizen Desktop reuse
  without starting that host work prematurely.

## Reconciliation decisions

### Completed UX is the baseline

Davinci Stages 1, 4, and 7 and Monet Stages 1, 6, and 8 overlap the earlier
`UX-01` to `UX-04` work. These stages should begin with a gap audit against the
existing implementation and browser evidence. Existing accessibility, honest
state, narrow-layout, and container-chrome behavior is not reopened unless a
specific regression or unmet acceptance criterion is identified.

### Health remains under Review Gate A

The newer plan pack does not replace the Health sequence. Complete `HLT-07` and
`HLT-08`, then review the Health data model, consent boundary, screenshots,
browser behavior, and completion claims before dataset or portable-app work.

`HLT-03` was originally routed to a higher-assurance implementation session but
was recorded as completed by another model. The implementation and seven focused
tests are useful evidence; model identity alone neither validates nor invalidates
the work. Because this is an authorization boundary, an independent contract
review remains required before Gate A closes.

### "Landed" describes documents, not completed stages

The status in `impl-plans-INDEX.md` currently means each plan file exists in the
repository. It must not be interpreted as all stages in that plan being shipped.
Stage completion needs its own commit, verification, UAT, and status evidence.

### Standalone behavior must remain semantically honest

The POET/Studio decoupling is a product improvement when standalone operations
are clearly distinguished from daemon-backed QualiaDB capabilities. Local DOM
inspection or a bounded local query must not be labelled as equivalent to a live
`GraphDatabase.sparql` result. The next Tool Chest audit should verify labels,
result provenance, disabled/gated behavior, and failure fallback semantics.

### Ontology intent and formal semantics both matter

The Marvin plan's SHACL-first treatment of persons, sacred relations, and the
living/natural world is preserved as a product and governance requirement. Its
formal wording should avoid claiming literal exclusion from `owl:Thing`, which
is OWL's universal class. The intended rule can be expressed more precisely as:
do not force persons or living/natural existence into an OWL class taxonomy;
use SHACL-first descriptions and reserve artifact-oriented OWL classification
for technical objects where appropriate.

## Outstanding verification

1. Independently review the `HLT-03` consent contract before closing Gate A.
2. Complete `HLT-07` and `HLT-08` before starting governed asset work.
3. Audit standalone Tool Chest labels and result provenance after `f784d0b2`.
4. Convert plan-index status from a single `landed` label into separate document
   and execution states when the source index is next revised.
5. Select the next Tool Chest chain through the project-owner/Captain gate.

The registration compilation gate found on 2026-09-05 was subsequently repaired
as `RM-01` by placing the unchanged router and its 16 unchanged children in a
directory-backed registration module. Product integrity passed 9/9, surface
inventory passed 1/1, and `trunk build` passed. This repair did not indicate a
regression in completed UX or Health behavior. The unrelated modified
`Cargo.lock` remains preserved.

The 3,322-line embedded POET stylesheet was then decomposed as `RM-02` into a
small Rust composition module and 14 purpose-specific CSS assets, none above
421 lines. The normalized stylesheet content hash was preserved; focused tests,
the web build, and desktop/mobile browser checks passed.

## Non-supersession

This work-in-progress reconciliation does not modify or supersede its sources.
It becomes authoritative only after project-owner review and explicit promotion.
