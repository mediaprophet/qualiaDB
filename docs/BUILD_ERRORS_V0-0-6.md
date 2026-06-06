# Build Errors & Known Issues — v0.0.6

Recorded 2026-06-06 against CI run
[27057957273](https://github.com/mediaprophet/qualiaDB/actions/runs/27057957273)
(tag `v0.0.6`).

---

## CI Status Summary

| Job | Platform | Status | Notes |
|---|---|---|---|
| CLI | ubuntu-22.04 | ✅ | `qualia-cli-linux-x86_64` uploaded |
| CLI | windows-latest | ✅ | `qualia-cli-windows-x86_64.exe` uploaded |
| CLI | macos-14 | ✅ | `qualia-cli-macos-aarch64` uploaded |
| WASM | ubuntu-22.04 | ✅ | `qualia-core-wasm.tar.gz` uploaded |
| Flutter | windows-latest | ✅ | `qualia-flutter-windows-x86_64.zip` uploaded |
| Flutter | ubuntu-22.04 | ❌ | Linux platform not scaffolded |
| Flutter | macos-14 | ❌ | macOS platform not scaffolded |

---

## Error 1 — Flutter Linux: platform not scaffolded

**Symptom:**
```
No Linux desktop project configured.
See https://flutter.dev/to/add-desktop-support
```

**Cause:** The `crates/qualia-flutter/linux/` directory does not exist. Flutter
requires per-platform boilerplate to be generated before it can target that OS.

**Fix:** Run once locally, then commit the generated directory:
```bash
cd crates/qualia-flutter
flutter create --platforms=linux .
git add linux/
git commit -m "feat(flutter): scaffold Linux desktop platform"
```

---

## Error 2 — Flutter macOS: platform not scaffolded

**Symptom:**
```
No macOS desktop project configured.
See https://flutter.dev/to/add-desktop-support
```
Then, even with `continue-on-error: true` on the build step, the packaging step
fails because the build output directory was never created:
```
tar: could not chdir to 'crates/qualia-flutter/build/macos/Build/Products'
```

**Cause:** Same as Linux — the `crates/qualia-flutter/macos/` directory does not
exist.

**Fix:** Run once locally on a Mac (or in CI with Xcode available):
```bash
cd crates/qualia-flutter
flutter create --platforms=macos .
git add macos/
git commit -m "feat(flutter): scaffold macOS desktop platform"
```

**Note on architecture:** `macos-14` on GitHub Actions is Apple Silicon (ARM64 /
M-series). This is correct — no Intel (x86_64) macOS build is needed or intended.
The asset name `qualia-flutter-macos-aarch64.tar.gz` is accurate.

---

## Error 3 — DirectML: SDK not linked

**Symptom (compile-time warning, non-fatal):**
```
warning: qualia-core-db@0.0.5: Qualia-DB Compiling for Windows:
DirectML Linking Configured (Awaiting SDK).
```

**Cause:** The `build.rs` in `qualia-core-db` detects Windows and emits a warning
that DirectML is intended but the SDK is not yet wired. The build succeeds but GPU
acceleration via DirectML is a no-op.

**What DirectML is:** Microsoft's GPU inference API for Windows. Required for
hardware-accelerated LLM inference on Windows via the local `LocalLlmAgent`
backend.

**Two fix paths (from research):**

### Option A — Direct `windows-rs` bindings
1. Download the [DirectML NuGet package](https://www.nuget.org/packages/Microsoft.AI.DirectML)
   (rename `.nupkg` → `.zip`, extract).
2. Locate `bin/x64/DirectML.dll` and `lib/x64/DirectML.lib`.
3. Update `crates/qualia-core-db/build.rs`:
   ```rust
   fn main() {
       if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
           println!("cargo:rustc-link-search=native=C:\\Path\\To\\DirectML_SDK\\lib\\x64");
           println!("cargo:rustc-link-lib=DirectML");
       }
   }
   ```
4. Ship `DirectML.dll` alongside `qualia-cli.exe` in the release artifact.

### Option B — Use the `ort` crate (ONNX Runtime, easier)
If the goal is hardware-accelerated inference rather than raw DirectML access:
```toml
[dependencies]
ort = { version = "2.0", features = ["directml"] }
```
```rust
use ort::{Session, ExecutionProviderDispatch};
let session = Session::builder()?
    .with_execution_providers([ExecutionProviderDispatch::DirectML(Default::default())])?
    .commit_from_file("model.onnx")?;
```

**Current state:** The existing `gguf_bridge.rs` / `wgpu` inference path targets
DirectML via `wgpu`'s DirectX 12 backend — not via the DirectML SDK directly or
`ort`. The warning is emitted because the `build.rs` anticipates a future explicit
link but has not yet been wired. This is tracked as a future task; the current
release ships without GPU acceleration on Windows.

---

## Error 4 — benchmark.html: Qualia WASM not loaded

**URL:** https://mediaprophet.github.io/qualiaDB/benchmark.html

**Symptom:** The "QualiaDB JS" column in the benchmark table loads but does not
use the actual `qualia_core_db_bg.wasm`.

**Cause:** `benchmark.html` was written to benchmark a JavaScript proxy engine
and Oxigraph WASM (from esm.sh CDN). It does **not** import
`qualia_core_db.js` / `qualia_core_db_bg.wasm` from the playground. The
"QualiaDB JS" engine is a JS simulation, not the compiled Rust WASM.

**Fix:** Import the actual WASM module at the top of `benchmark.html`:
```js
import init, { compile_query_to_json } from '../playground/qualia_core_db.js';
await init('../playground/qualia_core_db_bg.wasm');
```
Then wire `compile_query_to_json` into the benchmark harness as the Qualia engine.

---

## Error 5 — pages.yml: Node.js 20 deprecation warnings

**Symptom (non-fatal warnings):**
```
Node.js 20 actions are deprecated ... will be forced to Node.js 24 by default
starting June 16th, 2026.
```

**Affected actions:** `actions/cache@v4`, `actions/checkout@v4`,
`actions/configure-pages@v5`, `actions/deploy-pages@v4`,
`actions/upload-artifact@v4`.

**Fix:** Update to versions that support Node.js 24, or add to workflow env:
```yaml
env:
  FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true
```
Already present in `pages.yml`; needs adding to `release.yml`.

---

## Pending / Out of Scope for v0.0.6

- `qualia-flutter/macos/` scaffold (needs Mac with Xcode)
- `qualia-flutter/linux/` scaffold (needs Linux desktop dev env)
- DirectML SDK linkage for Windows GPU inference
- Benchmark page wired to actual Qualia WASM
- `win32` and other packages pinned to outdated versions (15 incompatible updates flagged by `flutter pub outdated`)
