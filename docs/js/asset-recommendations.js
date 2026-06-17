/**
 * Browser-side device profiling + catalog scoring for Design Studio.
 * Authoritative recommendations when desktop portal :8080 is reachable.
 */

export function detectBrowserDevice() {
  const nav = typeof navigator !== "undefined" ? navigator : {};
  const ram_gb = nav.deviceMemory ? Number(nav.deviceMemory) : 8;
  const cpu_cores = nav.hardwareConcurrency || 4;
  let tier = "mainstream";
  if (ram_gb < 6) tier = "edge";
  else if (ram_gb >= 16) tier = "high_performance";

  return {
    ram_gb,
    has_webgpu: false,
    cpu_cores,
    platform: "browser",
    tier,
    source: "navigator.deviceMemory",
  };
}

export async function probeWebGpu() {
  try {
    if (!navigator.gpu) return false;
    const adapter = await navigator.gpu.requestAdapter();
    return !!adapter;
  } catch {
    return false;
  }
}

export function inferDomains(text) {
  const lower = (text || "").toLowerCase();
  const rules = [
    ["product", ["design", "product", "assembly", "module", "switch", "socket", "device"]],
    ["electrical", ["electric", "power", "mains", "wiring", "powerpoint"]],
    ["iot", ["sensor", "wifi", "smart", "mcu", "home"]],
    ["health", ["medical", "anatomy", "clinical", "dicom"]],
    ["legal", ["contract", "rights", "policy"]],
    ["geography", ["map", "location", "geo", "building"]],
    ["linguistics", ["word", "language", "lexicon"]],
  ];
  const out = [];
  for (const [domain, kws] of rules) {
    if (kws.some((k) => lower.includes(k))) out.push(domain);
  }
  return out.length ? out : ["general"];
}

function llmFits(ram_gb, need_mb) {
  return ram_gb * 1024 * 0.72 >= need_mb;
}

export function recommendFromCatalog(catalog, device, prompt) {
  const domains = inferDomains(prompt);
  const llms = (catalog.llms || [])
    .map((llm) => {
      const need = llm.ram_estimate_mb || llm.size_mb || 9999;
      if (!llmFits(device.ram_gb, need)) return null;
      let priority = 40;
      const rec = llm.recommended_for || [];
      const reasons = [];
      if (device.tier === "edge" && rec.some((r) => r === "edge" || r === "very_low_ram")) {
        priority += 30;
        reasons.push("edge tier");
      }
      if (need <= 1500) {
        priority += 10;
        reasons.push("lightweight");
      }
      if (device.has_webgpu && need <= 1200) {
        priority += 8;
        reasons.push("browser-friendly");
      }
      return {
        kind: "llm",
        id: llm.id,
        name: llm.name,
        reason: reasons.join("; ") || "catalog fit",
        size_mb: llm.size_mb || 0,
        ram_estimate_mb: need,
        priority: Math.min(100, priority),
        install: {
          cli_hint: `qualia resources import llm ${llm.id}`,
          native_note: "Desktop: LLM Hub installs GGUF to storage; WASM demos can use the same catalog id.",
        },
      };
    })
    .filter(Boolean)
    .sort((a, b) => b.priority - a.priority)
    .slice(0, 4);

  const maxOntMb = device.tier === "edge" ? 3 : device.tier === "mainstream" ? 15 : 64;
  const ontologies = (catalog.ontologies || [])
    .map((ont) => {
      const size = ont.size_estimate_mb || 1;
      if (size > maxOntMb) return null;
      const isCore = (ont.tags || []).includes("core");
      const domainHit =
        isCore || domains.includes(ont.domain) || (ont.tags || []).some((t) => domains.includes(t));
      if (!domainHit) return null;
      let priority = isCore ? 70 : 45;
      const reasons = [];
      if (isCore) reasons.push("core");
      if (domains.includes(ont.domain)) reasons.push("domain match");
      if (size < 1) reasons.push("small");
      return {
        kind: "ontology",
        id: ont.id,
        name: ont.name,
        reason: reasons.join("; "),
        size_mb: size,
        priority: Math.min(100, priority),
        install: {
          kind: "ontology_catalog_import",
          job_payload: { kind: "ontology_catalog_import", ontology_id: ont.id },
          cli_hint: `qualia resources import ontology ${ont.id}`,
          native_note: "POST :8080/api/assets/enqueue when settings portal is running.",
        },
      };
    })
    .filter(Boolean)
    .sort((a, b) => b.priority - a.priority)
    .slice(0, 6);

  return { device, inferred_domains: domains, llms, ontologies };
}

export async function fetchPortalRecommendations(prompt, portalBase = "") {
  const bases = [
    portalBase,
    "",
    "http://127.0.0.1:8080",
  ].filter((v, i, a) => a.indexOf(v) === i);

  const device = detectBrowserDevice();
  device.has_webgpu = await probeWebGpu();

  for (const base of bases) {
    try {
      const url = `${base}/api/assets/recommend`;
      const res = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          device: {
            ram_gb: device.ram_gb,
            has_webgpu: device.has_webgpu,
            cpu_cores: device.cpu_cores,
            platform: device.platform,
          },
          design: { prompt, domains: inferDomains(prompt), keywords: [] },
        }),
      });
      if (!res.ok) continue;
      const data = await res.json();
      data.source = "native_portal";
      return data;
    } catch {
      /* try next base */
    }
  }
  return null;
}

export async function loadDocsCatalog() {
  const res = await fetch("resources/asset-catalog.json");
  if (!res.ok) throw new Error("asset catalog missing");
  return res.json();
}

export async function enqueueOntologyInstall(ontologyId, portalBase = "http://127.0.0.1:8080") {
  const res = await fetch(`${portalBase}/api/assets/enqueue`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ kind: "ontology_catalog_import", ontology_id: ontologyId }),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}