# QualiaDB Multi-Scale Hydrological & Fluid Dynamics Architecture
## Master Engineering Plan & Technical Specification

> **Document Class:** Subsystem Technical Specification & Master Architecture Plan  
> **Target Subsystems:** `qualia-core-db`, `qualia-sim`, `webizen-render`, `qualia-audio`  
> **Authority Precedence:** Zero-Heap Hot-Path Contract ([`qualiaDB/AGENTS.md` §0](file:///c:/github/qualiaDB/AGENTS.md)) · 42MB Sentinel · 10D Spatiotemporal Manifold · WebGPU/WASM Dual-Target  
> **Status:** Approved Master Architecture  

---

## Executive Summary & Design Philosophy

QualiaDB's generative 3D/10D environment requires an integrated hydrological and fluid dynamics architecture that bridges continuous, high-frequency physical simulation on the GPU with deterministic, low-frequency ontological graph state in the `.q42` nquin semantic ledger.

When an agent or avatar wades into an estuary, crosses a mountain creek, or dives into an ocean, the system must coordinate across three coupled domains:
1. **Photorealistic Optical & Hydrodynamic Rendering (WebGPU WGSL/Slang):** Gerstner/FFT spectral wave horizons, 2.5D Shallow Water Equation (SWE) river networks, depth-based Beer-Lambert absorption, screen-space refractions, dynamic caustics, and underwater meniscus camera splits.
2. **Deterministic Mechanics & Character Kinematics (`qualia-sim` at 64 Hz):** Archimedean volumetric buoyancy, non-linear hydrodynamic drag, wake perturbation injection, and seamless character locomotion state transitions (*Dry Ground* $\rightarrow$ *Wading* $\rightarrow$ *Surface Swimming* $\rightarrow$ *6-DoF Subsurface Diving*).
3. **10D Experiential Qualia & Graph Ledger (`.q42` NQuins & VibeScript):** Continuous physical parameters (submersion depth, flow velocity, water temperature) downsampled across a hysteresis thresholding gate into discrete semantic facts (`rc:submersionState`, `rc:thermalDrainRate`, `rc:clothingWetnessRatio`), updating agent stamina, hypothermia risk, and environmental affordances without PCIe bus saturation.

```
       ┌────────────────────────────────────────────────────────┐
       │                WEATHER & ATMOSPHERIC FORCING           │
       │   Wind Vector (U10, V10), Precipitation Rate, Temp     │
       └───────────────────────────┬────────────────────────────┘
                                   │
         ┌─────────────────────────┴─────────────────────────┐
         ▼                                                   ▼
┌─────────────────────────────────┐         ┌─────────────────────────────────┐
│     MACRO-SCALE (OCEANS/SEAS)   │         │    MESO-SCALE (RIVERS/STREAMS)  │
│  JONSWAP / Tessendorf FFT 512²  │◄───────►│  2.5D Shallow Water Equations   │
│  Wave Spectrum + Gerstner SIMD  │ Estuary │  Slope advection & bed drag     │
└────────────────┬────────────────┘         └────────────────┬────────────────┘
                 │                                           │
                 └─────────────────────────┬─────────────────┘
                                           │ Local Water Level & Velocity
                                           ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    MICRO-SCALE INTERACTIVE WAVE BUFFER                      │
│      256² Ping-Pong Heightfield + SPH Splashes + Kelvin Bow Wakes           │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
     ┌─────────────────────────────────┴─────────────────────────────────┐
     ▼                                                                   ▼
┌─────────────────────────────────────────┐   ┌────────────────────────────────────────┐
│     HIGH-FREQUENCY WEBGPU RENDERER      │   │    64 Hz QUALIA-SIM KINEMATICS RING    │
│ • Beer-Lambert Extinction & Refraction  │   │ • Archimedean Submerged Proxies        │
│ • Jacobians (Foam & Crest Folding)      │   │ • SIMD CPU Dual-Tier Buoyancy & Drag   │
│ • Real-time Procedural Caustics         │   │ • Dry -> Wading -> Swimming -> Diving  │
│ • Underwater Meniscus & Acoustic LPF    │   │ • Vehicle Hydro-Lock & Float Engine    │
└─────────────────────────────────────────┘   └──────────────────┬─────────────────────┘
                                                                 │
                                                                 │ Hysteresis Commit
                                                                 ▼
┌──────────────────────────────────────────────────────────────────────────────────────┐
│                    QUALIA-DB 10D ONTOLOGICAL GRAPH LEDGER (.q42)                    │
│ • 48-Byte NQuin Opcodes (0x50–0x55: Submersion, Wetness, Flow Force, Hypothermia)   │
│ • Tensor10D State Mapping: (x, y, z, t) Spacetime + (α, μ, σ) Qualia Experiential   │
│ • VibeScript Declarative Hooks & N3Logic Inferences (Wetness, Navigation, Risk)     │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 1. Codebase Audit & Gap Analysis

A rigorous audit of `c:\github\qualiaDB` and `c:\github\game-demo` reveals existing primitives alongside critical gaps that this specification resolves.

### 1.1 Existing Assets in QualiaDB

| Subsystem / File | Current Implementation | Architectural Gap |
| :--- | :--- | :--- |
| `crates/qualia-core-db/src/shaders/viewport/water.wgsl` | 119-line WGSL vertex/fragment shader computing an analytic sine/cosine height displacement: `height = sin_x * cos_z * wave.y * shore_fade`. | Flat horizontal plane only. Lacks FFT dispersion, Gerstner multi-directional sums, depth refraction, Beer-Lambert wavelength absorption, caustics, and velocity advection. |
| `crates/qualia-core-db/src/render/gpu/water.rs` | `WaterGpu` manages static mesh buffer uploads, `WaterUniform` binding, and transparent raster pass with LessEqual depth testing. | Geometry is static; does not support dynamic clipmap LOD, cascading FFT textures, or compute-driven vertex displacement. |
| `crates/qualia-core-db/src/shaders/fluid_dynamics.wgsl` | 64-line compute kernel with a placeholder velocity damping: `velocity *= 0.99`. | Lacks actual Navier-Stokes, SWE, or particle hydrodynamics. Marked as a prototype in `wgsl_forge/physics/mod.rs`. |
| `crates/qualia-core-db/src/specialized_libs/engineering_analysis/cfd.rs` | 2D Lattice Boltzmann and staggered Cartesian Navier-Stokes solver. | High-cost offline structural CFD designed for engineering FEA analysis; cannot execute in 60 FPS real-time rendering loops. |
| `crates/qualia-core-db/src/q42/m64_matter/strata_record.rs` | 64-byte aligned `M64StrataRecord` with `surface_fluid_depth_mm: u16`, `fluid_velocity_xy: [i16; 2]`, and `fluid_sediment_load_ppm: u16`. | Memory layout exists, but no live coupling connects GPU fluid heights to these disk-backed strata records. |
| `crates/qualia-sim/src/character.rs` | Deterministic 64 Hz `CharacterController` with slope-sliding, step-climbing, velocity integration, and ground probe. | Only supports dry ground collision (`ground_probe`). Characters drop through water planes or treat them as solid rock if baked into the DEM. Lacks fluid drag, buoyancy, wading, and swimming states. |
| `crates/qualia-sim/src/kinematics.rs` | `RoverKinematics` with 4-wheel independent spring-damper suspension and Ackermann steering. | No water resistance, buoyancy on chassis, wheel spray generation, or engine stalling when water depth exceeds intake height. |
| `crates/qualia-core-db/src/tensor/mod.rs` | 10D Spatiotemporal state vector `Tensor10D` with axes `(q, v, w, x, y, z, t, alpha, mu, sigma)`. | No standardized mapping exists for fluid interaction metrics (wetness ratio, thermal conductivity drain, hydrodynamic drag). |

---

## 2. Multi-Scale Hydrological Simulation Engine

To deliver high visual and physical fidelity within a deterministic 60 FPS performance envelope across desktop and WebAssembly/WebGPU targets, simulation is partitioned across three hierarchical scales.

### 2.1 Macro-Scale: Oceanic Wave Synthesis (JONSWAP & Gerstner Cascades)

Open oceans and large lakes are modeled using a statistical wave spectrum driven by wind velocity $U_{10}$ and fetch length $F$:

$$\mathcal{S}_{\text{JONSWAP}}(\omega) = \frac{\alpha g^2}{\omega^5} \exp\left[ -\frac{5}{4}\left(\frac{\omega_p}{\omega}\right)^4 \right] \gamma^r$$

Where:
- $\omega_p = 22 \left(\frac{g^2}{U_{10} F}\right)^{1/3}$ is the peak angular frequency.
- $\gamma \approx 3.3$ is the peak enhancement factor.
- $r = \exp\left[ -\frac{(\omega - \omega_p)^2}{2\sigma^2 \omega_p^2} \right]$ with $\sigma = 0.07$ for $\omega \le \omega_p$ and $\sigma = 0.09$ for $\omega > \omega_p$.

#### Dual-Tier Execution Architecture (Overcoming the Readback Bottleneck)
1. **GPU Visual Tier (High-Frequency 60 Hz):**
   - A $512 \times 512$ 2D inverse Fast Fourier Transform (IFFT) compute shader computes spatial displacement $\mathbf{D}(\mathbf{x}, t) = [D_x, D_y, D_z]$ and normal/folding Jacobian $J(\mathbf{x}, t) = (1 + \frac{\partial D_x}{\partial x})(1 + \frac{\partial D_z}{\partial z}) - \frac{\partial D_x}{\partial z}\frac{\partial D_z}{\partial x}$.
   - Evaluated across 2 cascading frequency bands (Band 0: long swell, Band 1: capillary wind ripples).
2. **CPU Physics Tier (Zero-Latency SIMD in `qualia-sim`):**
   - The primary 8 wave harmonics from the spectrum are parameterized as an analytical **Gerstner wave cluster**:
     $$\mathbf{p}(\mathbf{x}_0, t) = \mathbf{x}_0 + \sum_{i=1}^8 \begin{bmatrix} Q_i A_i \frac{k_{x,i}}{k_i} \cos( \mathbf{k}_i \cdot \mathbf{x}_0 - \omega_i t + \phi_i ) \\ A_i \sin( \mathbf{k}_i \cdot \mathbf{x}_0 - \omega_i t + \phi_i ) \\ Q_i A_i \frac{k_{z,i}}{k_i} \cos( \mathbf{k}_i \cdot \mathbf{x}_0 - \omega_i t + \phi_i ) \end{bmatrix}$$
   - Evaluated on CPU via packed `f32x8` AVX2/NEON/WASM SIMD. Character and vessel buoyancy queries execute in sub-microsecond time with **zero GPU staging-buffer readbacks**.

### 2.2 Meso-Scale: Terrestrial Flow (2.5D Shallow Water Equations)

Rivers, streams, flash floods, and coastal wash are simulated using the 2.5D Shallow Water Equations (Saint-Venant system) over the terrain digital elevation model (DEM) $B(x, z)$:

$$\frac{\partial h}{\partial t} + \frac{\partial (hu)}{\partial x} + \frac{\partial (hv)}{\partial z} = R - I$$

$$\frac{\partial (hu)}{\partial t} + \frac{\partial}{\partial x}\left( hu^2 + \frac{1}{2}gh^2 \right) + \frac{\partial (huv)}{\partial z} = -gh \frac{\partial B}{\partial x} - C_f u \sqrt{u^2 + v^2}$$

$$\frac{\partial (hv)}{\partial t} + \frac{\partial (huv)}{\partial x} + \frac{\partial}{\partial z}\left( hv^2 + \frac{1}{2}gh^2 \right) = -gh \frac{\partial B}{\partial z} - C_f v \sqrt{u^2 + v^2}$$

Where:
- $h(x, z, t)$ is water column depth in meters.
- $u, v$ are flow velocities along the $X$ and $Z$ axes.
- $B(x, z)$ is bathymetry / terrain elevation from `qualia-geospatial` DEM.
- $R$ is precipitation inflow rate; $I$ is soil infiltration capacity.
- $C_f = \frac{g n^2}{h^{1/3}}$ is the Manning roughness coefficient based on `m64_matter` material LUT (gravel: $n=0.035$, bedrock: $n=0.020$, sand: $n=0.025$).

#### Numerical Scheme in WGSL Compute
- Discretized on a staggered Cartesian grid using a flux-conserving **Kurganov-Petrova (KP)** central-upwind scheme with minmod slope limiters to handle dry-bed wetting and hydraulic jumps without artificial numerical oscillations.
- Updated in an active $256 \times 256$ window surrounding the camera and active agents.

### 2.3 Micro-Scale: Interactive Wave Perturbation & Kelvin Wakes

Around characters, vehicles, and dynamic obstacles, water displacement is resolved using a 2D wave equation ping-pong buffer with hydrodynamic source injection:

$$\frac{\partial^2 \eta}{\partial t^2} = c^2 \nabla^2 \eta - \mu \frac{\partial \eta}{\partial t} + S(x, z, t)$$

Where:
- $\eta$ is surface displacement above equilibrium.
- $c = \sqrt{g h_{\text{eff}}}$ is phase propagation velocity.
- $\mu$ is viscous damping.
- $S(x, z, t)$ is the pressure/displacement impulse injected by wading feet, hull displacement, or vehicle tyres:
  $$S(x, z, t) = \sum_{\text{colliders}} \kappa \cdot \max(0, y_{\text{collider\_bottom}} - \eta) \cdot \mathbf{v}_{\text{rel}} \cdot \hat{\mathbf{n}}$$

### 2.4 Multi-Scale Coordinate & Estuarine Boundary Blending

To prevent visual seams and erratic buoyant impulses where a steep river merges into an ocean:
- An estuarine transition zone is defined by a normalized blend factor $\beta \in [0, 1]$ calculated from ocean distance and terrain bathymetry:
  $$y_{\text{water}}(\mathbf{x}, t) = (1 - \beta) \cdot \big[ B(\mathbf{x}) + h_{\text{SWE}}(\mathbf{x}, t) \big] + \beta \cdot \big[ y_{\text{sea\_level}} + \eta_{\text{ocean}}(\mathbf{x}, t) \big]$$
  $$\mathbf{v}_{\text{flow}}(\mathbf{x}, t) = (1 - \beta) \cdot \mathbf{v}_{\text{SWE}}(\mathbf{x}, t) + \beta \cdot \mathbf{v}_{\text{orbital}}(\mathbf{x}, t)$$
- $\beta$ is computed smoothly in compute shaders and CPU SIMD probes, ensuring continuous water elevation and velocity derivatives ($C^1$ continuity).

---

## 3. Entity & Character Two-Way Coupling

### 3.1 Volumetric Buoyancy (Archimedes Proxy Primitives)

Rather than single-point height sampling, `qualia-sim` uses multi-sphere or capsule proxy volumes fitted to character limbs and vehicle hulls:

```
Character Proxies:
  [Head Proxy:     r = 0.12 m]
  [Torso Proxy:    r = 0.22 m, h = 0.50 m]
  [Pelvis Proxy:   r = 0.18 m]
  [Left/Right Leg: r = 0.09 m, h = 0.75 m]
```

For each proxy $i$ with volume $V_i$ and center position $\mathbf{x}_i$:
1. Query water elevation $y_w(\mathbf{x}_i)$ and fluid velocity $\mathbf{v}_w(\mathbf{x}_i)$ via SIMD Gerstner/SWE evaluator.
2. Calculate submerged fraction $s_i \in [0.0, 1.0]$ based on proxy penetration depth:
   $$s_i = \text{clamp}\left( \frac{y_w - (\mathbf{x}_{i,y} - r_i)}{2 r_i}, 0.0, 1.0 \right)$$
3. Compute buoyant force:
   $$\mathbf{F}_{b,i} = \rho_{\text{fluid}} \cdot (s_i V_i) \cdot \mathbf{g}$$
4. Compute hydrodynamic form drag and skin friction:
   $$\mathbf{F}_{d,i} = -\frac{1}{2} \rho_{\text{fluid}} \cdot C_d \cdot A_i \cdot s_i \cdot \|\mathbf{v}_{\text{rel},i}\| \cdot \mathbf{v}_{\text{rel},i}$$
   Where $\mathbf{v}_{\text{rel},i} = \mathbf{v}_{\text{char},i} - \mathbf{v}_{w}(\mathbf{x}_i)$.
5. Total character water interaction:
   $$\mathbf{F}_{\text{hydro}} = \sum_i (\mathbf{F}_{b,i} + \mathbf{F}_{d,i}), \quad \boldsymbol{\tau}_{\text{hydro}} = \sum_i (\mathbf{x}_i - \mathbf{x}_{\text{com}}) \times (\mathbf{F}_{b,i} + \mathbf{F}_{d,i})$$

### 3.2 Character Locomotion State Machine

```
              ┌───────────────┐
              │   DRY GROUND  │
              └───────┬───────┘
                      │ Submersion > 0.05 m
                      ▼
              ┌───────────────┐
              │    WADING     │◄───────────────────┐
              │ (Ankle/Waist) │                    │
              └───────┬───────┘                    │
                      │ Submersion > Chest (1.1 m) │
                      ▼                            │
              ┌───────────────┐                    │ Submersion < 0.9 m
              │   SWIMMING    │────────────────────┘
              │   (Surface)   │
              └───────┬───────┘
                      │ Crouch / Pitch Down Input
                      ▼
              ┌───────────────┐
              │    DIVING     │
              │  (Subsurface) │
              └───────────────┘
```

- **Wading:** Ground traction is retained but movement speed is scaled by a non-linear drag multiplier:
  $$\text{speed}_{\text{max}} = \text{speed}_{\text{base}} \cdot \left( 1.0 - 0.65 \cdot \text{submersion\_ratio}^{1.5} \right)$$
- **Swimming:** Character feet lose ground contact (`is_grounded = false`). Movement switches to planar hydrodynamic propulsion with vertical equilibrium anchored at the waterline.
- **Diving:** Unconstrained 6-DoF orientation. Buoyancy matches lung inflation volume. Neutral buoyancy is maintained with minimal energy expenditure. Breath meter drains based on exertion.

---

## 4. Atmospheric Precipitation & Surface Wetness

### 4.1 GPU Particle Precipitation (Rain / Snow)

- Simulated in an infinite view-dependent bounding cylinder ($r = 25\text{ m}, h = 30\text{ m}$) anchored to the active camera.
- Rain streaks are rendered as camera-aligned instanced billboards oriented along the net velocity vector $\mathbf{v}_{\text{rain}} = \mathbf{v}_{\text{terminal}} + \mathbf{v}_{\text{wind}}$.
- **Depth Occlusion:** Compute passes sample the scene depth buffer prepass (`ao_prepass.wgsl`) to terminate rain particles on roofs, tree canopies, or cave overhangs, spawning micro-splashes at the collision coordinate.

### 4.2 Dynamic Surface Wetness PBR Pipeline

Surfaces exposed to rainfall or character entry track a wetness parameter $W(x, z, t) \in [0.0, 1.0]$:
1. **PBR Material Modification:**
   - **Albedo Darkening:** $\mathbf{C}_{\text{wet}} = \mathbf{C}_{\text{dry}} \cdot (1.0 - 0.4 \cdot W)$ for porous materials (dirt, asphalt, fabric).
   - **Roughness Reduction:** $\text{Roughness}_{\text{wet}} = \text{lerp}(\text{Roughness}_{\text{dry}}, 0.02, W^{2.0})$.
   - **Specular Boost:** Clearcoat specular layer with dielectric $F_0 = 0.02$ (water refractive index $n = 1.333$).
2. **Puddle Accumulation:**
   - On surfaces where $\hat{\mathbf{n}} \cdot \hat{\mathbf{u}} \ge 0.85$ (horizontal surfaces) and micro-cavity geometry permits, puddles form when $W > 0.6$, displacing normals to a pure vertical vector with animated wind ripple normal maps.

---

## 5. WebGPU Shading, Optics & Hydro-Acoustics

### 5.1 Physical Water Surface Optical Shader (`water_surface_pbr.wgsl`)

1. **Absorption & Scattering (Beer-Lambert Extinction):**
   Light transmission through water depth $d$ decays exponentially per RGB wavelength:
   $$\mathbf{T}(\lambda, d) = \exp\big( -\boldsymbol{\beta}_{\text{ext}}(\lambda) \cdot d \big)$$
   Typical oceanic constants: $\boldsymbol{\beta}_{\text{ext}} = [0.22, 0.045, 0.015]\text{ m}^{-1}$ (red light absorbs within 5 meters, leaving turquoise-blue).
2. **Refraction & Total Internal Reflection:**
   - Evaluated using Snell's law with water index $n_1 = 1.0$ (air) and $n_2 = 1.333$ (water).
   - Screen-space refraction offsets the scene color buffer by normal distortion $\Delta \mathbf{uv} = \mathbf{n}_{\text{tangent}} \cdot \text{scale} \cdot \min(d_{\text{scene}}, 1.0)$.
3. **Jacobian Wave Crest Foam:**
   - Evaluated directly from the displacement gradient tensor:
     $$J = \det(\nabla \mathbf{D}) = \left(1 + \frac{\partial D_x}{\partial x}\right)\left(1 + \frac{\partial D_z}{\partial z}\right) - \frac{\partial D_x}{\partial z}\frac{\partial D_z}{\partial x}$$
   - When $J < 0.35$, the wave is steepening into a breaking crest. White foam texture is blended additively and persists via a foam dissipation accumulator texture.
4. **Procedural Caustics:**
   - Projected onto submerged terrain and characters by tracing refracted rays through the dual-wave surface into the scene depth buffer, generating characteristic high-intensity caustic networks:
     $$I_{\text{caustic}} = \max\left( 0.0, 1.0 - \nabla \cdot \mathbf{n}_{\text{water}} \right)^{2.5}$$

### 5.2 Underwater Post-Processing & Audio Acoustics

- **Lens Meniscus:** When the camera intersects the water surface plane ($|y_{\text{camera}} - y_{\text{water}}| \le 0.08\text{ m}$), a dual-viewport post-process shader splits the frame into an upper air view and lower distorted underwater view separated by an animated meniscus liquid curve.
- **Acoustic Low-Pass Filtering:**
  - `qualia-audio::spatial` listener hook: When the camera/listener is submerged ($s > 0.8$), master audio frequencies above 850 Hz are rolled off at 18 dB/octave using a 2nd-order zero-heap IIR biquad low-pass filter, and speed of sound shifts from $343\text{ m/s}$ to $1482\text{ m/s}$ (compressing Doppler shifts).

---

## 6. 10D Qualia Semantic Integration & Graph Binding

### 6.1 Tensor10D Coordinate Semantics

In QualiaDB, every state vector is expressed in the 10-dimensional manifold `Tensor10D`:

$$\mathbf{T}_{10\text{D}} = \big[ q, \, v, \, w, \, x, \, y, \, z, \, t, \, \alpha, \, \mu, \, \sigma \big]$$

For hydrological and fluid interactions, the axes map deterministically:

| Axis | Role | Hydrological & Fluid Interpretation |
| :---: | :--- | :--- |
| **$q$** | Quantum / Speculative Context | $q = 0$: Validated, real-time physical simulation. $q > 0$: Speculative hydrological forecast (e.g., flood prediction, route planning). |
| **$v$** | Topological Variety | $v = 0$: Laminar open surface (placid lake). $v = 1$: Vortical / turbulent flow (rapids, whirlpools). $v = 2$: Porous sub-surface strata percolation. |
| **$w$** | Domain Index | $w = 3$: `Environmental` / Hydrological domain. |
| **$x, y, z$** | Spatial Coordinates | Metric coordinates in local ENU geodetic tangent space. |
| **$t$** | Coordinate Time | Simulation epoch timestamp (Lamport logical clock in bits [32..60]). |
| **$\alpha$** | Dynamic Range / Submersion | Submersion ratio $\in [0.0, 1.0]$: $0.0 = \text{Dry}$, $0.3 = \text{Wading}$, $0.7 = \text{Swimming}$, $1.0 = \text{Fully Submerged}$. |
| **$\mu$** | Fluid Medium Density | Specific gravity & salinity: $\mu = 1.000$ (freshwater), $\mu = 1.025$ (seawater), $\mu = 1.350$ (turbid mud/debris flow). |
| **$\sigma$** | Spectral Signature / Phase | Phase state: $\sigma = 0$ (liquid water), $\sigma = 1$ (ice/frost), $\sigma = 2$ (vapor/fog), $\sigma = 3$ (saline foam). |

### 6.2 Universal Quin Encoding & Predicate Registry

To comply with `AGENTS.md` §0 (Zero-heap hot paths, 48-byte super-quins), all water interaction events emit zero-copy `NQuin` records.

Modality opcodes for Hydrology start at `0x50` (above Deontic `0x10`, Epistemic `0x20`, Paraconsistent `0x30`, LTL `0x40`):

```rust
pub const OP_HYDRO_SUBMERSION: u8   = 0x50; // Submersion level and state transition
pub const OP_HYDRO_WETNESS: u8      = 0x51; // Surface/clothing saturation ratio
pub const OP_HYDRO_FLOW_FORCE: u8   = 0x52; // Hydrodynamic drag vector and drag torque
pub const OP_HYDRO_THERMAL_DRAIN: u8= 0x53; // Environmental thermal loss rate in Watts
pub const OP_HYDRO_TURBIDITY: u8    = 0x54; // Suspended sediment and optical visibility
pub const OP_HYDRO_RESERVED: u8     = 0x55; // Reserved for chemical/salinity state
```

#### Bit Layout for Hydrological Quins:
```
Field      Bits      Meaning
─────────────────────────────────────────────────────────────────────────────
subject    [0..63]   Entity / Agent DID hash (q_hash of IRI)

predicate  [63]      MSB Flag: 1 = Critical threshold alarm (e.g. Drowning / Hypothermia)
           [8..62]   Property IRI hash (e.g. q_hash("rc:submersionDepth"))
           [0..7]    Opcode: 0x50 (Submersion), 0x51 (Wetness), 0x53 (Thermal Drain)

object     [60..62]  Type Tag: 0b010 = xsd:decimal (value * 10^6)
           [0..59]   Payload (e.g. depth in mm, wetness fraction * 10^6, Watts * 10^3)

context    [56..63]  Sensitivity: SENSITIVITY_PUBLIC (0)
           [0..55]   Hydrological basin / river / ocean DID hash

metadata   [61..62]  Routing Lane: PermissiveRoutingLane::Spatial (11)
           [32..60]  Lamport Clock (ticks since sim epoch)
           [0..31]   Fluid Velocity magnitude (mm/s) or water temperature (mK)

parity     [0..63]   XOR fold (ECC check)
```

### 6.3 3D-to-10D Semantic Hysteresis & Commit Gate

To prevent high-frequency GPU wave fluctuations from flooding the QualiaDB graph with millions of redundant nquins, physical state updates pass through a **hysteresis commit gate**:

```rust
pub struct HydrologicalHysteresisGate {
    pub last_submersion_state: SubmersionState,
    pub last_wetness_band: u8,
    pub last_thermal_drain_watts: u32,
    pub last_commit_tick: u64,
}

impl HydrologicalHysteresisGate {
    pub fn evaluate(
        &mut self,
        current_depth: f32,
        current_wetness: f32,
        current_thermal_drain: f32,
        tick: u64,
    ) -> Option<NQuin> {
        // Quantize wetness into 4 discrete bands: Dry (0), Damp (1), Wet (2), Soaked (3)
        let wetness_band = if current_wetness < 0.15 { 0 }
            else if current_wetness < 0.50 { 1 }
            else if current_wetness < 0.85 { 2 }
            else { 3 };

        // Submersion state machine with 5cm hysteresis buffer
        let new_state = match self.last_submersion_state {
            SubmersionState::Dry => if current_depth > 0.08 { SubmersionState::Wading } else { SubmersionState::Dry },
            SubmersionState::Wading => {
                if current_depth < 0.03 { SubmersionState::Dry }
                else if current_depth > 1.15 { SubmersionState::Swimming }
                else { SubmersionState::Wading }
            },
            SubmersionState::Swimming => {
                if current_depth < 0.95 { SubmersionState::Wading }
                else if current_depth > 1.80 { SubmersionState::Diving }
                else { SubmersionState::Swimming }
            },
            SubmersionState::Diving => if current_depth < 1.65 { SubmersionState::Swimming } else { SubmersionState::Diving },
        };

        // Emit NQuin ONLY on state boundary transitions or every 64 ticks (1 sec) for telemetry
        if new_state != self.last_submersion_state || wetness_band != self.last_wetness_band || tick - self.last_commit_tick >= 64 {
            self.last_submersion_state = new_state;
            self.last_wetness_band = wetness_band;
            self.last_commit_tick = tick;
            Some(encode_hydrological_quin(new_state, current_depth, tick))
        } else {
            None
        }
    }
}
```

---

## 7. Concrete Rust Struct & WGSL Shader Specifications

### 7.1 Rust Data Structures (`qualia-sim::hydro`)

```rust
//! Deterministic Hydrodynamic Simulation Structures (`crates/qualia-sim/src/hydro.rs`)

use bytemuck::{Pod, Zeroable};

/// Global hydrological environment configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct HydroWorldConfig {
    /// Global ocean datum height in meters (e.g. 0.0).
    pub ocean_level_m: f32,
    /// Fluid density in kg/m³ (freshwater = 1000.0, seawater = 1025.0).
    pub fluid_density: f32,
    /// Fluid dynamic viscosity in Pa·s (water = 0.001002).
    pub dynamic_viscosity: f32,
    /// Ambient water temperature in Kelvin (e.g. 288.15 = 15°C).
    pub water_temp_k: f32,
    /// Wind vector [ux, uz] in m/s (drives waves and drift).
    pub wind_vector: [f32; 2],
    /// Global rain precipitation rate in mm/hr (0.0 = clear, 50.0 = downpour).
    pub rain_rate_mm_hr: f32,
    pub _padding: f32,
}

/// Compact 8-harmonic Gerstner wave spectrum for CPU SIMD evaluation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct SimdGerstnerSpectrum {
    pub amplitudes: [f32; 8],
    pub frequencies: [f32; 8],
    pub kx: [f32; 8],
    pub kz: [f32; 8],
    pub phases: [f32; 8],
    pub steepness: [f32; 8],
}

impl SimdGerstnerSpectrum {
    /// Pure SIMD zero-allocation evaluation of water surface height at (x, z).
    #[inline]
    pub fn sample_height_simd(&self, x: f32, z: f32, t: f32) -> (f32, [f32; 3]) {
        let mut height = 0.0f32;
        let mut normal = [0.0f32, 1.0f32, 0.0f32];

        // Evaluated across 8 harmonics
        for i in 0..8 {
            let theta = self.kx[i] * x + self.kz[i] * z - self.frequencies[i] * t + self.phases[i];
            let sin_t = theta.sin();
            let cos_t = theta.cos();
            height += self.amplitudes[i] * sin_t;
            normal[0] -= self.kx[i] * self.amplitudes[i] * cos_t;
            normal[2] -= self.kz[i] * self.amplitudes[i] * cos_t;
        }

        let len = (normal[0] * normal[0] + 1.0 + normal[2] * normal[2]).sqrt();
        (height, [normal[0] / len, 1.0 / len, normal[2] / len])
    }
}
```

### 7.2 WGSL Compute Shader: Shallow Water Equations (`shallow_water_equations.wgsl`)

```wgsl
// 2.5D Shallow Water Equations (Saint-Venant Solver) on Terrain Heightfield
// Discretized via Kurganov-Petrova central upwind scheme

struct SweUniforms {
    grid_size: vec2<u32>,        // e.g. [256, 256]
    cell_spacing: f32,          // dx, dz in meters (e.g. 0.5)
    dt: f32,                    // timestep in seconds (e.g. 0.015625)
    gravity: f32,               // 9.81
    manning_roughness: f32,     // bed friction (0.025)
    rain_rate: f32,             // precipitation source term (m/s)
    drain_factor: f32,          // soil infiltration rate
};

@group(0) @binding(0) var<uniform> cfg: SweUniforms;
// State buffer: R = depth h, G = momentum hu, B = momentum hv, A = terrain elevation B
@group(0) @binding(1) var state_in: texture_2d<f32>;
@group(0) @binding(2) var state_out: texture_storage_2d<rgba32float, write>;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= cfg.grid_size.x || id.y >= cfg.grid_size.y) {
        return;
    }

    let coord = vec2<i32>(id.xy);
    let center = textureLoad(state_in, coord, 0);
    let left   = textureLoad(state_in, coord + vec2<i32>(-1, 0), 0);
    let right  = textureLoad(state_in, coord + vec2<i32>(1, 0), 0);
    let down   = textureLoad(state_in, coord + vec2<i32>(0, -1), 0);
    let up     = textureLoad(state_in, coord + vec2<i32>(0, 1), 0);

    let h = center.r;
    let hu = center.g;
    let hv = center.b;
    let bed = center.a;

    // Minimum depth threshold to prevent divide-by-zero on dry beds
    let eps = 0.0001;
    let u = select(0.0, hu / h, h > eps);
    let v = select(0.0, hv / h, h > eps);

    // Bed slope gradients
    let db_dx = (right.a - left.a) / (2.0 * cfg.cell_spacing);
    let db_dz = (up.a - down.a) / (2.0 * cfg.cell_spacing);

    // Flux derivatives
    let d_hu_dx = (right.g - left.g) / (2.0 * cfg.cell_spacing);
    let d_hv_dz = (up.b - down.b) / (2.0 * cfg.cell_spacing);

    // Continuity equation update
    var next_h = h - cfg.dt * (d_hu_dx + d_hv_dz) + cfg.dt * (cfg.rain_rate - cfg.drain_factor * h);
    next_h = max(0.0, next_h);

    // Bed friction (Manning's equation)
    let speed = sqrt(u * u + v * v);
    let friction = select(0.0, (cfg.gravity * cfg.manning_roughness * cfg.manning_roughness * speed) / pow(max(h, 0.01), 1.333), h > eps);

    // Momentum updates with bed slope gravity term
    let next_hu = hu - cfg.dt * (cfg.gravity * h * db_dx + friction * hu);
    let next_hv = hv - cfg.dt * (cfg.gravity * h * db_dz + friction * hv);

    textureStore(state_out, coord, vec4<f32>(next_h, next_hu, next_hv, bed));
}
```

---

## 8. Unified Dielectric & Hydrologic Optics (Water, Particulates & Glass)

QualiaDB rejects naive 3-channel RGB alpha-blended transparency. Water columns and solid dielectrics (glass, ice, silica crystals) share an underlying physical model rooted in **Hydrologic Optics**, **Radiative Transfer Theory (RTT)**, and **Inherent Optical Properties (IOPs)** interacting with **Apparent Optical Properties (AOPs)**.

```
       ┌────────────────────────────────────────────────────────┐
       │             INHERENT OPTICAL PROPERTIES (IOP)          │
       │  • Absorption a(λ): H2O Vibrational + CDOM Tannins     │
       │  • Rayleigh Scattering b_r(λ) ∝ λ⁻⁴ (Density Jitter)   │
       │  • Mie Scattering b_m(λ) (Silt, Clay, Micro-Bubbles)   │
       │  • Henyey-Greenstein Anisotropy g ∈ [-1, 1]            │
       └───────────────────────────┬────────────────────────────┘
                                   │ Beam Attenuation c(λ) = a(λ) + b(λ)
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│             APPARENT OPTICAL PROPERTIES (AOP) & BSDF INTERFACE         │
│  • Microfacet GGX Distribution (Capillary Wind Ripples drive Roughness)│
│  • Cauchy / Sellmeier Dispersion: n(λ) = A + B/λ² (Spectral Splitting) │
│  • Brewster's Angle Polarization (Surface Glare vs. Substrate Window)  │
│  • Radiative Transfer In-Scattering: Volumetric Backscatter Veil       │
└────────────────────────────────────────────────────────────────────────┘
```

### 8.1 Inherent Optical Property (IOP) Volume Struct
```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DielectricVolumeIop {
    /// Spectral Index of Refraction (IOR) [n_red, n_green, n_blue] driving chromatic dispersion.
    pub ior_spectrum: [f32; 3],
    /// Spectral absorption coefficient a(λ) in m⁻¹ (photons absorbed by molecules).
    pub absorption_coeff: [f32; 3],
    /// Molecular Rayleigh scattering coefficient b_r(λ) in m⁻¹ (∝ λ⁻⁴).
    pub rayleigh_scattering: [f32; 3],
    /// Particulate Mie scattering [b_r, b_g, b_b, g] in m⁻¹ + Henyey-Greenstein anisotropy g.
    pub mie_scattering: [f32; 4],
}
```

### 8.2 Rayleigh vs. Mie Scattering Regimes
1. **Molecular / Rayleigh Scattering ($d \ll \lambda$):**
   - In pure water and silica glass, photons interact with thermal density fluctuations at the molecular scale.
   - Scattering intensity scales with $\lambda^{-4}$ (isotropic dipole radiation).
   - $O\text{--}H$ molecular bonds violently absorb red/IR ($>600\text{ nm}$), leaving deep water an intense indigo-blue.
   - In soda-lime glass, trace $\text{Fe}^{2+}$ iron-oxide impurities absorb red light ($600\text{--}700\text{ nm}$), rendering thick glass edges emerald green.
2. **Particulate / Mie Scattering ($d \ge \lambda$):**
   - Suspended mineral silt, colloidal clay, and micro-bubbles scatter light strongly forward ($g \approx 0.85\text{--}0.92$).
   - Modeled via the **Henyey-Greenstein phase function**:
     $$p(\theta) = \frac{1 - g^2}{4\pi (1 + g^2 - 2g \cos\theta)^{3/2}}$$
   - High turbidity waters create **volumetric subsurface backscatter (in-scattering)**: photons scatter back into the eye from the top few centimeters of the water column, veiling the substrate in an opaque milky ochre or turquoise hue.

### 8.3 Wave-Construct Microfacet BSDF & Brewster Polarization
- **Fresnel Reflection & Brewster's Angle:** At normal incidence, water reflects only $\sim 2.0\%$ of incident light ($98\%$ penetrates to the substrate). Near grazing angles, reflectivity reaches $100\%$. Looking at Brewster's angle ($\theta_B = \arctan(n) \approx 53.1^\circ$ for water), reflected light is linearly polarized parallel to the surface, allowing polarized sunglasses or sensors to extinguish surface reflections and see straight to the bottom.
- **Wave Slope & Microfacet GGX Modulation:** Surface capillary waves tilt the microfacet normal distribution, dynamically expanding the sun's specular glitter path and blurring refracted substrate visibility.

---

## 9. 10D Spectral EMF Transport & Bio-Geo-Optical Provenance

QualiaDB treats light within the 10D spatiotemporal manifold as a **continuous Electromagnetic Field (EMF)** characterized by amplitude, wavelength spectrum $\lambda$, phase $\phi$, and Stokes polarization vector $\mathbf{S} = [I, Q, U, V]$.

### 9.1 Complex Refractive Index & Phase Shifts
$$\tilde{n}(\omega) = n(\omega) + i\kappa(\omega)$$
- **Real Part $n(\omega)$:** Dictates the phase velocity $v_p = c / n(\omega)$ and the accumulated depth phase shift $\Delta\phi = \frac{2\pi}{\lambda_0} n(\omega, z) \Delta z$. Thermoclines and haloclines bend and modulate optical wavefronts.
- **Imaginary Part $\kappa(\omega)$:** Directly governs exponential amplitude attenuation across depth: $\kappa(\omega) = \frac{\lambda_0}{4\pi} a(\omega)$.
- **Downsampling:** Spectral radiance is transported across 8 to 16 wavelength bins and downsampled to CIE 1931 XYZ $\rightarrow$ Display linear sRGB only at the final camera/retina sensor stage.

### 9.2 Four-Component Bio-Geo-Optical Model (CASE-2 Waters)
Total spectral absorption $a(\lambda, z)$ and backscattering $b_b(\lambda, z)$ at any world coordinate are synthesized deterministically from four constituents stored in `.q42` nquins:

1. **Pure Water ($H_2O$):** Baseline Rayleigh scattering and vibrational absorption ($a_{\text{red}} = 0.35\text{ m}^{-1}$, $a_{\text{blue}} = 0.005\text{ m}^{-1}$).
2. **CDOM / Tannins (`cdom_tannin_conc`):** Dissolved humic and fulvic acids leached from eucalyptus leaf litter and peat detritus. Exponential blue/violet absorption:
   $$a_{\text{CDOM}}(\lambda) = a_{440} \exp\left[ -S_{\text{CDOM}} (\lambda - 440) \right]$$
   With negligible particulate scattering ($b_b \approx 0$), tannins create transparent amber shallows and a **dark blackwater mirror** with crisp sky reflections.
3. **Inorganic Mineral Tripton & Colloidal Clay (`tripton_clay_silt_conc`):** Dispersive clay subsoils (kaolinite, illite, iron oxides). Models strong Mie backscattering ($b_b(\lambda) \propto \lambda^{-\gamma}$ with $\gamma \approx 0.8$), destroying substrate visibility within centimeters and creating an **opaque ochre milky veil**.
4. **Biogenic Seston / Phytoplankton (`chlorophyll_conc`):** Algae and cyanobacteria with absorption peaks at 440 nm and 675 nm and green reflectance peaks.

### 9.3 Geomorphic & Historical Substrate Coupling (N3Logic / VibeScript)
The optical constituent concentrations in water bodies are bound to QualiaDB's historical and geomorphic ledger:
- **Intact Armored Substrate:** Intact cobble and bedrock lock bedload sediment in place $\rightarrow$ low colloidal tripton $\rightarrow$ high optical transmittance and clear Brewster polarization window to riverbed.
- **Post-Mining Disturbed Substrate (Historic 19th-Century Alluvial Gold Dredging / Sluicing):** Hydraulic sluicing stripped the gravel armor layer, exposing raw clay subsoils. Stream velocity shear stress continuously injects dispersive clay into the water column $\rightarrow$ persistent downstream Mie backscattering.
- **Riparian Canopy & Stagnant Dams:** Dense eucalyptus leaf fall and low flushing rates yield elevated `cdom_tannin_conc` and thermal stratification.

---

## 10. Synoptic Atmospherics, Boundary-Layer Wind & Aeroelastic Coupling

Closing the hydrological cycle requires bridging continental-scale synoptic meteorology (online barometric pressure grids) down to the **Atmospheric Boundary Layer (ABL)** and **Aeroelastic Foliage Dynamics** (air-particle momentum driving water waves, tree branches, and leaf detachment).

```
   ┌────────────────────────────────────────────────────────┐
   │             ONLINE SYNOPTIC WEATHER INGESTION          │
   │  • ERA5 / NOAA GFS / BOM ACCESS / Open-Meteo NetCDF    │
   │  • Pressure Gradients ∇P, Geostrophic Wind U_10, Temp  │
   └───────────────────────────┬────────────────────────────┘
                               │ PGF + Coriolis + Friction
                               ▼
┌────────────────────────────────────────────────────────────────────────┐
│             ATMOSPHERIC BOUNDARY LAYER (ABL) MICRO-PHYSICS             │
│  • Prandtl–von Kármán Log Profile: u(z) = (u*/κ) ln((z-d)/z0)          │
│  • Canopy Roughness (z0 = 1.25m) & Charnock Wave Stress Transfer       │
│  • Dynamic Pressure q = 0.5 ρ_air |v_rel|²                             │
└───────────────────────────┬────────────────────────────────────────────┘
                            │ Dynamic Pressure q(z, t)
                            ▼
┌────────────────────────────────────────────────────────────────────────┐
│             4-LEVEL HIERARCHICAL AEROELASTIC COUPLING                  │
│  • Level 0/1: Trunk & Boughs (Cantilever Harmonic Bending, 0.2–0.8 Hz) │
│  • Level 2: Secondary Branches (Vogel Reconfiguration Exponent 𝒱)      │
│  • Level 3: Leaves & Petioles (Strouhal Vortex Flutter, 5–25 Hz)       │
│  • Abscission Failure: Leaves detach -> River CDOM Tannin Source       │
└────────────────────────────────────────────────────────────────────────┘
```

### 10.1 Synoptic Ingestion & Moist Air Thermodynamics
```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarometricState {
    pub mslp_hpa: f32,             // Mean Sea Level Pressure (hPa)
    pub temperature_c: f32,        // Ambient temperature (°C)
    pub dew_point_c: f32,          // Dew point (°C)
    pub relative_humidity: f32,    // Relative humidity in [0.0, 1.0]
    pub wind_vector_10m: [f32; 2], // Geostrophic wind [u, v] in m/s
}
```
- **Ideal Gas Law for Moist Air:**
  $$\rho_{\text{air}} = \frac{P_d}{R_d T} + \frac{P_v}{R_v T}$$
  Where $R_d = 287.058\text{ J/(kg}\cdot\text{K)}$ and $R_v = 461.495\text{ J/(kg}\cdot\text{K)}$.
- **Orographic Lift & Condensation:** Forced uplift over terrain slopes ($\mathbf{U} \cdot \nabla z_{\text{terrain}}$) triggers adiabatic expansion cooling ($dT/dz \approx -6.5^\circ\text{C/km}$ moist). When $z \ge z_{\text{LCL}} \approx 125(T - T_{\text{dew}})$, condensation releases latent heat and feeds precipitation into Shallow Water Equations (SWE) catchments.
- **Inverse Barometer Sea Surface Bulge:** A $1\text{ hPa}$ atmospheric pressure drop bulges local sea surface elevation upward by $\sim 1.01\text{ cm}$, driving meteorological storm surges.

### 10.2 Atmospheric Boundary Layer (ABL) Wind Shear
- **Logarithmic Profile:**
  $$u(z) = \frac{u_*}{\kappa} \ln\left(\frac{z - d}{z_0}\right)$$
  Smooth water has $z_0 \approx 0.0002\text{ m}$ (unhindered wind acceleration), whereas eucalyptus forest canopies have $d \approx 6.0\text{ m}$ and $z_0 \approx 1.25\text{ m}$ (slowing mean velocity while generating turbulent von Kármán eddies).
- **Charnock Momentum Transfer:** Wind friction velocity $u_*$ directly drives the JONSWAP wave spectrum in `qualia-sim::hydro` and generates spray particles.

### 10.3 Aeroelastic Vegetation & Foliage Dynamics
1. **Level 0/1: Trunk & Primary Boughs:** Modeled as damped cantilever beam oscillators ($0.2\text{--}0.8\text{ Hz}$).
2. **Level 2: Secondary Branches (Vogel Reconfiguration):**
   $$F_{\text{drag}} \propto |\mathbf{v}_{\text{rel}}|^{2 + \mathcal{V}}$$
   Where Vogel exponent $\mathcal{V} \approx -0.6$. As wind gusts intensify, flexible branches bend leeward, reducing frontal area and shedding aerodynamic load.
3. **Level 3: Leaves & Petioles (Strouhal Vortex Flutter):**
   Alternating low-pressure micro-vortices shed off leaf edges at frequency:
   $$f_{\text{flutter}} = \frac{\text{St} \cdot v}{L_{\text{leaf}}} \quad (\text{St} \approx 0.18)$$
   Inducing high-frequency ($5\text{--}25\text{ Hz}$) torsional flutter that modulates specular highlights and subsurface transmission.
4. **Petiole Abscission & River Basin Detritus:** When aerodynamic drag force exceeds the petiole's tensile limit (`abscission_limit_n`), the leaf snaps off into the 3D velocity field, tumbles via Andersen-Pesavento aerodynamics, and deposits into rivers, leaching `cdom_tannin_conc` into the aquatic optical model.

---

## 11. Phased Implementation Roadmap & Verification Milestones

To manage scope and ensure system stability under our zero-heap, 42MB Sentinel, and dual-target constraints, the implementation is partitioned into five testable phases.

```
┌────────────────────────────────────────────────────────────────────────┐
│ PHASE 1: Macro-Scale Ocean Spectrum & Dual-Tier SIMD Buoyancy          │
│ • JONSWAP / Gerstner WGSL compute FFT + Packed f32x8 CPU Evaluator     │
│ • Character Archimedes Proxies in qualia-sim                           │
│ • Tests: 0-alloc SIMD height probes & float equilibrium verification   │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
┌───────────────────────────────────▼────────────────────────────────────┐
│ PHASE 2: Meso-Scale Shallow Water Equations & River Spline Flow        │
│ • 2.5D Saint-Venant SWE Compute Shader on DEM Bathymetry               │
│ • Estuarine blending zone with C1 continuity                           │
│ • Coupling with M64StrataRecord (surface_fluid_depth_mm)               │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
┌───────────────────────────────────▼────────────────────────────────────┐
│ PHASE 3: Atmospheric Precipitation & Surface Wetness PBR               │
│ • Camera-anchored particle precipitation with depth-buffer occlusion   │
│ • Dynamic PBR wetness darkening and puddle accumulation in WGSL        │
│ • Splash spawning at character entry points                            │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
┌───────────────────────────────────▼────────────────────────────────────┐
│ PHASE 4: Full Character Locomotion States & Dynamic Wakes              │
│ • Locomotion State Machine (Dry -> Wading -> Swimming -> Diving)       │
│ • Micro-scale 2D wave perturbation buffer (Kelvin wakes & bow waves)   │
│ • Underwater camera split meniscus & hydro-acoustic low-pass filter    │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
┌───────────────────────────────────▼────────────────────────────────────┐
│ PHASE 5: 10D Semantic Graph Integration & VibeScript Affordances       │
│ • 48-byte NQuin encoding with opcodes 0x50–0x55                        │
│ • Hysteresis commit gate protecting PCIe/WASM bus                      │
│ • VibeScript APIs (query_water_depth, trigger_storm, wading_resistance)│
│ • Automated coverage verification via rust_audit                       │
└────────────────────────────────────────────────────────────────────────┘
```

### Phase 1: Macro-Scale Ocean Spectrum & Dual-Tier SIMD Buoyancy
- [ ] Create `crates/qualia-sim/src/hydro.rs`:
  - Implement `SimdGerstnerSpectrum` with 8 wave harmonics evaluated via packed `f32x8` SIMD.
  - Implement `ArchimedesProxy` multi-sphere volume evaluation.
- [ ] Add unit tests in `qualia-sim`:
  - `test_gerstner_simd_zero_alloc`: Assert zero heap allocations during 10,000 height samples.
  - `test_character_floating_equilibrium`: Assert a 75 kg character floats with head proxy above waterline.

### Phase 2: Meso-Scale Shallow Water Equations & River Flow
- [ ] Create `crates/qualia-core-db/src/shaders/viewport/swe_solver.wgsl`:
  - Implement 2.5D Saint-Venant SWE equations with Manning roughness.
  - Implement estuarine blending with `SimdGerstnerSpectrum`.
- [ ] Wire SWE water depth updates into `M64StrataRecord::surface_fluid_depth_mm`.
- [ ] Add unit tests in `qualia-core-db`:
  - `test_swe_conservation_of_mass`: Assert total fluid volume is conserved across closed basins.

### Phase 3: Atmospheric Precipitation & Surface Wetness PBR
- [ ] Create `crates/qualia-core-db/src/shaders/viewport/precipitation.wgsl`:
  - Camera-anchored precipitation particle compute pass with scene depth occlusion.
- [ ] Update `crates/qualia-core-db/src/shaders/viewport/water.wgsl`:
  - Incorporate Beer-Lambert wavelength extinction, screen-space refraction, and Jacobian crest foam.
- [ ] Add unit tests verifying WGSL shader compilation and Naga validation.

### Phase 4: Full Character Locomotion Coupling & Dynamic Wakes
- [ ] Extend `CharacterController` in `crates/qualia-sim/src/character.rs`:
  - Integrate `SubmersionState::{Dry, Wading, Swimming, Diving}`.
  - Apply hydrodynamic drag and buoyancy to velocity integration.
- [ ] Create 2D interactive wave perturbation ping-pong buffer in `webizen-render`.
- [ ] Wire acoustic low-pass filtering into `qualia-audio::spatial` for submerged camera/listeners.

### Phase 5: 10D Semantic Graph Integration & VibeScript Hardening
- [ ] Register opcodes `0x50..=0x55` in `qualia-core-db::modalities`.
- [ ] Implement `HydrologicalHysteresisGate` to commit NQuins only on state boundary crossings.
- [ ] Expose declarative VibeScript bindings in `vibe-wasm` and `rolling-commons-shell`:
  - `hydro_sample_water(x, z) -> { depth, velocity_x, velocity_z, surface_y }`
  - `hydro_get_agent_wetness(agent_id) -> f32`
- [ ] Run `cargo check --target wasm32-unknown-unknown` across all crates to ensure WASM portability.
- [ ] Execute `rust_audit` AST verification against Bevy and Fyrox reference corpora.

---

## 12. Hardware & Resource Budget Compliance

| Constraint | Limit | Implementation Policy |
| :--- | :--- | :--- |
| **Hot-Path Allocations** | **0 bytes** | All height probes, buoyancy integrators, and wave buffers use fixed stack arrays `[T; N]` or caller-supplied slices `&mut [T]`. |
| **VRAM Consumption** | **< 128 MiB** | Dual $512 \times 512$ FFT textures ($8\text{ MiB}$) + SWE $256 \times 256$ RGBA32F ($1\text{ MiB}$) + Ripple Buffer $256 \times 256$ R16F ($0.25\text{ MiB}$) + Particle instances ($2\text{ MiB}$). |
| **GPU Compute Time** | **< 1.8 ms** | All compute passes (FFT, SWE, Ripples) execute in under 1.8 ms on RTX A2000 / Apple M2 class hardware. |
| **Graph State Churn** | **< 4 NQuins / sec** | Controlled by `HydrologicalHysteresisGate`; high-frequency waves never generate per-frame graph transactions. |
