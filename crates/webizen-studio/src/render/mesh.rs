//! Mesh primitives and transforms — the geometry half of the engine dev-kit.
//!
//! A [`Mesh`] is backend-neutral: vertices in local space plus edge and face
//! index lists. The [`Renderer`](super::Renderer) draws edges as lines, faces as
//! filled polygons, and vertices as points, so the same mesh works on the CPU
//! Canvas2D backend today and a WebGPU backend later. Primitive builders
//! (`cube`, `grid`, `uv_sphere`, `quad`, `line`) replace the hand-rolled geometry
//! that JS engines (three.js/Babylon) used to provide.

use super::scene::Vec3;

/// Position / Euler-rotation / scale transform. Rotation is applied X→Y→Z.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::default(),
            rotation: Vec3::default(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

fn rotate_x(v: Vec3, a: f64) -> Vec3 {
    let (s, c) = a.sin_cos();
    Vec3::new(v.x, v.y * c - v.z * s, v.y * s + v.z * c)
}
fn rotate_y(v: Vec3, a: f64) -> Vec3 {
    let (s, c) = a.sin_cos();
    Vec3::new(v.x * c + v.z * s, v.y, -v.x * s + v.z * c)
}
fn rotate_z(v: Vec3, a: f64) -> Vec3 {
    let (s, c) = a.sin_cos();
    Vec3::new(v.x * c - v.y * s, v.x * s + v.y * c, v.z)
}

impl Transform {
    /// Identity transform translated to `position`.
    pub fn at(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    pub fn with_scale(mut self, s: f64) -> Self {
        self.scale = Vec3::new(s, s, s);
        self
    }

    pub fn with_rotation(mut self, rotation: Vec3) -> Self {
        self.rotation = rotation;
        self
    }

    /// Map a local-space point to this transform's space: scale → rotate → translate.
    pub fn apply(&self, v: Vec3) -> Vec3 {
        let scaled = Vec3::new(v.x * self.scale.x, v.y * self.scale.y, v.z * self.scale.z);
        let rotated = rotate_z(
            rotate_y(rotate_x(scaled, self.rotation.x), self.rotation.y),
            self.rotation.z,
        );
        rotated.add(self.position)
    }
}

/// Backend-neutral geometry: local-space vertices plus edge and face index lists.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub edges: Vec<(usize, usize)>,
    pub faces: Vec<Vec<usize>>,
}

impl Mesh {
    /// A single line segment.
    pub fn line(a: Vec3, b: Vec3) -> Self {
        Self {
            vertices: vec![a, b],
            edges: vec![(0, 1)],
            faces: vec![],
        }
    }

    /// An axis-aligned cube of the given edge length, centered at the origin.
    pub fn cube(size: f64) -> Self {
        let h = size * 0.5;
        let vertices = vec![
            Vec3::new(-h, -h, -h),
            Vec3::new(h, -h, -h),
            Vec3::new(h, h, -h),
            Vec3::new(-h, h, -h),
            Vec3::new(-h, -h, h),
            Vec3::new(h, -h, h),
            Vec3::new(h, h, h),
            Vec3::new(-h, h, h),
        ];
        let edges = vec![
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ];
        let faces = vec![
            vec![0, 1, 2, 3],
            vec![4, 5, 6, 7],
            vec![0, 1, 5, 4],
            vec![2, 3, 7, 6],
            vec![1, 2, 6, 5],
            vec![0, 3, 7, 4],
        ];
        Self {
            vertices,
            edges,
            faces,
        }
    }

    /// A single quad face on the XZ plane, centered at the origin.
    pub fn quad(size: f64) -> Self {
        let h = size * 0.5;
        Self {
            vertices: vec![
                Vec3::new(-h, 0.0, -h),
                Vec3::new(h, 0.0, -h),
                Vec3::new(h, 0.0, h),
                Vec3::new(-h, 0.0, h),
            ],
            edges: vec![(0, 1), (1, 2), (2, 3), (3, 0)],
            faces: vec![vec![0, 1, 2, 3]],
        }
    }

    /// A wireframe grid on the XZ plane: `steps`×`steps` cells spanning ±`span`.
    pub fn grid(span: f64, steps: usize) -> Self {
        let steps = steps.max(1);
        let step = span * 2.0 / steps as f64;
        let mut vertices = Vec::with_capacity((steps + 1) * (steps + 1));
        let idx = |x: usize, z: usize| z * (steps + 1) + x;
        for z in 0..=steps {
            for x in 0..=steps {
                vertices.push(Vec3::new(
                    -span + x as f64 * step,
                    0.0,
                    -span + z as f64 * step,
                ));
            }
        }
        let mut edges = Vec::new();
        for z in 0..=steps {
            for x in 0..steps {
                edges.push((idx(x, z), idx(x + 1, z)));
            }
        }
        for x in 0..=steps {
            for z in 0..steps {
                edges.push((idx(x, z), idx(x, z + 1)));
            }
        }
        Self {
            vertices,
            edges,
            faces: vec![],
        }
    }

    /// A UV sphere of `radius` with `rings` latitude bands and `segments`
    /// longitude divisions. Edges trace the wire mesh; faces are the quads.
    pub fn uv_sphere(radius: f64, rings: usize, segments: usize) -> Self {
        let rings = rings.max(2);
        let segments = segments.max(3);
        let mut vertices = Vec::new();
        let idx = |r: usize, s: usize| r * segments + s;
        for r in 0..=rings {
            let phi = std::f64::consts::PI * r as f64 / rings as f64;
            let (sp, cp) = phi.sin_cos();
            for s in 0..segments {
                let theta = std::f64::consts::TAU * s as f64 / segments as f64;
                let (st, ct) = theta.sin_cos();
                vertices.push(Vec3::new(radius * sp * ct, radius * cp, radius * sp * st));
            }
        }
        let mut edges = Vec::new();
        let mut faces = Vec::new();
        for r in 0..rings {
            for s in 0..segments {
                let s_next = (s + 1) % segments;
                edges.push((idx(r, s), idx(r, s_next)));
                edges.push((idx(r, s), idx(r + 1, s)));
                faces.push(vec![
                    idx(r, s),
                    idx(r, s_next),
                    idx(r + 1, s_next),
                    idx(r + 1, s),
                ]);
            }
        }
        Self {
            vertices,
            edges,
            faces,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_topology() {
        let c = Mesh::cube(2.0);
        assert_eq!(c.vertices.len(), 8);
        assert_eq!(c.edges.len(), 12);
        assert_eq!(c.faces.len(), 6);
        // corner sits at half-size
        assert_eq!(c.vertices[6], Vec3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn transform_scale_then_translate() {
        let t = Transform::at(Vec3::new(10.0, 0.0, 0.0)).with_scale(2.0);
        assert_eq!(t.apply(Vec3::new(1.0, 0.0, 0.0)), Vec3::new(12.0, 0.0, 0.0));
    }

    #[test]
    fn grid_vertex_count() {
        let g = Mesh::grid(5.0, 4);
        assert_eq!(g.vertices.len(), 25);
        assert!(g.faces.is_empty());
    }
}
