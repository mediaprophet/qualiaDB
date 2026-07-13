use qualia_core_db::render::assets::Mesh;

pub fn generate_sphere_mesh(segments: u32, rings: u32) -> Mesh {
    let mut positions = Vec::new();
    let mut triangles = Vec::new();

    for r in 0..=rings {
        let phi = std::f32::consts::PI * (r as f32) / (rings as f32);
        let y = phi.cos();
        let radius = phi.sin();
        for s in 0..=segments {
            let theta = 2.0 * std::f32::consts::PI * (s as f32) / (segments as f32);
            let x = radius * theta.cos();
            let z = radius * theta.sin();
            positions.push([x, y, z]);
        }
    }

    for r in 0..rings {
        for s in 0..segments {
            let a = r * (segments + 1) + s;
            let b = a + segments + 1;
            if r > 0 {
                triangles.push([a, b, a + 1]);
            }
            if r < rings - 1 {
                triangles.push([a + 1, b, b + 1]);
            }
        }
    }

    let min = [-1.0, -1.0, -1.0];
    let max = [1.0, 1.0, 1.0];
    Mesh { positions, triangles, min, max }
}

pub fn generate_sphere_mesh_colored(
    segments: u32,
    rings: u32,
    color_sampler: impl Fn(f32, f32) -> [f32; 3],
) -> (Vec<[f32; 3]>, Vec<[f32; 4]>, Vec<u32>) {
    let mesh = generate_sphere_mesh(segments, rings);
    let mut colors = Vec::with_capacity(mesh.positions.len());
    for &p in &mesh.positions {
        let lat = p[1].asin().to_degrees();
        let lon = p[0].atan2(p[2]).to_degrees();
        let [r, g, b] = color_sampler(lat, lon);
        colors.push([r, g, b, 1.0]);
    }
    let indices: Vec<u32> = mesh
        .triangles
        .iter()
        .flat_map(|t| [t[0], t[1], t[2]])
        .collect();
    (mesh.positions, colors, indices)
}

pub fn generate_starfield_mesh(positions: &[[f32; 3]], colors: &[[f32; 4]]) -> Mesh {
    let triangles: Vec<[u32; 3]> = (0..positions.len() as u32)
        .step_by(1)
        .map(|i| [i, i, i])
        .collect();
    let _ = colors;
    let min = [-1e6, -1e6, -1e6];
    let max = [1e6, 1e6, 1e6];
    Mesh {
        positions: positions.to_vec(),
        triangles,
        min,
        max,
    }
}

pub fn generate_terrain_mesh(heightfield: &[f32], width: u32, height: u32, scale: f32) -> Mesh {
    let mut positions = Vec::with_capacity((width * height) as usize);
    let mut triangles = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let h = heightfield.get(idx).copied().unwrap_or(0.0) * scale;
            let px = (x as f32 / width as f32 - 0.5) * 2.0;
            let pz = (y as f32 / height as f32 - 0.5) * 2.0;
            positions.push([px, h, pz]);
        }
    }

    for y in 0..height - 1 {
        for x in 0..width - 1 {
            let a = (y * width + x) as u32;
            let b = a + 1;
            let c = a + width;
            let d = c + 1;
            triangles.push([a, c, b]);
            triangles.push([b, c, d]);
        }
    }

    let min = [-1.0, 0.0, -1.0];
    let max = [1.0, scale, 1.0];
    Mesh { positions, triangles, min, max }
}
