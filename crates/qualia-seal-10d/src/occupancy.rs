//! OCC1 occupancy sidecar (written by etalon-reconstruct). Not a `.10d` section.

#[derive(Clone, Debug)]
pub struct OccupancyGrid {
    pub resolution: u32,
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub material: Vec<u8>,
}

pub const MAT_EMPTY: u8 = 0;
pub const MAT_MYOCARDIUM: u8 = 1;
pub const MAT_LUMEN: u8 = 2;
pub const MAT_EPICARDIAL_FAT: u8 = 3;

pub fn parse_occ1(bytes: &[u8]) -> Result<OccupancyGrid, String> {
    if bytes.len() < 36 || &bytes[0..4] != b"OCC1" {
        return Err("OCC1 magic missing".into());
    }
    let version = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
    if version != 1 {
        return Err(format!("OCC1 version {version} unsupported"));
    }
    let resolution = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    if !(8..=256).contains(&resolution) {
        return Err("OCC1 resolution must be in 8..=256".into());
    }
    let mut min = [0.0f32; 3];
    let mut max = [0.0f32; 3];
    for i in 0..3 {
        min[i] = f32::from_le_bytes(bytes[12 + i * 4..16 + i * 4].try_into().unwrap());
        max[i] = f32::from_le_bytes(bytes[24 + i * 4..28 + i * 4].try_into().unwrap());
    }
    let n = resolution as usize;
    let expected = 36 + n * n * n;
    if bytes.len() != expected {
        return Err(format!(
            "OCC1 length {} does not match resolution {resolution} (need {expected})",
            bytes.len()
        ));
    }
    Ok(OccupancyGrid {
        resolution,
        min,
        max,
        material: bytes[36..].to_vec(),
    })
}

impl OccupancyGrid {
    pub fn size(&self) -> [f32; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    pub fn index(&self, x: u32, y: u32, z: u32) -> usize {
        (x + self.resolution * (y + self.resolution * z)) as usize
    }

    pub fn material_at(&self, x: u32, y: u32, z: u32) -> u8 {
        self.material[self.index(x, y, z)]
    }

    pub fn voxel_center(&self, x: u32, y: u32, z: u32) -> [f32; 3] {
        let size = self.size();
        let n = self.resolution as f32;
        [
            self.min[0] + (x as f32 + 0.5) / n * size[0],
            self.min[1] + (y as f32 + 0.5) / n * size[1],
            self.min[2] + (z as f32 + 0.5) / n * size[2],
        ]
    }
}

#[cfg(test)]
pub fn synthetic_ball(resolution: u32, radius: f32) -> OccupancyGrid {
    let n = resolution as usize;
    let mut material = vec![MAT_EMPTY; n * n * n];
    let wall = (0.08 * resolution as f32).clamp(1.5, 12.0);
    let fat = (wall * 0.22).max(1.0);
    for z in 0..resolution {
        for y in 0..resolution {
            for x in 0..resolution {
                let px = (x as f32 + 0.5) / resolution as f32 * 2.0 - 1.0;
                let py = (y as f32 + 0.5) / resolution as f32 * 2.0 - 1.0;
                let pz = (z as f32 + 0.5) / resolution as f32 * 2.0 - 1.0;
                let r = (px * px + py * py + pz * pz).sqrt();
                if r > radius {
                    continue;
                }
                let depth = (radius - r) * resolution as f32 * 0.5;
                let i = (x + resolution * (y + resolution * z)) as usize;
                material[i] = if depth <= fat {
                    MAT_EPICARDIAL_FAT
                } else if depth <= wall {
                    MAT_MYOCARDIUM
                } else {
                    MAT_LUMEN
                };
            }
        }
    }
    OccupancyGrid {
        resolution,
        min: [-1.0, -1.0, -1.0],
        max: [1.0, 1.0, 1.0],
        material,
    }
}
