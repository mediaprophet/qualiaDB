# WebCivics WASM — Agent API Manual

**Audience:** AI agents and host adapters integrating QualiaDB for Civics
decision-evidence (browser, Node CI, mobile PWA / WebView).  
**Package:** `wasm-webcivics` (Cargo `--features wasm-webcivics`, **no** `gpu-runtime`)  
**Engine:** `0.0.39` · Profile label: `webcivics`  
**Pinned artifact:** `docs/pkg/webcivics/qualia_webcivics_bg.wasm`  
**Digests:** [`../releases/0.0.39-wasm-digests.md`](../releases/0.0.39-wasm-digests.md)  
**Typed contracts:** [`../contracts/qualia-engine-sdk.d.ts`](../contracts/qualia-engine-sdk.d.ts)  
**Civics binding:** `C:/Projects/civics.au/data/web-civics-profile/profile.json`  
**Host storage adapter:** [`../js/webcivics-device-storage.js`](../js/webcivics-device-storage.js)

> Prefer this package for Civics apps. Do **not** load `docs/playground`
> (`wasm-full`) for production admission — it pulls GPU/LLM weight.

---

## 0. Agent rules (read first)

1. **Always** call `list_capabilities_wasm()` / `get_engine_info()` after init and
   refuse to claim a capability that is absent.
2. **Never** fetch remote JSON-LD `@context` URLs at admission time — use the
   pinned context (`jsonld_context_digest_wasm`) or compact serialize.
3. **Quin fields** returned from parse APIs are **decimal strings** (u64 exceeds
   JS `Number.MAX_SAFE_INTEGER`). Prefer `BigInt(s)` when packing for serialize.
4. **RDFC-1.0** may be unavailable — `rdfc10_graph_hash_wasm` is fail-closed;
   do not treat provisional digests as RDFC-1.0.
5. **Sensitivity:** do not egress classified local context. Check rights /
   values gates before any network publish.
6. **Excluded:** `gpu-runtime`, WebGPU viewport, GGUF/LLM/MoE,
   `compile-rdf-to-q42-wasm` (optional derived product, not in this profile).

---

## 1. Load and initialise

### 1.1 Browser / PWA

```js
import init, {
  get_engine_version,
  get_engine_info,
  list_capabilities_wasm,
  compiled_profile, // if exported; else info.profile
} from './pkg/webcivics/qualia.js'; // or docs/pkg/webcivics/qualia.js

await init(); // fetches qualia_webcivics_bg.wasm beside the JS glue

const ver = get_engine_version();           // "0.0.39"
const info = get_engine_info();             // { profile, … }
const caps = list_capabilities_wasm();      // string[]
```

### 1.2 Node (CI / fixtures)

```js
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const mod = await import(pathToFileURL('docs/pkg/webcivics/qualia.js').href);
const bytes = readFileSync('docs/pkg/webcivics/qualia_webcivics_bg.wasm');
mod.initSync({ module: bytes });

assert.equal(mod.get_engine_version(), '0.0.39');
```

### 1.3 Rebuild (operators)

```powershell
$env:RUSTFLAGS='--cfg getrandom_backend="wasm_js" -C target-feature=+simd128'
wasm-pack build crates/qualia-core-db --target web --release --out-dir pkg-webcivics `
  -- --no-default-features --features wasm-webcivics
# publish → docs/pkg/webcivics/{qualia.js,qualia.d.ts,qualia_webcivics_bg.wasm}
node docs/tests/wasm-size-check.mjs docs/pkg/webcivics/qualia_webcivics_bg.wasm 4194304 1572864
node docs/tests/webcivics-solid-rdf.test.mjs
node docs/tests/webcivics-device-storage.test.mjs
```

Gate: ≤ **4 MiB** raw / **1.5 MiB** gzip.

---

## 2. Discovery APIs

| Export | Returns | Agent use |
|---|---|---|
| `get_engine_version()` | `string` | Pin / receipt `engine_version` |
| `get_engine_info()` | object | Profile, build metadata |
| `list_capabilities_wasm()` | `string[]` | Admission checklist |
| `list_native_only_capabilities_wasm()` | `string[]` | Must **not** claim in browser |
| `jsonld_context_digest_wasm()` | digest object | Package receipt pin |
| `package_exposure_manifest_wasm(shapes_json, vibe_cbor?)` | manifest | Content-addressed package header |
| `get_shacl_capability_manifest_wasm()` | coverage | Which SHACL components exist |
| `device_storage_policy_wasm()` | policy | OPFS + backup contract |

**Required WebCivics caps (non-exhaustive):**  
`json-ld-ingest`, `json-ld-serialize`, `json-ld-compact-serialize`,
`solid-rdf-media-types`, `rdf-serialization`, `n3-parser`, `turtle-parser`,
`shacl-property-validation`, `shacl-graph-validation`, `device-storage-policy`,
`opfs-primary-vault`, `backup-folder-link`, modal logic family, `numerical-solvers`,
`economics`, `q42-kernel-sparql-extensions`.

---

## 3. Quin wire format

A **Quin** is six unsigned 64-bit fields:  
`[subject, predicate, object, context, metadata, parity]` (48 bytes on the wire).

| Boundary | Representation |
|---|---|
| Into WASM (`serialize_rdf_wasm`) | `BigInt` or number; prefer `BigInt` |
| Out of WASM (`parse_*`) | **decimal string** per field |
| Packed DB bytes | little-endian `Uint8Array`, length multiple of 48 |

```js
function packQuin(s, p, o, c = 0n, m = 0n, parity = 0n) {
  const u = (v) => (typeof v === 'bigint' ? v : BigInt(v));
  return [u(s), u(p), u(o), u(c), u(m), u(parity)];
}
```

Hashes for IRIs use the engine’s 60-bit FNV-1a (`q_hash` in Rust). For agent
fixtures, prefer round-tripping through RDF text rather than inventing hashes.

---

## 4. Solid / LDP RDF

Solid RDF Sources use MIME types. Qualia format ids (`turtle`, `jsonld`, `n3`)
are also accepted.

### 4.1 Negotiate `Accept`

```js
const neg = solid_negotiate_accept_wasm(
  'application/ld+json, text/turtle;q=0.9, text/n3;q=0.5'
);
// {
//   content_type: 'application/ld+json',
//   format: 'jsonld',
//   supported: [ { content_type, format, solid_primary }, … ],
//   engine_version: '0.0.39'
// }
```

**Primary Solid types:** `text/turtle`, `application/ld+json`, `text/n3`  
**Also mapped:** `text/rdf+n3`, `application/n-triples`, `application/n-quads`,
`application/trig`. Default when empty / `*/*`: `text/turtle`.

### 4.2 Serialize

```js
const out = serialize_rdf_wasm({
  quins: [packQuin(/* … */)],
  format: 'text/turtle',           // or 'application/ld+json' | 'text/n3' | 'turtle' | …
  compact: true,                   // JSON-LD only: pinned @context + @graph
});
// {
//   rdf_data: string,
//   content_type: 'text/turtle' | 'application/ld+json' | 'text/n3' | …,
//   format: 'turtle' | 'jsonld' | 'n3' | …,
//   compact: boolean,
//   engine_version: '0.0.39'
// }
```

| Format | Notes |
|---|---|
| Turtle | Subject blocks with `;` lists |
| N3 | `@prefix` block + Turtle-compatible statements (no formulae synthesised from flat Quins) |
| JSON-LD expanded | Array of node objects |
| JSON-LD compact | `{ "@context": {…pinned…}, "@graph": […] }` — no remote fetch |

### 4.3 Parse by Content-Type

```js
const parsed = parse_rdf_document_wasm(out.content_type, out.rdf_data);
// {
//   content_type, format, quin_count, truncated,
//   quins: string[6][],   // decimal strings
//   engine_version
// }
```

**JSON-LD shortcut:** `parse_jsonld_wasm(payload)` — same quin packing, plus
`context_id` / `context_digest`.

**Also:** `parse_turtle_wasm`, `parse_n3logic_wasm` (N3 **logic** / rules surface),
`parse_cbor_ld_wasm` (vendor `application/vnd.qualia.nquin-cbor`, **not** W3C CBOR-LD).

### 4.4 Agent recipe — Solid GET/PUT cycle

1. `solid_negotiate_accept_wasm(req.headers.accept)`
2. Load quins → `serialize_rdf_wasm({ format: negotiated.content_type, compact: true })`
3. Respond with `Content-Type: result.content_type`
4. On PUT: `parse_rdf_document_wasm(req.headers['content-type'], body)`
5. Persist quins (OPFS / host) and emit a receipt with `engine_version` + digests

Fixture: `node docs/tests/webcivics-solid-rdf.test.mjs`

### 4.5 Leave Qualia → Solid (migration bundle)

Solid Pods / Solid-Databox store **RDF triples and quads**, not 48-byte Quins.
When a user leaves Webizen Desktop or QualiaDB, project their graph to a
portable Solid bundle, then LDP-PUT into any conformant Pod.

**Native (desktop / CLI):** `SolidExporter::export_to_solid_pod(q42, out_dir)`
writes `data.ttl`, `data.nq`, `data.jsonld`, `data.ttl.acl`, `manifest.jsonld`
(Q42LEX-resolved IRIs; classified never egresses; restricted included on owner leave).

**WASM (`wasm-webcivics`):**

```js
const notice = plan_sanctuary_migration_wasm({ quins });
// If notice.requires_choice → show notice.title/body and the two choice labels.
// Do not export until the user picks omit-sanctuary or reclassify-for-solid.

const bundle = export_solid_migration_wasm({
  quins,
  sanctuary_choice: 'omit-sanctuary', // or 'reclassify-for-solid'
  grant_public_read: false,
  owner_webid: 'https://me.example/profile/card#me',
});
// bundle.turtle | .nquads | .jsonld | .acl | .manifest_jsonld | .outcome_summary

// Return path (Solid → Qualia backup for Desktop/PWA restore):
const { backup_json } = import_solid_to_qualia_backup_wasm({
  resources: [{ name: 'data.ttl', content_type: 'text/turtle', body: bundle.turtle }],
});
```

**Solid-Databox import:** PUT `data.ttl` (and optionally `.nq` / `.jsonld`) as
LDP RDF Sources; PUT `data.ttl.acl` beside them; keep `manifest.jsonld` as the
receipt. Optional licensed wasm-webcivics on Databox may `parse_rdf_document_wasm`
for SHACL — **storage stays RDF**.

---

## 5. Device storage and backups (mobile / PWA)

Two layers:

| Layer | Role |
|---|---|
| **WASM policy** | Contract + plan + backup verify (no Window) |
| **JS host adapter** | OPFS I/O, directory picker, IndexedDB handle memory |

### 5.1 Policy WASM

```js
const policy = device_storage_policy_wasm();
// primary.opfsDirectory === 'webcivics'
// backup.suggestedSubdir === 'webcivics-backups'
// recovery.primaryWipeDoesNotEraseBackup === true

const plan = plan_device_storage_wasm({
  quotaBytes: 50_000_000,
  usageBytes: 1_000_000,
  backupFolderLinked: false,
  lastBackupUnix: null,
  nowUnix: Math.floor(Date.now() / 1000),
  staleAfterSecs: 7 * 24 * 3600,
});
// plan.recommendLinkBackupFolder, plan.recommendRunBackup, plan.warnings[]

const v = verify_backup_manifest_wasm(manifest, payloadSha256Hex);
// { ok, errors[], engine_version }
```

### 5.2 Low-level OPFS blocks (engine)

| Export | Purpose |
|---|---|
| `estimate_browser_storage()` | `{ quota, usage, available }` |
| `pack_quins_into_superblock(seq, ownerDid, rawQuinBytes)` | 40960-byte SuperBlock |
| `verify_superblock_ecc(bytes)` | JSON validity report |
| `write_opfs_block(index, bytes)` / `read_opfs_block` / `is_opfs_block_cached` | Block vault |

### 5.3 Host adapter (`docs/js/webcivics-device-storage.js`)

```js
import {
  requestPersist,
  pickBackupFolder,      // Chromium / Android WebView
  setHostBackupPath,     // Flutter / Tauri Documents path
  exportBackup,
  exportBackupDownload,  // iOS PWA fallback
  restoreBackup,
  deviceStorageStatus,
  putVaultFile,
  listVaultFiles,
} from '../js/webcivics-device-storage.js';

await requestPersist();
await pickBackupFolder();                 // remembers handle in IndexedDB
// or: await setHostBackupPath('/data/.../Documents/webcivics-backups');

await putVaultFile('case.jsonld', jsonLdText);
const snap = await exportBackup({ label: 'nightly' });
// snap.via === 'directory' | 'blob'

await restoreBackup(fileOrPackage, {
  verifyWasm: (m, hex) => verify_backup_manifest_wasm(m, hex),
});

const status = await deviceStorageStatus({
  planWasm: plan_device_storage_wasm,
});
```

**Install checklist for agents / hosts**

1. `requestPersist()`
2. Link backup (`pickBackupFolder` **or** `setHostBackupPath`)
3. Write vault files / OPFS blocks for live data
4. Schedule `exportBackup` (stale threshold from `plan_device_storage_wasm`)
5. On OPFS wipe / reinstall: `restoreBackup` from linked folder

Fixture: `node docs/tests/webcivics-device-storage.test.mjs`

---

## 6. SHACL admission

```js
const report = validate_shacl_json_wasm(dataN3, shapesJson);
// { conforms, results: [ { focusNode, resultPath, resultSeverity, … } ], … }

const report2 = validate_shacl_graph_wasm(dbBytes /* n×48 */, shapesJson);

const shapes = compile_shacl_turtle_wasm(turtleShapes);
const coverage = get_shacl_capability_manifest_wasm();
```

Fail closed on unsupported components (`unsupported: true` in a result). Do not
treat a demo facet helper as full `sh:NodeShape` graph enforcement unless
`conforms` and coverage say so.

---

## 7. Query and sample

```js
const prog = compile_query_to_json(sparqlOrBytecodeQuery); // JSON program / error
const json = execute_ntriples_query(query, dbBytes, maxResults); // JSON string
const sample = sample_packed_quins_wasm(dbBytes, maxQuins); // Uint8Array
```

Kernel SPARQL extensions are capability-gated (`q42-kernel-sparql-extensions`).

---

## 8. Logic modalities (bounded)

| Export | Role |
|---|---|
| `evaluate_deontic_wasm` | Obligations / permits / forbids |
| `evaluate_epistemic_wasm` | Knowledge / belief |
| `evaluate_ltl_trace_wasm` | Temporal trace |
| `route_paraconsistent_wasm` | Contradiction isolation |
| `check_subsumption_wasm` | Description logic |
| `enumerate_stable_models_wasm` | ASP worlds |
| `values_check_wasm` / consent / harm ceilings | Personhood / values guard |
| `resolve_lww_wasm` | CRDT LWW merge |

Inputs are structured objects (quin frames / traces). Always check capability
list before relying on a modality in an issued report.

---

## 9. Receipted calculations (Civics)

| Export | Domain |
|---|---|
| `calculate_welfare_metrics_wasm` | Gini / Atkinson / distribution |
| `calculate_leontief_multipliers_wasm` | IO multipliers |
| `compute_ols_diagnostics_wasm` | Simple (one-x) OLS + receipt |
| `verify_regression_model_receipt_wasm` | Multiple OLS + Ch.4 verification battery + receipt |
| `ols_multiple_wasm` | Multiple OLS (β, SE, t, p, R², F, residuals, fitted) |
| `verify_regression_model_wasm` | Same battery without receipt wrapper |
| `stats_jarque_bera_wasm` / `stats_breusch_pagan_wasm` / `stats_durbin_watson_wasm` / `stats_vif_wasm` | Individual verification tests |
| `stats_influence_wasm` / `stats_outlier_screen_univariate_wasm` / `stats_mahalanobis_outliers_wasm` | Outlier / influence **flags only** (never auto-delete) |
| `stats_ramsey_reset_wasm` / `stats_chow_test_wasm` / `stats_residual_symmetry_wasm` / `stats_residual_runs_wasm` | Specification / structural / residual pattern |
| `stats_stepwise_backward_wasm` | Exploratory selection (`exploratory: true` in result) |
| `design_dummies_wasm` / `transform_series_wasm` | Design helpers |
| `stats_logit_wasm` / `stats_lda_wasm` | Discrete Y |
| `stats_spurious_guard_wasm` | Trend / non-stationarity warning |

Each receipted call returns a **`CalculationReceipt`**: `engine_version`, `algorithm`,
`sample_size`, `seed`, `converged`, `warnings[]`, `receipt_hash`. Attach the
receipt to any issued report claim.

**Admission rule for linear regression claims:** run `verify_regression_model_receipt_wasm`
(or compose REG-01–08). If `ok === false` (hetero / autocorr / non-normal / high-VIF /
RESET), do **not** issue the claim unless an explicit human waiver Quin is attached.
Outlier / influence exports only **flag** row indices — never silently drop observations.

Units / exact math / stats / CAS / transforms are also exported; see the
generated `docs/pkg/webcivics/qualia.d.ts` for the full list. Prefer typed
shapes in `qualia-engine-sdk.d.ts` when present.

---

## 10. Package digests and provenance

```js
const ctx = jsonld_context_digest_wasm();
// { id, digest_alg: 'sha256', digest, context, byte_len, engine_version }

const pkg = package_exposure_manifest_wasm(shapesJson, vibeProgramCborOrNull);
// context_digest_sha256, shapes_digest_sha256, vibe_ast_digest_sha256, …

const hash = rdfc10_graph_hash_wasm({ quins, include_provisional: true });
// available === false until RDFC ships; may include provisional SPO digest only
```

Pin the **WASM binary SHA-256** from `0.0.39-wasm-digests.md` in every issued
run receipt (`profile.json` → `runtime.artifact`).

---

## 11. Explicit non-goals (this profile)

Do **not** call or advertise:

- WebGPU viewport / acoustic / 10D demos
- GGUF load, streaming LLM decode, MoE
- `compile-rdf-to-q42-wasm` (until exported and fixture-proven)
- Native-only: daemon filesystem volumes, NVMe/ZNS, BLE mesh, eBPF, BFV HE

Use `list_native_only_capabilities_wasm()` as the deny list.

For Vibe program encode/eval, use the separate **`vibe-wasm`** package
(`LocalHost`), not an in-crate vibe host on webcivics.

---

## 12. Common agent workflows

### A. Admit a public JSON-LD release

1. `parse_jsonld_wasm(body)` → quins  
2. `validate_shacl_json_wasm` / `validate_shacl_graph_wasm`  
3. `package_exposure_manifest_wasm` + `jsonld_context_digest_wasm`  
4. Persist via host vault / OPFS  
5. Emit receipt with WASM digest + context digest + `conforms`

### B. Serve Solid RDF

1. Negotiate Accept → serialize compact JSON-LD or Turtle  
2. On write, parse by `Content-Type`, validate shapes, backup

### B2. Leave Qualia → Solid Pod / Databox

1. Load quins (desktop Q42 → `export_to_solid`; browser → OPFS + `export_solid_migration_wasm`)  
2. Confirm `stats.redacted_classified` (must never be zeroed by forcing include)  
3. LDP PUT `data.ttl` (+ `.nq` / `.jsonld`) and `data.ttl.acl` into the Pod  
4. Retain `manifest.jsonld` as the leave receipt

### C. Mobile install hardening

1. `device_storage_policy_wasm` → UI copy  
2. `requestPersist` + link backup folder  
3. `plan_device_storage_wasm` on each session start  
4. If `recommend_run_backup`, call `exportBackup`

### D. Verify a pinned build

```text
node docs/tests/webcivics-solid-rdf.test.mjs
node docs/tests/webcivics-device-storage.test.mjs
Get-FileHash docs/pkg/webcivics/qualia_webcivics_bg.wasm -Algorithm SHA256
```

Compare to Civics `profile.json` `runtime.artifact.sha256`.

---

## 13. Related documents

| Doc | Role |
|---|---|
| [`wasm-webcivics-functionality.md`](wasm-webcivics-functionality.md) | Full functionality catalogue (caps + exports by domain) |
| [`wasm-capability-profiles.md`](wasm-capability-profiles.md) | Profile matrix |
| [`wasm-api.md`](wasm-api.md) | Portal / full playground (older surface) |
| [`../releases/0.0.39-wasm-digests.md`](../releases/0.0.39-wasm-digests.md) | SHA-256 pins |
| [`../contracts/qualia-engine-sdk.d.ts`](../contracts/qualia-engine-sdk.d.ts) | Typed Civics contracts |
| Civics `13-web-civics-profile.md` | Product contract narrative |
| Civics `12-qualia-wasm-development-register.md` | VW / QW status |

---

## 14. Export index (webcivics — logical groups)

Agents should discover via `list_capabilities_wasm` + `qualia.d.ts`; this index
is a navigation aid only.

- **Meta:** `get_engine_version`, `get_engine_info`, `list_capabilities_wasm`,
  `list_native_only_capabilities_wasm`
- **Solid RDF:** `serialize_rdf_wasm`, `parse_rdf_document_wasm`,
  `solid_negotiate_accept_wasm`, `parse_jsonld_wasm`, `parse_turtle_wasm`,
  `parse_n3logic_wasm`, `parse_cbor_ld_wasm`
- **Device storage:** `device_storage_policy_wasm`, `plan_device_storage_wasm`,
  `verify_backup_manifest_wasm`, `estimate_browser_storage`,
  `pack_quins_into_superblock`, `verify_superblock_ecc`,
  `write_opfs_block`, `read_opfs_block`, `is_opfs_block_cached`
- **SHACL / package:** `validate_shacl_*`, `compile_shacl_turtle_wasm`,
  `get_shacl_capability_manifest_wasm`, `jsonld_context_digest_wasm`,
  `package_exposure_manifest_wasm`, `rdfc10_graph_hash_wasm`
- **Query:** `compile_query_to_json`, `execute_ntriples_query`,
  `sample_packed_quins_wasm`
- **Logic / values:** `evaluate_*`, `route_paraconsistent_wasm`,
  `check_subsumption_wasm`, `enumerate_stable_models_wasm`, `values_*`,
  `resolve_lww_wasm`, `enforce_rights_ontology`
- **Receipted econ/stats:** `calculate_welfare_metrics_wasm`,
  `calculate_leontief_multipliers_wasm`, `compute_ols_diagnostics_wasm`,
  plus `stats_*`, `units_*`, `exact_*`, `la_*`, `cas_*`, `num_*`, `xform_*`
- **Ingest helpers:** `parse_csv_wasm`, `parse_json_mapping_wasm`,
  `serialize_csv_wasm`, `serialize_json_wasm`

Generated TypeScript: `docs/pkg/webcivics/qualia.d.ts` (authoritative for a
given pin).
