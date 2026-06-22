# Path to Rule-Governed Multi-Agent Collaboration via MCP

**Created 2026-06-22.** Timothy directs; the agent builds. This is the *connecting* plan — it
sequences the work needed so multiple agents (human + AI) can cooperate **under the rights /
deontic rules** to get real work done, and it is honest about the blockers. It does **not**
duplicate the deep plans it points to.

Companions: [`DEONTIC_LOGIC_PLAN.md`](DEONTIC_LOGIC_PLAN.md) (the rules engine — built),
[`LLM_Q42_STRATEGIC_PLAN.md`](LLM_Q42_STRATEGIC_PLAN.md) (the LLM capability — broken, plan exists),
[`STELLAR_MISSION.md`](STELLAR_MISSION.md) (the manifold / 10D substrate), `core-ontologies/CML_UPGRADE.md`
(library reprocessing).

---

## 0. The goal in one line
Multiple typed, accountable agents cooperate over MCP; **every action is gated by the legal-logic
engine** (who may do what, to/for whom, under which instrument); the work is durable, attributable,
and human-overridable. "More gets done" *without* losing governance.

---

## 1. The four layers — and where each one actually stands

| Layer | What it is | Status | Blocker |
|------|------------|--------|---------|
| **L1 · Identity & cooperation** | Who is calling, verified + grounded; the deontic gate decides if a request proceeds | **Built.** `mcp_cooperation.rs` gate + `mcp_cooperate` tool + flag-gated dispatch enforcement (`QUALIA_MCP_ENFORCE`, default off). The whole §1–§30 rules engine it consults is done. | — (rollout = flip the flag once callers carry a standpoint) |
| **L2 · Capability** | The reasoning an agent uses to actually do tasks — the LLM | **BROKEN.** Per `LLM_Q42_STRATEGIC_PLAN.md` + Timothy: deployed demos 404, models unhosted, the LLM→`.q42` pipeline doesn't work end-to-end for real users. | **#1 blocker** — agents can't do useful work without it |
| **L3 · Surface** | The pages/apps humans + agents meet in | **BROKEN / UNDEFINED.** Demo pages disabled by bad code/design; the GPU 10D-tensor manifold renderer is not fully defined; there are **no authoring docs** for "how to define / create apps / pages." | **#2 blocker** — Timothy can't update the GH Pages demos until the renderer is defined + documented |
| **L4 · Orchestration** | Many agents coordinating sub-tasks under the rules | **Partial substrate.** `chat_agents.rs` / `chat_relay.rs` / `chat_inference.rs` exist (group chat / sub-agents); not yet bound to the L1 gate or a working L2. | depends on L1 (have) × L2 × L3 |

**The shape of it:** L1 (governance) is done. The thing standing between "we have a rules engine"
and "agents collaborate to get more done" is **L2 (the LLM doesn't work)** and **L3 (no usable,
documented surface)**. L4 is mostly wiring once L1–L3 hold.

---

## 2. The blockers, named honestly (Timothy's list, verified)

1. **The LLM isn't working.** This is the capability agents need. Big job: overhaul the
   **LLM → `.q42`** pipeline (model hosting + the right HF URLs, the opaque-blob `.q42` format,
   the WASM bundle divergence). The honest status + target architecture already live in
   `LLM_Q42_STRATEGIC_PLAN.md` — that is the plan of record; this doc just marks it as the **L2
   gate**: multi-agent collaboration is capability-starved until it lands.
2. **The demos are disabled** by poor design / bad code / bad luck. They are the public face; an
   agent collaboration story is not demonstrable while they 404.
3. **The manifold renderer is undefined and undocumented.** Timothy's explicit gate: the **GPU
   10D-tensor renderer must be fully defined, with full docs on how to define / create apps /
   pages, *before* the GH Pages demos can be updated.** The §20 *logic bridge* is built; the
   *renderer + authoring docs* are the missing substrate (STELLAR tasks #11–13).

---

## 3. What must get done — the dependency-ordered path

**Track A — Governance rollout (L1, cheap, mostly done):**
- [x] Cooperation gate + flag-gated enforcement (done this session).
- [ ] Make callers carry a verified standpoint (the MCP clients / `chat_agents` supply a signed
      caller identity), then flip `QUALIA_MCP_ENFORCE` on per-deployment.
- [ ] Persist the interaction record (who did what, in what standpoint) — the accountability log.

**Track B — Capability (L2, the #1 blocker):** execute `LLM_Q42_STRATEGIC_PLAN.md`.
- [ ] Get one model genuinely serving end-to-end (host the model, fix the HF URL, one WASM bundle).
- [ ] Make `.q42` the real transport for weights (not opaque blobs) per that plan.
- [ ] Expose inference behind the **gated** MCP `llm_infer`/`llm_chat` so agent reasoning is itself
      rule-governed.

**Track C — Surface + authoring (L3, the #2/#3 blockers, gates the GH update):**
- [ ] **Define the GPU 10D-tensor renderer** (inputs, the 10D→render contract, the CPU fallback) —
      STELLAR #11–13.
- [ ] **Write the authoring docs**: "how to define / create apps / pages" on the substrate — the
      thing Timothy needs before touching GH Pages. (Pairs with `docs/manuals/`.)
- [ ] Repair the disabled demo pages (diagnose the bad code/design; one canonical, working demo).

**Track D — Orchestration (L4, once A×B×C hold):**
- [ ] Bind `chat_agents`/`chat_relay` sub-agents to the L1 gate (each agent action authorized).
- [ ] Multi-agent task: agents propose → the deontic gate evaluates → policy mode (allow / audit /
      block / ask-human) → WAL record. This is the goal realised.

---

## 4. Recommended sequence (solo capacity, honest)
The fastest route to "agents get more done, governed":
1. **Track A finish** (small) — standpoint on calls + interaction record. Low effort, high
   leverage; makes every later agent action accountable.
2. **Track B** (the LLM) — the single biggest unblock; everything agentic is capability-starved
   without it. Follow `LLM_Q42_STRATEGIC_PLAN.md`.
3. **Track C** (renderer + authoring docs) — required before the GH demos; can run in parallel
   with B since it's a different skill set.
4. **Track D** — wiring, once A–C give it something real to coordinate.

Each is a real project, not an afternoon. **Decision for Timothy:** which track do we open next —
B (revive the LLM), C (define the renderer + authoring docs to unblock GH), or A (finish the
governance rollout)? This doc is the map; pick the first leg.
