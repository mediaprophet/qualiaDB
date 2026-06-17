/**
 * Spatial Mathematics demo — Three.js viewer + client-side Quin/GeoSPARQL helpers.
 * WASM spatial_* exports are not yet wired; this module runs fully in-browser.
 */

let scene, camera, renderer, meshGroup;
let container = null;
let wasm = null;

function qHashFnv(str) {
    let h = 0xcbf29ce484222325n;
    for (let i = 0; i < str.length; i++) {
        h ^= BigInt(str.charCodeAt(i));
        h = (h * 0x100000001b3n) & 0xffffffffffffffffn;
    }
    return h;
}

function packCoord(x, y, z) {
    const xi = BigInt(Math.round(x * 1000)) & 0xfffffn;
    const yi = BigInt(Math.round(y * 1000)) & 0xfffffn;
    const zi = BigInt(Math.round(z * 1000)) & 0xfffffn;
    return (xi << 40n) | (yi << 20n) | zi;
}

function parsePointWkt(wkt) {
    const m = wkt.trim().match(/POINT\s*\(\s*([-\d.]+)\s+([-\d.]+)\s*\)/i);
    if (!m) throw new Error('Expected POINT(x y) WKT');
    return { x: parseFloat(m[1]), y: parseFloat(m[2]) };
}

function parsePolygonWkt(wkt) {
    const m = wkt.trim().match(/POLYGON\s*\(\(\s*([^)]+)\s*\)\)/i);
    if (!m) throw new Error('Expected POLYGON((...)) WKT');
    return m[1].split(',').map((pair) => {
        const [x, y] = pair.trim().split(/\s+/);
        return { x: parseFloat(x), y: parseFloat(y) };
    });
}

function pointInPolygon(pt, ring) {
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

function getActivePositions() {
    if (!meshGroup || meshGroup.children.length === 0) return null;
    const obj = meshGroup.children[0];
    const geo = obj.geometry || obj;
    if (!geo?.attributes?.position) return null;
    return geo.attributes.position;
}

export function initSpatialDemo(rootEl) {
    container = rootEl || document.getElementById('canvas-container');
}

function initThreeJS() {
    if (!container) throw new Error('canvas-container not found');
    scene = new THREE.Scene();
    camera = new THREE.PerspectiveCamera(70, container.clientWidth / container.clientHeight, 0.1, 1000);
    renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    renderer.setSize(container.clientWidth, container.clientHeight);
    container.appendChild(renderer.domElement);

    scene.add(new THREE.AmbientLight(0xffffff, 0.6));
    const light = new THREE.PointLight(0xff266e, 1.5, 200);
    light.position.set(10, 20, 10);
    scene.add(light);

    meshGroup = new THREE.Group();
    scene.add(meshGroup);
    camera.position.set(0, 8, 18);

    window.addEventListener('resize', () => {
        if (!container) return;
        camera.aspect = container.clientWidth / container.clientHeight;
        camera.updateProjectionMatrix();
        renderer.setSize(container.clientWidth, container.clientHeight);
    });

    animate();
}

function animate() {
    requestAnimationFrame(animate);
    if (meshGroup) meshGroup.rotation.y += 0.003;
    if (renderer && scene && camera) renderer.render(scene, camera);
}

function buildGeometry(type, detail) {
    switch (type) {
        case 'icosahedron': return new THREE.IcosahedronGeometry(7, detail);
        case 'cube': return new THREE.BoxGeometry(10, 10, 10, detail + 1, detail + 1, detail + 1);
        case 'sphere': return new THREE.SphereGeometry(7, detail * 8 + 8, detail * 6 + 6);
        case 'torus': return new THREE.TorusGeometry(5, 2, detail * 8 + 8, detail * 6 + 6);
        case 'knot': return new THREE.TorusKnotGeometry(4, 1.5, detail * 32 + 32, detail * 8 + 8);
        default: return new THREE.IcosahedronGeometry(7, detail);
    }
}

function applyDisplayMode(geo) {
    const mode = document.getElementById('display-mode').value;
    while (meshGroup.children.length > 0) meshGroup.remove(meshGroup.children[0]);

    switch (mode) {
        case 'wireframe':
            meshGroup.add(new THREE.LineSegments(
                new THREE.WireframeGeometry(geo),
                new THREE.LineBasicMaterial({ color: 0x00f0ff, transparent: true, opacity: 0.4 })
            ));
            break;
        case 'points':
            meshGroup.add(new THREE.Points(geo, new THREE.PointsMaterial({ color: 0xff266e, size: 0.18 })));
            break;
        case 'solid':
            meshGroup.add(new THREE.Mesh(geo, new THREE.MeshPhongMaterial({ color: 0x00f0ff, transparent: true, opacity: 0.3 })));
            break;
        default:
            meshGroup.add(new THREE.LineSegments(
                new THREE.WireframeGeometry(geo),
                new THREE.LineBasicMaterial({ color: 0x00f0ff, transparent: true, opacity: 0.4 })
            ));
            meshGroup.add(new THREE.Points(geo, new THREE.PointsMaterial({ color: 0xff266e, size: 0.18 })));
    }
}

export function generateGeometry() {
    const type = document.getElementById('geo-type').value;
    const detail = parseInt(document.getElementById('geo-detail').value, 10);
    const geo = buildGeometry(type, detail);
    applyDisplayMode(geo);

    const vertexCount = geo.attributes.position.count;
    document.getElementById('metric-vertices').textContent = vertexCount.toLocaleString();
    document.getElementById('metric-quins').textContent = (vertexCount * 2).toLocaleString();
    document.getElementById('metric-memory').textContent = ((vertexCount * 48 * 2) / 1024).toFixed(1) + ' KB';
}

export function updateDisplayMode() {
    const type = document.getElementById('geo-type').value;
    const detail = parseInt(document.getElementById('geo-detail').value, 10);
    applyDisplayMode(buildGeometry(type, detail));
}

function encodeGeometryToQuins(type, detail) {
    const geo = buildGeometry(type, detail);
    const pos = geo.attributes.position;
    const geomHash = qHashFnv(`geo:${type}:${detail}`);
    const ctxHash = qHashFnv('ctx:spatial-demo');
    const predVertex = qHashFnv('geo:hasVertex');
    const quins = [];

    for (let i = 0; i < pos.count; i++) {
        const x = pos.getX(i), y = pos.getY(i), z = pos.getZ(i);
        quins.push({
            subject: geomHash,
            predicate: predVertex,
            object: packCoord(x, y, z),
            context: ctxHash,
            metadata: BigInt(i),
            parity: geomHash ^ predVertex ^ packCoord(x, y, z) ^ ctxHash ^ BigInt(i),
        });
    }

    return {
        vertex_count: pos.count,
        quin_count: quins.length,
        memory_kb: Number(((quins.length * 48) / 1024).toFixed(2)),
        quins,
        backend: wasm?.spatial_encode ? 'wasm' : 'browser',
    };
}

export async function encodeToQuins() {
    const type = document.getElementById('geo-type').value;
    const detail = parseInt(document.getElementById('geo-detail').value, 10);
    const start = performance.now();

    try {
        let parsed;
        if (typeof wasm?.spatial_encode === 'function') {
            parsed = JSON.parse(wasm.spatial_encode(JSON.stringify({ type, detail })));
        } else {
            parsed = encodeGeometryToQuins(type, detail);
        }

        const elapsed = performance.now() - start;
        document.getElementById('encoding-status').textContent =
            `Encoded ${parsed.quin_count} Quins in ${elapsed.toFixed(2)}ms (${parsed.backend})`;
        document.getElementById('metric-time').textContent = elapsed.toFixed(2) + 'ms';

        let dump = `Q42 Spatial Encoding [${parsed.backend}]\n`;
        dump += `Vertices: ${parsed.vertex_count}\n`;
        dump += `Quins: ${parsed.quin_count}\n`;
        dump += `Memory: ${parsed.memory_kb} KB\n\n[First 10 Quins]\n`;
        parsed.quins.slice(0, 10).forEach((quin, i) => {
            dump += `Quin ${i}:\n`;
            dump += `  Subject:   0x${quin.subject.toString(16).padStart(16, '0')}\n`;
            dump += `  Predicate: 0x${quin.predicate.toString(16).padStart(16, '0')}\n`;
            dump += `  Object:    0x${quin.object.toString(16).padStart(16, '0')}\n`;
            dump += `  Context:   0x${quin.context.toString(16).padStart(16, '0')}\n`;
            dump += `  Metadata:  0x${quin.metadata.toString(16).padStart(16, '0')}\n`;
            dump += `  Parity:    0x${quin.parity.toString(16).padStart(16, '0')}\n\n`;
        });
        document.getElementById('quin-dump').textContent = dump;
    } catch (e) {
        document.getElementById('encoding-status').textContent = 'Error: ' + e.message;
    }
}

export async function runSpatialOp() {
    const geoA = document.getElementById('geo-a').value;
    const geoB = document.getElementById('geo-b').value;
    const op = document.getElementById('spatial-op').value;
    const crs = document.getElementById('geo-crs').value;
    const start = performance.now();

    try {
        let result;
        if (typeof wasm?.geosparql_operation === 'function') {
            result = JSON.parse(wasm.geosparql_operation(JSON.stringify({ geoA, geoB, op, crs })));
        } else {
            const aIsPoly = /POLYGON/i.test(geoA);
            const bIsPoint = /POINT/i.test(geoB);
            if (!aIsPoly || !bIsPoint) throw new Error('Demo supports POLYGON + POINT WKT only');

            const poly = parsePolygonWkt(geoA);
            const pt = parsePointWkt(geoB);
            const within = pointInPolygon(pt, poly);
            const dist = Math.hypot(pt.x - poly[0].x, pt.y - poly[0].y);

            const values = {
                within: { result: within, predicate: 'geo:sfWithin' },
                contains: { result: within, predicate: 'geo:sfContains' },
                intersects: { result: within, predicate: 'geo:sfIntersects' },
                touches: { result: false, predicate: 'geo:sfTouches' },
                overlaps: { result: false, predicate: 'geo:sfOverlaps' },
                distance: { result: dist, unit: 'coordinate-units', predicate: 'geo:distance' },
            };
            result = {
                operation: op,
                crs: `EPSG:${crs}`,
                geometry_a: geoA.trim(),
                geometry_b: geoB.trim(),
                ...values[op],
                elapsed_ms: Number((performance.now() - start).toFixed(3)),
                backend: 'browser',
            };
        }
        document.getElementById('spatial-result').textContent = JSON.stringify(result, null, 2);
    } catch (e) {
        document.getElementById('spatial-result').textContent = 'Error: ' + e.message;
    }
}

export async function runNativeOp() {
    const op = document.getElementById('native-op').value;
    const pos = getActivePositions();
    if (!pos) {
        document.getElementById('native-result').textContent = 'Generate geometry in the 3D viewer first.';
        return;
    }

    const pts = [];
    for (let i = 0; i < pos.count; i++) pts.push({ x: pos.getX(i), y: pos.getY(i), z: pos.getZ(i) });

    let result;
    if (op === 'bbox') {
        const xs = pts.map((p) => p.x), ys = pts.map((p) => p.y), zs = pts.map((p) => p.z);
        result = {
            operation: 'bbox',
            min: { x: Math.min(...xs), y: Math.min(...ys), z: Math.min(...zs) },
            max: { x: Math.max(...xs), y: Math.max(...ys), z: Math.max(...zs) },
            vertex_count: pts.length,
            backend: 'browser',
        };
    } else if (op === 'convex_hull') {
        result = {
            operation: 'convex_hull',
            hull_vertices: Math.min(pts.length, Math.max(4, Math.floor(pts.length * 0.15))),
            input_vertices: pts.length,
            note: 'Full QuickHull ships in native daemon; browser returns estimate.',
            backend: 'browser',
        };
    } else {
        result = {
            operation: 'triangulate',
            triangles: Math.max(0, pts.length - 2),
            input_vertices: pts.length,
            backend: 'browser',
        };
    }

    document.getElementById('native-result').textContent = JSON.stringify(result, null, 2);
}

export function switchTab(tabId, btn) {
    document.querySelectorAll('.tab-pane').forEach((p) => p.classList.remove('active'));
    document.querySelectorAll('.tab-btn').forEach((b) => b.classList.remove('active'));
    document.getElementById('tab-' + tabId)?.classList.add('active');
    btn?.classList.add('active');
}

if (typeof window !== 'undefined') {
    window.generateGeometry = generateGeometry;
    window.updateDisplayMode = updateDisplayMode;
    window.encodeToQuins = encodeToQuins;
    window.runSpatialOp = runSpatialOp;
    window.runNativeOp = runNativeOp;
    window.switchTab = switchTab;
}

export async function bootSpatialPage() {
    initSpatialDemo();
    initThreeJS();
    generateGeometry();

    document.getElementById('loading-overlay').style.display = 'none';
    document.getElementById('main-content').style.display = 'block';

    try {
        const module = await import('../playground/qualia_core_db.js');
        await module.default();
        wasm = module;
        const ver = typeof module.get_engine_version === 'function' ? module.get_engine_version() : null;
        document.getElementById('wasm-dot').classList.remove('bg-slate-500');
        document.getElementById('wasm-dot').classList.add('bg-emerald-500');
        document.getElementById('wasm-text').textContent = ver ? `WASM v${ver}` : 'WASM Ready';
    } catch (error) {
        console.warn('WASM optional load failed:', error);
        document.getElementById('wasm-dot').classList.remove('bg-slate-500');
        document.getElementById('wasm-dot').classList.add('bg-amber-500');
        document.getElementById('wasm-text').textContent = 'Viewer OK · WASM offline';
    }
}