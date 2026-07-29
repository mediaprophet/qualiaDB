//! Bounded mesh IR (caller-owned fixed arrays for edge path; Vec only in builders).

/// Max vertices in a single vision reconstruction mesh.
pub const MAX_VERTICES: usize = 4096;
/// Max triangle indices (3 per tri).
pub const MAX_INDICES: usize = 12288;

/// Axis-aligned bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Aabb {
    pub const EMPTY: Self = Self {
        min: [f32::INFINITY, f32::INFINITY, f32::INFINITY],
        max: [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    pub fn expand_point(&mut self, p: [f32; 3]) {
        for i in 0..3 {
            self.min[i] = self.min[i].min(p[i]);
            self.max[i] = self.max[i].max(p[i]);
        }
    }

    pub fn is_finite(&self) -> bool {
        self.min
            .iter()
            .chain(self.max.iter())
            .all(|v| v.is_finite())
    }
}

/// Triangle mesh in fixed buffers (zero growth after fill).
#[derive(Debug, Clone)]
pub struct MeshIR {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// Triangle list indices.
    pub indices: Vec<u32>,
    pub bounds: Aabb,
    /// Content hash of positions+indices (FNV-ish mix).
    pub content_hash: u64,
}

impl MeshIR {
    pub fn empty() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
            bounds: Aabb::EMPTY,
            content_hash: 0,
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn recompute_bounds_and_hash(&mut self) {
        self.bounds = Aabb::EMPTY;
        for p in &self.positions {
            self.bounds.expand_point(*p);
        }
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for p in &self.positions {
            for c in p {
                h ^= c.to_bits() as u64;
                h = h.wrapping_mul(0x100_0000_01b3);
            }
        }
        for &i in &self.indices {
            h ^= i as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
        self.content_hash = h;
    }

    /// Compact binary dump of positions+indices for digest (cold).
    pub fn packed_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.positions.len() * 12 + self.indices.len() * 4);
        for p in &self.positions {
            for c in p {
                v.extend_from_slice(&c.to_le_bytes());
            }
        }
        for i in &self.indices {
            v.extend_from_slice(&i.to_le_bytes());
        }
        v
    }
}
