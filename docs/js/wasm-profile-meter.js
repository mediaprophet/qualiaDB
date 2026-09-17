/**
 * Measure GitHub Pages WASM artifacts against the 0.0.38 size gates.
 * Gzip uses CompressionStream when the browser supports it.
 */

export const GATES = {
  ontology: { raw: 655360, gzip: 204800, label: "640 KiB / 200 KiB" },
  engine: { raw: 16777216, gzip: 4194304, label: "16 MiB / 4 MiB" },
};

export const ARTIFACTS = [
  {
    id: "lite",
    name: "Ontology MCP",
    product: "webizen-lite-wasm",
    path: "pkg/webizen-lite/webizen_lite_wasm_bg.wasm",
    gate: "ontology",
    includes: "MCP JSON-RPC, N3, Quin query, SHACL, modal kernels",
    excludes: "Portal, WebGPU, science, LLM",
  },
  {
    id: "portal",
    name: "Portal",
    product: "--features portal",
    path: "pkg/qualia/qualia_bg.wasm",
    gate: "engine",
    includes: "Viewport, acoustic, logic, WASM-safe science",
    excludes: "Daemon, NVMe/ZNS, BLE, eBPF, LLM",
  },
  {
    id: "playground",
    name: "Playground",
    product: "--features wasm-full",
    path: "playground/qualia_core_db_bg.wasm",
    gate: "engine",
    includes: "Portal + science + browser LLM",
    excludes: "Native-only filesystem / mesh / eBPF",
  },
];

export function formatBytes(n) {
  if (n == null || Number.isNaN(n)) return "—";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KiB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MiB`;
}

export function pctOf(n, cap) {
  if (!n || !cap) return 0;
  return Math.min(100, (n / cap) * 100);
}

async function gzipLength(buffer) {
  if (typeof CompressionStream === "undefined") return null;
  const stream = new Blob([buffer]).stream().pipeThrough(new CompressionStream("gzip"));
  const out = await new Response(stream).arrayBuffer();
  return out.byteLength;
}

export async function measureWasm(url) {
  const res = await fetch(url, { cache: "no-store" });
  if (!res.ok) {
    throw new Error(`${res.status} ${url}`);
  }
  const buf = await res.arrayBuffer();
  const gzip = await gzipLength(buf);
  return { raw: buf.byteLength, gzip, url };
}

export function docsRootFromPage() {
  const script = document.querySelector('script[src*="wasm-profile-meter.js"]');
  if (script) {
    const src = script.getAttribute("src") || "";
    if (src.includes("/")) return src.replace(/js\/wasm-profile-meter\.js.*$/, "");
  }
  const pagesBase = window.location.pathname.match(/^(.*\/qualiaDB\/)/);
  return pagesBase ? pagesBase[1] : "";
}
