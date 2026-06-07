/**
 * daemon-client.js
 *
 * Live Qualia graph daemon client for installed Anatomy apps.
 * Reads qualia_token + qualia_daemon_port from host launch URL params.
 */

const QualiaDaemon = (() => {
  let host = "127.0.0.1";
  let port = null;
  let token = null;
  let appName = null;
  let lastHealth = null;
  let lastQuery = null;

  function readLaunchAuth() {
    const params = new URLSearchParams(window.location.search);
    port = params.get("qualia_daemon_port");
    token = params.get("qualia_token");
    appName = params.get("qualia_qapp");
    return {
      port,
      token: Boolean(token),
      appName
    };
  }

  function baseUrl() {
    if (!port) return null;
    return `http://${host}:${port}`;
  }

  async function health() {
    const url = baseUrl();
    if (!url) {
      return { ok: false, reason: "missing_daemon_port" };
    }

    try {
      const response = await fetch(`${url}/health`);
      if (!response.ok) {
        return { ok: false, reason: `http_${response.status}` };
      }
      lastHealth = await response.json();
      return { ok: true, data: lastHealth };
    } catch (err) {
      return { ok: false, reason: err.message };
    }
  }

  async function query(queryText, options = {}) {
    const url = baseUrl();
    if (!url) throw new Error("Daemon port unavailable");
    if (!token) throw new Error("Missing qualia_token from host launch");

    const response = await fetch(`${url}/query`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Accept": options.accept || "application/ld+json",
        "X-Qualia-Token": token
      },
      body: JSON.stringify({
        query: queryText,
        format: options.format || "json-ld"
      })
    });

    const body = await response.text();
    if (!response.ok) {
      throw new Error(`Daemon query failed (${response.status}): ${body.slice(0, 240)}`);
    }

    let parsed = body;
    try {
      parsed = JSON.parse(body);
    } catch (_) {
      // n-triples or plain text remains as string
    }

    lastQuery = {
      status: response.status,
      query: queryText,
      result: parsed
    };
    return lastQuery;
  }

  async function queryHealthConditions() {
    // Qualia daemon compiles N-Triples patterns, not SPARQL.
    const queryText = "?subject ?predicate ?object .";
    return query(queryText, {
      format: "json-ld",
      accept: "application/ld+json"
    });
  }

  function getState() {
    return {
      connected: Boolean(port && token),
      port,
      appName,
      hasToken: Boolean(token),
      lastHealth,
      lastQuery
    };
  }

  function initFromUrl() {
    return readLaunchAuth();
  }

  return {
    initFromUrl,
    health,
    query,
    queryHealthConditions,
    getState
  };
})();

if (typeof window !== "undefined") {
  window.QualiaDaemon = QualiaDaemon;
}
