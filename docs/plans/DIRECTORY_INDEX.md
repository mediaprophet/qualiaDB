---
created: 2026-06-30
updated: 2026-08-16
update_scope: Minor
---

# plans Index

## Functionality Overview

Architecture, implementation, optimisation, migration, and release plans for QualiaDB,
Webizen, inference, rendering, desktop/QApps, and supporting compute infrastructure.

## File & Subdirectory Manifest

- `0.0.17-docs-kit.md`: Documentation-kit plan for the 0.0.17 development line.
- `0.0.22-llm-gguf-optimization.md`: GGUF and local-LLM optimisation plan and measurements.
- `acceleration-integration-map.md`: Map of solver and inference operations to accelerated
  backends and kernels.
- `AUDIO_PROJECT_STATUS.md`: Audio subsystem capability and implementation status.
- `cooperative-qapps-desktop-implementation-plan.md`: Cooperative desktop/QApps architecture,
  workflows, governance, and staged implementation plan.
- `dag-ir-forge.md`: Typed compute-DAG design and staged multi-backend WGSL Forge implementation.
- `deterministic-wgsl-forge.md`: Deterministic kernel generation, validation, certification,
  tuning, and caching plan.
- `inference_optimisation.md`: Current inference performance analysis and optimisation backlog.
- `llm-on-forge-q42-p64.md`: Plan for running the local LLM through Forge using Q42 graph
  semantics and P64 weights.
- `native-auditory-language-and-music-intelligence.md`: Capability-aware plan for acoustic
  understanding, community-governed language resources, speech, music analysis/production,
  generative audio, and cross-modal perception.
- `native-presentation-and-vibe-beyond-webview-2026-08-16.md`: Destination pair **JSON→CBOR-LD** and **JS→Vibe** (§0.1); HID not confined to HTML/CSS (§0.2); **WASM family as the bridge** (§0.3); §1.3 quick wins; incremental WebView *and* wire exit. Discussion paper for socialisation.
- `native-computational-economics.md`: Native Rust computational-economics library plan spanning
  stochastic processes, time series, dynamic programming, heterogeneous agents, input-output/network
  economics, market design, game theory, econometrics, welfare/public finance, ontology bridges, and
  Webizen/WASM/MCP exposure.
- `native-visual-intelligence-and-generative-3d.md`: Capability-aware plan for native image
  classification, object detection, video understanding, synthetic datasets, image generation,
  image-to-3D, canonical GLB-to-Q42 asset compilation, and tiered engineering/biological
  digital twins.
- `phone-console.md`: Mobile/phone console, linking, storage, and interaction plan.
- `vibe-nlp-desktop-TRACKER-2026-08-15.md`: Swarm-ready implementation tracker for Vibe 0.1, Poet backend, desktop harness, and document NLP (0.0.31-dev).
- `poet-webizen-desktop-implementation-plan-2026-08-15.md`: Research note that the tracker supersedes for execution.
- `sprint-inference-handover-native-viewport.md`: Inference and native viewport sprint handover.
- `vibescript-webgl-and-advanced-llm-agent-interface-plan-2026-08-18.md`: Architecture and implementation plan expanding VibeScript to support hardware-accelerated WebGL/WebGL2 rendering (zero-copy WASM buffer streaming, Naga WGSL-to-GLSL transpilation, std140 struct alignment) and an advanced scripting interface for in-process LLM agents (homoiconic CBOR-LD ASTs, DOMINO speculative decoding, 3-stage reflection loops, Q42 Semantic Blackboard shared context, multi-agent DAGs with autonomous routers, Paraconsistent Annotated Evidential Logic E-tau in 48-byte NQuins, and W3C Verifiable Credentials).
- `wasm-viewport-migration-plan.md`: Detailed migration plan for the WASM/native 10D viewport.
- `webizen-0.0.28-naturalised-desktop-implementation.md`: Implemented natural/technical
  desktop UX, setup, relations, spatial interfaces, and agent-QA verification contract.
- `wellfair-webizen-desktop/`: Multi-document WellFair/Webizen desktop implementation plan.

## Changelog

- **2026-08-18**: Added VibeScript WebGL rendering and advanced LLM agent scripting interface plan.
- **2026-08-16**: Presentation plan: destination pair is CBOR-LD wire + Vibe language (not JSON/JS host).
- **2026-07-29**: Added the 0.0.28 naturalised desktop implementation and verification handoff.
- **2026-07-03**: Expanded the visual/3D plan after a repository capability audit with the
  existing GLB-to-Q42 path, compiled geometry and analysis sidecars, fidelity/assurance tiers,
  and engineering/biological digital-twin phases.
- **2026-07-03**: Added the native auditory, language, and music intelligence companion plan and
  linked the visual plan into the shared eyes-and-ears architecture.
- **2026-07-07**: Added the native computational economics plan for a comprehensive, zero-heap-aware
  Rust economics substrate rather than isolated finance functions.
- **2026-07-03**: Added the native visual intelligence and generative 3D plan and refreshed the
  manifest to include all current plans.
- **2026-06-30**: Automated full index generation, extracting code definitions.
