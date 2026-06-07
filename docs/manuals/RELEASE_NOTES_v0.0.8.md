# Qualia-DB v0.0.8 Release Notes

## Major Highlights

### 1. Cooperative Group Chat with Sub-Agent Hierarchy

Local LLM and Webizen agents are **sub-agents of human participants**, not independent chat peers. Each participant may run a different model/backend (`local`, `remote`, or `hybrid`). Processed outcomes — summaries and grounded answers, never raw prompts — can be shared with other group members when an explicit **Outcome Sharing Policy** permits it.

- Sub-agent DIDs: `did:qualia:subagent:{principal_hash}:{session_hash}`
- FRB: `getLocalAgentConfig`, `updateAgentOutcomeSharing`, `getDefaultOutcomeSharing`
- Cooperative multi-LLM briefing injects peer-disclosed outcomes into inference context when `allow_peer_llm_context` is enabled

### 2. Daemon Chat Relay (`/chat/publish` + `/chat/pull`)

Group chats sync through the Qualia semantic graph daemon on loopback — no central cloud broker required.

- `POST /chat/publish` — append signed relay envelopes to `{storage}/ChatRelay/{session_id}/inbox.jsonl`
- `GET /chat/pull?session_id=…&since_lamport=…` — incremental inbox pull
- Ed25519 signature verification on publish when `signature_hex` is present
- Flutter: `syncChatRelay()` merges remote messages into the local session WAL

### 3. Qualia-Native WebTorrent Seeder

Ontology artifacts (`.c.q42`) are shared via HTTP web seeds served by the **Qualia daemon**, not UI-only stubs.

| Endpoint | Purpose |
|----------|---------|
| `GET /torrent/webseed/{info_hash}` | Serve `.c.q42` with Range support (BEP-19) |
| `POST /torrent/seed` | Register a seed |
| `POST /torrent/unseed` | Stop seeding |
| `GET /torrent/telemetry` | Live upload stats (`seeder: "qualia-daemon"`) |
| `POST /torrent/policy` | Global bandwidth policy |
| `POST /torrent/sync` | Reload seeds from workbench index |

Magnet URIs from the Ontology Workbench include a `ws=` parameter pointing at the daemon web-seed URL.

### 4. Ontology Workbench

Import remote ontologies, compress to `.c.q42`, compute SHA-1 info hashes, and share via magnet URIs with audience-scoped policies (contacts, session DIDs, categories).

- `workbenchImportOntologyUri`, `listWorkbenchOntologies`, `setWorkbenchSeed`
- `setWorkbenchTorrentPolicy`, `getTorrentBandwidthPolicy`, `setTorrentBandwidthPolicy`
- `listOntologySharesForSession` — share cards filtered by session DID

### 5. Rich Chat UX (Flutter Desktop)

- Ontology branches, fragment replies, reactions, file attachments with sharing policy
- Chat graph panel (`getChatGraph`, `createChatFragment`, `appendChatMessageReply`)
- Group sessions with participant management and session DIDs
- Bundled qapps (e.g. Anatomy) launchable from chat with host context handoff

### 6. Flutter Desktop as Primary Shipped Target

`crates/qualia-flutter/` is the primary desktop shell (Windows, macOS, Linux) via flutter_rust_bridge. Legacy Tauri `qualia-desktop` is not in release CI.

---

## Component Version Bumps

- `qualia-core-db`: **0.0.8**
- `qualia-client-core`: **0.0.8**
- `qualia-flutter`: **0.0.8**
- `qualia-cli`: **0.0.8**

*For architecture detail see [ARCHITECTURE.md](ARCHITECTURE.md) and [developer-guide.md](developer-guide.md).*
