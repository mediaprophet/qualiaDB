// Qualia-DB 3D Anatomy Renderer (HuBMAP CCF JSON-LD Parser)

const ONTOLOGY_GRAPH = {
  "@context": "https://hubmapconsortium.github.io/hubmap-ontology/ccf-context.jsonld",
  "@graph": [
    // --- NERVOUS SYSTEM (Brain) ---
    {
      "@id": "ccf:VHM_Brain", "name": "VHM Brain", "system": "nervous", "sex": "male", "color": 0xbd93f9,
      "ccf:x_dimension": 140, "ccf:y_dimension": 150, "ccf:z_dimension": 160,
      "ccf:placement": { "ccf:x_translation": 0, "ccf:y_translation": 700, "ccf:z_translation": 0 }
    },
    {
      "@id": "ccf:VHF_Brain", "name": "VHF Brain", "system": "nervous", "sex": "female", "color": 0xbd93f9,
      "ccf:x_dimension": 130, "ccf:y_dimension": 140, "ccf:z_dimension": 150,
      "ccf:placement": { "ccf:x_translation": 0, "ccf:y_translation": 660, "ccf:z_translation": 0 }
    },
    
    // --- CARDIOVASCULAR (Heart) ---
    {
      "@id": "ccf:VHM_Heart", "name": "VHM Heart", "system": "cardio", "sex": "male", "color": 0xff5555,
      "ccf:x_dimension": 90, "ccf:y_dimension": 100, "ccf:z_dimension": 80,
      "ccf:placement": { "ccf:x_translation": 0, "ccf:y_translation": 450, "ccf:z_translation": 20 }
    },
    {
      "@id": "ccf:VHF_Heart", "name": "VHF Heart", "system": "cardio", "sex": "female", "color": 0xff5555,
      "ccf:x_dimension": 80, "ccf:y_dimension": 90, "ccf:z_dimension": 70,
      "ccf:placement": { "ccf:x_translation": 0, "ccf:y_translation": 430, "ccf:z_translation": 20 }
    },

    // --- RESPIRATORY (Lungs) ---
    {
      "@id": "ccf:VHM_Left_Lung", "name": "VHM Left Lung", "system": "respiratory", "sex": "male", "color": 0x8be9fd,
      "ccf:x_dimension": 110, "ccf:y_dimension": 220, "ccf:z_dimension": 130,
      "ccf:placement": { "ccf:x_translation": -70, "ccf:y_translation": 450, "ccf:z_translation": 0 }
    },
    {
      "@id": "ccf:VHM_Right_Lung", "name": "VHM Right Lung", "system": "respiratory", "sex": "male", "color": 0x8be9fd,
      "ccf:x_dimension": 110, "ccf:y_dimension": 220, "ccf:z_dimension": 130,
      "ccf:placement": { "ccf:x_translation": 70, "ccf:y_translation": 450, "ccf:z_translation": 0 }
    },
    {
      "@id": "ccf:VHF_Left_Lung", "name": "VHF Left Lung", "system": "respiratory", "sex": "female", "color": 0x8be9fd,
      "ccf:x_dimension": 100, "ccf:y_dimension": 200, "ccf:z_dimension": 120,
      "ccf:placement": { "ccf:x_translation": -65, "ccf:y_translation": 430, "ccf:z_translation": 0 }
    },
    {
      "@id": "ccf:VHF_Right_Lung", "name": "VHF Right Lung", "system": "respiratory", "sex": "female", "color": 0x8be9fd,
      "ccf:x_dimension": 100, "ccf:y_dimension": 200, "ccf:z_dimension": 120,
      "ccf:placement": { "ccf:x_translation": 65, "ccf:y_translation": 430, "ccf:z_translation": 0 }
    },

    // --- URINARY (Kidneys) ---
    {
      "@id": "ccf:VHM_Left_Kidney", "name": "VHM Left Kidney", "system": "urinary", "sex": "male", "color": 0xf1fa8c,
      "ccf:x_dimension": 63.5, "ccf:y_dimension": 113.8, "ccf:z_dimension": 55.2,
      "ccf:placement": { "ccf:x_translation": -51.0, "ccf:y_translation": 250, "ccf:z_translation": -30 }
    },
    {
      "@id": "ccf:VHM_Right_Kidney", "name": "VHM Right Kidney", "system": "urinary", "sex": "male", "color": 0xf1fa8c,
      "ccf:x_dimension": 63.5, "ccf:y_dimension": 113.8, "ccf:z_dimension": 55.2,
      "ccf:placement": { "ccf:x_translation": 51.0, "ccf:y_translation": 250, "ccf:z_translation": -30 }
    },
    {
      "@id": "ccf:VHF_Left_Kidney", "name": "VHF Left Kidney", "system": "urinary", "sex": "female", "color": 0xf1fa8c,
      "ccf:x_dimension": 58.0, "ccf:y_dimension": 105.0, "ccf:z_dimension": 50.0,
      "ccf:placement": { "ccf:x_translation": -48.0, "ccf:y_translation": 230, "ccf:z_translation": -25 }
    },
    {
      "@id": "ccf:VHF_Right_Kidney", "name": "VHF Right Kidney", "system": "urinary", "sex": "female", "color": 0xf1fa8c,
      "ccf:x_dimension": 58.0, "ccf:y_dimension": 105.0, "ccf:z_dimension": 50.0,
      "ccf:placement": { "ccf:x_translation": 48.0, "ccf:y_translation": 230, "ccf:z_translation": -25 }
    }
  ]
};

let currentSex = 'male';
let scene, camera, renderer, controls;
let organMeshes = [];
const tooltip = document.getElementById('tooltip');

function initThreeJS() {
    const container = document.getElementById('canvas-container');

    scene = new THREE.Scene();
    
    // Setup Camera
    camera = new THREE.PerspectiveCamera(45, container.clientWidth / container.clientHeight, 1, 3000);
    camera.position.set(0, 400, 800);

    // Setup Renderer
    renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    renderer.setSize(container.clientWidth, container.clientHeight);
    renderer.setPixelRatio(window.devicePixelRatio);
    container.appendChild(renderer.domElement);

    // Lighting
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.4);
    scene.add(ambientLight);
    
    const pointLight = new THREE.PointLight(0x00f0ff, 1);
    pointLight.position.set(200, 500, 300);
    scene.add(pointLight);

    const backLight = new THREE.PointLight(0xff00ff, 0.8);
    backLight.position.set(-200, 200, -300);
    scene.add(backLight);

    // Controls
    controls = new THREE.OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.05;
    controls.target.set(0, 400, 0);

    // Grid helper
    const gridHelper = new THREE.GridHelper(1000, 20, 0x00f0ff, 0x333333);
    gridHelper.material.opacity = 0.2;
    gridHelper.material.transparent = true;
    scene.add(gridHelper);

    // Raycaster for tooltips
    const raycaster = new THREE.Raycaster();
    const mouse = new THREE.Vector2();

    container.addEventListener('mousemove', (event) => {
        const rect = container.getBoundingClientRect();
        mouse.x = ((event.clientX - rect.left) / container.clientWidth) * 2 - 1;
        mouse.y = -((event.clientY - rect.top) / container.clientHeight) * 2 + 1;

        raycaster.setFromCamera(mouse, camera);
        const intersects = raycaster.intersectObjects(organMeshes);

        if (intersects.length > 0) {
            const mesh = intersects[0].object;
            tooltip.style.display = 'block';
            tooltip.style.left = event.clientX + 'px';
            tooltip.style.top = event.clientY + 'px';
            tooltip.innerHTML = `<strong>${mesh.userData.name}</strong><br>ID: ${mesh.userData.id}<br>Routing: Spatiotemporal (0b11)`;
        } else {
            tooltip.style.display = 'none';
        }
    });

    window.addEventListener('resize', () => {
        camera.aspect = container.clientWidth / container.clientHeight;
        camera.updateProjectionMatrix();
        renderer.setSize(container.clientWidth, container.clientHeight);
    });

    animate();
    updateRender();
}

function buildMaterial(colorHex) {
    return new THREE.MeshPhysicalMaterial({
        color: colorHex,
        metalness: 0.1,
        roughness: 0.1,
        transparent: true,
        opacity: 0.6,
        transmission: 0.5,
        clearcoat: 1.0,
        emissive: colorHex,
        emissiveIntensity: 0.2
    });
}

function updateRender() {
    // Clear old meshes
    organMeshes.forEach(mesh => {
        scene.remove(mesh);
        mesh.geometry.dispose();
        mesh.material.dispose();
    });
    organMeshes = [];

    // Check UI states
    const activeSystems = {
        nervous: document.getElementById('sys-nervous').checked,
        cardio: document.getElementById('sys-cardio').checked,
        respiratory: document.getElementById('sys-respiratory').checked,
        urinary: document.getElementById('sys-urinary').checked
    };

    // Filter ontology graph
    const renderTargets = ONTOLOGY_GRAPH["@graph"].filter(node => {
        return node.sex === currentSex && activeSystems[node.system];
    });

    // Generate meshes
    renderTargets.forEach(node => {
        const geometry = new THREE.BoxGeometry(
            node["ccf:x_dimension"], 
            node["ccf:y_dimension"], 
            node["ccf:z_dimension"]
        );
        
        const material = buildMaterial(node.color);
        const mesh = new THREE.Mesh(geometry, material);
        
        const edges = new THREE.EdgesGeometry(geometry);
        const line = new THREE.LineSegments(edges, new THREE.LineBasicMaterial({ color: node.color, transparent: true, opacity: 0.8 }));
        mesh.add(line);

        mesh.position.set(
            node["ccf:placement"]["ccf:x_translation"],
            node["ccf:placement"]["ccf:y_translation"],
            node["ccf:placement"]["ccf:z_translation"]
        );

        mesh.userData = { id: node["@id"], name: node.name };
        scene.add(mesh);
        organMeshes.push(mesh);
    });
}

window.setSex = function(sex) {
    currentSex = sex;
    document.getElementById('btn-male').classList.toggle('active', sex === 'male');
    document.getElementById('btn-female').classList.toggle('active', sex === 'female');
    updateRender();
};

function animate() {
    requestAnimationFrame(animate);
    controls.update();
    
    // Gentle floating animation
    const time = Date.now() * 0.001;
    organMeshes.forEach((mesh, i) => {
        mesh.position.y += Math.sin(time + i) * 0.2;
    });

    renderer.render(scene, camera);
}

// Initialize
initThreeJS();
