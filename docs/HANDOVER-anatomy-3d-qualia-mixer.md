# HANDOVER — Anatomy 3D · `.qualia` bundles · attention mixer (session 2026-07-11)

Pick-up doc for a fresh session. Nothing below is committed — **all changes are in the working tree**
(`git status` shows them). Ask Timothy before committing.

---

## 0. THE MISSION (read first — it sets priorities)

**Human-centric:** an instrument for a person to take their *own* pathology results, imaging, and diagnoses
— the real information their doctors give them — and **see their conditions on their own body**, to
understand them. It is **not** an anatomy atlas, a research reference, or a completeness exercise. Anatomy
meshes/licences/semantics exist only to serve *the person visualizing what is real for them*. Corrected
2026-07-11 after a wrong "collect more models" tangent. See [[anti-enslavement-not-identity-wallet]],
[[selfhood-guardian-principal-agent-model]], [[three-d-anatomy-qapp-priority]].

The spine, in order: **(1) the person's records in → conditions** → **(2) conditions → organs/systems via
disease↔organ linked data** → **(3) seen on their body** (accumulation engine → σ colour + select/zoom stages).

---

## 1. What was built + VERIFIED this session

Real, working, verified — not stubs:

- **`.qualia` bundle format** — new `crates/qualia-core-db/src/bundle/` (`format.rs`/`writer.rs`/`reader.rs`).
  A **transparent container-of-files**: page-aligned intact `.q42`/`.10d`/`.p64` entries + CBOR index +
  whole-file CRC-32C + per-entry SHA-256; native zero-copy `BundleMmap` + same reader in WASM. **Does NOT
  interfere with interior segment access** (Timothy's constraint). `cargo test -p qualia-core-db --lib
  bundle::` → **9/9 green**. Memory: [[qualia-bundle-format]].
- **Anatomy pack producer** — `crates/qualia-client-core/src/wellfair/anatomy_pack.rs` + example
  `crates/qualia-client-core/examples/build_anatomy_pack.rs` (subcommands `list`/`build`/`verify`/`bounds`).
  Discovers HRA (`lod.humanatlas.io/sparql`) → fetches GLBs (`cdn.humanatlas.io`, CC-BY-4.0) → compiles `.10d`
  → packs one `.qualia` per model. **`build both <dir>` now builds the COMPLETE body** (all discovered
  organs; `curated: None`). Ran live: **XY 26 organs / 97 MB, XX 33 organs / 122 MB**, incl. blood-vasculature
  + skin. Shared meta type: `crates/qualia-core-db/src/render/anatomy_pack.rs::AnatomyOrganMeta`.
- **THE RENDER FIX** (the big one) — `QualiaPortal::load_body_organs_colored` in
  `crates/qualia-core-db/src/render/portal/mod.rs`. It used to **per-organ normalize** (centre + scale each to
  0.15 + scatter by an approximate table) → wrong proportions, wrong orientation, skin shrunk to a dot. The
  `build_anatomy_pack -- bounds` probe proved **the CCF meshes share ONE body coordinate space** (brain
  y≈0.83, pelvis y≈0.08, skin spans the whole body). Rewrote it to keep **true coordinates + ONE global
  centre+scale**. Result **verified in Chrome (WebGPU)**: proper upright body, organs in true places,
  translucent, skin as a toggleable envelope.
- **WASM facade** — `QualiaPortal::load_body_from_qualia_bundle(bytes)` and
  `…_mixed(bytes, {system: level})` (per-body-system mute/opacity), plus `set_ambient_enabled(bool)`.
- **Ambient field OFF by default** — `crates/qualia-core-db/src/render/gpu/mod.rs`: `PortalGpu.ambient_enabled`
  (both draw sites gated; auto-on when a **tensor** is uploaded, since the particles then ARE data). The
  "floating box of dots" was the decorative random-particle fallback (`generate_particles`, `epistemic_q=0`).
- **Attention mixer v1** — `docs/playground/anatomy.{html,js}` (10 system channels + ambient toggle; skin
  muted by default). Plan: [`docs/plans/attention-mixer.md`](plans/attention-mixer.md).
- **Pack-level `.q42` semantics** (NEW, this session) — the producer now aggregates every organ into ONE
  `.q42` graph volume: provenance (real CCF/HRA source URL + CC-BY-4.0 licence + creator + derived-from),
  organ→system, depiction/descriptors — built from the SAME `organ_container`/`body_container`
  (`anatomy_body.rs`) the desktop uses, so the pack's semantics ARE the product's. Carried **inside** the
  `.qualia` bundle (the `body.q42` entry) AND written beside it as a byte-identical, directly-linkable
  sidecar `anatomy-<sex>.q42` (one graph, two carriers, no drift). New
  `UnifiedVolumeBuilder::finish_to_bytes()` (`q42_volume.rs`) produces the `.q42` in-memory; `finish` now
  delegates to it. The demo copyright panel links the per-sex `.q42`; `build`/`verify` report its
  quins + CC-BY facts; release+Pages CI publishes/fetches the sidecars. Tests green:
  `q42_volume::…finish_to_bytes_matches_a_readable_on_disk_volume`,
  `anatomy_pack::…pack_q42_carries_provenance_and_system_semantics` (licence + source URL +
  respiratory/circulatory facts all queryable from the `.q42`). **This is the growth spine** — disease↔organ
  links and (privately, client-side) the person's own conditions append to this graph. *Not re-run over the
  live network this session (unit-verified equivalent); CI cuts the real sidecars on release.*
- **Extensible body-system registry** (NEW, this session — Timothy: "there should be more… support
  evaluation of it all") — the 17 systems were a compile-time `static`; now `wellfare-core/anatomy/registry.rs`
  makes them a **seeded, extensible registry**: `SystemDef` (owned label/plain_label/representation/overlay
  hosts/colour/provenance) + `SystemRegistry` (`seed()` the 17, `register()`/refine, lookups,
  `default_registry()`) + `SystemProvenance::{Seed, Ontology{iri}, Pack, User}` — the last is the **graph-fed
  hook** (UBERON system classes populate the registry once the disease↔organ/Monarch source is chosen). The
  engine already evaluated *any* `system_id`; the registry makes a *new* system **first-class** (label,
  colour, representation). Fixed a real bug: `import_condition_map` took no registry and **warn-and-skipped**
  a condition mapping to a non-seeded system (silent loss) — it now resolves through a registry, so a
  registered extension is evaluated. Unified the colour palette (`palette_for` → `registry.color_of`, one
  source). Demo mixer now surfaces **all 17** (was 10; ECS/ENS/glymphatic marked `·overlay`, disabled) —
  verified in Claude-in-Chrome. Decision on record: **registry-now + graph-fed**. Tests: wellfare-core anatomy
  87/87, client-core anatomy 27/27.
- **Organs as multi-system building blocks + system graph** (NEW, this session — Timothy: "organs aren't a
  separate category… they make up systems"; the 11 majors; "systems don't operate in isolation"). Organ→system
  was one-to-one (pancreas→digestive only); now `system_memberships_for_organ` returns primary + secondaries
  (pancreas = digestive+endocrine+exocrine, liver, kidney+endocrine, skin+sensory+exocrine, +diaphragm, …), and
  `AnatomyOrganMeta.systems: Vec<String>` (serde-default, back-compat) carries them so a pack supports **both**
  primary-colour and blended render (his "1 or 2"). `SystemDef` gained `tier` (CanonicalMajor×11 / SubSystem /
  CrossCutting), `parent` (sensory/vestibular/ENS/glymphatic → nervous), and `relations`
  (DependsOn/Regulates/Supplies — his calcium + nervous-control examples; **structural context only, no burden
  propagation**, his call). Tests: wellfare-core 89/89, core-db meta 2/2, client-core 27/27. **⚑ Mesh
  completeness:** the CCF pull is already complete (viscera only); full-body (muscles/glands/nerves/inner-ear)
  needs BodyParts3D/Z-Anatomy (FMA-keyed, **CC-BY-SA**) — Timothy's licence call (permissive-only vs a separate
  CC-BY-SA pack). The multi-system + FMA-join model is the prerequisite, and is built.
- **BodyParts3D ingestion — the completeness pack** (NEW, this session — Timothy pointed at the BodyParts3D STL
  library, i.e. "go"). `qualia-client-core/wellfair/bodyparts3d_resolver.rs` ingests the FMA-keyed open library
  that fills what CCF (viscera-only) lacks: 437 muscles, 251 bones, 99 nerves, glands, sense organs (937
  structures, ~1.3 GB). Pure part-of→system mapping (`Bp3dHierarchy` parses `parts_list_e.txt` +
  `conventional_part_of.txt`; `systems_for(id)` walks up to the 16 FMA system roots) — 934/937 resolve, 70
  multi-system from the real ontology (diaphragm → muscular+respiratory). Producer `build_bodyparts3d_pack`
  is **bandwidth-controlled** (systems + count + STL-size caps), fetches STL → `compile_organ_asset` (attested
  **CC-BY-SA-2.1-JP**, kept a separate pack; CCF stays permissive) → `.qualia` with full multi-system meta;
  `bodyparts` example subcommand. **Verified live**. **CI + demo now wired** (this session): release-wasm.yml
  builds `bodyparts all` → uploads `anatomy-bodyparts3d.{qualia,q42}` to the Release (a Release asset only —
  exceeds Pages' 100 MB/file limit); the demo has a **"Complete body" toggle** that fetches it from
  `releases/latest/download` and renders it **standalone** (its own coord space — so no CCF alignment needed
  yet) with the CC-BY-SA attribution + citation. Verified in Chrome (CCF unbroken; Complete button + note
  present; graceful fetch-fail with no release). ⚑ Remaining: cut a release to populate the pack; later,
  align BodyParts3D↔CCF spaces to *overlay* the two bodies (vs the standalone toggle).
- **BodyParts3D as an addressable ONTOLOGY + 10d library** (NEW, this session — Timothy: "define an ontology…
  store in a q42, with a library of d10 files"; "all of it, fully addressable semantically, for comorbidity
  implications"). Licence CONFIRMED: the *meshes* are **CC-BY-SA-2.1-JP** (DBCLS/lifesciencedb.jp); the repo's
  own MIT covers only Moerman's code, not the data (exact attribution + citation + DOI now recorded). The "Web
  API" is a map-IMAGE/heatmap service, not a data API — ontology + meshes come from the data archive.
  `bodyparts3d_ontology.rs::emit_ontology` keys each FMA concept by its **canonical OBO IRI** (`obo:FMA_<id>` →
  joins Monarch/UBERON/MONDO, so comorbidity reasoning is a graph walk) **+ house `q42:`/`geo:` aliases**, with
  `rdfs:label`, `rdfs:subClassOf` (is-a, from `FMA.csv`), `BFO_0000050` (part-of), `geo:bodySystem`, and
  `geo:compiledDigest` (hasMesh → the `.10d`) + a dataset node carrying the CC-BY-SA provenance. The producer
  emits the ontology **`.q42`** (`body.q42` + linkable sidecar) that **cites the `.10d` mesh library by digest**
  (two-layer geometry-asset model at ontology scale). Verified live (6 endocrine glands → a **70-quin ontology
  `.q42`**, round-trip). Full = all ~900 mesh concepts + both trees, via CI. Tests: resolver + ontology 7/7.
- **Demo is DYNAMIC — systems + parts from the pack, select/deselect at both levels** (NEW, this session —
  Timothy asked; it was a real gap: the mixer was a hardcoded 17-list, no part selection). New WASM facade
  `QualiaPortal::pack_manifest(bytes)` → `[{key,label,system,systems}]` per part (from the bundle index +
  meta); `AnatomyOrganMeta` gained `label`; `load_body_from_qualia_bundle_mixed` gained a 3rd arg
  `disabled_parts` (per-part hide). The demo mixer is now built from the systems the pack **actually
  contains** (CCF → 10 + overlays, not 17), and a **"Parts"** panel lists every structure grouped by system,
  searchable, with per-part checkboxes + Select/Deselect-all. **Verified in Chrome** (WASM rebuilt): CCF →
  10-system mixer + 26-part panel; "Deselect all" → 0 structures (per-part hide proven); scales to the
  ~900-part complete body via search. **Rebuild note:** the portal WASM was rebuilt (`wasm-pack build … 
  --features portal`) + copied/patched into `docs/pkg/qualia/*` — do this after any `render/portal` change.
- **Governance/provenance fix** — `compile_organ_asset` now takes `Option<&ProvenanceSidecar>`;
  `compile_body` attests each organ (`urn:hra:ccf:{organ}`, CC-BY-4.0). Without it the renderer **refused all
  organs** (fail-closed governance). `cargo test -p qualia-core-db --lib compile_10d::` → **11/11 green**.
- **The demo** — `docs/playground/anatomy.{html,js}` fully rewritten: portal WASM + `.qualia` + mixer +
  XY/XX toggle + **collapsible copyright container** (`<details>` bottom-right, per-source precisely-scoped
  licence, non-overclaiming note, `.q42` provenance hook `PROVENANCE_Q42_URL`) + a file:// diagnostic guard.
  Replaced the old three.js box placeholder; fixed the 0×0-canvas bug.
- **Release/bandwidth wiring** — `scripts/fetch_anatomy_packs_release.sh`; `pages.yml` step "Anatomy `.qualia`
  packs (prefer Release, fall back to building)"; `release-wasm.yml` new `build-anatomy-packs` job (builds +
  uploads packs to the GitHub Release on `v*` tags). Shell + YAML syntax-validated; **untested in CI**.
- **Earlier same session:** fixed the broken `webizen-desktop` build (E0521 in `main.rs`
  `anatomy_body_json_response`; the closure borrowed non-`'static` params into `execute_sync`); un-hardcoded
  the `webizen://…/anatomy/body.json` model. First-run setup plan + the mindware/channels architecture + the
  anti-enslavement directive (see other plans/memories).

## 2. How to SEE it (verification path — IMPORTANT)

- **Use Claude-in-Chrome** (his real Chrome, WebGPU works) to screenshot the demo — the in-app Claude_Browser
  pane **cannot capture WebGPU** (screenshots time out). Memory:
  [[feedback-webgpu-verify-with-claude-in-chrome]].
- Serve `docs/` over HTTP and open `http://127.0.0.1:8099/playground/anatomy.html` in a WebGPU browser
  (Chrome/Edge). **Must be HTTP, not `file://`** (ES modules + WASM fetch are blocked on file://; the page
  hangs on "Starting…"). A python static server was running this session:
  `python -m http.server 8099 --bind 127.0.0.1` from `docs/` (may have stopped — restart if needed).
- The demo loads `docs/pkg/qualia/qualia.js` (+ `qualia_bg.wasm`) and fetches `anatomy-{male,female}.qualia`
  from `docs/playground/`.

## 3. To rebuild the artifacts locally

- **Portal WASM** (has the facade): `RUSTFLAGS="-C target-feature=+simd128" wasm-pack build
  crates/qualia-core-db --release --target web --out-dir pkg-qualia -- --no-default-features --features
  portal`, then copy `crates/qualia-core-db/pkg-qualia/qualia_core_db.{js,_bg.wasm}` →
  `docs/pkg/qualia/qualia.js` + `qualia_bg.wasm` and run `bash scripts/patch-portal-wasm-js.sh
  docs/pkg/qualia/qualia.js` (renames the wasm ref). ~30–40 s release.
- **Packs**: `cargo run --release -p qualia-client-core --example build_anatomy_pack -- build both
  target/anatomy-pack` (needs network to the HRA), then copy the two `.qualia` → `docs/playground/`. `verify
  both <dir>` checks SHA-256 + meta; `bounds male <dir>` prints organ centroids/sizes.
- Packs + `pkg-qualia`/`target/portal-wasm` are **gitignored** (`.gitignore` updated); `docs/pkg/qualia/*` and
  `docs/playground/*.qualia` are the served copies (packs gitignored → CI/Release-produced).

## 4. Open decisions (Timothy's — ⚑)

1. **Disease↔organ linked-data source:** Monarch (MONDO→UBERON, open, **recommended**) vs **SNOMED CT via AU
   NCTS** (richest `finding site`, but registration). Bio2RDF as enrichment. See
   [`docs/plans/anatomy-linked-data-and-interactive-stages.md`](plans/anatomy-linked-data-and-interactive-stages.md).
2. **More-models licence:** stay CC-BY-4.0 (CCF only) vs add a **separate CC-BY-SA** BodyParts3D/Z-Anatomy
   pack (the `.10d` pipeline already imports OBJ/STL/GLB, `assets.rs:144`). Per the mission, only as needed to
   visualise the person's conditions — not for completeness.
3. **Next build order:** the `.q42` pack-semantics first (attribution+provenance made real, then grows) vs the
   **records-in** side (pathology/imaging/diagnosis → conditions) first.
4. **Vault at rest** (from the first-run plan, Fork A): opt-in passphrase — still open.

## 5. Next steps (re-centred on the mission)

1. **Pack `.q42` semantics** — ✅ **DONE this session** (see §1 "Pack-level `.q42` semantics"): pack-level
   `.q42` with real provenance + organ→system, inside the `.qualia` bundle (`body.q42`) + a linkable
   `anatomy-<sex>.q42` sidecar, linked from the copyright panel, unit-tested. **Next slice on this thread:**
   have the *browser* read the in-pack `.q42` (a bytes-based Q42 reader for WASM — `Q42Volume` is currently
   mmap/native-only) so the copyright panel renders the provenance graph inline instead of linking a download,
   and so the same reader can later drive "colour organ by the person's condition" in the web channel.
2. **Person's-data spine** — ingest the person's pathology/imaging/diagnosis records → conditions/`Factor`s
   (the WellFair engine exists: `crates/wellfare-core/src/anatomy/` — **17 systems**, `accumulate.rs`,
   `lens.rs` σ, `temporal.rs`, `scorecard.rs`, `pathway.rs` VOI, `knowledge.rs`; host wiring
   `qualia-client-core/wellfair/anatomy_view.rs::paint_organs` + `api.rs::cached_body_organ_percepts` — all
   REAL, tested; the *magnitudes/corpus* are curation-grade seed). Colour the body by accumulated burden.
3. **Selectable organs + zoom-into-organ stages** — the renderer **already has GPU picking**
   (`select_node_at`/`poll_selected_node`/`navigate_to_node`); tag each organ's vertex-range with its
   UBERON/FMA id → click → organ stage showing its issues (factors, implications, wire in `pathway.rs`).
4. Then: disease↔organ ingestion (decision #1), system/issue stages.

## 6. Gotchas

- WebGPU screenshots only work in **Claude-in-Chrome**, not the in-app pane.
- A `.10d` compiled **without a `ProvenanceSidecar` is REFUSED** by the renderer (fail-closed governance) —
  `compile_body` attests; `compile_asset`/`compile_organ_asset` default to none.
- The mesh render appears **translucent** (nice for anatomy) — likely the pipeline blends, so the mixer's
  per-system faders may already be real opacity, not just mute/show (confirm).
- Packs are ~100–120 MB each (per-user download). Timothy: big is fine, but they **must be Release-served**
  (wiring done, needs a release cut to activate).

## 7. Plan docs (repo)

- [`docs/plans/first-run-setup-and-inforg-onboarding.md`](plans/first-run-setup-and-inforg-onboarding.md) —
  first-run setup + the P0–P9 anatomy/render/pack history + mindware/channels + anti-enslavement correction.
- [`docs/plans/attention-mixer.md`](plans/attention-mixer.md) — the mixer (v1 done; v2 = alpha-blend faders +
  engine-side Mixer; v3 cross-fade; v4 Chora stages).
- [`docs/plans/anatomy-linked-data-and-interactive-stages.md`](plans/anatomy-linked-data-and-interactive-stages.md)
  — the disease↔system linked-data sources (Bio2RDF/Monarch/MONDO/UBERON/SNOMED/HRA), extra 3D-model sources
  (BodyParts3D/Z-Anatomy, FMA-keyed), and the stage-navigation architecture.
- `coordination/NOTICES.md` — the dated CLAIM/PROGRESS/RELEASE feed for this session.
