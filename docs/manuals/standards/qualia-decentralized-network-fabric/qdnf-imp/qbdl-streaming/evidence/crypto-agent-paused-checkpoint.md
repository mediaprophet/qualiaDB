# Paused checkpoint — 2026-11-13

Stopped at user-requested battery pause. No worker build/process running. Root owns active compile processes. No vendor preparation begun. No further parent edits applied after dense routing freeze.

## Routed dense Qwen (frozen, awaiting root compile)

New gguf_bridge/qwen35/gguf/{mod,metadata,plan,linear,session,bridge,tests}.rs; parent edits in gguf_bridge/{state,init,load,forward,ssm_forward,output,moe_ffn}.rs and qwen35/mod.rs. Six tests authored, not run by this agent. Root core90836 reported E0433: linear.rs and session.rs must use qualia_inference_kernel::recurrent (existing canonical façade) instead of etalon_inference_kernel::recurrent. Correction NOT applied while compile was frozen. After termination confirmation, replace that prefix in those two files and format. Generic architecture guard unchanged. Existing smoke tests extracted to gemma4_qwen35/tests.rs and whole-model to_vec replaced by retained ModelByteHandle; assertions unchanged.

## Gemma NEW, entirely unrouted

Directory gguf_bridge/gemma4/ contains {mod,metadata,plan,primitives,attention,ffn,ple,session,bridge,fixtures,tests}.rs. All files formatted, <=301 lines at last formatting (tests later received sixth case). No build/test run. Architecture metadata preserves actual per-layer KV8/2, sliding/global masks and shared producers. Session retains actual token PLE and packed f32 KV; token-aware dispatch owns embedding read. Existing FFN/scalar helpers mechanically extracted, no alternate quantization primitive. Six tests cover independent f64 attention/PLE formulas, equal-dimension shared producer distinction, KV8/2, interleaved sessions/nonzero tokens/reset, strict failure/context bounds.

These children reference future engine fields gemma4 and gemma4_required; root routing not applied. primitives.rs reuses existing gemma4_qwen35::rope_neox_partial_inplace, which needs pub(super) visibility at integration. Old thread-local PLE/legacy Gemma dispatcher still exists in the currently routed source; delete it only during the approved coordinated extraction and replace generic Gemma forward with refusal. Integrate native token-aware owner, reset/replacement clearing, kv_cache_bytes and output readiness; preserve one RoPE implementation. Smoke callers must pass their actual token via dispatch_gemma4_token, retaining all assertions. Parent approved replacing the misleading generic Gemma guard-success unit test with generic refusal plus actual checked Gemma Plan success/rejection fixtures, preserving unknown-PLE refusal. No such test-parent edit applied yet. Source compatibility review and actual numerical tests still pending.

## Canonical supplier fixes/tests

Edited C:/github/etalon/crates/etalon-model/src/ggml/blocks.rs: corrected Q5_0 and Q5_1 upper-half high-bit selection to bitj+16. Added ggml/tests/q5_1.rs and routing in ggml/tests.rs; appended concise Unreleased CHANGELOG entry. Root reports all3 Q5 selector tests passed, evidence/etalon-q5-1-repair.log. Existing Q5 tests are untouched since that run.

Latest completed action: added NEW ggml/tests/mxfp4.rs and its routing line in tests.rs, formatted. Two tests use independent natural-E2M1/f64 exponent oracle at exponents0,1,2,126,127,128,254,255 for all nibbles; test second block ordering, partial tail and bounds. These MXFP4 tests have NOT run. Supplier is again stable; no build started by this agent.

## Approved but NOT started

Root approved isolated vendor/etalon-model-6ace6cc under Qualia, copied from C:/Users/Hp/.cargo/git/checkouts/etalon-853ee32392c1daca/6ace6cc/crates/etalon-model with original license/provenance and only reviewed codec delta/tests. Preserve package etalon-model0.0.2 and SAME pinned git etalon-inference-kernel identity (https://github.com/mediaprophet/etalon.git rev6ace6cc6413a87ca10a1da58397ddc7c0b2fc41e); materialize workspace metadata/dependencies in the isolated manifest. Do not import current local ModelHyperparams, generic guard broadening, LayerTensors additions, kquants changes or router rounding changes. Root alone integrates Cargo/lock. Review evidence/gemma-codec-supplier-integration.md and gemma-codec-supplier-delta.patch; the patch already includes corrected Q5 high-bit expressions. Copy both independent Q5 and MXFP4 tests into backport. Vendor path has not been created.

## Next steps after explicit resume

1. Confirm root compile termination/unfreeze, apply the two dense kernel façade path corrections, collect actual core errors/tests.
2. Run coordinated supplier MXFP4 selector; no worker builds without root scheduling.
3. Finish Gemma child source review, integrate approved parents/tests only when unfrozen, and run real source fixture tests before physical model selectors. Three Gemma physical/unit failures remain open; no qualified support claim.
4. Prepare approved canonical pinned vendor backport with license/source hash manifest, submit to root for Cargo identity review.

Other evidence: core-model-graph-diagnosis.md, core-model-graph-source.json (earlier pre-routing snapshot), qwen35-debug-decode.log. Crypto/object-stream production remains frozen and unchanged during this inference work.
