/**
 * QualiaDB / Vibe WASM typed contracts for Civics decision-evidence (0.0.39).
 *
 * Hand-authored stable surface for receipted SHACL + scientific exports +
 * Solid RDF + device storage. Keep in sync with:
 *   - crates/qualia-core-db/src/wasm_bridge/semantic.rs
 *   - crates/qualia-core-db/src/wasm_bridge/device_storage.rs
 *   - crates/qualia-core-db/src/wasm_bridge/compute.rs
 *   - crates/vibe-wasm/src/lib.rs (VW-01…VW-03)
 *   - docs/manuals/wasm-webcivics-agent-api.md  ← agent integration manual
 *
 * These declarations document the contract Civics adapters should target.
 * Regenerating a full wasm-bindgen .d.ts from a release build remains the
 * authoritative export list for a pinned artifact digest
 * (`docs/pkg/webcivics/qualia.d.ts`).
 *
 * UE-054 artifact digests (2026-09-25): see docs/releases/0.0.39-wasm-digests.md
 *   webcivics SHA-256 7c2de8203d6456a2ed852084f9e45dabf4647cd6ac990b0ca25cd7767c89cb71
 *   portal SHA-256 d457ea508d618398a9424f86cfdac371f5d663376da39e92c1b78adc3b1ffbbb
 *   wasm-full SHA-256 3ed9eecad144607ccdcbaec94a0f22cb74bba5406dee19cbf66b43a33081d0ea
 *   JSON-LD context SHA-256 e358841aea43b7248e33f227b7463e591ea7a3f1f1b77baeb8c71aac049bc173
 *
 * Preferred Civics package: Cargo `--features wasm-webcivics` (no gpu-runtime).
 * Profile label from `compiled_profile()` → `"webcivics"`.
 */

/** QW-07 calculation receipt attached to welfare / Leontief / OLS WASM results. */
export interface CalculationReceipt {
  engine_version: string;
  algorithm: string;
  sample_size: number;
  seed: number | null;
  converged: boolean;
  tolerances: number | null;
  warnings: string[];
  receipt_hash: string;
}

/** One SHACL validation result (focus / path / component / severity). */
export interface ValidationResult {
  conforms?: boolean;
  focusNode?: string;
  resultPath?: string;
  value?: string;
  sourceConstraintComponent?: string;
  sourceShape?: string;
  resultSeverity?: "Violation" | "Warning" | "Info" | string;
  resultMessage?: string;
  /** Present when the component is not implemented — fail-closed. */
  unsupported?: boolean;
}

/** Graph-level SHACL ValidationReport (QW-04 / QW-05). */
export interface ValidationReport {
  conforms: boolean;
  results: ValidationResult[];
  shapes_hash?: string;
  data_hash?: string;
  engine_version?: string;
  timestamp_utc?: string;
}

/** Machine-readable SHACL coverage manifest (QW-05). */
export interface ShaclCapabilityManifest {
  standard_core: string[];
  extended_modalities: string[];
  computational_economics: string[];
  profile: string;
  engine_version: string;
}

export interface WelfareMetricsResult {
  gini: number;
  atkinson: number | null;
  mean: number;
  median: number;
  p10: number;
  p90: number;
  palma_ratio: number | null;
  receipt: CalculationReceipt;
}

export interface LeontiefMultipliersResult {
  total_output: number[];
  output_multipliers: number[];
  leontief_inverse: number[];
  receipt: CalculationReceipt;
}

export interface OlsDiagnosticsResult {
  slope: number;
  intercept: number;
  r_squared: number;
  residual_std_error: number;
  slope_std_error: number;
  slope_t: number;
  slope_p_value: number;
  intercept_std_error: number;
  intercept_p_value: number;
  n: number;
  receipt: CalculationReceipt;
}

/** VW-02 / VW-03 Vibe program execution receipt. */
export interface VibeExecutionReceipt {
  sourceHash: string;
  astHash: string;
  languageVersion: string;
  hostVersion: string;
  profile: string;
  status: "success" | string;
}

export interface VibeEvalWithReceipt {
  ok: boolean;
  value?: unknown;
  receipt?: VibeExecutionReceipt;
  error?: unknown;
}

export interface VibeDecodeProgramResult {
  ok: boolean;
  tag: number;
  canonicalSource: string;
  astHash: string;
  itemsCount: number;
  languageVersion: string;
  hostVersion: string;
  profile: string;
}

/** Content type / profile for tagged Vibe AST (Tag 4200). */
export declare const VIBE_CBOR_LD_PROFILE: "application/vnd.vibe.ast+cbor; version=0.1";

/** Pinned JSON-LD 1.1 context + SHA-256 digest for package receipts (UE-012). */
export interface JsonLdContextDigest {
  id: string;
  digest_alg: "sha256";
  digest: string;
  context: string;
  byte_len: number;
  engine_version: string;
}

/** RDFC-1.0 status — fail-closed until a conforming implementation ships (UE-013). */
export interface Rdfc10GraphHashResult {
  profile: "RDFC-1.0";
  available: boolean;
  rdfc10_digest: string | null;
  error: string | null;
  provisional?: {
    profile: "qualia:provisional-spo-sha256-v1" | string;
    digest_alg: "sha256";
    digest: string;
    warning: string;
  } | null;
  engine_version: string;
}

/** Six-field Quin as decimal strings (u64 > Number.MAX_SAFE_INTEGER). */
export type QuinWire = [string, string, string, string, string, string];

/** Solid / LDP Content-Type negotiation result. */
export interface SolidNegotiateAcceptResult {
  content_type: string;
  format: string;
  supported: Array<{
    content_type: string;
    format: string;
    solid_primary: boolean;
  }>;
  engine_version: string;
}

/** serialize_rdf_wasm input. */
export interface RdfSerializeParams {
  quins: Array<[bigint | number | string, bigint | number | string, bigint | number | string, bigint | number | string?, bigint | number | string?, bigint | number | string?]>;
  /** Qualia id (`turtle`, `jsonld`, `n3`, …) or Solid MIME (`text/turtle`, …). */
  format: string;
  /** When true and format is JSON-LD, emit compact document with pinned `@context`. */
  compact?: boolean;
}

export interface RdfSerializeResult {
  rdf_data: string;
  content_type: string;
  format: string;
  compact: boolean;
  engine_version: string;
}

export interface RdfParseDocumentResult {
  content_type: string;
  format: string;
  quin_count: number | string;
  truncated: boolean;
  quins: QuinWire[];
  engine_version: string;
}

/** Leave Qualia → Solid portable bundle (triples/quads, not Quins). */
export interface SolidMigrationExportParams {
  quins: Array<[bigint | number | string, bigint | number | string, bigint | number | string, bigint | number | string?, bigint | number | string?, bigint | number | string?]>;
  /** Required when sanctuary data present: `omit-sanctuary` | `reclassify-for-solid`. */
  sanctuary_choice?: 'omit-sanctuary' | 'reclassify-for-solid' | 'unset';
  /** Legacy alias for `reclassify-for-solid`. */
  include_restricted?: boolean;
  grant_public_read?: boolean;
  owner_webid?: string;
  lexicon?: Record<string, string>;
}

export interface SanctuaryMigrationNotice {
  sanctuary_quin_count: number;
  classified_quin_count: number;
  requires_choice: boolean;
  title: string;
  body: string;
  choice_omit_label: string;
  choice_reclassify_label: string;
  classified_note: string;
  engine_version: string;
}

export interface SolidMigrationBundleResult {
  turtle?: string;
  nquads?: string;
  jsonld?: string;
  acl?: string;
  manifest_jsonld: string;
  stats: {
    scanned: number;
    exported: number;
    redacted_classified: number;
    redacted_restricted: number;
    skipped_parity: number;
  };
  sanctuary_choice: string;
  outcome_summary: string;
  engine_version: string;
}

/** device_storage_policy_wasm — OPFS primary + backup folder contract. */
export interface DeviceStoragePolicy {
  profile: string;
  engineVersion: string;
  primary: {
    kind: "opfs" | string;
    opfsDirectory: string;
    manifestName: string;
    persistPermission: boolean;
    blockSizeBytes: number;
    minHeadroomBytes: number;
  };
  backup: {
    kind: string;
    idbName: string;
    idbStore: string;
    idbKey: string;
    suggestedSubdir: string;
    retentionCount: number;
    formats: string[];
    fallbackWhenNoPicker: string;
  };
  recovery: {
    restoreRequiresManifest: boolean;
    verifyContentHash: boolean;
    primaryWipeDoesNotEraseBackup: boolean;
  };
  capabilities: string[];
}

export interface StoragePlanInput {
  quotaBytes: number;
  usageBytes: number;
  backupFolderLinked: boolean;
  lastBackupUnix?: number | null;
  nowUnix: number;
  staleAfterSecs?: number;
}

export interface StoragePlan {
  availableBytes: number;
  primaryOk: boolean;
  recommendPersist: boolean;
  recommendLinkBackupFolder: boolean;
  recommendRunBackup: boolean;
  warnings: string[];
  engineVersion: string;
}

export interface BackupManifest {
  schema: "webcivics.vault-backup.v1" | string;
  createdUnix: number;
  contentSha256: string;
  byteLength: number;
  primaryKind: "opfs" | "host-path" | string;
  notes?: string | null;
}

export interface BackupVerifyResult {
  ok: boolean;
  errors: string[];
  engineVersion: string;
}

/**
 * Minimal ambient module shape for the WASM bridge exports Civics wires first.
 * Actual wasm-bindgen modules may nest these under a generated InitOutput.
 */
export interface QualiaEngineWasmExports {
  get_engine_version(): string;
  get_engine_info(): { profile?: string; engine_version?: string; [k: string]: unknown };
  list_capabilities_wasm(): string[];
  jsonld_context_digest_wasm(): JsonLdContextDigest;
  package_exposure_manifest_wasm(
    shapes_json: string,
    vibe_program_cbor?: number[] | null
  ): {
    profile: string;
    engine_version: string;
    context_id: string;
    context_digest_sha256: string;
    shapes_digest_sha256: string;
    vibe_ast_tag: number;
    vibe_ast_digest_sha256: string;
  };
  list_native_only_capabilities_wasm(): string[];
  rdfc10_graph_hash_wasm(params: {
    quins?: number[][];
    include_provisional?: boolean;
  }): Rdfc10GraphHashResult;
  serialize_rdf_wasm(params: RdfSerializeParams): RdfSerializeResult;
  parse_rdf_document_wasm(content_type: string, payload: string): RdfParseDocumentResult;
  solid_negotiate_accept_wasm(accept: string): SolidNegotiateAcceptResult;
  plan_sanctuary_migration_wasm(params: { quins: SolidMigrationExportParams['quins'] }): SanctuaryMigrationNotice;
  /** Project quins → Solid RDF leave bundle (requires sanctuary_choice when needed). */
  export_solid_migration_wasm(params: SolidMigrationExportParams): SolidMigrationBundleResult;
  /** Solid RDF Sources → Qualia webcivics.vault-backup.v1 JSON. */
  import_solid_to_qualia_backup_wasm(params: {
    resources: Array<{ name: string; content_type: string; body: string }>;
  }): { backup_json: string; engine_version: string };
  device_storage_policy_wasm(): DeviceStoragePolicy;
  plan_device_storage_wasm(params: StoragePlanInput): StoragePlan;
  verify_backup_manifest_wasm(
    manifest: BackupManifest,
    payload_sha256_hex: string
  ): BackupVerifyResult;
  parse_jsonld_wasm(payload: string): {
    profile: string;
    content_type: "application/ld+json";
    quin_count: number;
    truncated: boolean;
    /** Decimal strings — prefer BigInt when re-serializing. */
    quins: QuinWire[];
    context_id: string;
    context_digest: string;
    engine_version: string;
  };
  parse_cbor_ld_wasm(payload: Uint8Array): {
    /** Vendor profile — not W3C CBOR-LD. Inline typed literals are lossless via @value/@type. */
    profile: "application/vnd.qualia.nquin-cbor";
    subject: string;
    predicate: string;
    object: string;
    context: string;
  } | null;
  validate_shacl_json_wasm(data_n3: string, shapes_json: string): ValidationReport;
  validate_shacl_graph_wasm(db_bytes: Uint8Array, shapes_json: string): ValidationReport;
  get_shacl_capability_manifest_wasm(): ShaclCapabilityManifest;
  evaluate_deontic_wasm(params: {
    quins: number[][];
    now_unix?: number;
  }): { verdict_count: number; verdicts: unknown[]; engine_version: string };
  evaluate_epistemic_wasm(params: {
    quins: number[][];
    agent?: number;
    world?: number;
  }): { verdict_count: number; verdicts: unknown[]; engine_version: string };
  route_paraconsistent_wasm(params: {
    quins: number[][];
  }): { consistent: number[][]; isolated: number[][]; engine_version: string };
  evaluate_ltl_trace_wasm(params: {
    trace: number[][];
    formula: {
      kind: "globally" | "finally" | "next" | "until" | "release" | string;
      property?: number;
      ante?: number;
      consequent?: number;
      trigger?: number;
      invariant?: number;
    };
  }): { holds: boolean; engine_version: string };
  check_subsumption_wasm(params: {
    sub_class: number;
    super_class: number;
    tbox: number[][];
  }): { holds: boolean; engine_version: string };
  compile_shacl_turtle_wasm(turtle: string): {
    shape_count: number;
    shapes: Array<{
      target_class: string;
      path: string;
      severity: string;
      constraint_count: number;
      name?: string | null;
    }>;
    engine_version: string;
  };
  enumerate_stable_models_wasm(params: {
    base: number[];
    rules?: number[][];
  }): {
    model_count: number;
    world_contexts: number[];
    engine_version: string;
    note: string;
  };
  calculate_welfare_metrics_wasm(params: {
    incomes: number[];
    epsilon?: number;
  }): WelfareMetricsResult;
  calculate_leontief_multipliers_wasm(params: {
    technical_matrix: number[];
    sectors: number;
    final_demand: number[];
  }): LeontiefMultipliersResult;
  compute_ols_diagnostics_wasm(params: { x: number[]; y: number[] }): OlsDiagnosticsResult;
  forward_chain_wasm(params: {
    facts: string[];
    rules: Array<{ head: string; body: string[]; defeaters: string[] }>;
  }): {
    inferred: string[];
    fact_count: number;
    rule_count: number;
    inferred_count: number;
    receipt: CalculationReceipt;
  };
  values_check_wasm(params: {
    agentType?: string;
    claimsDignityRight?: boolean;
  }): { flagged: boolean; flag: string; engine_version: string };
  values_consent_non_coerced_wasm(params: {
    imbalance?: number;
    explicitThreat?: boolean;
    threshold?: number;
  }): { non_coerced: boolean; coerced: boolean; engine_version: string };
  values_harm_below_ceiling_wasm(params: {
    harm: number;
    ceiling: number;
  }): { below_ceiling: boolean; harm: number; ceiling: number; engine_version: string };
  causal_caused_wasm(params: {
    edges: number[][];
    roots: number[];
    effect: number;
  }): { caused: boolean; engine_version: string };
  fuzzy_t_norm_wasm(params: {
    a: number;
    b: number;
    family?: string;
  }): { degree: number; family: string; engine_version: string };
  stit_brought_about_wasm(params: {
    facts: number[][];
    agent: number;
    content: number;
  }): { brought_about: boolean; engine_version: string };
  jural_correlative_wasm(params: {
    position: number;
  }): { correlative: number; engine_version: string };
}

export interface VibeWasmExports {
  encode_program_cborld_wasm(src: string): Uint8Array;
  decode_program_cborld_wasm(bytes: Uint8Array): VibeDecodeProgramResult;
  eval_program_with_receipt_wasm(src: string): VibeEvalWithReceipt;
}
