//! Light sources — point, directional, spot, ambient.
//!
//! Simple light source definitions for the scene graph. These are data
//! structures; the GPU renderer consumes them via uniform buffers.

/// Light source type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    /// Omnidirectional point light.
    Point,
    /// Parallel rays from a direction.
    Directional,
    /// Cone-shaped spot light.
    Spot,
    /// Uniform ambient fill.
    Ambient,
}

/// A light source in the scene.
#[derive(Debug, Clone)]
pub struct Light {
    pub light_type: LightType,
    /// RGB colour (0.0–1.0 per channel).
    pub colour: [f32; 3],
    /// Intensity (lumens for point/spot, lux for directional, 0–1 for ambient).
    pub intensity: f32,
    /// World-space position (point/spot).
    pub position: [f32; 3],
    /// Direction (directional/spot).
    pub direction: [f32; 3],
    /// Cone inner angle in radians (spot only).
    pub inner_cone: f32,
    /// Cone outer angle in radians (spot only).
    pub outer_cone: f32,
    /// Maximum range (point/spot).
    pub range: f32,
    /// Whether the light casts shadows.
    pub cast_shadows: bool,
}

impl Light {
    /// Create a point light.
    pub fn point(position: [f32; 3], colour: [f32; 3], intensity: f32) -> Self {
        Self {
            light_type: LightType::Point,
            colour,
            intensity,
            position,
            direction: [0.0, -1.0, 0.0],
            inner_cone: 0.0,
            outer_cone: 0.0,
            range: 100.0,
            cast_shadows: false,
        }
    }

    /// Create a directional light.
    pub fn directional(direction: [f32; 3], colour: [f32; 3], intensity: f32) -> Self {
        Self {
            light_type: LightType::Directional,
            colour,
            intensity,
            position: [0.0; 3],
            direction,
            inner_cone: 0.0,
            outer_cone: 0.0,
            range: f32::INFINITY,
            cast_shadows: true,
        }
    }

    /// Create a spot light.
    pub fn spot(
        position: [f32; 3],
        direction: [f32; 3],
        colour: [f32; 3],
        intensity: f32,
        inner_cone: f32,
        outer_cone: f32,
    ) -> Self {
        Self {
            light_type: LightType::Spot,
            colour,
            intensity,
            position,
            direction,
            inner_cone,
            outer_cone,
            range: 50.0,
            cast_shadows: false,
        }
    }

    /// Create an ambient light.
    pub fn ambient(colour: [f32; 3], intensity: f32) -> Self {
        Self {
            light_type: LightType::Ambient,
            colour,
            intensity,
            position: [0.0; 3],
            direction: [0.0; 3],
            inner_cone: 0.0,
            outer_cone: 0.0,
            range: 0.0,
            cast_shadows: false,
        }
    }

    /// Set shadow casting.
    pub fn with_shadows(mut self, cast: bool) -> Self {
        self.cast_shadows = cast;
        self
    }

    /// Set range.
    pub fn with_range(mut self, range: f32) -> Self {
        self.range = range;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn point_light_creation() {
        let light = Light::point([1.0, 2.0, 3.0], [1.0, 0.9, 0.8], 100.0);
        assert_eq!(light.light_type, LightType::Point);
        assert_eq!(light.position, [1.0, 2.0, 3.0]);
        assert_eq!(light.intensity, 100.0);
    }

    #[test]
    fn directional_light_creation() {
        let light = Light::directional([0.0, -1.0, 0.0], [1.0; 3], 1.0);
        assert_eq!(light.light_type, LightType::Directional);
        assert!(light.cast_shadows);
        assert!(light.range.is_infinite());
    }

    #[test]
    fn spot_light_creation() {
        let light = Light::spot(
            [0.0, 5.0, 0.0],
            [0.0, -1.0, 0.0],
            [1.0; 3],
            50.0,
            PI / 6.0,
            PI / 4.0,
        );
        assert_eq!(light.light_type, LightType::Spot);
        assert!((light.inner_cone - PI / 6.0).abs() < 1e-6);
        assert!((light.outer_cone - PI / 4.0).abs() < 1e-6);
    }

    #[test]
    fn ambient_light_creation() {
        let light = Light::ambient([0.2; 3], 0.5);
        assert_eq!(light.light_type, LightType::Ambient);
        assert_eq!(light.intensity, 0.5);
    }

    #[test]
    fn light_with_shadows() {
        let light = Light::point([0.0; 3], [1.0; 3], 10.0).with_shadows(true);
        assert!(light.cast_shadows);
    }

    #[test]
    fn light_with_range() {
        let light = Light::point([0.0; 3], [1.0; 3], 10.0).with_range(25.0);
        assert_eq!(light.range, 25.0);
    }
}
