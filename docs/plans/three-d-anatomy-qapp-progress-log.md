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
