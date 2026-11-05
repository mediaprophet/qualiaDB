# Native test repair — 2026-11-05

## Scope and baseline — done

Repair the six native failures recorded in the previous CI workstream without
weakening assertions, skipping the physical model, or bypassing architecture
validation. Baseline: clean `abbe6fe2e` on `0.0.40.12-etalon-migration`.

Three independent lanes own request-epoch/ring lifecycle, local TLS/HTTP2 socket
ownership, and physical Qwen loading/architecture dispatch. The coordinator owns
integration review, this log, and final full-suite verification. All work uses the
canonical tree and shared build cache.

Previous measured baseline: 10,164 passed, six failed, three ignored. Two Qwen
failures also reproduce on the pre-repair binary; three ring cases and the TLS
case pass individually. No new build measurements yet.

Initial inspection confirms that closed request epochs must remain closed and
that the existing Qwen architecture rejection protects against executing a
hybrid model through the wrong forward path. These protections must remain.

Human input: none this step. Next: fix real lifecycle/dispatch defects, preserve
the existing physical execution assertions, and run focused plus full tests.

## Lifecycle corrections — focused verification passed

The three ring/producer tests explicitly reproduce closure of a prior request,
reject inactive publication, activate a genuine new request, preserve their
original assertions, and verify stale-envelope rejection across another epoch.
Native subprocess isolation follows the existing global-substrate test pattern;
the parent verifies the exact test name and one executed passing test. Production
epoch gates were not changed. The test module was extracted from the large owner.

The TLS/H2 endpoints now retain the listener and join their owned runtime at final
drop. A nested-runtime caller uses a joined shutdown thread. CONNECT setup has a
bounded deadline and TCP errors identify their stage. New tests establish port
reservation and immediate task destruction. The historical Windows 10048 failure
did not recur in 80 baseline repetitions; causation is not claimed from that
unreproduced excerpt.

Rust 1.96.1, locked native lib compilation passed. Focused verification: compute
universe 12/12 (1.50s), connectivity 22/22 (3.07s). An editor Cargo check held the
artifact lock initially; it was allowed to finish, not terminated.

## Qwen architecture integration — in progress

The generic `decode_supported` Qwen rejection remains. A separate FTW owner
admits the complete mixed schedule and uses pinned Etalon recurrent and gated
attention kernels, explicit FP8 row scales, and NVFP4 per-row F16 globals. The
legacy scalar-global expert view cannot represent this bank encoding. Full
attention Q/K/V descriptors slice both fused weights and matching row scales.
All original physical test assertions remain unchanged.

FreeToken source establishes global QKVZ and BA packing, per-head Q/gate packing,
and folded ordinary RMSNorm multipliers. The hotfix manifest added 413 tensors
(vision plus input scales) without replacing the existing tensor descriptors or
norms. Automatic admission selects a reviewed artifact receipt, independent of
its filesystem path, rather than guessing conventions from names/dimensions.

SHA-256 evidence:

| Artifact | Digest |
| --- | --- |
| freetoken_weight.json | `79bb256dc54229377986d1863f91cb123b43d7d015cc7418ddbf580aa8c9249d` |
| config.json | `58aefa1c9eff7989f431d748f2ddec39446cb1fd2a69acc46e285c6a37b0cecc` |
| hf_quant_config.json | `75fe7cc8d5836b58734e05ee67423a4ce91d602aaad45c8173a1b7597cd57663` |
| FreeToken qwen3_5_moe/weight.py | `90f6bafdcf91c52cfbd19de517ea71f7c2642f30672a56e595dd6f49c499cd0a` |
| FreeToken qwen3_5_moe/gdn.py | `3fe0d06a3ba3aeb65b53bcfbadfddf20ddcd15664820214a91f3c6d10cabc630` |
| FreeToken qwen3_5_moe/attention.py | `d2de35ac0acb47e126d9e793611502bcf12ba920fda95b0918879f1b500804db` |

The receipt selects exporter semantics; it is not a hash of the weight shards.
Source custody remains a separate condition. A replaced FTW source owner fails
closed. Governed snapshot/lease preparation that replaces that owner requires
fresh architecture admission before numerical execution. This integration does
not assert support for unreviewed exports or silently substitute a developer
request for a governed request.

The native route owns a fresh sequence, checks cancellation between stages,
poisons partial failed tokens, and uses existing publication/finalization gates.
It refuses unsupported modifiers. Budgeted state is outside the Sentinel arena;
all numerical outputs/scratch are preallocated and hot execution is allocation-free.
Next: compile integration, exercise the original physical tests, then full suite.

Integration verification: locked native build passed; Qwen-focused tests 15/15
and the split-shard fixture 1/1 passed. Complete-token execution and reset/replay
were measured at zero allocations, with exact replay equality. Source-owner and
directory replacement are rejected. A further physical admission failure exposed
compatibility-index canonicalization: fused QKV weights and their row scales map
to the same compatibility name. The architecture path now resolves these from
their distinct original FTW manifest entries, with a dedicated regression. The
first physical run stopped at admission in 4.55 seconds; this is not recorded as
a numerical pass. The next integrated build and physical rerun are pending.

Further integration findings: the SSM-only default is 36 MiB, insufficient for
the full Qwen recurrent/KV/scratch owner (about 108 MiB). The separate public
`adopt_ftw_package_with_budget` API selects context capacity and whole-model state
bytes; the convenience default is 1,024 positions and 128 MiB. It checks metadata
size before opening the package and checks plan plus session reservation before
allocating state. The legacy SSM and Sentinel budgets remain unchanged.

The original physical-load test then passed in 41.63s, executing all its original
SSM, MoE, mixed-forward, norm, and output projection assertions. The original
autoregressive test returned zero tokens after 43.46s with the unchanged
interactive deadline. Typed error/position/deadline diagnostics were added before
deciding whether this is a numerical or qualification-budget failure.

Exact-order FP8 and NVFP4 lookup kernels replace repeated per-scalar decoding and
temporary row expansion. They retain multiplication order and sequential F32
accumulation, including the different signed-zero identities of the two original
oracles. No approximation, SIMD reassociation, or full-model dequantization was
introduced. Combined native build passed (4m22s); Qwen focused tests 24/24 passed
(4.25s), including finite-code/scales, signed-zero, overflow, malformed-input,
allocation and lifetime oracles. The original generic Qwen rejection test passed
1/1. Optimized physical tests are running with the original deadline and assertions.

Optimized physical load passed in 14.77s (cache conditions uncontrolled; not a
formal benchmark). The original autoregressive test failed in 37.57s with the
precise diagnostic `position=0, error=Canceled, elapsed_ms=30040,
budget_ms=30000`. This confirms that the interactive deadline correctly rejects
an incomplete first 40-layer CPU token in the debug executable.

The physical functional qualification now explicitly selects a finite 30-minute
batch deadline through `PreparedInferenceSettings`' thread-local RAII request
scope. The three-token control, prompt, generation/provenance assertions, all
40 layers, and full-vocabulary projection remain intact. The scope propagates
to the worker and restores prior settings on success/unwind. No global timeout,
production interactive deadline, architecture gate, layer cap or skip was changed.
This is functional qualification, not a claim that this CPU path meets a
30-second interactive latency target. A complete serial native suite is next.

## Final native qualification

Final-source locked native build (Rust 1.96.1) passed in 4m09s. The complete serial
native library suite passed: **10,194 passed, 0 failed, 3 existing ignores,
0 filtered out; 10,197 total**, in **1,940.39 seconds**. All six original failures
passed in suite order, including the real installed physical Qwen package. The
baseline was 10,164 passed / 6 failed / 3 ignored: the repaired suite adds 24 tests.
No original assertion was removed and no architecture rejection was relaxed.

The original native executable was invoked with `--test-threads=1`. Its retained
bounded evidence is `target/qualification/native-suite-4e371a596c7b446cafd1a80e2f21d0fb/`:
marker (including binary identity), `suite.log` (935,736 bytes) and `receipt.json`
(exit code 0, no runner failure). This directory is explicitly retained for review;
it is excluded from source control. The wrapper finished in 1,941.96 seconds.

The TLS baseline's Windows error 10048 did not reproduce in 80 isolated runs.
Listener/runtime ownership regressions and the original authenticated CSCP/QSession
end-to-end test now pass; this does not prove which operation caused that earlier
OS error. Production epoch cancellation is unchanged. The architecture guard
`qwen35moe_decode_supported_fails_closed` remains unchanged and passes.

WASM compatibility: `cargo +1.96.1 check --locked --target wasm32-unknown-unknown
-p qualia-core-db --no-default-features --features wasm-llm` passed in 1m57s,
with 120 warnings and no errors. Jobs=1, incremental disabled, dev debug=0;
repository target flags and the existing target directory were used. This is a
compile check, not a mobile inference or browser execution qualification.
`git diff --check` also passed. No dependency versions or lockfiles changed.
