# Talk and Agents UI

**Status:** stub (U1-D)  
**Implementation plan:** UI waves U1–U3 in `webizen-ui-implementation-subagents-2026.md`

## Talk home

- **Chat** — local agent sessions; `stream_chat_inference` + `chat-token`.  
- **People / Reception / Projects / Mail** — social_hub tabs.  
- Model activate via `discover_models` / `set_active_model`.

## Backend labels

- **Local** — Qualia in-process GGUF (excellence path).  
- **Ollama** — optional; Settings only; copy must say not the Qualia engine.  
- **Hybrid / Remote** — consent-gated.

## Honesty chips

Talk header shows:

- **Needs model** when no active GGUF.  
- **Partial** when model active (runtime dogfood still required until harness proves e2e).

## Agent tools (U3)

Propose MCP tool → principal Permit/Deny → result card.  
Non-MCP external agent SDKs are **deferred**.

## Telemetry

`wellfair_get_llm_telemetry`: live VRAM + lifecycle + backend; `tokens_per_sec` only after a measured turn (`tokens_per_sec_source`).
