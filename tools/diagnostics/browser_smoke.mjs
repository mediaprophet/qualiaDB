#!/usr/bin/env node
/**
 * Produce a bounded, portable browser/WebGPU smoke-test receipt.
 *
 * A receipt directory is mandatory and must not already exist. Unsafe WebGPU
 * flags, headed browsing, page completion globals, and text checks are all
 * explicit opt-ins; there are no machine-specific paths or process-killing
 * side effects.
 */
import { mkdir, stat, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import process from 'node:process';
import { chromium } from 'playwright';

const DEFAULT_TIMEOUT_SECONDS = 120;
const DEFAULT_RECEIPT_KIB = 256;
const MAX_LOG_ENTRIES = 64;
const MAX_LOG_CHARS = 768;
const MAX_COMPLETION_CHARS = 32 * 1024;

function usage(message) {
  if (message) console.error(`error: ${message}`);
  console.error(`usage: npm run browser-smoke -- --url <http(s) URL> --out <new receipt dir>
  [--wait-for <CSS selector>] [--completion-global <identifier>]
  [--expect-text <text>] [--timeout-seconds <positive integer>]
  [--max-receipt-kib <positive integer>] [--headful] [--unsafe-webgpu] [--require-webgpu]
  [--allow-console-errors]`);
  process.exit(2);
}

function parseArgs(argv) {
  const parsed = { headful: false, unsafeWebgpu: false, requireWebgpu: false, allowConsoleErrors: false };
  const values = new Map([
    ['--url', 'url'], ['--out', 'out'], ['--wait-for', 'waitFor'],
    ['--completion-global', 'completionGlobal'], ['--expect-text', 'expectText'],
    ['--timeout-seconds', 'timeoutSeconds'], ['--max-receipt-kib', 'maxReceiptKib'],
  ]);
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === '--headful') parsed.headful = true;
    else if (argument === '--unsafe-webgpu') parsed.unsafeWebgpu = true;
    else if (argument === '--require-webgpu') parsed.requireWebgpu = true;
    else if (argument === '--allow-console-errors') parsed.allowConsoleErrors = true;
    else if (values.has(argument)) {
      const value = argv[++index];
      if (!value || value.startsWith('--')) usage(`missing value for ${argument}`);
      parsed[values.get(argument)] = value;
    } else usage(`unknown argument ${argument}`);
  }
  if (!parsed.url || !parsed.out) usage('--url and --out are required');
  let url;
  try { url = new URL(parsed.url); } catch { usage('--url must be an absolute URL'); }
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) {
    usage('--url must be an unauthenticated http(s) URL');
  }
  for (const field of ['timeoutSeconds', 'maxReceiptKib']) {
    if (parsed[field] !== undefined && (!/^\d+$/.test(parsed[field]) || Number(parsed[field]) < 1)) {
      usage(`--${field === 'timeoutSeconds' ? 'timeout-seconds' : 'max-receipt-kib'} must be positive`);
    }
  }
  if (parsed.completionGlobal && !/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(parsed.completionGlobal)) {
    usage('--completion-global must be a simple global identifier');
  }
  parsed.url = url.toString();
  parsed.out = resolve(parsed.out);
  parsed.timeoutSeconds = Number(parsed.timeoutSeconds || DEFAULT_TIMEOUT_SECONDS);
  parsed.maxReceiptBytes = Number(parsed.maxReceiptKib || DEFAULT_RECEIPT_KIB) * 1024;
  return parsed;
}

async function createReceiptDirectory(path) {
  try {
    await stat(path);
    throw new Error(`receipt directory already exists: ${path}`);
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
  await mkdir(path, { recursive: true });
}

function clip(value) {
  return String(value).slice(0, MAX_LOG_CHARS);
}

async function browserCapabilities(page) {
  return page.evaluate(async () => {
    const result = {
      secure_context: window.isSecureContext,
      cross_origin_isolated: window.crossOriginIsolated,
      webgpu_available: Boolean(navigator.gpu),
      webgpu_adapter: null,
    };
    if (!navigator.gpu) return result;
    try {
      const adapter = await navigator.gpu.requestAdapter();
      if (adapter) {
        result.webgpu_adapter = {
          available: true,
          feature_count: adapter.features.size,
          max_buffer_size: Number(adapter.limits.maxBufferSize || 0),
          max_storage_buffer_binding_size: Number(adapter.limits.maxStorageBufferBindingSize || 0),
        };
      } else {
        result.webgpu_adapter = { available: false };
      }
    } catch (error) {
      result.webgpu_adapter = { available: false, error: String(error).slice(0, 240) };
    }
    return result;
  });
}

async function completionValue(page, globalName, timeoutMs) {
  if (!globalName) return { observed: false };
  await page.waitForFunction((name) => {
    const value = window[name];
    return value && (value.status === 'complete' || value.status === 'fatal_error');
  }, globalName, { timeout: timeoutMs });
  return page.evaluate(({ name, maxChars }) => {
    const value = window[name];
    const status = typeof value?.status === 'string' ? value.status : null;
    try {
      const json = JSON.stringify(value);
      return {
        observed: true,
        status,
        json: json.slice(0, maxChars),
        truncated: json.length > maxChars,
      };
    } catch (error) {
      return { observed: true, status, serialization_error: String(error).slice(0, 240) };
    }
  }, { name: globalName, maxChars: MAX_COMPLETION_CHARS });
}

async function writeReceipt(directory, receipt, byteBudget) {
  const encoded = Buffer.from(`${JSON.stringify(receipt, null, 2)}\n`);
  if (encoded.byteLength > byteBudget) {
    throw new Error(`receipt would exceed --max-receipt-kib (${encoded.byteLength} bytes)`);
  }
  await writeFile(resolve(directory, 'receipt.json'), encoded, { flag: 'wx' });
}

async function run(config) {
  const consoleEntries = [];
  const pageErrors = [];
  const failures = [];
  const startedAt = new Date().toISOString();
  let browser;
  try {
    const args = config.unsafeWebgpu
      ? ['--enable-unsafe-webgpu', '--ignore-gpu-blocklist']
      : [];
    browser = await chromium.launch({ headless: !config.headful, args });
    const page = await browser.newPage();
    const timeoutMs = config.timeoutSeconds * 1000;
    page.setDefaultTimeout(timeoutMs);
    page.on('console', (message) => {
      if (consoleEntries.length < MAX_LOG_ENTRIES) {
        consoleEntries.push({ level: message.type(), text: clip(message.text()) });
      }
    });
    page.on('pageerror', (error) => {
      if (pageErrors.length < MAX_LOG_ENTRIES) pageErrors.push(clip(error.message));
    });
    await page.goto(config.url, { waitUntil: 'domcontentloaded', timeout: timeoutMs });
    if (config.waitFor) await page.locator(config.waitFor).waitFor({ state: 'visible', timeout: timeoutMs });
    const capabilities = await browserCapabilities(page);
    if (config.requireWebgpu && !capabilities.webgpu_adapter?.available) {
      failures.push('WebGPU adapter was required but unavailable');
    }
    const completion = await completionValue(page, config.completionGlobal, timeoutMs);
    if (completion.status === 'fatal_error') failures.push('page reported fatal_error');
    if (config.expectText) {
      const bodyText = await page.locator('body').innerText();
      if (!bodyText.includes(config.expectText)) failures.push(`expected text not found: ${config.expectText}`);
    }
    if (!config.allowConsoleErrors && (pageErrors.length || consoleEntries.some((entry) => entry.level === 'error'))) {
      failures.push('browser emitted errors');
    }
    return {
      schema: 'qualia.browser-smoke-receipt/v1',
      started_at: startedAt,
      finished_at: new Date().toISOString(),
      target_url: config.url,
      launch: { headful: config.headful, unsafe_webgpu: config.unsafeWebgpu, require_webgpu: config.requireWebgpu },
      capabilities,
      completion,
      console: consoleEntries,
      page_errors: pageErrors,
      status: failures.length ? 'failed' : 'passed',
      failures,
    };
  } catch (error) {
    return {
      schema: 'qualia.browser-smoke-receipt/v1',
      started_at: startedAt,
      finished_at: new Date().toISOString(),
      target_url: config.url,
      launch: { headful: config.headful, unsafe_webgpu: config.unsafeWebgpu, require_webgpu: config.requireWebgpu },
      status: 'failed',
      failures: [clip(error?.stack || error)],
      console: consoleEntries,
      page_errors: pageErrors,
    };
  } finally {
    if (browser) await browser.close();
  }
}

const config = parseArgs(process.argv.slice(2));
await createReceiptDirectory(config.out);
const receipt = await run(config);
try {
  await writeReceipt(config.out, receipt, config.maxReceiptBytes);
} catch (error) {
  console.error(`error: ${error.message}`);
  process.exit(1);
}
console.log(`${receipt.status.toUpperCase()}: ${resolve(config.out, 'receipt.json')}`);
process.exit(receipt.status === 'passed' ? 0 : 1);
