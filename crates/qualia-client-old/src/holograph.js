// Setup Three.js Scene
const container = document.getElementById('canvas-container');
const scene = new THREE.Scene();
scene.fog = new THREE.FogExp2(0x020202, 0.05);

const camera = new THREE.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000);
camera.position.z = 25;
camera.position.y = 5;

const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
renderer.setSize(window.innerWidth, window.innerHeight);
renderer.setPixelRatio(window.devicePixelRatio);
container.appendChild(renderer.domElement);

// Generate Minkowski Sparse Tensor Representation (Point Cloud)
// In a real system, this geometry is constructed dynamically from .q42 CBOR-LD graph mapping
const particleCount = 2048;
const geometry = new THREE.BufferGeometry();
const positions = new Float32Array(particleCount * 3);
const colors = new Float32Array(particleCount * 3);
const sizes = new Float32Array(particleCount);

const colorCyan = new THREE.Color(0x00f0ff);
const colorPurple = new THREE.Color(0xb026ff);

for (let i = 0; i < particleCount; i++) {
    // Abstract humanoid distribution
    const y = (Math.random() - 0.5) * 30;
    const radius = 5 * Math.sin((y + 15) / 30 * Math.PI) + (Math.random() * 2);
    const theta = Math.random() * Math.PI * 2;
    
    const x = radius * Math.cos(theta);
    const z = radius * Math.sin(theta);

    positions[i * 3] = x;
    positions[i * 3 + 1] = y;
    positions[i * 3 + 2] = z;

    // Color gradient based on Y axis
    const mixRatio = (y + 15) / 30;
    const vertexColor = colorPurple.clone().lerp(colorCyan, mixRatio);
    colors[i * 3] = vertexColor.r;
    colors[i * 3 + 1] = vertexColor.g;
    colors[i * 3 + 2] = vertexColor.b;
    
    sizes[i] = Math.random() * 0.5 + 0.1;
}

geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));
geometry.setAttribute('size', new THREE.BufferAttribute(sizes, 1));

// Custom Shader Material for pulsing particles
const material = new THREE.ShaderMaterial({
    uniforms: {
        time: { value: 0.0 },
        heartRate: { value: 72.0 }
    },
    vertexShader: `
        uniform float time;
        uniform float heartRate;
        attribute float size;
        attribute vec3 color;
        varying vec3 vColor;
        
        void main() {
            vColor = color;
            vec3 pos = position;
            
            // Biosignal pulse effect (chest cavity area approx y=5)
            float distToHeart = length(pos - vec3(0.0, 5.0, 2.0));
            float pulseFrequency = heartRate / 60.0 * 3.14159 * 2.0;
            float pulse = sin(time * pulseFrequency) * 0.5 + 0.5;
            
            if (distToHeart < 8.0) {
                pos += normal * pulse * (8.0 - distToHeart) * 0.1;
                vColor = mix(color, vec3(1.0, 0.2, 0.4), pulse * (1.0 - distToHeart/8.0));
            }
            
            vec4 mvPosition = modelViewMatrix * vec4(pos, 1.0);
            gl_PointSize = size * (300.0 / -mvPosition.z);
            gl_Position = projectionMatrix * mvPosition;
        }
    `,
    fragmentShader: `
        varying vec3 vColor;
        void main() {
            // Circular particle with soft edge
            float r = distance(gl_PointCoord, vec2(0.5));
            if (r > 0.5) discard;
            float alpha = smoothstep(0.5, 0.1, r);
            gl_FragColor = vec4(vColor, alpha * 0.8);
        }
    `,
    transparent: true,
    blending: THREE.AdditiveBlending,
    depthWrite: false
});

const holograph = new THREE.Points(geometry, material);
scene.add(holograph);

// Handle Window Resize
window.addEventListener('resize', onWindowResize, false);
function onWindowResize() {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
}

// Simulated Telemetry Stream (over IPC / WebSockets)
let currentHr = 72;
const hudHr = document.getElementById('hudHr');

setInterval(() => {
    // Simulate heart rate variability
    currentHr += (Math.random() - 0.5) * 4;
    currentHr = Math.max(60, Math.min(120, currentHr)); // Clamp 60-120
    hudHr.innerText = Math.round(currentHr) + " BPM";
}, 2000);

// Animation Loop
const clock = new THREE.Clock();
function animate() {
    requestAnimationFrame(animate);
    
    const time = clock.getElapsedTime();
    material.uniforms.time.value = time;
    material.uniforms.heartRate.value = currentHr;
    
    // Slow ambient rotation
    holograph.rotation.y = time * 0.2;
    
    renderer.render(scene, camera);
}

animate();
