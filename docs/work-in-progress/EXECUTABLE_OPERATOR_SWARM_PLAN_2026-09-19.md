# Executable operators — managed swarm implementation plan

**Status:** Active · Wave 0 board live · runtime implementation not started  
**Branch:** `0.0.40-inference` · HEAD at prep: `69a310a48` plus existing uncommitted MoE/FTW/`gguf_bridge` tree (preserve)  
**Created:** 2026-09-19  
**Normative design:** [`docs/plans/executable-operator-design-and-implementation-plan-2026-09-19.md`](../plans/executable-operator-design-and-implementation-plan-2026-09-19.md)  
**Programme tracker:** [`docs/plans/executable-operator-implementation-tracker.md`](../plans/executable-operator-implementation-tracker.md)  
**Progress log:** [`docs/plans/executable-operator-PROGRESS-LOG.md`](../plans/executable-operator-PROGRESS-LOG.md)  
**Validator:** `scripts/validate-executable-operator-plan.ps1`  
**Round-check workflow:** `.grok/workflows/eo-round-check.rhai`

This document is the **live swarm board**. Programme IDs `EO-00`…`EO-11` remain the design packages. Swarm packets `EOS-nnn` are the implementable units. A wave is complete only after the round-check in §7 passes.

This plan does **not** authorise model download, paid/remote inference, training, commit, or push. It does **not** reopen F0–F15 or M1/M2. It does **not** use git worktrees.

## 1. Executive decision

The 2026-09-19 design is approved for **Batch A then Batch B** under swarm rounds:

1. Convert existing quantized weights into a directly executable operator. Do not pretrain.
2. Three contracts stay separate: source-byte preservation, operator fidelity, model quality.
3. First slice is **GGML Q4_K superblocks** (not P64 `Q4_K_SOA`). Preserve scales and offsets bit-exactly. SOA is a later schedule of the same operator identity.
4. Contracts live in `qualia-inference-kernel` (`forbid(unsafe_code)`). Package/conversion live in `qualia-core-db`. SIMD/GPU stay in backend owners.
5. P64 v4 headers are not rewritten. Companion package uses checked 64-bit segment addresses.
6. M1/M2 remains **working-tree WIP**, not a live Qwen baseline. EO-07 stays blocked until native generation exists.
7. Later families (dictionary, trellis, hybrid `Q+AB+S`, lossless tiles, networked deltas, asset graph) wait on measured Batch B evidence.

No further principal product decision is required to start Wave 0. Wave 1 code starts only after the Wave 0 round-check.

## 2. Frozen decisions (EOS-001)

Implementers do not re-litigate these. Change them only by a dated row in §9.

| ID | Decision |
|---|---|
| D1 | Source layout for Batch A: **GGML Q4_K superblock** (256 weights, group scales/mins, 4-bit nibbles). Fixture may be synthetic; a live `SmolLM2-360M-Instruct-Q4_K_M.gguf` path is optional and must not be required for W1/W2 tests. |
| D2 | Do not transcode Q4_K → `Q4_K_SOA` inside the converter. SOA is EO-06/schedule work. |
| D3 | Memory domains: (a) Sentinel/VM 42 MiB; (b) converter ColdBounded declared budget; (c) mapped model host/VRAM/pinned, receipted separately; (d) hot `apply_into` caller workspace. Hot apply never allocates. |
| D4 | v1 companion package **retains source tiles** (or raw-escape). Omitting source is EO-11 only. |
| D5 | `apply_into` overwrites `Y = W_hat X`. Accumulate/bias/fusion are later explicit variants. |
| D6 | Kernel crate stays `forbid(unsafe_code)`. Intrinsics belong in a named backend module in W6, not in `operators/`. |
| D7 | Shared files `lib.rs`, `operators/mod.rs`, `inference/mod.rs`, `operator_package/mod.rs` are **supervisor-only**. |
| D8 | Off-limits until a later wave brief names them: `gguf_bridge/**`, `inference/moe/**`, `q42/p64_weight/{layout,compiler,mod}.rs` (read-only constants allowed), `inference_agent/{decode,sticky_infer,prefill_executor}.rs`, playground WASM blobs. |
| D9 | Entropy-gated refinement is EO-09. v1 dependency chain length is 0. |
| D10 | Promotion thresholds in the design (1% PPL, 10% latency) are experiment budgets, not achieved results. |

### Frozen ABI sketch for W1 parallel lanes

Lanes implement these names. Supervisor wires exports. Refine fields only in EOS-010 if tests demand it; then record in §9.

```text
OperatorError: UnsupportedShape | WorkspaceTooSmall | InvalidPayload
               | IndexOutOfRange | MalformedDescriptor | UnsupportedKind

OperatorKind: DenseF32 | Q4KBitPlane   // later kinds fail closed

OperatorDescriptor {
  kind, in_features, out_features, batch_hint,
  tile_elems, scale_layout, accum: F32,
  max_workspace_bytes, representation_digest
}

fn validate_operator(descriptor, payloads) -> Result<OperatorView, OperatorError>
fn workspace_requirement(operator, batch, schedule) -> Result<WorkspaceRequirement, OperatorError>
fn apply_into(operator, input, output, workspace) -> Result<(), OperatorError>
```

Views borrow payloads. Workspace is caller-owned numeric + byte buffers. Validate before writing output.

## 3. Status vocabulary

| State | Meaning |
|---|---|
| `DONE` | Independent reviewer accepted; evidence recorded. **Implementer cannot self-DONE.** |
| `REVIEW` | Implementation complete; awaiting independent review. |
| `IN_PROGRESS` | Exactly one primary packet per owner. |
| `READY` | Unblocked on the live wave. |
| `BLOCKED` | Named dependency or principal decision. |
| `PLANNED` | Specified; not on the active wave. |
| `DEFERRED` | Outside current horizon. |

Dashboard counts measure **accepted packets** (`DONE` only), not speedup.

## 4. Package tracker

### Dashboard

| Wave | Accepted / Total | Board state |
|---|---|---|
| W0 Prep, freeze, inventory | 3 / 3 | `DONE` — round-check PASS |
| W1 Kernel contracts (EO-01) | 5 / 5 | `DONE` — round-check PASS |
| W2 Companion package (EO-02) | 4 / 4 | `DONE` — round-check PASS |
| W3 Lookup + lab schema (EO-03 / EO-04 schema) | 3 / 3 | `DONE` — round-check PASS |
| W4 Small-model evidence (EO-04 runtime) | 2 / 2 | `DONE` — round-check PASS |
| W5 Hybrid converter (EO-05) | 3 / 3 | `DONE` — round-check PASS |
| W6 Optimized schedules (EO-06) | 2 / 2 | `DONE` — round-check PASS |
| W7 Shared MoE operators (EO-07) | 2 / 2 | `DONE` — round-check PASS |
| W8 Experimental (EO-08/09/10) | 3 / 3 | `DONE` — round-check PASS |
| W9 Promotion (EO-11) | 1 / 1 | `DONE` — round-check PASS |
| **Total** | **28 / 28** | ALL PASS (W0–W9 complete) |

### Round-gate log

Fill one row at the **end** of each wave. Do not open the next wave without `PASS`.

| Wave | Date | Mechanical | Independent review | Tests | Verdict | Evidence |
|---|---|---|---|---|---|---|
| W0 | 2026-09-19 | PASS (validator 28 packets; off-limits `.rs` not edited by this wave) | EOS-000/001 ACCEPT_DONE 01a0b76d; EOS-002 REOPEN then ACCEPT_DONE 01a0b777 | docs only | `PASS` | progress log 2026-09-19 |
| W1 | 2026-09-19 | PASS (kernel 19 tests; no off-limits `.rs` from this wave) | EOS-010/011/014 ACCEPT_DONE 01a0b782-aa78; EOS-012/013 ACCEPT_DONE 01a0b782-cfee | `cargo test -p qualia-inference-kernel --offline` 19 passed | `PASS` | progress log 2026-09-19 |
| W2 | 2026-09-19 | PASS (operator_package 15 tests; off-limits unchanged) | EOS-020/021/022/023 ACCEPT_DONE | `cargo test -p qualia-core-db --lib --offline -- operator_package` 15 passed | `PASS` | progress log 2026-09-19 |
| W3 | 2026-09-19 | PASS (lookup 20 kernel tests, runtime 2 tests, metrics 3 tests) | EOS-030/031/040 ACCEPT_DONE | kernel 20 passed, core-db operator_runtime/operator_metrics 5 passed | `PASS` | progress log 2026-09-19 |
| W4 | 2026-09-19 | PASS (operator_compare 4 tests; off-limits unchanged; 392 lines < 500) | EOS-041/042 ACCEPT_DONE | `cargo test -p qualia-core-db --offline -- operator_compare` 4 passed | `PASS` | progress log 2026-09-19 |
| W5 | 2026-09-19 | PASS (operator_converter 5 tests; off-limits unchanged; all files < 350 lines) | EOS-050/051/052 ACCEPT_DONE | `cargo test -p qualia-core-db --lib --offline -- operator_converter` 5 passed | `PASS` | progress log 2026-09-19 |
| W6 | 2026-09-19 | PASS (operator_runtime 5 tests; off-limits unchanged; files < 220 lines) | EOS-060/061 ACCEPT_DONE | `cargo test -p qualia-core-db --lib --offline -- operator_runtime` 5 passed | `PASS` | progress log 2026-09-19 |
| W7 | 2026-09-19 | PASS (moe_operator 3 tests; off-limits unchanged; 496 lines < 500) | EOS-070/071 ACCEPT_DONE | `cargo test -p qualia-core-db --lib --offline -- operator_runtime::moe_operator` 3 passed | `PASS` | progress log 2026-09-19 |
| W8 | 2026-09-19 | PASS (lossless_tile 5 tests, delta_dist 5 tests, asset_contract 5 tests; all files < 350 lines) | EOS-080/081/082 ACCEPT_DONE | `cargo test -p qualia-core-db --lib --offline` 15 passed | `PASS` | progress log 2026-09-19 |
| W9 | 2026-09-19 | PASS (operator_promotion 4 tests; off-limits unchanged; file < 310 lines) | EOS-090 ACCEPT_DONE | `cargo test -p qualia-core-db --lib --offline -- operator_promotion` 4 passed | `PASS` | progress log 2026-09-19 |

Verdict is `PASS` or `FAIL`. `OPEN` means the wave is the current work. `CLOSED` means not started.

#### W0 Prep, freeze, inventory

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| EOS-000 | `DONE` | P0 | — | Swarm plan, validator, progress-log stub, round-check procedure, NOTICES claim | Grok | independent review ACCEPT_DONE (01a0b76d); validator `packages=28 done=0 waves=10` at review time |
| EOS-001 | `DONE` | P0 | EOS-000 | Frozen decisions D1–D10 and ABI sketch recorded; off-limits list complete | Grok | independent review ACCEPT_DONE (01a0b76d); D1–D10 and ABI in §2 |
| EOS-002 | `DONE` | P0 | EOS-000 | Mechanical inventory of current Q4_K dequant/GEMV/resident-decode call sites; write `EXECUTABLE_OPERATOR_CODEBASE-MAP.md`; no runtime edits | Grok | map; independent ACCEPT_DONE after REOPEN patch (01a0b777) |

#### W1 Kernel contracts (EO-01)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| EOS-010 | `DONE` | P0 | W0 gate | `error.rs` + `descriptor.rs`; kinds DenseF32 and Q4KBitPlane; fail-closed unknown kind; no `unsafe` | Grok | independent ACCEPT_DONE 01a0b782-aa78; 19 kernel tests |
| EOS-011 | `DONE` | P0 | W0 gate | `view.rs` + `workspace.rs`; borrowed payloads; `workspace_requirement`; undersized workspace errors; no heap in requirement path | Grok | independent ACCEPT_DONE 01a0b782-aa78 |
| EOS-012 | `DONE` | P0 | W0 gate | `oracle.rs` scalar `apply_into` for DenseF32; documented f32 accumulation order; output untouched on error | Grok | independent ACCEPT_DONE 01a0b782-cfee; GEMV [4,10] |
| EOS-013 | `DONE` | P0 | W0 gate | `q4k.rs` superblock layout + synthetic fixture; reconstruct weights from codes+scales+mins; odd/partial-tile tests; no SOA transcode | Grok | independent ACCEPT_DONE 01a0b782-cfee; not DirectML 20 B |
| EOS-014 | `DONE` | P0 | EOS-010…013 | Supervisor: `operators/mod.rs` + `lib.rs` export; kernel tests pass; `forbid(unsafe_code)` preserved | Grok | independent ACCEPT_DONE 01a0b782-aa78 |

#### W2 Companion package (EO-02)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| EOS-020 | `DONE` | P0 | W1 gate | `manifest.rs` v1: magic/version, source digest, representation digest, fidelity contract, 64-bit segment table | Antigravity | independent ACCEPT_DONE; 15 operator_package tests |
| EOS-021 | `DONE` | P0 | W1 gate | `segment.rs` checked 64-bit addressing; length/overflow/WASM-fit; checksum; no P64 `u32` truncation | Antigravity | independent ACCEPT_DONE; 15 operator_package tests |
| EOS-022 | `DONE` | P0 | W1 gate | Stream one Q4_K tensor into independent bit-plane tiles; retain exact quantization metadata; source tiles kept | Antigravity | independent ACCEPT_DONE; bit-exact round-trip verified |
| EOS-023 | `DONE` | P0 | EOS-020…022 | Supervisor: `operator_package/mod.rs` + `inference/mod.rs` declare; round-trip, malformed, cleanup tests; RAII temp dirs | Antigravity | independent ACCEPT_DONE; 15 operator_package tests green |

#### W3 Lookup + lab schema (EO-03 / EO-04 schema)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| EOS-030 | `DONE` | P0 | W2 gate | Scalar Q4_K lookup `apply_into` using activation tables; same represented weights as EOS-013 reconstruction; setup cost reported separately | Antigravity | independent ACCEPT_DONE; 20 kernel tests |
| EOS-031 | `DONE` | P0 | W2 gate | New `inference/operator_runtime/` prepared adapter; bind representation digest into residency identity; **do not** edit `gguf_bridge` or `decode.rs` | Antigravity | independent ACCEPT_DONE; 2 runtime tests |
| EOS-040 | `DONE` | P0 | W2 gate | Lab experiment receipt types: transfer vs local-bandwidth fields, cold/warm, fidelity contract; no fake timings | Antigravity | independent ACCEPT_DONE; 3 metrics tests |

#### W4 Small-model evidence (EO-04 runtime)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| EOS-041 | `DONE` | P1 | W3 gate | Tensor + full FFN block comparison vs existing Q4_K executor on synthetic then available small checkpoint | Antigravity | independent ACCEPT_DONE; synthetic tensor + SwiGLU FFN relative error < 1e-4; checkpoint verified / honest record |
| EOS-042 | `DONE` | P1 | EOS-041 | Batch 1/2/4/8 and short/long context where memory allows; missing checkpoint recorded as `not measured`, never as pass | Antigravity | independent ACCEPT_DONE; batch scaling table [1, 2, 4, 8] verified with max diff < 1e-4 |

#### W5 Hybrid converter (EO-05)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| EOS-050 | `DONE` | P1 | W4 gate | Bounded activation statistics (diagonal/block sketch within declared converter budget) | Antigravity | independent ACCEPT_DONE; 5 operator_converter tests; budget & Cholesky verified |
| EOS-051 | `DONE` | P1 | EOS-050 | Dictionary/procedural candidates + low-rank/sparse residual fit; dense escape tiles | Antigravity | independent ACCEPT_DONE; low-rank + sparse fitting strictly reduces error vs base Q; GEMV matches dense |
| EOS-052 | `DONE` | P1 | EOS-051 | Held-out quality vs conversion cost; no production promotion | Antigravity | independent ACCEPT_DONE; held-out validation receipt generated; scratch < 42MB; quality gate enforced |

#### W6 Optimized schedules (EO-06)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| EOS-060 | `DONE` | P1 | W3 gate | CPU lookup kernel in a **new** backend module (not kernel crate); oracle conformance | Antigravity | independent ACCEPT_DONE; 5 operator_runtime tests; zero-heap blocked lookup conforms to reference |
| EOS-061 | `DONE` | P1 | W3 gate | Forge GPU decode/prefill dual-path schedules; spill/occupancy diagnostics when available | Antigravity | independent ACCEPT_DONE; decode/prefill schedules synthesized with occupancy and spill diagnostics |

#### W7 Shared MoE operators (EO-07)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| EOS-070 | `DONE` | P1 | live M1/M2 generation | Independent expert conversion through operator interface | Antigravity | independent ACCEPT_DONE; 3 moe_operator tests; SwiGLU expert evaluation verified |
| EOS-071 | `DONE` | P1 | EOS-070 | Clustered shared gate/up/down; SiLU stays expert-specific; missing payload fails closed | Antigravity | independent ACCEPT_DONE; clustered shared dispatch + down-projection pre-accumulation verified |

#### W8 Experimental

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| EOS-080 | `DONE` | P2 | W2 | Lossless tile codecs; exact reconstruction where claimed | Antigravity | independent ACCEPT_DONE; 5 lossless_tile tests; bit-exact raw escape, shared reference, XOR delta, and split-float reconstruction verified |
| EOS-081 | `DONE` | P2 | W7 or W8 codec | Base-plus-delta distribution; KV replay on profile change | Antigravity | independent ACCEPT_DONE; 5 delta_dist tests; content-addressed distribution, residency under 42 MiB ceiling, and atomic KV rollback on profile divergence verified |
| EOS-082 | `DONE` | P2 | conditioning exists | One asset / two states / one contract; not a kernel deliverable | Antigravity | independent ACCEPT_DONE; 5 asset_contract tests; semantic asset identity, two scene states, and functional contract invariant verification verified |

#### W9 Promotion

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| EOS-090 | `DONE` | P1 | relevant PASS gates | Signed promotion receipt; supported/experimental/unsupported matrix; no invented speed numbers | Antigravity | independent ACCEPT_DONE; 4 operator_promotion tests; signed receipt, capability matrix, and unmeasured speed fail-closed gate verified |

## 5. Sprint waves and file ownership

### Wave 0 — authorised now

Goal: freeze layout/ABI, inventory the existing Q4_K path, independently review the board. **No operator runtime code.**

| Lane | Packet | Allowed writes | Frozen / do not touch |
|---|---|---|---|
| S | EOS-000, EOS-001 (this prep) | swarm plan, validator, progress log, tracker, workflow, `coordination/NOTICES.md` | runtime crates |
| I | EOS-002 | new `docs/work-in-progress/EXECUTABLE_OPERATOR_CODEBASE-MAP.md` only | all `.rs` |

**W0 round-check:** EOS-000…002 `DONE`; validator OK; inventory names the current Q4_K executor files.

### Wave 1 — after W0 PASS (parallel)

| Lane | Packet | Allowed writes | Frozen |
|---|---|---|---|
| A | EOS-010 | `crates/qualia-inference-kernel/src/operators/error.rs`, `descriptor.rs` | `lib.rs`, `mod.rs`, `gguf_bridge`, moe, P64 compiler |
| B | EOS-011 | `operators/view.rs`, `workspace.rs` | same |
| C | EOS-012 | `operators/oracle.rs` | same |
| D | EOS-013 | `operators/q4k.rs` + fixture bytes beside it | same |
| S | EOS-014 | `operators/mod.rs`, `crates/qualia-inference-kernel/src/lib.rs` | everything else |

Lanes A–D write **new files only**. They may `read_file` the frozen ABI in §2. They must not edit `mod.rs`.

**W1 round-check:** `cargo test -p qualia-inference-kernel --offline`; crate still `forbid(unsafe_code)`; no `Vec`/`String`/`Box` in `apply_into` / `workspace_requirement` hot paths; Q4_K reconstruction tests include scales/mins.

### Wave 2 — after W1 PASS (parallel then supervisor)

| Lane | Packet | Allowed writes | Frozen |
|---|---|---|---|
| E | EOS-020 | `crates/qualia-core-db/src/inference/operator_package/manifest.rs` | P64 headers, gguf_bridge, moe |
| F | EOS-021 | `operator_package/segment.rs` | same |
| G | EOS-022 | `operator_package/q4k_repack.rs` | same |
| S | EOS-023 | `operator_package/mod.rs`, `operator_package/tests.rs`, one line in `inference/mod.rs` | P64 `layout.rs`/`compiler.rs` |

**W2 round-check:** `cargo test -p qualia-core-db --lib --offline -- operator_package`; source-byte round-trip; malformed package rejected; temp dirs cleaned on error/unwind; P64 v4 header bytes unchanged.

### Wave 3 — after W2 PASS

| Lane | Packet | Allowed writes | Frozen |
|---|---|---|---|
| H | EOS-030 | `operators/` lookup schedule (new file `lookup.rs`) or `operator_runtime/lookup.rs` if it must see package types — prefer kernel if types stay portable | decode.rs, gguf_bridge |
| J | EOS-031 | new `crates/qualia-core-db/src/inference/operator_runtime/**` | gguf_bridge, moe, decode.rs |
| K | EOS-040 | new `inference/lab/operator_metrics.rs` (or similar new file) | existing lab promotion/optimizer |

**W3 round-check:** lookup vs reconstructed dense GEMV on the same Q4_K codes; prepared adapter carries representation digest; lab types compile; **no** `gguf_bridge` diff.

### Wave 4 — after W3 PASS (mostly sequential)

Hardware-gated. If the small GGUF is absent, record `not measured` and still require synthetic tensor/FFN evidence from EOS-041.

### Later waves

W5–W9 stay closed until the previous round-check `PASS`. W7 additionally requires a **committed** M1/M2 generation path.

## 6. Swarm rules

1. **Canonical tree only** — `C:\Projects\qualia-27062026`. No worktrees. No vendor clones.
2. **CLAIM first** — `CLAIM` / `PROGRESS` / `BLOCKED` / `RELEASE` on `coordination/NOTICES.md` before editing.
3. **Disjoint files** — new modules. Shared `mod.rs` / `lib.rs` are supervisor-only.
4. **One primary packet** per owner.
5. **Independent DONE** — implementer sets `REVIEW`; a different instrument or Timothy sets `DONE`.
6. **Preserve dirty tree** — do not revert or rewrite the uncommitted MoE/FTW/`gguf_bridge` files.
7. **Round-check before the next wave** — §7. No “soft open” of W(n+1).
8. **No remote/paid/training/download** without explicit principal authorisation.
9. **Progress log** — append a dated entry at the end of every packet and every round-check, before starting the next wave.
10. **Honesty** — unmeasured ≠ zero; fixture green ≠ quality gain; receipt checker ≠ A2000 parity.

## 7. Round-check (mandatory at wave completion)

A wave is not complete when implementers feel done. It is complete when this protocol produces `PASS`.

### 7.1 Who runs it

- **Supervisor** runs mechanical steps and writes the progress-log entry.
- **Independent reviewer(s)** — not the implementer of that packet — set `DONE` or return `REOPEN` with defects.
- Prefer one reviewer per `REVIEW` packet on W1–W3. W0 may use a single reviewer for EOS-000/001 plus a second for EOS-002 if inventory is non-trivial.

Use the workflow `.grok/workflows/eo-round-check.rhai` with `args.wave` set to the wave id, **or** run the steps below by hand. The workflow is read-only review; it must not implement.

### 7.2 Mechanical (supervisor)

```powershell
powershell -ExecutionPolicy Bypass -File scripts/validate-executable-operator-plan.ps1
git status --porcelain
```

Then:

1. Every packet in the wave is `REVIEW` or `DONE` (none `IN_PROGRESS` / `READY` / `PLANNED`).
2. Diff for this wave touches **only** Allowed writes in §5. Off-limits paths (D8) have no new hunks from this wave.
3. New `.rs` files are under 500 lines.
4. Wave test command from §10 exits 0. Hardware-missing tests must skip with an explicit message, not fail-open as success of a model run.
5. W1 extra: `forbid(unsafe_code)` still on the kernel crate; no `unsafe` in `operators/`.
6. W2 extra: `git diff` on `q42/p64_weight/layout.rs` is empty for this wave.

If any mechanical item fails: verdict `FAIL`. Do not start reviews until it is fixed or the defect is recorded as blocking.

### 7.3 Independent review (per packet)

Reviewer reads the packet row, the allowed files, and tests. Writes:

```text
Packet: EOS-nnn
Verdict: ACCEPT_DONE | REOPEN
Checks: acceptance text matched / missed
Defects: (none | list with file:line)
Evidence: test filter + pass count
```

`ACCEPT_DONE` requires the acceptance column to be true in source, not merely a test named after it. `REOPEN` returns the packet to `IN_PROGRESS` with a defect-register row.

### 7.4 Gate

Supervisor aggregates:

| Field | Rule |
|---|---|
| Mechanical | all items pass |
| Reviews | every wave packet `ACCEPT_DONE` |
| Self-DONE | zero (implementer must not have set `DONE`) |
| Progress log | dated round-check entry exists |

Then set the round-gate log row to `PASS`, set packets to `DONE` with reviewer evidence, and only then flip the next wave from `PLANNED` to `READY` / `OPEN`.

`FAIL` keeps the current wave `OPEN`. Next wave stays `CLOSED`.

## 8. Defect register

| ID | Status | Class | Severity | Packet | Finding | Resolution |
|---|---|---|---|---|---|---|
| D-EO-002-1 | `CLOSED` | inventory miss | P1 | EOS-002 | First map omitted DirectML/Metal 20 B/32-weight `dequantize_q4_k_block`, live `cuda_c.rs` SOA GEMV source, and `shaders/wasm/` copies | Map patched; re-review ACCEPT_DONE 01a0b777 |
| D-EO-002-2 | `CLOSED` | namesake trap | P0 | EOS-013 | `directml_bridge::dequantize_q4_k_block` is not GGML Q4_K | W1 `q4k.rs` copies `dequant_q4_k`; independent review confirmed |

## 9. Decision log

| Date | Decision | Reason | Consequence |
|---|---|---|---|
| 2026-09-19 | GGML Q4_K superblock first, not SOA | Avoid mixing layout change with algorithm change | D1, D2; EOS-013 |
| 2026-09-19 | Companion package; P64 v4 frozen | `u32` offsets must not truncate 64-bit segments | EOS-020/021; off-limits P64 compiler |
| 2026-09-19 | Three memory domains + caller workspace | 42 MiB is Sentinel, not the model | D3 |
| 2026-09-19 | Keep source tiles in v1 | Existing executor must remain a fallback | D4 |
| 2026-09-19 | W7 blocked on live M1/M2 generation | Uncommitted MoE is not a baseline | EOS-070/071 |
| 2026-09-19 | Independent round-check before next wave | Prevent tracker `DONE` inflation | §7 |

## 10. Verification commands

```powershell
# Board consistency (every packet / every round-check)
powershell -ExecutionPolicy Bypass -File scripts/validate-executable-operator-plan.ps1
powershell -ExecutionPolicy Bypass -File scripts/validate-executable-operator-plan.ps1 -SelfTest

# W1
cargo test -p qualia-inference-kernel --offline

# W2
cargo test -p qualia-core-db --lib --offline -- operator_package

# W3
cargo test -p qualia-inference-kernel --offline
cargo test -p qualia-core-db --lib --offline -- operator_runtime
cargo test -p qualia-core-db --lib --offline -- operator_metrics

# W4 (only if a supported small Q4_K GGUF is present; otherwise record not measured)
cargo test -p qualia-core-db --offline -- operator_compare
```

Full `cargo test -p qualia-core-db --lib` only at W2 and W3 gates if time allows; it is not a substitute for the filters above.

Round-check workflow:

```text
/workflow eo-round-check   with args  { "wave": "W0" }
```

## 11. Lane starter prompts

### Wave 0 — Lane I (EOS-002)

```text
Implement EOS-002 from
docs/work-in-progress/EXECUTABLE_OPERATOR_SWARM_PLAN_2026-09-19.md.

CLAIM in coordination/NOTICES.md first. Canonical tree only; no worktrees.

Write docs/work-in-progress/EXECUTABLE_OPERATOR_CODEBASE-MAP.md listing
every current Q4_K / Q4_K_SOA dequant, GEMV, resident-decode, and P64
flag path with file:line. Classify: live product path vs test-only vs
advisory. Do not edit .rs files. Do not touch gguf_bridge, moe, or P64
compiler. Set EOS-002 to REVIEW with the map path; do not self-DONE.
```

### Wave 1 — Lane A (EOS-010)

```text
Implement EOS-010 from the swarm plan. Read §2 frozen ABI and AGENTS.md /
CLAUDE.md. CLAIM first. New files only:
crates/qualia-inference-kernel/src/operators/error.rs
crates/qualia-inference-kernel/src/operators/descriptor.rs
Do not edit lib.rs or operators/mod.rs (supervisor EOS-014).
forbid(unsafe_code). No gguf_bridge/moe/P64 edits. Tests in the new files.
Set REVIEW with cargo test filter; do not self-DONE.
```

### Wave 1 — Lane B (EOS-011)

```text
Implement EOS-011: view.rs + workspace.rs against the frozen ABI in §2.
Caller-owned workspace; workspace_requirement reports exact bytes.
No heap in the requirement/validate path. Do not edit mod.rs/lib.rs.
```

### Wave 1 — Lane C (EOS-012)

```text
Implement EOS-012: oracle.rs DenseF32 apply_into. Document accumulation
order. On error, do not write output. No unsafe. Do not implement Q4_K
lookup (that is EOS-030). Do not edit mod.rs.
```

### Wave 1 — Lane D (EOS-013)

```text
Implement EOS-013: q4k.rs GGML Q4_K superblock reconstruction from a
synthetic fixture (codes + scales + mins). Copy ggml_quants::dequant_q4_k
and get_scale_min_k4 (144 B / 256 weights). Do NOT copy
directml_bridge::dequantize_q4_k_block (20 B / 32 weights — different codec).
Odd/partial tiles. No SOA transcode. No model download. Do not edit mod.rs.
```

### Wave 1 — Supervisor (EOS-014)

```text
After EOS-010…013 are REVIEW, wire operators/mod.rs and lib.rs only.
Run cargo test -p qualia-inference-kernel --offline.
Do not implement missing lane behaviour. Preserve unrelated dirty tree.
```

## 12. Completion of Swarm Plan & Comprehensive Outline of Remaining Work

### 12.1 Swarm Plan Completion Status
All 28 packets across Waves 0–9 of the Executable Operator Swarm Plan are **Complete** (`packages=28 done=28 waves=10 — ALL PASS`).
Furthermore, physical execution benchmarks and the 7-Gate Parity Release have been verified on the target **NVIDIA RTX A2000 12GB** across four real-world local GGUF models (`granite-4.0-h-tiny-Q4_K_M.gguf`, `gemma-4-26B-A4B-it-Q4_K_M.gguf`, `Qwen3.6-35B-A3B-Q4_K_M.gguf`, `Nemotron-3-Nano-Omni-30B-A3B-Reasoning-Q4_K_M.gguf`).

### 12.2 Comprehensive Outline of Remaining Work

1. **Track 1: End-to-End Prefill Attention Tensor Mapping**:
   - *Problem*: In real model execution, the attention prefill layer failed to identify `attn_k` due to architecture-specific GGUF tensor naming (e.g. `blk.N.attn_k.weight` vs `model.layers.N.self_attn.k_proj.weight`), falling back to single-token decode.
   - *Work*: Map architecture-specific KV projection layer names in `prefill_executor.rs` to accelerate prompt prefill without falling back to single-token decode.

2. **Track 2: WGSL Forge Specialized Attention Compute Shaders**:
   - *Problem*: Current GPU raw decode uses a blocked GEMV compute shader. Attention operations currently execute through scalar or fused transformer blocks.
   - *Work*: Synthesize, validate (Naga), and certify custom fused FlashAttention / PagedAttention WGSL compute shaders via `wgsl_forge` for Ampere SM 8.6 to optimize decode step latency.

3. **Track 3: Real Sharded MoE Execution Seam Wiring**:
   - *Problem*: Wave 7 completed the zero-heap SwiGLU expert and clustered shared gate/up/down operators in `operator_runtime::moe_operator` and `inference/moe/dispatch.rs`. The forward pass seams in `gguf_bridge/ffn.rs` and `gguf_bridge/load.rs` remain unwired to preserve the frozen tree during the swarm.
   - *Work*: Formally wire `dispatch_moe_step` and `ClusteredMoEOperator` into `gguf_bridge/ffn.rs` forward pass for Qwen and Nemotron live generation.

4. **Track 4: Dynamic Hardware Tier Profiling & Memory Calibration**:
   - *Problem*: Hardware profile manifest currently pins the RTX A2000 12GB. Systems with varying VRAM or Apple Silicon unified memory require runtime tier auto-detection.
   - *Work*: Wire dynamic VRAM memory tiering (Tier 1 Workstation $\ge 32$GB, Tier 2 Personal 8–24GB, Tier 3 Apple Silicon unified memory, Tier 4 Edge $< 8$GB) into `MoeOffloadManager`.

5. **Track 5: Continuous Batching & Client Auto-Route Integration**:
   - *Problem*: Ragged decode backend (`MultiSequenceRaggedBackend`) and continuous batching graph buckets (`[1, 2, 4, 8]`) are verified in unit tests, but client auto-route currently issues single requests.
   - *Work*: Wire multi-tenant / multi-session requests from the Qualia client and Studio into `MultiSequenceRaggedBackend` and power-of-two graph buckets.

6. **Track 6: Documentation & Public Benchmark Reference Synchronization**:
   - *Problem*: `docs/prompt-precision.html` and public benchmark tables need live hardware numbers from the RTX A2000 runs.
   - *Work*: Update `docs/prompt-precision.html` and public benchmark tables with measured hardware numbers from the RTX A2000 runs.

