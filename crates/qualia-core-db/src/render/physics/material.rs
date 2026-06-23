//! Material, mass, and momentum for an artefact — the `P` (physical momentum) the STELLAR §C
//! Manifold-Coordinate carries, applied here to a rendered body. Zero-alloc, deterministic.

use super::aabb::Aabb;

/// Bulk material properties of an artefact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    /// Density in mass per unit volume (units are the caller's; consistent within a scene).
    pub density: f32,
    /// Whether the material resists compression (paired with `Admission::min_extent` upstream).
    pub incompressible: bool,
}

impl Material {
    pub const fn new(density: f32, incompressible: bool) -> Self {
        Material { density, incompressible }
    }
}

/// A physical body: a material filling a bounding box, with a linear velocity.
#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub material: Material,
    pub aabb: Aabb,
    pub velocity: [f32; 3],
}

impl Body {
    pub fn new(material: Material, aabb: Aabb, velocity: [f32; 3]) -> Self {
        Body { material, aabb, velocity }
    }

    /// Mass = density × enclosed volume.
    #[inline]
    pub fn mass(&self) -> f32 {
        self.material.density * self.aabb.volume()
    }

    /// Linear momentum `P = m·v` — the kinetic quantity stored in the Manifold-Coordinate.
    #[inline]
    pub fn momentum(&self) -> [f32; 3] {
        let m = self.mass();
        [self.velocity[0] * m, self.velocity[1] * m, self.velocity[2] * m]
    }

    /// Kinetic energy `½·m·|v|²`.
    #[inline]
    pub fn kinetic_energy(&self) -> f32 {
        let v2 = self.velocity[0] * self.velocity[0]
            + self.velocity[1] * self.velocity[1]
            + self.velocity[2] * self.velocity[2];
        0.5 * self.mass() * v2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mass_is_density_times_volume() {
        let b = Body::new(
            Material::new(2.0, true),
            Aabb::new([0.0, 0.0, 0.0], [1.0, 2.0, 3.0]), // volume 6
            [0.0, 0.0, 0.0],
        );
        assert_eq!(b.mass(), 12.0);
    }

    #[test]
    fn momentum_and_energy() {
        let b = Body::new(
            Material::new(1.0, false),
            Aabb::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]), // volume 1, mass 1
            [3.0, 0.0, 4.0],
        );
        assert_eq!(b.momentum(), [3.0, 0.0, 4.0]);
        assert_eq!(b.kinetic_energy(), 0.5 * 25.0); // |v|² = 25
    }
}
