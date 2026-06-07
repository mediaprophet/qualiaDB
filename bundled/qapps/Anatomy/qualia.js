/**
 * qualia.js
 *
 * QualiaDB bridge for Anatomy: WASM when available, JS knowledge parsing fallback.
 */

const Qualia = (() => {
  let wasmApi = null;
  let isInitialized = false;
  let initMode = "none";
  let logCallback = null;

  function log(message) {
    const timestamp = new Date().toLocaleTimeString();
    const fullMsg = `${timestamp} — ${message}`;
    if (logCallback) logCallback(fullMsg);
    else console.log("[Qualia]", fullMsg);
  }

  function setLogCallback(callback) {
    logCallback = callback;
  }

  async function tryLoadWasm(options = {}) {
    const glueCandidates = [
      options.wasmGlueUrl,
      "./wasm/qualia_core_db.js",
      "https://mediaprophet.github.io/qualiaDB/playground/qualia_core_db.js"
    ].filter(Boolean);

    for (const glueUrl of glueCandidates) {
      try {
        const mod = await import(glueUrl);
        if (typeof mod.default !== "function") continue;
        const wasmUrl =
          options.wasmUrl ||
          glueUrl.replace(/\.js$/, "_bg.wasm");
        await mod.default(wasmUrl);
        if (typeof mod.parse_turtle_wasm === "function") {
          log(`WASM loaded from ${glueUrl}`);
          return mod;
        }
      } catch (err) {
        log(`WASM candidate failed (${glueUrl}): ${err.message}`);
      }
    }
    return null;
  }

  async function init(options = {}) {
    if (isInitialized) return true;

    log("Initializing Qualia bridge...");
    wasmApi = await tryLoadWasm(options);
    initMode = wasmApi ? "wasm" : "js-fallback";
    isInitialized = true;
    log(`Qualia bridge ready (${initMode})`);
    return true;
  }

  async function parseConditionsTtl(text) {
    if (!window.KnowledgeParser) {
      throw new Error("knowledge-parser.js is not loaded");
    }
    if (!isInitialized) await init();
    return window.KnowledgeParser.parseConditionsTtl(text, wasmApi);
  }

  async function loadConditionsFromUrl(url = "./Knowledge/conditions.ttl") {
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`Could not fetch ${url}`);
    }
    const text = await response.text();
    return parseConditionsTtl(text);
  }

  function insertSpatialEntity(entityData) {
    const enriched = {
      type: "SpatialEntity",
      source: "HRA-3D-Reference",
      timestamp: new Date().toISOString(),
      ...entityData
    };
    log(`Spatial entity recorded: ${enriched.name || "unnamed"}`);
    return enriched;
  }

  function getStatus() {
    return {
      initialized: isInitialized,
      mode: initMode,
      wasm: Boolean(wasmApi),
      exports: wasmApi
        ? Object.keys(wasmApi).filter(key => typeof wasmApi[key] === "function")
        : []
    };
  }

  function getWasmApi() {
    return wasmApi;
  }

  return {
    init,
    parseConditionsTtl,
    loadConditionsFromUrl,
    insertSpatialEntity,
    getStatus,
    getWasmApi,
    setLogCallback
  };
})();

if (typeof window !== "undefined") {
  window.Qualia = Qualia;
}

if (typeof module !== "undefined" && module.exports) {
  module.exports = Qualia;
}
