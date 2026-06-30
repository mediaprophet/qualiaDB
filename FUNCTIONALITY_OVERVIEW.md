# QualiaDB System Functionality Overview

This document serves as the high-level functional map of the QualiaDB multi-agent collaboration ecosystem. It connects the major architectural domains of the system to their respective directory indexes, which in turn contain the comprehensive, extracted structural components (functions, structs, traits).

When bots or developers need to understand a specific domain, they should navigate to the linked **Directory Index** below for a granular view of all implemented functionality.

---

## 1. Core Database Engine (`qualia-core-db`)

The heart of the system is the zero-allocation, deterministic evaluator engine for semantic storage and multi-modal logic reasoning. It implements the "Quin" 48-byte layout, LTL (Linear Temporal Logic), and deontic frameworks.

- **Main Component Index:** [crates/qualia-core-db](crates/qualia-core-db/DIRECTORY_INDEX.md)
- **Modalities (Epistemic, Deontic, Paraconsistent):** [crates/qualia-core-db/src/modalities](crates/qualia-core-db/src/modalities/DIRECTORY_INDEX.md)
- **Domain Logic (Biological, Chemical, Financial):** [crates/qualia-core-db/src/domains](crates/qualia-core-db/src/domains/DIRECTORY_INDEX.md)
- **Solvers & Optimization (Calculus, Quantum, Graph Opt):** [crates/qualia-core-db/src/solvers](crates/qualia-core-db/src/solvers/DIRECTORY_INDEX.md)
- **GGUF Bridge (LLM Inference Pipeline):** [crates/qualia-core-db/src/gguf_bridge](crates/qualia-core-db/src/gguf_bridge/DIRECTORY_INDEX.md)
- **Storage Layer (CRDT, WAL, Memory Arena):** [crates/qualia-core-db/src/storage](crates/qualia-core-db/src/storage/DIRECTORY_INDEX.md)

---

## 2. WGSL Forge & GPU Rendering (`wgsl_forge` / `webizen-render`)

A major sub-component of the core is the N-Dimensional Renderer SDK and shader pipeline, executing compute kernels directly on WebGPU for accelerated tensor logic, physics, and volumetric rendering.

- **WGSL Forge Engine:** [crates/qualia-core-db/src/wgsl_forge](crates/qualia-core-db/src/wgsl_forge/DIRECTORY_INDEX.md)
- **Forge IR (Intermediate Representation):** [crates/qualia-core-db/src/wgsl_forge/ir](crates/qualia-core-db/src/wgsl_forge/ir/DIRECTORY_INDEX.md)
- **Forge Emit (WGSL Generation):** [crates/qualia-core-db/src/wgsl_forge/emit](crates/qualia-core-db/src/wgsl_forge/emit/DIRECTORY_INDEX.md)
- **Webizen Rendering Pipeline:** [crates/webizen-render](crates/webizen-render/DIRECTORY_INDEX.md)
- **GPU Shaders:** [crates/qualia-core-db/src/shaders](crates/qualia-core-db/src/shaders/DIRECTORY_INDEX.md)

---

## 3. Webizen Client Ecosystem (`webizen-*`)

The Webizen components encompass the client-facing interfaces, bridging the Core DB with the OS layer (Desktop, Web, WASM, and Studio).

- **Webizen Desktop App:** [crates/webizen-desktop](crates/webizen-desktop/DIRECTORY_INDEX.md)
- **Webizen Lite (WASM bridge for edge):** [crates/webizen-lite-wasm](crates/webizen-lite-wasm/DIRECTORY_INDEX.md)
- **Webizen Studio:** [crates/webizen-studio](crates/webizen-studio/DIRECTORY_INDEX.md)
- **Webizen Web Interface:** [crates/webizen-web](crates/webizen-web/DIRECTORY_INDEX.md)
- **Component Harvester:** [crates/webizen-component-harvester](crates/webizen-component-harvester/DIRECTORY_INDEX.md)
- **Webizen Runtime Engine:** [crates/webizen-runtime](crates/webizen-runtime/DIRECTORY_INDEX.md)

---

## 4. Semantic Knowledge Base & Ontologies

QualiaDB relies on a robust set of ontological standards (W3C, Schema.org, FIBO, UN Instruments) integrated directly into the inference system. 

- **Semantic Library Engine:** [crates/qualia-semantic-library](crates/qualia-semantic-library/DIRECTORY_INDEX.md)
- **Core Ontologies (Traces, Concepts, Regional):** [core-ontologies](core-ontologies/DIRECTORY_INDEX.md)
- **W3C Standards & Archives:** [bundled/ontologies/w3c](bundled/ontologies/w3c/DIRECTORY_INDEX.md)
- **Financial Industry Business Ontology (FIBO):** [bundled/ontologies/fibo](bundled/ontologies/fibo/DIRECTORY_INDEX.md)
- **UN Human Rights Instruments:** [core-ontologies/un-instruments](core-ontologies/un-instruments/DIRECTORY_INDEX.md)

---

## 5. Extensibility & Integration (`cli`, `mcp`, `solid`)

These modules provide the external boundaries, allowing agents, developers, and users to integrate with the Qualia engine via terminal, Solid protocol, or MCP servers.

- **Qualia CLI Tools:** [crates/qualia-cli](crates/qualia-cli/DIRECTORY_INDEX.md)
- **Qualia Extensions Layer:** [crates/qualia-extensions](crates/qualia-extensions/DIRECTORY_INDEX.md)
- **Solid Protocol Bridge:** [crates/qualia-solid-bridge](crates/qualia-solid-bridge/DIRECTORY_INDEX.md)
- **MCP Services (Anthropic Model Context Protocol):** [mcps/qualia](mcps/qualia/DIRECTORY_INDEX.md)

---

## 6. Mobile & Edge Computing

Frameworks and harnesses dedicated to running the inference engine on mobile and low-power hardware.

- **Mobile Harness:** [crates/qualia-mobile-harness](crates/qualia-mobile-harness/DIRECTORY_INDEX.md)

---

*Note to Bots & Agents: If you are tasked with modifying or understanding any of the above modules, follow the link to its `DIRECTORY_INDEX.md` file. That file contains the comprehensive, machine-generated inventory of every active function, struct, and class definition within that domain.*
