/**
 * daemon-client.js
 *
 * Live Qualia graph daemon client for installed Anatomy apps.
 * Reads qualia_token + qualia_daemon_port from host launch URL params.
 * Uses binary WebSocket connection to /qualia-bridge using McpIntentFrame.
 */

const QualiaDaemon = (() => {
  let host = "127.0.0.1";
  let port = null;
  let token = null;
  let appName = null;
  let ws = null;
  let isConnected = false;
  let responsePromises = new Map();
  let messageIdCounter = 0;

  function readLaunchAuth() {
    const params = new URLSearchParams(window.location.search);
    port = params.get("qualia_daemon_port");
    token = params.get("qualia_token");
    appName = params.get("qualia_qapp");

    // Token Scrubbing: Vaporize qualia_token from URL bar to prevent history leaks
    if (token && window.history && window.history.replaceState) {
      let cleanUrl = window.location.pathname;
      if (window.location.hash) {
        cleanUrl += window.location.hash;
      }
      window.history.replaceState(null, "", cleanUrl);
    }

    return {
      port,
      token: Boolean(token),
      appName
    };
  }

  function wsUrl() {
    if (!port) return null;
    return `ws://${host}:${port}/qualia-bridge?token=${token || ''}`;
  }

  async function connect() {
    if (ws && isConnected) return;
    const url = wsUrl();
    if (!url) throw new Error("Missing daemon port");

    return new Promise((resolve, reject) => {
      ws = new WebSocket(url);
      ws.binaryType = "arraybuffer";

      ws.onopen = () => {
        isConnected = true;
        resolve();
      };

      ws.onerror = (err) => {
        console.error("Daemon WS error:", err);
        reject(err);
      };

      ws.onclose = () => {
        isConnected = false;
      };

      ws.onmessage = (event) => {
        let data = event.data;
        // Decode response
        if (data instanceof ArrayBuffer) {
          if (window.Qualia && window.Qualia.getWasmApi() && window.Qualia.getWasmApi().unpack_mcp_response) {
            data = window.Qualia.getWasmApi().unpack_mcp_response(new Uint8Array(data));
          } else {
            const text = new TextDecoder().decode(data);
            try { data = JSON.parse(text); } catch (e) { data = text; }
          }
        } else if (typeof data === "string") {
          try { data = JSON.parse(data); } catch (e) {}
        }

        const msgId = data.msgId || data.id;
        if (msgId && responsePromises.has(msgId)) {
          if (data.error) {
            responsePromises.get(msgId).reject(new Error(data.error));
          } else {
            responsePromises.get(msgId).resolve(data);
          }
          responsePromises.delete(msgId);
        } else {
          console.log("Unsolicited or ghost state sync:", data);
        }
      };
    });
  }

  async function sendIntent(toolName, argumentsObj, overrides = {}) {
    if (!isConnected) await connect();

    const intentFrame = {
      purpose_hash: overrides.purpose_hash || 0,
      active_deontic_constraints: overrides.active_deontic_constraints || [],
      active_profile_id: overrides.active_profile_id || null,
      session_nonce: Math.floor(Math.random() * Number.MAX_SAFE_INTEGER),
      sanctuary_override: overrides.sanctuary_override || null,
      qpu_enabled: overrides.qpu_enabled !== undefined ? overrides.qpu_enabled : true,
    };

    const payload = {
      intent: intentFrame,
      tool_name: toolName,
      arguments_raw: argumentsObj // In JS this will be handled by WASM or stringified
    };

    const msgId = ++messageIdCounter;

    return new Promise((resolve, reject) => {
      responsePromises.set(msgId, { resolve, reject });

      try {
        let binaryPayload;
        const wasmApi = window.Qualia && window.Qualia.getWasmApi();
        if (wasmApi && wasmApi.pack_mcp_payload) {
          // Use WASM to pack into bincode for the daemon
          binaryPayload = wasmApi.pack_mcp_payload(
            JSON.stringify(intentFrame), 
            toolName, 
            JSON.stringify(argumentsObj), 
            msgId
          );
          ws.send(binaryPayload);
        } else {
          // Fallback to JSON if WASM is not available
          ws.send(JSON.stringify({ id: msgId, ...payload }));
        }
      } catch (err) {
        responsePromises.delete(msgId);
        reject(err);
      }
    });
  }

  async function safeSendIntent(toolName, args, overrides = {}) {
    try {
      return await sendIntent(toolName, args, overrides);
    } catch (err) {
      if (err.message && err.message.includes("SanctuaryGateTriggered")) {
        console.warn("Access Denied by Sanctuary Gate");
        // Trigger ghost state rollback UI event here
      } else if (err.message && err.message.includes("FeatureNotEnabled")) {
        console.warn("Feature not enabled. Using CPU fallback.");
      }
      throw err;
    }
  }

  async function health() {
    try {
      const res = await safeSendIntent("get_system_status", {});
      return { ok: true, data: res };
    } catch (e) {
      return { ok: false, reason: e.message };
    }
  }

  async function query(queryText, options = {}) {
    return await safeSendIntent("query_graph", {
      query: queryText,
      format: options.format || "json-ld"
    });
  }

  async function queryHealthConditions() {
    const queryText = "SELECT ?subject ?predicate ?object WHERE { ?subject ?predicate ?object }";
    return query(queryText, {
      format: "json-ld",
      accept: "application/ld+json"
    });
  }

  function getState() {
    return {
      connected: isConnected,
      port,
      appName,
      hasToken: Boolean(token)
    };
  }

  function initFromUrl() {
    return readLaunchAuth();
  }

  return {
    initFromUrl,
    connect,
    health,
    query,
    queryHealthConditions,
    sendIntent,
    safeSendIntent,
    getState
  };
})();

if (typeof window !== "undefined") {
  window.QualiaDaemon = QualiaDaemon;
}
