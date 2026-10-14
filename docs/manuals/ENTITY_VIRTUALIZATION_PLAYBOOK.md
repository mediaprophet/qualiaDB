# Ontological Entity Virtualization Playbook: Specimens, Personas & Constructs
**Standard Version:** 2.0.0 | **Author:** Qualia Core Architecture  
**Target Runtimes:** `qualiaDB` (`.10d` Container, WebGPU Portal), `webizen-os-shell`, `ecopunk`, `game-demo`

---

## 1. Executive Directive: Beyond the Industrial "Digital Twin"

This playbook is the normative specification for authoring multi-modal virtual embodiments of entities across distinct ontological realms.

### The Ontological Shift Away from "Digital Twin" Enclosure
The industrial framing of a "digital twin" treats every model as a corporate asset to be owned, captured, and assigned. This creates catastrophic legal and ethical hazards when applied to living organisms or human participants:
- **World of God / Inalienable Living Realm:**
  - **`DarwinianTaxon` $\to$ `Specimen`:** Natural organisms (*flora, fauna, funga*) belong to the ecological commons. A virtual model is a scientific, sensory *specimen* with provenance, never a privatized industrial clone.
  - **`NaturalPerson` $\to$ `Persona` / `Likeness`:** Human subjects possess inalienable moral rights and bodily sovereignty. A person cannot be commodified as a property or corporate asset. A virtual representation is a sovereign *persona* governed by DID consent.
- **World of Man / Manufactured Realm:**
  - **`ManufacturedProperty` $\to$ `Construct` / `Artifact`:** Fabricated tools, machinery, and vehicles (e.g. solar pumps, turbines, footwear). This is the legitimate domain of property rights, operational ratings, and maintenance regimes.
- **World of Culture / Expression:**
  - **`LiberalArtsWork` $\to$ `Expression` / `Lore`:** Authored cultural compositions, oral stories, 3D sculptures, and music, governed by attribution and creative stewardship.

### The Five Non-Negotiable Axioms:
1. **Ontological Integrity & Anti-Commodification:** An entity's ontological realm dictates its English nomenclature and cryptographic rights. Human persons can never be compiled as property or commercial assets (`GateError::HumanPersonCannotBeCommodifiedAsAsset`).
2. **Continuous EMF Spectral Materials:** No biological RGB tristimulus albedo values. All surface reflectance is defined via **Jakob & Hanika (2019)** polynomial coefficients $(c_0, c_1, c_2)$ guaranteeing bounded physical energy $R(\lambda) \in [0.0, 1.0]$.
3. **4-Tier Decoupled Virtual Geometry:** Every entity must provide references for all 4 resolution tiers (Hero Meshlets $\to$ Decimated Mesh $\to$ Octahedral Impostor Atlas $\to$ Hierarchical Surfels) to bypass the GPU $2 \times 2$ quad-overdraw penalty.
4. **Deterministic Zero-Heap Packaging:** Authored folders must compile through `compile_entity` into `.10d` containers (Sections 1, 13, 17, 18, 19, 20) sealed with CRC-32C checksums and packaged into `.hmc` bundles.
5. **Reverse-LOD Locality:** Containers are sequenced in Reverse-LOD layout (Header/Manifest $\to$ SURF1 $\to$ IMPO1 $\to$ Decimated Meshlets $\to$ Hero Meshlets) enabling immediate far-field display on edge and mobile WebGPU runtimes while streaming high-density clusters asynchronously.

---

## 2. The 4-Tier Virtualized Geometry Pipeline

| Tier | Screen-Space Radius ($R_{\text{eff}}$) | Technology | Container Section | GPU Sympathy |
|---|---|---|---|---|
| **Tier 1: Hero** | $\ge 256.0$ px | Cluster Meshlets (~128 tris) | Section 1 (`QuantizedMesh`) | Cone normal backface culling, fixed vertex-cache reuse. |
| **Tier 2: Decimated** | $64.0 \le R_{\text{eff}} < 256.0$ px | QEM Decimated Mesh | Section 1 (`QuantizedMesh`) | Progressive edge-collapse LOD. |
| **Tier 3: Impostor** | $8.0 \le R_{\text{eff}} < 64.0$ px | Hemi-Octahedral Atlas | Section 19 (`ImpostorAtlas`, `IMPO1`) | 2-triangle camera-facing quad sampling $16 \times 16$ directional views. |
| **Tier 4: Surfel** | $1.0 \le R_{\text{eff}} < 8.0$ px | Hierarchical Gaussian Splats | Section 3 (`Reconstruction`) | Single compute raster pass; 1 point per cluster. |

---

## 3. Continuous Electromagnetic Field (EMF) Spectral Materials

Light transport in QualiaDB simulates continuous radiant energy across $\lambda \in [380, 780]\text{ nm}$.

### 3.1 Bounded Spectral Basis (Jakob & Hanika 2019)
Given normalized wavelength $u = (\lambda - 380) / 400 \in [0.0, 1.0]$, surface reflectance is:
$$R(u) = 0.5 + 0.5 \cdot \frac{f(u)}{\sqrt{1.0 + f(u)^2}}, \quad f(u) = c_0 u^2 + c_1 u + c_2$$

In WGSL, this evaluates in exactly **6 ALU cycles**:
```wgsl
fn evaluate_spectral_reflectance(coeffs: vec3<f32>, u: vec4<f32>) -> vec4<f32> {
    let f = fma(u, fma(vec4<f32>(coeffs.x), u, vec4<f32>(coeffs.y)), vec4<f32>(coeffs.z));
    let denom = fma(f, f, vec4<f32>(1.0));
    return fma(f * inverseSqrt(denom), vec4<f32>(0.5), vec4<f32>(0.5));
}
```

### 3.2 16-Byte/Pixel WebGPU G-Buffer Layout
- **GBuffer0 (`rgba16float` - 64 bpp):** $c_0, c_1, c_2$ (`f16`, $[-128, 128]$) + Roughness $\alpha \in [0, 1]$.
- **GBuffer1 (`rgba8unorm` - 32 bpp):** Octahedral Normal $(N_x, N_y)$ + Metallic $m$ + Material Flags bitmask.
- **Depth (`depth32float` - 32 bpp):** Reversed-Z floating point.

---

## 4. Canonical Authoring Template: Natural Organism (*Eucalyptus regnans*)

Save as `entities/flora/eucalyptus_regnans/entity.json`:

```json
{
  "entity_id": "urn:qualia:taxon:eucalyptus_regnans_01",
  "ontology_class": "DarwinianTaxon",
  "embodiment_term": "Specimen",
  "taxonomic_data": {
    "kingdom": "Plantae",
    "clade": "Angiosperms",
    "order": "Myrtales",
    "family": "Myrtaceae",
    "genus": "Eucalyptus",
    "species": "Eucalyptus regnans",
    "common_name": "Mountain Ash",
    "maturity_stage": "adult",
    "max_height_m": 100.5,
    "canopy_radius_m": 14.2
  },
  "spectral_profile": {
    "materials": [
      {
        "id": "canopy_foliage",
        "type": "biological_translucent",
        "basis_coefficients": {
          "c0": 0.045,
          "c1": -1.200,
          "c2": 0.850
        },
        "bssrdf_mean_free_path_mm": {
          "blue_450nm": 0.45,
          "green_550nm": 2.10,
          "red_650nm": 3.80,
          "nir_750nm": 5.20
        },
        "chlorophyll_degradation_rate": 0.015
      },
      {
        "id": "fibrous_bark",
        "type": "dielectric_opaque",
        "basis_coefficients": {
          "c0": 0.012,
          "c1": -0.450,
          "c2": 0.320
        },
        "roughness": 0.85
      }
    ]
  },
  "procedural_synthesis": {
    "seed": "0x5EED_EUC_REGNANS_42",
    "cellular_voronoi_fissures": {
      "frequency": 14.5,
      "jitter": 0.85
    },
    "implicit_sdf_branch_fillet": {
      "blend_radius_m": 0.35,
      "threshold": 0.02
    },
    "allometric_growth": {
      "huxley_exponent_k": 1.42,
      "base_b": 0.18
    },
    "pga_secondary_wind_oscillation": {
      "motor_frequency_hz": 0.45,
      "damping_ratio": 0.15
    }
  },
  "geometry_tiers": {
    "tier_1_hero": {
      "type": "meshlet_cluster",
      "data_ref": "hmc://entities/flora/euc_regnans.10d#meshlets_sec1"
    },
    "tier_2_decimated": {
      "type": "quantized_index",
      "data_ref": "hmc://entities/flora/euc_regnans.10d#lod_sec1"
    },
    "tier_3_impostor": {
      "type": "octahedral_atlas",
      "resolution": 32,
      "grid_dim": 16,
      "data_ref": "hmc://entities/flora/euc_regnans.10d#impostor_sec19"
    },
    "tier_4_surfel": {
      "type": "hierarchical_gaussian_splat",
      "data_ref": "hmc://entities/flora/euc_regnans.10d#surfel_sec3"
    }
  },
  "physics_and_affordances": {
    "wind_compliance": 0.85,
    "shedding_affordance": {
      "action_id": "affordance:shed_bark_ribbons",
      "trigger_season": "summer",
      "preconditions": ["temperature_celsius >= 30.0"],
      "emitted_debris": "hmc://entities/debris/bark_ribbon.10d"
    }
  }
}
```

---

## 5. Canonical Authoring Template: Engineered Artifact (Solar Centrifugal Pump)

Save as `entities/machinery/solar_centrifugal_pump/entity.json`:

```json
{
  "entity_id": "urn:qualia:artifact:solar_pump_centrifugal_v2",
  "ontology_class": "ManufacturedProperty",
  "embodiment_term": "Construct",
  "manufacturing_data": {
    "manufacturer_did": "did:q42:corp:qualia_hardware_consortium",
    "model_name": "HydroVibe-400X",
    "serial_number": "HV400X-2026-09A",
    "dry_mass_kg": 42.5,
    "ip_rating": "IP68"
  },
  "spectral_profile": {
    "materials": [
      {
        "id": "casing_machined_steel",
        "type": "conductor",
        "complex_refractive_index": {
          "n_polynomial": [1.50, 0.20, -0.05],
          "k_polynomial": [2.80, 0.50, 0.10]
        },
        "roughness": 0.25,
        "metallic": 1.0
      },
      {
        "id": "solar_glass_cover",
        "type": "thin_film_dielectric",
        "basis_coefficients": {
          "c0": 0.000,
          "c1": 0.000,
          "c2": 0.045
        },
        "thin_film_thickness_nm": 450.0,
        "thin_film_ior": 1.45
      }
    ]
  },
  "procedural_synthesis": {
    "seed": "0x4D41_4348_5055_4D50",
    "stress_tensor_fracture": {
      "critical_stress_mpa": 450.0,
      "cleavage_plane_normal": [0.0, 1.0, 0.0]
    }
  },
  "geometry_tiers": {
    "tier_1_hero": {
      "type": "meshlet_cluster",
      "data_ref": "hmc://entities/machinery/pump.10d#meshlets_sec1",
      "kinematic_rig": "hmc://entities/machinery/pump.10d#rotor_armature_sec16"
    },
    "tier_2_decimated": {
      "type": "quantized_index",
      "data_ref": "hmc://entities/machinery/pump.10d#lod_sec1"
    },
    "tier_3_impostor": {
      "type": "octahedral_atlas",
      "resolution": 16,
      "grid_dim": 8,
      "data_ref": "hmc://entities/machinery/pump.10d#impostor_sec19"
    },
    "tier_4_surfel": {
      "type": "point_proxy",
      "data_ref": "hmc://entities/machinery/pump.10d#surfel_sec3"
    }
  },
  "physics_and_affordances": {
    "operational_rpm": 1450.0,
    "max_head_meters": 65.0,
    "max_flow_liters_min": 250.0,
    "wear_curves": {
      "bearing_degradation_per_1000h": 0.035,
      "impeller_cavitation_wear_factor": 0.012,
      "solar_glass_dust_soiling_rate": 0.002
    },
    "affordances": [
      {
        "action_id": "affordance:engage_solar_pump",
        "preconditions": [
          "solar_irradiance_w_m2 >= 200.0",
          "water_inlet_head_m >= 0.5"
        ],
        "effects": [
          "state.active = true",
          "flow_rate_l_min = 250.0 * (solar_irradiance / 1000.0)"
        ]
      }
    ]
  }
}
```

---

## 6. Offline Rust Compilation Pipeline (`compile_entity`)

To compile authored assets into `.10d` containers without Python dependencies, the compiler uses pure-Rust routines:

1. **Spectral Inversion:** Ingests legacy sRGB textures and converts them into $(c_0, c_1, c_2)$ using [`spectral_lut::sample_spectral_lut_trilinear`](file:///c:/github/qualiaDB/crates/qualia-core-db/src/render/spectral_lut.rs#L241).
2. **Meshlet Generation:** Groups triangles into 128-triangle clusters and calculates cone normals.
3. **Octahedral Atlas Baking:** Bakes the 16x16 directional views into Section 19 (`IMPO`) via [`impostor_section::encode_impostor_section`](file:///c:/github/qualiaDB/crates/qualia-core-db/src/container_10d/impostor_section.rs#L73).
4. **Entity Ontology Section:** Encodes the affordances and traits into Section 17 (`EONT`) via [`entity_ontology_section::encode_entity_ontology_section`](file:///c:/github/qualiaDB/crates/qualia-core-db/src/container_10d/entity_ontology_section.rs).
5. **Procedural Synthesis Section:** Encodes seeds and allometric ODE parameters into Section 18 (`PROC`) via [`procedural_synthesis::encode_procedural_synthesis_section`](file:///c:/github/qualiaDB/crates/qualia-core-db/src/container_10d/procedural_synthesis.rs#L83).
6. **Container Sealing:** Emits the final `.10d` file with per-section CRC-32C checksums and whole-file integrity verification.

---

## 7. CLI Subcommand Contract: `qualia-cli entity-compile`

Before packing and signing the `.10d` living container and `.hmc` package, the offline compiler executes deterministic validation gates implemented in [`render/entity_compile_gates.rs`](file:///c:/github/qualiaDB/crates/qualia-core-db/src/render/entity_compile_gates.rs) (and accessible via backward-compatible shim [`render/twin_compile_gates.rs`](file:///c:/github/qualiaDB/crates/qualia-core-db/src/render/twin_compile_gates.rs)) to guarantee runtime stability on zero-heap edge environments:

### Gate 1: Spectral Plausibility & Energy Conservation
* **Physical Reflectance Bounds:** Assert that all extracted material basis coefficients $(c_0, c_1, c_2)$ evaluate to $R(\lambda) \in [0.0, 1.0]$ at $10\text{ nm}$ increments across $\lambda \in [380, 780]\text{ nm}$ via `validate_spectral_coeffs`.
* **BSSRDF Positivity:** For Subsurface Scattering, assert that mapped mean free path parameters $\vec{l}_d$ are strictly positive non-zero, preventing division-by-zero singularities in the compute blur kernels (`validate_bssrdf_parameters`).
* **Gamut Inset Rejection:** Reject any legacy sRGB inputs that generate negative energy coefficients during the Levenberg-Marquardt LUT lookup.

### Gate 2: Virtualized Geometry Confinement
* **Hero Bounding Volumes:** Calculate the absolute bounding sphere and Axis-Aligned Bounding Box (AABB) of the Tier 1 Hero Mesh.
* **Impostor Projection Alignment:** Verify that the Section 19 Octahedral Impostor atlas rendering perfectly aligns its projection matrix with this bounding sphere.
* **Child LOD Confinement:** Assert that Tier 2 (Decimated) and Tier 4 (Hierarchical Surfel/Splat) primitives remain strictly contained within the established Tier 1 AABB (within a 5 cm decimation margin) via `validate_geometry_confinement`, preventing camera frustum culling mismatches.

### Gate 3: Ontological & Kinematic Integrity
* **Tripartite Semantic Resolution:** Resolve the entity's `ontology_class` IRI against the local `.q42` graph dictionary. Reject compilation if the ontological classification (`DarwinianTaxon`, `NaturalPerson`, `ManufacturedProperty`, `LiberalArtsWork`) is malformed or missing mandatory semantic edges (`validate_ontological_integrity`).
* **Kinematic Armature Normalization:** If the manifest specifies a `kinematic_rig`, parse the Tier 1 mesh data to assert that every vertex possesses valid, normalized bone weights ($\sum w_i = 1.0$) mapping to valid skeletal joint indices.
* **Physical Invariant Verification:** Enforce required physical constants for runtime simulation (e.g. `dry_mass_kg > 0.0` for `ManufacturedProperty` rigid bodies).

### Gate 4: Wasm / VibeScript Determinism Limits
* **Sandboxed Validation:** If the entity bundles procedural affordances (e.g. `voronoi_peel_script`, `bark_shedding_rule`), execute the bytecode inside a headless Wasm validator (`validate_script_determinism`).
* **Zero Non-Deterministic APIs:** Assert zero unauthorized host-calls and zero non-deterministic system API calls (such as unseeded entropy or system clock reads).
* **Instruction Budget:** Fail closed if execution exceeds the predefined cycle budget ($1,000,000$ instructions).

### Gate 5: Anti-Commodification of Natural Persons (OHCHR Human Rights Invariant)
* **Inalienable Personhood:** Enforced by `assert_person_anti_commodification`. If the entity represents a `NaturalPerson`, it cannot be designated as a commercial asset or manufactured property.
* **Fails Closed:** Rejects any compilation attempt targeting a human being with commercial asset flags (`GateError::HumanPersonCannotBeCommodifiedAsAsset`).
* **Consent & Sovereign Governance:** Sovereign human representations must be compiled as `Persona` manifests backed by sovereign DID verification keys.

---

## 8. WebGPU Deferred Resolve & Hero Wavelength TAA Integration

### 8.1 Tier-1 Bind Group Layout (Deferred Resolve Compute Shader)
Organizes resources strictly by update frequency across 3 of the 4 tier-1 bind group slots, leaving `@group(3)` free for dynamic environment maps, shadow atlases, or transient localized data:

```wgsl
// Group 0: Frame-Global Data (Updates once per frame)
@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<storage, read> cluster_grid: array<u32>;
@group(0) @binding(2) var<storage, read> light_data: array<Light>;
@group(0) @binding(3) var blue_noise_tex: texture_2d<f32>;
@group(0) @binding(4) var clamp_sampler: sampler;

// Group 1: Pass-Specific G-Buffer Inputs (Updated per pass)
@group(1) @binding(0) var gbuffer0: texture_2d<f32>; // c0, c1, c2, Roughness
@group(1) @binding(1) var gbuffer1: texture_2d<f32>; // Normal.xy, Metallic, Flags
@group(1) @binding(2) var depth_buffer: texture_depth_2d;

// Group 2: Output Targets (Updated per dispatch)
@group(2) @binding(0) var resolve_out: texture_storage_2d<rgba16float, write>;

// Group 3: Reserved for dynamic environment maps, shadow atlases, or transient probes
```

*Packing `CameraUniform` and light buffers into `@group(0)` minimizes descriptor set swapping on edge GPUs. G-Buffers are bound once per deferred resolve in `@group(1)`, while output storage in `@group(2)` isolates read-write hazard domains.*

### 8.2 Temporal Anti-Aliasing (TAA) for Hero Wavelengths
Temporal accumulation operates **after** projecting spectral radiance into CIE XYZ tristimulus space via Wyman et al. (2013) CMFs.

1. **Perceptual Oklab Neighborhood Clamping:**
   - Neighborhood clamping performed in **Oklab** color space via [`clamp_taa_history_oklab`](file:///c:/github/qualiaDB/crates/qualia-core-db/src/render/entity_compile_gates.rs#L202).
   - Sample $3 \times 3$ pixel neighborhood in CIE XYZ $\to$ transform to Oklab $\to$ compute AABB or variance bounding box in Oklab $\to$ clamp reprojected history in Oklab $\to$ transform back to linear CIE XYZ for blending.
   - *Rationale:* Clamping in spectral or linear XYZ causes severe chromatic shifts and metameric breakdown at high luminance. Oklab isolates lightness variance from chromaticity variance, ensuring that specular spectral glints do not drift in hue.

2. **Dynamic Blend Factor $\alpha$:**
   - Evaluated via [`calculate_taa_blend_factor`](file:///c:/github/qualiaDB/crates/qualia-core-db/src/render/entity_compile_gates.rs#L223).
   - **Static Pixels ($\text{Velocity} \approx 0$):** $\alpha = 0.05$ (20-frame accumulation window). Fully converges blue-noise spectral dither into a noiseless XYZ color.
   - **Dynamic Pixels ($\text{Velocity} > 0$):** Ramp $\alpha$ up to $0.15 \sim 0.20$ based on velocity magnitude. While this shortens the spectral integration window to 5–6 frames, the human visual system's Contrast Sensitivity Function (CSF) drops under retinal slip, masking the noise dynamically.

---

## 9. Multi-WASM Worker Topology & Shared WebGPU Brokerage

To bypass the 32-bit $4\text{ GiB}$ address space ceiling and single-threaded execution bottleneck in browser environments, execution is partitioned across isolated workers:

1. **Root Orchestrator (Webizen Sentinel - Worker 1):**
   - Operates inside the **42MB Sentinel arena** (`SlgArena`).
   - Owns Q42 graph, SHACL rules, and N3 forward chaining. Unpacks `.hmc` containers and transfers `.10d` geometry buffers zero-copy (`postMessage(..., [transferredBuffer])`) to the viewport worker.
2. **Viewport Engine (Worker 2):**
   - Owns `OffscreenCanvas`. Ingests geometry streams using `queue.writeBuffer()` for non-blocking asynchronous DMA transfer without synchronous map/unmap stalls or GC pressure.
   - **Section 20 Surfel Instancing:** Rather than using compute software rasterization with 64-bit atomics (unsupported on mobile WebGPU), surfels (1–8 px screen radius) are expanded to camera-facing quads in the vertex shader via `drawIndirect`, leveraging mobile Tile-Based Deferred Rasterization (TBDR) Hidden Surface Removal (HSR) / Forward Pixel Kill (FPK) for zero fragment overhead on occluded surfels.
3. **Single WebGPU Hardware Broker:**
   - Spawning multiple isolated `GPUDevice` contexts across separate workers on unified memory platforms (Apple Silicon M-series, Mali, Adreno) triggers OS OOM crashes due to uncoordinated physical VRAM oversubscription.
   - A single WebGPU broker holds the physical `GPUDevice` and sequences render dispatches and neural compute commands through shared command queues.

---

## 10. Polynomial Continuous Spectral Acoustics & AudioWorklet Spatialization

Physical acoustics are parameterized symmetrically to Jakob-Hanika optical reflectance:

1. **Logarithmic Frequency Normalization:**
   Maps the human audible spectrum ($20\text{ Hz}$ to $20\text{ kHz}$) to normalized frequency $u \in [0.0, 1.0]$:
   $$u = \frac{\log_{10}(f / 20.0)}{\log_{10}(1000.0)} = \frac{\log_{10}(f / 20.0)}{3.0}$$

2. **Polynomial Resonance & Material Occlusion Basis:**
   Surface acoustic transmission, wall occlusion, and foliage absorption follow the algebraic sigmoid:
   $$H(u) = 0.5 + 0.5 \cdot \frac{f(u)}{\sqrt{1.0 + f(u)^2}}, \quad f(u) = c_0 u^2 + c_1 u + c_2$$

3. **Quantum-Rate Biquad Synthesis (Zero Per-Sample Overhead):**
   - Evaluated **once per 128-sample quantum ($2.6\text{ ms}$)** at 3 anchor frequencies (Low: 200 Hz, Mid: 1 kHz, High: 8 kHz) rather than per-sample.
   - Configures a 3-band parametric biquad filter with coefficients linearly smoothed across the 128 samples to prevent zipper noise.

4. **Duplex KemarLite Analytical Spatialization (`audio/hrtf.rs`):**
   - **Low Frequencies ($f < 1.5\text{ kHz}$):** Localized via Interaural Time Difference (ITD) using a fractional-delay circular buffer ($\pm 650\text{ }\mu s$), taking $< 4$ cycles per sample with linear interpolation.
   - **High Frequencies ($f \ge 1.5\text{ kHz}$):** Localized via Interaural Level Difference (ILD) and spherical head shadow using a single 1-pole / 1-zero IIR filter per ear.
   - **Total Compute:** $\approx 10$ FLOPs per sample per voice, scaling easily to 64 concurrent voices within a $0.5\text{ ms}$ budget.
   - **Shared Reverb Bus:** Emitters send distance-attenuated signals to a single shared Feedback Delay Network (FDN) reverberator per zone parameterized by 10D tensor manifold coordinate $w$ (`room_damp`).

---

## 11. Voxel Cone Tracing WGSL Compute Kernel Details

Implemented in [`shaders/viewport/acoustic_voxel_cone.wgsl`](file:///c:/github/qualiaDB/crates/qualia-core-db/src/shaders/viewport/acoustic_voxel_cone.wgsl) and registered in [`shaders/viewport/mod.rs`](file:///c:/github/qualiaDB/crates/qualia-core-db/src/shaders/viewport/mod.rs#L37).

### Optimal Step Size & Trilinear Accumulation
Because the cone radius expands as a function of distance from the listener, a constant step size causes massive oversampling near the origin and severe undersampling at a distance. The ray-marching step size $\Delta t$ scales proportionally with the cone radius $r = t \cdot \tan(\theta/2)$, bounded by the voxel dimension $\Delta s$:

$$\Delta t = \max\left(\Delta s, \; t \cdot \tan\left(\frac{\theta}{2}\right)\right)$$

For accumulation, map the `rgba8uint` packed acoustic absorption values to a float vector $[0.0, 1.0]$. Use standard front-to-back volumetric alpha compositing to maintain the transmitted acoustic energy (Transmittance $T$) across the 3 frequency bands:

$$T_k = T_{k-1} \cdot (1.0 - \alpha_k \cdot \Delta t)$$

When $T$ drops below the audibility threshold ($0.01$) for all 3 bands, terminate the ray early to conserve compute cycles.

### Toroidal Wrapping & Trilinear Fetching
WebGPU compute shaders cannot use hardware `sampler` trilinear filtering on `texture_storage_3d`. A manual 8-tap trilinear interpolation is performed. To prevent boundary seam artifacts when fetching the 8 cube corners, the toroidal modulo is applied **after** computing the discrete integer coordinates for each corner, but **before** the texture fetch:

```wgsl
fn sample_toroidal_volume(world_pos: vec3<f32>) -> vec4<f32> {
    let v_float = world_pos / config.voxel_size;
    let v_int = vec3<i32>(floor(v_float));
    let fract_v = fract(v_float);

    var samples: array<vec4<f32>, 8>;
    var idx = 0u;

    for (var z = 0; z <= 1; z++) {
        for (var y = 0; y <= 1; y++) {
            for (var x = 0; x <= 1; x++) {
                let offset = vec3<i32>(x, y, z);
                let coord = v_int + offset;
                let wrapped_coord = vec3<u32>(((coord % 32) + 32) % 32);
                let raw_val = textureLoad(acoustic_volume, wrapped_coord);
                samples[idx] = vec4<f32>(raw_val) / 255.0;
                idx++;
            }
        }
    }

    let mix_x0 = mix(samples[0], samples[1], fract_v.x);
    let mix_x1 = mix(samples[2], samples[3], fract_v.x);
    let mix_x2 = mix(samples[4], samples[5], fract_v.x);
    let mix_x3 = mix(samples[6], samples[7], fract_v.x);
    let mix_y0 = mix(mix_x0, mix_x1, fract_v.y);
    let mix_y1 = mix(mix_x2, mix_x3, fract_v.y);
    return mix(mix_y0, mix_y1, fract_v.z);
}
```

---

## 12. AudioWorklet 3-Band Parametric Biquad Implementation

Implemented in [`audio/dsp/biquad_df2t.rs`](file:///c:/github/qualiaDB/crates/qualia-core-db/src/audio/dsp/biquad_df2t.rs) and re-exported in [`audio/mod.rs`](file:///c:/github/qualiaDB/crates/qualia-core-db/src/audio/mod.rs#L38).

### Direct Form Selection & Denormal Prevention
Uses **Direct Form II Transposed (DF2T)**. In 32-bit floating-point DSP, DF2T provides superior numerical stability against coefficient quantization and minimizes accumulated rounding noise for low-frequency poles (e.g., the 200 Hz anchor). It also reduces memory footprint to just 2 state variables per biquad instead of 4.

Mobile ARM NEON FPUs suffer catastrophic slowdowns (up to 100x performance penalties) when processing subnormal numbers (denormals) resulting from IIR filter delay lines decaying toward zero. To prevent this without relying on Javascript engine flush-to-zero (FTZ) flags, inject a microscopic DC offset directly into the input signal:

$$x[n] = x_{\text{raw}}[n] + 10^{-24}$$

This guarantees the state registers never enter the denormal range, preserving the 800 MFLOPS budget.

### Coefficient Ramping
Recalculating trigonometric functions for biquad coefficients $(b_0, b_1, b_2, a_1, a_2)$ per sample breaks the 2.67ms deadline.
1. Calculate the 5 filter coefficients exclusively at the boundary of the 128-sample quantum.
2. Determine the per-sample delta between the previous quantum's coefficients and the new target coefficients:

$$\Delta b_i = \frac{b_{i,\text{target}} - b_{i,\text{prev}}}{128}$$

3. Inside the inner `for i in 0..128` sample loop, add the delta to the active coefficients. Because the 2.67ms window is so brief, linear interpolation of the DF2T coefficients directly is mathematically stable for shelving and peaking filters and costs only 5 scalar additions per sample, completely eliminating audio artifacts and zipper noise.

---

## 13. `qualia-cli entity-compile` Asset Packaging Flow

Implemented in [`bundle/cbor_ld_manifest.rs`](file:///c:/github/qualiaDB/crates/qualia-core-db/src/bundle/cbor_ld_manifest.rs), [`bundle/entity_compiler.rs`](file:///c:/github/qualiaDB/crates/qualia-core-db/src/bundle/entity_compiler.rs), and [`qualia-cli/src/handlers/entity_compile.rs`](file:///c:/github/qualiaDB/crates/qualia-cli/src/handlers/entity_compile.rs) (with legacy backward-compatibility shims retained in [`bundle/twin_compiler.rs`](file:///c:/github/qualiaDB/crates/qualia-core-db/src/bundle/twin_compiler.rs) and [`qualia-cli/src/handlers/twin_compile.rs`](file:///c:/github/qualiaDB/crates/qualia-cli/src/handlers/twin_compile.rs)).

### Section Sequencing for Data Locality
To optimize memory-mapped file (mmap) performance on edge devices, binary sections are sequenced in **Reverse-LOD order**. This guarantees that the operating system pages in the initial chunks of the file to render the macro-representation immediately, faulting in high-density meshlets only when camera proximity demands it:

1. **Header & CBOR-LD Manifest** (Instant parse, cryptographic verification)
2. **Section 20: SURF1 Hierarchical Surfels** (Global far-field view)
3. **Section 19: IMPO1 Octahedral Atlases** (Mid-to-far field proxy)
4. **Section 2: Decimated Meshlets** (Mid-range geometry)
5. **Section 1: Hero Meshlets** (Close-up, heavy data payload)

### CBOR-LD Manifest Layout
The manifest acts as a self-contained W3C Verifiable Credential bridging the Webizen identity layer with the physical payload. It uses `credentialSubject` to bind the Ontological Entity Representation to the binary offsets, allowing the Sentinel worker to verify structural integrity before touching the geometry:

```json
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "hmc://schemas/webcivics/v1"
  ],
  "id": "urn:uuid:3f8a...",
  "type": ["VerifiableCredential", "QualiaEntity"],
  "issuer": "did:web:trustfactory.org:compiler",
  "issuanceDate": "2026-10-13T08:46:00Z",
  "credentialSubject": {
    "id": "urn:qualia:taxon:eucalyptus_regnans_01",
    "type": "DarwinianTaxon",
    "embodiment_term": "Specimen",
    "affordances": {
      "procedural_seed": "hmc://seeds/voronoi_bark",
      "acoustic_anchors": { "low_hz": 200, "mid_hz": 1000, "high_hz": 8000 }
    },
    "geometry_layout": {
      "surf1": { "offset": 2048, "length": 32768, "digest": "sha256-..." },
      "impo1": { "offset": 34816, "length": 524288, "digest": "sha256-..." },
      "mesh1": { "offset": 559104, "length": 14500000, "digest": "sha256-..." }
    }
  },
  "proof": {
    "type": "Ed25519Signature2020",
    "created": "2026-10-13T08:46:05Z",
    "verificationMethod": "did:web:trustfactory.org:compiler#key-1",
    "proofPurpose": "assertionMethod",
    "proofValue": "z58D..."
  }
}
```
