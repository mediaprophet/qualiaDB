//! 3D editing — objects, modelling, rigging, animation, materials, camera/light.
//!
//! ~30 required functions.

use std::collections::BTreeMap;

/// A 3D object in the scene.
#[derive(Debug, Clone)]
pub struct Object3D {
    pub id: String,
    pub name: String,
    pub mesh_id: Option<String>,
    pub transform: Transform3D,
    pub material_id: Option<String>,
    pub children: Vec<String>,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Transform3D {
    pub position: [f32; 3],
    pub rotation: [f32; 4], // quaternion
    pub scale: [f32; 3],
}

impl Default for Object3D {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            mesh_id: None,
            transform: Transform3D::default(),
            material_id: None,
            children: Vec::new(),
            parent: None,
        }
    }
}

impl Object3D {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn set_position(&mut self, x: f32, y: f32, z: f32) {
        self.transform.position = [x, y, z];
    }

    pub fn set_rotation(&mut self, x: f32, y: f32, z: f32, w: f32) {
        self.transform.rotation = [x, y, z, w];
    }

    pub fn set_scale(&mut self, x: f32, y: f32, z: f32) {
        self.transform.scale = [x, y, z];
    }

    pub fn set_mesh(&mut self, mesh_id: &str) {
        self.mesh_id = Some(mesh_id.to_string());
    }

    pub fn set_material(&mut self, material_id: &str) {
        self.material_id = Some(material_id.to_string());
    }

    pub fn add_child(&mut self, child_id: &str) {
        self.children.push(child_id.to_string());
    }
}

/// A 3D material definition.
#[derive(Debug, Clone)]
pub struct Material3D {
    pub id: String,
    pub name: String,
    pub base_colour: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub normal_map: Option<String>,
    pub albedo_map: Option<String>,
    pub double_sided: bool,
}

impl Material3D {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            base_colour: [0.8, 0.8, 0.8, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            emissive: [0.0; 3],
            normal_map: None,
            albedo_map: None,
            double_sided: false,
        }
    }

    pub fn set_base_colour(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.base_colour = [r, g, b, a];
    }

    pub fn set_metallic_roughness(&mut self, metallic: f32, roughness: f32) {
        self.metallic = metallic.clamp(0.0, 1.0);
        self.roughness = roughness.clamp(0.0, 1.0);
    }

    pub fn set_emissive(&mut self, r: f32, g: f32, b: f32) {
        self.emissive = [r, g, b];
    }
}

/// A 3D camera.
#[derive(Debug, Clone)]
pub struct Camera3D {
    pub id: String,
    pub transform: Transform3D,
    pub fov: f32,
    pub near_plane: f32,
    pub far_plane: f32,
    pub aspect_ratio: f32,
    pub projection: ProjectionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionType {
    Perspective,
    Orthographic,
}

impl Camera3D {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            transform: Transform3D::default(),
            fov: 60.0,
            near_plane: 0.1,
            far_plane: 1000.0,
            aspect_ratio: 16.0 / 9.0,
            projection: ProjectionType::Perspective,
        }
    }

    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov.clamp(1.0, 179.0);
    }

    pub fn set_clip_planes(&mut self, near: f32, far: f32) {
        self.near_plane = near.max(0.001);
        self.far_plane = far.max(self.near_plane + 0.001);
    }
}

/// A 3D light.
#[derive(Debug, Clone)]
pub struct Light3D {
    pub id: String,
    pub light_type: LightType3D,
    pub colour: [f32; 3],
    pub intensity: f32,
    pub transform: Transform3D,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType3D {
    Point,
    Directional,
    Spot,
    Area,
}

impl Light3D {
    pub fn new(id: &str, light_type: LightType3D) -> Self {
        Self {
            id: id.to_string(),
            light_type,
            colour: [1.0; 3],
            intensity: 1.0,
            transform: Transform3D::default(),
        }
    }
}

/// A rig for skeletal animation.
#[derive(Debug, Clone)]
pub struct Rig {
    pub id: String,
    pub name: String,
    pub bones: Vec<Bone>,
    pub root_bone: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Bone {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub rest_transform: Transform3D,
    pub inverse_bind: [[f32; 4]; 4],
}

impl Rig {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            bones: Vec::new(),
            root_bone: None,
        }
    }

    pub fn add_bone(&mut self, bone: Bone) {
        if self.root_bone.is_none() && bone.parent.is_none() {
            self.root_bone = Some(bone.id.clone());
        }
        self.bones.push(bone);
    }

    pub fn bone_count(&self) -> usize {
        self.bones.len()
    }

    pub fn find_bone(&self, id: &str) -> Option<&Bone> {
        self.bones.iter().find(|b| b.id == id)
    }
}

/// An animation clip — keyframes for bones.
#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub id: String,
    pub name: String,
    pub duration: f64,
    pub tracks: Vec<AnimationTrack>,
}

#[derive(Debug, Clone)]
pub struct AnimationTrack {
    pub bone_id: String,
    pub property: AnimatedProperty,
    pub keyframes: Vec<Keyframe>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimatedProperty {
    Position,
    Rotation,
    Scale,
}

#[derive(Debug, Clone)]
pub struct Keyframe {
    pub time: f64,
    pub value: [f32; 3], // position/scale or euler rotation
}

impl AnimationClip {
    pub fn new(id: &str, name: &str, duration: f64) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            duration,
            tracks: Vec::new(),
        }
    }

    pub fn add_track(&mut self, track: AnimationTrack) {
        self.tracks.push(track);
    }

    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }
}

/// A 3D scene / world container.
#[derive(Debug, Clone, Default)]
pub struct Scene3D {
    pub objects: BTreeMap<String, Object3D>,
    pub materials: BTreeMap<String, Material3D>,
    pub cameras: BTreeMap<String, Camera3D>,
    pub lights: BTreeMap<String, Light3D>,
    pub rigs: BTreeMap<String, Rig>,
    pub animations: BTreeMap<String, AnimationClip>,
}

impl Scene3D {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_object(&mut self, obj: Object3D) {
        self.objects.insert(obj.id.clone(), obj);
    }

    pub fn add_material(&mut self, mat: Material3D) {
        self.materials.insert(mat.id.clone(), mat);
    }

    pub fn add_camera(&mut self, cam: Camera3D) {
        self.cameras.insert(cam.id.clone(), cam);
    }

    pub fn add_light(&mut self, light: Light3D) {
        self.lights.insert(light.id.clone(), light);
    }

    pub fn add_rig(&mut self, rig: Rig) {
        self.rigs.insert(rig.id.clone(), rig);
    }

    pub fn add_animation(&mut self, clip: AnimationClip) {
        self.animations.insert(clip.id.clone(), clip);
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_creation() {
        let obj = Object3D::new("o1", "Cube");
        assert_eq!(obj.name, "Cube");
        assert_eq!(obj.transform.position, [0.0; 3]);
    }

    #[test]
    fn object_set_transform() {
        let mut obj = Object3D::new("o1", "Cube");
        obj.set_position(1.0, 2.0, 3.0);
        obj.set_scale(2.0, 2.0, 2.0);
        assert_eq!(obj.transform.position, [1.0, 2.0, 3.0]);
        assert_eq!(obj.transform.scale, [2.0, 2.0, 2.0]);
    }

    #[test]
    fn material_creation() {
        let mat = Material3D::new("m1", "Metal");
        assert_eq!(mat.metallic, 0.0);
        assert_eq!(mat.roughness, 0.5);
    }

    #[test]
    fn material_set_metallic_roughness() {
        let mut mat = Material3D::new("m1", "Metal");
        mat.set_metallic_roughness(0.9, 0.1);
        assert_eq!(mat.metallic, 0.9);
        assert_eq!(mat.roughness, 0.1);
    }

    #[test]
    fn camera_creation() {
        let cam = Camera3D::new("cam1");
        assert_eq!(cam.fov, 60.0);
        assert_eq!(cam.projection, ProjectionType::Perspective);
    }

    #[test]
    fn camera_set_fov() {
        let mut cam = Camera3D::new("cam1");
        cam.set_fov(90.0);
        assert_eq!(cam.fov, 90.0);
    }

    #[test]
    fn light_creation() {
        let light = Light3D::new("l1", LightType3D::Point);
        assert_eq!(light.light_type, LightType3D::Point);
        assert_eq!(light.intensity, 1.0);
    }

    #[test]
    fn rig_add_bone() {
        let mut rig = Rig::new("rig1", "Armature");
        rig.add_bone(Bone {
            id: "b1".into(),
            name: "Root".into(),
            parent: None,
            children: vec![],
            rest_transform: Transform3D::default(),
            inverse_bind: [[0.0; 4]; 4],
        });
        assert_eq!(rig.bone_count(), 1);
        assert_eq!(rig.root_bone, Some("b1".to_string()));
    }

    #[test]
    fn animation_clip() {
        let mut clip = AnimationClip::new("a1", "Walk", 2.0);
        clip.add_track(AnimationTrack {
            bone_id: "b1".into(),
            property: AnimatedProperty::Position,
            keyframes: vec![Keyframe {
                time: 0.0,
                value: [0.0; 3],
            }],
        });
        assert_eq!(clip.track_count(), 1);
        assert_eq!(clip.duration, 2.0);
    }

    #[test]
    fn scene_add_objects() {
        let mut scene = Scene3D::new();
        scene.add_object(Object3D::new("o1", "Cube"));
        scene.add_material(Material3D::new("m1", "Metal"));
        scene.add_camera(Camera3D::new("cam1"));
        scene.add_light(Light3D::new("l1", LightType3D::Point));
        assert_eq!(scene.object_count(), 1);
        assert_eq!(scene.materials.len(), 1);
        assert_eq!(scene.cameras.len(), 1);
        assert_eq!(scene.lights.len(), 1);
    }
}
