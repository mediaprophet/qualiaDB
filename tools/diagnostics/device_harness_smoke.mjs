#!/usr/bin/env node
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, isAbsolute, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { once } from 'node:events';
import { chromium } from 'playwright';
import { createDocsServer } from '../../scripts/serve-device-harness.mjs';
import { compatibilityOnlyWorkerSource } from './compatibility-only.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const reportLimit = 1024 * 1024;
const args = process.argv.slice(2);
const allowUnsupported = args.includes('--allow-unsupported-gpu');
const checkEmail = args.includes('--check-email');
const compatibilityOnly = args.includes('--compatibility-only');
const refusalModes = ['--adapter-refusal', '--worker-adapter-refusal', '--rust-adapter-refusal'];
const refusalMode = refusalModes.find(mode => args.includes(mode));
assert(args.filter(arg => refusalModes.includes(arg)).length <= 1, 'Choose at most one adapter refusal fixture');
assert(!refusalMode || !compatibilityOnly, 'Adapter refusal and compatibility-only fixtures are separate');
const outIndex = args.indexOf('--out');
if (outIndex < 0 || !args[outIndex + 1]
  || args.some((arg, index) => !['--out', '--allow-unsupported-gpu', '--check-email', '--compatibility-only', ...refusalModes].includes(arg) && index !== outIndex + 1)) {
  throw new Error('Usage: node tools/diagnostics/device_harness_smoke.mjs --out target/qualification/NEW-DIRECTORY [--allow-unsupported-gpu] [--check-email] [--compatibility-only | --adapter-refusal | --worker-adapter-refusal | --rust-adapter-refusal]');
}
const output = resolve(root, args[outIndex + 1]);
const qualification = resolve(root, 'target/qualification');
const part = relative(qualification, output);
assert(part && !isAbsolute(part) && part !== '..' && !part.startsWith(`..${sep}`),
  'Receipt directory must be inside target/qualification');
await mkdir(dirname(output), { recursive: true });
await mkdir(output);

const logs = [];
const record = (kind, message) => {
  if (logs.length < 64) logs.push({ kind, text: String(message).slice(0, 2048) });
};
const receipt = {
  schema_version: 1, started_at: new Date().toISOString(), outcome: 'failed',
  test: 'actual source-linked browser UI and downloaded report',
  viewport: { width: 390, height: 844, mobile_emulation: true },
  limitations: ['Viewport emulation is not physical phone hardware coverage',
    'Browser WebGPU execution is not native Etalon HAL or Naga validation'],
  adapter_refusal_fixture: refusalMode ?? null,
};
const server = createDocsServer();
let browser;
let deadline;
try {
  const listening = once(server, 'listening');
  server.listen(0, '127.0.0.1');
  await listening;
  const origin = `http://127.0.0.1:${server.address().port}`;
  const manifest = JSON.parse(await readFile(resolve(root, 'docs/playground/device-harness-build.json'), 'utf8'));
  assert.equal(manifest.schema_version, 1);
  for (const key of ['qualia_revision', 'etalon_revision']) assert.match(manifest[key], /^[a-f0-9]{40}$/);
  const manifestUrl = new URL('/playground/device-harness-build.json', origin);
  for (const key of ['portal_js', 'portal_wasm']) {
    const asset = manifest.assets[key];
    const url = new URL(asset.url, manifestUrl);
    assert.equal(url.origin, origin);
    const assetPath = resolve(root, 'docs', `.${decodeURIComponent(url.pathname)}`);
    const docPart = relative(resolve(root, 'docs'), assetPath);
    assert(!isAbsolute(docPart) && docPart !== '..' && !docPart.startsWith(`..${sep}`));
    const bytes = await readFile(assetPath);
    assert.equal(bytes.length, asset.bytes, `${key} bytes differ from manifest`);
    assert.equal(createHash('sha256').update(bytes).digest('hex'), asset.sha256, `${key} hash differs from manifest`);
  }
  browser = await chromium.launch({ channel: 'msedge', headless: true });
  const context = await browser.newContext({
    viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, acceptDownloads: true,
  });
  if (refusalMode === '--adapter-refusal') await context.addInitScript(() => {
    if (navigator.gpu) navigator.gpu.requestAdapter = async () => null;
  });
  await context.route('**/*', async route => {
    const url = new URL(route.request().url());
    if (compatibilityOnly && url.pathname === '/js/device-harness/worker.js') {
      const response = await route.fetch();
      return route.fulfill({ response, body: compatibilityOnlyWorkerSource(await response.text()) });
    }
    if (refusalMode && url.pathname === '/js/device-harness/worker.js') {
      const response = await route.fetch();
      const prefix = refusalMode === '--rust-adapter-refusal' ? `
const originalRequestAdapter = navigator.gpu.requestAdapter.bind(navigator.gpu);
let requestCount = 0;
navigator.gpu.requestAdapter = options => ++requestCount === 1 ? Promise.resolve(null) : originalRequestAdapter(options);
` : 'navigator.gpu.requestAdapter = async () => null;\n';
      return route.fulfill({ response, body: prefix + await response.text() });
    }
    return url.origin === origin || ['blob:', 'data:'].includes(url.protocol)
      ? route.continue() : route.abort('blockedbyclient');
  });
  const page = await context.newPage();
  const failedScripts = [];
  page.on('response', response => {
    if (response.request().resourceType() === 'script' && response.status() >= 400) {
      failedScripts.push(`${response.status()} ${response.url()}`);
    }
  });
  page.on('console', message => record(`console:${message.type()}`, message.text()));
  page.on('pageerror', error => record('pageerror', error.message));
  page.on('requestfailed', request => record('requestfailed', `${request.url()}: ${request.failure()?.errorText}`));
  deadline = setTimeout(() => browser?.close().catch(() => {}), 270000);
  await page.goto(`${origin}/playground/etalon-vibe-harness.html`, { waitUntil: 'networkidle', timeout: 30000 });
  assert.equal(failedScripts.length, 0, `Required UI script failed to load: ${failedScripts.join(', ')}`);
  await page.locator('#run').click();
  await page.waitForFunction(() => {
    const text = document.getElementById('report')?.textContent;
    try { return ['completed', 'failed', 'unsupported', 'cancelled'].includes(JSON.parse(text).outcome); }
    catch { return false; }
  }, null, { timeout: 245000 });
  const displayedText = await page.locator('#report').textContent();
  assert(Buffer.byteLength(displayedText) <= reportLimit, 'Displayed report exceeds one MiB');
  const report = JSON.parse(displayedText);
  await writeFile(resolve(output, 'ui-report.json'), displayedText, { flag: 'wx' });
  receipt.build = report.build;
  receipt.execution_counts = report.execution_counts;
  receipt.browser = report.browser;
  assert.equal(report.schema_version, 1);
  assert.equal(report.outcome, 'completed', `UI failed: ${JSON.stringify(report.errors)}`);
  assert.equal(report.assets_verified, true);
  assert.equal(report.browser.secure_context, true, 'localhost must remain a secure context');
  for (const key of ['qualia_revision', 'etalon_revision']) assert.equal(report.build[key], manifest[key]);
  assert.equal(report.build.dirty, manifest.dirty);
  if (manifest.tracked_diff_sha256) assert.equal(report.build.tracked_diff_sha256, manifest.tracked_diff_sha256);
  for (const key of ['portal_js', 'portal_wasm']) assert.equal(report.build.assets[key].sha256, manifest.assets[key].sha256);
  assert.equal(report.cases.length, 3);
  assert.deepEqual(report.cases.map(item => item.name).sort(), [
    'canonical_vibe_graph_transaction', 'canonical_vibe_capability_rejection',
    'etalon_browser_compute_readback',
  ].sort());
  assert.equal(report.execution_counts.cpu.passed, 2);
  assert.equal(report.execution_counts.cpu.failed, 0);
  const gpu = report.cases.find(item => item.execution_class === 'gpu');
  assert.equal(gpu.outcome, refusalMode || (allowUnsupported && gpu.outcome === 'unsupported') ? 'unsupported' : 'passed',
    `GPU integration did not pass: ${JSON.stringify(gpu.details)}`);
  if (gpu.outcome === 'passed') {
    assert.equal(report.webgpu_diagnostics, undefined, 'Successful Rust GPU checks must not run discovery probes');
    if (compatibilityOnly) assert.equal(gpu.details.requested_feature_level, 'compatibility');
    assert.equal(gpu.details.actualbackend, 'BrowserWebGpu');
    assert.equal(gpu.details.dispatches, 11);
    assert.deepEqual(gpu.details.element_counts, [1, 63, 64, 65, 257]);
    assert.equal(gpu.details.sparse_zero_initialization_verified, true);
    assert.equal(gpu.details.submission_index, null);
    assert.equal(gpu.details.native_hal_or_naga_coverage, false);
  }
  if (refusalMode) {
    const classifications = {
      '--adapter-refusal': 'browser_refused_core_and_compatibility_in_both_contexts',
      '--worker-adapter-refusal': 'window_adapter_available_worker_unavailable',
      '--rust-adapter-refusal': 'raw_worker_adapter_available_after_rust_refusal',
    };
    assert.equal(report.webgpu_diagnostics.classification, classifications[refusalMode]);
    assert.equal(report.webgpu_diagnostics.gpu_execution, false);
    assert.equal(report.execution_counts.gpu.passed, 0);
    assert.equal(report.execution_counts.gpu.unsupported, 1);
    assert.equal(report.errors.length, 0);
    receipt.webgpu_diagnostics = report.webgpu_diagnostics;
  }
  assert.equal(report.npu.execution, 'not_run');
  assert.equal(report.execution_counts.npu.passed, 0);
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false,
    'Harness overflows the mobile viewport horizontally');
  const downloaded = page.waitForEvent('download', { timeout: 10000 });
  await page.locator('#download').click();
  const download = await downloaded;
  assert.equal(download.suggestedFilename(), 'qualia-etalon-device-report.json');
  const stream = await download.createReadStream();
  assert(stream, 'Download stream is unavailable');
  let length = 0;
  const chunks = [];
  for await (const chunk of stream) {
    length += chunk.length;
    assert(length <= reportLimit, 'Downloaded report exceeds one MiB');
    chunks.push(chunk);
  }
  const downloadBytes = Buffer.concat(chunks);
  assert.deepEqual(JSON.parse(downloadBytes.toString('utf8')), report);
  await writeFile(resolve(output, 'downloaded-report.json'), downloadBytes, { flag: 'wx' });
  if (checkEmail) {
    await page.evaluate(() => {
      window.__shareMode = 'success';
      window.__shareCalls = [];
      window.__mailtos = [];
      Object.defineProperty(navigator, 'canShare', { configurable: true, value: payload =>
        window.__shareMode !== 'unsupported' && payload.files.length === 1 });
      Object.defineProperty(navigator, 'share', { configurable: true, value: async payload => {
        const file = payload.files[0];
        window.__shareCalls.push({ name: file.name, type: file.type, size: file.size, text: await file.text() });
        if (window.__shareMode === 'cancel') throw new DOMException('Synthetic cancellation', 'AbortError');
      } });
      const click = HTMLAnchorElement.prototype.click;
      HTMLAnchorElement.prototype.click = function () {
        if (this.protocol === 'mailto:') { window.__mailtos.push(this.href); return; }
        return click.call(this);
      };
    });
    await page.locator('#email-recipient').fill('self-test@example.invalid');
    await page.locator('#email').click();
    await page.waitForFunction(() => document.getElementById('status').textContent.includes('Share sheet completed'));
    const shared = await page.evaluate(() => window.__shareCalls[0]);
    assert.equal(shared.name, 'qualia-etalon-device-report.json');
    assert.equal(shared.type, 'application/json');
    assert(shared.size <= reportLimit);
    assert.deepEqual(JSON.parse(shared.text), report);
    assert(!shared.text.includes('self-test@example.invalid'), 'Optional recipient leaked into report');
    await page.evaluate(() => { window.__shareMode = 'cancel'; });
    await page.locator('#email').click();
    await page.waitForFunction(() => document.getElementById('status').textContent.includes('Sharing cancelled'));
    assert.deepEqual(JSON.parse(await page.locator('#report').textContent()), report);
    assert.equal(await page.locator('#email').isEnabled(), true);
    await page.evaluate(() => { window.__shareMode = 'unsupported'; });
    await page.locator('#email').click();
    await page.waitForFunction(() => window.__mailtos.length === 1);
    const mailto = new URL(await page.evaluate(() => window.__mailtos[0]));
    assert.equal(mailto.protocol, 'mailto:');
    assert.equal(decodeURIComponent(mailto.pathname), 'self-test@example.invalid');
    assert(Buffer.byteLength(mailto.href) <= 1800);
    assert.match(mailto.searchParams.get('body'), /COMPACT SUMMARY ONLY/);
    assert.match(mailto.searchParams.get('body'), /attach it manually/);
    assert.match(mailto.searchParams.get('body'), new RegExp(report.build.etalon_revision));
    await page.locator('#email-draft').click();
    await page.waitForFunction(() => window.__mailtos.length === 2);
    await page.locator('#email-recipient').fill('invalid-recipient');
    await page.locator('#email-draft').click();
    assert.equal(await page.evaluate(() => window.__mailtos.length), 2);
    assert.match(await page.locator('#status').textContent(), /plain email address/);
    assert.deepEqual(JSON.parse(await page.locator('#report').textContent()), report);
    receipt.email_ui = { mocked_share_file_matches_report: true, cancellation_preserves_report: true,
      unsupported_share_uses_compact_mailto: true, manual_draft_verified: true,
      invalid_recipient_rejected: true, actual_email_sent: false,
      limitation: 'Browser share APIs mocked; operating-system mail delivery not exercised' };
  }
  receipt.outcome = 'passed';
  receipt.download_verified = true;
} catch (error) {
  receipt.error = String(error?.stack || error).slice(0, 8192);
  process.exitCode = 1;
} finally {
  clearTimeout(deadline);
  await browser?.close().catch(error => record('cleanup', error.message));
  server.closeAllConnections();
  await new Promise(resolveClosed => server.close(resolveClosed));
  receipt.finished_at = new Date().toISOString();
  await writeFile(resolve(output, 'receipt.json'), JSON.stringify(receipt, null, 2), { flag: 'wx' });
  await writeFile(resolve(output, 'console.json'), JSON.stringify(logs, null, 2), { flag: 'wx' });
}
console.log(JSON.stringify({ outcome: receipt.outcome, output, error: receipt.error }));
