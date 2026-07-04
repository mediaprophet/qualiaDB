# NotebookLM Cinematic Script — QualiaDB

> **Project:** *Quinta Essentia* — a cinematic documentary on the QualiaDB engine.
> **Audience:** technically literate viewers who want to see what a real, working,
> native, in-process semantic-and-inference engine actually looks like when you
> open the box.
> **Format:** voice-over narration driven by NotebookLM, paired with on-screen
> visuals drawn from the codebase, the runtime telemetry, and the volumetric
> renderer that the engine itself drives.
> **Tone:** measured, precise, occasionally awe-struck. No marketing fluff. No
> "revolutionary," no "next-generation." Show the work.

---

## 1. What this is

The `crates/` folder of this repository is not a typical Rust workspace. It is
**a single, coherent, multi-target semantic engine** that ships as:

- a desktop shell (`webizen-desktop`, Tauri 2),
- a CLI (`qualia-cli`),
- a browser-local ontology MCP (`webizen-lite-wasm`, WASM),
- a mobile harness (`qualia-mobile-harness`, Dioxus),
- a Solid-pod bridge (`qualia-solid-bridge`),
- a wellfare/health library (`wellfare-core`),
- a render SDK (`webizen-render`),
- a Dioxus studio (`webizen-studio`),
- an LLM lifecycle / chat client (`qualia-client-core`),
- and the **engine itself** (`qualia-core-db`).

Everything else is a surface. The engine is the substrate.

## 2. The five-second thesis

> "QualiaDB is a single Rust binary that holds a 48-byte-per-fact semantic
> memory in a 42-megabyte arena, reasons over it with thirty-plus distinct
> formal logics, runs a real local LLM through wgpu, paints a 10-dimensional
> volumetric scene, and refuses to do any of it unless a deontic contract says
> it may."

That sentence is the spine of the film. Every act below proves one clause of it.

## 3. The narrative arc

We open on the **problem** (Act I — why a generic graph + a generic LLM is not
enough for human-rights-grade software). Then we go **under the hood** (Act II —
the NQuin ABI and the SLG Arena). Then we **unfold the modalities** (Act III —
thirty logics, one wire format). Then we **show the inference engine running**
(Act IV — GGUF through wgpu, with the bifurcated Sentinel). Then we **show it
being governed** (Act V — deontic, epistemic, ratification, paraconsistent
isolation). Then we **show it doing actual work** across nine scientific and
industrial domains (Act VI — crypto, ML, finance, medical, physics, chemistry,
engineering, statistics, plus linear-algebra BFV/DP). Then we **show what it
looks like** (Act VII — the volumetric renderer, the Tensor10D projector, the
acoustic-visual parity). Then we **show the quantum arm** (Act VIII — QPU
dispatch, QUBO, VQE, lattice, DFT). Then we **show it talking to people**
(Act IX — chat, relay, agents, guardianship). Then we **show it everywhere**
(Act X — desktop, CLI, browser, mobile, Solid). Then we **show the seam**
(Act XI — the MCP server, the ontology MCP, the cooperation gating). Then we
**close on the human** (Act XII — Timothy, the project, what it is for).

## 4. What we never say

- We never say "AI." We say *the engine*, *the LLM*, *the inference loop*.
- We never say "next-generation," "revolutionary," "groundbreaking." Show, don't tell.
- We never say "Ollama," "llama.cpp," or imply any external model server. There is none.
- We never anthropomorphise the system. It is software. It has constraints. The constraints are the point.
- We never use the words "just" or "simply." Nothing here is simple. Some of it is small.

## 5. What we always say

- *48-byte NQuin.* Whenever we touch the wire format, name its width.
- *42-megabyte SLG Arena.* Whenever we touch the working memory, name its ceiling.
- *Zero heap in hot paths.* Whenever we explain why something is fast, name the discipline.
- *Caller-supplied output buffers.* Whenever we explain why something is safe, name the mechanism.
- *Real implementation.* Whenever we show a capability, name the file and the test count.

## 6. Visual language

- **Cyan-blue node graph** (the project icon: five nodes around a center node) is the recurring motif. It appears as a watermark, as a transition, as the "thinking" indicator.
- **Hex dumps** of 48-byte NQuins appear on screen whenever we discuss the wire format. They are real bytes. They are not stock footage.
- **wgpu capture frames** of the volumetric renderer appear whenever we discuss rendering. They are real frames from `webizen-render`.
- **Telemetry HUD** (memory pressure, network ripple, baking crystallization, logic flashes, inference heat, quantum activity, spectral shift, temporal pulse, epistemic density, manifold pressure) is the "pulse" of the film — it pulses in the corner of nearly every shot.

## 7. Cast of concepts (one-line each)

| Concept | One-line description |
|---|---|
| **NQuin** | A 48-byte, six-`u64` semantic datum. The unit of meaning in QualiaDB. |
| **SLG Arena** | A 42-megabyte, 917,504-Quin ring buffer that holds the working memory. |
| **Webizen VM** | A bounded bytecode engine that fires rules over the Arena. |
| **Sentinel** | A bifurcated thread that reads logits and can `DenyRollback` mid-generation. |
| **Deontic logic** | Obligation / permission / prohibition, with a defeater bit. |
| **Paraconsistent logic** | Routes contradictions to an isolated sub-context without halting. |
| **Tensor10D** | A 10-dimensional projector that turns manifolds into scenes. |
| **P64** | A 64-byte-aligned weight container with CRC-32C and a 10D manifold coordinate per tensor. |
| **WGSL Forge** | A typed IR and deterministic emitter that produces, validates, and tunes WGSL kernels. |
| **BFV** | Brakerski-Fan-Vercauteren homomorphic encryption for exact integer/decimal arithmetic over encrypted data. |
| **DP** | Differential privacy with calibrated Laplace/Gaussian noise, basic + advanced + RDP accounting. |
| **MCP** | Model Context Protocol. The seam between the engine and the outside world. |
| **Cooperation gating** | The MCP cooperation layer that refuses unverified callers. |

These eleven concepts are the vocabulary. Every act uses them.

## 8. Pacing

- **Act I (Origin):** slow, quiet, dark. Music: solo piano or no music. ~90 seconds.
- **Act II (Architecture):** the engine wakes up. Music: low synth pulse. ~120 seconds.
- **Act III (Modalities):** a montage. Each modality gets 4–6 seconds. Music: rhythmic, building. ~180 seconds.
- **Act IV (Inference):** the engine talks. Music: warm, present. ~150 seconds.
- **Act V (Governance):** the engine refuses. Music: tense, deliberate. ~120 seconds.
- **Act VI (Specialized libraries):** the engine works. Music: industrial, varied. ~180 seconds.
- **Act VII (Render):** the engine shows itself. Music: ambient, expansive. ~120 seconds.
- **Act VIII (Quantum):** the engine reaches outside classical silicon. Music: glassy, high. ~90 seconds.
- **Act IX (Chat & relay):** the engine talks to people. Music: human, warm. ~120 seconds.
- **Act X (Surfaces):** the engine is everywhere. Music: percussive, fast. ~90 seconds.
- **Act XI (MCP):** the engine meets the world on its own terms. Music: precise, careful. ~90 seconds.
- **Act XII (Outro):** the engine is for someone. Music: returns to solo piano. ~90 seconds.

Total runtime: **~25 minutes.** This is a short documentary, not a sizzle reel.

## 9. How to read these files

Each act file has the same structure:

1. **Title card** — what the act is called.
2. **One-sentence thesis** — what this act proves.
3. **Voice-over script** — the narration, paragraph by paragraph. Each paragraph is one shot.
4. **On-screen notes** — what should be visible while the voice-over plays.
5. **Source code anchors** — the files and tests that prove what we are claiming.

When NotebookLM reads these files, it should treat the *voice-over script* as the
authoritative narration. The *on-screen notes* are prompts for the editor. The
*source code anchors* are not narrated; they are the receipts.

## 10. The single most important rule

**Every claim in this film is provable from the repository.** If a future reader
opens the file we cite, runs the test we name, and looks at the byte we point
at, they should see the same thing the narrator described. If they don't, the
narration is wrong and must be fixed before the film ships.

This is not a marketing video. It is a documentary about working software.
