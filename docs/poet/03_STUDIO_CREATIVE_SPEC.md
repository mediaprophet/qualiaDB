# POET Creative Studio, Audio & Spatial Media Specification

**Document ID:** `POET-SPEC-003`  
**Status:** Canonical Domain Specification  
**Scope:** Audio synthesis, multi-track mixing, 3D/10D scene graphs, WGSL shaders, and the Dual Studio environment in POET.

---

## 1. Overview & Creative Workspace Topology

The POET Creative Studio provides an integrated spatial environment for sonic composition, 3D geometry manipulation, shader development, and real-time interactive computing.

```
+-----------------------------------------------------------------------------------+
|                            STUDIO WORKSPACE TOPOLOGY                              |
+-----------------------------------------------------------------------------------+
|  [Audio Studio & Channel Strips] <===> [3D / 10D Spatial Scene Graph]             |
|  - Multi-track volume faders           - Scene hierarchy & parent-child nodes     |
|  - Pan, EQ, Synth parameter knobs      - Transform gizmos (translate, rotate)     |
|  - Transport controls (Play/Stop/Rec)  - Lighting, materials, camera orbits       |
|                                                                                   |
|  [WGSL Shader & Forge Runner]    <===> [Dual Studio Synchronized Viewport]        |
|  - Naga-validated WGSL pipelines       - Live VibeScript / Rust code editor       |
|  - Zero-heap GPU execution             - 60 FPS real-time animated player         |
|  - Real-time latency telemetry         - AST 3-way merge and parameter scrubbing  |
+-----------------------------------------------------------------------------------+
```

---

## 2. Audio Studio & Multi-Track Channel Strips

The Audio Studio provides a visual, tactile mixing desk:

### 2.1 Channel Strip Component
Each audio channel strip provides:
- **Volume Fader:** Vertical slider with visual dB meter (-∞ to +6 dB) and peak indicator.
- **Pan Knob:** Rotary control (-100% Left to +100% Right) with center detent.
- **Parametric EQ:** 3-band frequency knobs (Low, Mid, High) with gain adjustment.
- **Channel State:** Dedicated `Mute` (M) and `Solo` (S) toggle buttons.
- **Source Selector:** Input routing from synth oscillators, audio samples, or live input capture.

### 2.2 Master Transport Controls
- Global Play, Pause, Stop, Record, and Loop toggle buttons in the studio toolbar.
- Time signature, BPM tempo selector (20–300 BPM), and metronome toggle.
- Live waveform / spectrum visualizer rendering current audio output at 60 FPS.

---

## 3. 3D & 10D Spatial Scene Graph

The spatial workspace enables authoring of complex multi-dimensional scenes:

### 3.1 Scene Hierarchy Tree
- Collapsible tree view showing scene nodes, meshes, lights, cameras, and particle emitters.
- Drag-and-drop parenting and reordering of scene graph entities.
- Node visibility toggle (`👁`) and lock toggle (`🔒`).

### 3.2 Viewport & Transform Gizmos
- 3D perspective / orthographic viewport powered by `wgpu` and WebGL2.
- Interactive on-canvas transform gizmos for **Translation** (XYZ axes), **Rotation** (Euler rings), and **Scale** (uniform/axial).
- Smooth orbit, pan, and zoom camera navigation with preset views (Top, Front, Side, Isometric).

### 3.3 Materials & Lighting Editor
- Material property controls: Base Color, Metallic, Roughness, Emissive, and Ambient Occlusion.
- Light source configurations: Directional, Point, and Ambient lights with intensity and shadow controls.
- **10D Manifold Projection:** Mathematical projection sliders mapping 10-dimensional topological manifolds onto 3D visible representations.

---

## 4. Dual Studio & WGSL Shader Pipeline

Dual Studio bridges code and immediate visual feedback:
- **Synchronized Split View:** Left pane features a syntax-highlighted VibeScript/WGSL editor; right pane renders the live 60 FPS viewport.
- **Parameter Scrubbing:** Dragging numbers in code immediately updates the live viewport parameters.
- **WGSL Shader Pipeline:** Live compilation and execution of Naga-validated WGSL shaders on the GPU with frame time metrics and zero-heap verification.

---

## 5. Studio Requirements

| Requirement ID | Title | Description | Target Component |
|---|---|---|---|
| `POET-STU-001` | **Multi-Track Audio Channel Strips** | Visual channel strips with vertical volume faders, dB peak meters, pan knobs, EQ, mute, and solo. | `channel_strip.rs`, `audio_synth.rs` |
| `POET-STU-002` | **Master Audio Transport** | Master transport bar with Play/Pause/Stop/Record, BPM tempo control, and waveform spectrum visualizer. | `transport.rs`, `meter_bridge.rs` |
| `POET-STU-003` | **3D Scene Hierarchy Explorer** | Interactive scene graph tree with node selection, drag-and-drop parenting, and visibility toggling. | `scene_graph.rs`, `scene_view.rs` |
| `POET-STU-004` | **Interactive 3D Transform Gizmos** | Viewport gizmos for translation, rotation, and scaling of selected 3D scene objects. | `scene_view.rs`, `spatial_10d.rs` |
| `POET-STU-005` | **Material & Lighting Inspector** | Property editor for PBR materials (roughness, metalness, color) and light source parameters. | `material_editor.rs`, `lighting_editor.rs` |
| `POET-STU-006` | **10D Manifold Projector** | Visual projection controls translating 10D tensor manifolds into 3D renderable surfaces. | `spatial_10d.rs`, `tensor_inspector.rs` |
| `POET-STU-007` | **Dual Studio Live Environment** | Synchronized code editor + 60 FPS viewport with AST parameter scrubbing and hot reload. | `dual_studio.rs` |
| `POET-STU-008` | **WGSL Shader Forge Pipeline** | Interactive WGSL shader editor with Naga compilation, GPU execution, and latency telemetry. | `shader_pipelines.rs` |
| `POET-STU-009` | **Spatial Audio & HRTF Control** | 3D audio listener positioning and binaural HRTF personalization parameters. | `spatial_audio.rs`, `hrtf_personalization.rs` |
