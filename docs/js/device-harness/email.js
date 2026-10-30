import { counts, REPORT_LIMIT, serializeReport } from './report.js';

export const REPORT_FILENAME = 'qualia-etalon-device-report.json';
export const MAILTO_LIMIT = 1800;

export function reportSharePayload(report, FileType = globalThis.File) {
  if (typeof FileType !== 'function') throw new Error('File sharing is unavailable');
  const file = new FileType([serializeReport(report)], REPORT_FILENAME, { type: 'application/json' });
  if (file.size > REPORT_LIMIT) throw new Error('Report exceeds attachment budget');
  return {
    files: [file], title: 'Qualia / Etalon device report',
    text: 'Device diagnostics JSON. Review before sharing; choose your mail app and recipient.',
  };
}

export function reportEmailDraft(report, recipient = '') {
  report = JSON.parse(serializeReport(report));
  const to = String(recipient).trim();
  if (to.length > 254 || (to && !/^[^\s@<>:,;?&#]+@[a-zA-Z0-9](?:[a-zA-Z0-9.-]*[a-zA-Z0-9])?$/.test(to))) {
    throw new Error('Enter one plain email address, or leave the recipient blank');
  }
  const totals = counts(report.cases ?? []);
  const revision = value => /^[a-f0-9]{40}$/.test(value) ? value : 'unknown';
  const outcome = ['running', 'failed', 'completed', 'unsupported', 'cancelled'].includes(report.outcome) ? report.outcome : 'unknown';
  const body = [
    'COMPACT SUMMARY ONLY. Full diagnostics are not in this draft.',
    `Download ${REPORT_FILENAME} from the harness and attach it manually.`,
    `Outcome: ${outcome}; assets verified: ${report.assets_verified === true}.`,
    `Qualia: ${revision(report.build?.qualia_revision)}`,
    `Etalon: ${revision(report.build?.etalon_revision)}`,
    `Source dirty: ${typeof report.build?.dirty === 'boolean' ? report.build.dirty : 'unknown'}.`,
    ...['cpu', 'gpu', 'npu'].map(lane => `${lane.toUpperCase()}: ${totals[lane].passed} passed, ${totals[lane].failed} failed, ${totals[lane].unsupported} unsupported.`),
    'Browser NPU discovery/execution is not provided.',
    `Errors: ${report.errors?.length ?? 0}; console records: ${report.logs?.length ?? 0}. See the full JSON for details.`,
    'Review the JSON, address and attachment before sending. Delivery is not confirmed by the harness.',
  ].join('\n');
  const url = `mailto:${encodeURIComponent(to)}?subject=${encodeURIComponent('Qualia / Etalon device diagnostics')}&body=${encodeURIComponent(body)}`;
  if (new TextEncoder().encode(url).length > MAILTO_LIMIT) {
    throw new Error('Email draft exceeds its URL budget. Download JSON and attach it in your mail app manually');
  }
  return { url, body };
}
