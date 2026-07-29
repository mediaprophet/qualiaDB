/**
 * Knowledge Universe — full-viewport 3D canvas viz for QualiaDB Pages.
 *
 * Layers:
 *  1. Procedural milky dust + faint field stars
 *  2. Chora Yale bright-star catalog (celestial shell)
 *  3. Ontology-pack galaxies (mass ∝ log(bytes))
 *  4. Inter-pack links (nebula arcs)
 *  5. CODATA / periodic / math constants as labeled pulsars
 *
 * No WASM required — pure Canvas2D so the page always works offline from static assets.
 */

const MANIFEST_URL = 'data/knowledge-universe-manifest.json';
const SCIENCE_URL = 'data/science-constants.json';

const state = {
  manifest: null,
  science: null,
  yaw: 0.55,
  pitch: -0.18,
  zoom: 1.05,
  auto: true,
  layers: {
    celestial: true,
    galaxies: true,
    links: true,
    science: true,
    dust: true,
  },
  selected: null,
  hover: null,
  dragging: false,
  lx: 0,
  ly: 0,
  particles: [],
  celestial: [],
  labels: [],
  scienceNodes: [],
  linkSegs: [],
  galaxyCenters: new Map(),
  t0: performance.now(),
  W: 0,
  H: 0,
  dpr: 1,
  canvas: null,
  ctx: null,
  anim: 0,
};

function hsl(h, s, l, a = 1) {
  return `hsla(${h}, ${s}%, ${l}%, ${a})`;
}

function bvToRgb(bv) {
  bv = Math.max(-0.4, Math.min(2.0, bv));
  const t = (bv + 0.4) / 2.4;
  const r = 1.0 - Math.min(t * 0.3, 0.5);
  const g = 1.0 - Math.abs(t * 0.5);
  const b = 1.0 - Math.min((1.0 - t) * 0.4, 0.5);
  const br = 1.0 - Math.abs(bv) * 0.1;
  return [r * br, g * br, b * br];
}

function raDecToXyz(raDeg, decDeg, radius) {
  const ra = (raDeg * Math.PI) / 180;
  const dec = (decDeg * Math.PI) / 180;
  return [
    radius * Math.cos(dec) * Math.cos(ra),
    radius * Math.sin(dec),
    radius * Math.cos(dec) * Math.sin(ra),
  ];
}

function mulberry32(a) {
  return function () {
    let t = (a += 0x6d2b79f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function hashStr(s) {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

function formatBytes(n) {
  if (n >= 1e6) return (n / 1e6).toFixed(2) + ' MB';
  if (n >= 1e3) return (n / 1e3).toFixed(1) + ' KB';
  return n + ' B';
}

function particleCountForBytes(bytes) {
  // log scale: schema.org ~80, fibo ~140, tiny packs ~12
  const n = Math.round(10 + Math.log10(Math.max(bytes, 100)) * 28);
  return Math.min(220, Math.max(12, n));
}

function buildGalaxyParticles(g) {
  const rng = mulberry32(hashStr(g.id));
  const n = particleCountForBytes(g.bytes);
  const [cx, cy, cz] = g.pos;
  const spread = g.spread || 0.4;
  const out = [];

  // Core + spiral arms
  for (let i = 0; i < n; i++) {
    const arm = i % 3;
    const t = i / n;
    const angle = t * Math.PI * 4 + arm * ((Math.PI * 2) / 3) + rng() * 0.4;
    const r = spread * (0.08 + t * 0.92) * (0.7 + rng() * 0.5);
    const elev = (rng() - 0.5) * spread * 0.35;
    const x = cx + Math.cos(angle) * r;
    const y = cy + elev + Math.sin(angle * 0.5) * spread * 0.08;
    const z = cz + Math.sin(angle) * r;
    const pulse = rng();
    out.push({
      x, y, z,
      size: 0.8 + rng() * 2.2,
      hue: g.hue + (rng() - 0.5) * 18,
      alpha: 0.35 + rng() * 0.55,
      kind: 'galaxy',
      galaxyId: g.id,
      pulse,
    });
  }

  // Sub-modules (FIBO) as satellite clusters
  if (Array.isArray(g.modules)) {
    g.modules.forEach((m, mi) => {
      const a = (mi / g.modules.length) * Math.PI * 2;
      const mr = spread * 0.72;
      const mx = cx + Math.cos(a) * mr;
      const my = cy + (mi % 2 === 0 ? 0.12 : -0.1) * spread;
      const mz = cz + Math.sin(a) * mr;
      const mn = Math.min(40, particleCountForBytes(m.bytes) / 2);
      const mrng = mulberry32(hashStr(g.id + m.id));
      for (let i = 0; i < mn; i++) {
        const u = mrng();
        const v = mrng();
        const w = mrng();
        const rr = spread * 0.12 * Math.cbrt(u);
        const th = v * Math.PI * 2;
        const ph = Math.acos(2 * w - 1);
        out.push({
          x: mx + rr * Math.sin(ph) * Math.cos(th),
          y: my + rr * Math.cos(ph),
          z: mz + rr * Math.sin(ph) * Math.sin(th),
          size: 0.7 + mrng() * 1.6,
          hue: g.hue + 12,
          alpha: 0.4 + mrng() * 0.4,
          kind: 'module',
          galaxyId: g.id,
          moduleId: m.id,
          pulse: mrng(),
        });
      }
    });
  }

  state.galaxyCenters.set(g.id, { x: cx, y: cy, z: cz, g });
  return out;
}

function buildCelestial(stars) {
  const R = 3.4;
  return stars.map((s) => {
    const [x, y, z] = raDecToXyz(s.ra, s.dec, R);
    const [r, g, b] = bvToRgb(s.bv);
    const size = Math.max(0.7, ((6.5 - s.mag) / 6.5) * 3.6);
    return { x, y, z, r, g, b, size, name: s.name, mag: s.mag, kind: 'star' };
  });
}

function buildScienceNodes(science, scienceGalaxy) {
  const nodes = [];
  const [cx, cy, cz] = scienceGalaxy ? scienceGalaxy.pos : [0, 0, 1.2];
  const phys = science.physical_constants || [];
  const math = science.mathematical_constants || [];
  const elems = science.periodic_elements || [];

  phys.forEach((c, i) => {
    const a = (i / Math.max(1, phys.length)) * Math.PI * 2;
    const r = 0.38 + (i % 3) * 0.05;
    nodes.push({
      x: cx + Math.cos(a) * r,
      y: cy + Math.sin(a * 1.3) * 0.12,
      z: cz + Math.sin(a) * r,
      label: c.id,
      title: c.name,
      detail: `${c.value} ${c.unit} · ${c.domain}`,
      hue: 320,
      size: 3.2,
      kind: 'constant',
    });
  });

  math.forEach((c, i) => {
    const a = (i / Math.max(1, math.length)) * Math.PI * 2 + 0.4;
    const r = 0.22;
    nodes.push({
      x: cx + Math.cos(a) * r,
      y: cy + 0.22,
      z: cz + Math.sin(a) * r,
      label: c.symbol || c.id,
      title: c.name,
      detail: String(c.value),
      hue: 280,
      size: 2.6,
      kind: 'math',
    });
  });

  // Periodic sample as a helical arm off the science hub
  elems.forEach((e, i) => {
    const t = i / Math.max(1, elems.length - 1);
    const a = t * Math.PI * 3.5;
    const r = 0.55 + t * 0.35;
    nodes.push({
      x: cx + Math.cos(a) * r * 0.85,
      y: cy - 0.35 - t * 0.45,
      z: cz + Math.sin(a) * r * 0.85,
      label: e.symbol,
      title: e.name,
      detail: `Z=${e.Z} · mass ${e.mass} · period ${e.period}`,
      hue: 200 + (e.group || 1) * 4,
      size: 1.8 + Math.min(2, (e.radius_pm || 80) / 100),
      kind: 'element',
    });
  });

  return nodes;
}

function buildLinks(manifest) {
  const segs = [];
  for (const L of manifest.links || []) {
    const a = state.galaxyCenters.get(L.from);
    const b = state.galaxyCenters.get(L.to);
    if (!a || !b) continue;
    segs.push({
      ax: a.x, ay: a.y, az: a.z,
      bx: b.x, by: b.y, bz: b.z,
      kind: L.kind || 'link',
      hue: ((a.g.hue + b.g.hue) / 2) | 0,
    });
  }
  return segs;
}

function project(x, y, z) {
  const cy = Math.cos(state.yaw);
  const sy = Math.sin(state.yaw);
  const cp = Math.cos(state.pitch);
  const sp = Math.sin(state.pitch);
  let x1 = x * cy + z * sy;
  let z1 = -x * sy + z * cy;
  let y1 = y * cp - z1 * sp;
  z1 = y * sp + z1 * cp;
  const f = (420 * state.zoom) / Math.max(0.25, z1 + 4.2);
  return {
    px: state.W / 2 + x1 * f,
    py: state.H / 2 - y1 * f,
    depth: z1,
    scale: f / 420,
  };
}

function resize() {
  const canvas = state.canvas;
  state.dpr = Math.min(window.devicePixelRatio || 1, 2);
  state.W = window.innerWidth;
  state.H = window.innerHeight;
  canvas.width = Math.floor(state.W * state.dpr);
  canvas.height = Math.floor(state.H * state.dpr);
  canvas.style.width = state.W + 'px';
  canvas.style.height = state.H + 'px';
  state.ctx.setTransform(state.dpr, 0, 0, state.dpr, 0, 0);
}

function drawBackground(ctx, t) {
  ctx.fillStyle = '#020617';
  ctx.fillRect(0, 0, state.W, state.H);

  if (!state.layers.dust) return;

  // Slow nebula washes
  const blobs = [
    [0.28, 0.32, 0.55, 'rgba(99,102,241,0.14)', 'rgba(56,189,248,0.04)'],
    [0.72, 0.55, 0.45, 'rgba(244,114,182,0.10)', 'rgba(251,191,36,0.03)'],
    [0.5, 0.75, 0.4, 'rgba(52,211,153,0.08)', 'rgba(14,165,233,0.03)'],
  ];
  for (const [ux, uy, rad, c0, c1] of blobs) {
    const ox = Math.sin(t * 0.00015 + ux * 10) * 30;
    const oy = Math.cos(t * 0.00012 + uy * 8) * 24;
    const g = ctx.createRadialGradient(
      state.W * ux + ox, state.H * uy + oy, 10,
      state.W * ux + ox, state.H * uy + oy, state.W * rad
    );
    g.addColorStop(0, c0);
    g.addColorStop(0.55, c1);
    g.addColorStop(1, 'rgba(2,6,23,0)');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, state.W, state.H);
  }

  // Faint field stars
  const rng = mulberry32(42);
  for (let i = 0; i < 420; i++) {
    const a = rng() * Math.PI * 2;
    const b = rng() * Math.PI - Math.PI / 2;
    const r = 2.8 + rng() * 1.4;
    const [x, y, z] = raDecToXyz((a * 180) / Math.PI, (b * 180) / Math.PI, r);
    const p = project(x, y, z);
    if (p.depth < -1.8) continue;
    const s = 0.35 + (i % 4) * 0.2;
    const tw = 0.12 + 0.1 * Math.sin(t * 0.002 + i);
    ctx.fillStyle = `rgba(226,232,240,${tw})`;
    ctx.beginPath();
    ctx.arc(p.px, p.py, s, 0, Math.PI * 2);
    ctx.fill();
  }
}

function drawLinks(ctx, t) {
  if (!state.layers.links) return;
  for (const L of state.linkSegs) {
    const a = project(L.ax, L.ay, L.az);
    const b = project(L.bx, L.by, L.bz);
    if (a.depth < -2 && b.depth < -2) continue;
    // Bezier control lifted toward camera
    const mx = (a.px + b.px) / 2;
    const my = (a.py + b.py) / 2 - 40 - 10 * Math.sin(t * 0.001 + L.hue);
    const grd = ctx.createLinearGradient(a.px, a.py, b.px, b.py);
    grd.addColorStop(0, hsl(L.hue, 70, 60, 0.05));
    grd.addColorStop(0.5, hsl(L.hue, 80, 70, 0.35));
    grd.addColorStop(1, hsl(L.hue, 70, 60, 0.05));
    ctx.strokeStyle = grd;
    ctx.lineWidth = 1.25;
    ctx.beginPath();
    ctx.moveTo(a.px, a.py);
    ctx.quadraticCurveTo(mx, my, b.px, b.py);
    ctx.stroke();

    // Traveling pulse
    const u = (Math.sin(t * 0.0012 + L.hue * 0.1) + 1) / 2;
    const px = (1 - u) * (1 - u) * a.px + 2 * (1 - u) * u * mx + u * u * b.px;
    const py = (1 - u) * (1 - u) * a.py + 2 * (1 - u) * u * my + u * u * b.py;
    ctx.fillStyle = hsl(L.hue, 90, 75, 0.85);
    ctx.beginPath();
    ctx.arc(px, py, 2.2, 0, Math.PI * 2);
    ctx.fill();
  }
}

function drawParticles(ctx, t) {
  if (!state.layers.galaxies) return;
  // Sort by depth for crude painter's algorithm
  const sorted = state.particles
    .map((p) => {
      const pr = project(p.x, p.y, p.z);
      return { p, pr };
    })
    .filter((o) => o.pr.depth > -2.2)
    .sort((a, b) => a.pr.depth - b.pr.depth);

  for (const { p, pr } of sorted) {
    const tw = 0.75 + 0.25 * Math.sin(t * 0.003 + p.pulse * 20);
    const sel = state.selected && state.selected.id === p.galaxyId;
    const hov = state.hover && state.hover.id === p.galaxyId;
    const alpha = p.alpha * tw * (sel || hov ? 1.15 : 1);
    const size = p.size * Math.max(0.4, pr.scale) * (sel ? 1.35 : 1);
    ctx.fillStyle = hsl(p.hue, 78, sel ? 72 : 62, alpha);
    ctx.beginPath();
    ctx.arc(pr.px, pr.py, size, 0, Math.PI * 2);
    ctx.fill();
    if (sel || hov) {
      ctx.strokeStyle = hsl(p.hue, 90, 80, 0.35);
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.arc(pr.px, pr.py, size + 3, 0, Math.PI * 2);
      ctx.stroke();
    }
  }

  // Galaxy labels at centers
  for (const [, center] of state.galaxyCenters) {
    const pr = project(center.x, center.y, center.z);
    if (pr.depth < -1.5) continue;
    const g = center.g;
    const active = state.selected && state.selected.id === g.id;
    ctx.font = `${active ? 600 : 500} ${active ? 13 : 11}px system-ui,Segoe UI,sans-serif`;
    ctx.fillStyle = hsl(g.hue, 70, 78, active ? 0.95 : 0.7);
    ctx.textAlign = 'center';
    ctx.fillText(g.label, pr.px, pr.py - 14);
    if (active) {
      ctx.font = '10px system-ui,Segoe UI,sans-serif';
      ctx.fillStyle = 'rgba(226,232,240,0.55)';
      ctx.fillText(formatBytes(g.bytes), pr.px, pr.py - 2);
    }
  }
}

function drawCelestial(ctx, t) {
  if (!state.layers.celestial) return;
  for (const s of state.celestial) {
    const pr = project(s.x, s.y, s.z);
    if (pr.depth < -1.5) continue;
    const tw = 0.85 + 0.15 * Math.sin(t * 0.0025 + s.mag * 3);
    const size = s.size * Math.max(0.5, pr.scale) * state.zoom * 0.9;
    const alpha = Math.min(1, 0.45 + size * 0.12) * tw;
    // glow
    const glow = ctx.createRadialGradient(pr.px, pr.py, 0, pr.px, pr.py, size * 3.5);
    glow.addColorStop(0, `rgba(${(s.r * 255) | 0},${(s.g * 255) | 0},${(s.b * 255) | 0},${0.35 * alpha})`);
    glow.addColorStop(1, 'rgba(0,0,0,0)');
    ctx.fillStyle = glow;
    ctx.beginPath();
    ctx.arc(pr.px, pr.py, size * 3.5, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = `rgba(${(s.r * 255) | 0},${(s.g * 255) | 0},${(s.b * 255) | 0},${alpha})`;
    ctx.beginPath();
    ctx.arc(pr.px, pr.py, size, 0, Math.PI * 2);
    ctx.fill();
    if (s.name && s.mag < 1.5 && state.zoom > 0.85) {
      ctx.font = '10px system-ui,Segoe UI,sans-serif';
      ctx.fillStyle = `rgba(226,232,240,${0.45 * alpha})`;
      ctx.textAlign = 'left';
      ctx.fillText(s.name, pr.px + size + 4, pr.py + 3);
    }
  }
}

function drawScience(ctx, t) {
  if (!state.layers.science) return;
  for (const n of state.scienceNodes) {
    const pr = project(n.x, n.y, n.z);
    if (pr.depth < -1.8) continue;
    const pulse = 0.7 + 0.3 * Math.sin(t * 0.004 + n.x * 5);
    const size = n.size * Math.max(0.45, pr.scale) * pulse;
    const glow = ctx.createRadialGradient(pr.px, pr.py, 0, pr.px, pr.py, size * 4);
    glow.addColorStop(0, hsl(n.hue, 90, 70, 0.45));
    glow.addColorStop(1, 'rgba(0,0,0,0)');
    ctx.fillStyle = glow;
    ctx.beginPath();
    ctx.arc(pr.px, pr.py, size * 4, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = hsl(n.hue, 85, 72, 0.95);
    ctx.beginPath();
    ctx.arc(pr.px, pr.py, size, 0, Math.PI * 2);
    ctx.fill();
    if (state.zoom > 0.9 || n.kind === 'constant') {
      ctx.font = '10px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace';
      ctx.fillStyle = hsl(n.hue, 40, 85, 0.75);
      ctx.textAlign = 'center';
      ctx.fillText(n.label, pr.px, pr.py - size - 4);
    }
    n._pr = pr; // for hit test
  }
}

function frame(now) {
  const t = now - state.t0;
  if (state.auto && !state.dragging) {
    state.yaw += 0.00055;
    state.pitch = -0.18 + Math.sin(t * 0.0002) * 0.04;
  }
  const ctx = state.ctx;
  drawBackground(ctx, t);
  drawLinks(ctx, t);
  drawParticles(ctx, t);
  drawCelestial(ctx, t);
  drawScience(ctx, t);
  state.anim = requestAnimationFrame(frame);
}

function nearestGalaxy(mx, my) {
  let best = null;
  let bestD = 48;
  for (const [, center] of state.galaxyCenters) {
    const pr = project(center.x, center.y, center.z);
    if (pr.depth < -1.5) continue;
    const d = Math.hypot(pr.px - mx, pr.py - my);
    if (d < bestD) {
      bestD = d;
      best = center.g;
    }
  }
  return best;
}

function nearestScience(mx, my) {
  let best = null;
  let bestD = 22;
  for (const n of state.scienceNodes) {
    if (!n._pr || n._pr.depth < -1.5) continue;
    const d = Math.hypot(n._pr.px - mx, n._pr.py - my);
    if (d < bestD) {
      bestD = d;
      best = n;
    }
  }
  return best;
}

function updateInfoPanel(sel) {
  const el = document.getElementById('ku-info');
  const title = document.getElementById('ku-info-title');
  const body = document.getElementById('ku-info-body');
  const meta = document.getElementById('ku-info-meta');
  const link = document.getElementById('ku-info-link');
  if (!el || !title) return;

  if (!sel) {
    el.classList.add('dim');
    title.textContent = 'Explore the knowledge universe';
    body.textContent =
      'Each glowing cluster is a real bundled dataset on this site. Mass scales with on-disk pack size. Outer shell: Chora Yale bright stars. Science hub: CODATA constants + periodic sample.';
    meta.textContent = '';
    if (link) link.hidden = true;
    return;
  }

  el.classList.remove('dim');
  if (sel._science) {
    title.textContent = sel.title;
    body.textContent = sel.detail;
    meta.textContent = `Science node · ${sel.kind}`;
    if (link) {
      link.hidden = false;
      link.href = 'science-playground.html';
      link.textContent = 'Open science playground →';
    }
    return;
  }

  title.textContent = sel.label;
  body.textContent = sel.blurb || '';
  meta.textContent = `${formatBytes(sel.bytes)} · ${sel.path || ''} · ${sel.family || ''}`;
  if (link) {
    if (sel.href) {
      link.hidden = false;
      link.href = sel.href;
      link.textContent = 'Open related demo →';
    } else {
      link.hidden = true;
    }
  }
}

function setStatus(msg, err = false) {
  const el = document.getElementById('ku-status');
  if (!el) return;
  el.textContent = msg;
  el.classList.toggle('err', !!err);
}

function bindUi() {
  const canvas = state.canvas;
  canvas.addEventListener('pointerdown', (e) => {
    state.dragging = true;
    state.auto = false;
    state.lx = e.clientX;
    state.ly = e.clientY;
    canvas.setPointerCapture(e.pointerId);
  });
  canvas.addEventListener('pointermove', (e) => {
    if (state.dragging) {
      const dx = e.clientX - state.lx;
      const dy = e.clientY - state.ly;
      state.yaw += dx * 0.005;
      state.pitch = Math.max(-1.2, Math.min(1.2, state.pitch + dy * 0.005));
      state.lx = e.clientX;
      state.ly = e.clientY;
    } else {
      state.hover = nearestGalaxy(e.clientX, e.clientY);
      canvas.style.cursor = state.hover ? 'pointer' : 'grab';
    }
  });
  canvas.addEventListener('pointerup', (e) => {
    state.dragging = false;
    const g = nearestGalaxy(e.clientX, e.clientY);
    const s = nearestScience(e.clientX, e.clientY);
    if (s) {
      state.selected = { ...s, _science: true, id: 'science:' + s.label };
      updateInfoPanel(state.selected);
    } else if (g) {
      state.selected = g;
      updateInfoPanel(g);
    }
  });
  canvas.addEventListener('pointerleave', () => {
    state.dragging = false;
    state.hover = null;
  });
  canvas.addEventListener(
    'wheel',
    (e) => {
      e.preventDefault();
      state.zoom = Math.max(0.45, Math.min(2.8, state.zoom * (e.deltaY > 0 ? 0.92 : 1.08)));
    },
    { passive: false }
  );

  document.getElementById('ku-auto')?.addEventListener('click', () => {
    state.auto = !state.auto;
    const b = document.getElementById('ku-auto');
    if (b) b.classList.toggle('active', state.auto);
  });
  document.getElementById('ku-reset')?.addEventListener('click', () => {
    state.yaw = 0.55;
    state.pitch = -0.18;
    state.zoom = 1.05;
    state.selected = null;
    updateInfoPanel(null);
  });

  for (const key of Object.keys(state.layers)) {
    const btn = document.getElementById('ku-layer-' + key);
    if (!btn) continue;
    btn.addEventListener('click', () => {
      state.layers[key] = !state.layers[key];
      btn.classList.toggle('active', state.layers[key]);
    });
    btn.classList.toggle('active', state.layers[key]);
  }

  // Galaxy chips
  const chips = document.getElementById('ku-chips');
  if (chips && state.manifest) {
    chips.innerHTML = '';
    for (const g of state.manifest.galaxies) {
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'ku-chip';
      b.style.setProperty('--chip-hue', g.hue);
      b.textContent = g.label;
      b.title = formatBytes(g.bytes);
      b.addEventListener('click', () => {
        state.selected = g;
        // Soft fly-to: bias yaw toward galaxy
        const [x, , z] = g.pos;
        state.yaw = Math.atan2(x, z + 0.01) + Math.PI * 0.15;
        state.pitch = -0.12;
        state.zoom = 1.35;
        state.auto = false;
        document.getElementById('ku-auto')?.classList.remove('active');
        updateInfoPanel(g);
        chips.querySelectorAll('.ku-chip').forEach((c) => c.classList.remove('active'));
        b.classList.add('active');
      });
      chips.appendChild(b);
    }
  }
}

async function loadJson(url) {
  const res = await fetch(url, { cache: 'no-cache' });
  if (!res.ok) throw new Error(`${url}: ${res.status}`);
  return res.json();
}

export async function bootKnowledgeUniverse(canvasId = 'ku-sky') {
  state.canvas = document.getElementById(canvasId);
  if (!state.canvas) throw new Error('canvas missing');
  state.ctx = state.canvas.getContext('2d');
  resize();
  window.addEventListener('resize', resize);

  setStatus('Loading datasets…');
  try {
    const [manifest, science] = await Promise.all([
      loadJson(MANIFEST_URL),
      loadJson(SCIENCE_URL).catch(() => null),
    ]);
    state.manifest = manifest;
    state.science = science;

    state.celestial = buildCelestial(manifest.bright_stars || []);
    state.particles = [];
    state.galaxyCenters.clear();
    for (const g of manifest.galaxies) {
      state.particles.push(...buildGalaxyParticles(g));
    }
    state.linkSegs = buildLinks(manifest);
    const sciG = manifest.galaxies.find((g) => g.id === 'science');
    state.scienceNodes = science ? buildScienceNodes(science, sciG) : [];

    const totalBytes = manifest.galaxies.reduce((s, g) => s + (g.bytes || 0), 0);
    setStatus(
      `${manifest.galaxies.length} galaxies · ${formatBytes(totalBytes)} packs · ${state.celestial.length} catalog stars · ${state.scienceNodes.length} science nodes`
    );
    bindUi();
    updateInfoPanel(null);
    document.getElementById('ku-auto')?.classList.add('active');
    state.anim = requestAnimationFrame(frame);
  } catch (err) {
    console.error(err);
    setStatus(String(err.message || err), true);
  }
}

// Auto-boot when loaded as classic script with type=module from universe.html
if (typeof window !== 'undefined') {
  window.bootKnowledgeUniverse = bootKnowledgeUniverse;
}
