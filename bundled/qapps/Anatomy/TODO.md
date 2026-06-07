# TODO / Roadmap - Qualia Anatomy

This roadmap now tracks Anatomy as:

- a standalone browser app,
- a browser/WASM-enhanced app,
- and a Qualia Flutter App Vault component.

## Priority 1 - Lock the integration contract

- [ ] Decide the first supported launch path:
  - standalone static
  - standalone WASM
  - Flutter App Vault
- [ ] Keep [app.json](C:\Projects\qualiaDB\app-development\Anatomy\app.json) aligned with the current launcher
- [ ] Finalize the first representation payload contract for chat -> anatomy handoff
- [ ] Define the minimum query/shape set Anatomy should be allowed to access

## Priority 2 - Make the app useful in every mode

### A. Static / no-local-install mode

- [ ] Keep the viewer fully usable without daemon or WASM
- [ ] Add demo overlays driven from local knowledge assets
- [ ] Add a "local Qualia not detected" fallback UX

### B. WASM mode

- [ ] Replace the simulated `Qualia.init()` path in [qualia.js](C:\Projects\qualiaDB\app-development\Anatomy\qualia.js)
- [ ] Bind only to exports that are actually present in `wasm_bridge.rs`
- [ ] Add a browser-safe capability check for WASM availability
- [ ] Add local parsing/validation for anatomy-related knowledge files

### C. Flutter App Vault mode

- [ ] Verify install/launch using the current App Vault flow
- [ ] Add launch-state UI for "running inside Qualia Flutter"
- [ ] Define how the app receives local context from Flutter
- [ ] Define how the app returns user selections back to Flutter chat/workflows

## Priority 3 - Chat-driven anatomy

- [ ] Add a representation mode that can accept a structured payload
- [ ] Support graph summary cards beside the 3D view
- [ ] Support condition -> organ/system overlays
- [ ] Support explanation traces such as "why this organ is highlighted"
- [ ] Support "open in Anatomy" from chat results

## Priority 4 - Daemon-aware online mode

- [ ] Add a health probe flow for detecting a local daemon
- [ ] Handle unknown daemon port gracefully
- [ ] Handle daemon-auth-required states explicitly
- [ ] Add a "continue in local desktop app" UX when browser auth cannot proceed

## Priority 5 - API and platform gaps to resolve after the current build settles

- [ ] Qualia launcher should pass an app/session token or equivalent context to the web app
- [ ] Desktop should expose the active daemon port to installed apps
- [ ] App Vault should have a real manifest schema beyond `name/version/required_shapes/dev_port`
- [ ] Chat should be able to launch Anatomy with structured parameters
- [ ] Anatomy should be able to call back into chat or directory workflows
- [ ] CORS/origin policy should support installed local apps reliably

## Priority 6 - Anatomy-specific knowledge and UX

- [ ] Expand anatomical mappings beyond the current starter set
- [ ] Add organ groups, systems, and pathways
- [ ] Add consent/provenance displays for personal overlays
- [ ] Add systemic burden and multi-condition overlays
- [ ] Add medication and diet modifiers once the underlying graph path is ready

## Notes

- The current repo already has useful building blocks, but the integrated path is
  still incomplete.
- The detailed assessment and the list of missing capabilities are in
  [API_CAPABILITIES_ASSESSMENT.md](C:\Projects\qualiaDB\app-development\Anatomy\API_CAPABILITIES_ASSESSMENT.md).
