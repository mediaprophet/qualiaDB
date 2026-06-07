/**
 * Persistent WebSocket client for Qualia daemon /qualia-bridge.
 *
 * Protocol:
 *   ← { type: "HANDSHAKE_SUCCESS", payload: { mode, version } }
 *   → { type: "query", id, query, format: "metrics" }
 *   ← { type: "result", id, match_count, vm_cycles, direct_jump_ops, lexicon_lookup_ops }
 *   → { type: "bench_load", id, byte_length } then binary frame (dev daemon)
 *   ← { type: "bench_load_ready", id } then client sends binary
 *   ← { type: "bench_loaded", id, quin_count, graph_quin_count }
 */

const DEFAULT_HTTP_BASE = 'http://127.0.0.1:4242';

function wsBaseFromHttp(httpBase) {
  return httpBase.replace(/^http/, 'ws').replace(/\/$/, '');
}

export class QualiaWsClient {
  constructor(httpBase = DEFAULT_HTTP_BASE) {
    this.httpBase = httpBase;
    this.wsBase = wsBaseFromHttp(httpBase);
    this._ws = null;
    this._seq = 0;
    this._pending = new Map();
    this._connectPromise = null;
  }

  async connect(timeoutMs = 5000) {
    if (this._ws?.readyState === WebSocket.OPEN) return;
    if (this._connectPromise) return this._connectPromise;

    this._connectPromise = new Promise((resolve, reject) => {
      const ws = new WebSocket(`${this.wsBase}/qualia-bridge`);
      const timer = setTimeout(() => {
        ws.close();
        this._connectPromise = null;
        reject(new Error('WebSocket connect timeout'));
      }, timeoutMs);

      ws.onopen = () => {
        clearTimeout(timer);
        this._ws = ws;
        this._connectPromise = null;
        resolve();
      };

      ws.onerror = () => {
        clearTimeout(timer);
        this._connectPromise = null;
        reject(new Error('WebSocket connection failed'));
      };

      ws.onclose = () => {
        this._ws = null;
        for (const [, handlers] of this._pending) {
          handlers.reject(new Error('WebSocket closed'));
        }
        this._pending.clear();
      };

      ws.onmessage = (event) => this._onMessage(event);
    });

    return this._connectPromise;
  }

  _onMessage(event) {
    if (typeof event.data !== 'string') return;

    let frame;
    try {
      frame = JSON.parse(event.data);
    } catch {
      return;
    }

    const id = frame.id;
    if (id == null || !this._pending.has(id)) return;

    const handlers = this._pending.get(id);

    if (frame.type === 'bench_load_ready' && handlers.awaitingBinary) {
      try {
        this._ws.send(handlers.bytes);
      } catch (e) {
        this._pending.delete(id);
        handlers.reject(e);
      }
      return;
    }

    this._pending.delete(id);
    if (frame.type === 'error') {
      handlers.reject(new Error(frame.message || frame.code || 'daemon error'));
    } else {
      handlers.resolve(frame);
    }
  }

  _register(id, handlers, timeoutMs) {
    const timer = setTimeout(() => {
      if (!this._pending.has(id)) return;
      this._pending.delete(id);
      handlers.reject(new Error('WebSocket request timeout'));
    }, timeoutMs);
    this._pending.set(id, {
      ...handlers,
      resolve: (result) => {
        clearTimeout(timer);
        handlers.resolve(result);
      },
      reject: (err) => {
        clearTimeout(timer);
        handlers.reject(err);
      },
    });
  }

  async queryMetrics(query, { timeoutMs = 30000 } = {}) {
    await this.connect();
    const id = ++this._seq;
    const frame = await new Promise((resolve, reject) => {
      this._register(id, { resolve, reject }, timeoutMs);
      this._ws.send(JSON.stringify({ type: 'query', id, query, format: 'metrics' }));
    });
    return {
      matchCount: frame.match_count ?? 0,
      vmCycles: frame.vm_cycles ?? 0,
      directJumpOps: frame.direct_jump_ops ?? 0,
      lexiconLookupOps: frame.lexicon_lookup_ops ?? 0,
    };
  }

  async benchLoad(dbBytes, { timeoutMs = 120000 } = {}) {
    await this.connect();
    const id = ++this._seq;
    return new Promise((resolve, reject) => {
      this._register(
        id,
        { resolve, reject, awaitingBinary: true, bytes: dbBytes },
        timeoutMs,
      );
      this._ws.send(JSON.stringify({
        type: 'bench_load',
        id,
        byte_length: dbBytes.byteLength,
      }));
    });
  }

  close() {
    this._ws?.close();
    this._ws = null;
  }
}
