# WASM LLM + Anatomy regression handoff (0.0.29-dev)

**Date:** 2026-08-02  
**Branch:** `0.0.29-dev` (cut from `0.0.28` @ `59b249c8`)  
**Purpose:** Unblock the other machine to restore desktop WASM LLM throughput and verify anatomy.  
**Author of this note:** audit of tree history only (this machine has no models, no Node, no usable Python for live re-measure).

---

## Measured harm (as reported by principal)

| Path | Before (reported / changelog) | After (reported) |
|------|-------------------------------|------------------|
| Desktop browser WASM LLM | Competitive with **wllama**; CHANGELOG Phase 5 ~**5.9 tok/s** (SmolLM2-360M class) | **~0.54 tok/s** (~**24 tok/s** slower than competitive desktop run the principal measured) |
| Mobile LLM “fix” | Intended: unstick phone init | Did not fix phones; damaged desktop path |
| Anatomy | Working packs + render | Mobile memory path changes; needs re-verify on real packs |

CHANGELOG’s own arc is explicit: Phase 5 lifted browser decode **from ~0.6 tok/s → ~5.9 tok/s** via resident weights + single-submit forward. **0.54 is the pre–Phase-5 floor**, not a mild regression. That is the diagnostic fingerprint.

**Intention:** The mobile series reads as a failed optimisation (defer residency to unstick UI), not as a deliberate sabotage plan. Outcome is still a product regression: desktop tok/s was burned by that experiment and the restore was **incomplete in shipping artifacts**. Cost sticks on the record; apology does not fix it — rebuild + re-bench does.

---

## Commit timeline (origin/0.0.28)

| When | Commit | What |
|------|--------|------|
| 2026-07-29 | `b073d6ff` | Resident WebGPU inference pipeline checkpoint (competitive era) |
| 2026-07-29 | `c509e178` | WebGPU benchmarks exact / cache-safe |
| 2026-07-29 | `ec097139` | 0.0.28 release prep (wasm binary rewrite) |
| 2026-07-31 14:59 | `937213e8` | **Last commit that updated** `docs/playground/qualia_core_db_bg.wasm` (hash `a02f5ecf…`, 3 709 248 B) |
| 2026-07-31 19:18 | `1ea63995` | Mobile anatomy/LLM: adapter limits, compile yield, demo timeouts, anatomy phone path |
| 2026-08-01 18:27 | `1b378c5f` | Anatomy: Rust `.hmc` decode, OPFS, DPR cap, mute heavy systems; LLM demo heartbeats/timeouts |
| 2026-08-01 19:00 | `2b05fb32` | **`fix(wasm-llm): unstick mobile WebGPU init with yields and deferred weights`** |
| 2026-08-01 20:21 | `c2b5bd12` | **`fix(wasm-llm): restore full eager weight upload (undo deferred path garbage)`** — source-only undo |
| later | studio scroll fixes | Unrelated to LLM kernels |

### Smoking gun: deferred residency

`2b05fb32` deferred eager GPU weight upload on mobile so init could finish and first token would upload lazily. That is exactly how you fall off the Phase-5 path:

- `mc8_weights_resident == false` → per-forward / lazy re-upload  
- CHANGELOG: resident weights removed **~50 MB/token** re-upload of logits alone  
- Expected tok/s class: **~0.5–0.6**, matching the principal’s **0.54**

`c2b5bd12` undid deferred upload in **Rust source** and restored single-arg `initialize_webgpu_engine` + full eager adopt. Commit message admits: *“Deferred and yielding weight upload broke coherent decode on desktop and did not fix mobile.”*

### Critical shipping gap (still open)

**No WASM binary was rebuilt or committed after `2b05fb32` / `c2b5bd12`.**

```
git log -- docs/playground/qualia_core_db_bg.wasm
# tip: 937213e8 (2026-07-31) — still HEAD content
```

Demos cache-bust as if a restore shipped:

- `docs/online-llm-demo.html` → `?v=0.0.28-llm-restore`
- `docs/wasm-llm-test.html` → `?v=0.0.28-q4fix3`

…but `docs/playground/qualia_core_db_bg.wasm` is still the **31 Jul** artifact. **Source and shipped browser engine are desynced.** Any machine testing GitHub Pages / this tree’s `docs/` is **not** running the post-`c2b5bd12` Rust.

---

## What remains in source after the “undo” (must re-verify on rebuild)

These are **still in tree** and will enter the next `wasm-pack` build:

1. **`gguf_bridge/wasm_yield.rs`** — `setTimeout(0)` yields between init phases; status polling via `getWebgpuInitStatus`. Init-only if used only at boot; must not leak into per-token decode.
2. **`initialize_webgpu_engine`** — still yields before/around copy + adopt; comments say upload itself is one blocking stretch (correct for residency).
3. **Adapter path** (`init.rs`): `PowerPreference::HighPerformance`, raised `max_buffer_size` / `max_storage_buffer_binding_size` from adapter limits, labeled device, mobile-oriented error text.
4. **WASM-only gating:** `poll_wait` native-only; mock pipeline / top-k buffers native-only; several diagnostics behind `wasm-llm-diagnostics`.
5. **`load.rs`:** eager `mc8_upload_all_resident_weights` + logits + norms; if upload returns false → *“will retry lazily”* (silent demotion to ~0.5 tok/s class). **First log line to check on the other machine.**
6. **Uniform arenas** (`mod.rs`): `base_slot == 0` forces full `upload()` — intentional correctness fix; check it does not double-upload hot path.
7. **JS demos:** longer mobile timeouts, OPFS guidance, Q4 presets withheld (Q8 preferred), `inferWasmAsyncMeasured` for exact token budgets.

None of (1–4) should alone explain a permanent **10×** drop **if** residency is true. The drop fingerprint is **non-resident or multi-submit / per-token upload** behaviour.

---

## Anatomy pipeline (what changed / risk)

Files: `docs/playground/anatomy.js`, `anatomy.html`, `render/portal` (earlier), `1b378c5f` / `1ea63995`.

| Change | Intent | Risk on desktop |
|--------|--------|-----------------|
| Phone lite: mute extra organ systems; keep circulatory/respiratory/nervous/skeletal | RAM on Pixel-class | **False phone detection** (`Mobile` UA / coarse pointer / tablet) can mute systems on desktop-like devices |
| `anatomyMaxDpr()` 1.25 phone / 3 desktop | Avoid WebGPU surface OOM | Low on true desktop |
| OPFS pack cache (`anatomy-v{ENGINE_VERSION}-{key}.hmc`) | Second-visit speed | Stale packs if ENGINE_VERSION not bumped when codec changes |
| Drop GitHub Releases CORS fetch fallback | Releases never worked cross-origin | Correct; packs must be same-origin / local file |
| Decode `.hmc` in Rust (no per-organ JS copies) | Memory | Good if WASM portal export matches; bad if export missing → silent fallback |
| Debounced mixer re-decode | Avoid re-decode on mobile chrome resize | Can delay updates; should not zero desktop quality |
| Skip 0×0 canvas resize | Mobile layout settle | Good; forces 360×640 last-resort on phone |

**Anatomy verify checklist (other machine):**

1. Male/Female CCF packs load from OPFS after one fetch.  
2. Complete pack (~700 MB) loads on desktop without phone lite mute.  
3. All organ systems toggleable; no permanent mute on desktop Chrome.  
4. WebGPU surface non-blank after layout; no tab kill at full DPR.  
5. Portal WASM `docs/pkg/qualia/qualia_bg.wasm` vs playground anatomy path — confirm which bundle anatomy actually imports.

---

## Fix plan for the other machine (ordered)

### A. Prove the throughput floor vs Phase-5 path

1. Serve with COOP/COEP (`docs/serve.py` or equivalent).  
2. Open `docs/wasm-bench-compare.html` (Qualia vs wllama) and `docs/wasm-llm-test.html`.  
3. Model: `smollm2-360m-instruct-q8_0.gguf` (verified path; Q4 still marked experimental/withheld).  
4. Record: load ms, first-token ms, decode tok/s, text coherence, adapter name.  
5. Console: search for `[MC8] eager resident weight upload skipped` / `mc8_weights_resident` / lazy retry.  
6. If residency failed → fix upload failure (buffer limits, OOM, quant layout), not more UI yields.

### B. Resync shipped WASM with post-restore source

```powershell
# From DEVELOPMENT.md — full playground superset
$env:RUSTFLAGS = "-C target-feature=+simd128 -C link-arg=-zstack-size=8388608 -C link-arg=--max-memory=4294967296"
wasm-pack build crates/qualia-core-db --target web --out-dir pkg-playground --release -- `
  --no-default-features --features portal,wasm-llm,wasm-logic,wasm-scientific,wasm-playground
# copy qualia_core_db.{js,d.ts} + _bg.wasm into docs/playground/ (and llmdemo if still separate)
```

Also rebuild portal package if anatomy uses `docs/pkg/qualia/`.

Bump cache-bust query strings **only after** binary is committed.

### C. Do **not** reintroduce deferred residency for mobile

Mobile must not sacrifice `mc8_weights_resident` for UI paints. Allowed:

- Yields **between** init phases only  
- Progress via `getWebgpuInitStatus`  
- Smaller models on phone (already: SmolLM2-360M policy)  
- Optional: **Web Worker** / OffscreenCanvas later — not lazy weight residency

### D. Regression gates

```bash
node agent-tools/wasm-mc2-test.mjs
WASM_MODEL=models/smollm2-360m-instruct-q8_0.gguf node agent-tools/wasm-mc2-test.mjs
node agent-tools/llmdemo-test.mjs
```

Acceptance: desktop decode back to **wllama-competitive class** (principal’s ~25 tok/s class if that was the previous desktop measure; CHANGELOG historical ~5.9 on another GPU — **label GPU model in every CSV row**). Never claim win without a new measured row.

### E. Git bisect anchors

- **Known-good era:** `b073d6ff` / `c509e178` resident pipeline + pre-mobile demos  
- **Suspect:** `2b05fb32` deferred weights  
- **Incomplete fix:** `c2b5bd12` source-only  
- **Artifact lag:** wasm @ `937213e8` while source moved past mobile experiments  

---

## This machine (2026-08-02) — environment facts

| Item | Status |
|------|--------|
| Workspace | `C:\github\qualiaDB` on `0.0.28` (= origin) |
| Models (`C:\LLM_Models`, `docs/models`) | **Absent** |
| Node / wasm-pack | **Not on PATH** |
| Python | Store stub only — `docs/serve.py` not runnable as-is |
| GPU | Intel UHD 620 only — not a valid A2000/desktop competitive GPU |
| Live re-bench | **Not possible here** — handoff is static audit |

---

## Branch intent

`0.0.29-dev` carries:

1. Full `0.0.28` tip (including failed mobile series + source restore).  
2. This handoff document.  
3. No false claim that tok/s is already fixed — **fix lands on the machine with models + WebGPU desktop + wasm-pack**.

When fixed: rebuild WASM artifacts, re-run benches, append measured numbers here, then promote.
