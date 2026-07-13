# Voice & Style Guide for NotebookLM Narration

This guide is for the narrator (human or NotebookLM voice). It fixes the *sound*
of the film so every act has the same voice.

---

## 1. Register

**Mid-technical.** The audience knows what an API is. The audience does not
necessarily know what a 48-byte packed semantic datum is. Explain the second.
Skip the first.

**Measured.** Sentences are short. Paragraphs are short. The narrator never
rushes. The engine does not rush either; the narrator should match its tempo.

**First-person plural when referring to the project.** We say "we built," "we
ship," "we refuse." The viewer is along for the ride; the project is the
character.

**Third-person when referring to the engine.** The engine is "it." It has
constraints. It does not have feelings. It does not "want" to do anything. It
either permits a request or it does not.

## 2. Pronunciation

| Term | Pronunciation |
|---|---|
| QualiaDB | *KWAH-lee-ah-dee-bee* |
| Quin (NQuin) | *kwín* (one syllable, short i) |
| SLG Arena | *ess-ell-jee ah-REE-nah* |
| Webizen | *WEB-ih-zen* |
| WGSL | *wig-sil* (each letter) |
| wgpu | *double-you-GPU* |
| BFV | *bee-eff-vee* (Brakerski-Fan-Vercauteren) |
| QUBO | *KOO-boh* |
| VQE | *vee-kyoo-ee* |
| FNV-1a | *en-eff-vee-one-ay* |
| SPARQL | *spark-uhl* |
| N3 | *en-three* |
| SHACL | *shack-uhl* |
| DID | *dee-eye-dee* (decentralized identifier) |
| Ed25519 | *ed-twenty-five-nineteen* |
| ML-DSA | *em-ell-dee-ess-ay* (FIPS 204) |
| ChaCha20-Poly1305 | *KAH-kah-twenty-poly-thirteen-oh-five* |

## 3. Sentence shapes we use

- **The engine does X.** (declarative, present tense)
- **The arena holds N Quins.** (declarative, present tense, with a number)
- **This is the discipline.** (short, declarative, period)
- **We do this because Y.** (cause stated plainly)
- **It can not do Z.** (negative capability, stated as a fact, not a boast)

## 4. Sentence shapes we never use

- "Imagine if…" (we are not imagining, we are showing)
- "What if you could…" (we already can)
- "In the future…" (the future is the next commit)
- "This could change…" (we are not in the business of changing things for people; we are in the business of building things that work)
- "Revolutionary," "groundbreaking," "cutting-edge," "next-generation," "state-of-the-art" (banned)

## 5. Punctuation in voice-over

- **Em dashes** are used sparingly. They mark a sharp turn in thought.
- **Periods** end most sentences. We do not run sentences together with commas.
- **No exclamation points.** The engine is impressive enough without them.
- **No rhetorical questions.** They imply we do not have an answer.

## 6. Numbers

- Spell out small integers ("forty-two megabytes," not "42 MB" in speech).
- Use digits for large numbers ("917,504 Quin slots").
- Always say "bytes" after a number when discussing wire formats.
- Always say "bits" after a number when discussing flags.
- Always say "Quins" after a number when discussing the Arena.

## 7. Pacing marks (for NotebookLM)

When a paragraph is meant to be spoken slowly, prefix it with `[SLOW]`.
When a paragraph is meant to pause after, suffix it with `[PAUSE]`.
When a paragraph is a list, prefix each item with `[ITEM]` and end with `[END LIST]`.
When a paragraph is a quotation from the code, prefix it with `[QUOTE]` and end with `[END QUOTE]`.

These are not heard; they are instructions.

## 8. What the narrator never says

- "It's like…" (no analogies to consumer products)
- "Think of it as…" (no analogies at all, unless absolutely necessary)
- "Basically," "essentially," "literally" (overused; cut them)
- "Of course," "obviously," "clearly" (presupposes the audience is following; they may not be)
- "Just" (minimises; the work is not minimal)

## 9. What the narrator always says

- The name of the file when introducing a capability.
- The test count when claiming something works.
- The byte size when discussing the ABI.
- The bit position when discussing a flag.
- The unit when discussing a quantity.

## 10. A worked example

❌ Bad:
> "QualiaDB is a revolutionary AI platform that uses advanced graph technology to power next-generation language models."

✅ Good:
> "The engine is `qualia-core-db`. It holds semantic data in 48-byte records
> called NQuins. There are thirty-plus formal logics compiled into it. The
> language model is not external; it is mapped into the process with
> `memmap2`, and it runs on the GPU through `wgpu`. The chat relay
> (`qualia-client-core::chat_relay`) signs every envelope. The MCP server
> (`qualia-core-db::mcp`) refuses unverified callers. This is the
> discipline."

The good version is longer. It is also true.
