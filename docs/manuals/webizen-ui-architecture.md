# Webizen UI Architecture

**Status:** stub + accurate map (U1-D); IA frame → socio-neuromorphic  
**Plan:** [`docs/plans/comprehensive-ui-gui-webizen-plan-2026.md`](../plans/comprehensive-ui-gui-webizen-plan-2026.md)  
**Human IA + capability inventory:** [`docs/plans/socio-neuromorphic-ict-interface-plan.md`](../plans/socio-neuromorphic-ict-interface-plan.md)  
**Sub-agents:** [`docs/plans/webizen-ui-implementation-subagents-2026.md`](../plans/webizen-ui-implementation-subagents-2026.md)

## Surfaces

```text
webizen-desktop (Tauri 2)
  shell, tray, browser chrome, 300+ commands
        │
webizen-studio (Dioxus)
  Talk, Library, 10D browser, QApps, panes
        │
qualia-client-core → qualia-core-db + specialized_libs
  graph, inference, modalities, vision, audio
```

## Non-goals

- No second GPU adapter for chrome.
- Ollama is optional Settings harness — not the Qualia engine.
- Audio product cut is planned separately (algorithms catalogue); UI waves first.
- Product shell stays Tauri 2 + Dioxus + engine wgpu.

## Inference honesty

| Backend | Role |
|---------|------|
| Local GGUF | Primary excellence path |
| Ollama | Opt-in HTTP bridge |
| Hybrid / Remote | Principal-gated |

Telemetry: `wellfair_get_llm_telemetry` returns live VRAM/lifecycle/backend and **last measured** tok/s only (never static mocks).

## Related manuals

- [Event catalogue](./webizen-ui-event-catalogue.md)
- [Talk and agents](./talk-and-agents-ui.md)
