# Act I — Origin

> *Why a generic graph plus a generic LLM is not enough.*

---

## Thesis

> **A semantic engine that decides things about people must be auditable,
> bounded, and governed — and a generic graph plus a generic LLM is none of
> those three.**

---

## Voice-over script

### Shot 1 — Black screen, single line of white text. [SLOW]

> Most software that decides things about people was not built to be
> audited. It was built to ship. [PAUSE]

### Shot 2 — A generic-looking dashboard with charts and a "model confidence" slider. [ITEM]

> It has a graph. The graph holds facts. [PAUSE] [ITEM]
> It has a model. The model answers questions. [PAUSE] [ITEM]
> It has a permissions system. The permissions system is a flag. [END LIST] [PAUSE]

### Shot 3 — Same dashboard, but the model has just produced an answer with no citation. [SLOW]

> When the model produces an answer, the graph does not know whether the
> answer was grounded. When the graph mutates, the model does not know
> whether the mutation was permitted. When the permissions flag is set, the
> system does not know who set it, when, or under what contract. [PAUSE]

### Shot 4 — Cut to black; the cyan-blue node graph fades in, slowly. [SLOW]

> We wanted something different. [PAUSE]
> We wanted a system where every fact has a fixed size in memory, where
> every reasoning step has a name, and where every action is gated by a
> norm that the system itself can read. [PAUSE]

### Shot 5 — The node graph pulses. A center node lights up. [ITEM]

> We wanted a forty-eight-byte semantic datum. [PAUSE] [ITEM]
> We wanted a forty-two-megabyte working memory. [PAUSE] [ITEM]
> We wanted thirty-plus formal logics compiled into one binary. [PAUSE] [ITEM]
> We wanted a language model that lives inside the same process as the
> graph, with no HTTP server between them. [END LIST] [PAUSE]

### Shot 6 — The node graph rotates, revealing more nodes. [SLOW]

> We wanted the system to refuse. [PAUSE]
> Not to fail. Not to crash. To refuse, with a reason, in a language the
> caller can read. [PAUSE]

### Shot 7 — Title card, in the same cyan-blue. [SLOW]

> This is QualiaDB. [PAUSE]
> This is what we built. [PAUSE]
> This is what we are about to show you. [PAUSE]

---

## On-screen notes

- **Shot 1:** Black. White serif text, centered. No music yet.
- **Shot 2:** A deliberately generic-looking analytics dashboard. The point is that it looks familiar. The "model confidence" slider is the giveaway — no production system has one.
- **Shot 3:** The dashboard's answer panel is empty of citations. The cursor blinks. This is the violation.
- **Shot 4:** The QualiaDB node graph (the project icon) fades in over 3 seconds. A low synth pulse begins.
- **Shot 5:** As each item is read, a corresponding node lights up on the graph. The center node is the engine. The five surrounding nodes are: NQuin, SLG Arena, Modalities, Inference, Governance.
- **Shot 6:** The graph rotates slowly. The five nodes are labeled.
- **Shot 7:** Title card: **Quinta Essentia** in cyan-blue serif. Subtitle: *A film about QualiaDB.* The synth pulse resolves on a single tone.

---

## Source code anchors

- `crates/qualia-core-db/src/lib.rs` — defines `NQuin` (the 48-byte datum).
- `crates/qualia-core-db/src/governance/webizen.rs` — `SlgArena` (the 42-megabyte ring buffer).
- `crates/qualia-core-db/src/modalities/mod.rs` — the modality registry (thirty-plus logics).
- `crates/qualia-core-db/src/inference/inference_agent.rs` — `AgentRuntime` (the in-process LLM seam).
- `crates/qualia-core-db/src/mcp/mcp_cooperation.rs` — `authorize_call` (the refusal mechanism).

---

## Duration

Approximately 90 seconds. This is the slowest act. It is the foundation.
