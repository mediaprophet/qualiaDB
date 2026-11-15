// QualiaPortal host for the anatomy playground.
// Whole-body pills still read the frozen welfare HMC.
// Organ picker / layer peels load working encyclopedia CAD .10d files.

import { loadQualiaPortal, startPortalLoop, stopPortalLoop } from "../js/qualia-shell.js";

const HMC_URL = "anatomy-studio/human-welfare.hmc";
const EXPORT_ROOT = "anatomy-studio/export/";
const CATALOG_URL = "anatomy-studio/working/encyclopedia-catalog.json";
const CAD_ROOT = "anatomy-studio/working/cad";
const WORKING_HEART_10D = "anatomy-studio/working/heart-exterior.10d";

const PROFILES = {
  male: { slug: "male", week: 0, sex: "male", label: "Adult Male (XY)" },
  female: { slug: "female", week: 0, sex: "female", label: "Adult Female (XX)" },
  female_trimester_1: { slug: "female_trimester_1", week: 10, sex: "female", label: "Pregnancy T1 (W10)" },
  female_trimester_2: { slug: "female_trimester_2", week: 22, sex: "female", label: "Pregnancy T2 (W22)" },
  female_trimester_3: { slug: "female_trimester_3", week: 38, sex: "female", label: "Pregnancy T3 (W38)" },
  female_week_34: { slug: "female_week_34", week: 34, sex: "female", label: "Pregnancy W34" },
};

const LAYER_SUFFIX = {
  all: "",
  surface: "",
  skeletal: "_skeleton",
  muscular: "_ecorche",
  cardiopulmonary: "",
  visceral: "",
  neuro_endocrine: "",
  maternal_fetal: "",
  sigma_xray: "",
};

const LAYER_SYSTEMS = {
  all: null,
  surface: ["integumentary"],
  skeletal: ["skeletal"],
  muscular: ["muscular"],
  cardiopulmonary: ["circulatory", "respiratory"],
  visceral: ["digestive", "urinary", "immune_lymphatic"],
  neuro_endocrine: ["nervous", "endocrine", "sensory", "glymphatic", "ecs"],
  maternal_fetal: ["reproductive"],
  sigma_xray: null,
};

const LAYER_ORGAN = {
  all: null,
  surface: "skin_torso",
  skeletal: "spine",
  muscular: "pectoralis",
  cardiopulmonary: "heart",
  visceral: "liver",
  neuro_endocrine: "brain",
  maternal_fetal: "uterus",
  sigma_xray: "heart",
};

const LAYER_LABEL = {
  all: "Composite",
  surface: "Skin / Surface",
  skeletal: "Skeletal",
  muscular: "Muscular écorché",
  cardiopulmonary: "Cardiopulmonary",
  visceral: "Visceral GI/Renal",
  neuro_endocrine: "Neuro-endocrine",
  maternal_fetal: "Maternal-fetal",
  sigma_xray: "OSCE σ-burden",
};

const SYSTEM_COLOR = {
  circulatory: [0.72, 0.22, 0.24, 1],
  respiratory: [0.86, 0.58, 0.60, 1],
  digestive: [0.62, 0.42, 0.28, 1],
  nervous: [0.86, 0.78, 0.62, 1],
  muscular: [0.70, 0.28, 0.28, 1],
  skeletal: [0.90, 0.86, 0.78, 1],
  endocrine: [0.85, 0.72, 0.32, 1],
  immune_lymphatic: [0.55, 0.22, 0.22, 1],
  integumentary: [0.82, 0.62, 0.50, 1],
  urinary: [0.55, 0.32, 0.28, 1],
  reproductive: [0.78, 0.42, 0.50, 1],
  sensory: [0.92, 0.92, 0.95, 1],
  vestibular: [0.70, 0.62, 0.85, 1],
  exocrine: [0.72, 0.58, 0.32, 1],
  ecs: [0.55, 0.70, 0.62, 1],
  ens: [0.62, 0.48, 0.32, 1],
  glymphatic: [0.55, 0.72, 0.82, 1],
};

const SYSTEM_LABEL = {
  circulatory: "Circulatory",
  respiratory: "Respiratory",
  digestive: "Digestive",
  nervous: "Nervous",
  muscular: "Muscular",
  skeletal: "Skeletal",
  endocrine: "Endocrine",
  immune_lymphatic: "Immune / lymphatic",
  integumentary: "Integumentary",
  urinary: "Urinary",
  reproductive: "Reproductive",
  sensory: "Sensory",
  vestibular: "Vestibular",
  exocrine: "Exocrine",
  ecs: "ECS",
  ens: "ENS",
  glymphatic: "Glymphatic",
};

const CAMERA_PRESETS = {
  hero: { yaw: 0.72, pitch: 0.32, zoom: 3.5 },
  anterior: { yaw: 0.0, pitch: 0.12, zoom: 3.6 },
  lateral: { yaw: Math.PI / 2, pitch: 0.08, zoom: 3.6 },
  pelvic: { yaw: 0.35, pitch: -0.55, zoom: 2.6 },
  cranial: { yaw: 0.28, pitch: 1.05, zoom: 2.4 },
};

const PERSON_BOUNDARY =
  "This is a general picture to help you get a sense of things — it is not medical advice or a diagnosis. If anything here concerns you, it's worth discussing with a clinician.";
const CLINICIAN_BOUNDARY =
  "Decision-support considerations only — not a diagnosis, a test order, or a prescription. Surfaced from the person's own records for the clinician's independent evaluation (OSCE-Prac aid).";

let portal = null;
let liveCanvas = null;
let hmcBytes = null;
let catalog = { members: [], systems: [] };
let profile = "male";
let layer = "all";
let lens = "clinician";
let osceCase = 0;
let meshMode = "body";
let selectedOrgan = null;
let partQuery = "";
let autoSpin = false;
let dragging = false;
let cam = { ...CAMERA_PRESETS.hero };

function setStatus(text, kind) {
  const el = document.getElementById("status");
  if (!el) return;
  el.textContent = text;
  el.dataset.kind = kind || "";
  el.classList.toggle("error", kind === "err");
  el.classList.toggle("ok", kind === "ok");
}

function assetStem() {
  const slug = PROFILES[profile].slug;
  const suffix = LAYER_SUFFIX[layer] || "";
  return `human/${slug}${suffix}`;
}

function meshKey() {
  return `outputs/${assetStem()}.10d`;
}

function organCadUrl(id) {
  return `${CAD_ROOT}/${id}/${id}.10d`;
}

function organColor(member) {
  const sys = member?.system?.[0];
  return SYSTEM_COLOR[sys] || [0.72, 0.72, 0.74, 1];
}

function applyCamera() {
  if (!portal?.set_camera) return;
  try {
    portal.set_camera(cam.yaw, cam.pitch, cam.zoom);
  } catch (err) {
    console.warn("set_camera", err);
  }
}

function syncCameraFromPortal() {
  if (!portal) return;
  try {
    if (typeof portal.camera_yaw === "function") cam.yaw = portal.camera_yaw();
    if (typeof portal.camera_pitch === "function") cam.pitch = portal.camera_pitch();
    if (typeof portal.camera_zoom === "function") cam.zoom = portal.camera_zoom();
  } catch {
    /* keep local cam */
  }
}

function lightScene() {
  try {
    portal?.set_lighting?.(0.45, 0.85, 0.40, 1.45, 0.42);
    portal?.set_clear_color?.(0.027, 0.051, 0.047, 1);
    portal?.set_ambient_enabled?.(false);
  } catch {
    /* optional */
  }
}

function markActive(selector, id) {
  document.querySelectorAll(selector).forEach((node) => {
    node.classList.toggle("active", node.id === id);
  });
}

function markLayerPills(mode) {
  document.querySelectorAll("[data-layer-pill]").forEach((node) => {
    node.classList.toggle("active", node.getAttribute("data-layer-pill") === mode);
  });
  const sidebarIds = {
    all: "layer-all",
    sigma_xray: "layer-sigma",
    surface: "layer-surface",
    muscular: "layer-muscular",
    skeletal: "layer-skeletal",
    cardiopulmonary: "layer-cardiopulm",
    visceral: "layer-visceral",
    neuro_endocrine: "layer-neuro",
    maternal_fetal: "layer-maternal",
  };
  Object.entries(sidebarIds).forEach(([key, id]) => {
    document.getElementById(id)?.classList.toggle("active", key === mode);
  });
  const badge = document.getElementById("active-layer-badge");
  if (badge) badge.textContent = LAYER_LABEL[mode] || mode;
}

function markCamPreset(name) {
  document.querySelectorAll(".cam-bar .cam-btn").forEach((node) => {
    const onclick = node.getAttribute("onclick") || "";
    const match = onclick.match(/setCameraPreset\('([^']+)'\)/);
    node.classList.toggle("active", match?.[1] === name);
  });
}

async function fetchBytes(url) {
  const response = await fetch(`${url}?t=${Date.now()}`, { cache: "no-store" });
  if (!response.ok) throw new Error(`${url} (${response.status})`);
  return new Uint8Array(await response.arrayBuffer());
}

function receiptTriangles(receipt) {
  return receipt?.triangle_count ?? receipt?.total_triangles ?? receipt?.triangles ?? "";
}

function findMember(id) {
  return catalog.members.find((m) => m.id === id) || { id, system: [] };
}

async function loadCadOrgan(id, { heartExterior = false } = {}) {
  if (!portal) return;
  const url = heartExterior ? WORKING_HEART_10D : organCadUrl(id);
  setStatus(`Loading ${url}`, "");
  const bytes = await fetchBytes(url);
  const member = findMember(id);
  const [r, g, b, a] = organColor(member);
  let receipt;
  if (typeof portal.load_10d_colored === "function") {
    receipt = portal.load_10d_colored(bytes, r, g, b, a);
  } else {
    receipt = portal.load_10d(bytes);
  }
  lightScene();
  applyCamera();
  const tris = receiptTriangles(receipt);
  const badge = document.getElementById("top-container-badge");
  if (badge) badge.textContent = heartExterior ? "Working heart · QualiaPortal" : "Encyclopedia CAD · QualiaPortal";
  setStatus(`${url} · ${tris} triangles · QualiaPortal.load_10d`, "ok");
  selectedOrgan = id;
  meshMode = heartExterior ? "heart" : "organ";
  updateOrganCard(member, heartExterior);
  renderPartsList();
}

async function loadBodyMesh() {
  if (!portal || !hmcBytes) return;
  const key = meshKey();
  setStatus(`Loading ${key}`, "");
  const receipt = await portal.load_hmc_mesh(hmcBytes, key);
  lightScene();
  applyCamera();
  const tris = receiptTriangles(receipt);
  setStatus(`${key} · ${tris} triangles · QualiaPortal`, "ok");
  const badge = document.getElementById("top-container-badge");
  if (badge) badge.textContent = "Synth HMC · QualiaPortal";
  selectedOrgan = null;
  meshMode = "body";
  renderPartsList();
}

async function showMesh() {
  if (!portal) return;
  if (meshMode === "heart") {
    await loadCadOrgan("heart", { heartExterior: true });
    return;
  }
  if (meshMode === "organ" && selectedOrgan) {
    await loadCadOrgan(selectedOrgan);
    return;
  }
  const peelOrgan = LAYER_ORGAN[layer];
  if (peelOrgan && !LAYER_SUFFIX[layer]) {
    meshMode = "organ";
    selectedOrgan = peelOrgan;
    await loadCadOrgan(peelOrgan);
    return;
  }
  await loadBodyMesh();
}

function fillGrid(id, rows) {
  const host = document.getElementById(id);
  if (!host) return;
  host.replaceChildren();
  for (const [label, value] of rows) {
    const row = document.createElement("div");
    row.className = "metric-row";
    const name = document.createElement("span");
    name.textContent = label;
    const val = document.createElement("b");
    val.textContent = value;
    row.append(name, val);
    host.append(row);
  }
}

function updateOrganCard(member, heartExterior) {
  const title = document.getElementById("hud-title");
  const name = document.getElementById("selected-organ-name");
  const desc = document.getElementById("selected-organ-desc");
  const sys = document.getElementById("selected-organ-sys");
  const label = prettyOrgan(member.id);
  const systems = (member.system || []).map((s) => SYSTEM_LABEL[s] || s).join(" · ");
  if (title) title.textContent = heartExterior ? "Working heart (CAD)" : `${label} · encyclopedia CAD`;
  if (name) name.textContent = label;
  if (sys) sys.textContent = systems || "Encyclopedia";
  if (desc) {
    desc.textContent = heartExterior
      ? "Working CAD heart from anatomy/working/heart. QualiaPortal greys SurfaceReading; colour here is the circulatory system paint."
      : `Working CAD member ${member.id}. Status ${member.status || "working_cad"}. This is a Synth loft/revolve/extrude envelope, not the frozen welfare ellipsoid.`;
  }
}

async function showMeasurements() {
  const slug = PROFILES[profile].slug;
  const url = `${EXPORT_ROOT}human/${slug}_measurements.json`;
  const response = await fetch(url);
  if (!response.ok) {
    if (meshMode === "body") setStatus(`Measurements missing: ${url}`, "err");
    return;
  }
  const data = await response.json();
  const a = data.anthropometrics || {};
  const c = data.maternal_cardiorespiratory || {};
  const o = data.obstetric_fetal_biometrics || {};
  const w = data.welfare_thresholds_q42 || {};
  fillGrid("metrics-anthropometric", [
    ["Stature", `${a.stature_cm} cm`],
    ["Mass", `${a.body_mass_kg} kg`],
    ["BMI", `${a.bmi_kg_m2}`],
    ["Biacromial", `${a.biacromial_shoulder_cm} cm`],
    ["Bi-iliac", `${a.biiliac_pelvic_cm} cm`],
    ["Pelvic inlet", `${a.pelvic_inlet_transverse_mm} mm`],
    ["Subpubic angle", `${a.subpubic_angle_deg}°`],
    ["Lordosis", `${a.lumbar_lordosis_deg}°`],
  ]);
  fillGrid("metrics-obstetric", [
    ["Week", String(data.gestational_week ?? 0)],
    ["Fundal height", `${o.symphysis_fundal_height_cm} cm`],
    ["EFW", `${o.estimated_fetal_weight_g} g`],
    ["CRL", `${o.crown_rump_length_mm} mm`],
    ["BPD", `${o.biparietal_diameter_mm} mm`],
    ["Femur", `${o.femur_length_mm} mm`],
    ["AFI", `${o.amniotic_fluid_index_cm} cm`],
    ["FHR", `${o.fetal_heart_rate_bpm} bpm`],
  ]);
  fillGrid("metrics-physiology", [
    ["Heart rate", `${c.resting_heart_rate_bpm} bpm`],
    ["Cardiac output", `${c.cardiac_output_l_min} L/min`],
    ["Blood volume", `+${c.blood_volume_expansion_pct} %`],
    ["Uterine flow", `${c.uterine_blood_flow_ml_min} mL/min`],
    ["Diaphragm", `+${c.diaphragmatic_elevation_cm} cm`],
    ["Max lift", `${w.max_recommended_lift_kg} kg`],
    ["Safe lumbar flex", `${w.max_lumbar_flexion_safe_deg}°`],
  ]);
  const rules = document.getElementById("q42-rules-list");
  if (rules) {
    rules.textContent = `${slug}_logic.q42 is inside the HMC. Lift limit ${w.max_recommended_lift_kg} kg. Safe lumbar flexion ${w.max_lumbar_flexion_safe_deg}°.`;
  }
  const boundary = document.getElementById("osce-boundary-note");
  if (boundary) boundary.textContent = lens === "person" ? PERSON_BOUNDARY : CLINICIAN_BOUNDARY;
  if (meshMode === "body") {
    const title = document.getElementById("hud-title");
    if (title) title.textContent = data.label || PROFILES[profile].label;
    const name = document.getElementById("selected-organ-name");
    if (name) name.textContent = data.label || slug;
    const desc = document.getElementById("selected-organ-desc");
    if (desc) {
      desc.textContent =
        lens === "person"
          ? "Welfare whole-body from the frozen HMC. Use the organ picker to open encyclopedia CAD members."
          : "Welfare whole-body schematic. Encyclopedia CAD organs are in the picker; layer peels for viscera/cardio/neuro load those CAD members.";
    }
  }
}

function prettyOrgan(id) {
  return String(id || "").replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

function visibleMembers() {
  const sex = PROFILES[profile]?.sex;
  const systems = LAYER_SYSTEMS[layer];
  const q = partQuery.trim().toLowerCase();
  return catalog.members.filter((m) => {
    if (m.sex === "male" && sex === "female") return false;
    if (m.sex === "female" && sex === "male") return false;
    if (systems && !m.system.some((s) => systems.includes(s))) return false;
    if (q && !(`${m.id} ${(m.system || []).join(" ")}`).toLowerCase().includes(q)) return false;
    return true;
  });
}

function renderPartsList() {
  const host = document.getElementById("parts-list");
  const count = document.getElementById("parts-count");
  if (!host) return;
  host.replaceChildren();
  const members = visibleMembers();
  if (count) count.textContent = `${members.length}`;
  const groups = new Map();
  for (const m of members) {
    const sys = m.system?.[0] || "other";
    if (!groups.has(sys)) groups.set(sys, []);
    groups.get(sys).push(m);
  }
  for (const [sys, rows] of groups) {
    const details = document.createElement("details");
    details.className = "parts-group";
    details.open = true;
    const summary = document.createElement("summary");
    summary.textContent = `${SYSTEM_LABEL[sys] || sys} (${rows.length})`;
    details.append(summary);
    for (const m of rows) {
      const row = document.createElement("div");
      row.className = "part-row" + (selectedOrgan === m.id ? " selected" : "");
      row.dataset.organ = m.id;
      row.textContent = prettyOrgan(m.id);
      row.addEventListener("click", () => window.selectOrgan(m.id));
      details.append(row);
    }
    host.append(details);
  }
}

function renderMixer() {
  const host = document.getElementById("mixer-systems");
  if (!host) return;
  host.replaceChildren();
  const systems = catalog.systems.length ? catalog.systems : Object.keys(SYSTEM_LABEL);
  for (const sys of systems) {
    const row = document.createElement("div");
    row.className = "mixer-row";
    const dot = document.createElement("span");
    dot.className = "SysDot";
    const [r, g, b] = SYSTEM_COLOR[sys] || [0.7, 0.7, 0.7];
    dot.style.background = `rgb(${Math.round(r * 255)}, ${Math.round(g * 255)}, ${Math.round(b * 255)})`;
    const label = document.createElement("button");
    label.type = "button";
    label.className = "mixer-label";
    label.style.cssText = "background:none;border:0;color:inherit;text-align:left;cursor:pointer;padding:0;";
    label.textContent = SYSTEM_LABEL[sys] || sys;
    label.addEventListener("click", () => {
      const member = catalog.members.find((m) => m.system?.includes(sys));
      if (member) window.selectOrgan(member.id);
    });
    row.append(dot, label);
    host.append(row);
  }
}

async function present() {
  try {
    await showMesh();
  } catch (err) {
    setStatus(`Load failed: ${err?.message || err}`, "err");
    return;
  }
  await showMeasurements();
  renderPartsList();
}

window.setWorkingHeart = () => {
  meshMode = "heart";
  selectedOrgan = "heart";
  markActive(".profile-pills .pill-btn", "pill-working-heart");
  present();
};

window.selectOrgan = (id) => {
  meshMode = "organ";
  selectedOrgan = id;
  markActive(".profile-pills .pill-btn", "");
  present();
};

window.setSex = (key) => {
  if (key === "wallaby" || key === "whitegum") {
    setStatus("That subject is a different VibeScript package. This page presents the human welfare HMC and encyclopedia CAD organs.", "");
    return;
  }
  if (!PROFILES[key]) return;
  meshMode = "body";
  selectedOrgan = null;
  profile = key;
  osceCase = 0;
  markActive(".pill-btn", {
    male: "pill-male",
    female: "pill-female",
    female_trimester_1: "pill-t1",
    female_trimester_2: "pill-t2",
    female_trimester_3: "pill-t3",
  }[key]);
  document.getElementById("btn-male")?.classList.toggle("active", key === "male");
  document.getElementById("btn-female")?.classList.toggle("active", key === "female");
  present();
};

window.onGestationalWeekChange = (val) => {
  const week = Number(val);
  const label = document.getElementById("gestation-week-label");
  if (label) label.textContent = week <= 0 ? "Week 0 (Baseline)" : `Week ${week}`;
  if (week >= 34 && week < 37) profile = "female_week_34";
  else if (week >= 27) profile = "female_trimester_3";
  else if (week >= 13) profile = "female_trimester_2";
  else if (week >= 1) profile = "female_trimester_1";
  else profile = "female";
  meshMode = "body";
  selectedOrgan = null;
  present();
};

window.setAnatomyLayerMode = (mode) => {
  layer = LAYER_SUFFIX[mode] === undefined ? "all" : mode;
  if (mode === "maternal_fetal" && PROFILES[profile].sex === "male") profile = "female_trimester_3";
  markLayerPills(layer);
  const peel = LAYER_ORGAN[layer];
  if (peel && !LAYER_SUFFIX[layer]) {
    meshMode = "organ";
    selectedOrgan = peel;
  } else {
    meshMode = "body";
    selectedOrgan = null;
  }
  present();
};

window.setAudienceLens = (next) => {
  lens = next === "person" ? "person" : "clinician";
  markActive("#btn-lens-person, #btn-lens-clinician", next === "person" ? "btn-lens-person" : "btn-lens-clinician");
  const badge = document.getElementById("hud-lens-badge");
  if (badge) badge.textContent = lens === "person" ? "Patient Lens" : "OSCE Clinician Lens";
  showMeasurements();
};

window.selectOsceCase = (index) => {
  osceCase = index;
  if (index === 1) {
    profile = "male";
    meshMode = "organ";
    selectedOrgan = "kidney_left";
    layer = "visceral";
  } else if (index === 2) {
    profile = "female_week_34";
    meshMode = "organ";
    selectedOrgan = "uterus";
    layer = "maternal_fetal";
  } else if (index === 3) {
    meshMode = "organ";
    selectedOrgan = "liver";
    layer = "visceral";
  } else if (index === 4) {
    meshMode = "organ";
    selectedOrgan = "spine";
    layer = "skeletal";
  } else {
    meshMode = "body";
    selectedOrgan = null;
    layer = "all";
  }
  document.querySelectorAll("[id^='osce-case-']").forEach((node) => {
    node.classList.toggle("active", node.id === `osce-case-${index}`);
  });
  markLayerPills(layer);
  present();
};

window.exportCurrent10dContainer = () => {
  const a = document.createElement("a");
  if (meshMode === "heart") {
    a.href = WORKING_HEART_10D;
    a.download = "heart-exterior.10d";
  } else if (meshMode === "organ" && selectedOrgan) {
    a.href = organCadUrl(selectedOrgan);
    a.download = `${selectedOrgan}.10d`;
  } else {
    a.href = `${EXPORT_ROOT}${assetStem()}.10d`;
    a.download = `${PROFILES[profile].slug}.10d`;
  }
  a.click();
};

window.exportCurrentQ42Container = () => {
  const slug = PROFILES[profile].slug;
  const a = document.createElement("a");
  a.href = `${EXPORT_ROOT}human/${slug}_logic.q42`;
  a.download = `${slug}_logic.q42`;
  a.click();
};

window.setClipMode = (mode) => {
  document.getElementById("clip-none")?.classList.toggle("active", mode === "none");
  document.getElementById("clip-sagittal")?.classList.toggle("active", mode === "sagittal");
  document.getElementById("clip-explode")?.classList.toggle("active", mode === "explode");
  const label = document.getElementById("clip-label");
  if (label) label.textContent = mode === "sagittal" ? "Sagittal" : mode === "explode" ? "Exploded" : "Full 3D";
  setStatus("Clip and explode are visibility compiles inside a packed body HMC. Encyclopedia CAD is one organ at a time.", "");
};

window.onCalipersToggle = () => {};
window.onComorbidityArcsToggle = () => {};
window.onAmbient = (on) => {
  try {
    portal?.set_ambient_enabled?.(!!on);
  } catch {
    /* optional */
  }
};
window.resetSystemMixer = () => window.setAnatomyLayerMode("all");
window.onPartSearch = (value) => {
  partQuery = value || "";
  renderPartsList();
};
window.setAllParts = (all) => {
  if (all) {
    meshMode = "body";
    selectedOrgan = null;
    layer = "all";
    markLayerPills("all");
    present();
    return;
  }
  meshMode = "organ";
  selectedOrgan = "heart";
  present();
};

window.setCameraPreset = (name) => {
  const preset = CAMERA_PRESETS[name] || CAMERA_PRESETS.hero;
  cam = { ...preset };
  applyCamera();
  markCamPreset(name);
};

window.nudgeZoom = (delta) => {
  cam.zoom = Math.min(12, Math.max(0.25, cam.zoom + delta));
  applyCamera();
};

window.toggleAutoSpin = () => {
  autoSpin = !autoSpin;
  const btn = document.getElementById("btn-spin-toggle");
  if (btn) btn.textContent = autoSpin ? "Spin: ON" : "Spin: OFF";
};

window.toggleMeasurementsPanel = () => {
  document.getElementById("right-panel")?.classList.toggle("mobile-visible");
  document.getElementById("sidebar")?.classList.toggle("open");
};

window.toggleLayersPanel = () => {
  const sidebar = document.getElementById("sidebar");
  const scrim = document.getElementById("sheet-scrim");
  const btn = document.getElementById("btn-controls");
  const open = !sidebar?.classList.contains("open");
  sidebar?.classList.toggle("open", open);
  scrim?.classList.toggle("open", open);
  btn?.setAttribute("aria-expanded", open ? "true" : "false");
};

function bindOrbit(canvas) {
  if (!canvas || canvas.dataset.orbitBound === "1") return;
  canvas.dataset.orbitBound = "1";
  let lastX = 0;
  let lastY = 0;
  canvas.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    dragging = true;
    lastX = e.clientX;
    lastY = e.clientY;
    canvas.setPointerCapture(e.pointerId);
  });
  canvas.addEventListener("pointermove", (e) => {
    if (!dragging) return;
    const dx = e.clientX - lastX;
    const dy = e.clientY - lastY;
    lastX = e.clientX;
    lastY = e.clientY;
    cam.yaw += dx * 0.005;
    cam.pitch = Math.max(-1.35, Math.min(1.35, cam.pitch + dy * 0.005));
    applyCamera();
  });
  const endDrag = () => {
    dragging = false;
  };
  canvas.addEventListener("pointerup", endDrag);
  canvas.addEventListener("pointercancel", endDrag);
  canvas.addEventListener(
    "wheel",
    (e) => {
      e.preventDefault();
      const factor = e.deltaY > 0 ? 1.08 : 0.92;
      cam.zoom = Math.min(12, Math.max(0.25, cam.zoom * factor));
      applyCamera();
    },
    { passive: false },
  );
}

function onFrame(_tier, dt) {
  if (!autoSpin || dragging) return;
  const step = Number.isFinite(dt) ? dt : 16.67;
  cam.yaw += step * 0.00045;
  applyCamera();
}

async function loadCatalog() {
  try {
    const response = await fetch(CATALOG_URL, { cache: "no-store" });
    if (!response.ok) throw new Error(String(response.status));
    catalog = await response.json();
    if (!Array.isArray(catalog.members)) catalog.members = [];
    if (!Array.isArray(catalog.systems)) catalog.systems = [];
  } catch (err) {
    console.warn("encyclopedia catalog", err);
    catalog = { members: [], systems: Object.keys(SYSTEM_LABEL) };
  }
}

async function boot() {
  const container = document.getElementById("canvas-container");
  if (!container) return;
  const canvas = document.createElement("canvas");
  canvas.id = "anatomy-canvas";
  canvas.style.cssText = "width:100%;height:100%;display:block;touch-action:none;";
  container.appendChild(canvas);
  liveCanvas = canvas;
  setStatus("Opening QualiaPortal", "");
  try {
    const loaded = await loadQualiaPortal(canvas, { allowWebGl2: true });
    portal = loaded.portal;
    stopPortalLoop();
    startPortalLoop(canvas, onFrame, (next) => {
      liveCanvas = next;
      bindOrbit(next);
    });
    bindOrbit(canvas);
  } catch (err) {
    setStatus(`Portal failed: ${err?.message || err}`, "err");
    return;
  }
  if (!portal) {
    setStatus("QualiaPortal did not start. The HMC is still the produced body.", "err");
    return;
  }
  document.getElementById("sheet-scrim")?.addEventListener("click", () => window.toggleLayersPanel());
  setStatus("Fetching encyclopedia catalog and welfare HMC", "");
  await loadCatalog();
  renderMixer();
  renderPartsList();
  try {
    const response = await fetch(HMC_URL, { cache: "no-store" });
    if (!response.ok) throw new Error(`HMC missing (${response.status})`);
    hmcBytes = new Uint8Array(await response.arrayBuffer());
  } catch (err) {
    setStatus(String(err?.message || err), "err");
  }
  const query = new URLSearchParams(location.search);
  if (query.get("working") === "heart" || query.get("organ") === "heart") {
    meshMode = "heart";
    selectedOrgan = "heart";
    markActive(".profile-pills .pill-btn", "pill-working-heart");
  } else if (query.get("organ")) {
    meshMode = "organ";
    selectedOrgan = query.get("organ");
  }
  const attr = document.getElementById("attribution-body");
  if (attr) {
    attr.textContent =
      "Whole-body pills load the frozen welfare HMC. Organ picker and viscera/cardio/neuro peels load working encyclopedia CAD from anatomy/working. Distribution stays frozen until a member is visually accepted.";
  }
  await present();
}

boot();
