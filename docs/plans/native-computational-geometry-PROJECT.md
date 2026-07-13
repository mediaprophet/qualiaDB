# Native Computational Geometry & the `.10d` Living Container — Project Charter

**Status:** project charter (2026-07-04).
**Design (the how):** [`native-computational-geometry.md`](native-computational-geometry.md).
**Tracked execution (the where-are-we):** [`native-computational-geometry-EXECUTION.md`](native-computational-geometry-EXECUTION.md).
**Progress log (§9):** [`native-computational-geometry-PROGRESS-LOG.md`](native-computational-geometry-PROGRESS-LOG.md).

## 1. What this project is

One project, four coherent deliverables, all native to the QualiaDB / Webizen engine:

1. **The `.10d` living container** — the on-disk form of the 10-D volumetric tensor
   `[q, v, w, x, y, z, t, α, μ, σ]`. Not a static asset: a self-describing, append-only, witnessed,
   rights-bearing record in which certainty (`q`), rights-physics (`v`), knowledge-domain (`w`), time (`t`),
   and spectral truth (`α, μ, σ`) are *coordinates in the same record as position*, not sidecar metadata.
2. **The native computational-geometry substrate** — the geometric-operations engine of the 10-D tensor,
   across *all* its faces: the spatial geometry of `x,y,z`; the spectral/EMF geometry of `α,μ,σ` (visual and
   audio); and the probability/information geometry of `q` and inference. A robust, deterministic, GPU- and
   WASM-capable kernel with the algorithm families the ecosystem actually needs.
3. **The API & creation surface** — the ergonomic, browser-native, renderer-driven layer through which people
   *make things* — spatial, spectral, probabilistic, geometric — in qapps and in code, and keep the result as
   their own provenance-bearing `.10d` asset. A sovereign maker platform, categorically not a viewer.
4. **The browser / renderer path** — the 10-D display engine wired through to a browser WebGPU surface, so the
   whole thing runs on hardware people own, offline.

These are one project because they are one substrate seen from four sides: the geometry engine operates on the
tensor, writes into `.10d`, is exposed through the creation surface, and is drawn by the renderer.

## 2. Why it exists — its place in the broader body of work

The 10-D tensor is the substrate on which the ecosystem reasons — semantics, provenance, rights, spectra, and
epistemic state unified into one manifold. Computational geometry is that substrate's **geometric-operations
engine**: "queries become geometric projections, distances, temporal slices, context collapses, and spectral
blends." `.10d` is its **on-disk form**. This project turns those from a design promise into a running,
attestable capability.

It composes with, and serves, the rest of the work:

- **WellFair / the 3D Anatomy Qapp** is the first real consumer — a whole-person, systemic, accumulative view
  that maps records onto a native-rendered body over time. Its needs (LOD decimation, spatial picking,
  temporal recovery trajectories, rights-bounded rendering of restricted health data, systemic *proposals not
  diagnoses*) are the concrete pull that keeps this substrate honest.
- **Q42** (the semantic graph / identity / rights control plane) and **P64** (the weight container) are
  siblings, not parents: `.10d` links to Q42 identity and is kept distinct from P64 (geometry is never
  disguised as model weights).
- **The gravito-thermodynamic baking**, the **Ground-State Resolver**, the **native renderer**, and the
  **Permissive Commons** all draw on this substrate — baking needs density-aware manifold construction, the
  commons needs content-addressed, rights-gated, patchable assets, and the renderer needs the `.10d` geometry.
- Above all it serves **human-centric AI**: sovereignty, freedom of thought, dignity, offline-first, no silent
  egress. A person makes and keeps their own knowledge body; the software does not take it.

## 3. What makes it categorically different

Static mesh containers answer one question — *what does this look like* — and hold "is." `.10d` holds
*becoming, knowing, and meaning*. It is append-only and time-scrubbable rather than overwritten; it holds
superposed what-if geometry beside verified fact and re-crystallizes hypotheses on a dated ledger slice; it
stores the spectral *cause* and projects to colour **or** sound at the last mile; it carries confidence as
gravitating mass that self-organizes the volume; it welds provenance and consent into the signal itself; and
it **defaults to refuse** — the file enforces its own rights policy at draw time rather than trusting the host
to. *A static mesh file is a photograph of a shape; a `.10d` is a signed, time-witnessed, rights-bearing
logbook of a knowledge body.* The defining capabilities and their format requirements are specified in
[`native-computational-geometry.md`](native-computational-geometry.md) §4.1.

## 4. Tools and references — not parents

The synthesis, the axes, the formats, and the substrate are this project's own. External work is used the way
an inventor uses a transistor — as an instrument, transferring no authorship:

- **CGAL** is a public-domain **capability reference**: its CC0 documentation is a specification and its CC0
  test suites are golden-output oracles. Its GPL/LGPL algorithm *source* is never copied or derived from. This
  is a native implementation on this engine, **substantially different** from CGAL — not a port or
  transliteration.
- **HDF5 / NetCDF-LD** are structural influences the container work references; **three.js / Draco /
  meshoptimizer** are the foils and reference algorithms the creation surface and codecs measure against.

None of these are the baseline this work "extends." They are cited only as contrast or reference, never as the
lineage that owns the result.

## 5. Governance, honesty, and rights

Load-bearing invariants, not aspirations:

- **Determinism → attestability.** Geometric *decisions* (predicate signs) are exact and platform-independent,
  so a geometry result and a `.10d` file are hash-stable and can carry a provenance receipt into the WAL.
- **Rights are structural.** Consent, sensitivity, and deontic policy live *in the data* and **fail closed**;
  a derived/decimated view inherits the most restrictive class of its sources.
- **Fidelity ≠ assurance.** A heavier geometric or physical computation never labels itself "safe,"
  "certified," or "clinically valid" without the evidence and competent-human review that class requires.
- **Honesty of status.** Every capability is marked *implemented-in-code* vs *spec-reserved*, and every task
  *done / foundation / planned*; measured numbers are real, with their caveats, never extrapolated.
- **Sovereign & offline-first.** No silent egress; qapp-authored creations are the maker's own, with their
  licence and lineage recorded, never silently the platform's.
- **House rules.** Zero-heap / caller-buffered hot paths; the 42-MiB Sentinel per pass; CC0-only reference
  material; per-step dated progress logging (PROJECT RULE §9).

## 6. Definition of done

The project is complete when:

1. `.10d` faithfully and attestably **serializes the living 10-D runtime** — geometry, epistemic state, rights,
   spectral truth, and the append-only temporal ledger — with a published, honest axis-role taxonomy and
   canonical deterministic bytes.
2. The **geometry substrate covers its capability set** — kernel, topology, spatial index, 2-D/3-D algorithms,
   reconstruction/meshing, and the spectral-operator and TDA/information-geometry families — each validated
   against CC0 golden oracles including degeneracies, with CPU/GPU/WASM parity.
3. A person can **make a 3-D / spatial / spectral thing** in a qapp or the browser on this stack, watch it move
   on the 10-D display engine, and **keep it as their own provenance-bearing `.10d` asset**.
4. The WellFair anatomy body renders natively from `.10d`, coloured by systemic burden over time, picking to
   contributing factors, rights-bounded — the first consumer proving the whole path end-to-end.

This is a large body of work. It advances in dependency-ordered phases (see the execution plan), each
independently reviewable, each landing tested and honestly logged, with the curation decisions that are
Timothy's to make surfaced as concrete asks rather than assumed.
