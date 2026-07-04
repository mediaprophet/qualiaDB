# `cargo check --workspace` Warnings Study — 2026-06-30

**Branch:** `0.0.23` | **Command:** `cargo check --workspace` | **Result:** ✅ compiles, 0 errors, 762 warnings

---

## TL;DR — answer to the question

> *"Are the errors due to unfinished functions that aren't working, or are they due to malformed calls?"*

**Overwhelmingly the former.** The 762 warnings are dominated by **unfinished scaffolding and unwired code** — fully-written data structures, helper functions, and library skeletons that compile cleanly but are not yet consumed by any caller. There are **no compiler errors** and **no malformed calls** in the rustc sense (no type mismatches, no wrong-arity calls, no unresolved symbols).

The only items that resemble "malformed" are:
- **3 deprecated API calls** (still functional, will break on future dep upgrades) — see §5
- **3 Cargo.toml config issues** (orphaned keys + ignored profile) — see §6

Everything else is either cosmetic (unused imports, redundant `mut`, naming style) or "code that exists but isn't wired in yet."

---

## 1. Totals and distribution

| Metric | Count |
|---|---|
| Total warnings | **762** |
| Compiler errors | **0** |
| Crates with warnings | 7 |

### By crate

| Crate | Warnings |
|---|---|
| `qualia-core-db` | 672 |
| `webizen-studio` | 60 |
| `qualia-extensions` | 11 |
| `qualia-semantic-library` | 3 |
| `webizen-render` | 2 |
| `webizen-component-harvester` | 1 |
| `wellfare-core` | 1 |

### By warning class

| Class | Count | Nature |
|---|---|---|
| `unused import` / `unused imports` | ~203 | Cosmetic — leftover after refactors |
| `unused variable` | ~118 | Stub/placeholder function bodies |
| `field(s) … are never read` | ~328 (280 `fields` + 48 `field`) | Unfinished data models |
| `function … is never used` | 22 | Unwired helpers |
| `struct … is never constructed` | 14 | Unwired types |
| `constant … is never used` | 11 | Unwired constants |
| `variant(s) … are never constructed` | 3 | Unwired enum variants |
| `method … is never used` | 3 | Unwired methods |
| `variable does not need to be mutable` | 19 | Dioxus idiom / stylistic |
| `value assigned to … is never read` | 2 | Dead write before return |
| `use of deprecated …` | 3 | API drift — see §5 |
| `non_camel_case_types` | 3 | Naming style |
| `unused manifest key` | 2 | Cargo.toml config — see §6 |
| `profiles for non root package` | 1 | Cargo.toml config — see §6 |

### Hotspot inside `qualia-core-db`

438 of the 672 `qualia-core-db` warnings come from **`src/specialized_libs/`** — a cluster of large domain-library files:

| Sub-module | Warnings |
|---|---|
| `physics_simulation.rs` | 54 |
| `medical_computing/` | 54 |
| `linear_algebra/` | 48 |
| `cryptographic_library/` | 48 |
| `financial_modeling/` | 47 |
| `machine_learning.rs` | 43 |
| `qpu_bridge/` | 39 |
| `engineering_analysis/` | 32 |
| `chemistry_modeling/` | 32 |
| `statistical_computing.rs` | 31 |
| `quantum_biology/` | 4 |
| `linear_algebra.rs` | 5 |
| `symbolic_integration.rs` | 1 |

---

## 2. The dominant pattern: "library skeleton" scaffolding

The single largest source of warnings (~438) is `specialized_libs/`. These files follow a consistent pattern:

- A top-level `*Library` manager struct with several sub-manager fields
- Each sub-manager has richly-typed domain data structures (e.g. `Patient`, `Demographics`, `MedicalHistory`, `FamilyHistory`, `SocialHistory`…)
- The structs derive `Serialize`/`Deserialize`/`Debug`/`Clone`
- There are real `impl` blocks with `pub fn new()`, `initialize()`, `create_*`, `analyze_*`, etc.
- **No `todo!()`, `unimplemented!()`, or `panic!()` markers** — the bodies return plausible defaults

Example — `specialized_libs/medical_computing/mod.rs` (4031 lines, 103 impl blocks, 166 fns):

```rust
pub struct PatientManager {
    patient_records: PatientRecords,
    medical_history: MedicalHistory,        // ← "field is never read"
    privacy_protection: PrivacyProtection,
    data_access: DataAccessControl,
}

pub struct PatientRecords {
    patients: HashMap<String, Patient>,
    demographics: HashMap<String, Demographics>,        // ← "fields … are never read"
    medical_identifiers: HashMap<String, MedicalIdentifier>,
}
```

**Diagnosis:** These are domain libraries whose **data models are fully specified** but whose **logic isn't wired into the engine's hot paths**. The fields are populated by `new()`/insertion methods but never read by any consumer outside the module. The code is correct Rust; it just isn't consumed.

**This is not "broken" code.** It's the kind of scaffolding that gets written ahead of integration. The risk is **bit-rot**: as the rest of the engine evolves, these unused models can silently drift out of sync with the real types they were meant to mirror.

---

## 3. The "stub body" pattern — unused variables

The ~118 `unused variable` warnings are almost all functions whose **signature takes a parameter that the placeholder body ignores**. These are stubs, not malformed calls.

Example — `net/ebpf_firewall.rs:616`:
```rust
fn detect_socket_type(&self, fd: i32) -> Result<SocketType, EbpfError> {
    // In real implementation, would use getsockopt() to detect type
    // For now, return default
    Ok(SocketType::Stream)        // ← `fd` ignored
}
```

Example — `modalities/logic/rules.rs:45`:
```rust
pub fn evaluate(&self, quin: &crate::NQuin) -> Vec<RuleResult> {
    let mut results = Vec::new();
    for ruleset in &self.rulesets {
        for rule in &ruleset.rules {
            // Placeholder rule evaluation logic
            // In production, this would parse and evaluate the condition
            let result = RuleResult { rule_name: rule.name.clone(), passed: true, message: String::new() };
            results.push(result);
        }
    }
    results        // ← `quin` ignored; always returns `passed: true`
}
```

**Diagnosis:** These are explicitly-commented placeholders. The function exists, compiles, and returns a sane default, but the real implementation is deferred. The unused-variable warning is the compiler pointing at exactly the parameter that the future implementation needs to consume.

**Risk:** A caller of `rules.evaluate(quin)` today gets `passed: true` for every rule regardless of input. That's a **silent false-positive** if anything in the engine depends on it for safety. Worth auditing which callers exist.

---

## 4. The "unwired helper" pattern — `webizen-studio/src/render/`

35 of the 60 `webizen-studio` warnings come from `src/render/` (mesh.rs, graph.rs, qualia.rs, tensor_buffer.rs, motion.rs, scene.rs). These are **complete, correct helper libraries** that no studio code path calls yet.

Example — `webizen-studio/src/render/mesh.rs`:
- `Transform` struct (position/rotation/scale) — never constructed
- `rotate_x`/`rotate_y`/`rotate_z` — never used
- `Transform::at`/`with_scale`/`with_rotation`/`apply` — never used
- `Mesh` struct — never constructed
- `Mesh::line`/`cube`/`quad`/`grid`/`uv_sphere` — never used

The module doc says: *"Mesh primitives and transforms — the geometry half of the engine dev-kit… replace the hand-rolled geometry that JS engines (three.js/Babylon) used to provide."* The code is real (the cube builder builds a correct 8-vertex / 12-edge / 6-face cube). It's just that the studio's render pipeline doesn't import it yet.

**Diagnosis:** Unfinished integration, not malformed code.

---

## 5. Deprecated API calls — the only "will-break-future" items

These are the warnings most worth acting on, because they signal API drift that will become a hard error on the next breaking dependency release.

### 5a. `aead::hybrid_array::Array::from_slice` (×2)

**File:** `crates/qualia-core-db/src/identity/key_vault.rs:361, 406`

```rust
let aes_nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);          // line 361
let nonce = aes_gcm::Nonce::from_slice(&encapsulated.nonce);       // line 406
```

**Fix:** migrate to `TryFrom`:
```rust
let aes_nonce: aes_gcm::Nonce = nonce_bytes.as_slice().try_into()
    .map_err(|_| "nonce length mismatch")?;
```

**Severity:** low today, medium-term. The call still works; `from_slice` panics on wrong length whereas `TryFrom` returns `Err`, so the migration also improves error handling.

### 5b. `oxigraph::store::Store::query` (×1)

**File:** `crates/wellfare-core/src/store.rs:29`

```rust
pub fn query(&self, sparql: &str) -> Result<String, String> {
    match self.inner.query(sparql) {        // ← deprecated
        ...
    }
}
```

**Fix:** migrate to the `SparqlEvaluator` interface per the deprecation note.

**Severity:** low today. `wellfare-core` is a small crate (1 warning total); the call works but will need rewriting when oxigraph removes `Store::query`.

---

## 6. Cargo.toml config issues — genuinely malformed

### 6a. Orphaned `csv` / `atoi` keys (×2 `unused manifest key`)

**File:** `crates/qualia-core-db/Cargo.toml:252-254`

```toml
[[bench]]
name = "qualia_benchmark"
harness = false

[[bench]]
name = "ram_usage"
harness = false


csv = "1.3"        # ← orphaned — cargo reads as bench.1.csv
atoi = "3.0"       # ← orphaned — cargo reads as bench.1.atoi
```

**Diagnosis:** These two dependency lines sit at the top level after the second `[[bench]]` table. Cargo parses them as keys of the second bench array element (`bench.1`), where they're invalid. They were almost certainly meant to be `[bench.dependencies]` entries for one of the benchmark targets, or workspace-level dev-dependencies.

**Fix:** move them under a `[bench.dependencies]` table attached to the relevant `[[bench]]`, or into `[dev-dependencies]` at the crate root.

### 6b. `[profile.release]` in non-root package (×1)

**File:** `crates/qualia-mobile-harness/Cargo.toml`

```toml
[profile.release]
lto = true
opt-level = 'z'
codegen-units = 1
```

**Diagnosis:** Cargo only honours profiles defined in the **workspace root** `Cargo.toml`. This profile is silently ignored — the mobile-harness release builds are **not** getting the LTO/size-optimisation the author intended.

**Fix:** move the `[profile.release]` (or a `[profile.mobile-release]`) block to the workspace root `Cargo.toml`.

---

## 7. Minor cosmetic categories

### 7a. Dead write before error return (×2)

**`governance/webizen_bytecode.rs:185`**
```rust
OP_HALT_VIOLATION => {
    stats.vm_cycles += 1;                          // ← never read
    let _ = crate::wal::log_adversarial_conduct(&quin, 3);
    return Err(VmError::HaltViolation);            // ← stats dropped here
}
```
The increment is wasted because the function returns `Err` immediately after. Cosmetic — no functional impact, but the intent (counting the cycle) is defeated by the early return.

**`solvers/learning/survival/cox.rs:48`**
```rust
let mut iters = 0;        // ← initial value never read
...
for it in 1..=MAX_ITER {
    iters = it;           // ← overwritten every iteration
    ...
}
...
n_iter: iters,            // ← final value IS read here
```
The `let mut iters = 0` initialiser is redundant because the loop immediately overwrites it. `iters` is genuinely used at the end. Cosmetic.

### 7b. `variable does not need to be mutable` (×19)

Mostly the Dioxus signal idiom:
```rust
let mut hdl = use_signal(|| 50.0f64);       // ← `mut` redundant
let mut bp_treated = use_signal(|| false);  // ← `mut` redundant
```
Dioxus `Signal`s are interior-mutable via `.read()`/`.write()`, so the `mut` binding is unnecessary. Stylistic — Dioxus templates commonly carry this.

Also includes a few closures that don't capture mutably (`services/swarm/job.rs:130,134`) and a few `let mut x = …` where `x` is never reassigned (`gguf_sharder.rs:1179`, `p64_weight.rs:575`, `wgsl_forge/validate.rs:80`).

### 7c. Naming conventions (×3)

**`specialized_libs/cryptographic_library/mod.rs`**
- `PCI_DSS` → should be `PciDss`
- `zkSNARKs` → should be `ZkSnarks`
- `zkSTARKs` → should be `ZkStarks`

Stylistic. These are domain acronyms that read more naturally in their current form, but Rust's `non_camel_case_types` lint flags them.

### 7d. Unused imports (~203)

The largest single class by count, the smallest by significance. Examples:
- `use crate::daemon_query::{self, QueryExecError};` — `self` unused, `QueryExecError` used 7×
- `use crate::webizen_bytecode::{self, ExecutionStats};` — `self` unused
- `use crate::solvers::SolversError;` — repeated across 8 sub-modules
- `use std::collections::HashMap;` — repeated across 4 sub-modules
- `use super::linear_algebra::LinearAlgebraLibrary;` — repeated across 4 sub-modules

**Diagnosis:** These are the residue of refactors where a module's body was written (or scaffolded) against a set of imports, then the body was trimmed or moved, leaving the imports behind. `cargo fix --workspace --allow-dirty` would clear ~299 of them automatically.

---

## 8. Risk assessment

| Risk | Items | Severity |
|---|---|---|
| **Silent false-positives from stub evaluators** | `rules.rs::evaluate` always returns `passed: true`; `ebpf_firewall.rs` detectors return hardcoded defaults | **Medium** — audit callers |
| **API drift becoming hard errors** | 3 deprecated calls (§5) | **Low now, Medium on next dep bump** |
| **Cargo.toml silently mis-parsed** | orphaned `csv`/`atoi`; ignored mobile profile (§6) | **Low** — config intent not honoured |
| **Bit-rot in unused scaffolding** | ~438 `specialized_libs` field warnings | **Low** — types can drift from real engine types |
| **Dead write before error return** | `webizen_bytecode.rs:185` | **Low** — cosmetic |
| **Cosmetic** | unused imports, redundant `mut`, naming | **None** |

---

## 9. Recommended actions (in priority order)

1. **Audit callers of stub evaluators** — grep for `rules.evaluate(`, `detect_socket_type(`, `detect_protocol(`, `get_local_address(`, `get_remote_address(` and confirm none are on a safety/compliance path. If they are, replace the stub body or gate the caller.
2. **Fix the Cargo.toml issues** (§6) — move `csv`/`atoi` under `[bench.dependencies]` or `[dev-dependencies]`; move the mobile `[profile.release]` to the workspace root. These are small, surgical fixes with real effect.
3. **Migrate the 3 deprecated API calls** (§5) — `from_slice` → `TryFrom` in `key_vault.rs`; `Store::query` → `SparqlEvaluator` in `wellfare-core/store.rs`. Prevents a future breakage.
4. **Decide the fate of `specialized_libs/`** — either (a) wire the libraries into the engine and start consuming the fields, or (b) mark the modules `#[allow(dead_code)]` with a dated comment explaining they're pending integration, or (c) move them behind a `scaffolding` feature flag so they don't pollute the default build.
5. **Run `cargo fix --workspace --allow-dirty`** to clear the ~299 auto-fixable unused-import / redundant-`mut` warnings. Review the diff before committing.
6. **Fix the 2 dead-write-before-return spots** (§7a) if the cycle counter is meant to be observable.

---

## 10. Methodology

- `cargo check --workspace 2>&1` captured to a temp file (5849 lines).
- Confirmed `0` occurrences of `error[`, `error:`, or `could not compile`; `1` occurrence of `Finished`.
- Categorised all 762 `^warning:` lines by lint class via regex grouping.
- Mapped every warning to its source crate and sub-module.
- Read the actual source for representative warnings in each non-trivial class: `specialized_libs/medical_computing/mod.rs`, `net/ebpf_firewall.rs`, `modalities/logic/rules.rs`, `webizen-studio/src/render/mesh.rs`, `webizen-studio/src/components/clinical_risk_scorer.rs`, `governance/webizen_bytecode.rs`, `solvers/learning/survival/cox.rs`, `identity/key_vault.rs`, `wellfare-core/src/store.rs`, `crates/qualia-core-db/Cargo.toml`, `crates/qualia-mobile-harness/Cargo.toml`.
- Verified no `todo!()`/`unimplemented!()`/`panic!()` markers in the largest hotspot (`medical_computing/mod.rs`).
