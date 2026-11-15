export const REPORT_LIMIT = 1024 * 1024;
export const EXPECTED = Object.freeze({
  cpu: ['canonical_vibe_graph_transaction', 'canonical_vibe_capability_rejection'],
  gpu: ['etalon_browser_compute_readback'],
});

export function boundedText(value, limit = 2048) {
  return String(value ?? '').slice(0, limit);
}

export function validateManifest(value, manifestUrl) {
  if (value?.schema_version !== 1) throw new Error('Unsupported build manifest');
  if (typeof value.dirty !== 'boolean') throw new Error('Build manifest must declare clean or dirty sources');
  for (const key of ['qualia_revision', 'etalon_revision']) {
    if (!/^[a-f0-9]{40}$/.test(value[key])) throw new Error(`Invalid ${key}`);
  }
  const assets = {};
  for (const [key, limit] of [['portal_js', 16 * 1024 * 1024], ['portal_wasm', 128 * 1024 * 1024]]) {
    const asset = value.assets?.[key];
    if (!asset || !/^[a-f0-9]{64}$/.test(asset.sha256)
      || !Number.isSafeInteger(asset.bytes) || asset.bytes < 1 || asset.bytes > limit) {
      throw new Error(`Invalid or oversized ${key}`);
    }
    const url = new URL(asset.url, manifestUrl);
    if (url.origin !== new URL(manifestUrl).origin || !['http:', 'https:'].includes(url.protocol)
      || url.username || url.password) throw new Error('Assets must use the manifest origin');
    assets[key] = { url: url.href, sha256: asset.sha256, bytes: asset.bytes };
  }
  return {
    schema_version: 1, qualia_revision: value.qualia_revision,
    etalon_revision: value.etalon_revision, generated_at_utc: boundedText(value.generated_at_utc, 64),
    dirty: value.dirty === true, attribution: boundedText(value.attribution, 256),
    etalon_dirty: value.etalon_dirty === true,
    etalon_tracked_diff_sha256: /^[a-f0-9]{64}$/.test(value.etalon_tracked_diff_sha256) ? value.etalon_tracked_diff_sha256 : null,
    etalon_untracked_file_count: Number.isSafeInteger(value.etalon_untracked_file_count) && value.etalon_untracked_file_count >= 0 ? value.etalon_untracked_file_count : null,
    tracked_diff_sha256: /^[a-f0-9]{64}$/.test(value.tracked_diff_sha256) ? value.tracked_diff_sha256 : null,
    untracked_file_count: Number.isSafeInteger(value.untracked_file_count) && value.untracked_file_count >= 0 ? value.untracked_file_count : null,
    captured_at_utc: boundedText(value.captured_at_utc, 64), assets,
  };
}

export function validateCases(raw, lane) {
  if (typeof raw !== 'string' || raw.length > 128 * 1024) throw new Error('Invalid export receipt size');
  const value = JSON.parse(raw);
  const expected = EXPECTED[lane];
  const scope = lane === 'cpu' ? 'qualia-vibe-cpu' : 'etalon-browser-webgpu';
  if (value.schema_version !== 1 || value.scope !== scope || !Array.isArray(value.cases)
    || value.cases.length !== expected.length) throw new Error('Unexpected export coverage');
  const seen = new Set();
  for (const item of value.cases) {
    if (!expected.includes(item.name) || seen.has(item.name) || item.execution_class !== lane
      || !['passed', 'failed', 'unsupported'].includes(item.outcome)) throw new Error('Invalid case outcome');
    seen.add(item.name);
  }
  return value.cases;
}

export function counts(cases) {
  const result = Object.fromEntries(['cpu', 'gpu', 'npu'].map(k => [k, { passed: 0, failed: 0, unsupported: 0 }]));
  for (const item of cases) {
    if (result[item.execution_class]?.[item.outcome] !== undefined) result[item.execution_class][item.outcome]++;
  }
  return result;
}

export function serializeReport(report) {
  const serialized = JSON.stringify({ ...report, execution_counts: counts(report.cases ?? []) }, null, 2);
  if (new TextEncoder().encode(serialized).length <= REPORT_LIMIT) return serialized;
  return JSON.stringify({
    schema_version: 1, outcome: 'failed', error: 'Report exceeded 1 MiB budget; evidence omitted',
    started_at: report.started_at, build: report.build,
    execution_counts: { cpu: { passed: 0 }, gpu: { passed: 0 }, npu: { passed: 0 } },
  }, null, 2);
}
