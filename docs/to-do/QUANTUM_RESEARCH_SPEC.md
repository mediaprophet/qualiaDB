# Specification: "Nexus" - Quantum Biology & Physics Research Cooperative
_Version: 1.0.0-draft | Target: Webizen Edge Platform (QApp)_

## 1. Executive Summary
"Nexus" is a specialized Webizen Social Cooperative (QApp) tailored for researchers operating in the domains of quantum physics, quantum biology, and advanced computational chemistry. 

Rather than relying on disjointed tools (PDF managers, chat apps, reference managers, and cloud compute), Nexus provides a unified, collaborative, and temporally-aware environment. It leverages the underlying QualiaDB engine's native hardware dispatch for physics and bioinformatics simulations, combined with a strict causal timeline that guarantees provenance and chronological integrity of all research artifacts.

---

## 2. Core Operational Paradigms

### 2.1 The "Living Research" Timeline (Temporal Ontology)
Research is inherently non-linear. Nexus abandons standard "folder" structures in favor of a 4D Temporal Timeline.
*   **Retrospective Injection:** When researchers discover an old paper, correspondence (e.g., an email from 2012), or dataset, it is injected into the semantic timeline at its *actual historical origin date*, not the date it was uploaded.
*   **Causal Provenance:** The system maintains strict causal links (using QualiaDB's temporal LTL logic). If a conclusion in 2026 was based on a flawed paper from 2018, modifying the status of the 2018 paper automatically cascades "epistemic uncertainty" flags to all downstream conclusions.
*   **Attribution & Authorship:** Every node created, edited, or ingested is cryptographically signed by the contributor's DID (Decentralized Identifier). "Who did what, when" is mathematically verifiable without centralized gatekeepers.

### 2.2 Native Simulation Dispatch (SlgOpcode)
Nexus does not require researchers to offload compute to external web APIs. It hooks directly into the QualiaDB zero-copy ABI for bare-metal performance:
*   **Quantum Biology / Bioinformatics:** Native execution of Smith-Waterman (SW) alignments, k-mer analysis, and protein structure processing.
*   **Quantum Physics & Chemistry:** Real-time computation of Density Functional Theory (DFT) ground states, Physics-Informed Neural Networks (PINN) for binding affinity, Thermodynamics MCMC, and RK4 ODE simulations.

---

## 3. Key Interface Modules

### 3.1 The Ingestion Engine (PDF & Link Processor)
A dedicated interface for rapid semantic hydration of external documents.
*   **PDF Parsing & Extraction:** Researchers drop PDFs into the workspace. The local Semantic Agent (Inforg) locally parses the document, extracts chemical structures (SMILES, InChI), mathematical formulae, and citation graphs.
*   **Semantic Highlighting:** Users highlight text in a PDF, turning it into a "Claim Node." This claim can then be connected via N3 logic rules to other claims (e.g., `[Paper A Claim] -> <Contradicts> -> [Paper B Claim]`).
*   **Web-Link Archiving:** URLs are fetched, archived locally via Information Centric Networking (ICN) hashes, and tokenized to ensure they never suffer from link rot.

### 3.2 Cooperative Research Canvas (Spatial View)
A shared, real-time spatial workspace mapped to the `CoordinateSpace` presentation layer.
*   **Multi-Agent Collaboration:** Researchers visualize the same complex multidimensional datasets (e.g., HDF5 containers or mapped quantum states) simultaneously.
*   **Dynamic Lenses:** Toggle specialized UI lenses to view the canvas mathematically (raw logic rules), visually (3D molecule rendering), or causally (chronological impact graphs).

### 3.3 Epistemic Discussion Threads
Standard comment threads are replaced with "Epistemic/Doxastic Logic" nodes.
*   When collaborating, researchers do not just "reply" to a thread. They assert claims using explicit modal logic (`OP_KNOWS`, `OP_BELIEVES`, `OP_COMMON_KNOWLEDGE`).
*   Disagreements trigger a "Paraconsistent Isolation" context, allowing two contradictory theories regarding a quantum biological mechanism to be developed simultaneously in parallel "world states" without breaking the main research database.

---

## 4. Collaborative Governance & Publishing

### 4.1 Permissive Commons Integration
*   Research data is published directly to the **Permissive Commons** using the WebTorrent DHT.
*   **Threshold Shift Licenses (TSL):** The cooperative can set explicit "Obligation Costs" (e.g., micropayments or peer-review labor) required for external entities to access their proprietary quantum models or datasets. Once the threshold is met, the data mathematically shifts to the public commons.

### 4.2 Bilateral Micro-Commons
*   Private, encrypted sub-networks can be established instantly with external universities or laboratories. Using the `EnforceBilateralMicroCommons` routing lane, sensitive experimental data (e.g., pre-published clinical or physical trials) is strictly governed by multi-signature smart contracts.

---

## 5. Technical Requirements for Webizen Studio Implementation
To implement "Nexus" within Webizen Edge, the following components must be built:
1.  **Canvas Layout:** A highly customized version of the `Infosphere` spatial canvas equipped with chemistry/physics rendering plugins (e.g., SMILES to 2D/3D structure renderers).
2.  **Timeline Component:** A dynamic scrubbing timeline UI using Shoelace components (`<sl-range>`) mapped to Lamport clocks and UNIX epochs.
3.  **WASM Pipeline Integration:** WebAssembly bindings that allow the UI to dispatch `SlgOpcode::NativePhysics` and `SlgOpcode::NativeBioinformatics` calls directly to the local Qualia daemon.
