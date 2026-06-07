/**
 * Browser fetch helper for Qualia docs pages (compare.html, playground, etc.).
 *
 * Strategy order:
 *   1. Same-origin or CORS-friendly rewrite (jsDelivr for this repo's GitHub raw URLs)
 *   2. Direct GET (works when the remote host allows CORS)
 *   3. Local Qualia daemon GET /proxy/fetch?url=… (when qualia-cli daemon is running)
 *   4. Public read-only CORS relay (allorigins) as last resort for GH Pages demos
 */

const DAEMON_PROXY_BASE = 'http://127.0.0.1:4242';
const ALLORIGINS_RAW = 'https://api.allorigins.win/raw?url=';

function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes < 0) return 'unknown size';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

/** Rewrite mediaprophet/qualiaDB GitHub raw/blob URLs to jsDelivr (CORS-enabled CDN). */
export function rewriteToCorsFriendlyUrl(url) {
  try {
    const parsed = new URL(url);
    if (typeof window !== 'undefined' && parsed.origin === window.location.origin) {
      return { url, strategy: 'same-origin' };
    }

    const rawMatch = parsed.href.match(
      /^https:\/\/raw\.githubusercontent\.com\/mediaprophet\/qualiaDB\/([^/]+)\/(.+)$/,
    );
    if (rawMatch) {
      return {
        url: `https://cdn.jsdelivr.net/gh/mediaprophet/qualiaDB@${rawMatch[1]}/${rawMatch[2]}`,
        strategy: 'jsdelivr',
      };
    }

    const blobMatch = parsed.href.match(
      /^https:\/\/github\.com\/mediaprophet\/qualiaDB\/blob\/([^/]+)\/(.+)$/,
    );
    if (blobMatch) {
      return {
        url: `https://cdn.jsdelivr.net/gh/mediaprophet/qualiaDB@${blobMatch[1]}/${blobMatch[2]}`,
        strategy: 'jsdelivr',
      };
    }

    return { url, strategy: 'direct' };
  } catch {
    return { url, strategy: 'direct' };
  }
}

async function fetchWithProgress(url, { onProgress, signal } = {}) {
  const response = await fetch(url, { signal, credentials: 'omit' });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);

  const contentType = response.headers.get('content-type') || '';
  const total = Number(response.headers.get('content-length')) || 0;

  if (!response.body?.getReader) {
    const text = await response.text();
    onProgress?.({ loaded: text.length, total: text.length || text.length, pct: 100 });
    return { text, size: text.length, contentType };
  }

  const reader = response.body.getReader();
  const chunks = [];
  let received = 0;

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
    received += value.byteLength;
    const pct = total ? Math.min(100, (received / total) * 100) : null;
    onProgress?.({ loaded: received, total: total || null, pct });
  }

  const bytes = new Uint8Array(received);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }

  const text = new TextDecoder().decode(bytes);
  return { text, size: received, contentType };
}

async function tryDaemonReachable() {
  try {
    const ctrl = new AbortController();
    const timer = setTimeout(() => ctrl.abort(), 800);
    const r = await fetch(`${DAEMON_PROXY_BASE}/health`, { signal: ctrl.signal, credentials: 'omit' });
    clearTimeout(timer);
    return r.ok;
  } catch {
    return false;
  }
}

/**
 * Fetch remote text with CORS fallbacks and optional progress callbacks.
 *
 * onProgress({ loaded, total, pct, label, strategy })
 */
export async function fetchRemoteText(url, { onProgress, signal } = {}) {
  const rewritten = rewriteToCorsFriendlyUrl(url);
  const strategies = [];

  if (rewritten.strategy !== 'direct') {
    strategies.push({ name: rewritten.strategy, url: rewritten.url });
  }
  strategies.push({ name: 'direct', url });
  if (await tryDaemonReachable()) {
    strategies.push({
      name: 'daemon',
      url: `${DAEMON_PROXY_BASE}/proxy/fetch?url=${encodeURIComponent(url)}`,
    });
  }
  strategies.push({
    name: 'allorigins',
    url: `${ALLORIGINS_RAW}${encodeURIComponent(url)}`,
  });

  const seen = new Set();
  let lastError = null;

  for (const strat of strategies) {
    if (seen.has(strat.url)) continue;
    seen.add(strat.url);

    onProgress?.({
      loaded: 0,
      total: null,
      pct: null,
      label: `Trying ${strat.name}…`,
      strategy: strat.name,
    });

    try {
      const result = await fetchWithProgress(strat.url, {
        signal,
        onProgress: (p) => {
          onProgress?.({
            ...p,
            label: totalLabel(p, strat.name),
            strategy: strat.name,
          });
        },
      });
      onProgress?.({
        loaded: result.size,
        total: result.size,
        pct: 100,
        label: `Fetched ${formatBytes(result.size)} via ${strat.name}`,
        strategy: strat.name,
      });
      return { ...result, via: strat.name };
    } catch (error) {
      lastError = error;
      onProgress?.({
        loaded: 0,
        total: null,
        pct: null,
        label: `${strat.name} failed: ${error.message}`,
        strategy: strat.name,
      });
    }
  }

  throw lastError || new Error('All fetch strategies failed');
}

function totalLabel(p, strategy) {
  if (p.pct != null && p.total) {
    return `${strategy}: ${formatBytes(p.loaded)} / ${formatBytes(p.total)} (${p.pct.toFixed(0)}%)`;
  }
  return `${strategy}: ${formatBytes(p.loaded)}`;
}
