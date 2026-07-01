# QualiaDB Multi-Agent Collaboration Pipeline: Quantum Chemistry
_Branch: `0.0.17-dev`_

This document extends the Global `AGENTS.md` rules with specific targets and pipelines to build the Quantum electronic-structure Hartree-Fock (HF) and Density Functional Theory (DFT) engine (Challenge #20). 

These tasks are designed for independent, parallel AI subagent swarm execution.

---

## 4. Quantum Chemistry Task Map 

Each task below is scoped to be completable in one session (≤ 2h of code). Tasks are
**independent** — they do not depend on each other unless noted. All implementations MUST adhere strictly to the global 42MB Prolog Sentinel and Zero-Heap hot-path rules.

---

### Task H — Basis Set & Spatial Discretization
**Target:** `crates/qualia-core-db/src/specialized_libs/chemistry_modeling/basis_set.rs`  
**Objective:** 
- Implement `category_theory::Object` structures to represent contracted Cartesian and real-spherical Gaussian-Type Orbitals (GTOs).
- Create a deserialization module that natively parses JSON atomic basis datasets (STO-3G, def2-SVP) from the Basis Set Exchange.
- Automate the parsing of Effective Core Potentials (ECPs) for heavy elements to include scalar-relativistic effects without intractable cost.

### Task I — Analytical Integral Engine
**Target:** `crates/qualia-core-db/src/specialized_libs/chemistry_modeling/integrals.rs`  
**Objective:** 
- Build the core molecular integral evaluator for overlap, kinetic, nuclear attraction, and two-electron repulsion integrals (ERIs).
- Implement an adaptive dispatch policy:
  - Utilize the **Obara-Saika (OS) / Head-Gordon-Pople (HGP)** recursive schemes for low angular momentum (s, p, d).
  - Utilize **Rys Quadrature** to bypass deep recursion for high angular momentum (f, g) orbitals.
- All integral tensors must be represented using `shared::ZeroHeapMatrix`.

### Task J — Self-Consistent Field (SCF) Iterative Driver
**Target:** `crates/qualia-core-db/src/specialized_libs/chemistry_modeling/scf.rs`  
**Objective:** 
- Solve the nonlinear generalized Roothaan-Hall eigenvalue problem ($FC = SCE$).
- Handle Restricted (RHF) and Unrestricted (UHF) formalisms.
- Implement advanced quasi-Newton convergence acceleration specifically **Direct Inversion in the Iterative Subspace (DIIS)** and **Broyden mixing** to mathematically manage charge-sloshing instabilities in transition metal complexes.
- Implement objective markers to mandate $10^{-8}$ Hartree energy convergence.

### Task K — Density Functional Theory (DFT) Integration
**Target:** `crates/qualia-core-db/src/specialized_libs/chemistry_modeling/dft.rs`  
**Objective:** 
- Extend the SCF driver to include Exchange-Correlation Integration.
- Implement numerical grid evaluation for Local Density Approximation (LDA) and Generalized Gradient Approximation (GGA).
- **Rule:** Do NOT use C-bindings to `libxc`. You must utilize Rust-native automatic differentiation (autodiff) frameworks to compute exact analytical derivatives of the functional expressions.

### Task L — Final Q-Forge Semantic Bridge (Current Swarm Priority)
**Target:** `crates/qualia-extensions/src/qpu_extension.rs`  
**Objective:** 
- Finalize the `qualia-q-forge` bridge implementation.
- Convert final StateVector responses to deterministic `NQuin` topological pointers.
- Construct unit tests asserting that the DQC API scheduler correctly leverages the **QGroup heuristic** (sorting the VQE job queue by similarity in circuit depth and total shot count) to perfectly mitigate parallel hardware slowdowns.
