# Act IX — Chat & Relay

> *The engine talks to people.*

---

## Thesis

> **The engine has a chat surface. It has agents. It has a relay. It has
> guardianship. It has front-door DIDs. It has connect invites. It has a
> social graph. The chat is signed. The relay is signed. The agents are
> scoped.**

---

## Voice-over script

### Shot 1 — A chat window. A user types a message. [SLOW]

> This is the chat surface. [PAUSE]
> It is `qualia-client-core::chat_session`. [PAUSE]
> The user types a message. [PAUSE]

### Shot 2 — The message is wrapped in a `RelayEnvelope`. The envelope is signed. [SLOW]

> The message is wrapped in a `RelayEnvelope`. [PAUSE]
> The envelope is signed with Ed25519. [PAUSE]
> The signature is verifiable. [PAUSE]

### Shot 3 — The envelope is published to the relay. The relay is a local daemon on port 4242. [SLOW]

> The envelope is published to the relay. [PAUSE]
> The relay is a local daemon on port four-two-four-two. [PAUSE]
> It is the graph engine, not an LLM server. [PAUSE]

### Shot 4 — The chat inference loop runs. `run_chat_inference_with_options`. [SLOW]

> The chat inference loop runs. [PAUSE]
> It is `run_chat_inference_with_options`. [PAUSE]
> It validates the intent. [PAUSE]
> It routes through the orchestrator. [PAUSE]
> It runs the LLM. [PAUSE]
> It validates the output. [PAUSE]

### Shot 5 — The output is wrapped in a `RelayEnvelope` and published back. [SLOW]

> The output is wrapped in a `RelayEnvelope`. [PAUSE]
> It is signed. [PAUSE]
> It is published back to the relay. [PAUSE]

### Shot 6 — The user receives the response. The provenance citation is visible. [SLOW]

> The user receives the response. [PAUSE]
> The provenance citation is visible. [PAUSE]
> The citation is a Quin. [PAUSE]

### Shot 7 — An agent is configured. The agent has a backend, a DID, a scope. [ITEM]

> Agents can be configured. [PAUSE] [ITEM]
> The backend — local, remote, or hybrid. [PAUSE] [ITEM]
> The DID — the agent's identity. [PAUSE] [ITEM]
> The scope — what the agent may do. [END LIST] [PAUSE]

### Shot 8 — A connect invite is generated. The user shares it. [SLOW]

> A connect invite is generated. [PAUSE]
> It is a short code, encoded with the user's front-door DID. [PAUSE]
> The user shares it. [PAUSE]
> The recipient accepts it. [PAUSE]
> A contact is added. [PAUSE]

### Shot 9 — A guardianship contract is shown. Three guardians. Two must consent. [SLOW]

> A guardianship contract requires multi-party ratification. [PAUSE]
> Three guardians are listed. [PAUSE]
> Two must consent. [PAUSE]
> The transaction is suspended. [PAUSE]
> When two guardians apply consent tokens, the transaction is executed. [PAUSE]

### Shot 10 — The chat graph is shown. Fragments and edges. [SLOW]

> The chat graph is a record of what was said. [PAUSE]
> Fragments are the messages. [PAUSE]
> Edges are the replies. [PAUSE]
> The graph is queryable. [PAUSE]

### Shot 11 — The chat ontology is shown. Branch types, reactions, classifications. [SLOW]

> The chat ontology classifies branches. [PAUSE]
> Branch types are stored. [PAUSE]
> Reactions are stored. [PAUSE]
> The classification is deterministic. [PAUSE]

### Shot 12 — The chat retrieval is shown. The graph is queried for context. [SLOW]

> The chat retrieval queries the graph for context. [PAUSE]
> The context is bundled with the prompt. [PAUSE]
> The LLM receives the context. [PAUSE]
> The LLM produces a grounded answer. [PAUSE]

### Shot 13 — Title card: **The chat is signed. The relay is signed. The agents are scoped.** [SLOW]

> The chat is signed. [PAUSE]
> The relay is signed. [PAUSE]
> The agents are scoped. [PAUSE]

---

## On-screen notes

- **Shot 1:** A chat window. The user's message is in a bubble.
- **Shot 2:** The bubble is wrapped in an envelope. The envelope is signed. The signature is shown as a hex string.
- **Shot 3:** The envelope is published to the relay. The relay is shown as a daemon process.
- **Shot 4:** The inference loop runs. The phases are shown: validate intent, route, run LLM, validate output.
- **Shot 5:** The output envelope is published back.
- **Shot 6:** The user receives the response. The provenance citation is highlighted.
- **Shot 7:** An agent's configuration is shown as a JSON object.
- **Shot 8:** A connect invite is shown as a short code. The user shares it.
- **Shot 9:** A guardianship contract. Three guardians. Two consent tokens have been applied. The transaction is executed.
- **Shot 10:** The chat graph. Fragments are nodes. Edges are arrows.
- **Shot 11:** The chat ontology. Branch types are listed. Reactions are shown.
- **Shot 12:** The chat retrieval. The graph is queried. The context is bundled.
- **Shot 13:** Title card.

---

## Source code anchors

- `crates/qualia-client-core/src/chat_session.rs` — `ChatSession`, `ChatEnvironment`, `OntologyScopeSummary`.
- `crates/qualia-client-core/src/chat_relay.rs` — `RelayEnvelope`, `sign_envelope`, `publish_session_message`, `publish_envelope`, `pull_from_relay`.
- `crates/qualia-client-core/src/chat_inference.rs` — `run_chat_inference_with_options`, `run_chat_inference_full`, `run_orchestrated_inference`, `should_retry_symbolic_block`, `build_corrective_retry_prompt`.
- `crates/qualia-client-core/src/chat_agents.rs` — `AgentBackendKind`, `OutcomeSharingPolicy`, `ParticipantAgentConfig`, `compile_sub_agent_did`.
- `crates/qualia-client-core/src/chat_graph.rs` — `ChatFragment`, `ChatGraphEdge`, `ChatGraphSnapshot`, `append_jsonl`, `read_fragments`, `read_edges`, `load_graph`.
- `crates/qualia-client-core/src/chat_ontology.rs` — `BranchClassification`, `ChatReaction`, `classify_branch`, `add_reaction`.
- `crates/qualia-client-core/src/chat_retrieval.rs` — `retrieve_graph_context`, `format_retrieval_block`, `query_daemon_for_prompt`.
- `crates/qualia-client-core/src/chat_files.rs` — `ChatFileSharing`, `ParsedDocument`, `infer_sensitivity_from_sharing`.
- `crates/qualia-client-core/src/guardianship.rs` — `apply_guardian_token`, `deny_guardian_affirmation`, `is_agreement_ratified`, `pending_affirmation_count`.
- `crates/qualia-client-core/src/social_connect.rs` — `ConnectInvitePayload`, `generate_connect_invite`, `accept_connect_invite`, `list_chat_contacts`.
- `crates/qualia-client-core/src/dns_resolver.rs` — `resolve_qdp_did`, `verify_front_door_did_via_dns`, `encode_did_for_ns`.
- `crates/qualia-client-core/src/context_binding.rs` — `compile_chat_environment`, `InferenceContextPacket`.
- `crates/qualia-client-core/src/ontology_router.rs` — `route_prompt_to_ontologies`, `extend_namespaces`.
- `crates/qualia-client-core/src/user_profile.rs` — `UserProfile`, `resolve_public_did`, `public_profile_card`.
- `CLAUDE.md §5` — the daemon on port 4242 is the graph engine, not an LLM server.

---

## Duration

Approximately 120 seconds. This is the act where the engine talks to people.
