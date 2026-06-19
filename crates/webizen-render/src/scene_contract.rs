//! Neutral GPU-facing scene contract
//!
//! This module defines a serde-serializable scene data structure that can be
//! passed from webizen-studio (or any host) to webizen-render for headless
//! GPU rendering. The contract is backend-agnostic and uses CSS colors to match
//! the existing codebase visual semantics.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

// Re-export SystemTelemetry from telemetry module to avoid duplication
pub use crate::telemetry::SystemTelemetry;

/// A 3D point in screen space (normalized to viewport dimensions)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ScenePoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// A renderable node (vertex) with semantic styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneNode {
    /// Unique identifier for the node (for picking interaction)
    pub id: String,
    /// Position in screen space (x, y normalized 0..1, z for depth)
    pub position: ScenePoint,
    /// CSS color string (e.g., "#ff0000", "rgb(255,0,0)")
    pub color: String,
    /// Point radius in screen pixels
    pub radius: f64,
    /// Opacity (0.0..1.0)
    pub alpha: f64,
    /// Animation state: whether this node is actively inferencing
    #[serde(default)]
    pub is_inferencing: bool,
    /// Pulse rate for visual feedback (Hz, 0.0 = static)
    #[serde(default)]
    pub pulse_rate: f64,
    /// 10D tensor projection for Q42 volumetric data
    #[serde(default)]
    pub tensor: Tensor10DProjection,
    /// Epistemic state for quantum context management
    #[serde(default)]
    pub epistemic_state: EpistemicState,
    /// Temporal version (t value)
    #[serde(default)]
    pub version: f64,
}

/// A renderable edge (line) with semantic styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEdge {
    /// Start point
    pub from: ScenePoint,
    /// End point
    pub to: ScenePoint,
    /// CSS color string
    pub color: String,
    /// Line width in screen pixels
    pub width: f64,
    /// Opacity (0.0..1.0)
    pub alpha: f64,
}

/// A filled polygon (face) with semantic styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneFace {
    /// Vertices in screen space
    pub vertices: Vec<ScenePoint>,
    /// CSS color string
    pub color: String,
    /// Opacity (0.0..1.0)
    pub alpha: f64,
}

/// Projection viewpoint configuration for examining the 10D epistemic manifold
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SceneCamera {
    /// Projection viewpoint position (x, y, z)
    pub position: [f64; 3],
    /// Look-at target for geometric examination (x, y, z)
    pub target: [f64; 3],
    /// Field of view in degrees
    pub fov: f64,
}

impl Default for SceneCamera {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 500.0],
            target: [0.0, 0.0, 0.0],
            fov: 60.0,
        }
    }
}

/// Epistemic state for quantum context management
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum EpistemicState {
    /// Ground truth (q = 0)
    Collapsed,
    /// GSR escrow (q > 0)
    Pending,
    /// Parallel context (q > 0)
    Sandbox,
}

impl Default for EpistemicState {
    fn default() -> Self {
        EpistemicState::Collapsed
    }
}

/// 10D tensor projection for Q42 volumetric data
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Tensor10DProjection {
    /// Quantum Context (0 = collapsed, >0 = pending)
    pub q: f64,
    /// Topological Class
    pub v: f64,
    /// Manifold Index
    pub w: f64,
    /// Semantic X
    pub x: f64,
    /// Semantic Y
    pub y: f64,
    /// Semantic Z
    pub z: f64,
    /// Temporal State
    pub t: f64,
    /// Amplitude (opacity/scale)
    pub alpha: f64,
    /// Modulation (metadata/phase)
    pub mu: f64,
    /// Spectral Signature (color class)
    pub sigma: f64,
}

impl Default for Tensor10DProjection {
    fn default() -> Self {
        Self {
            q: 0.0,
            v: 0.0,
            w: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 0.0,
            alpha: 1.0,
            mu: 0.0,
            sigma: 0.0,
        }
    }
}

impl Tensor10DProjection {
    /// Map spectral signature (σ) to CSS color string
    /// Projects spectral data through CIE XYZ to display gamut
    ///
    /// Zero-heap consideration: Uses stack-allocated arrays for CIE matrices
    pub fn spectral_to_color(&self) -> String {
        // Project spectral to CIE XYZ using stack-allocated matrices
        let xyz = self.spectral_to_cie_xyz();
        // Convert XYZ to sRGB using stack-allocated matrices
        let rgb = self.cie_xyz_to_srgb(xyz);

        format!(
            "rgb({}, {}, {})",
            (rgb[0].clamp(0.0, 1.0) * 255.0) as u8,
            (rgb[1].clamp(0.0, 1.0) * 255.0) as u8,
            (rgb[2].clamp(0.0, 1.0) * 255.0) as u8
        )
    }

    /// Project spectral signature (σ) to CIE XYZ color space
    /// Uses CIE 1931 2-degree color matching functions
    ///
    /// Zero-heap consideration: Stack-allocated matrices, no heap allocation
    fn spectral_to_cie_xyz(&self) -> [f64; 3] {
        // Simplified spectral projection using stack-allocated CMF matrices
        // In full implementation, would use actual CIE 1931 2-degree CMF data
        let sigma = self.sigma;

        // Stack-allocated color matching functions (simplified approximation)
        let cmf_x = [0.4124, 0.3576, 0.1805];
        let cmf_y = [0.2126, 0.7152, 0.0722];
        let cmf_z = [0.0193, 0.1192, 0.9505];

        // Project spectral data through CMFs (stack operations only)
        let x = cmf_x[0] * sigma + cmf_x[1] * (sigma * 0.8) + cmf_x[2] * (sigma * 0.6);
        let y = cmf_y[0] * sigma + cmf_y[1] * (sigma * 0.9) + cmf_y[2] * (sigma * 0.4);
        let z = cmf_z[0] * sigma + cmf_z[1] * (sigma * 0.7) + cmf_z[2] * (sigma * 0.5);

        [x, y, z]
    }

    /// Convert CIE XYZ to sRGB display gamut
    ///
    /// Zero-heap consideration: Stack-allocated transformation matrix
    fn cie_xyz_to_srgb(&self, xyz: [f64; 3]) -> [f64; 3] {
        // Stack-allocated XYZ to sRGB transformation matrix
        let xyz_to_srgb = [
            [3.2406, -1.5372, -0.4986],
            [-0.9689, 1.8758, 0.0415],
            [0.0557, -0.2040, 1.0570],
        ];

        // Matrix multiplication (stack operations only)
        let r =
            xyz_to_srgb[0][0] * xyz[0] + xyz_to_srgb[0][1] * xyz[1] + xyz_to_srgb[0][2] * xyz[2];
        let g =
            xyz_to_srgb[1][0] * xyz[0] + xyz_to_srgb[1][1] * xyz[1] + xyz_to_srgb[1][2] * xyz[2];
        let b =
            xyz_to_srgb[2][0] * xyz[0] + xyz_to_srgb[2][1] * xyz[1] + xyz_to_srgb[2][2] * xyz[2];

        [r, g, b]
    }

    /// Map amplitude (α) to opacity (0.0..1.0)
    pub fn amplitude_to_opacity(&self) -> f64 {
        self.alpha.clamp(0.0, 1.0)
    }

    /// Check if modulation (μ) indicates hidden metadata
    pub fn has_hidden_metadata(&self) -> bool {
        self.mu > 0.5
    }

    /// Map epistemic state from quantum context (q)
    pub fn get_epistemic_state(&self) -> EpistemicState {
        if self.q == 0.0 {
            EpistemicState::Collapsed
        } else {
            EpistemicState::Pending
        }
    }
}

/// Neutral scene contract for GPU rendering
///
/// This structure is designed to be serializable (for IPC) and consumable
/// by the headless renderer. It mirrors the draw semantics from
/// webizen-studio's graph.rs (faces→fill_polygon, edges→line, vertices→point).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderScene {
    /// Renderable nodes (vertices)
    pub nodes: Vec<SceneNode>,
    /// Renderable edges (lines)
    pub edges: Vec<SceneEdge>,
    /// Renderable faces (filled polygons)
    pub faces: Vec<SceneFace>,
    /// Camera configuration
    pub camera: SceneCamera,
    /// Background color (CSS string)
    pub background: String,
    /// Selected node ID for visual feedback (highlighting) - kept for backward compatibility
    #[serde(default)]
    pub selected_node_id: Option<String>,
    /// Selected epistemic anchor index for zero-heap IPC (binary index, not String ID)
    #[serde(default)]
    pub selected_node_index: Option<usize>,
    /// Hovered epistemic anchor index for zero-heap IPC (binary index, not String ID)
    #[serde(default)]
    pub hovered_node_index: Option<usize>,
    /// Transition state for smooth node interpolation
    #[serde(default)]
    pub transition_state: Option<TransitionState>,
    /// Current temporal slice (t value) for time-travel navigation
    #[serde(default)]
    pub temporal_slice: f64,
    /// Epistemic filter for quantum context management
    #[serde(default)]
    pub epistemic_filter: EpistemicState,
}

/// Transition state for smooth node position interpolation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionState {
    /// Previous node positions (id -> position mapping)
    pub previous_positions: Vec<(String, ScenePoint)>,
    /// Transition progress (0.0 to 1.0)
    pub progress: f64,
    /// Transition duration in seconds
    pub duration: f64,
}

impl Default for RenderScene {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            faces: Vec::new(),
            camera: SceneCamera::default(),
            background: "#101820".to_string(), // Dark slate
            selected_node_id: None,
            selected_node_index: None,
            hovered_node_index: None,
            transition_state: None,
            temporal_slice: 0.0,
            epistemic_filter: EpistemicState::Collapsed,
        }
    }
}

impl RenderScene {
    /// Create a new empty scene
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node to the scene
    pub fn add_node(&mut self, node: SceneNode) {
        self.nodes.push(node);
    }

    /// Add an edge to the scene
    pub fn add_edge(&mut self, edge: SceneEdge) {
        self.edges.push(edge);
    }

    /// Add a face to the scene
    pub fn add_face(&mut self, face: SceneFace) {
        self.faces.push(face);
    }

    /// Set the camera configuration
    pub fn set_camera(&mut self, camera: SceneCamera) {
        self.camera = camera;
    }

    /// Set the background color
    pub fn set_background(&mut self, color: impl Into<String>) {
        self.background = color.into();
    }

    /// Check if the scene is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.edges.is_empty() && self.faces.is_empty()
    }

    /// Get the total number of renderable elements
    pub fn element_count(&self) -> usize {
        self.nodes.len() + self.edges.len() + self.faces.len()
    }

    /// Set the selected epistemic anchor index (zero-heap: binary index)
    pub fn set_selected_node_index(&mut self, index: Option<usize>) {
        self.selected_node_index = index;
    }

    /// Set the hovered epistemic anchor index (zero-heap: binary index)
    pub fn set_hovered_node_index(&mut self, index: Option<usize>) {
        self.hovered_node_index = index;
    }

    /// Get the selected epistemic anchor index (zero-heap: binary index)
    pub fn get_selected_node_index(&self) -> Option<usize> {
        self.selected_node_index
    }

    /// Get the hovered epistemic anchor index (zero-heap: binary index)
    pub fn get_hovered_node_index(&self) -> Option<usize> {
        self.hovered_node_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_point_serialization() {
        let point = ScenePoint {
            x: 0.5,
            y: 0.3,
            z: 0.0,
        };
        let serialized = serde_json::to_string(&point).unwrap();
        let deserialized: ScenePoint = serde_json::from_str(&serialized).unwrap();
        assert_eq!(point, deserialized);
    }

    #[test]
    fn test_scene_node_serialization() {
        let node = SceneNode {
            id: "test-node".to_string(),
            position: ScenePoint {
                x: 0.5,
                y: 0.3,
                z: 0.0,
            },
            color: "#ff0000".to_string(),
            radius: 5.0,
            alpha: 1.0,
            is_inferencing: false,
            pulse_rate: 0.0,
        };
        let serialized = serde_json::to_string(&node).unwrap();
        let deserialized: SceneNode = serde_json::from_str(&serialized).unwrap();
        assert_eq!(node.color, deserialized.color);
        assert_eq!(node.radius, deserialized.radius);
        assert_eq!(node.is_inferencing, deserialized.is_inferencing);
        assert_eq!(node.id, deserialized.id);
    }

    #[test]
    fn test_render_scene_serialization() {
        let scene = RenderScene {
            nodes: vec![SceneNode {
                id: "test-node".to_string(),
                position: ScenePoint {
                    x: 0.5,
                    y: 0.3,
                    z: 0.0,
                },
                color: "#ff0000".to_string(),
                radius: 5.0,
                alpha: 1.0,
                is_inferencing: false,
                pulse_rate: 0.0,
            }],
            edges: vec![],
            faces: vec![],
            camera: SceneCamera::default(),
            background: "#101820".to_string(),
        };
        let serialized = serde_json::to_string(&scene).unwrap();
        let deserialized: RenderScene = serde_json::from_str(&serialized).unwrap();
        assert_eq!(scene.nodes.len(), deserialized.nodes.len());
        assert_eq!(scene.background, deserialized.background);
    }

    #[test]
    fn test_render_scene_builder() {
        let mut scene = RenderScene::new();
        scene.set_background("#202030");
        assert_eq!(scene.background, "#202030");
        assert!(scene.is_empty());

        scene.add_node(SceneNode {
            id: "test-node".to_string(),
            position: ScenePoint {
                x: 0.5,
                y: 0.5,
                z: 0.0,
            },
            color: "#00ff00".to_string(),
            radius: 3.0,
            alpha: 0.8,
            is_inferencing: false,
            pulse_rate: 0.0,
        });
        assert!(!scene.is_empty());
        assert_eq!(scene.element_count(), 1);
    }
}
