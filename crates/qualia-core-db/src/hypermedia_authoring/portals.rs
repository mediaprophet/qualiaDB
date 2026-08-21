//! Portals / worlds — world building, objects, portals, avatars, physics.
//!
//! ~20 required functions.

use std::collections::BTreeMap;

/// A world — a spatial environment with objects and portals.
#[derive(Debug, Clone)]
pub struct World {
    pub id: String,
    pub name: String,
    pub bounds: WorldBounds,
    pub objects: BTreeMap<String, WorldObject>,
    pub portals: BTreeMap<String, Portal>,
    pub avatars: BTreeMap<String, Avatar>,
    pub physics_config: PhysicsConfig,
}

#[derive(Debug, Clone, Default)]
pub struct WorldBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Debug, Clone, Default)]
pub struct PhysicsConfig {
    pub gravity: [f32; 3],
    pub fixed_timestep: f32,
    pub substeps: u32,
}

impl World {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            bounds: WorldBounds {
                min: [-1000.0; 3],
                max: [1000.0; 3],
            },
            objects: BTreeMap::new(),
            portals: BTreeMap::new(),
            avatars: BTreeMap::new(),
            physics_config: PhysicsConfig {
                gravity: [0.0, -9.81, 0.0],
                fixed_timestep: 1.0 / 60.0,
                substeps: 1,
            },
        }
    }

    pub fn add_object(&mut self, obj: WorldObject) {
        self.objects.insert(obj.id.clone(), obj);
    }

    pub fn add_portal(&mut self, portal: Portal) {
        self.portals.insert(portal.id.clone(), portal);
    }

    pub fn add_avatar(&mut self, avatar: Avatar) {
        self.avatars.insert(avatar.id.clone(), avatar);
    }

    pub fn set_gravity(&mut self, x: f32, y: f32, z: f32) {
        self.physics_config.gravity = [x, y, z];
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn portal_count(&self) -> usize {
        self.portals.len()
    }
}

/// A world object with physics properties.
#[derive(Debug, Clone)]
pub struct WorldObject {
    pub id: String,
    pub name: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
    pub mass: f32,
    pub collider: Collider,
    pub static_object: bool,
}

#[derive(Debug, Clone)]
pub enum Collider {
    Box { half_extents: [f32; 3] },
    Sphere { radius: f32 },
    Capsule { radius: f32, height: f32 },
    Mesh { mesh_id: String },
}

impl WorldObject {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            velocity: [0.0; 3],
            angular_velocity: [0.0; 3],
            mass: 1.0,
            collider: Collider::Sphere { radius: 0.5 },
            static_object: false,
        }
    }

    pub fn set_position(&mut self, x: f32, y: f32, z: f32) {
        self.position = [x, y, z];
    }

    pub fn set_velocity(&mut self, x: f32, y: f32, z: f32) {
        self.velocity = [x, y, z];
    }

    pub fn set_mass(&mut self, mass: f32) {
        self.mass = mass.max(0.0);
    }

    pub fn set_collider(&mut self, collider: Collider) {
        self.collider = collider;
    }

    /// Apply a force for one timestep (simplified Euler integration).
    pub fn apply_force(&mut self, fx: f32, fy: f32, fz: f32, dt: f32) {
        if self.static_object || self.mass == 0.0 {
            return;
        }
        let ax = fx / self.mass;
        let ay = fy / self.mass;
        let az = fz / self.mass;
        self.velocity[0] += ax * dt;
        self.velocity[1] += ay * dt;
        self.velocity[2] += az * dt;
    }

    /// Step physics for this object (Euler integration with gravity).
    pub fn step_physics(&mut self, gravity: [f32; 3], dt: f32) {
        if self.static_object {
            return;
        }
        self.velocity[0] += gravity[0] * dt;
        self.velocity[1] += gravity[1] * dt;
        self.velocity[2] += gravity[2] * dt;
        self.position[0] += self.velocity[0] * dt;
        self.position[1] += self.velocity[1] * dt;
        self.position[2] += self.velocity[2] * dt;
    }
}

/// A portal connecting two worlds.
#[derive(Debug, Clone)]
pub struct Portal {
    pub id: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub target_world: String,
    pub target_position: [f32; 3],
    pub radius: f32,
    pub active: bool,
}

impl Portal {
    pub fn new(id: &str, target_world: &str) -> Self {
        Self {
            id: id.to_string(),
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            target_world: target_world.to_string(),
            target_position: [0.0; 3],
            radius: 1.0,
            active: true,
        }
    }

    pub fn set_position(&mut self, x: f32, y: f32, z: f32) {
        self.position = [x, y, z];
    }

    pub fn set_target(&mut self, world: &str, x: f32, y: f32, z: f32) {
        self.target_world = world.to_string();
        self.target_position = [x, y, z];
    }

    pub fn activate(&mut self) {
        self.active = true;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

/// An avatar representing a user in the world.
#[derive(Debug, Clone)]
pub struct Avatar {
    pub id: String,
    pub user_did: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub appearance: AvatarAppearance,
    pub movement_mode: MovementMode,
}

#[derive(Debug, Clone, Default)]
pub struct AvatarAppearance {
    pub model_id: Option<String>,
    pub height: f32,
    pub scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementMode {
    Walking,
    Flying,
    Teleporting,
    Sitting,
}

impl Avatar {
    pub fn new(id: &str, user_did: &str) -> Self {
        Self {
            id: id.to_string(),
            user_did: user_did.to_string(),
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            appearance: AvatarAppearance {
                model_id: None,
                height: 1.8,
                scale: 1.0,
            },
            movement_mode: MovementMode::Walking,
        }
    }

    pub fn move_to(&mut self, x: f32, y: f32, z: f32) {
        self.position = [x, y, z];
    }

    pub fn set_movement_mode(&mut self, mode: MovementMode) {
        self.movement_mode = mode;
    }

    pub fn set_appearance(&mut self, model_id: &str, height: f32, scale: f32) {
        self.appearance.model_id = Some(model_id.to_string());
        self.appearance.height = height;
        self.appearance.scale = scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_creation() {
        let world = World::new("w1", "My World");
        assert_eq!(world.name, "My World");
        assert_eq!(world.object_count(), 0);
    }

    #[test]
    fn world_add_object() {
        let mut world = World::new("w1", "World");
        world.add_object(WorldObject::new("o1", "Cube"));
        assert_eq!(world.object_count(), 1);
    }

    #[test]
    fn world_add_portal() {
        let mut world = World::new("w1", "World");
        world.add_portal(Portal::new("p1", "w2"));
        assert_eq!(world.portal_count(), 1);
    }

    #[test]
    fn world_set_gravity() {
        let mut world = World::new("w1", "World");
        world.set_gravity(0.0, -20.0, 0.0);
        assert_eq!(world.physics_config.gravity, [0.0, -20.0, 0.0]);
    }

    #[test]
    fn object_physics_step() {
        let mut obj = WorldObject::new("o1", "Ball");
        obj.set_position(0.0, 10.0, 0.0);
        obj.step_physics([0.0, -9.81, 0.0], 1.0 / 60.0);
        // After one step, velocity should be -9.81/60, position should drop.
        assert!(obj.velocity[1] < 0.0);
        assert!(obj.position[1] < 10.0);
    }

    #[test]
    fn object_apply_force() {
        let mut obj = WorldObject::new("o1", "Ball");
        obj.set_mass(2.0);
        obj.apply_force(10.0, 0.0, 0.0, 1.0);
        // a = F/m = 5, v = 5 * 1 = 5
        assert!((obj.velocity[0] - 5.0).abs() < 0.01);
    }

    #[test]
    fn object_static_no_physics() {
        let mut obj = WorldObject::new("o1", "Wall");
        obj.static_object = true;
        obj.set_position(0.0, 10.0, 0.0);
        obj.step_physics([0.0, -9.81, 0.0], 1.0);
        assert_eq!(obj.position, [0.0, 10.0, 0.0]);
    }

    #[test]
    fn portal_creation() {
        let portal = Portal::new("p1", "w2");
        assert_eq!(portal.target_world, "w2");
        assert!(portal.active);
    }

    #[test]
    fn portal_activate_deactivate() {
        let mut portal = Portal::new("p1", "w2");
        portal.deactivate();
        assert!(!portal.active);
        portal.activate();
        assert!(portal.active);
    }

    #[test]
    fn avatar_creation() {
        let avatar = Avatar::new("a1", "did:q42:user1");
        assert_eq!(avatar.user_did, "did:q42:user1");
        assert_eq!(avatar.movement_mode, MovementMode::Walking);
    }

    #[test]
    fn avatar_move() {
        let mut avatar = Avatar::new("a1", "user1");
        avatar.move_to(10.0, 0.0, 5.0);
        assert_eq!(avatar.position, [10.0, 0.0, 5.0]);
    }

    #[test]
    fn avatar_set_appearance() {
        let mut avatar = Avatar::new("a1", "user1");
        avatar.set_appearance("model_a", 1.9, 1.1);
        assert_eq!(avatar.appearance.height, 1.9);
        assert_eq!(avatar.appearance.scale, 1.1);
    }
}
