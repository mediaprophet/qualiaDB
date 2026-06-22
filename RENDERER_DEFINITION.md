# The QualiaDB Manifold Engine — Definition (sense · reason · render)

> **Status:** working definition (draft 1, 2026-06-22).
> **What this is:** a single reference that consolidates the manifold-engine architecture
> **Timothy Holborn defined** across the source specs in the Appendix and the design dialogue, so that
> the qapps authoring spec ([`docs/manuals/qapps_specification.md`](docs/manuals/qapps_specification.md))
> and the implementation can be built against one definition.
> **What this is not:** it introduces **no new architecture** and **claims no authorship**. Where it adds
> anything it is synthesis and cross-reference, marked as such. The architecture is Timothy's; this is the
> map of it. (Authorship law: the program is Timothy's work; the software that wrote this file is a tool he
> uses — see `memory/feedback-no-authorship-claim.md`.)

The file is named `RENDERER_DEFINITION.md` for continuity, but the thing being defined is **not a renderer**.
It is an **engine that senses, reasons, and renders over one substrate**. "Render" is one of its three
directions. Keep that in mind throughout.

---

## 0. One paragraph

QualiaDB's engine treats perception and depiction as **two directions of one operation over a single 10-D
manifold**. The manifold is `[q, v, w, x, y, z, t, α, μ, σ]` (the 10D tensor spec). Sensors project the
world *into* it (microphone, camera, radio, thermal → wave coordinates → discrete facts); the renderer
projects it *out* (manifold → percept on a screen, speaker, or print). In between, the logic/values layer
reasons over the **same** bytes the LLM, the renderer, and the audio path compute over — because they all
share one GPU device and one resident substrate (graph–tensor duality, `compute_universe.rs`). Physics and
law are not bolted on; they are folded into the substrate. The whole thing is governed by the same rails in
both directions, and **wisdom stays out-of-band, in the human** — the engine carries Data→Information→
Knowledge and stops there.

---

## 1. First principle — every view is a *projection* of the 10D manifold

From **STELLAR §E** and the **10D tensor spec §2**: a 2D screen, a 3D scene, and a 4D spacetime view are
**enumerated from the same 10D structure**. There is one operation —

```
project : 10D manifold → target (2D | 3D | 4D)
```

— computed by the volume metric (`tensor/volume_gpu.rs`, `tensor/manifold.rs`). Lower dimensionalities
*derive from* the 10D foundation; that is the reason 10D is the foundation rather than an add-on. The CML
Studio 2D canvas and a 3D world scene are **the same manifold at different projections**, not two engines.

**Corollary — percepts are enumerated, not stored** (STELLAR §D). Colour, pitch, brightness, timbre, heat
are *pure functions over the fixed coordinates within a band*: `colour = f(wavelength, amplitude)` in the
visible-EMF band; `pitch = f(frequency)` in the audible-acoustic band. Nothing stores an RGB literal or a
sample buffer; the percept is computed at the last mile, on the fly, from fixed dims. This is why "store the
physics (the wavelength), map the colour at render" is not a slogan but the storage model — and why it reads
a little like ray-tracing: the depiction is *derived from the physical description*, not the other way round.

---

## 2. The engine is bidirectional — sense ↔ reason ↔ render over the wave substrate

The completion of the design (dialogue 2026-06-22 + **STELLAR §D**): sensors are not special-cased I/O.
A microphone (acoustic), a camera (optical EMF), a radio — Wi-Fi / Bluetooth (RF EMF), a thermal sensor —
are all **bands of one wave substrate**, reducible to a common **wave coordinate**:
frequency/wavelength · amplitude · phase (+ modulation μ, signature σ) — exactly
`SpectralDecomposition{amplitude, phase, frequency}` and the 10D spectral axes `[α, μ, σ]`.

```
        SENSE  (input-projection INTO the manifold)
   mic → STFT/CQT ┐
   camera → spectral├─→ wave coordinate ─→ percept→fact bridge ─┐
   radio → RF       │     (fixed 10D dims, zero-heap)   (∫Ψ > τ → Fact, §20 manifold_logic.rs)
   thermal → band  ┘                                            │
                                                                ▼
        REASON   NQuin graph + logic modalities (deontic / legal / temporal / spatial) + values
                                                                │
        RENDER (output-projection FROM the manifold)            ▼
   manifold ─→ project : 10D → target ─→ percept (screen / speaker / print)
```

Same substrate, run in both directions. Sensors are modelled as **W3C SOSA/SSN** observations (sensors-as-
observations); a **new modality is just a new band/projection** of the same substrate — the engine is
sensor-extensible by construction, not by adding codecs.

**Two disciplines carry over from §D and must not be dropped:**
- **EMF ≠ acoustic — don't flatten.** They share the *wave abstraction* but are physically distinct kinds
  (EM transverse waves at `c` in vacuum vs mechanical pressure waves at medium-dependent `v`). The kind is a
  tagged band parameter, so enumeration and propagation physics respect it (colour comes from EMF-visible,
  pitch from acoustic-audible). Same anti-flattening rule as the man-made/natural and identifier/identity
  boundaries elsewhere in the project.
- **Access is tiered; say so honestly.** The substrate *models* every band uniformly, but *access* is not
  uniform. A microphone is readily available (real STFT/CQT today). Wi-Fi/Bluetooth at the **protocol
  level** (scan, RSSI, presence, beacons) are reachable via OS APIs and good for spatial/presence sensing;
  **raw RF spectrum / Wi-Fi CSI** (STELLAR §D's "sense through walls / vitals") needs an **SDR or special
  hardware + permissions**. The engine never pretends to sniff arbitrary RF without that hardware.

The implication for the spec: there is a **sensing/input contract** (SOSA/SSN-aligned) that is the
**symmetric twin** of the render/output contract — and both run under the same governance rails (§8). The
engine is a **perception engine**, not merely a renderer.

---

## 3. Physics, logic, and compute — one substrate, fast on every device

From **STELLAR §F** — the fabric that already exists (`compute_universe.rs`): one physical `wgpu::Device`
(`gpu_context::shared_gpu`); the semantic `NQuin` graph and the `Tensor10D` SOA **share one resident
substrate** (graph–tensor duality); compute runs as coordinated **universes** — U0 (LLM) · U1 (tensor) ·
Sentinel (governance) — over lock-free SPSC rings (the Phase-8 bifurcation), under one `VramLedger`. Data is
encoded **once** and the GPU **enumerates** it for every consumer. That is the performance argument *and*
the architectural one: there is no marshalling between "an LLM process" and "a renderer process," because
there are no separate processes — there is one manifold and many projections of it.

- **Physics is in the substrate.** Gravito-thermodynamic operators (10D spec §2.4: α as mass, T as
  diffusion/plasticity, P as density/constraint — applied at bake-time + as modulated geometric operators);
  **Projective Geometric Algebra** multivectors `M = α + v + B + T` bound to `kinematics.wgsl`; geometry that
  **"refuses to contract"** when a suggested action violates a physical bounding box (deterministic
  prevention, STELLAR §E). An artefact carries its physics, not just its shape.
- **Logic is over the same substrate.** The modality stack (deontic / legal / Hohfeld / STIT / temporal
  LTL+Allen / spatial RCC-8 / probabilistic / defeasible) evaluates the **same** 10D bytes. Spatio-temporal
  logic about an artefact is therefore **queryable by the same modalities the values layer uses** — "where/
  when/under-what-jurisdiction is this" is the same kind of question as "is this permitted."

This is why the LLM, display, and audio were brought *into* the engine rather than federated: a single
pipeline manifold (STELLAR §F). (Honest scope: §F's *remaining* work is to bring render `viewport/` and
audio `spectral/` fully *under* the universe orchestration as formal universes, and to land the single fused
cross-manifold pass; today they are distinct pipelines on the one shared device — see §9.)

### 3.1 One encoding, many silicons — dispatch each *operation* to the chip built for it

The reason it can be fast *everywhere*, not just on a big discrete GPU: data is encoded **once** on the
shared substrate, and a **capability + power-telemetry profiler** routes each **operation** (not the whole
job) to the processor built for it — **never a global handicap** (10D spec §3; STELLAR §G). This is the
bedrock *under* the §F universe fabric.

- **NPU** (WebNN · CoreML/ANE · DirectML · NNAPI · OpenVINO) — **tensor contraction as a primitive**: the
  multi-way dot-product reductions over the q42 LLM weights, the PGA / 10D tensors, and the manifold metric,
  **without flattening the geometry**. Power-efficient → the right home for the LLM matmuls on laptop/mobile.
- **GPU** (WebGPU → Metal / Vulkan / DirectML) — continuous physics & spatial dataflow (`kinematics.wgsl`,
  `tensor_volume.wgsl`), GEMM, and the render projection (§1).
- **GPU also runs *logic*, before the math finishes** — the **neuro-symbolic sieve** (STELLAR §B): WGSL
  shaders reason over the ontology graph *before* the probability math completes, and **deontic token masking**
  sets forbidden token IDs to probability zero **at the hardware level** (e.g. "no clinical advice" masks the
  relevant SNOMED/RadLex tokens). Logic is literally piped through the GPU, not bolted on after.
- **CPU** (WASM-SIMD · AVX2 · NEON, `simd_kernel.rs`) — deterministic logic and the **deontic/DID
  gatekeeper** (`shacl_compiler.rs`, `n3_parser.rs`): it **short-circuits the bus before an unlawful or
  unneeded vector is ever dispatched.** This is where *fast* and *governed* are the **same** thing — §F
  redefines "attention" as a constraint-satisfaction / deontic gateway that discards irrelevant or forbidden
  data at the bus. The cheapest work is the work you prove you don't have to do.
- **One program, every tier.** The *same* 10D structure + 64-opcode VM bytecode runs from a phone to an
  A2000; only the **math backend swaps** (10D spec §4–§5, graceful degradation). The dispatcher changes the
  engine, never the code.

### 3.2 The q42 perf path — what makes a model fit a phone or an Intel iGPU (STELLAR §A)

The heavy work is **AOT**: done once at transcode/bake, with scale factors baked into the **CBOR-LD header**,
so the hot path stays zero-heap geometric ops sized to the cache.

- **Ternary (BitNet 1.58b)** FFN packing — weights ∈ {−1, 0, 1} turn FMAs into **adds/subs** in the WGSL/SIMD
  kernels (reported 3–6× speed, ~70% energy). Decisive on weak ALUs and on battery.
- **KIVI KV-cache** — 2-bit key / 4-bit value → 100k-token context in consumer/edge memory via a ring buffer.
- **W4A4 + AWQ** (the "Concentration-Alignment Transform") → Q8-equivalent math at 4-bit speed.
- **Speculative decoding** via zero-copy mmap (small draft + target verified in one pass) → 2–3× perceived
  TPS, no heap penalty.
- **Demand-paged mmap** → run models larger than physical RAM (page a layer in at the microsecond of use).

These are *the* enablers for the constrained targets — without them, mobile/Intel is a slideshow; with them,
the same model is interactive.

### 3.3 Per-target honesty (best case → hard case)

> The **canonical build-target matrix** — native **Windows** (DX12) · **macOS** (Metal) · **Linux** (Vulkan)
> · **iOS/Android** (mobile) · **`wasm32`** · **`wasm64`/memory64** — lives in
> [`RENDERER_IMPLEMENTATION_PLAN.md`](RENDERER_IMPLEMENTATION_PLAN.md) §0 (single source). The per-hardware
> notes below are the *why* behind that matrix.

- **Apple Silicon / Metal — best case.** **Unified memory** makes "one resident substrate, mmap to VRAM"
  *literally zero-copy* — no CPU↔GPU marshalling at all, which is exactly what §F's design wants. GPU + **Apple
  Neural Engine** (CoreML) + **AMX / Accelerate** matrix units; `metal_bridge.rs`. Lean in hard; "awesome" is
  easiest here.
- **Dedicated-GPU laptop / modern NPU — Tier 2/1.** Whole 10D volume mmap to VRAM; TMU/BMM for cross-`w`
  projection; DirectML on Windows.
- **Mobile (phones & tablets) — Tier 0/1, the focus.** A native **PWA shell** (`android_pwa_edge` /
  `ios_pwa_edge`) for the raw WebGPU + thread control a consumer browser won't grant; **ANE / Hexagon /
  NNAPI** for the NPU; **512 MB RAM cap**, OPFS storage; **battery-aware** dispatch (Deficit → 1.58b inference,
  training suspended · Equilibrium → Q4 · Surplus → saturate). Ternary + KIVI are **non-optional** here.
- **Intel integrated graphics (and genuinely GPU-less) — the hard case, handled by design.** An iGPU works
  via **DirectML / WebGPU** but is weak and shares system RAM → lean on **CPU AVX2 SIMD** + aggressive
  INT4/INT8 quant to fit L1/L2; use the newer **Intel NPU ("AI Boost")** via **WebNN / OpenVINO** where
  present. On a machine with no usable GPU, the **same program** falls back to pure CPU SIMD (Tier 0): slower,
  but it *runs*. Graceful degradation — never a crash, never a global handicap.

### 3.4 The blocker to clear first — or "everywhere" is a lie

The web/WebGPU path is currently broken on the very devices this targets: **`wgpu 0.19.4` sends
`maxInterStageShaderComponents` → `requestDevice` fails on current Chrome** (defect #1), and the WASM bundle
isn't loading on GH-Pages. Until the **wgpu upgrade** lands **once** (for compute *and* render on the shared
device), "fast on every device" is aspirational on the browser path; native (Metal / DirectML) is less
affected. This is why it is step 2 of §10, gating the rest.

---

## 4. What the engine exposes — capabilities (game-engine-like, more advanced)

Exposed the way a game engine is exposed, but governed and physics/logic-native. Four capabilities, all
projections/uses of the one substrate:

1. **Engine-as-platform.** A declaratively-authored, governed space that "things" live in — documents,
   widgets, scenes, worlds — authored via the spectrum in §5–§7 and rendered by §1's projection.
2. **Generative CAD / build — the LLM as *pilot*.** Constraint-satisfying geometric modelling, **not**
   probabilistic 3D guessing (STELLAR §E): the deontic/attention layer verifies **watertight + structurally
   sound + printable** (overhang / wall-thickness ontology) **before** tensor reduction. Output targets:
   3D-printer files, construction/design, fabrication. Photogrammetry is **inverse physics** (2D sequences →
   SDF / point cloud → 5D NQuins, so the object is *known* semantically, not pixels).
3. **Earth-grid spatio-temporal world engine.** `x,y,z` (space) + `t` (time → temporal evolution/animation)
   + **place/jurisdiction** (GeoSPARQL) evaluated by native **RCC-8** (`spatio_temporal.rs`) and **Allen/
   LTL** temporal reasoning. Earth as a grid environment on which to display history, science, botany — each
   artefact situated in space **and** time **and** place, and that situation is logic-queryable.
4. **Domain models (e.g. 3D anatomy).** Each asset = *physical manifold + kinematic multivector* (joints as
   multivectors in the 5th dimension); aberrations bind to domain ontologies (e.g. RadLex) so the model is
   semantically grounded. Biometrics never leave the device (§8).

---

## 5. The LLM is a pilot, not the driver

The LLM produces **up to knowledge** (D→I→K) and is **one authoring mode among several** — a *pilot* that
flies the engine, not its owner. It is governed on **every** call by the deontic gate
(`orchestrate_inference`: intent pre-flight, provenance-citation post-flight — never bypassed). Crucially:

> **Wisdom stays out-of-band, in the natural person.** (`memory/principle-dikw-wisdom-out-of-band.md`.)
> The engine may *apply* prior, attested human wisdom (a non-derogable baseline, a signed constraint), but
> it **never authors wisdom of its own**. The renderer/world-engine is a *medium for human wisdom* — it
> makes complex topics legible and communicable — not a generator of the final "ought." This is the top
> rail over everything below.

People author directly too (§6). The pilot is powerful and welcome; it is not sovereign.

---

## 6. Authoring spectrum — how people and agents make "things"

Four ways in, **one convergence**:

1. **Declarative RDF — a significantly enhanced `w3.org/ns/ui`.** One unified UI vocabulary that spans
   **HTML-like documents *and* 3D manifold scenes *and* SVG vector graphics**. Source of truth is RDF; the
   author describes *what exists and how it should project*, not draw calls. (Today's `ns/ui` does forms/
   widgets; §7 is the extension to "worlds.")
2. **CML inline markup** — inline declarative context that lands in the NQuin context field; the existing
   human-curated authoring path.
3. **LLM pilot** — generative, governed (§5).
4. **Direct edit** — the CML Studio canvas / the dev-bench.

**They converge through one bridge:**

```
RDF / enhanced-ns·ui / CML  ──expand──►  CBOR-LD  ──compile──►  48-byte NQuin / .q42
          (source of truth)        (binary, fast)        (one hash-space: q_hash)
```

- **CBOR-LD is the bridge encoding** (binary, compact) precisely to make the RDF→engine path fast — this is
  already the project's decision; the open piece is **`@context` expansion into the one hash-space**
  (task #8). RDF stays the human-readable source; CBOR-LD is the wire/edge form; NQuin is the resident form.
- **SVG** is the vector form for image objects — but per the storage model (§1), the *underlying* truth is
  **the physics (spectral signature / wavelength)**, and the colour/stroke is the *mapped* projection. A
  vector primitive declares geometry + a spectral signature, not a baked colour.

---

## 7. The enhanced `ns/ui` vocabulary — what has to be designed (the next artifact's input)

The qapps spec today ([`qapps_specification.md`](docs/manuals/qapps_specification.md)) describes **2D pane
layouts** (CssGrid, `PanePlacements`, `data_bindings`). The renderer definition is what lets it grow from
*panes* to *worlds*. The enhanced vocabulary must cover, declaratively and RDF-native:

| Concern | What the vocabulary declares |
|---|---|
| **Document** | HTML-like layout/flow (the existing pane/grid model, kept) |
| **Scene** | 3D scene graph: nodes, transforms, camera, lights |
| **Geometry** | mesh (vertex/index) · SDF · PGA motor (joints/kinematics in the 5th dim) |
| **Material / physics** | mass · material · momentum `P` (the Manifold-Coordinate, §C) · bounding box |
| **Place / space / time** | `x,y,z` + `t` + GeoSPARQL place/jurisdiction (the Earth-grid binding) |
| **Vector** | SVG primitives (path/shape) with a **spectral signature**, not a baked colour |
| **Percept mapping** | the projection rule: which band → which percept at render (store-physics-map-percept) |
| **Sensing** | SOSA/SSN observation bindings (the input twin of a render binding) |
| **Capability + governance** | capability intent + `SensitivityLabel` + standpoint (kept from qapps spec) |

This is the brief for upgrading `qapps_specification.md` once this definition is agreed.

---

## 8. Non-negotiable rails — the project's general governance substrate (sense and render alike)

These are **not engine-local safety** and **not a wrapper on AI output.** They are the project's **general,
rights-grounded governance substrate** — woven into the core and governing **every agent: human,
institutional, and machine** ("rule of law over man AND bot," STELLAR §0) — with **asymmetric accountability**
aimed primarily at *power* and protecting *persons*. Its reach is **broader than the engine**: the same
substrate governs the civic/social fabric (values credentials from the UN instruments; interpersonal commons;
guardianship; resilient relational identity). The engine, renderer, and ingest pipeline **inherit** it — they
do not get their own; the §12 merge guardrails are an *application* of this substrate (defeasible / consensus /
deontic + the rights instruments as the non-derogable set), **not a new mechanism**. From **STELLAR §0** + the
project's principle memories; constraints on every direction of the engine, not features:

- **Wisdom out-of-band** (the top rail, §5) — the engine never authors wisdom; D→I→K only.
- **Deontic + standpoint gate on every sense and every render.** Sensing is power: a system that ingests
  microphones and radios is, ungoverned, a surveillance engine — the *exact inverse* of this project. So
  sensing is legitimate only as **the person sensing their own environment, with consent, under the gate**
  (peace-infra: medical, environmental, accessibility), never covert watching of others.
- **Asymmetric accountability** — transparency UP (public power/funds), privacy DOWN (persons, esp.
  vulnerable). The same engine that *could* surveil is the one that makes *institutional* sensing inspectable
  and *shields the natural person*.
- **Agency, not sovereignty; identifier, not identity.** Meaning is resolved against the manifold + context;
  the token is never the self.
- **Self-custody — never hold the user's keys.** Biometrics and sensed personal signal never leave the
  device; sensitive data lives in the credential-gated vault keyed to the person.
- **Store physics, map percept.** Truth layer is spectral (SPD / STFT-CQT) + linear amplitude + modulation;
  tone-mapping/clipping happens only at the last mile, per device + user preference (e.g. accessibility,
  Sanctuary mode). Decouples data sovereignty from any current display/speaker.
- **Curation Prime Directive** — machine *proposes* (`closeMatch`/`Proposed`), a signed human *attests*
  (`exactMatch`/`Attested`). This includes the **percept→fact** step (the `∫Ψ > τ` vigilance threshold is a
  proposal, not a verdict) and any generative output.
- **Zero-heap hot paths; fixed stride.** 48-byte NQuin / fixed 10D dims for *any* modality (no `Vec` of
  samples, no per-pixel RGB objects, no `String` on the hot path); one `wgpu` device; out-buffer hydration
  only.
- **Runs on the hardware people own — the affordability & honest-scope test (the objective, Timothy
  2026-06-22).** No design ships if it forces a person to trade *food for compute* or needs a **$150k+ server
  on the user's side**. Heavy passes (merge / fusion / training / consolidation, §12) run **once** on whatever
  capable hardware exists — a desktop, a solar-surplus node, a guild server — and are **distributed**, so the
  user's phone / Intel-iGPU / no-GPU machine only ever pays the **cheap zero-heap fold** over a pre-compressed
  base. And we are **honest that this does not replace datacenter-scale compute** — the value is *sovereignty,
  governance, provenance, and locality on hardware they control*, not raw capability.

---

## 9. Honest current state — and the gap to close first (no overclaiming)

From **STELLAR §E** (honest state), the **migration review**, and [`RENDERER_SURVEY.md`](RENDERER_SURVEY.md):

**What is real:**
- The *data* is genuinely 10D with real 3D — `Tensor10D{q,v,w,x,y,z,t,α,μ,σ}`, `SpacetimeCoord{x,y,z,t}`.
- `webizen-render` has 3D **scaffolding**: a 4×4 `view_projection` matrix, a look-at `SceneCamera`, PGA math,
  z-depth scaling.
- The 10D manifold *metric* is GPU-resident (`tensor_volume.wgsl` ports `Tensor10D::full_distance`).
- The acoustic/spectral *surface* is partly present (`portal_acoustic`, `portal_spectral`).

**What is NOT yet real (the gap):**
- The implemented output is the **~2.5D ambient particle field** (50k points, screen-space + z-for-depth).
  There is **no depth-stencil buffer, no mesh vertex/index geometry, and no `.obj`/`.stl`/OpenUSD import**.
  → **3D *assets* are not yet rendered.** This is an *output/renderer gap on a sound foundation*, **not a
  dimensional limit**.
- There is **no real audio/spectral DSP** yet — only contract types (`SpectralParams`, `AudioSpectralSheet`)
  and telemetry counters (migration review §3). The sense path is contract, not signal, today.
- **Duplication hazard:** the renderer exists **twice** — `webizen-render` (in webizen-browser, building)
  and `portal_*` (in `qualia-core-db`, building) — plus **orphaned** `crates/webizen-*` copies that are
  committed but **not in the workspace** (so they don't build and only drift). Consolidation is decided but
  not done.
- **GH-Pages demo is dark:** `docs/spatial.html` is wired (2 canvases, WebGPU present) but shows **"WASM
  Engine Required"** — gated on a WASM bundle that isn't loading (same bundle family as the LLM-demo
  breakage). This is the visible "disabled" symptom the user has been living with.
- **`wgpu 0.19.4` device-init bug** (`maxInterStageShaderComponents` → `requestDevice` fails on current
  Chrome) gates **both** the WASM LLM **and** the consolidated WASM render path.

---

## 10. The path to build (this *executes* Timothy's plan; it does not redefine it)

Sequenced from **STELLAR §E** closing steps + the **migration review §4**:

1. **Resolve the duplication → one `qualia-render` crate** (workspace member), based on `portal_*` (already
   in-engine, already has the wasm canvas path); **absorb** `webizen-render`'s unique pieces (scene-graph /
   `scene_contract`, native offscreen frame delivery, audio/spectral contract, glTF intent); **delete** the
   orphaned `crates/webizen-*` copies after salvage. Engine stays platform-agnostic (no `winit`/`tauri`/`rfd`
   /`dioxus`); the browser is an optional consumer.
2. **Fix the `wgpu` upgrade once** (defect #1) — unblocks WASM LLM *and* WASM render on the shared device.
3. **World-space 3D scene** — reuse `view_projection` + `SceneCamera` + PGA; **add** depth-stencil
   (occlusion), **mesh vertex/index** buffers, and **asset import**. `glb_bridge` (and `.obj`/`.stl`/OpenUSD)
   must take **`&[u8]` → `NQuin`** (no `std::fs`; the shell hands bytes down — migration §2.1).
4. **Physics of artefacts** — mass/material/momentum (`P` in the Manifold-Coordinate);
   `specialized_libs/physics_simulation`; PGA "refuses to contract" on bounding-box violation.
5. **Place/space/time** — GeoSPARQL + RCC-8 (`spatio_temporal.rs`) + Allen/LTL (`temporal_ltl.rs`).
6. **One projection, many views** — `project : 10D → target` enumerating 2D/3D/4D via the volume metric.
7. **Sense path (the input twin)** — SOSA/SSN sensor ingest → wave coordinates → percept→fact bridge;
   **microphone STFT/CQT first** (the readily-available band, real DSP); RF / Wi-Fi CSI later
   (SDR/permission-gated). Each under the §8 rails.
8. **Authoring** — design the enhanced `ns/ui` vocabulary (§7); land the **RDF → CBOR-LD → NQuin** bridge
   (`@context` expansion, task #8); SVG vector with spectral signatures; CML; LLM pilot.
9. **Re-light the demo** — fix the WASM bundle so a 10D/spectral scene visibly runs; **then** upgrade
   `qapps_specification.md` to the enhanced vocabulary; **then** update the GH-Pages demos.

---

## 11. What this unblocks

- **Immediately downstream:** the upgrade of [`docs/manuals/qapps_specification.md`](docs/manuals/qapps_specification.md)
  from 2D panes to manifold worlds (the enhanced `ns/ui` authoring vocabulary, §7).
- **Then:** implementation per §10 (the STELLAR §E / migration §4 sequence).
- **Then:** the demos can be honestly re-enabled — which is the precondition the user set before returning
  to the LLM→q42 work.

---

## 12. Ingest & convergence — how models and knowledge enter the manifold

How a GGUF / safetensor / MLX model — or any signed knowledge delta — enters the substrate. Builds on §3
(one substrate) and the ecosystem-of-parts; **governed by the §8 rails**. *(Status: design — task #12. The
LoRA / CRDT pieces have real code; the merge / fusion / consolidation pipeline does not.)*

**Implementation-math companion:** [`INGEST_PIPELINE_SPEC.md`](INGEST_PIPELINE_SPEC.md) carries the formal
math, the cryptographic structures, and the proposed policy-knob defaults — this section stays architectural.

### 12.1 Source = locator + integrity + signer (not the URI alone)

The source **URI** says *where* a delta came from — necessary provenance, but a **locator, not a trust
anchor** (the project's **identifier ≠ identity** law: a URI is a handle — spoofable, mutable, re-pointable;
the same URI can serve different bytes tomorrow, and a merge-hijack attacker hosts a backdoored model at a
plausible URI). The **source-of-record for a delta is a triple**:
- **content hash** — *what* (integrity; content-addressed),
- **signer DID** — *who* (authenticity; the §H guild trust gate),
- **retrieval URI + time** — *where / when* (provenance metadata).

The **signature gates the merge**; the URI is the *weakest-for-trust* of the three. (This is the acquisition
provenance triple — `sourceURL + retrievalDate + sourceContentHash` — applied to weights.)

### 12.2 The trust gate — no verified signer, no merge

Before any conflict logic: a delta whose signer cannot be cryptographically verified against a known / guild
DID is **rejected at the perimeter**. This is what stops [merge-hijack / backdoor
injection](https://arxiv.org/pdf/2505.23561) and [privacy-phishing
merges](https://arxiv.org/pdf/2502.11533) *before* they touch the substrate. Verification is an enumerated
state over multiple cryptographic identifiers (agent- and entity-centric).

### 12.3 Two convergence tracks

- **Track A — homogeneous (shared lineage).** Same base / architecture → weight-space **task-vectors** with
  magnitude-aware merging (DELLA / TIES / DARE) + **permutation alignment** (Git-Re-Basin / optimal transport)
  *before* adding. Aligns **hidden units** (weight space).
- **Track B — heterogeneous (different architecture / tokenizer).** No common `W_base` → **knowledge fusion**
  ([FuseLLM / FuseChat](https://arxiv.org/pdf/2408.07990) lineage; [*Can Heterogeneous Language Models Be
  Fused?*](https://arxiv.org/pdf/2604.01674), 2026): align **output distributions** + token mappings via
  lightweight continual training. Aligns **meanings / distributions** — i.e. the §B semantic-frame
  convergence. (Weight arithmetic does **not** apply here: there is no shared basis to subtract.)

Both ride the one hash-space; both retain DID provenance.

### 12.4 Additive, not cartridge — commutative folds over destructive baking

Deltas are retained as **signed, first-class commutative layers** (LoRA / CRDT) that **fold at enumeration
time** — never "summed into the base and forgotten" (destructive baking zeroes/drops weights and erases *who
contributed what*). The baked merge is a **derived, re-derivable artifact** — the CML *SOURCE-vs-GENERATED*
split applied to weights; the **signed deltas remain the source of record**. The CRDT join is over a
**content-addressed set of signed delta-IDs** (so it is properly *idempotent* — a re-gossiped or duplicated
delta is applied once, never double-counted): `W_active = W_base ⊕ Σ(unique signed deltas)`, evaluable
out-of-order across shards without drift. *(Track A deltas and LoRA adapters fold this way; **Track B fusion
requires training**, so it produces a new consolidated base — AOT only, §12.5 — not a cheap JIT fold.)*

### 12.5 Static base + dynamic folds + periodic consolidation

- **Dynamic (hot path):** cheap, zero-copy, reversible fold of signed adapters at enumeration — how the
  manifold **grows continuously**. Zero-heap.
- **Static (AOT):** periodically, **consensus-approved** adapters are baked into the base
  (dequantize → align → merge/fuse → requantize to the CBOR-LD schema) — a **CRDT compaction / WAL
  checkpoint**, off the hot path. Signed deltas retained.
- **High-fidelity ingest:** **never lift a Q4** (the loss is already baked in); mandate **F16 / Q8** sources
  so folds and merges do not compound precision degradation.

### 12.6 Conflict resolution — adjudicated, not averaged

When a newly-signed delta conflicts with a **high-confidence value already in the base**, the engine **does
not silently overwrite** (silent overwrite = erasure, and the merge-hijack vector). It **adjudicates** using
existing machinery, and the **default verdict is suspend-in-superposition + surface**:

1. **Trust gate** (§12.2) — unverified → rejected before adjudication.
2. **Non-derogable check** — if the established value is an *attested non-derogable baseline* (encoded prior
   human wisdom — e.g. a rights floor), **no delta can defeat it** → quarantine the delta, flagged; never
   fold. (Also the protected-value merge-hijack defense.)
3. **Classify the defeat** (`DefeatKind`): *rebut* (contradicts the conclusion) vs *undercut* (attacks the
   support).
4. **Weigh evidence, not argmax** — α (confidence) × independent corroboration count (consensus of *distinct*
   signers) × signer / provenance tier × recency (`t`). A **ranking**, never an automatic winner (the TIES
   sign-election error: majority-sign can erase a minority-but-correct signal).
5. **Act by stakes × dominance:**
   - concordant / refining → **fold dynamically**;
   - low-stakes & clearly evidence-dominant → **fold, but retain the superseded value in the `t`-ledger**
     (reversible);
   - **conflicts with a high-confidence base value → land at `q>0` (escrow superposition); keep both; raise a
     conflict flag; collapse only on a consensus quorum (N independent signed corroborations) OR human
     attestation.** ← the wisdom-out-of-band line.
6. **Collapse on resolution** — attest / quorum → promote to `q=0`, increment `t` (audit of *when / why* it
   changed), retain the prior value + the signed deltas (accountable, reversible).

**Policy knobs are Timothy's — value choices, not mechanism, set by attestation:** the stakes threshold that
forces human review; the α-dominance ratio; the consensus quorum size; and which base values are
non-derogable. The engine supplies the **mechanism** and a proposed default; the **policy** is attested
out-of-band. Wisdom stays with the person — even for the rule that resolves conflicts.

---

### 12.7 The merge log — a model "birth record" (provenance, to the right extent)

A merge **emits a cryptographically signed derivation record** — in effect a *birth record* for the offspring
model, and the connective tissue between the CRDT lineage DAG (§12.3–§12.4) and the project's **PROV-O /
DigitalBirthRecord** layer. It is itself an NQuin-graph citizen (queryable by the same modalities), recording:

- **parents** — `prov:wasDerivedFrom` each source delta / base, by **DID + content hash**;
- **recipe (the "genome")** — `prov:wasGeneratedBy` the merge activity: track (A/B), permutation map / λ
  coefficients (A) or KL weights + calibration set (B), DELLA threshold, etc.;
- **attestation** — `prov:wasAttributedTo` the authorizing agent (who consented to the union and is
  **answerable** for the offspring);
- **identity** — resulting content hash, `t`, and the `q`-state it was admitted at (`q>0` sandbox vs `q=0`
  authoritative).

This is what makes lineage auditable *from its origin identifier*: you can ask any merged model *who are your
parents, what combined to make you, who authorized it.* Repeated merges form a **family tree** — which is
exactly the framing of evolutionary model-merging (recipes as genomes; selection by governance / consensus).

**The honest extent — what the birth record does and doesn't give.** It gives **genealogical provenance**: a
verifiable family tree, the recipe, and the authorization — strong and real. It does **not** by itself give
**behavioral attribution** (which ancestor caused a specific output): the DAG preserves *which delta touched
which coordinate* (structural lineage), but *which parent is responsible for an emergent behavior* is an
interpretability problem, only partially recoverable. "Provenance to some extent" is exactly right — the
extent is **lineage + recipe + authorization**, not per-behavior causation.

**Tool, not person.** The "two models having a child" metaphor illuminates the *lineage structure*; it does
not grant the offspring personhood. Parents and child are **artifacts**; the **consent to create** and the
**responsibility for** the offspring vest in the human / entity agents behind the attesting DIDs (agency + the
wisdom-out-of-band rail). The machine proposes the union; a person attests the birth.

---

## Appendix — authoritative source specs (Timothy's; not to be re-derived)

- [`STELLAR_MISSION.md`](STELLAR_MISSION.md) — the primary spec home. **§D** multimodal-as-physics (the wave
  substrate, EMF≠acoustic, SOSA/SSN, percepts-enumerated, Wi-Fi CSI); **§E** the manifold renderer
  (projection, PGA, CAD-as-constraint, photogrammetry, OpenUSD, the honest ~2.5D state + the closing steps);
  **§F** the compute-universe fabric (`compute_universe.rs`, one device, graph–tensor duality); **§G**
  heterogeneous compute (CPU/GPU/NPU/QPU); **§A** transcode-to-manifold-native; **§C** the 10D→5D NQuin;
  **§0** governance rails.
- [`10d/q42-10d-volumetric-tensor-spec.md`](10d/q42-10d-volumetric-tensor-spec.md) — the 10D tensor
  `[q,v,w,x,y,z,t,α,μ,σ]`, the spectral-logical payload `[α,μ,σ]`, gravito-thermodynamic operators, the
  hardware tiers + GSR, the 64-opcode zero-heap VM, the ingest/bake pipeline. Companion standard:
  [`docs/manuals/standards/q42-10d-tensor-standard.md`](docs/manuals/standards/q42-10d-tensor-standard.md).
- [`20260621_webizen-browser-engine-migration-review.md`](20260621_webizen-browser-engine-migration-review.md)
  — the consolidation decision (one `qualia-render`; engine never depends on the browser; `glb_bridge` as
  `&[u8]`→NQuin; lift the kernel abstraction; de-dup the 10D/Quin types; the duplication hazard + sequence).
- [`RENDERER_SURVEY.md`](RENDERER_SURVEY.md) — where the code currently sits, verified, for status.
- Sonic plane: `docs/manuals/adr/0007-u3-acoustic-plane-symbolic-audio.md` +
  `docs/manuals/standards/q42-acoustic-plane-draft.md`.
- Principle memories: `principle-dikw-wisdom-out-of-band`, `principle-identifiers-not-identity`,
  `feedback-no-authorship-claim`, `feedback-terminology-agency-not-sovereign`.
