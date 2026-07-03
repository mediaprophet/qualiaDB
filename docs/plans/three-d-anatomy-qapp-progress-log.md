# 3D Anatomy Qapp — progress log

Honest engineering record for the [3D Anatomy Qapp](three-d-anatomy-qapp.md) build. One dated entry
per slice (project rule §9). Errors/regressions included; measurement-honest; no personal circumstances.

---

## 2026-07-03 — S1: core factor + body-system accumulation — **done**

**What was built.** `crates/wellfare-core/src/anatomy.rs` (initial monolith): the general `Factor`
model (kinds: pathology finding · condition · medication · food · herb · tea · whole-food · nutrient ·
supplement · lifestyle · environmental) mapping onto the **17 body systems** (mirrored from
`bundled/qapps/Anatomy/Knowledge/system-map.json`) with an `Effect` (adverse/supportive/modulating), an
`EvidenceTier` (community < traditional < nutritional < mechanistic < clinical), and a bounded integer
magnitude (`weight_milli`, 0..=1000 — no float health arithmetic). `accumulate` → per-system burden
(support nets against adverse, floored at baseline); `interactions` → herb–drug / compounding / opposing
pairs; `systemic_implications` → convergence-thresholded **proposals** carrying dominant evidence tier +
`EpistemicStatus::Hypothesis` (never a diagnosis). Community "hot takes" can only surface at the lowest
tier as a marked hypothesis.

**Measured results.** 7 tests passing (`cargo test -p wellfare-core anatomy`). Committed `0256e4be`.

**⚑ Where I need the human.** Curation datums for later slices (not blocking S1/S2): (1) trusted
food/herb/nutrient knowledge sources; (2) anatomy GLB meshes; (3) clinician-lens rule sign-off.

**Next step.** S2 temporal-state engine.

---

## 2026-07-03 — S2: temporal-state engine — **done**

**What was built.** Split the growing `anatomy.rs` into a library per project rule §11 —
`crates/wellfare-core/src/anatomy/`: `mod.rs` (re-exports + shared `system_key`/`push_unique`),
`systems.rs` (the 17-system taxonomy), `factor.rs` (the S1 factor model), `accumulate.rs` (S1
accumulation/interaction/implication), and the new `temporal.rs`. Each submodule owns its `#[cfg(test)]`;
the public path `crate::anatomy::*` is preserved (non-breaking).

`temporal.rs` adds the time dimension:
- **`Kinetics`** — per-(event, system) onset→clearance curve (`onset_minutes`, `half_life_minutes`;
  `0` half-life = chronic/standing). `magnitude_at` evaluates rise-to-peak then half-life decay in
  **integer milli** (linear interp inside each half-life; capped shift so it decays to 0).
- **`FactorEvent`** — a `Factor` applied at `at_minute` with a `dose_scale_pct` ("a slab" vs a can) and
  per-system kinetics overrides (so one intake clears on *different clocks* per system).
- **`EnvironmentModulator`** — heat/season scales matching contributions over a window (entered/imported,
  never a live weather call).
- **`Timeline`** — events + interventions (supportive `FactorEvent`s) + environment. `snapshot_at(minute)`
  reconstructs a slice-1 factor set at that instant, so `accumulate`/`systemic_implications` run unchanged
  on any time slice; `system_trajectory` traces one system's net burden; `burden_at` is the per-system roll.
- **`recovery_band`** — coarse honest horizon (`Hours`/`Days`/`Weeks`/`Extended`), **never** a clock time
  or a fitness-to-operate/BAC claim.

**Measured results.** 16 tests passing (8 S1 + 8 S2), `cargo test -p wellfare-core anatomy::` →
"test result: ok. 16 passed". The load-bearing `hot_week_beer_and_water_recover_on_different_clocks`
proves the design goal: with a slab of beer at t0 (hepatic slow clearance + renal fluid loss), a
rehydration intervention at t+2h, and a week-long heatwave amplifying the renal load, the **renal/urinary**
system recovers to near-baseline within **hours** (`RecoveryBand::Hours`; net < 60 milli by 12h) while the
**hepatic/digestive** system stays more loaded (not `Hours`; net at 12h > renal) — water bends the renal
curve but not alcohol clearance. These are *illustrative model numbers*, not clinical measurements.

**⚑ Where I need the human.** None blocking S3's engine work — but S3 (knowledge base) needs datum (1)
above (trusted food/herb/nutrient sources) to seed against something real rather than a placeholder; S5
(native 3D) needs datum (2) (GLB meshes).

**Next step.** S3 knowledge base + import (content-addressed, provenance-tagged, versioned factor
knowledge + import adapters + an honest seed set) — or, if preferred, S4's person-lens host API first so
the timeline is visible moving on the body sooner.

---

## 2026-07-03 — S3: knowledge base + import machinery + honest seed — **done (machinery); ⚑ corpus is Timothy's**

**What was built.** `crates/wellfare-core/src/anatomy/knowledge.rs` — the reusable factor-knowledge
library S4 will consult to turn WellFair records / diet logs into factors. Per the plan's division, the
agent builds the *machinery*; the *authoritative corpus* is Timothy's to supply.
- **`FactorKnowledge`** — a provenance-tagged, versioned template (key · kind · label · targets ·
  `Provenance` · `version` · `content_hash`). `to_factor()` instantiates a slice-1 `Factor`; `to_event()`
  instantiates a slice-2 `FactorEvent` wiring each target's default kinetics (so one intake clears on its
  own per-system clock straight out of the knowledge base).
- **Content-addressing** — `content_hash` is SHA-256 (existing `sha2`/`hex` deps, no new dependency) over
  the canonical fields; `integrity_ok()` detects any post-seal tampering.
- **`KnowledgeSource` trust ceiling** — each source declares the strongest `EvidenceTier` it may assert;
  `KnowledgeBase::insert` **caps** every target's evidence to the source ceiling, then re-seals. This is the
  structural guarantee that a community "hot take" source cannot masquerade as clinical evidence, and that
  traditional knowledge is kept at its own honest tier (neither erased nor overclaimed).
- **Import adapters (offline)** — `import_condition_map` reads the bundled `condition-map.json`
  (label-keyed → resolved to system ids via `body_system_by_label`, unresolved labels reported as warnings,
  not silently dropped); `import_entries` reads a native knowledge JSON and flags any imported hash that
  doesn't match a recompute.
- **`seed_knowledge_base()`** — a small, clearly-labelled illustrative seed exercising every tier and the
  temporal wiring (milk thistle → digestive *traditional-use*; chamomile tea → nervous *traditional-use*;
  beer → hepatic+renal *nutritional-data* with per-system kinetics; water+electrolytes → renal support; a
  deliberately over-tagged "detox tea" community claim that the cap forces down to *community-anecdotal*).
  **No fabricated citations** — sources are named reference *classes* with `citation: None`.

**Measured results.** 28 anatomy tests passing (8 S1 + 8 S2 + 6 S3 across two runs; wellfare-core lib 125
total incl. anatomy), `cargo test -p wellfare-core anatomy::` → ok, no anatomy build warnings. Tests cover:
hash seals+tamper-detection, the source-trust cap forcing a clinical-tagged community claim down to
community-anecdotal (while milk thistle keeps traditional-use), condition-map import with an unresolved-label
warning, and seed templates instantiating through both the accumulation and temporal engines.

**⚑ Where I need the human.** This is the standing S3 curation datum: the **authoritative food / herb /
nutrient → system / effect / evidence / kinetics corpus, and which sources you trust** (nutrition DBs,
traditional-medicine references). The seed is honest scaffolding, not that corpus. Point me at one or two
sources and I'll write the real import + seed against them. (Also still pending, not blocking: GLB meshes
for S5; clinician-lens rule sign-off for S4's clinician lens.)

**Next step.** S4 — host API + both lenses (WellFair records → factors via the knowledge base; person-lens
wellbeing gist; clinician-lens OSCE-Prac *considerations* on `comorbidity_eval`/`clinical_engine`). This
crosses into `qualia-client-core/wellfair` (my established lane) — I'll re-check NOTICES before touching
shared files.
