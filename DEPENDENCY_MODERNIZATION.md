# Dependency Modernization — backlog

**Standing rules:** [`CLAUDE.md`](CLAUDE.md) **§13** (modernize to the current dependency's API *and
capabilities*, fix problems along the way — don't work around or step over) + **§14** (spawn sub-agents
for appropriate, parallelizable work). Each item below is an independent, **worktree-isolatable** unit —
the model case for a sub-agent; several can run in parallel.

**This list is not exhaustive.** Timothy maintains the fuller `A…` set; the items here are the ones
surfaced so far (F/G/H were named explicitly; wgpu is the one already breaking the build). Versions below
are the actual pins (verified 2026-06-26).

| Id | Dependency (pinned) | Modernize to | Likely call sites | Lane / caution |
|----|----|----|----|----|
| — | **wgpu 29** in `qualia-core-db` + `qualia-extensions`; **still 0.19** in `webizen-runtime` (0.19.4) + `webizen-render` (0.19) | drop the old ~0.20 surface (`wgpu::Maintain` → `PollType`) + adopt 29 capabilities; **unify the 0.19 crates up to 29** | `services/`, `lora/webgpu_lora.rs`, `gpu_context.rs`, gguf_bridge GPU dispatch; `webizen-runtime`/`render` | **breaking the build now**; gguf_bridge/shader bits = LLM lane → coordinate |
| **F** | **naga 29** (`wgsl-in`, `qualia-core-db`) | shader reflection / validation / capabilities → naga 29 API | shader pipeline, `gpu_context.rs` | pairs with the wgpu 29 work |
| **G** | **arkworks 0.6** (`ark-groth16`/`-relations`/`-snark`/`-ff`/`-serialize`/`-ec`/`-std`) | `CanonicalSerialize`/`Deserialize` + trait bounds → ark 0.6 surface & traits | `zk_proofs.rs`, `deontic_circuit.rs`, zk/fiduciary | zk-culling feature-gated |
| **H** | **reqwest 0.13** (legacy `blocking` feature) | blocking I/O → async `Client` (hyper 1.0 streaming) | `qualia-client-core`; also `qualia-cli`, `webizen-component-harvester`; `qualia-semantic-library` still on **0.12** | ⚠ **`qualia-client-core` = Gemini's live lane — DO NOT spawn here; coordinate, don't duplicate.** The other crates' blocking users + the 0.12 bump are separate units. |
| … | (others) | — | Timothy's fuller `A…` list | enumerate as surfaced |

**Method per item (CLAUDE.md §13/§14):**
- Own worktree (`isolation: "worktree"`); scoped strictly to that dependency's call sites.
- Update call sites to the new API **and adopt its new capabilities** — not just "make it compile".
- Green build + targeted tests pasted; behaviour preserved (or the change is the fix, stated plainly).
- Per-step log + one `NOTICES.md` line.
- **Never reach into another instrument's lane** — flag it (NOTICES + report) instead.
