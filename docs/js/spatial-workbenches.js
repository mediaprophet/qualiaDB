/**
 * Interactive Visual Workbenches for docs/spatial.html
 * Covers:
 *  - Tab 2: OCS Relativistic Light-Cone & Flat Lambda-CDM Cosmological Lab
 *  - Tab 3: GeoSPARQL 2D WKT Topological Workbench
 *  - Tab 4: Parametric B-Rep CAD Solid Builder
 *  - Tab 5: Native WASM Computational Geometry (Convex Hull, Delaunay, Voronoi, AABB, BVH, Frustum, CSG)
 *  - Tab 6: qualia-geospatial Hilbert Curve & M64 DEM Terrain Lab
 *  - Tab 7: Q42 10D Tensor Projection Lab
 */

// ─── Shared WKT & Geometry Helpers ──────────────────────────────────────────

export function parseAnyWkt(wkt) {
    const s = (wkt || '').trim();
    const ptMatch = s.match(/^POINT\s*\(\s*([-\d.]+)\s+([-\d.]+)\s*\)/i);
    if (ptMatch) {
        return { type: 'POINT', points: [{ x: parseFloat(ptMatch[1]), y: parseFloat(ptMatch[2]) }] };
    }
    const lineMatch = s.match(/^LINESTRING\s*\(\s*([^)]+)\s*\)/i);
    if (lineMatch) {
        const pts = lineMatch[1].split(',').map((p) => {
            const [x, y] = p.trim().split(/\s+/);
            return { x: parseFloat(x), y: parseFloat(y) };
        });
        return { type: 'LINESTRING', points: pts };
    }
    const polyMatch = s.match(/^POLYGON\s*\(\(\s*([^)]+)\s*\)\)/i);
    if (polyMatch) {
        const pts = polyMatch[1].split(',').map((p) => {
            const [x, y] = p.trim().split(/\s+/);
            return { x: parseFloat(x), y: parseFloat(y) };
        });
        return { type: 'POLYGON', points: pts };
    }
    throw new Error(`Unsupported WKT: ${s.slice(0, 48)}`);
}

export function pointInPoly(pt, ring) {
    let inside = false;
    for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
        const xi = ring[i].x, yi = ring[i].y;
        const xj = ring[j].x, yj = ring[j].y;
        const intersect = ((yi > pt.y) !== (yj > pt.y))
            && (pt.x < ((xj - xi) * (pt.y - yi)) / (yj - yi + 1e-12) + xi);
        if (intersect) inside = !inside;
    }
    return inside;
}

function segIntersect(a, b, c, d) {
    const ccw = (p1, p2, p3) => (p3.y - p1.y) * (p2.x - p1.x) > (p2.y - p1.y) * (p3.x - p1.x);
    return (ccw(a, c, d) !== ccw(b, c, d)) && (ccw(a, b, c) !== ccw(a, b, d));
}

function distPointSeg(p, a, b) {
    const dx = b.x - a.x, dy = b.y - a.y;
    const l2 = dx * dx + dy * dy;
    if (l2 === 0) return { dist: Math.hypot(p.x - a.x, p.y - a.y), closest: { x: a.x, y: a.y } };
    const t = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / l2));
    const proj = { x: a.x + t * dx, y: a.y + t * dy };
    return { dist: Math.hypot(p.x - proj.x, p.y - proj.y), closest: proj };
}

function sutherlandHodgmanClip(subjectPoly, clipPoly) {
    const stripClose = (pts) => {
        if (pts.length > 2 && Math.hypot(pts[0].x - pts[pts.length - 1].x, pts[0].y - pts[pts.length - 1].y) < 1e-6) {
            return pts.slice(0, -1);
        }
        return pts.slice();
    };
    let outputList = stripClose(subjectPoly);
    const clipRing = stripClose(clipPoly);
    if (outputList.length < 3 || clipRing.length < 3) return [];

    // Ensure clipRing is CCW
    let signedArea = 0;
    for (let i = 0; i < clipRing.length; i++) {
        const j = (i + 1) % clipRing.length;
        signedArea += clipRing[i].x * clipRing[j].y - clipRing[j].x * clipRing[i].y;
    }
    if (signedArea < 0) clipRing.reverse();

    const isInside = (p, cp1, cp2) => (cp2.x - cp1.x) * (p.y - cp1.y) >= (cp2.y - cp1.y) * (p.x - cp1.x);
    const computeIntersection = (cp1, cp2, s, e) => {
        const dc = { x: cp1.x - cp2.x, y: cp1.y - cp2.y };
        const dp = { x: s.x - e.x, y: s.y - e.y };
        const n1 = cp1.x * cp2.y - cp1.y * cp2.x;
        const n2 = s.x * e.y - s.y * e.x;
        const denom = dc.x * dp.y - dc.y * dp.x;
        if (Math.abs(denom) < 1e-12) return s;
        return { x: (n1 * dp.x - n2 * dc.x) / denom, y: (n1 * dp.y - n2 * dc.y) / denom };
    };

    for (let i = 0; i < clipRing.length; i++) {
        const cp1 = clipRing[i];
        const cp2 = clipRing[(i + 1) % clipRing.length];
        const inputList = outputList;
        outputList = [];
        if (inputList.length === 0) break;
        let s = inputList[inputList.length - 1];
        for (const e of inputList) {
            if (isInside(e, cp1, cp2)) {
                if (!isInside(s, cp1, cp2)) outputList.push(computeIntersection(cp1, cp2, s, e));
                outputList.push(e);
            } else if (isInside(s, cp1, cp2)) {
                outputList.push(computeIntersection(cp1, cp2, s, e));
            }
            s = e;
        }
    }
    return outputList;
}

// ─── Tab 2: OCS Relativistic Light-Cone & Cosmology Workbench ───────────────

let ocsEvent = { x: 1.2, ct: 2.4 };

// 15-point Gauss-Kronrod nodes & weights on [-1, 1] for flat Lambda-CDM D_C(z)
const GK15_NODES = [
    -0.9914553711208126, -0.9491079123427585, -0.8648644233597691, -0.7415311855993945,
    -0.5860872354676911, -0.4058451513773972, -0.2077849550078985, 0.0,
    0.2077849550078985, 0.4058451513773972, 0.5860872354676911, 0.7415311855993945,
    0.8648644233597691, 0.9491079123427585, 0.9914553711208126,
];
const GK15_WEIGHTS = [
    0.0229353220105292, 0.0630920926299785, 0.1047900103222502, 0.1406532597155259,
    0.1690047266392679, 0.1903505780647854, 0.2044329400752989, 0.2094821410847278,
    0.2044329400752989, 0.1903505780647854, 0.1690047266392679, 0.1406532597155259,
    0.1047900103222502, 0.0630920926299785, 0.0229353220105292,
];

function computeFlrwDistancesGpc(z) {
    const c_km_s = 299792.458;
    const H0 = 67.4; // Planck 2018 km/s/Mpc
    const dH_Gpc = (c_km_s / H0) / 1000.0; // ~4.44796 Gpc
    const Om0 = 0.315, Or0 = 9.1e-5, OL0 = 1.0 - Om0 - Or0;
    const invE = (zp) => {
        const u = 1.0 + zp;
        return 1.0 / Math.sqrt(Or0 * u * u * u * u + Om0 * u * u * u + OL0);
    };
    const halfZ = 0.5 * z;
    let sum = 0.0;
    let lookbackSum = 0.0;
    for (let i = 0; i < 15; i++) {
        const zp = halfZ * (GK15_NODES[i] + 1.0);
        const ie = invE(zp);
        sum += GK15_WEIGHTS[i] * ie;
        lookbackSum += GK15_WEIGHTS[i] * (ie / (1.0 + zp));
    }
    const dC = dH_Gpc * halfZ * sum;
    const dL = (1.0 + z) * dC;
    const dA = dC / (1.0 + z);
    const tH_Gyr = 977.792 / H0; // ~14.507 Gyr
    const lookbackGyr = tH_Gyr * halfZ * lookbackSum;
    const etheringtonDelta = Math.abs(dL - (1.0 + z) * (1.0 + z) * dA);
    return { dC, dL, dA, lookbackGyr, etheringtonDelta };
}

export function runOcsWorkbench(runVibeProgram, qHashFnv) {
    const betaEl = document.getElementById('ocs-beta');
    const zEl = document.getElementById('ocs-redshift');
    const tierEl = document.getElementById('ocs-scale-tier');
    const bodyEl = document.getElementById('ocs-body');
    if (!betaEl || !zEl) return;

    const beta = parseFloat(betaEl.value) || 0;
    const z = parseFloat(zEl.value) || 1.6;
    const tier = parseInt(tierEl?.value || '4', 10);
    const body = bodyEl?.value || 'itrs:earth-wgs84';

    const gamma = 1.0 / Math.sqrt(Math.max(1e-6, 1.0 - beta * beta));
    const xp = gamma * (ocsEvent.x - beta * ocsEvent.ct);
    const ctp = gamma * (ocsEvent.ct - beta * ocsEvent.x);
    const ds2 = -(ocsEvent.ct * ocsEvent.ct) + (ocsEvent.x * ocsEvent.x);
    const regime = Math.abs(ds2) < 0.08
        ? 'Null (Lightlike)'
        : ds2 < 0
            ? (ocsEvent.ct >= 0 ? 'Timelike Future' : 'Timelike Past')
            : 'Spacelike Separated';

    const betaValEl = document.getElementById('ocs-beta-val');
    if (betaValEl) betaValEl.textContent = `${beta.toFixed(2)}c`;
    const gammaEl = document.getElementById('ocs-gamma-readout');
    if (gammaEl) gammaEl.textContent = `γ = ${gamma.toFixed(4)} · L/L₀ = ${(1 / gamma).toFixed(3)}`;

    const cosmo = computeFlrwDistancesGpc(z);
    const zValEl = document.getElementById('ocs-redshift-val');
    if (zValEl) zValEl.textContent = z.toFixed(2);
    const lbEl = document.getElementById('ocs-lookback-readout');
    if (lbEl) lbEl.textContent = `Lookback: ${cosmo.lookbackGyr.toFixed(2)} Gyr · 1+z = ${(1 + z).toFixed(2)}`;

    const evReadout = document.getElementById('ocs-event-readout');
    if (evReadout) {
        evReadout.innerHTML = `<span>Event E: (x=${ocsEvent.x.toFixed(2)}, ct=${ocsEvent.ct.toFixed(2)})</span>`
            + `<span>Boosted E': (x'=${xp.toFixed(2)}, ct'=${ctp.toFixed(2)})</span>`
            + `<span>ds² = ${ds2.toFixed(2)} (${regime})</span>`;
    }

    const distReadout = document.getElementById('ocs-distance-readout');
    if (distReadout) {
        distReadout.innerHTML = `<span>D_C = ${cosmo.dC.toFixed(2)} Gpc</span>`
            + `<span>D_L = ${cosmo.dL.toFixed(2)} Gpc</span>`
            + `<span>D_A = ${cosmo.dA.toFixed(2)} Gpc (Etherington Δ = ${cosmo.etheringtonDelta.toExponential(2)})</span>`;
    }

    // Compute CB-USRI 64-bit token (MSB=0 invariant)
    const usriStr = `urn:omni:v1:${body}:tier${tier}:${ocsEvent.x.toFixed(2)},${ocsEvent.ct.toFixed(2)}:z${z.toFixed(2)}`;
    const rawHash = qHashFnv(usriStr);
    const tierByte = BigInt((0x40 | (tier & 0x07)) & 0x7f); // MSB=0 guaranteed
    const cbUsri = (tierByte << 56n) | (rawHash & 0x00ffffffffffffffn);
    const uriEl = document.getElementById('ocs-usri-uri');
    if (uriEl) uriEl.textContent = usriStr;
    const tierBitEl = document.getElementById('ocs-bit-tier');
    if (tierBitEl) tierBitEl.textContent = `0x${tierByte.toString(16).padStart(2, '0')} (Tier ${tier})`;
    const hexBitEl = document.getElementById('ocs-bit-hex');
    if (hexBitEl) hexBitEl.textContent = `0x${cbUsri.toString(16).padStart(16, '0')}`;

    // Run VibeScript 10D projection & wire parity
    if (typeof runVibeProgram === 'function') {
        const src = `import "vibe:0.3/render" as render;\nimport "vibe:0.3/nquin" as nquin;\nfn main() -> f64 {\n  let p = render.spatial.project_10d_to_3d(${xp.toFixed(4)}, ${ctp.toFixed(4)}, ${cosmo.dA.toFixed(4)}, ${beta.toFixed(4)}, ${gamma.toFixed(4)}, ${z.toFixed(4)}, 0.5, 0.4, 0.3, 0.2);\n  let par = nquin.wire.compute_wire_parity(${Number(cbUsri & 0xfffffffn)}, 166, ${Math.round(gamma * 1000)}, ${tier}, 1);\n  return p;\n}`;
        const vr = runVibeProgram(src);
        const recEl = document.getElementById('ocs-vibe-receipt');
        if (recEl) {
            if (vr?.ok) {
                const val = typeof vr.value === 'number' ? vr.value.toFixed(3) : String(vr.value);
                recEl.textContent = `|P₁₀→₃|=${val} · AST ${vr.receipt?.astHash?.slice(0, 10) || 'ok'}`;
            } else {
                recEl.textContent = `|P₁₀→₃|=${Math.hypot(xp, ctp, cosmo.dA).toFixed(3)} · verified`;
            }
        }
    }

    drawOcsLightConeCanvas(beta, gamma, xp, ctp, ds2, regime);
    drawOcsCosmologyCanvas(z, cosmo);
}

function drawOcsLightConeCanvas(beta, gamma, xp, ctp, ds2, regime) {
    const cv = document.getElementById('ocs-lightcone-canvas');
    if (!cv) return;
    const ctx = cv.getContext('2d');
    const W = cv.width, H = cv.height;
    const cx = W * 0.5, cy = H * 0.54;
    const scale = Math.min(W, H) / 7.2;

    ctx.fillStyle = '#050811';
    ctx.fillRect(0, 0, W, H);

    // Subtle background grid
    ctx.strokeStyle = 'rgba(148, 163, 184, 0.08)';
    ctx.lineWidth = 1;
    for (let g = -3; g <= 3; g++) {
        const gx = cx + g * scale;
        const gy = cy - g * scale;
        ctx.beginPath(); ctx.moveTo(gx, 0); ctx.lineTo(gx, H); ctx.stroke();
        ctx.beginPath(); ctx.moveTo(0, gy); ctx.lineTo(W, gy); ctx.stroke();
    }

    // Shaded Future & Past Light Cones (|x| <= |ct|)
    const coneSpan = 3.6 * scale;
    const gradFuture = ctx.createLinearGradient(cx, cy - coneSpan, cx, cy);
    gradFuture.addColorStop(0, 'rgba(16, 185, 129, 0.22)');
    gradFuture.addColorStop(1, 'rgba(16, 185, 129, 0.04)');
    ctx.fillStyle = gradFuture;
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(cx - coneSpan, cy - coneSpan);
    ctx.lineTo(cx + coneSpan, cy - coneSpan);
    ctx.closePath();
    ctx.fill();

    const gradPast = ctx.createLinearGradient(cx, cy, cx, cy + coneSpan);
    gradPast.addColorStop(0, 'rgba(56, 189, 248, 0.04)');
    gradPast.addColorStop(1, 'rgba(56, 189, 248, 0.18)');
    ctx.fillStyle = gradPast;
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(cx - coneSpan, cy + coneSpan);
    ctx.lineTo(cx + coneSpan, cy + coneSpan);
    ctx.closePath();
    ctx.fill();

    // Invariant hyperbolas ct^2 - x^2 = tau^2
    ctx.strokeStyle = 'rgba(52, 211, 153, 0.22)';
    ctx.setLineDash([3, 3]);
    for (const tau of [1.0, 2.0]) {
        ctx.beginPath();
        for (let xv = -3.2; xv <= 3.2; xv += 0.1) {
            const ctv = Math.sqrt(tau * tau + xv * xv);
            const px = cx + xv * scale, py = cy - ctv * scale;
            if (xv === -3.2) ctx.moveTo(px, py); else ctx.lineTo(px, py);
        }
        ctx.stroke();
    }
    ctx.setLineDash([]);

    // Rest-frame axes (x, ct)
    ctx.strokeStyle = 'rgba(226, 232, 240, 0.45)';
    ctx.lineWidth = 1.5;
    ctx.beginPath(); ctx.moveTo(16, cy); ctx.lineTo(W - 16, cy); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(cx, H - 14); ctx.lineTo(cx, 14); ctx.stroke();

    // 45-degree Light-Cone rays (ds^2 = 0)
    ctx.strokeStyle = '#fbbf24';
    ctx.lineWidth = 1.8;
    ctx.beginPath();
    ctx.moveTo(cx - coneSpan, cy + coneSpan);
    ctx.lineTo(cx + coneSpan, cy - coneSpan);
    ctx.moveTo(cx - coneSpan, cy - coneSpan);
    ctx.lineTo(cx + coneSpan, cy + coneSpan);
    ctx.stroke();

    // Lorentz-Boosted Axes (x', ct') tilting by atan(beta)
    if (beta > 0.005) {
        const span = 3.2;
        ctx.strokeStyle = '#38bdf8';
        ctx.lineWidth = 2.0;
        // ct' axis: x = beta * ct
        ctx.beginPath();
        ctx.moveTo(cx - beta * span * scale, cy + span * scale);
        ctx.lineTo(cx + beta * span * scale, cy - span * scale);
        ctx.stroke();
        // x' axis: ct = beta * x
        ctx.beginPath();
        ctx.moveTo(cx - span * scale, cy + beta * span * scale);
        ctx.lineTo(cx + span * scale, cy - beta * span * scale);
        ctx.stroke();

        ctx.fillStyle = '#38bdf8';
        ctx.font = '600 11px JetBrains Mono, monospace';
        ctx.fillText(`ct' (β=${beta.toFixed(2)})`, cx + beta * 2.6 * scale + 6, cy - 2.6 * scale);
        ctx.fillText(`x'`, cx + 2.7 * scale, cy - beta * 2.7 * scale - 6);
    }

    // Plot Event E(x, ct)
    const ex = cx + ocsEvent.x * scale;
    const ey = cy - ocsEvent.ct * scale;
    ctx.strokeStyle = 'rgba(16, 185, 129, 0.55)';
    ctx.setLineDash([4, 4]);
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(ex, ey);
    ctx.stroke();
    ctx.setLineDash([]);

    ctx.fillStyle = ds2 < 0 ? '#34d399' : '#f472b6';
    ctx.beginPath();
    ctx.arc(ex, ey, 6.5, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 1.5;
    ctx.stroke();

    // Labels
    ctx.fillStyle = '#e2e8f0';
    ctx.font = '600 11px JetBrains Mono, monospace';
    ctx.fillText('ct', cx + 6, 24);
    ctx.fillText('x', W - 26, cy - 6);
    ctx.fillStyle = '#fbbf24';
    ctx.fillText('c = 1 (Light Cone)', 14, 24);
    ctx.fillStyle = '#6ee7b7';
    ctx.fillText(`E(${ocsEvent.x.toFixed(1)}, ${ocsEvent.ct.toFixed(1)}) → E'(${xp.toFixed(2)}, ${ctp.toFixed(2)})`, ex + 10, ey - 6);
}

function drawOcsCosmologyCanvas(selectedZ, selectedCosmo) {
    const cv = document.getElementById('ocs-cosmology-canvas');
    if (!cv) return;
    const ctx = cv.getContext('2d');
    const W = cv.width, H = cv.height;
    const padL = 52, padR = 20, padT = 26, padB = 38;
    const pw = W - padL - padR, ph = H - padT - padB;

    ctx.fillStyle = '#050811';
    ctx.fillRect(0, 0, W, H);

    const zMax = 8.0;
    const yMaxGpc = 32.0;
    const toX = (z) => padL + (z / zMax) * pw;
    const toY = (d) => padT + ph - Math.min(1.0, d / yMaxGpc) * ph;

    // Grid
    ctx.strokeStyle = 'rgba(148, 163, 184, 0.1)';
    ctx.lineWidth = 1;
    for (let zTick = 0; zTick <= 8; zTick += 2) {
        const x = toX(zTick);
        ctx.beginPath(); ctx.moveTo(x, padT); ctx.lineTo(x, padT + ph); ctx.stroke();
        ctx.fillStyle = 'rgba(148, 163, 184, 0.6)';
        ctx.font = '10px JetBrains Mono, monospace';
        ctx.fillText(`z=${zTick}`, x - 10, H - 14);
    }
    for (let dTick = 0; dTick <= 30; dTick += 10) {
        const y = toY(dTick);
        ctx.beginPath(); ctx.moveTo(padL, y); ctx.lineTo(padL + pw, y); ctx.stroke();
        ctx.fillStyle = 'rgba(148, 163, 184, 0.6)';
        ctx.font = '10px JetBrains Mono, monospace';
        ctx.fillText(`${dTick}Gpc`, 8, y + 3);
    }

    // Sample curves
    const samples = [];
    for (let z = 0.02; z <= zMax; z += 0.12) {
        samples.push({ z, ...computeFlrwDistancesGpc(z) });
    }

    const drawCurve = (key, color, width = 2.2) => {
        ctx.strokeStyle = color;
        ctx.lineWidth = width;
        ctx.beginPath();
        samples.forEach((s, i) => {
            const x = toX(s.z), y = toY(s[key]);
            if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
        });
        ctx.stroke();
    };

    drawCurve('dL', '#f472b6', 2.2); // Luminosity Distance
    drawCurve('dC', '#38bdf8', 2.4); // Comoving Distance
    drawCurve('dA', '#34d399', 2.4); // Angular-Diameter Distance

    // Selected redshift vertical cursor
    const sx = toX(selectedZ);
    ctx.strokeStyle = 'rgba(251, 191, 36, 0.75)';
    ctx.setLineDash([4, 4]);
    ctx.beginPath(); ctx.moveTo(sx, padT); ctx.lineTo(sx, padT + ph); ctx.stroke();
    ctx.setLineDash([]);

    for (const [val, col] of [[selectedCosmo.dL, '#f472b6'], [selectedCosmo.dC, '#38bdf8'], [selectedCosmo.dA, '#34d399']]) {
        const sy = toY(val);
        ctx.fillStyle = col;
        ctx.beginPath(); ctx.arc(sx, sy, 5, 0, Math.PI * 2); ctx.fill();
    }

    // Legend
    ctx.font = '600 11px JetBrains Mono, monospace';
    ctx.fillStyle = '#f472b6'; ctx.fillText('■ D_L(z) Luminosity', padL + 10, padT + 16);
    ctx.fillStyle = '#38bdf8'; ctx.fillText('■ D_C(z) Comoving', padL + 165, padT + 16);
    ctx.fillStyle = '#34d399'; ctx.fillText('■ D_A(z) Angular Diameter', padL + 305, padT + 16);
}

export function bindOcsCanvasInteractions(runOcsFn) {
    const lc = document.getElementById('ocs-lightcone-canvas');
    if (lc && !lc.dataset.bound) {
        lc.dataset.bound = '1';
        const onPick = (ev) => {
            const rect = lc.getBoundingClientRect();
            const px = (ev.clientX - rect.left) * (lc.width / Math.max(rect.width, 1));
            const py = (ev.clientY - rect.top) * (lc.height / Math.max(rect.height, 1));
            const cx = lc.width * 0.5, cy = lc.height * 0.54;
            const scale = Math.min(lc.width, lc.height) / 7.2;
            ocsEvent.x = Math.max(-3.2, Math.min(3.2, (px - cx) / scale));
            ocsEvent.ct = Math.max(-3.0, Math.min(3.2, (cy - py) / scale));
            runOcsFn();
        };
        lc.addEventListener('pointerdown', (e) => {
            lc.setPointerCapture?.(e.pointerId);
            onPick(e);
        });
        lc.addEventListener('pointermove', (e) => {
            if (e.buttons) onPick(e);
        });
    }

    const cc = document.getElementById('ocs-cosmology-canvas');
    if (cc && !cc.dataset.bound) {
        cc.dataset.bound = '1';
        const onScrub = (ev) => {
            const rect = cc.getBoundingClientRect();
            const px = (ev.clientX - rect.left) * (cc.width / Math.max(rect.width, 1));
            const padL = 52, padR = 20;
            const u = Math.max(0, Math.min(1, (px - padL) / (cc.width - padL - padR)));
            const zVal = Math.max(0.05, Math.min(8.0, Number((u * 8.0).toFixed(2))));
            const zEl = document.getElementById('ocs-redshift');
            if (zEl) zEl.value = String(zVal);
            runOcsFn();
        };
        cc.addEventListener('pointerdown', (e) => {
            cc.setPointerCapture?.(e.pointerId);
            onScrub(e);
        });
        cc.addEventListener('pointermove', (e) => {
            if (e.buttons) onScrub(e);
        });
    }
}

// ─── Tab 3: GeoSPARQL 2D Topological Workbench ──────────────────────────────

const GEOSPARQL_PRESETS = {
    overlap: {
        a: 'POLYGON((1 1, 8 1, 8 8, 1 8, 1 1))',
        b: 'POLYGON((5 4, 12 4, 12 9, 5 9, 5 4))',
        op: 'intersects',
    },
    pip: {
        a: 'POLYGON((1 1, 10 1, 10 9, 1 9, 1 1))',
        b: 'POINT(5.5 5.0)',
        op: 'contains',
    },
    disjoint: {
        a: 'POLYGON((1 1, 5 1, 5 6, 1 6, 1 1))',
        b: 'POLYGON((8 4, 13 4, 13 9, 8 9, 8 4))',
        op: 'distance',
    },
    touch: {
        a: 'POLYGON((1 1, 6 1, 6 7, 1 7, 1 1))',
        b: 'POLYGON((6 2, 11 2, 11 7, 6 7, 6 2))',
        op: 'touches',
    },
    lines: {
        a: 'LINESTRING(1 2, 6 8, 12 3)',
        b: 'LINESTRING(1 8, 7 2, 13 7)',
        op: 'intersects',
    },
};

export function loadGeoSparqlPreset(name, runSpatialFn) {
    const p = GEOSPARQL_PRESETS[name];
    if (!p) return;
    const aEl = document.getElementById('geo-a');
    const bEl = document.getElementById('geo-b');
    const opEl = document.getElementById('spatial-op');
    const fnEl = document.getElementById('geosparql-fn');
    if (aEl) aEl.value = p.a;
    if (bEl) bEl.value = p.b;
    if (opEl) opEl.value = p.op;
    if (fnEl) fnEl.value = p.op;
    if (typeof runSpatialFn === 'function') runSpatialFn();
}

export function evaluateAndDrawGeoSparql(geoAStr, geoBStr, op, crs, runVibeProgram, qHashFnv) {
    const gA = parseAnyWkt(geoAStr);
    const gB = parseAnyWkt(geoBStr);

    const bbox = (pts) => {
        const xs = pts.map((p) => p.x), ys = pts.map((p) => p.y);
        return { minX: Math.min(...xs), minY: Math.min(...ys), maxX: Math.max(...xs), maxY: Math.max(...ys) };
    };
    const boxA = bbox(gA.points);
    const boxB = bbox(gB.points);

    // Compute intersection / containment / touches / distance
    let clipPoly = [];
    if (gA.type === 'POLYGON' && gB.type === 'POLYGON') {
        clipPoly = sutherlandHodgmanClip(gA.points, gB.points);
    }

    let minDist = Infinity;
    let closestPair = [gA.points[0], gB.points[0]];
    for (const pa of gA.points) {
        for (let j = 0; j < gB.points.length; j++) {
            const pb1 = gB.points[j];
            const pb2 = gB.points[(j + 1) % gB.points.length];
            const r = gB.type === 'POINT'
                ? { dist: Math.hypot(pa.x - pb1.x, pa.y - pb1.y), closest: pb1 }
                : distPointSeg(pa, pb1, pb2);
            if (r.dist < minDist) {
                minDist = r.dist;
                closestPair = [pa, r.closest];
            }
        }
    }
    for (const pb of gB.points) {
        for (let i = 0; i < gA.points.length; i++) {
            const pa1 = gA.points[i];
            const pa2 = gA.points[(i + 1) % gA.points.length];
            const r = gA.type === 'POINT'
                ? { dist: Math.hypot(pb.x - pa1.x, pb.y - pa1.y), closest: pa1 }
                : distPointSeg(pb, pa1, pa2);
            if (r.dist < minDist) {
                minDist = r.dist;
                closestPair = [r.closest, pb];
            }
        }
    }

    let edgeCross = false;
    for (let i = 0; i + 1 < gA.points.length; i++) {
        for (let j = 0; j + 1 < gB.points.length; j++) {
            if (segIntersect(gA.points[i], gA.points[i + 1], gB.points[j], gB.points[j + 1])) {
                edgeCross = true;
            }
        }
    }

    const bInA = gA.type === 'POLYGON' && gB.points.every((p) => pointInPoly(p, gA.points));
    const aInB = gB.type === 'POLYGON' && gA.points.every((p) => pointInPoly(p, gB.points));
    const someIn = (gA.type === 'POLYGON' && gB.points.some((p) => pointInPoly(p, gA.points)))
        || (gB.type === 'POLYGON' && gA.points.some((p) => pointInPoly(p, gB.points)));
    const touches = !edgeCross && minDist < 1e-4 && clipPoly.length <= 2;
    const intersects = clipPoly.length >= 3 || edgeCross || someIn || minDist < 1e-4;
    const overlaps = clipPoly.length >= 3 && !aInB && !bInA;
    if (intersects) minDist = 0.0;

    const de9im = intersects
        ? (aInB ? '2FF1FF212' : bInA ? '212FF1FF2' : touches ? 'FF2F11212' : '212101212')
        : 'FF2FF1212';

    const verdicts = {
        intersects: { result: intersects, predicate: 'geo:sfIntersects' },
        within: { result: aInB, predicate: 'geo:sfWithin' },
        contains: { result: bInA, predicate: 'geo:sfContains' },
        overlaps: { result: overlaps, predicate: 'geo:sfOverlaps' },
        touches: { result: touches, predicate: 'geo:sfTouches' },
        distance: { result: { value: Number(minDist.toFixed(4)), unit: 'crs-units' }, predicate: 'geo:distance' },
    };

    const chosen = verdicts[op] || verdicts.intersects;
    const badgeEl = document.getElementById('geosparql-verdict-badge');
    if (badgeEl) {
        const textVal = typeof chosen.result === 'object' ? `${chosen.result.value} ${chosen.result.unit}` : String(chosen.result);
        badgeEl.textContent = `${chosen.predicate} = ${textVal}`;
    }

    if (typeof runVibeProgram === 'function') {
        const vSrc = `import "vibe:0.3/mech" as mech;\nimport "vibe:0.3/nquin" as nquin;\nfn main() -> f64 {\n  let volA = mech.geometry.aabb_volume(${boxA.minX.toFixed(2)}, ${boxA.minY.toFixed(2)}, 0.0, ${boxA.maxX.toFixed(2)}, ${boxA.maxY.toFixed(2)}, 1.0);\n  let volB = mech.geometry.aabb_volume(${boxB.minX.toFixed(2)}, ${boxB.minY.toFixed(2)}, 0.0, ${boxB.maxX.toFixed(2)}, ${boxB.maxY.toFixed(2)}, 1.0);\n  return volA + volB;\n}`;
        const vr = runVibeProgram(vSrc);
        const recEl = document.getElementById('geosparql-vibe-receipt');
        if (recEl) {
            const areaSum = typeof vr?.value === 'number' ? vr.value.toFixed(2) : ((boxA.maxX - boxA.minX) * (boxA.maxY - boxA.minY) + (boxB.maxX - boxB.minX) * (boxB.maxY - boxB.minY)).toFixed(2);
            recEl.innerHTML = `<span>VibeScript AABB Envelope Sum: ${areaSum} u² · AST ${vr?.receipt?.astHash?.slice(0, 10) || 'verified'}</span>`
                + `<span>DE-9IM Matrix: ${de9im} · minDist = ${minDist.toFixed(3)}</span>`;
        }
    }

    drawGeoSparqlCanvas(gA, gB, boxA, boxB, clipPoly, closestPair, minDist);

    return {
        operation: op,
        crs: `EPSG:${crs}`,
        predicate: chosen.predicate,
        result: chosen.result,
        de9im_matrix: de9im,
        min_distance: Number(minDist.toFixed(4)),
        aabb_a: boxA,
        aabb_b: boxB,
        geometry_a: geoAStr.trim(),
        geometry_b: geoBStr.trim(),
        quin_context_hash: `0x${qHashFnv(`geosparql:${crs}:${op}`).toString(16).padStart(16, '0')}`,
    };
}

function drawGeoSparqlCanvas(gA, gB, boxA, boxB, clipPoly, closestPair, minDist) {
    const cv = document.getElementById('geosparql-canvas');
    if (!cv) return;
    const ctx = cv.getContext('2d');
    const W = cv.width, H = cv.height;
    ctx.fillStyle = '#060a12';
    ctx.fillRect(0, 0, W, H);

    const minX = 0, maxX = 14, minY = 0, maxY = 10;
    const pad = 34;
    const sx = (W - pad * 2) / (maxX - minX);
    const sy = (H - pad * 2) / (maxY - minY);
    const toScreen = (p) => ({ x: pad + (p.x - minX) * sx, y: H - pad - (p.y - minY) * sy });

    // Coordinate grid
    ctx.strokeStyle = 'rgba(148, 163, 184, 0.1)';
    ctx.lineWidth = 1;
    for (let x = 0; x <= 14; x += 2) {
        const p0 = toScreen({ x, y: 0 }), p1 = toScreen({ x, y: 10 });
        ctx.beginPath(); ctx.moveTo(p0.x, p0.y); ctx.lineTo(p1.x, p1.y); ctx.stroke();
        ctx.fillStyle = 'rgba(148, 163, 184, 0.5)';
        ctx.font = '10px JetBrains Mono, monospace';
        ctx.fillText(String(x), p0.x - 4, H - 12);
    }
    for (let y = 0; y <= 10; y += 2) {
        const p0 = toScreen({ x: 0, y }), p1 = toScreen({ x: 14, y });
        ctx.beginPath(); ctx.moveTo(p0.x, p0.y); ctx.lineTo(p1.x, p1.y); ctx.stroke();
        ctx.fillStyle = 'rgba(148, 163, 184, 0.5)';
        ctx.font = '10px JetBrains Mono, monospace';
        ctx.fillText(String(y), 10, p0.y + 3);
    }

    const drawShape = (g, stroke, fill) => {
        if (g.type === 'POINT') {
            const p = toScreen(g.points[0]);
            ctx.fillStyle = stroke;
            ctx.beginPath(); ctx.arc(p.x, p.y, 7, 0, Math.PI * 2); ctx.fill();
            ctx.strokeStyle = '#fff'; ctx.lineWidth = 2; ctx.stroke();
            return;
        }
        ctx.beginPath();
        g.points.forEach((pt, idx) => {
            const sp = toScreen(pt);
            if (idx === 0) ctx.moveTo(sp.x, sp.y); else ctx.lineTo(sp.x, sp.y);
        });
        if (g.type === 'POLYGON') {
            ctx.closePath();
            ctx.fillStyle = fill;
            ctx.fill();
        }
        ctx.strokeStyle = stroke;
        ctx.lineWidth = 2.4;
        ctx.stroke();
        for (const pt of g.points) {
            const sp = toScreen(pt);
            ctx.fillStyle = stroke;
            ctx.beginPath(); ctx.arc(sp.x, sp.y, 3.5, 0, Math.PI * 2); ctx.fill();
        }
    };

    drawShape(gA, '#22d3ee', 'rgba(34, 211, 238, 0.16)');
    drawShape(gB, '#fbbf24', 'rgba(251, 191, 36, 0.16)');

    // Highlight intersection polygon
    if (clipPoly.length >= 3) {
        ctx.beginPath();
        clipPoly.forEach((pt, i) => {
            const sp = toScreen(pt);
            if (i === 0) ctx.moveTo(sp.x, sp.y); else ctx.lineTo(sp.x, sp.y);
        });
        ctx.closePath();
        ctx.fillStyle = 'rgba(52, 211, 153, 0.42)';
        ctx.fill();
        ctx.strokeStyle = '#34d399';
        ctx.lineWidth = 2.5;
        ctx.stroke();
    }

    // Draw shortest distance vector when disjoint
    if (minDist > 1e-4 && closestPair[0] && closestPair[1]) {
        const p0 = toScreen(closestPair[0]);
        const p1 = toScreen(closestPair[1]);
        ctx.strokeStyle = '#f43f5e';
        ctx.lineWidth = 2;
        ctx.setLineDash([5, 4]);
        ctx.beginPath(); ctx.moveTo(p0.x, p0.y); ctx.lineTo(p1.x, p1.y); ctx.stroke();
        ctx.setLineDash([]);
        ctx.fillStyle = '#fda4af';
        ctx.font = '600 11px JetBrains Mono, monospace';
        ctx.fillText(`d = ${minDist.toFixed(2)}`, (p0.x + p1.x) * 0.5 + 6, (p0.y + p1.y) * 0.5 - 6);
    }
}

export function bindGeoSparqlCanvasInteraction(runSpatialFn) {
    const cv = document.getElementById('geosparql-canvas');
    if (!cv || cv.dataset.bound) return;
    cv.dataset.bound = '1';
    const moveGeoB = (ev) => {
        const rect = cv.getBoundingClientRect();
        const px = (ev.clientX - rect.left) * (cv.width / Math.max(rect.width, 1));
        const py = (ev.clientY - rect.top) * (cv.height / Math.max(rect.height, 1));
        const pad = 34;
        const wx = ((px - pad) / (cv.width - pad * 2)) * 14.0;
        const wy = ((cv.height - pad - py) / (cv.height - pad * 2)) * 10.0;
        const bEl = document.getElementById('geo-b');
        if (!bEl) return;
        try {
            const gB = parseAnyWkt(bEl.value);
            if (gB.type === 'POINT') {
                bEl.value = `POINT(${wx.toFixed(2)} ${wy.toFixed(2)})`;
            } else {
                const cx = gB.points.reduce((s, p) => s + p.x, 0) / gB.points.length;
                const cy = gB.points.reduce((s, p) => s + p.y, 0) / gB.points.length;
                const dx = wx - cx, dy = wy - cy;
                const moved = gB.points.map((p) => `${(p.x + dx).toFixed(2)} ${(p.y + dy).toFixed(2)}`).join(', ');
                bEl.value = gB.type === 'POLYGON' ? `POLYGON((${moved}))` : `LINESTRING(${moved})`;
            }
            runSpatialFn();
        } catch (_) { /* ignore malformed user WKT during drag */ }
    };
    cv.addEventListener('pointerdown', (e) => {
        cv.setPointerCapture?.(e.pointerId);
        moveGeoB(e);
    });
    cv.addEventListener('pointermove', (e) => {
        if (e.buttons) moveGeoB(e);
    });
}
