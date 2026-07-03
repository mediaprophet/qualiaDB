# ADR (DRAFT) — Sanctuary Vault v2: CBOR-LD harmonization + coercion-response decoy mirroring

**Status:** DRAFT for Timothy's sign-off. Supersedes nothing yet; extends
[adr-sanctuary-threat-model](adr-sanctuary-threat-model.md) (D1–D6) with the format + decoy-model
decisions raised 2026-07-03.

**Origin:** Timothy's direction — (1) harmonize the vault to CBOR / CBOR-LD like the rest of the
system, and (2) a coercion-response decoy model where activity in the *decoy* session is surfaced in
the *real* lane (for the victim's awareness / evidence), the real user can curate the decoy to sustain
the coercer's false belief, and the whole thing has evidentiary value if the victim cannot escape or an
investigation is under way.

---

## 1. Is there "bifurcation in the q42 file structure" that helps? — Yes, but be precise

There are (at least) three distinct bifurcations in the q42 stack. They are **structural / semantic /
governance** bifurcations — routing and provenance — **not cryptographic confidentiality boundaries**.
That distinction is the crux.

| Bifurcation | Where | What it separates |
|---|---|---|
| **`w` Manifold Index / "Topological Bifurcation"** | `q42-10d-tensor-standard.md` §4.3 | Domains/manifolds (Medical / Legal / Personal), with cross-domain "wormhole" correlation between them |
| **Natural-person vs software-agent DID** | `Q42VolumeHeader` (quin `object` upper-4 bits) | *Who authored* a quin — provenance / agent type |
| **`wf:` bifurcated CRDT consensus** | `webizen-protocol-rfc.md` §Tri-Party | WellFair data (no auto-merge; manual Tri-Party Authorization) vs Commons `qp:` (Lamport LWW) |

**What this buys the decoy model:** the `w`-manifold bifurcation gives a clean, standards-native *place*
to model **real / decoy / audit as distinct manifolds** (distinct `w`), and the DID bifurcation gives a
ready-made **provenance tag** for "who wrote this decoy-session entry" — exactly what an evidentiary audit
log needs. The `wf:` bifurcated-CRDT + Tri-Party model is the right governance frame for consent over that
data.

**What it does NOT buy:** the manifold index is not a key boundary. Putting real in `w=0` and decoy in
`w=1` does **not** keep a coercer out of `w=0` — only *distinct keys* do. So the bifurcation is the
skeleton; **confidentiality and one-way visibility still come entirely from the crypto we layer on top.**
Anyone who claims the q42 bifurcation *is* the decoy isolation is wrong; it is the container, not the lock.

## 2. CBOR-LD harmonization (Timothy: yes)

**Decision (recommend: yes).** Move the vault serialization from `serde_json` to CBOR-LD, aligning with
the project standard (`ciborium` is already a core-db dep; there is a real `cbor_parser::parse_cbor_ld_stream`;
CBOR-LD is a first-class `SemanticFormat`). Also fix the mislabel where `companion_bundle` writes
`serde_json::to_vec` into a `package-manifest.cbor`.

**What it buys, honestly:** (a) consistency with the graph/ingestion path; (b) the lanes + audit log
become q42 quins/manifolds — expressible in the graph, provenance-tagged, governed by the `wf:` model;
(c) binary, so a coercer who opens the file in a text editor sees bytes, not the literal words `real` /
`decoy`. **What it does NOT buy:** cryptographic hiding. CBOR-LD is self-describing — anyone who *decodes*
it recovers the structure. So this is consistency + expressiveness + casual-readability, **not** deniability.

## 3. The coercion-response decoy model (Timothy's design)

The intent: **decoy activity → reported into the real lane; real user curates the decoy; evidentiary.**
There is a fundamental tension to resolve first, then two crypto primitives that resolve it cleanly.

**The tension:** in a decoy session you have the *decoy* PIN, never the *real* PIN, so the real lane's key
is never derived — you therefore **cannot write into the real lane from a decoy session.** A naive "log the
coercer's edits into the real lane" is impossible as stated. The fix is asymmetry.

### 3.1 Decoy → real: a write-only, blind audit channel (asymmetric crypto)

At setup, generate an **audit keypair**. The **public** key is available in *any* session (including
decoy); the **private** key lives only in the real lane (wrapped under the real key). Then:

- Every decoy-session mutation (add / edit / delete a note) appends an **audit record** — timestamp,
  action, a copy/diff of what the coercer wrote, and the DID-bifurcation provenance tag — **encrypted to
  the audit public key** into an append-only region.
- The decoy session can **write** these records (public-key encrypt) but **cannot read** them (no private
  key) — so the coercer cannot tell they are being logged, and cannot tamper-read or selectively delete
  with knowledge of content.
- On the next **real** unlock, the real lane unwraps the audit private key and **decrypts the coercer's
  activity** — the victim sees exactly what was added/changed under duress and decides: **keep** it,
  **keep it updated** (ongoing evidence if they can't yet escape / an investigation is live), or **archive**
  it. Append-only + provenance-signed = evidentiary.

This is the standard "write-only log / receiver-decrypts" pattern and it is sound.

### 3.2 Real → decoy: a one-way key hierarchy (curate the decoy)

To let the real user **maintain the decoy** (seed it, keep it lived-in, add plausible updates so the
coercer's re-unlock shows fresh, believable content and reinforces "I have the real one"):

- At setup, **wrap the decoy lane key under the real lane key.** In a real session the app can unwrap the
  decoy key and write into the decoy lane — *without* the user re-entering the decoy PIN.
- The reverse is impossible: the decoy lane holds nothing that unwraps toward real. **One-way: real
  governs decoy; decoy is sealed from real.**

Result: the victim, in a safe real session, curates the decoy; under coercion the decoy behaves as a
fully-live vault (it already does — confirmed in code); the coercer's own writes persist *and* the seeded
updates make it plausible.

## 4. The load-bearing trade-off (the one place I'd have you decide, not me)

Every enrichment in §3 — the wrapped decoy-key, the audit keypair, the append-only audit region — **adds
on-disk structure**. To a coercer who only *uses the app*, this is powerful and invisible. To an adversary
who *inspects the bytes* (forensics, a savvy partner with a hex editor), each added relationship is **more
evidence that a superior real lane exists behind the decoy.**

So there is a real fork, and it cannot be fully won both ways in one container:

- **A — Functional decoy (recommend for the intimate-partner / coercive-control threat).** Rich decoy
  mirroring + audit + curation. Optimizes for a coercer who *uses the device*. Accepts that a *forensic*
  inspector can see a two-tier structure exists. This matches the stated threat (a partner demanding the
  PIN), and delivers the evidentiary value.
- **B — Indistinguishable hidden volume (VeraCrypt-class).** No on-disk marker of a second lane at all;
  fixed-size opaque container. Optimizes for the *forensic/border* inspector. Costs the rich mirroring
  (a hidden volume can't safely host a real→decoy key hierarchy without leaking it), and is a much larger
  build.
- **C — Layered (both, later).** B on the outside (opaque container), A on the inside (rich model within
  the hidden volume). Most protective, most work; a distinct future workstream.

**My recommendation:** ship **A** now (it fits the coercive-control threat you're actually describing and
carries the evidentiary value), name its limit plainly in the UI, and keep **B/C** as a scoped future
option. **This is your call, not mine — it's a values decision about which victim you're protecting first.**

## 5. Argon2id + PIN policy (already built; ADR D1/D5)

Landed and format-independent (survives the CBOR-LD move unchanged — same per-lane `KdfDescriptor`):
Argon2id (64 MiB, t=3, p=1) for new vaults, a per-lane KDF descriptor (so old PBKDF2 vaults still open),
and a PIN-strength floor (min 6, blocks all-identical / sequential / common). **Lazy PBKDF2→Argon2id
re-derivation was intentionally NOT finished on the JSON format** — it folds into the v2 re-serialization
so we migrate the container **once**, not twice.

## 6. Decisions — RESOLVED (Timothy, 2026-07-03)

- [x] **D-fmt:** CBOR-LD for the vault — **yes**.
- [x] **D-decoy-model:** the §3 asymmetric-audit + one-way-key-hierarchy design — **approved**.
- [x] **D-fork (§4):** **Option A (functional decoy)** — chosen. B/C scoped as future options. The
      byte-level two-tier visibility is accepted and must be stated plainly in the UI.
- [x] **D-retention:** **both** — a real-session toggle (see §8). Default = auto-archive.
- [x] Sequencing: Argon2id/PIN landed (`a60671a1`); v2 = CBOR-LD + lazy migration + decoy mirroring, one build.

## 7. Two corrections to the design before build (precision, not agreement)

The design is sound, but two claims in the review round need fixing or the "evidence" it produces is
misleading:

1. **Provenance axis (must fix).** The proposal tags coercer actions with the *software-agent DID* to
   distinguish them from the user's. That is wrong: **the coercer is a natural person**, so labelling
   their writes "software agent" mislabels the very evidence we want. The Q42 natural-person /
   software-agent DID bifurcation encodes *agent type*, not *trust*. What we actually need is a separate,
   orthogonal **session-context** field on each audit nquin — `authored_in: real_session` vs
   `decoy_session (actor unauthenticated / under duress)`. We keep the true agent-type DID *and* add the
   session-context axis; we do **not** overload the DID bifurcation to mean "coercer."
2. **"Tamper-proof forensic record" → calibrate to "tamper-evident to the owner".** The sealed-box gives
   confidentiality + integrity *to the real-lane holder* (the coercer can't read it or silently alter it
   undetected). That is **not** court-admissible evidence, which additionally needs chain-of-custody,
   trusted timestamping, and device attestation. Build and label it as **tamper-evident to you**;
   external legal admissibility is a larger, separate effort and the UI must not over-promise it.

**Crypto primitive (specificity):** the blind append is an **X25519 sealed box** (anonymous ECIES:
ephemeral X25519 + AEAD to the audit public key) — the decoy session needs no identity of its own, just
the public key. The real lane holds the X25519 *secret* wrapped under the real lane key; the decoy lane
key is likewise wrapped under the real key (one-way hierarchy). Audit region is append-only.

## 8. Retention toggle + UI copy (locked)

Set **only in the real session** (invisible + unreachable from the decoy session, or it tips the coercer).
Default = **auto-archive** (a user in crisis has no spare cognitive bandwidth).

- **Auto-archive (default):** on real unlock, decrypted audit nquins are bound to an append-only holding
  region automatically, carrying the `decoy_session` provenance (§7.1). Zero-touch.
- **Manual triage:** on real unlock, the audit nquins appear as an unclassified inbox; the user explicitly
  keeps / discards / classifies each. Nothing is retained until they act.

Plain-language copy (no jargon; second-person; names the limit honestly):

> **If you're ever forced to unlock**
> Your cover space can quietly keep a record of anything someone changes when they make you open it — so
> you stay in control of your real space.
>
> ◉ **Save a record for me automatically** *(recommended)* — You don't have to do anything in the moment.
>   Next time you open your real space, you can look back at what happened.
> ○ **Let me choose each time** — Next time you open your real space, you'll see what changed and decide
>   what to keep or delete. Nothing is saved until you do.
>
> *This protects you from someone using the app. It can't hide your real space from an expert examining
> the files on your device.*

Term note: "cover space" for the decoy is placeholder — Timothy may recoin it. The feature and this
setting must never render in the decoy session.

## 9. Layer multiplicity & agent bifurcation — generalize the container, constrain the policy

Timothy's framing (2026-07-03): bifurcation is between **two or more agents of any nature** (human, a
natural person acting via software, a software agent, a weather-simulation model, …), and the real
question is the **number of bifurcated layers**, not the agents' type.

- **Bifurcation is n-way and agent-agnostic.** Not "human vs software", not "victim vs coercer" — it
  separates *n* distinct agents/contexts, each with its own DID + independently-derived key, addressed by
  a layer coordinate (the CBOR-LD/quin manifold index `w`). The same mechanism serves decoy lanes, N
  climate models, delegated agents, multi-tenant data.
- **No new opcodes** (answers Timothy's question directly). In this system opcodes (`0x10+`) are for logic
  **modalities** — `mini_parser` owns `0x00–0x04`. Agent identity and layer count are **data** (DIDs,
  manifold coordinates, quins), never opcodes. Minting an opcode per agent-type or per-layer would break
  the extensibility model. Let the DIDs + `w` coordinate do the work.
- **The crypto primitives are already n-agnostic.** X25519 sealed-box, key-wrap, per-layer AEAD operate
  per-key/per-region — there is no "n" inside them. So "support arbitrary n" = make the **container** a
  collection of independently-keyed layers (not a hardcoded `{real, decoy}` pair). Do this — it is nearly
  free and generalizes; it also naturally expresses the one-way hierarchy (a superior layer wraps
  subordinate layer keys).
- **Provenance = per-layer DID, which supersedes the §7.1 `session-context` field.** Each layer/session
  carries its own DID; a record's provenance is `(authoring DID, layer)`, from which "decoy/duress
  session" is *inferred* (the record is bound to the decoy-layer DID). The natural/software agent-**type**
  flag stays truthful and orthogonal (a coercer is a natural person operating under the decoy-layer DID).
  This is cleaner than a bespoke session flag and generalizes to the weather-model case. **Adopt this;
  drop the §7.1 session-context axis.**

**Sanctuary policy constrains what the protocol permits — for safety, not just UX.** The container supports
n; **Sanctuary must not expose n.** For the coercion threat, more decoys is *worse*:
1. **Cognitive collapse** — remembering an escalating PIN sequence under acute stress is a failure point.
2. **The coercion-loop trap** — under Option A the layer structure is byte-visible; an attacker who sees
   (or is told about) many spaces simply keeps demanding PINs until one is "real". More layers = more rope.
3. **Footprint growth is itself a side-channel** — an auto-growing container signals activity.

Therefore, **against Timothy's "spawn a fresh untouched decoy on each use" instinct, do NOT auto-spawn.**
The goal (always have a plausible, untouched-looking decoy) is met better by **wipe-and-reseed**: from the
real lane's top-down access, reset + re-seed the single decoy and rotate its duress PIN. Recommended
Sanctuary shape:
- Default **real + exactly one decoy**; optionally a *small user-created* set (≤3), never auto-grown.
- **Wipe-and-reseed** lifecycle, not auto-spawn — keeps PIN count minimal and the container shape stable.
- **Constant-shape container** — allocate a fixed number of layer slots and pad, so the on-disk layer
  *count* reveals nothing about how many are real / decoy / empty (a cheap hardening even under Option A).

**Build scope decision:** build the primitives + container **n-general** (they're n-agnostic anyway), and
enforce the small-fixed-shape **in the Sanctuary policy layer**, not the container. Wire `real` + one decoy
for the UI now.

## 10. The decoy history as a git-like DAG (Timothy's refinement — adopted over §9 wipe-and-reseed)

Timothy: use a git-like method over the *existing* decoy — a new entry point per session — to track the
number of attackers; and if attackers share credentials, other (behavioral) logs matter. This is better
than both auto-spawn and my wipe-and-reseed; it also resolves the evidence-vs-plausibility tension. Adopt,
with three precisions:

1. **Working tree vs history — the load-bearing split.** The decoy's *visible content* is a **working
   tree** the coercer sees and mutates — tamperable, and that's fine (it's cover). The **truth** is a
   separate **append-only, hash-linked audit DAG** (git-like: content-addressed nodes via `q_hash`, parent
   pointers, per-node author + timestamp), written as **X25519 sealed-box commits** the coercer can *append*
   (encrypt-to-audit-pubkey) but cannot *read or rewrite*. Evidence integrity lives in the DAG, **never** in
   the coercer-controlled working tree — otherwise the coercer, who owns the decoy, can `git reset` their
   tracks away.
2. **Branch per session, one PIN (not PIN-per-attacker).** A single decoy PIN, but each duress unlock opens
   a **new ref/branch** in the audit DAG. Per-session attribution *without* making the user remember an
   escalating PIN sequence (the §9 cognitive-collapse failure). "Number of attackers" ≈ **number of distinct
   entry-point sessions** — a proxy / lower bound, not a verified headcount.
3. **Collusion is a crypto blind spot — Timothy's caveat is exactly right.** Shared credentials → one entry
   point → reads as one attacker; one attacker over many sessions → reads as several. Disambiguation needs
   **behavioral "other logs"** (unlock timestamps, inter-action timing, device/sensor context, recurring
   edits) layered *on top* of the structural branch count — and even then it is heuristic; a determined
   colluding pair is indistinguishable from one persistent actor at the crypto layer. **The UI must not
   claim a hard attacker count.**

**This unifies evidence + plausibility.** History is append-only-preserved in the sealed DAG (evidence,
even across resets); the *presented* decoy working tree can be reset/reseeded by the real user
(plausibility) — curate the working tree, never lose the history. Supersedes §9 "wipe-and-reseed" as a
*destructive* op.

**Build it as git *concepts* in the quin/CBOR-LD model — not an embedded git repo.** Quins are already
content-addressed (`q_hash`); the DAG is quins with parent-hash links + refs. No libgit2, no
history-rewrite footguns.

**Integrity boundary (from the slice-C adversarial pass — a real constraint, not a bug).** The chain
link is *unkeyed* `BLAKE3(parent ‖ payload)` — **tamper-evident, not a MAC.** Anyone holding the audit
*public* key can forge a fresh internally-consistent chain, and a coercer with full device access can
destroy or wholesale-replace the (still unreadable) log. So **S5 MUST anchor each session-branch head**
in something the decoy session cannot forge or reset — pin the head into the *real* lane on review,
and/or a keychain-held monotonic anchor — so that *partial* tampering / dropped records are detectable.
And the UI must say plainly (Option A): this defeats a coercer who *uses the app*, not one who does
forensic surgery on the device's files. The sealed-box confidentiality is unconditional; the log's
*completeness* is only as strong as the head anchor.

**Does not change the first build slice:** the foundation is still **X25519 sealed-box + `q_hash`
content-addressed hash-chaining** — the primitives already planned, now structured as an append-only commit
DAG.

## 12. Implementation slices — build order + honest swarm curation

Vault v2 is **one coupled, security-critical container**. Most of it is sequential surgery on
`sanctuary_vault.rs` and cannot be safely fanned out to parallel agents (they would collide on the same
module and the crypto must not be built blind). The clever part is *what to parallelise*: only the
genuinely isolated NEW-file pieces (self-contained domain logic, UI, adversarial tests). The integrator
owns every contract below and every edit to the vault crypto.

**Done**
- **S0 — audit primitives** `core-db/crypto/sanctuary_audit.rs`: X25519 sealed box + key-wrap +
  BLAKE3 hash-chain. 8 tests. (commit `2d69a21f`, local, not pushed.)

**Parallel — swarm-safe (isolated NEW files, one registration file each, no cross-agent deps)**
- **A · Audit DAG + retention domain** — NEW `core-db/crypto/sanctuary_audit_dag.rs`. Self-contained on
  top of S0. `AuditRecord { id=chain_hash, parent, branch_ref, actor_did, role, stated_purpose, action,
  unix, sealed }`; `verify_chain` (recompute → detect tamper/gap); `derive_sessions` (group by
  `branch_ref` → **distinct entry-point sessions**, explicitly *not* a verified head-count);
  `route(records, RetentionMode::{AutoArchive, ManualTriage}) -> Routing`. ≥12 tests incl. chain-tamper,
  session grouping, both retention modes.
- **B · Retention-toggle Studio panel** — NEW `webizen-studio/.../decoy_retention_panel.rs` + host_client
  bridge stubs + nav. The §8 copy verbatim, default auto, real-session-only. Green host + wasm.
- **C · Adversarial primitive suite** — NEW `core-db/tests/sanctuary_audit_adversarial.rs`: independent
  forge/malleability/cross-recipient/reorder attempts over S0. All must fail-closed.

**Sequential — integrator only (coupled, security-critical; NOT swarmed)**
- **S3 · n-layer container + CBOR-LD codec** — `Layer { id, salt, kdf, verifier, records, audit_pubkey,
  wrapped_keys }`; constant-shape padded `Container { version, layers }`; ciborium encode/decode;
  `from_legacy_json` migration reader. This *is* the on-disk crypto layout — I build it.
- **S5 · vault surgery** — `VaultMeta`→container + CBOR-LD save/load; blind-append audit on decoy writes;
  real-lane DAG read; real→decoy key hierarchy (wrap decoy key + audit secret under the real key);
  Argon2id lazy migration in the new container.
- **S6 · host API + Tauri + bridges + nav** — record-on-behalf→decoy; real-lane audit review; retention
  set; curate-decoy; wire panel B.

**Order:** A · B · C fan out now. I build S3, then S5 (consumes S0/A/S3), then S6 (wires B). Every slice:
green build + tests before it counts. **Nothing pushed without Timothy's word.**
