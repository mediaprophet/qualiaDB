import { boundedText, serializeReport } from './report.js';
import { reportEmailDraft, reportSharePayload, REPORT_FILENAME } from './email.js';

const byId = id => document.getElementById(id);
const run = byId('run');
const download = byId('download');
const copy = byId('copy');
const status = byId('status');
let worker;
let timer;
let report;
let finished = true;
let sharing = false;

function render() {
  byId('report').textContent = serializeReport(report);
  download.disabled = false;
  copy.disabled = false;
  byId('email').disabled = sharing;
  byId('email-draft').disabled = false;
}

function finish(outcome, message) {
  clearTimeout(timer);
  if (worker) { worker.onmessage = null; worker.onerror = null; worker.onmessageerror = null; }
  worker?.terminate();
  worker = undefined;
  finished = true;
  report.outcome = outcome;
  report.finished_at = new Date().toISOString();
  if (message) report.errors.push(boundedText(message));
  run.disabled = false;
  byId('cancel').disabled = true;
  status.textContent = message ?? (outcome === 'completed'
    ? (report.assets_verified ? 'Checks complete. Review each outcome below.' : 'Limited HTTP CPU diagnostics complete; assets unverified and GPU unsupported.')
    : 'Run stopped. Partial report is available.');
  render();
}

function deadline(phase, milliseconds) {
  clearTimeout(timer);
  report.phase = phase;
  status.textContent = `${phase}…`;
  timer = setTimeout(() => finish('failed', `${phase} exceeded ${milliseconds / 1000}s; worker terminated`), milliseconds);
}

run.addEventListener('click', () => {
  if (!finished) return;
  finished = false;
  report = {
    schema_version: 1, started_at: new Date().toISOString(), outcome: 'running',
    scope: 'Qualia canonical Vibe CPU and Rust Etalon browser WebGPU integration; no native HAL or CTS claim',
    browser: {
      user_agent: boundedText(navigator.userAgent, 1024), secure_context: isSecureContext,
      cross_origin_isolated: typeof crossOriginIsolated !== 'undefined' && crossOriginIsolated, webassembly: typeof WebAssembly === 'object',
      worker: typeof Worker === 'function', webgpu_api: !!navigator.gpu,
      crypto_sha256: !!globalThis.crypto?.subtle,
    },
    npu: { hardware: 'unknown', runtime: 'not_probed', workload: 'not_probed', execution: 'not_run', reason: 'Browser harness provides no NPU discovery or execution proof' },
    assets_verified: false, cases: [], errors: [], logs: [],
  };
  run.disabled = true;
  byId('cancel').disabled = false;
  render();
  if (typeof Worker !== 'function' || typeof WebAssembly !== 'object' || (isSecureContext && !globalThis.crypto?.subtle)) {
    finish('unsupported', 'Module Workers and WebAssembly are required; secure contexts also require SHA256 verification.');
    return;
  }
  try {
    worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });
    worker.onmessage = ({ data }) => {
      if (finished) return;
      if (data?.type === 'log') {
        if (report.logs.length < 64) report.logs.push({ level: boundedText(data.value?.level, 16), text: boundedText(data.value?.text) });
      } else if (data?.type === 'manifest') report.build = data.value;
      else if (data?.type === 'ready') {
        report.build = data.value.build;
        report.assets_verified = data.value.assets_verified === true;
        report.build_attribution = boundedText(data.value.build_attribution, 512);
        deadline('CPU integration', 20000);
        worker.postMessage({ type: 'cpu' });
      } else if (data?.type === 'cpu') {
        report.cases.push(...data.value);
        render();
        deadline('Browser GPU integration', 60000);
        worker.postMessage({ type: 'gpu' });
      } else if (data?.type === 'gpu') {
        report.cases.push(...data.value);
        finish(report.cases.some(x => x.outcome === 'failed') ? 'failed' : 'completed');
      } else if (data?.type === 'failure') finish('failed', `${boundedText(data.value?.phase, 64)}: ${boundedText(data.value?.message)}`);
    };
    worker.onerror = event => { event.preventDefault(); if (!finished) finish('failed', boundedText(event.message)); };
    worker.onmessageerror = () => { if (!finished) finish('failed', 'Worker message could not be decoded'); };
    deadline('Verify and load build', 150000);
    worker.postMessage({ type: 'load', manifestUrl: new URL('../../playground/device-harness-build.json', import.meta.url).href });
  } catch (error) { finish('failed', boundedText(error)); }
});

byId('cancel').addEventListener('click', () => { if (!finished) finish('cancelled', 'Cancelled by user; worker terminated'); });
download.addEventListener('click', () => {
  const url = URL.createObjectURL(new Blob([serializeReport(report)], { type: 'application/json' }));
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = REPORT_FILENAME;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 10000);
});
copy.addEventListener('click', async () => {
  try { await navigator.clipboard.writeText(serializeReport(report)); status.textContent = 'Report copied.'; }
  catch { status.textContent = 'Clipboard unavailable. Use Download JSON or select the report text.'; }
});

function openEmailDraft() {
  try {
    const draft = reportEmailDraft(report, byId('email-recipient').value ?? '');
    const anchor = document.createElement('a');
    anchor.href = draft.url;
    anchor.click();
    status.textContent = 'Email draft requested with a compact summary only. Download JSON and attach it manually; confirm delivery in your mail app.';
  } catch (error) { status.textContent = boundedText(error?.message ?? error); }
}

byId('email-draft').addEventListener('click', openEmailDraft);
byId('email').addEventListener('click', async () => {
  if (!report || sharing) return;
  let payload;
  try {
    payload = reportSharePayload(report);
    if (typeof navigator.share !== 'function' || typeof navigator.canShare !== 'function'
      || !navigator.canShare({ files: payload.files })) { openEmailDraft(); return; }
  } catch { openEmailDraft(); return; }
  sharing = true;
  byId('email').disabled = true;
  try {
    await navigator.share(payload);
    status.textContent = 'Share sheet completed. Check the JSON attachment, recipient and delivery in your chosen app; the harness cannot confirm sending.';
  } catch (error) {
    status.textContent = error?.name === 'AbortError'
      ? 'Sharing cancelled. Your report remains available to download or share again.'
      : 'File sharing was unavailable. Download JSON, then use Open email draft and attach it manually.';
  } finally { sharing = false; byId('email').disabled = false; }
});
addEventListener('pagehide', () => { clearTimeout(timer); worker?.terminate(); });
