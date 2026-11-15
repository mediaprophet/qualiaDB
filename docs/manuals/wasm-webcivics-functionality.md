# `wasm-webcivics` — comprehensive functionality catalogue

**Audience:** humans and agents integrating, reviewing, or packaging the Civics
decision-evidence WASM build.  
**Cargo feature:** `--no-default-features --features wasm-webcivics`  
**Profile label:** `webcivics` (`compiled_profile` / `get_engine_info`)  
**Engine version:** `0.0.39`  
**Does not enable:** `gpu-runtime` (no WebGPU viewport, no GGUF/LLM/MoE in this binary)

| Artifact | Location |
|----------|----------|
| WASM | `docs/pkg/webcivics/qualia_webcivics_bg.wasm` |
| JS glue | `docs/pkg/webcivics/qualia.js` |
| Generated types | `docs/pkg/webcivics/qualia.d.ts` |
| Civics typed contracts | `docs/contracts/qualia-engine-sdk.d.ts` |
| Digests | `docs/releases/0.0.39-wasm-digests.md` |
| Agent how-to | `docs/manuals/wasm-webcivics-agent-api.md` |
| Profile matrix | `docs/manuals/wasm-capability-profiles.md` |
| Host OPFS/backup adapter | `docs/js/webcivics-device-storage.js` |
| Civics pin | `C:/Projects/civics.au/data/web-civics-profile/profile.json` |

**Pin (2026-09-25 leave/migrate closeout):**  
SHA-256 `68987faa5d53dc3c66f3a82e4adfc2cb9953faeeff027e087648b0a6c57ec2ad` ·  
raw **3,072,294** · gzip **1,055,238** · gate ≤ 4 MiB / 1.5 MiB gzip.

**Licence note:** treat this artifact as a **Web Civics / Qualia licensed module**
when embedding in Solid-Databox or third-party servers — do not republish it under
an unrelated MIT tree as if it were free commons.

---

## 1. What this package is

`wasm-webcivics` is the **Civics decision-evidence** slice of QualiaDB compiled to
WebAssembly for **browser, Node CI, and mobile WebView**. It is designed to:

1. Ingest and emit **Solid-friendly RDF** (Turtle, JSON-LD, N3, N-Triples, N-Quads, TriG).
2. Validate with **SHACL** and evaluate **modal / deontic / epistemic** logic over Quins.
3. Run **Civics-relevant compute receipts** (stats, regression verification, econ, numerics, LA, CAS) **without** shipping a GPU/LLM stack.
4. Persist locally via **OPFS** + optional **user backup folder**.
5. Support **leave/migrate** to Solid and **return** from Solid to a Qualia vault-backup package.

Everything semantic is still grounded in the **48-byte Quin** ABI; Solid Pods store
**RDF triples/quads**, not Quins. Leave/migrate projects Quins → RDF; return parses
RDF → Quins / backup JSON.

---

## 2. What this package is not

| Absent | Why |
|--------|-----|
| `gpu-runtime` / WebGPU viewport | Explicitly excluded from the feature |
| GGUF / MoE / streaming LLM decode | Inference is native / other profiles |
| `compile-rdf-to-q42-wasm` | Optional derived product; not in this profile |
| Full BFV HE / calibrated DP releases | Listed under `list_native_only_capabilities_wasm()` |
| Daemon filesystem, NVMe/ZNS, BLE mesh, eBPF/WFP | Native desktop/edge only |
| In-crate Vibe Host | Use separate `vibe-wasm` (`LocalHost`) for program encode/eval |

Always call `list_capabilities_wasm()` after init and **refuse to claim** anything
absent. Use `list_native_only_capabilities_wasm()` as a deny list for browser claims.

---

## 3. Build and verify

```powershell
$env:RUSTFLAGS='--cfg getrandom_backend="wasm_js" -C target-feature=+simd128'
wasm-pack build crates/qualia-core-db --target web --release --out-dir pkg-webcivics `
  -- --no-default-features --features wasm-webcivics
# Publish renaming: qualia_core_db_bg.wasm → docs/pkg/webcivics/qualia_webcivics_bg.wasm
#                   qualia_core_db.js      → docs/pkg/webcivics/qualia.js  (fix wasm URL)

node docs/tests/wasm-size-check.mjs docs/pkg/webcivics/qualia_webcivics_bg.wasm 4194304 1572864
node docs/tests/webcivics-solid-rdf.test.mjs
node docs/tests/webcivics-device-storage.test.mjs
Get-FileHash docs/pkg/webcivics/qualia_webcivics_bg.wasm -Algorithm SHA256
```

---

## 4. Init and discovery

| Export | Behaviour |
|--------|-----------|
| `init` / `initSync` | Load the `.wasm` module (browser fetch or Node bytes) |
| `get_engine_version()` | `"0.0.39"` |
| `get_engine_info()` | Profile / build metadata object |
| `list_capabilities_wasm()` | Advertised capability strings (`WEBCIVICS`) |
| `list_native_only_capabilities_wasm()` | Must **not** claim in browser |
| `jsonld_context_digest_wasm()` | Pinned JSON-LD `@context` digest for receipts |
| `package_exposure_manifest_wasm(shapes_json, vibe_cbor?)` | Content-addressed package header |
| `get_shacl_capability_manifest_wasm()` | Which SHACL components are present |

### 4.1 Advertised capability registry (`WEBCIVICS`)

Source of truth: `crates/qualia-core-db/src/wasm_capabilities.rs`.

| Capability id | Domain |
|---------------|--------|
| `nquin-48-byte-abi` | Core Quin ABI |
| `q-hash` | FNV-1a URI hashing |
| `json-ingest` | Plain JSON → quin mapping |
| `json-ld-ingest` | JSON-LD 1.1 parse |
| `json-ld-serialize` | JSON-LD emit |
| `json-ld-compact-serialize` | Compact JSON-LD + pinned `@context` |
| `json-ld-context-digest` | Context digest for receipts |
| `rdfc-1.0-status` | RDFC-1.0 availability (fail-closed if unavailable) |
| `vendor-nquin-cbor` | `application/vnd.qualia.nquin-cbor` (not W3C CBOR-LD) |
| `package-exposure-manifest` | Package exposure / digests |
| `n3-parser` | N3 / N3-logic surface |
| `turtle-parser` | Turtle documents |
| `rdf-serialization` | Multi-format RDF serialize |
| `solid-rdf-media-types` | Solid MIME negotiate + primary types |
| `solid-leave-migrate` | Qualia → Solid leave bundle + sanctuary choice |
| `solid-qualia-return` | Solid RDF → Qualia vault-backup |
| `ntriples-query` | Packed N-Triples query over quin bytes |
| `query-compiler` | Query → JSON IR |
| `shacl-property-validation` | Numeric / property SHACL |
| `shacl-graph-validation` | Graph SHACL |
| `deontic-logic` | Obligates / permits / forbids |
| `epistemic-logic` | Knows / believes / common knowledge |
| `paraconsistent-routing` | Contradiction isolation |
| `temporal-ltl` | G/F/X/U/R over quin traces |
| `description-logic` | Subsumption |
| `answer-set-programming` | Stable models |
| `linear-logic` | Resource consume / fire |
| `interaction-governance` | Rights / agreement gates |
| `values-personhood-guard` | Values / harm / consent checks |
| `lww-crdt` | Last-writer-wins resolve |
| `economics` | VaR / welfare / Leontief / GBM paths |
| `q42-kernel-sparql-extensions` | SPARQL q42 kernels |
| `symbolic-logic` | SAT / related symbolic helpers |
| `numerical-solvers` | Special functions, ODE stubs, PID, etc. |
| `device-storage-policy` | OPFS + backup policy contract |
| `opfs-primary-vault` | Superblock OPFS I/O |
| `backup-folder-link` | Backup folder / restore verify |
| `regression-verification` | Full REG verification suite |
| `multiple-ols` | Multiple OLS |
| `jarque-bera` | Normality |
| `breusch-pagan` | Heteroscedasticity |
| `durbin-watson` | Autocorrelation |
| `vif` | Multicollinearity |
| `influence-diagnostics` | Cook / leverage / DFBETA-class |
| `ramsey-reset` | Functional form |
| `chow-test` | Structural break |
| `logit-lda` | Discrete choice / LDA |

### 4.2 Native-only deny list (do not claim in browser)

`solvers-qpu`, `specialized-libs-qpu-bridge`, `privacy-he-bfv`,
`privacy-dp-calibrated-releases`, `cryptographic-library-full`,
`machine-learning-heavy`, `engineering-cfd`, `quantum-biology`,
`daemon-filesystem-volumes`, `nvme-zns-csd`, `ble-mesh`,
`ebpf-wfp-network-filter`, `directml-vulkan-native-inference`.

---

## 5. Quin wire format

A **Quin** is six `u64` fields (48 bytes):  
`[subject, predicate, object, context, metadata, parity]`.

| Boundary | Representation |
|----------|----------------|
| Into WASM | Prefer `BigInt` (or number for small ids) |
| Out of WASM (`parse_*`) | **Decimal strings** per field (u64 > `Number.MAX_SAFE_INTEGER`) |
| Packed DB | Little-endian `Uint8Array`, length multiple of 48 |

Parity: leave/migrate and collectors recompute when `parity === 0`. Always prefer
round-tripping through RDF text for fixtures rather than inventing hashes.

Sensitivity lives in the Quin `context` high bits (`SENSITIVITY_*`). **Classified
must never egress** the local node (Informatics Fiduciary Mandate).

---

## 6. Solid / LDP RDF

### 6.1 Media types

| Role | Types |
|------|--------|
| Primary Solid RDF Sources | `text/turtle`, `application/ld+json`, `text/n3` |
| Also mapped | `text/rdf+n3`, `application/n-triples`, `application/n-quads`, `application/trig` |
| Default when empty / `*/*` | `text/turtle` |
| Vendor (not Solid primary) | `application/vnd.qualia.nquin-cbor` |

### 6.2 Exports

| Export | Function |
|--------|----------|
| `solid_negotiate_accept_wasm(accept)` | Parse `Accept` → preferred Content-Type + format list |
| `serialize_rdf_wasm({ quins, format, compact? })` | Quins → RDF string + Content-Type |
| `parse_rdf_document_wasm(content_type, payload)` | RDF → quins (decimal strings) |
| `parse_jsonld_wasm(payload)` | JSON-LD shortcut (+ context digest fields) |
| `parse_turtle_wasm(payload)` | Turtle → quins |
| `parse_n3logic_wasm(payload)` | N3 **logic / rules** surface |
| `parse_cbor_ld_wasm(bytes)` | Vendor nquin-CBOR only |
| `rdfc10_graph_hash_wasm(...)` | RDFC-1.0 graph hash when available; fail-closed otherwise |

Compact JSON-LD embeds the **pinned** Qualia `@context` — no remote context fetch
at admission time.

### 6.3 Agent recipe — Solid GET/PUT

1. `solid_negotiate_accept_wasm(req.headers.accept)`
2. `serialize_rdf_wasm({ format: negotiated.content_type, compact: true })`
3. Respond with that Content-Type
4. On PUT: `parse_rdf_document_wasm(content-type, body)` → persist quins / OPFS
5. Receipt: `engine_version` + digests + capability list

Fixture: `docs/tests/webcivics-solid-rdf.test.mjs`.

---

## 7. Leave Qualia → Solid / return Solid → Qualia

Solid stores triples/quads. This profile projects and re-ingests them.

### 7.1 Sanctuary choice (mandatory)

When sanctuary / restricted / bilateral quins are present, the product **must**
present a choice before export:

| Choice | String | Effect |
|--------|--------|--------|
| Omit | `omit-sanctuary` | Leave sanctuary data behind |
| Reclassify | `reclassify-for-solid` | Owner accepts permission change so data may leave as RDF |
| Unset | `unset` | **Fail closed** if sanctuary present (`SANCTUARY_CHOICE_REQUIRED`) |

**Classified never leaves**, regardless of choice.

| Export | Function |
|--------|----------|
| `plan_sanctuary_migration_wasm({ quins })` | Notice: title, body, choice labels, counts, `requires_choice` |
| `export_solid_migration_wasm({ quins, sanctuary_choice, owner_webid?, grant_public_read?, lexicon? })` | Bundle: turtle, nquads, jsonld, acl, manifest_jsonld, stats, outcome_summary |
| `import_solid_to_qualia_backup_wasm({ resources: [{ name, content_type, body }] })` | → `webcivics.vault-backup.v1` JSON for Desktop/PWA `restoreBackup` |

Capabilities: `solid-leave-migrate`, `solid-qualia-return`.

Native desktop equivalent: `SolidExporter::plan_notice` / `export_with_choice`
(`crates/qualia-core-db/src/services/solid_ldp/`).

Databox agent brief:  
`C:/Projects/Solid-Databox-webcivics/databox/devdocs/handoffs/AGENT-BRIEF-qualia-leave-migrate.md`.

Default ACL: **owner only**. Public `foaf:Agent` Read only if `grant_public_read: true`.

---

## 8. Device storage and backups

Two layers:

| Layer | Role |
|-------|------|
| WASM policy | Contract, plan, backup verify (no Window) |
| JS host | `docs/js/webcivics-device-storage.js` — OPFS I/O, directory picker, IDB handle memory |

### 8.1 Policy constants

| Constant | Value |
|----------|--------|
| OPFS directory | `webcivics/` |
| Vault manifest | `vault-manifest.v1.json` |
| Backup IDB | `webcivics-device-storage-v1` / store `handles` / key `backup_dir_handle` |
| Backup subdir | `webcivics-backups` |
| Block size | 40,960 bytes (SuperBlock) |
| Min headroom | 8 MiB |
| Retention soft target | 5 snapshots |

### 8.2 WASM exports

| Export | Function |
|--------|----------|
| `device_storage_policy_wasm()` | Full policy document |
| `plan_device_storage_wasm({ quotaBytes, usageBytes, backupFolderLinked, lastBackupUnix, nowUnix, staleAfterSecs })` | Recommendations / warnings |
| `verify_backup_manifest_wasm(manifest, payloadSha256Hex)` | Fail-closed verify |
| `estimate_browser_storage()` | Quota / usage (async) |
| `pack_quins_into_superblock(seq_id, owner_did, raw_quin_bytes)` | Pack ledger page |
| `verify_superblock_ecc(block_bytes)` | ECC check |
| `write_opfs_block` / `read_opfs_block` / `is_opfs_block_cached` | OPFS block I/O |

### 8.3 Host adapter highlights

`exportBackup` / `exportBackupDownload` / `restoreBackup` produce or consume
`application/vnd.web-civics.vault-backup+json` with schema `webcivics.vault-backup.v1`.
Primary wipe does **not** erase the linked backup folder.

Fixture: `docs/tests/webcivics-device-storage.test.mjs`.

---

## 9. Ingest / serialise (non-Solid helpers)

| Export | Notes |
|--------|-------|
| `parse_json_wasm` / `serialize_json_wasm` | Plain JSON |
| `parse_json_mapping_wasm` | Mapping helpers |
| `parse_csv_wasm` / `serialize_csv_wasm` | CSV |
| `parse_jsonld_wasm` | JSON-LD → quins |
| `execute_ntriples_query(query, db_bytes, max_results)` | Query packed quin DB |
| `compile_query_to_json(query)` | Query IR |
| `sample_packed_quins_wasm(db_bytes, max_quins)` | Sample slice |
| `serialize_float_array` / `serialize_float64_array` | Numeric buffers |

---

## 10. SHACL and packages

| Export | Function |
|--------|----------|
| `validate_shacl_constraint_wasm` | Single numeric facet |
| `validate_shacl_json_wasm(data_n3, shapes_json)` | N3 data + JSON shapes |
| `validate_shacl_graph_wasm(db_bytes, shapes_json)` | Packed graph + shapes |
| `compile_shacl_turtle_wasm(turtle)` | Turtle shapes → engine form |
| `get_shacl_capability_manifest_wasm()` | Coverage manifest |
| `package_exposure_manifest_wasm` | Digests for shapes / optional Vibe CBOR |
| `jsonld_context_digest_wasm` | Pinned context id + sha256 |

Do not fetch remote `@context` URLs at admission; use the pinned context / compact
serialize path.

---

## 11. Modal logic and governance

| Capability | Exports (representative) |
|------------|---------------------------|
| Deontic | `evaluate_deontic_wasm` |
| Epistemic | `evaluate_epistemic_wasm` |
| Paraconsistent | `route_paraconsistent_wasm` |
| LTL | `evaluate_ltl_trace_wasm` |
| Description logic | `check_subsumption_wasm` |
| ASP | `enumerate_stable_models_wasm` |
| Linear / fuzzy / jural / STIT / causal | `fuzzy_t_norm_wasm`, `jural_correlative_wasm`, `stit_brought_about_wasm`, `causal_caused_wasm`, … |
| Values / personhood | `values_check_wasm`, `values_consent_non_coerced_wasm`, `values_harm_below_ceiling_wasm` |
| Rights | `enforce_rights_ontology(subject_did)` |
| LWW CRDT | `resolve_lww_wasm` |
| Webizen agreements (light) | `webizen_propose_agreement`, `webizen_sign_agreement`, `webizen_poll_agreements` |
| Law package | `verify_law_package_wasm` |
| Mesh | `prune_and_validate_mesh` |
| Offload intents | `intercept_computational_opcode`, `intercept_pharmacogenomics_intent` (intent only — heavy work stays native) |

---

## 12. Computational engine (Civics receipts)

Included under `wasm-webcivics` via `wasm_bridge/engine` + `compute` (no GPU).

### 12.1 Statistics and regression verification

Descriptive and classical tests:  
`stats_describe_wasm`, `stats_correlation_wasm`, `stats_quantile_wasm`,
`stats_one_sample_t_wasm`, `stats_two_sample_t_wasm`, `stats_paired_t_wasm`,
`stats_anova_wasm`, `stats_chi_square_*`, `stats_normal_wasm`, `stats_students_t_wasm`,
`stats_fisher_f_wasm`, `stats_friedman_wasm`, `stats_mcnemar_wasm`, …

**Regression verification (Welc/Rodriguez-class; flag outliers, never auto-delete):**

| Export | Role |
|--------|------|
| `ols_multiple_wasm` | Multiple OLS |
| `stats_linear_regression_wasm` | Simple / companion path |
| `compute_ols_diagnostics_wasm` | Residual diagnostics bundle |
| `stats_jarque_bera_wasm` | Residual normality |
| `stats_breusch_pagan_wasm` | Heteroscedasticity |
| `stats_durbin_watson_wasm` | Serial correlation |
| `stats_vif_wasm` | Multicollinearity |
| `stats_influence_wasm` | Influence / leverage |
| `stats_ramsey_reset_wasm` | Functional form |
| `stats_chow_test_wasm` | Structural break |
| `stats_logit_wasm` / `stats_lda_wasm` | Discrete / LDA |
| `stats_mahalanobis_outliers_wasm` / `stats_outlier_screen_univariate_wasm` | Outlier **flagging** |
| `stats_spurious_guard_wasm` | Spurious regression guard |
| `stats_stepwise_backward_wasm` | Selection helper |
| `stats_residual_runs_wasm` / `stats_residual_symmetry_wasm` | Residual structure |
| `design_dummies_wasm` | Design matrix helpers |
| `verify_regression_model_wasm` / `verify_regression_model_receipt_wasm` | Model / receipt verify |

### 12.2 Linear algebra

`la_matmul_wasm`, `la_transpose_wasm`, `la_determinant_wasm`, `la_solve_wasm`,
`la_eigenvalues_wasm`, `la_eigen_symmetric_wasm`, `la_svd_wasm`,
`la_polynomial_roots_wasm`.

### 12.3 Computer algebra (CAS)

`cas_simplify_wasm`, `cas_expand_wasm`, `cas_factor_wasm`, `cas_differentiate_wasm`,
`cas_evaluate_wasm`, `cas_solve_quadratic_wasm`.

### 12.4 Exact arithmetic

`exact_bigint_*` (add, mul, divmod, pow, gcd, factorial), `exact_rational_add` /
`exact_rational_mul`.

### 12.5 Numerics / special functions

Bessel I/J/K/Y, Airy, zeta, orthopoly, factorial, binomial, combinatorics,
divisors, primes, gcd/lcm, mod pow/inverse, interpolation (linear, Lagrange,
Newton, cubic spline), poly fit, minimize, partitions, arithmetic functions, …

### 12.6 Units

`units_convert`, `units_quantity_op`, `units_constant`, `units_list_units`,
`units_list_constants`.

### 12.7 Transforms

`xform_dft` / `xform_idft`, Laplace numeric/table, Z-transform helpers.

### 12.8 Graph / relational

`graph_shortest_path`, `graph_spreading_activation`, `graph_fuzzy_similarity`,
`graph_kge_score`, `graph_kge_predict`.

### 12.9 Crypto (bounded browser surface)

`crypto_sha256`, `crypto_sha512`, `crypto_sha3_256`, `crypto_blake3`,
`crypto_hkdf_sha256`, `crypto_aead_encrypt`, `crypto_aead_decrypt`.  
(Not the full native cryptographic library / BFV HE.)

### 12.10 Economics / welfare / control

`black_scholes_wasm`, `simulate_gbm_path_wasm`, `calculate_leontief_multipliers_wasm`,
`calculate_welfare_metrics_wasm`, `solve_ode_exponential_decay_wasm`,
`compute_pid_step_wasm`, `run_semantic_simulation`, `solve_sat_wasm`, …

---

## 13. Module map (Rust → JS)

| Rust area | Included in webcivics? |
|-----------|-------------------------|
| `wasm_bridge/semantic.rs` | Yes — RDF, Solid, leave/migrate, SHACL, JSON-LD, packages |
| `wasm_bridge/logic.rs` | Yes — modal / values / CRDT / rights |
| `wasm_bridge/dataio.rs` | Yes — JSON/CSV helpers |
| `wasm_bridge/meta.rs` | Yes — version, caps, manifests |
| `wasm_bridge/device_storage.rs` | Yes — storage policy |
| `wasm_bridge/compute.rs` | Yes (webcivics **or** scientific) — econ/welfare |
| `wasm_bridge/engine/*` | Yes (webcivics **or** scientific) — LA/CAS/stats/… |
| `wasm_bridge/bio`, `chemistry`, `medical`, `geometry` | **No** — `wasm-scientific` only |
| `services/solid_ldp/*` | Library used by leave/return (wasm32-safe filter/consent/import) |

---

## 14. Hosting patterns

### 14.1 Browser / PWA

```js
import init, { list_capabilities_wasm, serialize_rdf_wasm } from './qualia.js';
await init();
```

Use `docs/js/webcivics-device-storage.js` for OPFS + backup folder.

### 14.2 Node CI / Solid-Databox

```js
mod.initSync({ module: readFileSync('…/qualia_webcivics_bg.wasm') });
// Verify SHA-256 against digests doc before trust.
```

Solid-Databox should load this as a **licensed** dependency and use
`QualiaSolidBridge` for import/export (see Databox agent brief).

### 14.3 Desktop

Prefer native `SolidExporter` / daemon paths for large Q42 volumes; use the same
sanctuary-choice contract so UX matches the browser leave path.

---

## 15. Fixtures and conformance

| Suite | Path | Covers |
|-------|------|--------|
| Solid RDF + leave/return | `docs/tests/webcivics-solid-rdf.test.mjs` | MIME round-trip, Accept, leave, sanctuary notice, return backup, caps |
| Device storage | `docs/tests/webcivics-device-storage.test.mjs` | Policy, plan, backup verify |
| Size gate | `docs/tests/wasm-size-check.mjs` | Raw/gzip ceilings |

Civics acceptance checklist:  
`C:/Projects/civics.au/docs/plans/decision-evidence/14-qualia-wasm-webcivics-acceptance-review.md`.

---

## 16. Related documents

| Doc | Role |
|-----|------|
| [`wasm-webcivics-agent-api.md`](wasm-webcivics-agent-api.md) | Agent recipes and rules |
| [`wasm-capability-profiles.md`](wasm-capability-profiles.md) | All WASM profiles |
| [`wasm-api.md`](wasm-api.md) | Broader / portal surface |
| [`../plans/qualia-solid-leave-migrate.md`](../plans/qualia-solid-leave-migrate.md) | Leave/migrate design |
| [`../releases/0.0.39-wasm-digests.md`](../releases/0.0.39-wasm-digests.md) | Pin numbers |
| [`../contracts/qualia-engine-sdk.d.ts`](../contracts/qualia-engine-sdk.d.ts) | Typed Civics SDK |

---

## 17. One-line summary

**`wasm-webcivics`** is QualiaDB’s Civics-facing WASM: Solid RDF + SHACL + modal
logic + local OPFS/backup + stats/regression/econ receipts + sanctuary-aware
Solid leave/return — **without** GPU or LLM — pinned and capability-advertised for
agents and Solid-Databox to load as a licensed module.
