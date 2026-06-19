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

/**
 * Phase 4 AOT ingest: return a `.q42` container for `ggufUrl`, compiling once and caching the
 * RESULT in OPFS (the source `.gguf` is never stored). Warm starts read the `.q42` straight from
 * OPFS with zero network + zero compile. The cache is keyed on the wasm format `version`, so a
 * format bump auto-recompiles instead of booting a stale container.
 *
 * Conformance: the hot loop is untouched; the `.q42` write STREAMS to disk (no Cache.put); the
 * one-time GGUF buffer + compile is the cold-path ingest tier (LLM-load heap exception), freed
 * immediately. `compile` = the wasm `compileGgufToQ42`; `formatVersion` = wasm `q42FormatVersion()`.
 *
 * @returns {Promise<{bytes: Uint8Array, source: 'opfs-q42-hit'|'compiled'}>}
 */
export async function loadOrCompileQ42(ggufUrl, baseName, { compile, formatVersion, onProgress } = {}) {
  const safe = safeName(baseName) + '.q42';
  const part = safe + '.part';
  const root = await opfsRoot();

  // ── Warm: cached .q42 whose magic + format version match the current engine ──
  if (root) {
    try {
      const fh = await root.getFileHandle(safe);
      const file = await fh.getFile();
      if (file.size >= 8) {
        const head = new Uint8Array(await file.slice(0, 8).arrayBuffer());
        const magicOk = head[0] === 0x51 && head[1] === 0x34 && head[2] === 0x32 && head[3] === 0x57; // "Q42W"
        const ver = head[4] | (head[5] << 8);
        if (magicOk && ver === formatVersion) {
          const bytes = new Uint8Array(await file.arrayBuffer());
          if (onProgress) onProgress(bytes.length, bytes.length, 'q42-hit');
          return { bytes, source: 'opfs-q42-hit' };
        }
      }
      try { await root.removeEntry(safe); } catch {} // stale/foreign version → recompile
    } catch {
      /* miss → compile */
    }
  }

  // ── Cold: fetch GGUF (cold-path buffer), AOT-compile, stream .q42 to OPFS ──
  const resp = await fetch(ggufUrl);
  if (!resp.ok) throw new Error(`fetch ${resp.status}`);
  const total = Number(resp.headers.get('Content-Length')) || 0;
  let gguf;
  if (resp.body) {
    const reader = resp.body.getReader();
    const chunks = [];
    let loaded = 0;
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      chunks.push(value);
      loaded += value.length;
      if (onProgress) onProgress(loaded, total, 'download');
    }
    gguf = new Uint8Array(loaded);
    let o = 0;
    for (const c of chunks) { gguf.set(c, o); o += c.length; }
  } else {
    gguf = new Uint8Array(await resp.arrayBuffer());
  }

  if (onProgress) onProgress(0, 0, 'compile');
  const q42 = compile(gguf, 14); // wasm AOT compile (16 KB pages) → Uint8Array
  gguf = null; // free the GGUF cold-path buffer ASAP

  // Stream the .q42 to OPFS in chunks (no whole-blob Cache.put); atomic .part → move.
  if (root) {
    try {
      const partHandle = await root.getFileHandle(part, { create: true });
      const writable = await partHandle.createWritable();
      const CHUNK = 8 * 1024 * 1024;
      for (let off = 0; off < q42.length; off += CHUNK) {
        await writable.write(q42.subarray(off, Math.min(off + CHUNK, q42.length)));
      }
      await writable.close();
      if (typeof partHandle.move === 'function') {
        await partHandle.move(safe);
      } else {
        const fin = await root.getFileHandle(safe, { create: true });
        const w = await fin.createWritable();
        for (let off = 0; off < q42.length; off += CHUNK) {
          await w.write(q42.subarray(off, Math.min(off + CHUNK, q42.length)));
        }
        await w.close();
        try { await root.removeEntry(part); } catch {}
      }
    } catch (e) {
      console.warn('[q42-cache] OPFS write failed (recompile next load):', e && e.message ? e.message : e);
      try { const r = await opfsRoot(); if (r) await r.removeEntry(part); } catch {}
    }
  }
  if (onProgress) onProgress(q42.length, q42.length, 'compiled');
  return { bytes: q42, source: 'compiled' };
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
      if (/\.(gguf|q42)(\.part)?$/.test(n)) names.push(n);
    }
    for (const n of names) {
      try { await root.removeEntry(n); removed++; } catch {}
    }
  } catch {}
  return removed;
}
