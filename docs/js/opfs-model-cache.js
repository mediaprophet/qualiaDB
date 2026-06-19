// OPFS-backed GGUF model cache (Phase 3).
//
// Streams the `fetch` response directly to the Origin Private File System via a
// FileSystemWritableFileStream, so >250MB GGUF models cache reliably where the
// Cache Storage API's `put` fails — and without ever buffering the whole file in
// the JS heap during download. The WASM engine contract is unchanged: this returns
// a Uint8Array for `initialize_webgpu_engine`.
//
// Atomicity: the download writes to `<name>.part`; the final `<name>` only appears
// after the streamed byte count matches Content-Length and `.part` is promoted via
// FileSystemFileHandle.move(). So the mere existence of `<name>` means "complete".
//
// Best-effort: any OPFS failure (quota, unsupported, interrupted) falls back to a
// plain buffered network fetch — model loading never blocks.

function safeName(name) {
  return name.replace(/[^A-Za-z0-9._-]/g, '_');
}

async function opfsRoot() {
  if (!navigator || !navigator.storage || !navigator.storage.getDirectory) return null;
  try {
    return await navigator.storage.getDirectory();
  } catch {
    return null;
  }
}

async function readHandle(handle) {
  const file = await handle.getFile();
  return { size: file.size, bytes: new Uint8Array(await file.arrayBuffer()) };
}

async function bufferedFetch(url, onProgress) {
  const resp = await fetch(url);
  if (!resp.ok) throw new Error(`fetch ${resp.status}`);
  const bytes = new Uint8Array(await resp.arrayBuffer());
  if (onProgress) onProgress(bytes.length, bytes.length, 'done');
  return { bytes, source: 'network' };
}

/**
 * Load a GGUF model, caching it in OPFS. Returns { bytes, source }.
 * @param {string} url             model URL (same-origin /models/*.gguf)
 * @param {string} name            cache key (model basename)
 * @param {number} [expectedSize]  optional integrity check for the cache-hit path
 * @param {(loaded:number,total:number,phase:'hit'|'download'|'done')=>void} [onProgress]
 */
export async function loadGgufCached(url, name, expectedSize, onProgress) {
  const safe = safeName(name);
  const part = safe + '.part';
  const root = await opfsRoot();

  // ── Cache hit: final file exists ⟺ a prior download completed (atomic promote) ──
  if (root) {
    try {
      const fh = await root.getFileHandle(safe);
      const { size, bytes } = await readHandle(fh);
      if (!expectedSize || size === expectedSize) {
        if (onProgress) onProgress(size, size, 'hit');
        return { bytes, source: 'opfs-hit' };
      }
      try { await root.removeEntry(safe); } catch {} // stale size → re-fetch
    } catch {
      /* miss → fall through */
    }
  }

  // ── Cache miss with OPFS streaming write ──
  if (root) {
    try {
      const resp = await fetch(url);
      if (!resp.ok) throw new Error(`fetch ${resp.status}`);
      if (!resp.body) throw new Error('no response body stream');
      const total = Number(resp.headers.get('Content-Length')) || expectedSize || 0;

      const partHandle = await root.getFileHandle(part, { create: true });
      const writable = await partHandle.createWritable();
      let loaded = 0;
      const counter = new TransformStream({
        transform(chunk, controller) {
          loaded += chunk.byteLength;
          if (onProgress) onProgress(loaded, total, 'download');
          controller.enqueue(chunk);
        },
      });
      // Streams chunks straight to disk with backpressure; closes `writable` on completion.
      await resp.body.pipeThrough(counter).pipeTo(writable);

      // Integrity gate: promote only if the streamed byte count matches Content-Length.
      if (total && loaded !== total) {
        try { await root.removeEntry(part); } catch {}
        throw new Error(`incomplete download ${loaded}/${total}`);
      }

      if (typeof partHandle.move === 'function') {
        await partHandle.move(safe); // atomic .part → final
      } else {
        // Fallback for browsers without FileSystemFileHandle.move().
        const { bytes } = await readHandle(partHandle);
        const fin = await root.getFileHandle(safe, { create: true });
        const w = await fin.createWritable();
        await w.write(bytes);
        await w.close();
        try { await root.removeEntry(part); } catch {}
      }

      const { size, bytes } = await readHandle(await root.getFileHandle(safe));
      if (onProgress) onProgress(size, size, 'done');
      return { bytes, source: 'opfs-miss' };
    } catch (e) {
      try {
        const r = await opfsRoot();
        if (r) await r.removeEntry(part);
      } catch {}
      console.warn('[opfs-cache] streaming write failed; buffered-fetch fallback:', e && e.message ? e.message : e);
      return await bufferedFetch(url, onProgress);
    }
  }

  // ── No OPFS available ──
  return await bufferedFetch(url, onProgress);
}

/** Remove a single cached model (and any stale .part). */
export async function clearOpfsModel(name) {
  const root = await opfsRoot();
  if (!root) return 0;
  const safe = safeName(name);
  let removed = 0;
  for (const n of [safe, safe + '.part']) {
    try { await root.removeEntry(n); removed++; } catch {}
  }
  return removed;
}

/** Remove all OPFS-cached GGUF models (best-effort). Returns count removed. */
export async function clearAllOpfsModels() {
  const root = await opfsRoot();
  if (!root || !root.entries) return 0;
  let removed = 0;
  try {
    const names = [];
    for await (const [n] of root.entries()) {
      if (n.endsWith('.gguf') || n.endsWith('.gguf.part')) names.push(n);
    }
    for (const n of names) {
      try { await root.removeEntry(n); removed++; } catch {}
    }
  } catch {}
  return removed;
}
