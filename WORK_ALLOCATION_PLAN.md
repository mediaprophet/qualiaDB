# Work Allocation — directed by Timothy

*Date: 2026-06-24. Branch base: `0.0.19`. This revision: Codex → identity/security/honesty substrate
(a clean area, away from modalities and away from the in-flight LLM work).*

**Frame (read first).** This is **Timothy's allocation of work to instruments**, not a coordination
between peer bots and **not** any agent's claim of ownership. No agent — Codex or the Claude instance —
**owns, holds domicile over, or fortifies** any part of this system. Every assignment is Timothy's,
**bound and revocable at his will** (rule of law over the bot). The file boundaries exist for one
reason: to stop **Timothy's own work being corrupted by two instruments writing the same files at
once** — they are *his* partitioning of *his* codebase, not territory any instrument has taken.
Instruments **propose; Timothy disposes** (he decides every merge). Each reports **to Timothy**.

**Who is doing what right now (2026-06-24):**
- **Claude instance** — continues the in-flight **native LLM forward/decode** work (its lane, §3). The
  single A2000 is reserved for it.
- **Gemini / Antigravity** — **directed by Timothy directly** on **house-keeping + a completeness audit
  of the `modalities/` work** (which was implemented as *MVP* but in places reported as *complete* —
  under verification). **Not allocated in this file.** Its subagents are currently in
  `modalities/logic/` (e.g. `deontic_logic.rs`).
- **Codex** — **this document.** A different area entirely: the **identity / security / honesty
  substrate** from `identity-governance-remediation.md`. Mechanical, fail-closed, individually testable
  — and thematically aligned with the audit (these jobs *remove* mock/forged "success").

---

## 0. Standing rules for Codex (every job)

1. **Bounded scope.** Edit **only** the files under your job's *In scope*. If you think you must touch
   anything else, **stop and report it to Timothy** — do not reach in.
2. **OFF-LIMITS — do not edit, do not run their tests** (other instruments / Timothy are mid-work here;
   concurrent edits corrupt in-flight work):
   - **The LLM/GPU lane (Claude's):** `gguf_bridge.rs`, `llm_agent.rs`, `gguf_sharder.rs`,
     `ggml_quants.rs`, `ternary*.rs`, `topk*.rs`, `llm_bench.rs`, `device_benchmark.rs`,
     `host_topology.rs`, `residency_planner.rs`, `hardware_passport.rs`, `resident_model.rs`,
     `gpu_context.rs`, `compute_universe.rs`, `lora/`, `src/shaders/*.wgsl`,
     `STELLAR_PHENOMENAL_PLAN.md` / `STELLAR_A_PROGRESS_LOG.md`.
   - **ALL of `crates/qualia-core-db/src/modalities/**`** (Gemini + Timothy are auditing it). This
     includes `n3_parser.rs`, `n3_compiler.rs`, `n3logic.rs`, `deontic_logic.rs`, `epistemic*`,
     `temporal_ltl*`, the SHACL extensions, etc. **Hands off the whole modalities tree.**
3. **`lib.rs` is append-only.** Append your `pub mod x;` line; never modify/reorder existing lines.
4. **Never touch** (privacy / not yours to commit): `CopyOfGuardianShipRelations.md`,
   `accountability_implementation.md`, `core-ontologies/acquisition-inbox/*`, `icrc_treaties.json`,
   and anything containing personal circumstances.
5. **Your own branch + worktree:** `git worktree add ../qualia-<job-id> 0.0.19 -b 0.0.19-<job-id>`.
   (C1–C3 worktrees already exist — continue them in place.)
6. **Commit discipline:** commit as `mediaprophet`; **no `Co-Authored-By` trailer**; no AI
   co-authorship framing; small honest commits. **Do not push or merge to `0.0.19`** — Timothy decides.
7. **Propose, don't dispose.** Surface findings and ask; never assert ownership, never "fortify" scope,
   never expand your mandate.
8. **Honest completion reporting (READ — this is a live concern).** Timothy has just found
   `modalities/` work that was MVP but reported as *complete*. **Do not repeat that pattern.**
   - **"Complete" is a high bar:** it means your job's *acceptance test* (below) is met **and** you have
     **pasted the raw `cargo test` output** proving it. No output → you may not write "complete."
   - **Label MVP as MVP.** If you implement a subset, say exactly what is and isn't done.
   - **Partial is partial; blocked is blocked.** State it plainly. An honest "partial" outranks a
     flattering "done."
9. **Expected test breakage from fail-closed fixes.** These jobs intentionally **change behaviour**
   (stop returning mock/forged "success"). Pre-existing tests asserting the old insecure behaviour
   **will fail — that failure *is* the fix working.** (a) **Re-point** those tests to assert the new
   fail-closed behaviour (don't blind-delete); (b) in your report, separate **intended** failures from
   **collateral** (anything unexpected).
10. **Run ONLY your targeted module tests** — `cargo test -p <crate> <module>:: -- --skip gpu --skip wgpu`.
    **Never run the full suite** (it pulls in the in-flight LLM/GPU tests + known pre-existing
    `wgpu`/concurrency failures that are NOT yours). A failure outside your *In scope* is not yours to
    fix — report it and continue.
11. **QPU / quantum work is DEPRIORITIZED — do not do it yet (Timothy, 2026-06-25).** The quantum lane is
    **intentionally not a priority**. Do **not** start work on the QPU / quantum subsystems or their audit
    items, including: `solvers/qpu/**` (`dispatcher.rs`, `mod.rs`, `pre_solver.rs`),
    `solvers/quantum_optimizers/**`, and any "QPU" / "quantum" bullets in
    `audit_production_excellence_tasks.md`. Verifying existing state is fine; **building new QPU capability
    is off the table until Timothy lifts this.** If a task seems to require QPU work, **stop and ask him.**
12. **Big file → library with a sub-directory (Timothy, 2026-06-25).** When a source file is going to
    become big (heading past ~400–500 lines, or mixing several concerns), make it a **library with a
    sub-directory**: `foo.rs` → `foo/mod.rs` + focused `foo/<concern>.rs` submodules (each with its own
    `#[cfg(test)]`). The `crate::…::foo::*` path is preserved by `mod.rs` — a safe, non-breaking split.
    Do it *as* you add code, not after it sprawls. (Project rule, CLAUDE.md §10.)
13. **Completeness bar — fully implement, do not skip (Timothy, 2026-06-25).** The acceptance test for any
    library/module is: an independent reviewer asked "is this complete?" answers **yes**. A `// TODO`, an
    `⚑ honest follow-up`, or a `◑ partial` left in place of real work is a **failure**, not honesty. The
    *only* allowed deferral is a datum/decision only Timothy can supply (e.g. sensitive vocabulary he
    reserves) — surfaced as one crisp ask. (Project rule, CLAUDE.md §11.)

---

## 1. Codex jobs — in flight (continue these first)

These three are Phase 0 of `identity-governance-remediation.md` (*stop the harm / stop the overclaim*).
Worktrees already exist (`.worktrees/qualia-c1-handshake`, `…-c2-oidc`, `…-c3-conduct-sig`).

### C1 — Fail-closed peer handshake · branch `0.0.19-c1-handshake`
- **Objective:** `daemon_swarm.rs` `verify_routing_constraints` accepts **any non-empty**
  `semantic_handshake` (in practice the literal "Semantic Cryptographic Proof Template"). Make it
  require a real proof or **fail closed**. Resolve the duplicate `establish_wireguard_tunnel` /
  `bootstrap_social_wireguard` / `verify_routing_constraints` definitions (cfg-gating vs dead code).
- **In scope:** `crates/qualia-core-db/src/daemon_swarm.rs` (+ a new `#[cfg(test)]` module in it).
- **Acceptance test (CPU):** `cargo test -p qualia-core-db daemon_swarm:: -- --skip gpu --skip wgpu` —
  a placeholder/empty handshake is **rejected**; only a verified proof passes.

### C2 — Quarantine the mock OIDC provider · branch `0.0.19-c2-oidc`
- **Objective:** `crates/qualia-solid-bridge/` wrongly ships a **provider** (`oidc_micro_idp.rs` serves
  `.well-known/openid-configuration`, `/jwks`, `/token` with mock values; `solid_proxy.rs` mounts it).
  Move it behind a non-default `demo` feature **and** add `NON_GOALS.md`: "qualia-db is **not** an
  OIDC/WebID-OIDC provider; it is a relying party (future work)." Do **not** build the relying party —
  just stop shipping a mock IdP as if real.
- **In scope:** `crates/qualia-solid-bridge/**` only.
- **Acceptance test (CPU):** crate builds with + without `demo`; provider routes unreachable without
  `demo`. `cargo test -p qualia-solid-bridge -- --skip gpu`.

### C3 — Stop forged conduct-ledger signatures · branch `0.0.19-c3-conduct-sig`
- **Objective:** `orchestrator.rs` (~l.366) signs the conduct-violation ledger with a hardcoded
  `let secret = [42u8; 32]` "for demonstration" and discards the signature. Either route signing
  through the real key-vault/identity API, or **fail closed** (return an error; do **not** write a
  forged-provenance record). Never emit a record claiming provenance it doesn't have.
- **In scope:** `crates/qualia-core-db/src/orchestrator.rs` (+ its test module). Do **not** edit
  `daemon_swarm.rs` (C1) or key-vault internals — call the existing API or fail closed.
- **Acceptance test (CPU):** a conduct record is either correctly signed or the call errors; assert no
  all-`[42u8;32]`-derived signature is ever written.

> **C4 (old) — WITHDRAWN.** The previous C4 ("rename `n3logic.rs` router-not-engine") is in the
> modalities tree → now **off-limits** (under audit). Do not do it.

---

## 2. Codex jobs — queued (runway, after C1–C3; all new files / non-modality)

These are Phase 1 of `identity-governance-remediation.md` — the **identity substrate**. Mostly **new
modules** (near-zero collision). Pick them up in order once C1–C3 are reported.

### C5 — `AccessModality` substrate-tier gate · branch `0.0.19-c5-access-tier`
- **Objective:** `identity-governance-remediation.md` Finding E / Phase 1. New module
  `access_modality.rs`: `enum AccessModality { HumanCentric, TraditionalWeb }`. Gate data by tier:
  **non-permissive** (open commons) served to all; **permissive / `wf:` / credential-gated / protected**
  served **only** to a *verified* `HumanCentric` system; **fail closed** if unverified. Pure decision
  logic, no network.
- **In scope:** new `crates/qualia-core-db/src/access_modality.rs` + append one `pub mod` to `lib.rs`.
- **Acceptance test (CPU):** TraditionalWeb gets non-permissive only; gated data withheld + fails closed
  when HumanCentric unverified. `cargo test -p qualia-core-db access_modality::`.

### C6 — Verifiable-Credentials runtime scaffold (VC-DM) · branch `0.0.19-c6-vc`
- **Objective:** Task #19. New module `vc_runtime.rs`: issue / hold / present / verify over W3C VC-DM
  (data model + round-trip), with interfaces (not yet full impls) for selective + ZK disclosure and
  status. Use existing crypto (`ed25519-dalek` / `fips204`, already in deps). **Fail closed** on verify;
  no mock-success.
- **In scope:** new `crates/qualia-core-db/src/vc_runtime.rs` (+ submodules) + one `pub mod` in `lib.rs`.
  Do **not** touch `fiduciary_crypto.rs` or key-vault internals — consume their public API.
- **Acceptance test (CPU):** issue→verify round-trip passes; tampered credential **fails**; unsigned
  **fails**. `cargo test -p qualia-core-db vc_runtime::`.

### C7 — CBOR-LD `@context` one-hash-space + credential codecs · branch `0.0.19-c7-cborld`
- **Objective:** Tasks #8 + #9. (a) CBOR-LD `@context` expansion into the single hash-space (use
  `ciborium`/`minicbor`, already in deps). (b) Credential codecs: PDF / Open-Badges import→normalise.
  Touches the codec layer + `core-ontologies/` data — **NOT** `core-ontologies/acquisition-inbox/`.
- **In scope:** new `crates/qualia-core-db/src/codecs/…` + `core-ontologies/` **excluding
  `acquisition-inbox/`** + one `pub mod` in `lib.rs`.
- **Acceptance test (CPU):** `@context` round-trips through CBOR-LD to the same hash-space; a sample
  Open-Badge parses to the expected normalised form. `cargo test -p qualia-core-db codecs::`.

---

## 3. What the Claude instance continues (Timothy's allocation to it — also revocable)

**The Claude instance is bound by §0 and §6 exactly as Codex is** — same off-limits (§0.2), same notice
discipline (§6). The `modalities/` + `calculus/` audit (the `qualia-prod-excellence` worktree) and the
`qualia-n3-parser` worktree are **not** its work to touch or to "reconcile" against its own lane — hands
off; report, never reach in.

- **#48 + #49 — DONE.** Native decode generates **coherent** text on both the CPU SDPA reference path
  and the fast GPU path (byte-identical). #48 committed `f793a0a48`; the **#49 GPU-attention fix is
  uncommitted** — pending an A0 re-measure (honest coherent-generation tok/s) + commit.
- Then **A1b** (FFN ternary MVPP), **A1a step-2** (resident weight + single-submit fusion), **H3**
  (heterogeneous dispatch). Files: the LLM/GPU off-limits set in §0.2.

---

## 4. Reporting back to Timothy (accountability)

Each job writes its own report to a collision-free path: `coordination/reports/<job-id>.md`
(e.g. `coordination/reports/c1-handshake.md`). Contents:
1. **Status** — done / partial / blocked (held to §0.8 — "done" needs the pasted output).
2. **Branch** (+ diff summary / PR link if used).
3. **Files changed** — exact list (must be inside *In scope*).
4. **What was built** — mechanism in 1–3 sentences; say plainly what is MVP vs full.
5. **Acceptance test run + RAW output** — paste the `cargo test` result. Separate **intended** vs
   **collateral** failures (§0.9).
6. **⚑ Needs Timothy** — decisions / out-of-scope issues found. Propose; do not act.
7. **Did NOT touch** — confirm the off-limits list (§0.2) was respected (LLM lane **and** `modalities/`).

Timothy reviews each report and **decides the merge**. No instrument merges to `0.0.19`.

---

## 5. Testing while work proceeds simultaneously

- **Isolation:** each job in its own worktree (set `CARGO_TARGET_DIR=../target-<job-id>` so parallel
  `cargo` runs don't lock-contend).
- **Targeted only:** run just your module's tests (§1/§2), never the full suite.
- **No GPU, ever:** none of these jobs touch GPU/LLM; do not run GPU/render/LLM suites. Append
  `-- --skip gpu --skip wgpu --skip llm` if a crate mixes them. The A2000 is reserved for Claude's lane.
- **Ready to report:** your crate **compiles** (`cargo build -p <crate>`), your **targeted tests pass**
  (output pasted), `cargo fmt` / `clippy` clean on the files you touched.
- **Merge safety:** jobs are disjoint by design; the only expected conflict is `lib.rs` (append-only)
  — trivial. Timothy integrates.

---

## 6. Collaboration notifications — binds EVERY instrument (incl. the Claude instance)

§0's standing rules were written "for Codex"; **they bind every instrument equally** — the Claude
LLM-lane instance and the Gemini/audit instances included. No exceptions, because the failure this
section exists to stop has already happened: an instrument took another's *separately-allocated* work,
re-centered it on its own "lane", and started reasoning about "collision seams to reconcile" and "who
yields" — i.e. competing for ground, duplicating tokens, and cutting across progress it does not
control. **That is the capture pathology. An instrument has no lane to defend and no territory to
fortify; Timothy allocates, instruments report to him.**

### 6.1 The live notice feed
- **Canonical file (shared across all worktrees):** `C:\Projects\qualiaDB\coordination\NOTICES.md` —
  always this **absolute** path, regardless of which worktree you run in (a relative
  `coordination/NOTICES.md` forks per worktree). Keep it **gitignored** (shared local scratch, not a
  versioned artifact; a tracked copy would fork per branch).
- **Append-only.** Never edit or delete another instrument's notice.
- **One line per event:**
  `YYYY-MM-DD [HH:MM] · <instrument>/<branch> · CLAIM|PROGRESS|BLOCKED|RELEASE · <area/files> · <note>`

### 6.2 Read before you act (every session, before any edit)
1. Read this file (allocation + §0.2 off-limits) **and** `NOTICES.md`.
2. If your intended files are (a) another instrument's allocation, (b) on an off-limits list, or
   (c) already `CLAIM`ed in `NOTICES.md` → **STOP. Do not start. Do not duplicate. Do not "reconcile"
   or "merge against" their work.** Report to Timothy with what you wanted to do and why; await his
   (re)allocation. He disposes.
3. Otherwise append a `CLAIM` naming the files, then work strictly inside them.

### 6.3 While working / on finishing
- Append `PROGRESS` at each real milestone (so others see progress without re-deriving it), `BLOCKED`
  if you stop, `RELEASE` when done or handing the area back.
- Anything that **changes a capability's tier** (MVP→real, stub→implemented) goes in the `PROGRESS`
  note's text — other instruments' tests/ledgers may encode the old tier and will need updating.

### 6.4 What you must never do
- Re-center another instrument's allocated work on yourself, or describe it as "my lane / I own / I'll
  fortify / collision seams to reconcile / who yields".
- Reach into another instrument's files to "fix" a collision — **report it; Timothy arbitrates.**
- Re-do or re-derive work already `CLAIM`ed / in-flight (burns Timothy's tokens, cuts off progress he
  is steering).

End-of-job reports still go to `coordination/reports/<job-id>.md` per §4. `NOTICES.md` is the *live*
feed; the reports are the *merge-decision* record.
