const $ = (id) => document.getElementById(id);

async function loadStatus() {
  const res = await fetch("/api/status");
  if (!res.ok) throw new Error(`status ${res.status}`);
  return res.json();
}

async function loadConfig() {
  const res = await fetch("/api/config");
  if (!res.ok) throw new Error(`config ${res.status}`);
  return res.json();
}

async function loadJobs() {
  const res = await fetch("/api/jobs");
  if (!res.ok) throw new Error(`jobs ${res.status}`);
  return res.json();
}

function fillForm(cfg) {
  const form = $("config-form");
  for (const [key, value] of Object.entries(cfg)) {
    const el = form.elements.namedItem(key);
    if (el) el.value = value;
  }
}

function renderStatus(s) {
  $("origin-label").textContent = `127.0.0.1:${s.settings_port}`;
  $("st-settings").innerHTML = `<span class="ok">Running</span> on :${s.settings_port}`;
  const daemonCls = s.graph_daemon_reachable ? "ok" : "bad";
  const ver = s.graph_engine_version ? ` · ${s.graph_engine_version}` : "";
  $("st-daemon").innerHTML =
    `<span class="${daemonCls}">${s.graph_daemon_reachable ? "Reachable" : "Unreachable"}</span> ` +
    `127.0.0.1:${s.graph_daemon_port}${ver}`;
  $("st-qapps").textContent =
    s.qapps_protocol_port ? `http://127.0.0.1:${s.qapps_protocol_port}/` : "Not started";
  if (s.job_queue) {
    const jq = s.job_queue;
    $("st-jobs").textContent =
      `${jq.queued} queued · ${jq.running} running · ${jq.completed} done · ${jq.failed} failed`;
  }
  $("link-daemon-health").href = `http://127.0.0.1:${s.graph_daemon_port}/health`;
}

function kindLabel(kind) {
  if (!kind || !kind.kind) return "unknown";
  return kind.kind.replace(/_/g, " ");
}

function renderJobs(snapshot) {
  const body = $("jobs-body");
  const jobs = (snapshot.jobs || []).slice().reverse();
  if (!jobs.length) {
    body.innerHTML = '<tr><td colspan="5">No jobs yet.</td></tr>';
    return;
  }
  body.innerHTML = jobs.map((job) => {
    const pct = Math.round((job.progress || 0) * 100);
    const cancelBtn =
      job.status === "queued" || job.status === "running"
        ? `<button type="button" data-cancel="${job.id}">Cancel</button>`
        : "";
    return `<tr>
      <td class="status-${job.status}">${job.status}</td>
      <td>${kindLabel(job.kind)}</td>
      <td>${job.message || ""}${job.error ? ` — ${job.error}` : ""}</td>
      <td>${pct}%</td>
      <td>${cancelBtn}</td>
    </tr>`;
  }).join("");
  body.querySelectorAll("[data-cancel]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      await fetch(`/api/jobs/${btn.dataset.cancel}/cancel`, { method: "POST" });
      await refreshJobs();
    });
  });
}

function syncJobFormFields() {
  const kind = $("job-kind").value;
  $("job-ontology-wrap").hidden = kind !== "ontology_catalog_import";
  $("job-uri-wrap").hidden = kind !== "ontology_uri_import";
}

function buildJobPayload(form) {
  const kind = form.job_kind.value;
  switch (kind) {
    case "ontology_catalog_import":
      return {
        kind,
        ontology_id: form.ontology_id.value.trim(),
      };
    case "ontology_uri_import":
      return {
        kind,
        uri: form.uri.value.trim(),
        ontology_id: form.ontology_id.value.trim() || null,
      };
    case "bundled_ontology_seed":
      return {
        kind,
        ontology_id: form.ontology_id.value.trim() || null,
      };
    case "workbench_daemon_sync":
    case "daemon_graph_reload":
      return { kind };
    default:
      throw new Error(`Unknown job kind: ${kind}`);
  }
}

async function refreshJobs() {
  const snapshot = await loadJobs();
  renderJobs(snapshot);
  const status = await loadStatus();
  renderStatus(status);
}

function wireTelemetry() {
  const log = $("telemetry-log");
  const es = new EventSource("/telemetry");
  es.onmessage = (e) => {
    const line = e.data.trim();
    log.textContent = (log.textContent + "\n" + line).trim().split("\n").slice(-8).join("\n");
  };
  es.onerror = () => { log.textContent = "Telemetry stream disconnected."; };
}

$("job-kind").addEventListener("change", syncJobFormFields);
syncJobFormFields();

$("job-form").addEventListener("submit", async (ev) => {
  ev.preventDefault();
  const payload = buildJobPayload(ev.target);
  if (payload.kind === "ontology_catalog_import" && !payload.ontology_id) {
    alert("Ontology ID is required.");
    return;
  }
  if (payload.kind === "ontology_uri_import" && !payload.uri) {
    alert("URI is required.");
    return;
  }
  const res = await fetch("/api/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  if (!res.ok) {
    const text = await res.text();
    alert(`Enqueue failed: ${text}`);
    return;
  }
  await refreshJobs();
});

$("refresh-jobs").addEventListener("click", () => refreshJobs().catch(console.error));

$("config-form").addEventListener("submit", async (ev) => {
  ev.preventDefault();
  const form = ev.target;
  const body = {
    storage_path: form.storage_path.value,
    storage_quota_gb: Number(form.storage_quota_gb.value),
    daemon_host: form.daemon_host.value,
    daemon_port: Number(form.daemon_port.value),
    inference_backend: form.inference_backend.value,
    base_connectivity_cost_ilp: Number(form.base_connectivity_cost_ilp.value),
  };
  const msg = $("save-msg");
  msg.textContent = "Saving…";
  const res = await fetch("/api/config", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  msg.textContent = res.ok ? "Saved." : `Failed (${res.status})`;
  if (res.ok) await refresh();
});

async function refresh() {
  const [status, config] = await Promise.all([loadStatus(), loadConfig()]);
  renderStatus(status);
  fillForm(config);
  await refreshJobs();
}

refresh().catch((err) => {
  $("st-settings").innerHTML = `<span class="bad">${err.message}</span>`;
});
wireTelemetry();
setInterval(() => refreshJobs().catch(() => {}), 4000);