# Act VII — Render & Volumetric

> *The engine shows itself. In ten dimensions.*

---

## Thesis

> **The engine does not just reason. It renders. The volumetric renderer
> takes a 10-dimensional manifold coordinate per node, projects it to a
> spectral color, and paints it through `wgpu` — on the same device as the
> inference loop.**

---

## Voice-over script

### Shot 1 — A blank canvas. A node graph appears in the center. [SLOW]

> This is the volumetric renderer. [PAUSE]
> It is `webizen-render`. [PAUSE]
> It runs on `wgpu` version twenty-nine. [PAUSE]

### Shot 2 — A node is given a `Tensor10D`. The renderer projects it. [SLOW]

> Every node has a ten-dimensional coordinate. [PAUSE]
> Ten floats. Ten axes. [PAUSE]
> The renderer projects the coordinate to a spectral color. [PAUSE]
> The projection is deterministic. The same coordinate always gives the
> same color. [PAUSE]

### Shot 3 — The spectral color is shown on screen. It is the engine's "spectral oracle." [SLOW]

> The projection is the engine's spectral oracle. [PAUSE]
> It is not a stock LUT. It is a hand-derived CIE-XYZ-to-sRGB pipeline,
> with amplitude-to-opacity. [PAUSE]

### Shot 4 — Edges are added. They are depth-tested against the nodes. [SLOW]

> Edges are added. [PAUSE]
> They are depth-tested against the nodes. [PAUSE]
> They are not opaque lines drawn on top; they are part of the scene. [PAUSE]

### Shot 5 — Faces are added. They form a mesh. The mesh is depth-tested. [SLOW]

> Faces are added. [PAUSE]
> They form a mesh. [PAUSE]
> The mesh is depth-tested. [PAUSE]
> The result is a volumetric scene, not a 2D drawing. [PAUSE]

### Shot 6 — The scene is rendered to an RGBA8 buffer. The buffer is read back. [SLOW]

> The scene is rendered to an RGBA8 buffer. [PAUSE]
> The buffer is caller-owned. [PAUSE]
> The renderer writes into it. The caller reads from it. [PAUSE]
> No allocation in the hot path. [PAUSE]

### Shot 7 — The same scene is rendered to a PNG. The PNG is encoded. [SLOW]

> The same scene can be rendered to a PNG. [PAUSE]
> The PNG encoder is in the same crate. [PAUSE]
> The encoding is deterministic. [PAUSE]

### Shot 8 — The audio contract is shown. The same scene has a `GenerativeAudioSheet`. [SLOW]

> The same scene has an audio contract. [PAUSE]
> A `GenerativeAudioSheet` describes the sounds the scene makes. [PAUSE]
> The spectral parameters map directly to the visual spectral oracle. [PAUSE]
> Visual and audio are derived from the same coordinates. [PAUSE]

### Shot 9 — A camera orbits the scene. The orbit is smooth. The depth is consistent. [SLOW]

> The camera can orbit. [PAUSE]
> The orbit is smooth. [PAUSE]
> The depth is consistent. [PAUSE]
> The scene is alive. [PAUSE]

### Shot 10 — The telemetry HUD pulses. Memory pressure. Network ripple. Baking crystallization. Logic flashes. Inference heat. Quantum activity. Spectral shift. Temporal pulse. Epistemic density. Manifold pressure. [ITEM]

> The renderer is wired to the engine's telemetry. [PAUSE] [ITEM]
> Memory pressure. [PAUSE] [ITEM]
> Network ripple. [PAUSE] [ITEM]
> Baking crystallization. [PAUSE] [ITEM]
> Logic flashes. [PAUSE] [ITEM]
> Inference heat. [PAUSE] [ITEM]
> Quantum activity. [PAUSE] [ITEM]
> Spectral shift. [PAUSE] [ITEM]
> Temporal pulse. [PAUSE] [ITEM]
> Epistemic density. [PAUSE] [ITEM]
> Manifold pressure. [END LIST] [PAUSE]
> The scene reflects the engine's state. [PAUSE]

### Shot 11 — Title card: **Ten dimensions. One renderer.** [SLOW]

> Ten dimensions. [PAUSE]
> One renderer. [PAUSE]
> The engine shows itself. [PAUSE]

---

## On-screen notes

- **Shot 1:** A blank canvas. The node graph fades in.
- **Shot 2:** A node is highlighted. Its `Tensor10D` is shown as a vector of ten floats. The projection is animated.
- **Shot 3:** The spectral color is shown. The CIE-XYZ-to-sRGB pipeline is hinted at with a small diagram.
- **Shot 4:** Edges are drawn. The depth buffer is shown briefly.
- **Shot 5:** Faces are drawn. The mesh is visible. The depth test is visible.
- **Shot 6:** The RGBA8 buffer is shown as a flat array of bytes. The renderer writes into it. The caller reads from it.
- **Shot 7:** A PNG is shown. It is the same scene.
- **Shot 8:** The audio contract is shown. A waveform is played.
- **Shot 9:** The camera orbits. The scene is alive.
- **Shot 10:** The telemetry HUD pulses. Each metric is a small bar or sparkline.
- **Shot 11:** Title card.

---

## Source code anchors

- `crates/webizen-render/src/volumetric.rs` — `VolumetricRenderer`, `render_scene_rgba8_into`, `read_rgba8_into`, `render_scene_png`.
- `crates/webizen-render/src/scene_contract.rs` — `Tensor10DProjection`, `spectral_to_color`, `spectral_to_cie_xyz`, `cie_xyz_to_srgb`, `amplitude_to_opacity`, `has_hidden_metadata`.
- `crates/webizen-render/src/scene.rs` — `RenderScene`, `SceneNode`, `SceneEdge`, `SceneFace`, `SceneCamera`.
- `crates/webizen-render/src/wgpu_renderer.rs` — the wgpu 29 device, shared with the inference loop.
- `crates/webizen-render/src/audio_contract.rs` — `GenerativeAudioSheet`, `PCMAudioSheet`, `map_tensor_to_spectral`.
- `crates/webizen-render/src/telemetry.rs` — the ten telemetry channels.
- `crates/qualia-core-db/src/render/` — the canonical draw graph and device ABI.
- `crates/qualia-core-db/src/render/projection.rs` — `project`, `one_projection_many_views`.
- `crates/qualia-core-db/src/render/pga.rs` — Projective Geometric Algebra motors, `tensor_deontic_lane`, `bilateral_pull_active`.
- `crates/qualia-core-db/src/render/sense.rs` — `pack_percept`, `unpack_percept`, `perceived_fact_quin`.
- `crates/qualia-core-db/src/render/acoustic.rs` — `phenomenal_acoustic_params`, `sigma_to_wavelength_nm`, `sigma_to_center_frequency_hz`.
- `crates/qualia-core-db/src/render/model_substrate.rs` — `compose_substrate`, `project_manifold`, `build_model_substrate`.
- `crates/qualia-core-db/src/render/navigation.rs` — `CameraFlyTo`, `cpu_pick_node_at`.
- `crates/webizen-desktop/src/telemetry_bridge.rs` — the desktop telemetry bridge.
- `crates/webizen-desktop/src/telemetry_hooks.rs` — the desktop telemetry hooks.
- `AGENTS.md §7` (2026-06-30 session) — cross-platform volumetric renderer SDK.

---

## Duration

Approximately 120 seconds. This is the act where the engine shows itself.
