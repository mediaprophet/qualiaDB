---
layout: page
title: Hard Sciences & Advanced Logic Showcase
permalink: /hard-sciences-showcase/
---

# Hard Sciences & Advanced Logic Showcase

The Qualia-DB engine has evolved far beyond a typical semantic database. It is now a **bifurcated compute fabric** capable of performing continuous mathematical, physical, and biological calculations natively on the graph without allocation overhead. 

This document serves as a comprehensive showcase of the newly integrated capabilities.

---

## 1. The Bifurcated Compute Fabric
The engine dynamically splits its operational logic depending on the host environment:

* **The Swarm (Native 64-bit Daemon):** Operates on Desktop and powerful host nodes. It fully unleashes the host hardware—spanning up "Fractal Shards" (isolated 512MB worker cells) to run heavy continuous computations, ODE solvers, and tensor contractions natively on the CPU/GPU.
* **The WASM Edge Router:** Operates securely inside Browser and Mobile WebView environments. It handles topological pruning, Rights Ontology (SHACL) enforcement, and gracefully delegates intense calculations out to a Federated Swarm node via WebRTC.

---

## 2. Webizen VM: Advanced Reasoning Expansion
The Bytecode Abstract Machine now handles the uncertainties and time-domains of the physical sciences.

* **Continuous Constraints:** The VM supports exact scalar comparisons (`<`, `>`, `<=`, `>=`) natively against tagged floating-point pointers, without boxing.
* **Temporal Logic (LTL/CTL):** Implements time-series evaluation allowing constraints like `Always`, `Eventually`, and `Next` across temporal semantic blocks.
* **IEEE-754 Float & Complex Number Tagging:** Floating-point arrays are embedded directly into the 64-bit `Object` vectors using the top 4 bits as type tags. Complex numbers ($a + bi$) are natively supported for quantum wavefunction evaluations.
* **Calculated Consequences:** Rules can mathematically transform graph states. (e.g., Yield a new Quin where `Object = Mass × Acceleration`).
* **Stochastic/Fuzzy Logic:** Parses the 5th Metadata Vector to represent probability weights and Monte Carlo confidence intervals.

---

## 3. Pure-Rust Physics & Engineering Engines
Heavy physical calculations are now executed entirely natively, eliminating C-library dependencies to maintain memory safety and speed.

* **Differential Equation Solvers (ODEs):** Features a pure-Rust Runge-Kutta 4th Order solver to calculate continuous time-series dynamics natively in memory.
* **Quantum DFT & PINNs:** Evaluates molecular graphs and binds to Physics-Informed Neural Networks (PINNs). Predicts binding affinities and ground states cleanly.
* **Thermodynamics:** Calculates macroscopic properties (Entropy, Gibbs Free Energy) via Markov Chain Monte Carlo (MCMC) stochastic sampling.
* **FEA Sparse Meshing:** Incorporates 3D meshing indices (voxels/tetrahedra) directly into `.q42.bidx` block files for fast structural stress computations.

---

## 4. Hardware-Aware Hardware Acceleration
Qualia-DB natively adapts to diverse architectures to squeeze out maximum performance from the host silicon.

* **GPU Compute Shaders (`wgpu`):** Features custom compute pipelines for `kinematics.wgsl`, `fluid_dynamics.wgsl`, and `molecular_dynamics.wgsl`. It leverages **Vulkan, DirectX 12, or Apple Metal** (depending on the OS) to run massive N-body simulations and apply Periodic Boundary Conditions (PBCs) directly on the graphics card (including NVIDIA and AMD GPUs).
* **SIMD Bioinformatics:** Sequence alignment (e.g., Smith-Waterman modifications) utilizes a **Define-Detect-Dispatch** pattern. 
  * On **Apple M-Series & Snapdragon (ARM64)**, it natively hooks into the NEON vector paths.
  * On **Intel/AMD Desktop**, it dynamically tests for Advanced Vector Extensions (`AVX2`) before unrolling the alignment loops across the CPU registers.

---

## 5. Mobile & Desktop Integration
Because these features are deeply embedded in the `qualia-core-db` foundation, they permeate through all client interfaces.

* **Qualia Vault (Android):** The mobile client runs the native Rust engine on ARM64, accessing biometric JNI wrappers and NEON vector capabilities natively on-device.
* **Qualia Desktop (Tauri):** The desktop Webizen environment orchestrates the Swarm daemon, capable of ingesting vast libraries of semantic data, dispatching local LLM inference, and unleashing the full power of a dedicated NVIDIA/AMD GPU for quantum and engineering computations.

---

## 6. Specialized Library Extensions

Ten domain-specific extension libraries are planned under the advanced extensions framework,
covering: Chaos Theory, Classical/Relativistic Mechanics, Complex Analysis, Differential Geometry,
Electromagnetism, Fluid Dynamics, Group Theory, Information Theory, Number Theory, and Statistical Mechanics.

Infrastructure is complete: PINN (ternary quantisation + SMX), SNN (noisy-gradient CRDT), WebGPU compute, and QPU bridge are all implemented.

The Geometric Algebra SIMD kernel is **implemented** in `crates/qualia-core-db/src/geometric_algebra/simd_kernel.rs` — AVX2/FMA-accelerated Multivector, Rotor, and Translator operations with NQuin integration.

Six compiled domain engines exist under `crates/qualia-core-db/src/domains/`:
- **Bioinformatics** (Smith-Waterman/Needleman-Wunsch, BLOSUM62, k-mer, MinHash, Tanimoto)
- **Organic Chemistry** (SMILES, Lipinski/Veber/Ghose/Egan filters, LogP, Morgan fingerprint, Arrhenius, Gibbs-Helmholtz, Green chemistry)
- **Thermodynamics** (Metropolis-Hastings MCMC, Gibbs free energy — covers Statistical Mechanics)
- **Geometric** (LorentzVector / Minkowski space, tropical geometry — covers Classical/Relativistic Mechanics)
- **Financial** and **Geospatial** domain models

Statistical Mechanics, Classical/Relativistic Mechanics, Differential Geometry, and Fluid Dynamics have substantial partial coverage through the above compiled code.
Electromagnetism and Chaos Theory have PINN/WebGPU enum wiring.
Number Theory, Information Theory, and Group Theory have no dedicated code yet.

Nine large specialized library files (physics simulation, chemistry modeling, engineering analysis, cryptographic, statistical computing, etc.) exist in `specialized_libs/` but are currently disabled due to build errors.

See the full status breakdown in [`docs/specialized-library-extensions-status.md`](specialized-library-extensions-status.md).
