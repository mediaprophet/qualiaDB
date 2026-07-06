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
**1,800,000 B raw f32 geometry → 900,048 B native (exactly 2× smaller)**, visually lossless.

**Real measurement (the `measure_real_glb` ignored harness) on an actual HRA organ** — `VH_M_Liver.glb`
(CC-BY, VH_Male v1.2), imported via `assets::import_glb` (which handled the real production asset cleanly:
31,264 verts / 60,369 tris):

| | bytes |
|---|---|
| source GLB | 1,135,892 |
| raw f32 geometry | 1,099,596 |
| **native Q42 mesh** | **549,846** |

→ **2.07× smaller than the source GLB** (48.4%), max round-trip error **1.6e-6 model units** on a ~0.2-unit
bbox (~0.0008%, visually lossless). Before any decimation. (This GLB was geometry-dominated, so vs-GLB ≈
vs-raw-geometry; organs carrying normals/materials shrink more against the GLB.)

**⚑ Where I need the human.** None this step. (Standing: which HRA release to target — v1.2 male has the
rich common-organ set; the food/herb corpus for the diet role; both remain open but don't block S5.0.)

**Next step.** S5.1 — render-from-native path (decode Q42 mesh → `upload_mesh_colored`, colour-by-burden,
pick→organ) + the GLB→Q42 pre-process tool that measures the real GLB→native ratio on an HRA organ, then
publishes M/F organ assets. Decimation LOD (the budget-degradation tier) is a follow-up.

---

## 2026-07-05 — S5.0b: retarget compiled geometry onto the canonical `.10d` container + close the manifest→container join — **done (8/8 green)**

**Why (Timothy's steer).** Timothy: "do you remember that there's a `.10d` format also now?" The S5.0 `Q42M`
mesh buffer (`render/mesh_asset.rs`) was the pre-`.10d` interim; the CG lane's **P0.4 subsequently deleted
`mesh_asset.rs` entirely** and made the `.10d` container the one on-disk geometry format (NOTICES
2026-07-04, Devin P0.4+P0.6). So this slice retargets the compiled-geometry emission off the deleted bespoke
buffer and onto the canonical **`.10d` `QuantizedMesh` section**, and wires the two-layer q42/`.10d` model
(`docs/manuals/standards/geometry-asset-ontology.md`): q42 = semantic manifest that *cites* the `.10d` by
content hash; `.10d` = the dense compiled sidecar.

**What was built.**
- **`render/compile_10d.rs`** (NEW, wired `pub mod compile_10d;` in `render/mod.rs`):
  - `compile_mesh_to_10d(&Mesh) -> Vec<u8>` — emits a **sealed** `.10d` container holding one
    `QuantizedMesh` section (Page-aligned so the payload is GPU-stageable), whole-file CRC-32C sealed
    (self-verifying). Sizes the output by a dry-run against `&mut []` reading back
    `SectionTableError::OutputBufferTooSmall{needed}`. Deterministic (identical mesh → byte-identical
    container).
  - `compiled_digest(&[u8]) -> u32` — the container's whole-file CRC-32C = the manifest's `compiledDigest`.
  - `decode_10d_mesh(&[u8]) -> Mesh` — reads the `.10d` back (the renderer/anatomy path that avoids
    reparsing source GLB).
  - `compile_asset(bytes, hint, asset_uri, source_format) -> CompiledAsset` — the **end-to-end
    orchestrator**: `import_asset` → `compile_mesh_to_10d` → hash both layers (`compiled_digest` +
    `crc32c(source)`) → `mesh_to_nquins_with_digests`. Returns `CompiledAsset { mesh, container_10d,
    compiled_digest, source_digest, quins, lexicon }` — the whole "GLB → `.10d` + q42 manifest citing it".
  - `Compile10dError` (not `Clone` — wraps the non-`Clone` `AssetError`): `Import/Mesh/Section/
    NoMeshSection/BadHeader/SectionOutOfBounds`.
- **`render/assets.rs`** (EXTEND, my lane per NOTICES CLAIM): predicate consts `P_SOURCE_DIGEST` /
  `P_COMPILED_DIGEST` (`q_hash("urn:qualia:geometry:sourceDigest"|"compiledDigest")`) + pub
  `mesh_to_nquins_with_digests(...)` — delegates to the existing `mesh_to_nquins` then appends the two
  digest facts, so the q42 manifest structurally binds to the `.10d` it describes.
- **`shapes/geometry-asset.shacl.ttl`** (NEW) + **`docs/manuals/standards/geometry-asset-ontology.md`**
  (NEW, prior step): the normative schema + machine-checkable SHACL surface over the real
  `urn:qualia:geometry:*` facts (not a parallel vocabulary). Cross-property constraints (bbox non-inverted,
  index-in-bounds, parity, `compiledDigest == real container CRC`, sensitivity high-water-mark) are marked
  for the `geometry_asset_shacl.rs` SLG-VM shim (next).

**The consumer already exists.** `render/portal/mod.rs::load_10d` (P9.2) is the read-back/render half:
header parse → **whole-file CRC verify** → section-table walk → `QuantizedMesh` decode → Tensor10D
provenance μ → **governance fail-closed** (default-Refuse + no attestation ⇒ displayable-but-not-citable) →
GPU upload. So producer (`compile_asset`, this slice) + consumer (`load_10d`, existing) now close the
`.10d` geometry pipeline for a single mesh.

**Measured results.** `cargo test -p qualia-core-db --lib compile_10d` → **8 passed; 0 failed** (0.38 s;
3758 filtered out). The 3 new `compile_asset` tests: round-trips the container + `compiledDigest`/
`sourceDigest` fields equal the real CRCs + both digests appear as manifest facts (`compile_asset_binds_
manifest_to_its_container`), byte-identical determinism (`compile_asset_is_deterministic`), import errors
surface as `Compile10dError::Import` (`compile_asset_surfaces_import_errors`); on top of the 5 existing
(seal verifies, quantization round-trip within extent/65535, determinism + stable digest, digest changes on
geometry change, garbage rejected).

**Build-contention footnote (honest process record).** This slice sat compile-verified but *unrun* for
~30 min: the HDD fault froze six concurrent `cargo` jobs mid-build (Fable-5 inference + workspace-check
watchers + full `cargo test` integration-test links), starving my `--lib` job on the shared `target/debug`
lock (it accrued ~1.3 CPU-s over 20+ min — lock-starved, not failing). Once Timothy fixed the HDD, I cleared
the hung zombies (near-zero CPU since 04:00) and the crate compiled — with a single lib error, a closure-
lifetime bug in Fable-5's live-CLAIMed `gguf_bridge/resident_decode.rs:540` (NOT my code; the sole error in
the whole lib, which is itself the proof my modules compiled clean). Per §10 I flagged it rather than
reaching into Fable-5's lane; it was fixed, and the suite went green.

**⚑ Where I need the human.** (Standing, unchanged) the **CCF/HRA VH-Male anatomy GLB meshes** — only
Timothy can supply/point at the release. Everything above is testable on a synthetic OBJ/GLB until then;
the *end-to-end* organ body needs the real meshes. No new ask this step.

**Next step.** S5.1 — the burden→colour connective slice: per-system `SystemBurden.net_milli` (0..1000) /
`WellbeingLevel` (Settled/WorthWatching/UnderStrain, already in `wellfare-core::anatomy`) → coarse,
honesty-preserving per-organ RGBA → `gpu::upload_mesh_colored` in `load_10d`. Mesh-independent, synthetically
testable. Then the `geometry_asset_shacl.rs` shim + a `geo:bodySystem` manifest predicate so each organ
`.10d` declares which system's burden colours it.

---

## 2026-07-05 — S5.1: burden → σ → **both** visual and sonic spectrum (modality-first colour-by-load) — **done (wellfare-core 32/32, anatomy_view 7/7)**

**Why (Timothy's steer — the layer was wrong).** I had started S5.1 as "`WellbeingLevel` → hex RGBA
swatch." Timothy corrected mid-slice: *"it uses EMF which is then encoded into sonic or visual spectrum."*
That's the modality-first spine, and my hex plan flattened it — hard-coding the **output** of the visual
encoder and discarding the EMF source and the entire audio path. Grounded the correction in the actual
engine code: `render/spectral.rs` maps a scalar **σ** to a wavelength (`λ = 400 + σ·300` nm) → CIE XYZ →
linear sRGB; `render/acoustic.rs` is explicitly the *"shared σ truth for vision and AcousticPlane"* and
folds the **same** 400–700 nm band into 1760–110 Hz. σ is also the tenth axis of `Tensor10D` (the `.10d`
node atom carries `.sigma`). So σ is the one truth; colour and pitch are two encodings of it.

**What was built (corrected).**
- **`wellfare-core/src/anatomy/lens.rs`** — `WellbeingLevel::from_net` made `pub` (the canonical coarse
  classifier) + `pub fn burden_to_sigma(net_milli) -> f32`: encodes accumulated burden (0..=1000) to σ on
  the EMF band — settled → green (~550 nm, σ≈0.50) through amber to under-strain → red (~680 nm, σ≈0.93),
  the same green→amber→red hue arc as the coarse bands but as a *continuous physical quantity*. Honest by
  construction: σ derives from the transparent bounded-integer `net_milli` accumulation (no float health
  arithmetic); the person still only ever sees the coarse band — the continuous spectrum is the substrate,
  not a false-precision clinical readout. Re-exported from `anatomy/mod.rs`.
- **`qualia-client-core/src/wellfair/anatomy_view.rs`** — `SystemPercept { system_id, level, sigma, rgba,
  frequency_hz }` + `AnatomyViewReport::system_percepts()`: for each burden, encode σ **once**, then derive
  the visual colour via `qualia_core_db::render::spectral::sigma_to_linear_rgb` **and** the sonic pitch via
  `render::acoustic::sigma_to_center_frequency_hz` from that single σ. Plus `sigma_to_normalized_linear_rgba`
  (peak-channel normalize + opaque alpha → linear vertex colour for `upload_mesh_colored`). So an organ
  under strain is redder **and** lower-pitched — the same anatomy state renders to sight or sound without
  re-deciding what it means.

**Measured results.** `cargo test -p wellfare-core --lib anatomy::` → **32 passed / 0 failed** (incl.
`burden_sigma_walks_green_to_red_within_the_emf_band`). `cargo test -p qualia-client-core --lib
wellfair::anatomy_view` → **7 passed / 0 failed**, incl.: `percept_parity_strain_is_redder_and_lower_pitched`
(from one σ, strain's RGBA is red-dominant while its pitch is *below* settled's — the visual/sonic parity
proven in one assertion) and `system_percepts_cover_every_burden_and_stay_in_the_emf_band` (one percept per
burden, all σ within [0.50, 0.93], Hypertension load lands on `circulatory`). These are illustrative model
values, not clinical measurements.

**⚑ Where I need the human.** (a) Standing: the **CCF/HRA VH-Male organ GLBs** — S5.1 gives the per-system
percept, but painting it onto the *actual* body needs the organ→system binding, which is mesh-gated
(each organ `.10d` needs a `geo:bodySystem` fact; the source of that is the anatomy assets' structure→system
map). (b) A small **direction call** worth surfacing, not blocking: the Studio text-dot hexes
(`anatomy_panel.rs`, Grok's lane) are still a *separate* hand-picked readout — reconciling them so the dot
and the 3D body both derive from σ would make the whole surface one honest colour language; I left them
untouched (lane discipline) and flag it.

**Next step.** S5.2 — the organ→system binding: `geo:bodySystem` manifest predicate on each organ `.10d`
(so `load_10d` can look up the system → `system_percepts()` → colour that organ), + wiring `system_percepts`
into a `load_10d`-adjacent coloured-upload path. Then the `geometry_asset_shacl.rs` SLG-VM shim for the
cross-property manifest constraints. Both are mesh-independent to build and become end-to-end once the GLBs land.

---

## 2026-07-05 — S5.2: male/female reference models by XY/XX + organ→system binding + colour-by-load render path — **done (wellfare-core 37/37, compile_10d 9/9, anatomy_view 8/8, portal wasm-check green)**

**Why (Timothy's steer).** "get it all done … it's important to have both a male and female model. it should
be automatically associated to the user based on their DNA selection (XY or XX)." Precise and load-bearing:
the model is selected from the **chromosomal basis** — a biological-substrate attribute the user *declares*
(`XY`/`XX`), **not** a gender or identity claim (consistent with DID-is-identifier-not-identity — one
attribute, never collapsed into identity). Found the existing partial VH_Male asset registry in
`webizen-desktop/.../glb_ingest.rs` (Grok's lane), so the domain mapping belongs in `wellfare-core` (which the
desktop loader consults) — I did not refactor that file.

**What was built.**
- **`wellfare-core/src/anatomy/model.rs`** (NEW) — model-selection + organ→system domain core:
  `Karyotype { Xy, Xx }` (a closed enum, not a free string — `parse` fails closed on anything but the two
  curated values rather than guessing a body) → `anatomy_model()` → `AnatomyModel { Male, Female }` →
  `asset_set()` (`"VH_Male"`/`"VH_Female"`) + `file_infix()` (`"m"`/`"f"`). Plus `normalize_organ_key` (strips
  asset prefix / `.glb` / laterality) + `body_system_for_organ` over a curated ~45-organ → 17-system table
  (model-agnostic — the model decides which organs are *present*, e.g. `prostate` vs `uterus`, both →
  `reproductive`). Unknown organs → `None` (reported, never guessed); a test asserts every mapped id is a real
  `BodySystem`.
- **`render/assets.rs`** — `geo:bodySystem` + `geo:anatomyModel` predicates + `mesh_to_nquins_with_meta`
  (string facts via the lexicon, like `sourceFormat`); **`render/compile_10d.rs`** — `compile_organ_asset`
  (the organ mesh's q42 manifest carries which system colours it + which model it belongs to; `compile_asset`
  now delegates to it with `None`/`None`).
- **`qualia-client-core/src/wellfair/anatomy_view.rs`** — `OrganPercept { organ_key, system_id, percept }` +
  `AnatomyViewReport::paint_organs(organ_keys)`: resolve each organ's system → that system's `SystemPercept`
  (the σ→{colour, pitch} of S5.1); an organ on an unburdened system gets the **settled baseline** so the whole
  body renders; an organ not in the curated map is reported, never silently coloured.
- **`render/portal/mod.rs`** — `load_10d_colored(bytes, r,g,b,a)`: the wasm GPU paint path — decode +
  CRC-verify + governance fail-closed (as `load_10d`) then `upload_mesh_colored` with the per-organ σ-colour.
  Host flow: user's `Karyotype` → `AnatomyModel` → loader pulls that model's `asset_set()` files → each organ
  `.10d` (with its `geo:bodySystem` fact) → `paint_organs` → this coloured upload.

**Amendment (systems coverage, same day — Timothy checked the 17).** Timothy verified against the full
system list. The taxonomy already held **all 17** (his 11 major + Sensory, Vestibular, Exocrine, ECS, ENS,
Glymphatic — the seed *is* those 17). But the organ→mesh paint map only covered 12; closed the real gaps —
added **Vestibular** (inner-ear / semicircular-canal / vestibule) and **Exocrine** (salivary / parotid /
sublingual / lacrimal / sweat / sebaceous glands; liver & pancreas stay digestive-primary as dual-role). The
remaining three — **ECS, ENS, Glymphatic** — are genuinely *distributed networks* (receptors CNS-wide; the
gut's ~500M-neuron web; astrocyte+CSF brain clearance): no standalone mesh, so added `SystemRepresentation
{ DiscreteOrgans, DistributedOverlay }` + `system_representation()` marking exactly those three as overlays.
They remain first-class (burden + σ percept); they're rendered as a highlight on their host structures, not a
painted organ. A test now enforces that **every one of the 17 is accounted for** (has an organ *or* is an
explicit overlay) — no silent gap.

**Measured results.** `cargo test -p wellfare-core --lib anatomy::` → **39 passed** (7 new model tests incl.
`every_one_of_the_17_systems_is_accounted_for`); `-p qualia-core-db --lib compile_10d` → **9 passed** (incl.
`compile_organ_asset_binds_body_system_and_model`
— identical container, +2 manifest facts); `-p qualia-client-core --lib wellfair::anatomy_view` → **8 passed**
(incl. `paint_organs_colours_by_system_and_reports_unknown_organs` — burdened circulatory redder than settled
respiratory, unknown organ reported). The wasm-only portal path: `cargo check --target wasm32-unknown-unknown
--no-default-features --features portal,wasm-scientific` → **Finished** (load_10d_colored compiles clean for
the real target). Runtime GPU paint is browser + mesh gated → compile-verified, not runtime-verified (stated
honestly, not claimed green).

**⚑ Where I need the human.**
1. **Standing (now the single gate to a visible body):** the CCF/HRA **VH-Male *and* VH-Female** organ GLB
   sets (`ccf-3d-reference-object-library`, CC-BY-4.0). Every layer above is ready; the desktop registry lists
   only 5 VH_Male organs and no VH_Female.
2. A **direction confirm** (not blocking): I modelled the DNA selection as a closed `XY`/`XX` enum that *fails
   closed* on any other value rather than guessing a body — matching "XY or XX" exactly. Additional curated
   karyotypes later would be an explicit reviewed extension.

**Coordination flags (§10, not fixed by me).** (a) `webizen-desktop/.../glb_ingest.rs` (Grok) needs the
VH_Female organ list + selection by `AnatomyModel::asset_set()` from the karyotype — domain hooks provided.
(b) **Pre-existing wasm feature-gate bug in the CG lane** (Devin): `container_10d/topology_section.rs:20` +
`spatial_index_section.rs:19` `use crate::specialized_libs::…` unconditionally, but `specialized_libs` is
`#[cfg(any(not(wasm32), feature="wasm-scientific"))]` — so `--features portal` without `wasm-scientific`
fails the wasm build. Flagged, not touched.

**Next step.** The `geometry_asset_shacl.rs` SLG-VM shim (cross-property manifest constraints) — the last
mesh-independent piece. After that everything waits on the GLBs (⚑1) for the end-to-end visible + audible body.

---

## 2026-07-05 — S5.3: distributed-overlay render support + geometry-asset SHACL validation shim — **done (model 8/8, anatomy_view 9/9, geometry_asset_shacl 8/8)**

Timothy: "fix the gaps, etc. get it all done properly." Two remaining mesh-independent pieces, both closed.

**Distributed-overlay render support.** The systems-coverage work classified ECS/ENS/glymphatic as
`DistributedOverlay`, but `paint_organs` (discrete organs) silently omitted them — so a burden on those
networks had nowhere to render. Closed it: `overlay_host_systems(system_id)` in `wellfare-core/anatomy/
model.rs` (ENS→`digestive`, glymphatic→`nervous`, ECS→whole-body) + `OverlayPercept { system_id, percept,
host_systems }` and `AnatomyViewReport::overlay_percepts()` in `anatomy_view.rs`. Now `paint_organs`
(discrete) + `overlay_percepts` (distributed) together cover the whole body — **nothing that carries burden
is unrepresented**, and each overlay knows which host structures to highlight over. Tests: host hints are
real discrete systems; only ECS/ENS/glymphatic surface as overlays; a burdened glymphatic overlay still
carries a real σ colour + pitch.

**Geometry-asset SHACL shim** — `crates/qualia-core-db/src/modalities/logic/geometry_asset_shacl.rs`
(registered **ungated** in `modalities/logic/mod.rs`, so it works on all targets incl. plain wasm/portal —
unlike `specialized_libs_shacl`). The runtime half of `shapes/geometry-asset.shacl.ttl`, honestly split:
(1) `GeometryAssetConfiguration::to_opcodes()` — per-property bounds as SLG-VM opcodes (counts ≤ 2²²,
`sourceFormat`/`unit` ∈ set), the same `Configuration→to_opcodes` pattern as `specialized_libs_shacl`;
(2) `validate_geometry_manifest(facts, cfg)` — the **cross-property** checks plain SHACL cannot express
(the `.ttl` deferred these to the shim, and here they are real): bbox finite + non-inverted, every triangle
index `< vertexCount`, **`compiledDigest == the real .10d CRC`** (a manifest that lies about its container is
caught), and the **sensitivity high-water-mark** (a derived asset cannot down-classify below its most
restrictive input; unknown class ⇒ fail-closed as most restrictive).

**Measured results.** `cargo test -p wellfare-core --lib anatomy::model` → **8 passed** (adds overlay host
hints); `-p qualia-client-core --lib wellfair::anatomy_view` → **9 passed** (adds `overlay_percepts`);
`-p qualia-core-db --lib geometry_asset_shacl` → **8 passed** (well-formed passes; each of the 8 violation
classes is caught, incl. the container-lie and sensitivity-downgrade). All three crates compile green with
every change.

**Status of the whole anatomy pipeline.** Everything mesh-independent is now **built and verified**: records →
burden → σ → {colour, pitch}; XY/XX → M/F model + asset set; all **17** systems supported (14 discrete-organ,
3 distributed-overlay); organ→system + manifest facts; `compile_organ_asset`; `paint_organs` +
`overlay_percepts`; `load_10d_colored` (wasm-checked); and the geometry-asset validation shim. **The only
remaining gate is ⚑1 — the VH-Male and VH-Female organ GLB sets** (Timothy). Cross-lane follow-ups
(glb_ingest VH_Female list → Grok; the `container_10d` wasm feature-gate bug → Devin) remain flagged in
NOTICES, not touched.

**Next step.** None mesh-independent left in my lane. When the GLBs land: `compile_organ_asset` each →
`.10d` set per model → the desktop selects the set by `AnatomyModel::asset_set()` → `paint_organs` +
`overlay_percepts` → `load_10d_colored` in the browser = the visible + audible body.

---

## 2026-07-05 — S5.4: turnkey body ingestion (`compile_body`) + GLB acquisition blocker confirmed — **done (2/2); ⚑ files not on disk**

Timothy: "yup [get the GLBs in]." Built the single call that ingests a model's organ set:
`qualia-client-core/src/wellfair/anatomy_body.rs` — `compile_body(model, [(organ_key, bytes)]) ->
BodyCompileResult { organs: Vec<CompiledOrgan{organ_key, system_id, asset}>, unmapped, failed }`. For each
organ it resolves the body system (`body_system_for_organ`), derives the source format from the extension,
and compiles to a sealed `.10d` whose manifest carries `geo:bodySystem` + `geo:anatomyModel` — organs with
no system mapping or bad bytes are reported, never silently dropped. Also made `normalize_organ_key` strip
any mesh extension (`.glb`/`.gltf`/`.obj`/`.stl`), not just `.glb`. `cargo test -p qualia-client-core --lib
wellfair::anatomy_body` → **2 passed**; `wellfare-core anatomy::model` still **8 passed**. The mesh bytes are
the caller's to supply (desktop `glb_ingest` file I/O = Grok's lane); this owns the compile.

**⚑ Acquisition blocker (confirmed by inspection).** The CCF/HRA GLB library is **not on disk** — no `.glb`
anywhere under `C:\Projects`, and the path `glb_ingest.rs` defaults to
(`C:\Projects\qualiaDB\local\ccf-3d-reference-object-library-main`) does not exist. That legacy path also
conflicts with PROJECT RULE §0 (canonical repo is `C:\Projects\qualia-27062026`). So lighting up the real
body needs: (a) the actual VH-Male + VH-Female organ GLBs (source: `hubmapconsortium/ccf-3d-reference-object-library`,
CC-BY-4.0 — likely Git-LFS / CDN-served, so a naive raw fetch may return LFS pointers); (b) a decision on a
canonical **gitignored** assets location (recommend `assets/ccf/<model>/…`) and updating `glb_ingest`'s
default off the legacy path (Grok's lane). Surfaced to Timothy as the one real decision; everything to consume
the files is built and verified.

**Next step.** Timothy to point me at the GLB source / authorize a fetch + confirm the assets location; then
`compile_body` each model → visible + audible body. No further mesh-independent work remains in my lane.

---

## 2026-07-05 — S5.5: SPARQL-driven CCF asset discovery + **real organ compiled end-to-end** — **done (ccf_resolver 5/5; real liver 2.07×)**

Timothy's steer: *"you should be able to find the sparql endpoint to find them, which would be the more
scalable solution, is that correct?"* — **correct, and proven live.** The GLBs don't need a repo clone: the
HuBMAP HRA publishes them as linked data.

**Verified against the live endpoint** (`https://lod.humanatlas.io/sparql`, HTTP 200,
`application/sparql-results+json`): the reference-organ GLBs are registered as `foaf:depiction` `xsd:anyURI`
values across named graphs, pointing at `cdn.humanatlas.io` — **real binaries, no Git-LFS pointers**. One
query returns **59 distinct ref-organ GLBs, both male and female** (blood-vasculature, brain, eye, heart,
kidney, larynx, liver, lung, lymph-node, bronchus, pancreas, skin, intestines, spinal-cord, spleen, thymus,
trachea, ureter, bladder + sex-specific prostate / ovary / uterus / fallopian-tube / mammary / placenta),
each URL encoding organ + sex + version.

**What was built.** `qualia-client-core/src/wellfair/ccf_resolver.rs` (pure, no network — so unit-tested
against captured real endpoint JSON): `ref_organ_glb_query()` (the stable SPARQL query),
`parse_ref_organs(json) -> Vec<RefOrgan{filename, glb_url, model}>` (model read from the unambiguous
`-f-`/`-m-` filename infix; unsexed/malformed skipped, not guessed), `organs_for_model()`. A discovered
filename feeds straight into `body_system_for_organ` → `compile_body`. Added `serde` derives to
`Karyotype`/`AnatomyModel`/`SystemRepresentation` so the manifest is a wire DTO.

**Real end-to-end proof.** Fetched the SPARQL-returned male liver from its CDN URL (verified `glTF` magic,
1,136,412 B) and ran it through `compile_body` (new `#[ignore]`d `compile_real_ccf_organ_end_to_end`
harness, `QUALIA_TEST_GLB` env): **`3d-vh-m-liver.glb` → system=digestive · 31,264 verts / 60,369 tris ·
GLB 1,136,412 B → sealed `.10d` 549,966 B (2.07×)**, `.10d` round-trips back to the mesh. That is the whole
pipeline — SPARQL discovery → CDN fetch → `import_glb` → organ→system → `compile_organ_asset` — running on
real Human Reference Atlas data.

**Measured results.** `cargo test -p qualia-client-core --lib wellfair::ccf_resolver` → **5 passed** (incl.
discovered-filename→system); real-organ harness → **1 passed** with the numbers above.

**⚑ / coordination — what's left for the full body.**
1. **Batch binary fetch** (all N organs of a model from their CDN URLs) — qualia-client-core's async HTTP is
   **Gemini's reqwest lane (§14)**; I proved the fetch with `curl` but the in-app batch fetcher should live
   in that lane. Flag/coordinate.
2. **Asset cache location** — recommend a gitignored `assets/ccf/` (or the OS cache dir); the fetched liver
   is currently only in scratch.
3. **Architecture call (Timothy):** query the external HRA endpoint at runtime, **or** federate the
   ref-organ triples into QualiaDB's own graph and query locally (more in keeping with local-first /
   sparql-fed). The resolver is written to serve **either** (pure query+parse).
4. Desktop wiring (`glb_ingest` → use `ccf_resolver` + `AnatomyModel::asset_set()` instead of the hardcoded
   list; drop the legacy `C:\Projects\qualiaDB` default path) — **Grok's lane**, flagged.

**Next step.** With Timothy's call on (3) + the fetch lane (1): pull a full model's organ set → `compile_body`
→ `paint_organs`/`overlay_percepts` → `load_10d_colored` = the visible + audible body.

---

## 2026-07-05 — S5.6: **both full bodies compiled from the live SPARQL endpoint** (real HRA data) — **done**

Timothy freed the blocked lanes ("neither Gemini's nor Grok is running, so if jobs need to get done do
them"), so I built the fetch and ran the whole thing end-to-end on real data.

**Live fetcher** (`ccf_resolver.rs`, gated `#[cfg(not(wasm32))]`): `discover_ref_organs(endpoint)` (SPARQL
1.1 **POST** — `application/sparql-query` body; `GET ?query=` had no `.query()` on reqwest 0.13's blocking
builder) + `fetch_glb(url)` (blocking reqwest, rustls). Pure query/parse stays unit-tested; transport is the
only network part.

**Both bodies, discovered → fetched → system-resolved → `.10d`-compiled:**

| Model | Organs | Systems | Source GLB | Sealed `.10d` | Ratio |
|---|---|---|---|---|---|
| **Male** | **26 / 26** | 10 | 191,553,844 B | 97,077,174 B | 1.97× |
| **Female** | **33 / 33** | 10 | 294,101,552 B | 122,559,282 B | 2.40× |

Zero unmapped, zero failed. Female's larger set includes the sex-specific organs (uterus, ovary ×2,
fallopian-tube ×2, mammary-gland ×2, placenta) — all resolved. The 10 systems with discrete HRA meshes:
circulatory, digestive, immune_lymphatic, integumentary, nervous, reproductive, respiratory, sensory,
skeletal, urinary. (The other 7 of the 17: HRA's ref-organ set ships no discrete mesh for muscular /
endocrine / exocrine / vestibular; ecs / ens / glymphatic are the distributed overlays by design — honest,
not a pipeline gap.)

**Real-data-driven map fixes (found *because* it ran on the real set).** The first full-male run compiled
only 19/26 — the failure surfaced genuine gaps, now fixed + unit-tested: (1) `normalize_organ_key` only
stripped the `vh` provider — generalised to strip up to the `-m-`/`-f-` sex marker for **any** provider
(`allen` brain, `sbu` large-intestine, `nih` lymph-node); (2) added tokens `main-bronchus`, `mouth`,
`pelvis`, `urinary-bladder`, `placenta[-full-term]`. Re-run → 26/26 and 33/33.

**Harness.** `compile_full_body_from_sparql` (`#[ignore]`d, `QUALIA_TEST_MODEL=male|female`) — the reusable
whole-body proof. Added `discover_ref_organs`/`fetch_glb`. Verified green after changes: wellfare-core
anatomy **40**, client-core anatomy **11** (+2 ignored harnesses), ccf_resolver **5**.

**⚑ / remaining for a *visible* body.** The assets now flow all the way to sealed `.10d` + per-organ σ
percepts. What's left is the **render surface** — a WebGPU canvas that calls `load_10d_colored` per organ
with its `paint_organs` colour + the orbit camera + the M/F body assembly. That lives in the desktop/Studio
UI (Grok's lane, currently idle). Also open: the runtime-query-vs-federate architecture call, and an asset
**cache** (the harness re-downloads each run — ~200–290 MB; a gitignored `assets/ccf/` cache would make it
one-time).

**Next step.** Build the render surface (Studio WebGPU body view) so Timothy can see + hear it — or, if
preferred first, an offline asset cache + a headless whole-body percept snapshot (systems coloured by a
sample burden set) as an interim visual.

---

## 2026-07-05 — S7: fetal/embryonic developmental assets — **the Carnegie embryo series compiles from NIH 3D (6/6, real, CC-BY), first use of the `t`-axis**

Timothy (returning to the anatomy core after the terminology side-track): *"are we able to build the assets
for aging and fetal stages? … fetal stages is an important start … to demonstrate the consideration."*
**Answer: yes — proven on real, cleanly-licensed data.**

**Source found + verified.** The HRA endpoint is adult-only (its "fetal"/"embryo" hits are descriptions of
*adult* structures). The authoritative embryology atlases (Amsterdam 3D Atlas; HDBR) are **CC-BY-NC-ND /
-NC-SA** (NonCommercial + NoDerivatives/ShareAlike) and 3D-PDF — unusable for a derived `.10d` pipeline. But
**NIH 3D** (`3d.nih.gov` — the same repo that hosts the HRA adult library) publishes the **Carnegie Human
Embryo series** (author kbrowne, NIH/NIAID) as **CC-BY** GLBs: stages **12/14/16/18/20/23** (~26–56
postfertilization days, the embryonic period, weeks 4–8). Downloads go through `https://3d.nih.gov/api/files/
<fileId>` (raw S3 is `AccessDenied`); the WAF **403s requests with no User-Agent** — reqwest sends none by
default, so an explicit UA was required.

**Built.** `qualia-client-core/src/wellfair/fetal_stages.rs` — `CarnegieStage{stage, postfertilization_days,
nih3d_entry, glb_file_id}` + `carnegie_series()` (the curated CC-BY series, ordered = the `t`-axis) +
`glb_url()`. Reuses `ccf_resolver::fetch_glb` (now sends a `QualiaDB-anatomy/1.0` User-Agent — fixes the NIH
3D 403; harmless to the HRA CDN). Unit test: series monotonic in stage + gestational age (the `t`-axis).

**Measured (real, live).** `compile_fetal_series_from_nih3d` (ignored harness) fetched + compiled **all 6**
stages → sealed `.10d`, 0 failures: Carnegie 12 (~26 d, 81,454 v/162,962 t, 2.9 MB→2.4 MB) · 14 (~32 d,
56,467 v, 2.0 MB→1.0 MB) · 16 (~39 d, 45,548 v, 1.6 MB→820 KB) · 18 (~44 d, 23,991 v, 865 KB→432 KB) · 20
(~49 d, 51,748 v, 1.9 MB→932 KB) · 23 (~56 d, 15,351 v, 554 KB→277 KB). (Vert counts vary by specimen/mesh
density, not monotonically — the `t`-axis is gestational age, not vertex count.) Non-network: fetal-series
table + ccf_resolver **6/6, 5/5** green.

This is the **first concrete use of the `.10d` `t`-axis** (reproductive plan §2): a developing body as a
function of gestational age — and it demonstrates *the consideration* Timothy asked for (fetal development as
part of the female-anatomy picture, not scoped out). It **resolves the reproductive plan's standing ⚑** (the
"HDCA is cell-level → no whole-fetus 3-D" gap) **for the embryonic period**.

**⚑ Honest remainders.** (1) **The later *fetal* period (9 weeks → birth)** has no comparably clean CC-BY 3-D
series — a separate acquisition (Timothy's pointer, or accept embryonic-only for the demo). (2) **Aging**:
there is no clean scanned aged-body series like the Carnegie embryos; aging is better modelled as **parametric
state-modulation** (the body changing with age, via the reproductive-continuum `StateModulator`), not distinct
scanned assets — fetal-first was the right call. (3) The `t`-axis is carried as stage metadata in the resolver;
writing it into the `.10d` node's `t` field + a `geo:gestationalAgeDays` manifest fact is the small next step.

**Next step.** Either (a) write the `t`-coordinate into each fetal `.10d` (node `t` + manifest fact) so the
series is a proper 4-D developmental body, and place the embryo within the maternal frame (the dyad — showing
the impact on female anatomy structures); or (b) the reproductive-continuum P1 state machine. Both mesh-ready.

---

## 2026-07-05 — S7b: developmental `t`-coordinate on the fetal `.10d` + the maternal–fetal DYAD (real data)

Timothy: "yup, do it." Two parts, both built + proven.

**Part 1 — the `t`-coordinate on the fetal `.10d`.** NEW `render/assets.rs::mesh_to_nquins_with_dev` +
`render/compile_10d.rs::compile_developmental_asset` — bind `geo:gestationalAgeDays` (postfertilization) +
`geo:carnegieStage` as `u64` manifest facts, so a fetal `.10d` is a *slice of a 4-D developmental body*
(consecutive stages ordered by gestational age = the `t`-axis). The `fetal_stages` series harness now
compiles via `compile_developmental_asset` (carries the coordinate). Test
`compile_developmental_asset_binds_the_t_axis_coordinate` → **compile_10d 10/10** (identical container to
`compile_asset`, +2 facts; the 44-day/stage-18 objects present).

**Part 2 — the maternal–fetal dyad.** NEW `qualia-client-core/src/wellfair/anatomy_dyad.rs`:
`MaternalFetalDyad{maternal_model=Female, host_structure="uterus", carnegie_stage, gestational_age_days}` +
`place_within(host_bbox, fetal_bbox, fill) → DyadPlacement{translate, scale}` (centre the embryo at the host
centroid, scale to fit inside — *illustrative*: the HRA and NIH meshes are in different source scales;
real-world-scale registration is a flagged follow-up). Pure test green. **Real-asset harness
`place_embryo_in_maternal_uterus_from_real_assets`**: discovered + fetched the HRA **female uterus** + the
Carnegie-18 embryo, compiled both, and seated the embryo in the uterus → *Carnegie 18 (~44 d) at the uterus
centroid `[-0.014, 0.028, -0.059]` m, scale 0.00265; uterus extent ~`[0.052, 0.051, 0.063]` m (~5–6 cm,
anatomically plausible), embryo source extent `[4.8, 11.5, 8.6]`.* This is **the consideration demonstrated
on real anatomy** — the developing embryo placed within the female reproductive structure at a gestational
age; the maternal–fetal dyad, real on both sides.

**Measured.** compile_10d **10/10**, anatomy_dyad pure **1/1**, fetal_stages table **1/1** (harness compiles
via the developmental path); dyad real-asset harness green (placement above). Non-network suites unaffected.

**⚑ Honest.** The dyad placement is **illustrative** (no shared real-world scale between the HRA adult and NIH
embryo sources — anatomically-registered placement needs real-world-scale metadata on both meshes; the
follow-up). The `t`-coordinate is a manifest fact now; writing it into the `.10d` node's `t` **field** (so the
container is natively 4-D, not just annotated) is the deeper step (touches `container_10d` node-section
writing — coordinate with the CG/.10d lane). Later fetal period (9 wk→birth) still ⚑ (no clean CC-BY 3-D).

**Next step.** The reproductive-continuum P1 state machine (the female continuum as whole-body states) — the
remaining substance of "the dignity of people born female," fully in-lane and mesh-independent.
