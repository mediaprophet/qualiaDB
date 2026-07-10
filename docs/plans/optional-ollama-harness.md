# Optional Ollama harness (bridge while native Qualia inference lands)

**Status:** harness + settings UI wired (2026-07-11)  
**Default:** still **Local GGUF** (in-process Qualia). Ollama is **opt-in only**.

## Why

Native Qualia chat inference is not yet reliable enough for daily work. An optional
Ollama HTTP harness lets the principal keep using Webizen chat, graph retrieval,
ontology routing, and (next) PDF ETL / CML / logic gates against a working local
model server — without replacing the Qualia engine architecture.

## What was built

| Layer | Role |
|-------|------|
| `qualia-client-core::ollama_harness` | HTTP client: `/api/tags`, `/api/generate`, `/api/chat`, `/api/embeddings` |
| `inference_backend.json` + `InferenceBackendSettings` | Persisted Local / Remote / Hybrid / **Ollama** + URL/model/timeouts |
| `chat_inference` | When backend = Ollama: Qualia retrieval/routing → Ollama generate → citations from graph |
| `context_binding` | Packet build without requiring GGUF activation on Ollama path |
| Desktop Tauri | `get/save_inference_backend_settings`, `probe_ollama_status`, `list_ollama_models`, `ollama_generate` |
| Settings UI | Engine backend select + **Ollama harness** panel (URL, models, probe, save) |

## UI requirements (done for this phase)

1. **Inference backend** select: Local · Ollama · Hybrid · Remote  
2. **Ollama panel**: base URL, generation model, embed model, optional API key, timeout, `num_ctx`  
3. **Test connection** → probe tags + latency + model list  
4. **Save Ollama settings** → `inference_backend.json` + sync `AgentConfig.inference_backend`  
5. Clear copy: Ollama is optional; Qualia remains primary  

## Principal follow-ups (you said details after harness/UI)

- PDF ETL pipeline using Ollama (or Local) for extraction + Qualia graph commit  
- Logic / CML / deontic gates on Ollama outputs (same Webizen path as Local)  
- Streaming tokens from Ollama (`stream: true`) into the chat UI  
- Hybrid: Local first → Ollama fallback only when lifecycle ≠ Active  

## How to use now

1. Install/run Ollama; `ollama pull llama3.2` (or preferred tag).  
2. Desktop → **Settings** → Inference Backend = **Ollama**.  
3. Set base URL if needed; **Test connection**; pick model; **Save Ollama settings**.  
4. Chat without GGUF activation — retrieval still uses Qualia graph/ontologies.

## Non-goals

- Ollama is **not** the Qualia engine and must not be documented as such.  
- No silent default to Ollama; no telemetry to third parties via this path.  
