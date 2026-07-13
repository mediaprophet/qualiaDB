# Qualia-DB enablement backlog — agency agents & Qapps

**Status:** Proposed backlog (companion to the agency-agents refactor)
**Date:** 2026-07-03
**Sibling repo:** `C:\Projects\webcivics\webize-agency-agents` (the reframed agency roster)
**Sibling doc:** `webize-agency-agents/docs/QUALIA-WEBIZEN.md` (how an agent is written "the Qualia-DB way")

## 0. Why this document exists

We are refactoring the **Agency Agents** roster (a large catalogue of AI agent personalities) so it is
native to Qualia-DB / Webizen instead of a generic Node/React/cloud world. Doing that honestly forces a
second question: **what does Qualia-DB still need to build so these agents — and the Qapps they help make —
are first-class citizens of the node?**

This is that list. It is deliberately scoped to the **agent + Qapp enablement** seam. It does **not**
restate the large cooperative/desktop initiative already captured in
[`cooperative-qapps-desktop-implementation-plan.md`](cooperative-qapps-desktop-implementation-plan.md)
(WP0–WP11, Releases A–F) and tracked in
[`../../COOPERATIVE_QAPPS_PROGRESS_LOG.md`](../../COOPERATIVE_QAPPS_PROGRESS_LOG.md). Where an item is
already covered there, this doc **cross-references** it rather than re-specifying it.

The one genuinely-new area this refactor surfaces is **§B1: the agent-as-installable-artifact layer** — the
runtime, install, and governance surface for *externally authored agent definitions* (which the agency
roster is a supplier of). That layer does not exist yet.

---

## A. What already exists (so agents reference reality, not fiction)

Verified against the tree on branch `0.0.24`. Reframed agents should name these, not invent capabilities.

| Capability | Where | Notes |
|---|---|---|
| Principal-bound sub-agents | `qualia-client-core/src/chat_agents.rs` (`ParticipantAgentConfig`) | `principal_did → sub_agent_did`, backend `Local/Remote/Hybrid`, `OutcomeSharingPolicy`. **No persona / system-prompt / tool-allowlist field yet** — see B1. |
| Gated inference | `qualia-core-db/src/orchestrator.rs::orchestrate_inference` | `validate_intent → infer → validate_output`; ungrounded output rejected; denials WAL'd + signed. |
| Native inference stack | `qualia-core-db/src/inference/` (`gguf_sharder`, `gguf_bridge`, `llm_agent`) + `shaders/fused_tensor_contraction.wgsl` | memmap2 + wgpu, Phase 8 bifurcated compute. No Ollama. |
| MCP host tool surface (~58 tools) | `qualia-core-db/src/mcp/mcp_server.rs` | graph (`query_sparql`…), inference (`llm_infer`…), compute (`matrix_operation`, `financial_model`, `medical_score`…), governance (`shacl_*`), qapp introspection (`list_qapps`…). **No per-agent tool allowlist enforcement** — see B1.2. |
| Qapp platform | `qualia-client-core/src/{qapp_registry,qapp_install,qapp_manifest,qapps_protocol,api}.rs` | `qapp.json` schema, atomic install, hashes, ABI + Ed25519 checks, loopback serving. |
| Companion PWA packaging | `qualia-cooperative-core/src/qapp_package/{manifest,pwa}.rs` | `QappManifest`, least-privilege `Capability`, `generate_pwa`. |
| Studio Package & Publish | `webizen-studio` "Qapps" area + `qualia-client-core` `qapp_publish.rs` | Human authoring path; folder-picker → installable PWA scaffold. |
| Cooperative domain | `qualia-cooperative-core/src/{project,work_item,roadmap,contribution,budget,agreement,agency_domain,trigger,provenance,agency_delegation}.rs` | Event-sourced, replay-safe projections, supported-agency ABAC. |
| Desktop hosts | `webizen-desktop` (Tauri) + `webizen-studio` (Dioxus panels) | Qapp launch in dedicated WebView; WellFair/Projects/Agency panels. |
| Coordination ISA (partial) | `docs/manuals/standards/MULTI_AGENT_PROTOCOL.md` + `governance/coordination.rs` | Deterministic core exists; root-delegation / suspended-queue / VC persistence / routing are host work (plan §5.4). |

---

## B. Gaps to build

Legend — effort: **S** (≤1 session) · **M** (multi-session) · **L** (multi-release, cross-reference).

### B1. Agent-as-installable-artifact layer — **NEW, not covered elsewhere**

This is the missing half. Qualia-DB can bind a sub-agent to a principal and gate its inference, but it has
no notion of an **installed agent definition** — a persona/role with a system prompt, a scoped tool set, a
sensitivity ceiling, and governance flags — authored outside the node and installed like a Qapp. The
agency roster is exactly such a supplier. Without this layer, the reframed agents are documentation, not
runnable artifacts.

| # | Item | What | Where | Effort |
|---|---|---|---|---|
| **B1.1** | **External agent manifest + loader** | Define a `webizen-agent/1` manifest (name, slug, `default_backend`, `max_sensitivity_clearance`, `allowed_mcp_tools[]`, `required_ontologies[]`, `outcome_sharing_default`, `governance{requires_intent_validation, requires_output_provenance}`, `system_prompt`). Add a host loader that reads it from the node's agents dir and binds it to the active principal via `compile_sub_agent_did`. | extend `chat_agents.rs`; new `agent_registry.rs` | **M** |
| **B1.2** | **Per-agent MCP tool allowlist enforcement** | The 58 MCP tools dispatch without per-sub-agent scoping. Enforce the manifest's `allowed_mcp_tools` at the `dispatch_tool_call` boundary, keyed by the calling sub-agent's DID. A tool not in the allowlist fails closed with a policy receipt. | `mcp/mcp_server.rs` dispatch; bind to loaded agent | **M** |
| **B1.3** | **Agent persona is a governed artifact** | An installed agent definition must be signed (Ed25519), content-hashed, sensitivity-bounded, and its `system_prompt` itself subject to `validate_intent` at install and at bind. Reuse the `qapp_install.rs` pattern (atomic install, hash, ABI check, revocation). | new `agent_install.rs` (mirror `qapp_install.rs`) | **M** |
| **B1.4** | **Agent Vault / registry surface + MCP introspection** | Analogue of the Qapp Vault: `list_agents`, `get_agent_manifest`, `inspect_agent_readiness` MCP tools + a Studio/desktop "Agents" area to install, inspect capabilities/sensitivity, enable/revoke. | `mcp_server.rs`; `webizen-studio` new panel | **M** |
| **B1.5** | **The `webizen` convert/install target** (cross-repo) | In `webize-agency-agents`: a `tools.json` `"webizen"` entry (`format: webizen-agent`, `installKind: per-agent`), a `convert_webizen` renderer emitting the B1.1 manifest, and `install_webizen` dropping it into the node's agents dir. This is where the reframed prose becomes an installable artifact. Depends on B1.1 for the target schema. | `webize-agency-agents/scripts/{convert,install}.sh`, `tools.json` | **S–M** |
| **B1.6** | **Agent-driven Qapp authoring, same gates** | The pilot "Qapp Engineer" agent implies an agent can drive Package & Publish. Expose the lint/offline/CSP/sign pipeline as governed host intents an agent may call (with human confirmation on sign), not just a Studio-only UI path. | `qapp_lint.rs`/`qapp_builder.rs` (plan §12.2) + intent surface | **M** |
| **B1.7** | **Outcome-sharing + provenance on agent output** | `OutcomeSharingPolicy` exists on `ParticipantAgentConfig`; ensure an installed agent's outputs honour it (visibility, `share_provenance`, `share_model_attribution`, `allow_peer_llm_context`) and always attach the ≥1 provenance NQuin the VM requires. | `chat_agents.rs` + orchestrator output path | **S** |

### B2. Qapp platform hardening — **mostly covered; cross-referenced**

These are prerequisites the agent+Qapp story depends on but which the cooperative plan already owns. Listed
so the dependency is explicit; **do not duplicate the spec** — implement against the plan.

| # | Item | Owner doc | Status |
|---|---|---|---|
| **B2.1** | Qapp session **token v2** (expiry, audience, nonce, capability + sensitivity, revocation) | cooperative plan **§7** | Release gate; WP1. |
| **B2.2** | Per-Qapp **origin isolation** + CSP / nosniff / referrer / frame policy on the loopback server | cooperative plan **§7** | WP1. |
| **B2.3** | Package **lint / offline / CSP audit** before sign | cooperative plan **§12.3** | Partial (P0 `validate()`); expand. |
| **B2.4** | Studio **Package & Publish** full workflow (capability picker, report, archive) | cooperative plan **§12** | Foundations landed (WP2); complete. |
| **B2.5** | Dedicated `qualia-client-core/src/cooperative/` service (currently persists via WellFair journal) | cooperative plan **§8.1**; progress log 2026-07-03 | Deferred; noted. |

### B3. Substrate honesty items — surfaced by the reframed agents

Concrete "the agent needs X but the substrate has a rough edge" items. Small, real, and worth fixing so the
agents don't describe a smoother world than exists.

| # | Item | Why an agent hits it | Where | Effort |
|---|---|---|---|---|
| **B3.1** | **N3 parser multi-triple / SWAP-math limitation** | The Governance Steward and NQuin Curator agents must author rules the parser truncates today; they currently have to fall back to single-rule forms or native derivations. | cooperative plan §9; `mini_parser.rs` / N3 path | **M** |
| **B3.2** | **Reusable Qapp UI kit (CSP-clean, offline, accessible, no CDN)** | The Qapp Engineer agent has no shared component/asset-bundler kit to point at — every Qapp reinvents CSP-clean UI. | new, under `webizen-render`/studio | **M** |
| **B3.3** | **GPU decode path rough edges** (DX12 decode hang; ternary path dead → Q4_K_M ships) | The Local Inference Engineer agent must warn around these; they should be tracked, not folklore. | `inference/` + `docs/HANDOVER-0.0.23-release.md` | **M–L** |
| **B3.4** | **Agent honesty guard coverage** | The measurement-honesty rule (no extrapolated tok/s, no fabricated provenance) should be test-enforced for agent output, extending `qualia-core-db/tests/agent_honesty_guard.rs`. | that test + output validator | **S** |
| **B3.5** | **`describe_qapp_surface_schema` → agent-consumable capability catalog** | Agents need a machine-readable list of host capabilities/tools + sensitivity consequences to reason about least-privilege (plan §12.1 step 4). | `mcp_server.rs` schema tool | **S** |

---

## C. Suggested sequence

1. **B1.1 + B1.5** — define the agent manifest and stand up the `webizen` convert target end-to-end (even
   with a thin loader). This makes one reframed agent *actually install and bind* on a node — the proof the
   whole refactor is real, not just prose.
2. **B1.2 + B1.7** — enforce tool allowlist + outcome/provenance, so an installed agent is *governed*, not
   just present.
3. **B1.3 + B1.4** — sign/verify agents and give them a Vault surface (parity with Qapps).
4. **B2.1/B2.2** land in parallel on the cooperative track (they gate restricted-data Qapps regardless).
5. **B3** items as they block a specific agent.

Dependency: B1 is unblocked *today* for B1.1/B1.5/B1.7 (they extend existing `chat_agents.rs` +
`mcp_server.rs`); B1.3/B1.4 reuse the proven `qapp_install.rs` shape.

---

## D. ⚑ Where I need the human (Timothy)

1. **Is B1 in scope, or is the roster documentation-only for now?** If the agency roster should *run* on the
   node (not just describe how to work), B1 (the installable-agent layer) is the load-bearing new build. If
   it's reference-only, B1 collapses to B1.5 (a convert target that emits manifests other tools consume).
2. **Priority vs. the cooperative track.** B1 and the cooperative WPs share the same Studio/host surfaces
   and the same author's attention. Which leads? (My recommendation: a thin B1.1+B1.5 slice first — it's
   small and it de-risks the whole agency-agents direction — then rejoin the cooperative sequence.)
3. **Sensitive vocabulary.** The governance/agency agents touch rights, consent, and supported-agency
   language you may reserve the right to coin. Tell me which terms are yours to define so I don't put words
   in your mouth.
4. **Home & upstream** (still open from the opening question): standalone-mergeable vs. fold-into-qualia vs.
   hard-divergence for the `webize-agency-agents` repo. This decides whether B1.5 stays additive.
