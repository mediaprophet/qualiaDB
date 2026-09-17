# Talk and Agents UI

**Status:** stub (U1-D)  
**Implementation plan:** UI waves U1–U3 in `webizen-ui-implementation-subagents-2026.md`

## Talk home

- **Chat** — people-first messaging (`append_chat_message`). `stream_chat_inference` + `chat-token` only when the human asks an instrument (`@slug` or Ask instrument).  
- **People / Reception / Projects / Mail** — social_hub tabs.  
- Model activate via `discover_models` / `set_active_model` — a **tool**, never the other party.

## Backend labels

- **Local** — Qualia in-process GGUF (excellence path).  
- **Ollama** — optional; Settings only; copy must say not the Qualia engine.  
- **Hybrid / Remote** — consent-gated.

## Honesty chips

Talk header shows:

- **held / not yet** when no active GGUF (teachable instrument path; Send still works).  
- **Partial** when a model is active (instrument under principal, not a peer person).

## Agent tools (U3)

Propose MCP tool → principal Permit/Deny → result card.  
Non-MCP external agent SDKs are **deferred**.

## Telemetry

`wellfair_get_llm_telemetry`: live VRAM + lifecycle + backend; `tokens_per_sec` only after a measured turn (`tokens_per_sec_source`).
