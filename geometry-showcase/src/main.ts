import './style.css';
import init, { wasm_convex_hull_2d, wasm_delaunay_triangulation_2d } from '../pkg/qualia_core_db.js';

let wasmLoaded = false;
let currentAlgo = 'convex-hull';
let points: number[] = []; // flat array [x, y, x, y, ...]

const canvas = document.getElementById('geometry-canvas') as HTMLCanvasElement;
const ctx = canvas.getContext('2d')!;

const statPoints = document.getElementById('stat-points')!;
const statTime = document.getElementById('stat-time')!;

// Handle resize
function resizeCanvas() {
  const rect = canvas.parentElement!.getBoundingClientRect();
  canvas.width = rect.width;
  canvas.height = rect.height;
  redraw();
}

window.addEventListener('resize', resizeCanvas);

// Bind UI
document.querySelectorAll('.controls button[data-algo]').forEach(btn => {
  btn.addEventListener('click', (e) => {
    document.querySelectorAll('.controls button[data-algo]').forEach(b => b.classList.remove('active'));
    const target = e.target as HTMLButtonElement;
    target.classList.add('active');
    currentAlgo = target.dataset.algo!;
    redraw();
  });
});

document.getElementById('btn-clear')!.addEventListener('click', () => {
  points = [];
  redraw();
});

document.getElementById('btn-random')!.addEventListener('click', () => {
  points = [];
  for (let i = 0; i < 50; i++) {
    points.push(Math.random() * canvas.width);
    points.push(Math.random() * canvas.height);
  }
  redraw();
});

// Canvas Interaction
canvas.addEventListener('mousedown', (e) => {
  const rect = canvas.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;
  points.push(x, y);
  redraw();
});

function drawPoints() {
  ctx.fillStyle = '#3b82f6';
  for (let i = 0; i < points.length; i += 2) {
    ctx.beginPath();
    ctx.arc(points[i], points[i + 1], 4, 0, Math.PI * 2);
    ctx.fill();
  }
}

function redraw() {
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  statPoints.textContent = (points.length / 2).toString();
  
  if (!wasmLoaded || points.length < 6) { // Need at least 3 points
    drawPoints();
    statTime.textContent = '0';
    return;
  }
  
  const t0 = performance.now();
  const pointsFlat = new Float64Array(points);
  
  ctx.strokeStyle = '#10b981';
  ctx.lineWidth = 2;
  
  try {
    if (currentAlgo === 'convex-hull') {
      const hullIndices = wasm_convex_hull_2d(pointsFlat);
      if (hullIndices.length > 0) {
        ctx.beginPath();
        const startX = points[hullIndices[0] * 2];
        const startY = points[hullIndices[0] * 2 + 1];
        ctx.moveTo(startX, startY);
        
        for (let i = 1; i < hullIndices.length; i++) {
          const idx = hullIndices[i];
          ctx.lineTo(points[idx * 2], points[idx * 2 + 1]);
        }
        ctx.closePath();
        ctx.fillStyle = 'rgba(16, 185, 129, 0.1)';
        ctx.fill();
        ctx.stroke();
      }
    } else if (currentAlgo === 'delaunay') {
      const tris = wasm_delaunay_triangulation_2d(pointsFlat);
      ctx.beginPath();
      for (let i = 0; i < tris.length; i += 3) {
        const i1 = tris[i] * 2;
        const i2 = tris[i+1] * 2;
        const i3 = tris[i+2] * 2;
        
        ctx.moveTo(points[i1], points[i1+1]);
        ctx.lineTo(points[i2], points[i2+1]);
        ctx.lineTo(points[i3], points[i3+1]);
        ctx.lineTo(points[i1], points[i1+1]);
      }
      ctx.stroke();
    }
  } catch (e) {
    console.error("WASM Error:", e);
  }
  
  drawPoints();
  const t1 = performance.now();
  statTime.textContent = (t1 - t0).toFixed(2);
}

async function boot() {
  try {
    await init();
    wasmLoaded = true;
    resizeCanvas(); // Will trigger redraw
  } catch (e) {
    console.error("Failed to load WASM:", e);
  }
}

boot();
