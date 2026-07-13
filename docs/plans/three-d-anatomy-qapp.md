# 3D Anatomy Qapp — implementation plan

**Status:** proposed plan for review (2026-07-03). Priority (Timothy).
**Register:** [future-work-register](future-work-register.md) (★ prioritized).
**Reuses:** the existing bundled Anatomy knowledge base + Rust backend; the **native** 3D engine.

## 0. What it is

The illustrative surface for **whole-person, systemic, accumulative wellbeing**. It maps a person's
WellFair records **and** diet / traditional natural medicine **and** lifestyle (sleep, exercise, social)
**and** environment onto a **native-rendered 3D body**, over **time**, and shows where things *converge and
compound* into system-level implications — as sourced, contestable **proposals**, never diagnoses or advice.

## 1. Two audiences, one engine

| Lens | For | Framing (hard boundary) |
|---|---|---|
| **Clinician (OSCE-Prac aid)** | doctors / medical professionals | Decision **support**: differential *considerations*, tests worth *ordering*, medicines *advised / contraindicated* (interaction-aware), specialist-review *triggers*. **Never** an autonomous diagnosis or an order. An aid for the professional's own evaluation. |
| **Person (wellbeing gist)** | everyone | A plain, simple "how am I doing" whole-body picture shaped by sleep/exercise/social/diet. **Never** advice — "worth discussing with a clinician." |

Same underlying engine + graph; the lens is a presentation + disclosure choice.

## 2. Accessibility is the core, not a mode

Simple / large / high-contrast / **plain-language by default** — it must work for someone elderly, with
poor eyesight, who does not want complexity. Advanced constructs sit **behind progressive disclosure**. The
3D body is the *simple* illustrative surface (a body, a few systems shaded by "how they're doing," tap for a
plain sentence). Reuses `wellfair/accessibility_prefs`. This is a first-class acceptance criterion, not a
later polish pass.

## 3. Honesty & safety boundaries (load-bearing)

- **Proposals, not facts.** Every computed systemic implication carries an `EpistemicStatus`
  (`Asserted` only for confirmed source records; `Hypothesis` for computed/community), provenance, and
  confidence. Contestable; a correction never deletes the original.
- **Evidence tiers**, preserved and sourced, never collapsed into "medical fact":
  `clinical-evidence > mechanistic > nutritional-data > traditional-use > community/anecdotal`.
  Traditional-medicine knowledge is honoured at its own tier, never subordinated nor overclaimed.
  **Community "hot takes"** (internet/anecdotal) enter at the lowest tier → `Hypothesis`, clearly marked
  "some people say — unverified."
- **Temporal projection is coarse + honest:** direction, relative magnitude, "hours vs days" bands with
  explicit uncertainty. **No operational safety thresholds** — never a BAC number or a fitness-to-drive/
  operate claim.
- **Offline-first, consent, sensitivity propagation.** Knowledge sources are *imported* (content-addressed,
  provenance-tagged, versioned), never live cloud calls / silent egress. Restricted/Classified health data
  never leaks into an unrestricted overlay.

## 4. The model (the stable core — build first)

- **`Factor`** — the general thing that maps onto the body. Kinds: pathology finding · condition ·
  medication · food · herb · tea · whole-food · nutrient · supplement · lifestyle (sleep/exercise/social) ·
  environmental exposure. Each: target structures/systems, **effect** (adverse / supportive / modulating),
  **evidence tier**, **sourced provenance**.
- **`FactorEvent`** (temporal) — a factor applied at a time, with quantity/dose and **kinetics**
  (onset → accumulation → clearance/recovery).
- **Environmental modulators** — temperature/season/activity scale kinetics + baseline (entered/imported).
- **State integration over time** — per-system state variable(s) integrated from factor events; recovery
  trajectory; **interventions** (water, electrolytes, rest, time) bend the curve. *Different subsystems
  recover on different clocks / respond to different interventions* — illustrating that is the core value.
- **Accumulation + interaction** — per-structure/system burden at time T; **convergence** (many factors on
  one system) and **interactions** (herb–drug, food–condition) → systemic-implication proposals.

## 5. Reuse (grounded) / retire

**Reuse:**
- **Knowledge base** `bundled/qapps/Anatomy/Knowledge/` — `condition-map.json` (condition→primarySystem +
  IRI), `system-map.json` (17 systems ↔ organs ↔ IRIs), `conditions.ttl/.n3`, `biomarkers.n3`, `rules.n3`,
  `dicom-organ-map.json`, SHACL shapes (radlex-anatomy, dicom-healthcare). Extend into the general factor
  model; the placeholder `qualia.anatomy.example` IRIs get canonicalised.
- **Rust backend** `qualia-client-core/src/anatomy_context.rs` — condition→system + daemon graph query +
  DICOM overlay spec. **Clinician lens** builds on `qualia-core-db/src/medical/comorbidity_eval.rs`
  (defeasible `exacerbates`, compounded-risk) + `clinical_engine.rs` (clinical scores, drug checks, FHIR).
- **WellFair records + epistemic model** — `wellfair/api.rs` (`list_health_records`/`list_journal_by_kind`;
  kinds: condition, clinical_report, medication, med_administration, diet, sleep, wellbeing_*, assessment),
  `wellfare-core/record.rs` (`EpistemicStatus`, `EvidenceType`, `SensitivityClass`). **Temporal precedent:**
  `sleep_analytics` already does accumulative debt/recovery.
- **Native 3D engine** — `webizen-render` `RenderScene` (per-node id **picking** + per-node CSS **color** +
  `temporal_slice` + `EpistemicState` filter), `VolumetricRenderer::upload_mesh_colored` (per-vertex RGBA),
  `Tensor10DProjection.t` (time axis), CPU `compute_pick_positions`. GLB→Q42 compiler
  `render/assets.rs` (`import_glb`, `mesh_to_nquins` — parity fixed 2026-07-03).

**Retire / replace:**
- **Babylon.js** in `bundled/qapps/Anatomy/` (Timothy: native 3D engine now).
- The desktop `webizen-desktop/commands/glb_ingest.rs` `Tensor10DMapping` prototype → thin client of the
  canonical core importer (visual-plan Phase 9). The `SemanticExtractor` (reads `extras.fma_id/snomed_id`)
  is worth keeping as a per-component semantic-id source.

## 6. Slices (substance-first; each tested + locally committed)

- **S1 — Core factor + system model + accumulation** (`wellfare-core/src/anatomy/`): `Factor`, `Effect`,
  `EvidenceTier`, the 17-system taxonomy (from `system-map.json`) + structure↔system, and non-temporal
  accumulation → per-system burden + convergence + interaction flags → systemic-implication proposals with
  `EpistemicStatus`. Pure Rust, tested. **← start here.**
- **S2 — Temporal-state engine:** `FactorEvent` kinetics, environmental modulators, per-system state
  integration over time, recovery trajectories, interventions (the beer/heat/water example). Tested.
- **S3 — Knowledge base + import:** general factor knowledge (food/herb/nutrient/lifestyle →
  system/effect/evidence/kinetics) as content-addressed, provenance-tagged, versioned records + import
  adapters + an honest seed set. ⚑ Timothy supplies the corpus + trusted sources.
- **S4 — Host API + bridges + both lenses:** WellFair records → factors; person-lens wellbeing gist;
  clinician-lens OSCE-Prac support (differentials/tests/med-flags/specialist as *considerations*, built on
  `comorbidity_eval`/`clinical_engine`). Tauri commands.
- **S5 — Native 3D presentation:** per-organ meshes as coloured, pickable `RenderScene` nodes; colour-by-
  burden; pick → contributing factors + "why highlighted" trace; **temporal scrub** via `temporal_slice`;
  simple/advanced disclosure. Retire Babylon + the desktop prototype. ⚑ needs the GLB assets.
- **S6 — Accessibility + governance + wiring:** plain-language simple-default pass, consent/provenance
  displays, sensitivity propagation, Studio "Anatomy" area/panel + nav.

## 7. ⚑ Curation datums only Timothy can supply

1. **Anatomy GLB meshes** — not in the repo (the CCF/HRA VH-Male reference library, CC-BY; the hardcoded
   legacy path `C:\Projects\qualiaDB\local\ccf-3d-reference-object-library-main` is absent). The 3D bodies.
2. **Knowledge corpus** — the authoritative food/herb/nutrient → structure/effect/evidence mappings, and
   **which sources you trust** (nutrition DBs, traditional-medicine references). Point me at one or two and
   I'll shape the import schema + seed against something real.
3. **Clinician-lens rule sign-off** — which OSCE-Prac considerations / tests / medication flags are
   appropriate to surface (this is clinical content; it needs your/clinical judgement, not the agent's).
