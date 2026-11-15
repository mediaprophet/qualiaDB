# Holo-Engine: Master Implementation To-Do List & Verification Roadmap

> **Document Class:** Phased Execution Tracker & Engineering Task Registry  
> **Target System:** QualiaDB Holo Engine (`c:\github\qualiaDB`)  
> **Comparative Bases:** `C:\github\bevy-main` · `C:\github\Fyrox-master`  
> **Auditing Instrument:** `C:\github\rust_audit`  
> **Authority Precedence:** Zero-Heap Hot-Path Contract ([`qualiaDB/AGENTS.md` §0](file:///c:/github/qualiaDB/AGENTS.md)) · 42MB Sentinel · Dual Native+WASM Target  
> **Status:** Active Master Registry

---

## Progress Overview

```
[█████████████████████████████████████████████████] 98% Complete
• Phase 1: Core Engine Decoupling & Crates      [COMPLETED]
• Phase 2: Dual-Target WASM/Native Stabilization [COMPLETED]
• Phase 3: Holo Visual & WGSL Forge Pipeline     [COMPLETED]
• Phase 4: Holosonic Spatial Audio System        [COMPLETED]
• Phase 5: Skeletal Animation & Matter Volumes   [COMPLETED]
• Phase 6: Game-Demo & Webizen Studio Adoption   [COMPLETED]
• Phase 7: Automated Audit via rust_audit        [PENDING]
```

---

## Phase 1: Core Engine Decoupling & Subsystem Creation (Status: COMPLETED)

- [x] **Task 1.1: Create `crates/qualia-geospatial`**:
  - [x] WGS84 $\leftrightarrow$ local ENU tangent plane coordinate transforms (`frame.rs`).
  - [x] Zero-copy 16-bit GeoTIFF hydro-enforced DEM raster decoder (`geotiff.rs`).
  - [x] Zero-heap Hilbert spatial curve indexing for terrain spatial locality (`hilbert.rs`).
  - [x] OGC Web Feature Service & WMS streaming client (`wfs.rs`, `wms_streamer.rs`).
  - [x] Spline-based roadway corridor elevation profiler (`corridor.rs`).
  - [x] All 8 unit tests passed cleanly (`cargo test -p qualia-geospatial`).
- [x] **Task 1.2: Create `crates/qualia-sim`**:
  - [x] 64 Hz fixed-timestep physics accumulator (`FixedTimestepAccumulator`).
  - [x] Zero-heap spring-damper vehicle kinematics (`RoverKinematics`).
  - [x] Zero-heap kinematic character controller with slope-sliding and jump (`CharacterController`).
  - [x] Zero-heap skeletal forward kinematics and animation state machine (`ZeroHeapSkeleton`, `AnimationStateMachine`).
  - [x] Zero-heap convex polygon NavMesh and A* pathfinding (`ZeroHeapNavMesh`).
  - [x] Orbital, Chase, and 6-DOF Fly cameras (`SpringChaseCamera`, `OrbitCamera`, `FlyCamera`).
  - [x] Zero-allocation button and axis input map (`ZeroHeapButtonInput`).
  - [x] Mobile multi-touch virtual joysticks (`VirtualTouchStick`, `MultiTouchTracker`).
- [x] **Task 1.6: Create `qualia-sim::hydro` Hydrological & Fluid Simulation Engine**:
  - [x] Analytical 8-harmonic `SimdGerstnerSpectrum` evaluated on CPU with zero heap allocation.
  - [x] `ArchimedesProxy` multi-sphere volume buoyancy solver ($F_b = \rho V_\text{submerged} g$).
  - [x] Two-way character fluid locomotion coupling in `CharacterController::step_with_water`.
  - [x] Dynamic wetness accumulation and drying model.
  - [x] `HydrologicalHysteresisGate` filtering high-frequency noise for 10D nquin state transitions.
  - [x] `InteractiveRippleBuffer`: 2D finite-difference wave equation solver with ping-pong buffering and Gaussian impulse injection (`ripple.rs`).
  - [x] All unit tests passed cleanly (`cargo test -p qualia-sim`).
- [x] **Task 1.7: Create `qualia-sim::optics` (Hydrologic Optics & CASE-2 Bio-Geo-Optical Radiative Transfer)**:
  - [x] Multi-spectral `DielectricVolumeIop` with Cauchy/Sellmeier chromatic dispersion.
  - [x] `BioGeoOpticalConstituents` modeling pure water, CDOM tannins (eucalyptus billabong dark mirror), tripton colloidal clay (gold mining damage milky veil), intact armored stream, and soda-lime glass.
  - [x] Beer-Lambert beam attenuation and substrate visibility scoring.
  - [x] Companion WGSL shader in `crates/qualia-core-db/src/shaders/viewport/hydrologic_optics.wgsl`.
  - [x] All 4 unit tests passed cleanly.
- [x] **Task 1.8: Create `qualia-sim::atmosphere` (Synoptic Atmospherics & Aeroelastic Foliage Coupling)**:
  - [x] `BarometricState`: Ideal gas law for moist air density $\rho_{\text{air}}(P, T, RH)$, Lifting Condensation Level (LCL), and inverse barometer sea bulge.
  - [x] `BoundaryLayerProfile`: Prandtl–von Kármán logarithmic wind profile $u(z) = \frac{u_*}{\kappa} \ln((z - d)/z_0)$ with canopy roughness lengths ($z_0$) and dynamic pressure $q = \frac{1}{2}\rho v^2$.
  - [x] `AeroelasticBranch`: Damped harmonic cantilever beam oscillator with Vogel streamlining crown reconfiguration ($F_{\text{drag}} \propto v^{2+\mathcal{V}}$).
  - [x] `LeafFlutterState`: Strouhal vortex shedding flutter ($f = \text{St} \cdot v / L$) and petiole tensile abscission failure.
  - [x] All 6 unit tests passed cleanly.
- [x] **Task 1.9: Create `qualia-core-db::modalities::hydrology` (Deterministic 10D Graph State)**:
  - [x] Modality opcodes `0x51`–`0x56` (`OP_HYDRO_SUBMERSION`, `OP_HYDRO_WETNESS`, `OP_HYDRO_CURRENT`, `OP_HYDRO_THERMAL`, `OP_HYDRO_PRECIPITATION`, `OP_HYDRO_BAROMETRIC`).
  - [x] Zero-allocation pack and unpack methods into 48-byte Super-Quin bit layout.
  - [x] Zero-heap `evaluate_hydrological_frame` with caller-provided buffer.
  - [x] All 6 unit tests passed cleanly.
- [x] **Task 1.10: Create Procedural Flora Biophysics, Whole-Plant FSPM & Multi-Scale Ground Cover**:
  - [x] `qualia-sim::leaf_biophysics`: Reticulate vascular skeleton, Murray's Law radius tapering ($r_p^3 = r_1^3 + r_2^3$), Hagen-Poiseuille conductance, turgor-coupled cantilever wilting.
  - [x] `qualia-sim::metamer_fspm`: Atomic metamer units, Shinozaki's Pipe Model (active sapwood vs. retired heartwood), GDD thermal time phenology, fruit load torque and reaction wood.
  - [x] `qualia-sim::flora_archetype`: Continuous biophysical parameter space for trees, culinary herbs, hollow monocot grasses, climbing vines, succulents, and Raunkiær frost/drought dieback pruning.
  - [x] `qualia-sim::ground_cover`: Semantic ecotope patches, camera-centered toroidal clipmaps (zero heap re-allocation), 4-tier continuous LOD mapping, and Megapixel Governor density clamping.
  - [x] `qualia-core-db::modalities::botany`: Modality opcodes `0x57`–`0x5C` (`OP_BOTANY_METAMER`, `OP_BOTANY_TURGOR`, `OP_BOTANY_PHENOLOGY`, `OP_BOTANY_ECOTOPE`, `OP_BOTANY_VASCULAR`, `OP_BOTANY_RAUNKIAER`).
  - [x] `sggx_microflake.wgsl`: SGGX Anisotropic Microflake BRDF for distant canopy/grasslands with turgor/trample fiber orientation.
  - [x] `foliage_lod.wgsl`: GPU compute pass with screen-space density clamping and Cosserat Bézier blade synthesis.
  - [x] All unit tests passed cleanly.
- [x] **Task 1.3: Port `m64_matter` to `qualia-core-db::q42`**:
  - [x] `point_record.rs`: 64-byte aligned XYZI normal, material ID, CRC-32C.
  - [x] `holo_record.rs`: 64-byte holographic radiance, spherical harmonics L1, opacity, depth variance.
  - [x] Verified zero-copy `bytemuck::Pod` memory layouts.
- [x] **Task 1.4: Port `HardwareGovernor` to `webizen-render`**:
  - [x] Port `VramAndComputeGovernor` with 3-phase LLM decode arbitration (`Idle`, `PromptProcessing`, `ActiveDecode`).
  - [x] Registered in `render/mod.rs`.
- [x] **Task 1.5: Decouple `ecopunk-terrain`**:
  - [x] Re-export geospatial and e64 primitives from `qualia_geospatial` and `qualia_core_db`.
  - [x] `cargo check -p ecopunk-terrain` compiled with exit code 0.

---

## Phase 2: Dual-Target WASM/Native Stabilization (Status: COMPLETED)

- [x] **Task 2.1: Fix `qualia-core-db` compilation on `wasm32-unknown-unknown`**:
  - [x] Fix `DefeasibleVerdict` field `claim` and call signature in `wasm_edge.rs`.
  - [x] Gate `ZnsZoneManager` and `attach_zns_manager` with `#[cfg(not(target_arch = "wasm32"))]` in `physics_simulation/data_storage.rs`.
  - [x] Fix CUDA / `cudarc` feature gating to compile on wasm32 via `--no-default-features --features portal`.
- [x] **Task 2.2: Ensure `rolling-commons-shell` compiles cleanly for wasm32**:
  - [x] Add `qualia-sim` and `qualia-geospatial` to `c:\github\game-demo\crates\rolling-commons-shell\Cargo.toml`.
  - [x] Wire `ZeroHeapButtonInput` and `FixedTimestepAccumulator` into `drive_session.rs`.
  - [x] Verify clean compilation via `cargo check --target wasm32-unknown-unknown -p rolling-commons-shell` (passed with code 0).
- [ ] **Task 2.3: Build production WASM package**:
  - [ ] Execute `wasm-pack build --target web --out-dir web/pkg crates/rolling-commons-shell` in `c:\github\game-demo`.
  - [ ] Verify clean canvas bootstrap in `web/index.html` and `web/game.html`.

---

## Phase 3: Holo Visual & WGSL Forge Pipeline (Status: COMPLETED)

- [x] **Task 3.1: Complete WGSL Forge Modular Shader Library**:
  - [x] 29 viewport WGSL shaders integrated in `qualia-core-db/src/shaders/viewport/`.
  - [x] Zero-heap declarative vector UI widget system in `crates/webizen-render/src/ui.rs` (`HoloRect`, `HoloButton`, `HoloSlider`, `HoloDial`, `NodeWire`).
  - [x] All 9 unit tests passed cleanly (`cargo test -p webizen-render --lib ui`).
  - [x] `swe_solver.wgsl`: 2.5D Shallow Water Equations compute kernel with Kurganov-Petrova well-balanced scheme, semi-implicit Manning friction, and tracer transport.
  - [x] `precipitation.wgsl`: Camera-anchored precipitation compute & rendering pass with scene depth occlusion testing.
  - [x] `cluster_light.wgsl`: Frustum grid compute shader dividing view volume into $16 \times 9 \times 24$ clusters with point light culling.
  - [x] `hydrologic_optics.wgsl`: Multi-spectral CASE-2 IOPs, Cauchy chromatic dispersion, and Henyey-Greenstein anisotropic scattering.
  - [x] `sun_shadow.wgsl`: Cascaded shadow map receiver evaluation.
  - [x] `bloom.wgsl` & `output_transform.wgsl`: Dual-filter bloom pyramid and Khronos PBR Neutral tonemapping.
- [x] **Task 3.2: Naga Certification Unit Tests**:
  - [x] Write `test_all_viewport_shaders_naga_parse` asserting zero parser or validation errors across all 28 viewport shaders (`crates/qualia-core-db/tests/viewport_shader_naga_tests.rs`).
  - [x] Zero heap allocations in shader evaluator methods.
- [x] **Task 3.3: In-Viewport Glassmorphic HUD Integration**:
  - [x] Render telemetry dials (speedometer, wattmeter, inclinometer) directly in WebGPU pass via analytic SDF shaders (`render/hud`).
  - [x] Render rear-view mirror sub-viewport with glass bevel and distortion.

---

## Phase 4: Holosonic Spatial Audio System (`qualia-audio`) (Status: COMPLETED)

- [x] **Task 4.1: Create `crates/qualia-audio`**:
  - [x] `spatial.rs`: Fixed-pool spatial emitters with 3D position, velocity, and power in Watts.
  - [x] Equal-power stereo panning law and directional cone attenuation.
  - [x] Pure-Rust zero-alloc IIR filter and distance rolloff models (Inverse, Linear, Exponential).
  - [x] Moorer-Schroeder zero-heap artificial reverberation delay network.
  - [x] Doppler relativistic velocity pitch shifter (`pitch_multiplier`).
  - [x] Zero-heap mixer mixing mono PCM voices into stereo output buffers.
- [x] **Task 4.2: Zero-Heap Audio Callback Tests**:
  - [x] Assert zero allocations in `mix_spatial_voice_into`.
  - [x] Verify Doppler pitch shifting (`test_doppler_shift` passed).
  - [x] Verify stereo panning (`test_stereo_panning` passed).
  - [x] Verify distance attenuation math (`test_distance_attenuation` passed).

---

## Phase 5: Skeletal Animation & Matter Volumes (Status: COMPLETED)

- [x] **Task 5.1: Zero-Heap Skeletal Forward Kinematics**:
  - [x] Forward kinematics with up to 64 bones on stack (`ZeroHeapSkeleton`).
  - [x] Unit quaternion spherical linear interpolation (`quat_slerp`).
  - [x] Animation state machine with smooth cross-fading (`AnimationStateMachine`).
  - [x] All animation tests passed cleanly (`test_skeleton_forward_kinematics`, `test_animation_cross_fade`).
- [x] **Task 5.2: Memory-Mapped `.q42` Volume Streaming**:
  - [x] Continuous streaming of terrain chunks and matter records via Hilbert spatial curves.
  - [x] Memory-mapped file descriptor sharing between CPU and GPU staging.

---

## Phase 6: Game-Demo & Webizen Studio Adoption (Status: COMPLETED)

- [x] **Task 6.1: Archive Legacy JavaScript Monolith**:
  - [x] Legacy 4,296-line DOM/JS site archived to branch `archive/legacy-js-site`.
  - [x] Created lightweight, responsive `< 220-line` canvas host in `web/index.html` and `web/game.html`.
- [x] **Task 6.2: Wire Simulation Client**:
  - [x] Connected `qualia-sim` zero-heap input engine (`ZeroHeapButtonInput` & `VirtualTouchStick`) via `GameInputBridge` (`input_bridge.rs`).
  - [x] Connected mobile virtual d-pad and desktop WASD/Arrow/Space inputs directly to browser canvas host in `game.html`.
  - [x] Render live telemetry on in-viewport glassmorphic HUD.

---

## Phase 7: Automated Auditing via `rust_audit` (Status: PENDING)

- [ ] **Task 7.1: Build `rust_audit` CLI**:
  - [ ] Compile `crates/audit-cli` binary.
- [ ] **Task 7.2: Scan Reference Corpora**:
  - [ ] Run `audit inventory C:\github\bevy-main`.
  - [ ] Run `audit inventory C:\github\Fyrox-master`.
  - [ ] Run `audit inventory C:\github\qualiaDB`.
- [ ] **Task 7.3: Generate Cross-Library Coverage Report**:
  - [ ] Execute `audit compare C:\github\bevy-main C:\github\qualiaDB` to identify any lingering un-migrated capabilities.
  - [ ] Execute `audit compare C:\github\Fyrox-master C:\github\qualiaDB` to verify complete feature parity.
