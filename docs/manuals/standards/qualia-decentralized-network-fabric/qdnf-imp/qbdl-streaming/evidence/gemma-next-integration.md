# Gemma integration handoff after resumed source review

UPDATE: reviewed parent integration is now applied. Source fingerprints: gemma-integrated-source.json. Root core compile/execution is still pending. The isolated pinned supplier tests passed17 GGML unit tests plus6 public GGML contract tests; see gemma-codec-backport-preparation.json.

The initial child review was unrouted. After the root core build passed and routing was explicitly unfrozen, integration below was applied. Twelve Gemma tests are now authored (nine graph/ownership/legacy-refusal and three metadata cases), not executed. No physical model success claim is made. Source snapshot: gemma-unrouted-source-review.json.

The checked owner retains actual token input, per-layer KV counts and sliding/global class, explicit shared producers, the full caller-requested context, session-local PLE and a poisoned all-layers-or-zero token step. It reuses retained ModelByteHandle, canonical tensor decoding, existing GEMM, norms, expert routing and the existing single RoPE function. The extracted FFN arithmetic is unchanged. Source review added refusal for a foreign Qwen PLE dialect, cached global/PLE aliases that disagree with admitted directory rows, duplicate/unknown roles and truncated payloads; use checks reject a replaced source or directory. Metadata tests preserve KV8/2 arrays and boolean masks and reject duplicates, bounds, unsupported transforms and nonfinite geometry. Numerical tests use explicit f64 softmax and PLE formulas; these are not full-model fidelity qualification.

## Coordinated parent edits after root unfreezes

1. Route native gguf_bridge::gemma4; add native engine gemma4: Option<gemma4::Session> and gemma4_required: bool, initialize empty/false. Keep wasm behavior unchanged.
2. In native checked GGUF load, parse gemma4::Profile before generic decode_supported. Only Some(profile) routes load_gemma4_gguf with the existing requested context/state budget. Do not weaken the generic architecture guard. New bridge installs the checked index/cache and owns packed f32 KV; parent kv_cache_bytes must report Session::kv_cache_bytes.
3. Clear both Gemma fields at every existing model replacement/FTW/dense-Qwen adoption boundary. Reset Session through reset_kv_cache. Generic hidden-only layer/forward, chunk prefill, legacy hybrid snapshot/restore and native CUDA paths must refuse when gemma4_required; no token-zero substitution or stale fallback after take/panic.
4. Route Gemma output norm and output-readiness checks before generic output. dispatch_gemma4_token takes the actual token ID and reads its retained embedding/PLE row itself. It returns all layers or zero. Logit softcap remains existing output owner behavior.
5. Mechanically remove the legacy Gemma layer dispatcher, thread-local PLE ring and now-extracted Gemma-only helpers from gemma4_qwen35.rs. Preserve Qwen gated attention and the single rope_neox_partial_inplace; make that helper pub(super) so the child import resolves. Do not leave two active Gemma graph owners.
6. Existing real-model smoke helper passes its actual token to dispatch_gemma4_token for Gemma. Preserve every existing layer-count, finite-state and output assertion. Retained source handle test extraction is already routed.
7. Replace the misleading generic Gemma success assertion in inference/gguf_sharder/tests.rs with generic guard refusal; retain unknown-PLE refusal. The checked architecture Plan success/refusal fixtures live in the new child tests above, not in a broadened generic guard.

## Supplier integration

Root reviews vendor/etalon-model-6ace6cc/PROVENANCE.md, Cargo.toml and BACKPORT-SHA256.json, then owns the root git-source patch and lock resolution. Exact canonical kernel git identity remains pinned. The backport is required for 26B Q5_1/MXFP4; E4B uses existing pinned codecs. No current-local ModelHyperparams/LayerTensors/general architecture admission or router-rounding changes are imported. Source preparation is complete; vendor tests/dependency resolution are unrun.

## Execution still required

Root-run supplier MXFP4 edge vectors and full pinned-backport GGML tests; one coordinated core no-run build with routed Gemma children; synthetic metadata/graph owner tests; then real E4B and 26B model selectors and retained all-layer assertions. The prior Qwen failure needs its routed dense-owner test run separately. The three Gemma failures remain open until actual execution establishes results.

Integration review found a legacy speculative verifier that ignores the forward result; with a previously ready Gemma session it could project a raw embedding after hidden-only forward refusal. It now refuses architecture-owned Gemma/dense-Qwen sessions before changing state; a new regression checks hidden/context/cursor preservation. The GPU top-k compatibility entry similarly refuses these owner profiles, matching the existing top1/CUDA boundary. General native/browser consumer token loops outside the approved smoke helper were not rewritten; callers must use the explicit token-aware Gemma API, and hidden-only Gemma calls refuse.
