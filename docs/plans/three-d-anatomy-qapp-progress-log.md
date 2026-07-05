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
