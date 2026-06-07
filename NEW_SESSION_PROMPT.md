# New Session Briefing — QualiaDB v0.0.8

> Copy everything below this line into the first message of a new Claude Code session
> opened against `C:\Projects\qualiaDB`.

---

I'm continuing development on QualiaDB (`C:\Projects\qualiaDB`), a Rust/Flutter semantic
graph engine with in-process LLM inference. Read `HANDOVER.md` (top section) and
`CLAUDE.md` before doing anything — CLAUDE.md has hard rules about the inference stack
that must not be violated.

**Branch:** `0.0.8-dev`

## What was done (do not redo)

### GPU inference (Phase 8)
- `infer_local_model()` runs a real autoregressive decode loop through DirectML / Accelerate / wgpu.
- `GgufTokenizer` parses GGUF KV metadata (vocabulary, BOS/EOS).
- Phase 8 SPSC ring buffers (LogitStream + ControlStream) remain mandatory.
- **Known limitation**: pseudo-embeddings (sin-based from token ID) — real `token_embd.weight` lookup needs GGUF tensor-info parser.

### Group chat (v0.0.8)
- Sub-agent hierarchy: `chat_agents.rs` — agents are sub-agents of human principals.
- Outcome sharing policies gate relay of processed results only.
- Daemon relay: `POST /chat/publish`, `GET /chat/pull`.
- Flutter: graph panel, fragments, reactions, file attachments, `syncChatRelay()`.

### WebTorrent seeder (v0.0.8)
- Daemon serves `.c.q42` via `GET /torrent/webseed/{hash}` (BEP-19 HTTP web seed).
- Seeding runs in `qualia-core-db`, not UI stubs. Magnets include `ws=` parameter.
- Ontology Workbench: URI import → `.c.q42` → magnet → audience-scoped sharing.

## Suggested next tasks

1. **GGUF tensor-info parser** — read `token_embd.weight` offsets for real embeddings (see `HANDOVER.md`).
2. **Full UDP DHT / WebRTC wire protocol** — current seeder is HTTP web-seed only.
3. **Phase 7 gaps** — PBKDF2 for Sanctuary PINs, unified `.q42` write path, WASM OPFS bindings.

## Key constraints (from CLAUDE.md — mandatory)

- No `Vec`/`String`/`Box` in hot paths.
- 48-byte `QualiaQuin` for all semantic data.
- The LLM backend is `gguf_bridge.rs` + `wgpu` — not Ollama.
- `orchestrate_inference()` gates every LLM call — do not bypass.
- Daemon on 4242 is the graph engine + relay + web seeds — not an LLM HTTP server.

## Files to read first

1. `CLAUDE.md` — hard rules
2. `docs/manuals/RELEASE_NOTES_v0.0.8.md` — what shipped
3. `crates/qualia-client-core/src/chat_agents.rs` — sub-agent model
4. `crates/qualia-core-db/src/webtorrent_seeder.rs` — daemon seeder
5. `crates/qualia-core-db/src/llm_agent.rs` — inference loop

## Public documentation rule

Do **not** add QPU oracle / chat-command details to public docs (`docs/`, `CHANGELOG.md`, release notes, API explorer). QPU remains internal/engine-only.
