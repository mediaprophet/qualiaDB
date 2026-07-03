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

---

## 2026-07-03 — S4a: two-lens view model + record→factor bridge (domain half) — **done**

**What was built.** Grounded first with an Explore pass over the actual integration surface (host API
`WebizenHostApi`, `list_journal_by_kind`, `JournalEntry` with its `summary` JSON projection, `record.rs`
epistemic model, `anatomy_context.rs`, `comorbidity_eval`/`clinical_engine`, and the Tauri+Studio wiring
pattern) — all three key assumptions verified (conditions carry a `label`, records carry
`EpistemicStatus`, there's a clean derived-read-method pattern). Then built the pure, testable domain half
of S4 in `wellfare-core/anatomy`:

- **`lens.rs`** — `build_view(factors, Lens, threshold) -> AnatomyView`. **Person lens:** every loaded
  system, plain-language headline built from a new accessibility-first `plain_label` on each `BodySystem`
  ("digestion", "kidneys and fluid balance", …), coarse `WellbeingLevel` (Settled/WorthWatching/UnderStrain),
  detail behind progressive disclosure, boundary "not medical advice … worth discussing with a clinician."
  **Clinician lens:** only convergence-flagged systems, with converging factors + interactions + dominant
  evidence tier as *structural considerations* ("Consider whether this pattern warrants review") and an
  explicit "clinical specifics are the clinician's own judgement" — it does **not** invent tests/meds/referrals
  (that's the sign-off datum). Both carry `EpistemicStatus::Hypothesis` and an uncertainty note.
- **`bridge.rs`** — `RecordRef` (normalized id/kind/label/at_minute/dose) → factors via the knowledge base.
  `records_to_factors`, `records_to_timeline`, and one-shot `build_view_from_records`; kind-appropriate key
  candidates (a `diet` log fans out across `food:`/`whole-food:`/`herb:`/`tea:`/`supplement:`/`nutrient:`).
  Unmapped records are **reported, not dropped**.
- Hoisted `slugify` to the anatomy module root (shared by the importer + bridge).

**Measured results.** 31 anatomy tests passing (`cargo test -p wellfare-core anatomy::` → ok); wellfare-core
lib green, no anatomy warnings. Covers: person-lens plain/non-diagnostic/worst-first, clinician-lens
structural-considerations + herb–drug surfacing + "no invented clinical specifics", gentle empty summary,
record resolution + unmapped reporting, temporal bridge, end-to-end person view from records.

**Known limitation (honest).** The bridge resolves a record by slugging **its own label** to a knowledge
key (`"Beer"` → `food:beer`). A record labelled differently from the entry (`"Beer (alcohol)"` →
`beer-alcohol`) is honestly reported as *unmapped* rather than fuzzy-matched — a documented test asserts
exactly this. Real-world resolution (aliases / synonyms / a fuzzier match) is a **corpus-design concern**:
Timothy's corpus should define keys that match how records are labelled, or carry aliases. Flagged as an
S4/S3 follow-up, not papered over.

**⚑ Where I need the human.** (a) The knowledge **corpus** (S3 datum) is what makes medications, diet, herbs
actually map — today only conditions map cleanly (via bundled `condition-map.json`). (b) **Clinician-lens
rule sign-off** if we want the clinician lens to go past structural considerations into named tests/meds.
(c) A **direction call** for S4b: wire a text/list Studio panel now so the two lenses are visible, or hold
the UI until the S5 native-3D body (the intended illustrative surface) — since a text panel is a stepping
stone you may not want.

**Next step.** S4b — the host method (`WebizenHostApi::compute_anatomy_view(lens, …)` reading records →
`RecordRef`s → `build_view_from_records`) + Tauri command; then the Studio surface, pending the direction
call above.

---

## 2026-07-03 — S4b: host method + Tauri command + Studio "Anatomy" panel — **done** (Timothy chose: visible text panel)

**Direction call.** Timothy chose "host plumbing + text panel" — wire it so both lenses are visible and
clickable this session on what maps today. Done end-to-end.

**What was built.**
- **`qualia-client-core/src/wellfair/anatomy_view.rs`** — the host bridge. Builds the host knowledge base
  from the **bundled `condition-map.json` embedded via `include_str!`** (offline, layout-independent — real
  conditions map regardless of runtime file location) **plus** the illustrative seed (food/herb/tea).
  Normalizes condition/medication/diet journal entries → `RecordRef`s (extracts the label from each entry's
  `summary` JSON — `label`/`name`/`description`; skips ceased meds) → `AnatomyViewReport { view, burdens,
  unmapped, mapped_count, total_records, disclosure }`. The `disclosure` field states plainly that food/herb/
  med mappings are illustrative-seed pending the curated corpus, so the UI never passes seed off as fact.
- **`WebizenHostApi::compute_anatomy_view(lens, threshold)`** (api.rs) — read-only; lists the three kinds
  and delegates. **Tauri command** `wellfair_compute_anatomy_view` + handler registration (desktop).
- **Studio "Anatomy" area** — new nav area; `WellfairAnatomyPanel` with a **Simple view / Clinician view**
  toggle. Simple view = plain-language one-liner per system (plain_label), a colour-coded level dot, the
  hard boundary shown prominently, advanced detail behind a **"Show detail"** progressive-disclosure toggle
  (accessibility-core). Unmapped items shown honestly in a `<details>`; mapped/total + disclosure at the
  foot. host_client mirror DTOs + `fetch_anatomy_view` (wasm) with a non-wasm stub.

**Measured results.** `anatomy_view` 5 tests passing (incl. proof the embedded condition-map maps
Hypertension→circulatory, and that real conditions map + unknowns report + ceased meds skip + both lens
boundaries). **Builds green:** `cargo check` on webizen-desktop, and webizen-studio on **both host and
wasm32** targets. Not browser-previewable in isolation (the panel needs the Tauri host for `invoke`; without
it `fetch_anatomy_view` returns "requires the Tauri desktop host") — verification is the end-to-end unit
tests + compile-green across host+wasm, stated honestly rather than a faked screenshot.

**What maps today, honestly.** Conditions map via the bundled reference (~20 conditions → primary systems).
Medications, diet and herbs only light up from the illustrative seed until the corpus lands — the "major
role" (diet / traditional medicine) is corpus-gated, and the panel says so.

**⚑ Where I need the human.** The curated **food/herb/nutrient corpus + trusted sources** (S3 datum) is now
the single highest-leverage thing — it's what turns the diet/traditional-medicine role from seed examples
into substance. Also still open: GLB meshes (S5 native 3D), clinician-lens rule sign-off (past structural
considerations).

**Next step.** Either S5 (native 3D body — needs GLB meshes) or the real corpus import (needs a source).
Both are Timothy-gated; the engine + a visible two-lens surface are in place to receive either.

---

## 2026-07-03 — S5.0: native quantized mesh geometry format (prerequisite) — **done**

**Why (Timothy's correction, verified).** Timothy noted the GLB→native conversion only produced a
semantic summary, not renderable geometry, "likely because it was done before the renderer upgrades."
Git history confirms it: `assets::mesh_to_nquins` landed in **Phase 1.3 (`65a14dd7`)**, *before* the Q42
container / 10D-manifold format (**Phase 6 `903e8000`**, `c5d6e188`, `f9be78e4`). A grep confirms there is
**no native triangle-mesh geometry format** in the render tree — the manifold is Tensor10D *points*, and
the mesh path was transient (re-import the source GLB every time). So the geometry-into-native work is a
real gap, and now buildable. Sources for the meshes: [`hubmapconsortium/ccf-3d-reference-object-library`]
(CC-BY-4.0), both `VH_Male` and `VH_Female` (biological sex of the user) — pre-processed and shipped as
**GitHub Release assets** (not repo blobs), decision by Timothy.

**What was built.** `crates/qualia-core-db/src/render/mesh_asset.rs` — the native quantized mesh buffer,
in the house container style (`tensor::buffer_export` convention: magic `"Q42M"`, `#[repr(C, align(4))]`
bytemuck header, little-endian, zero-copy).
- `MeshBufferHeader` (exactly 48 bytes): magic/version/flags, vertex+triangle counts, and the bbox
  (`min`/`max`) as the **dequantization frame** — the same bbox the 13 semantic quins already carry.
- `encode_mesh_q42(&Mesh)` / `decode_mesh_q42(&[u8])`: positions quantized to **u16 per axis within the
  bbox** (6 B/vert vs 12), triangle indices **u16 when ≤65 536 verts** (6 B/tri vs 12), fail-closed on bad
  magic / version / truncation.

**Measured results.** 5 tests passing (`cargo test -p qualia-core-db render::mesh_asset::` → ok): round-trip
within one quantization step (`extent/65535`) with exact indices, u32-index fallback past 65 k verts,
truncation/magic rejection, 48-byte header. **Size (deterministic):** a 50 k-vert / 100 k-tri organ mesh =
**1,800,000 B raw f32 geometry → 900,048 B native (exactly 2× smaller)**, visually lossless. Versus the
*source GLB* the reduction is larger and variable (we also drop normals/UVs/materials) — to be measured on a
real organ when the fetch/pre-process step lands.

**⚑ Where I need the human.** None this step. (Standing: which HRA release to target — v1.2 male has the
rich common-organ set; the food/herb corpus for the diet role; both remain open but don't block S5.0.)

**Next step.** S5.1 — render-from-native path (decode Q42 mesh → `upload_mesh_colored`, colour-by-burden,
pick→organ) + the GLB→Q42 pre-process tool that measures the real GLB→native ratio on an HRA organ, then
publishes M/F organ assets. Decimation LOD (the budget-degradation tier) is a follow-up.
