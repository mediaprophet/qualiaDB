# EXP-B0 — Chat-graph seam decision

**Date:** 2026-09-06  
**Status:** Decided (default under freeze)

## Decision

Keep **chat-graph desktop / client-core only**. Do **not** add a `ChatGraph.*`
Vibe / Host family under `vibe-host-0.1` freeze.

## Rationale

1. Stack already exists: `qualia-client-core::chat_graph` + desktop
   `get_chat_graph` / FRB — not an unbound library.
2. Freeze forbids inventing Host widen / new Family.method without explicit
   exception; chat-graph is session/storage heavy and sensitivity-gated.
3. Poet WASM must not fake a graph via unrelated binds (e.g. `GraphDatabase.stats`).

## Follow-on (`EXP-B1a`)

- Instrument ribbon `social:graph` → honest **unavailable** status pointing at
  Webizen Desktop / `get_chat_graph`.
- No `ChatGraph.*` ids in `ALL_BOUND`.

## Re-open path

Owner may later approve a **minimal** bind set (`ChatGraph.load` /
`create_fragment` / `link_reply`) as an explicit freeze exception — that becomes
`EXP-B1b`, not this packet.
